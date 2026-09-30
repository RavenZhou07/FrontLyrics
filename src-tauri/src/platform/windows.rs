//! WinRT is owned by one MTA thread. Callbacks only set dirty flags/wake it;
//! metadata and artwork never block the UI or the WinRT event dispatch thread.
use crate::{
    lyrics::matching::canonical_key,
    model::{now_ms, Playback, PlaybackStatus},
    playback::{extrapolate, reconcile_position, Clock},
};
use base64::Engine;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{sync_channel, SyncSender},
        Arc,
    },
    time::{Duration, Instant},
};
use windows::{
    Foundation::TypedEventHandler,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as SessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    },
    Storage::Streams::DataReader,
    Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
};

thread_local! { static WINRT_EXECUTOR: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread().enable_time().build().expect("WinRT executor"); }
fn wait<T: windows::core::RuntimeType + 'static>(
    operation: windows_future::IAsyncOperation<T>,
) -> windows::core::Result<T> {
    WINRT_EXECUTOR.with(|runtime| {
        runtime.block_on(async {
            match tokio::time::timeout(Duration::from_secs(4), async { operation.clone().await })
                .await
            {
                Ok(result) => result,
                Err(_) => {
                    let _ = operation.Cancel();
                    Err(windows::core::Error::new(
                        windows::core::HRESULT(0x800705B4u32 as i32),
                        "WinRT request timed out",
                    ))
                }
            }
        })
    })
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}
struct ManagerEvents {
    manager: SessionManager,
    sessions: i64,
    current: i64,
}
impl Drop for ManagerEvents {
    fn drop(&mut self) {
        let _ = self.manager.RemoveSessionsChanged(self.sessions);
        let _ = self.manager.RemoveCurrentSessionChanged(self.current);
    }
}

const WINDOWS_EPOCH_MS: i64 = 11644473600000;
fn app_name(id: &str) -> String {
    let lower = id.to_lowercase();
    for (needle, label) in [
        ("spotify", "Spotify"),
        ("cloudmusic", "网易云音乐"),
        ("netease", "网易云音乐"),
        ("qqmusic", "QQ 音乐"),
        ("applemusic", "Apple Music"),
        ("youtube", "YouTube Music"),
        ("msedge", "Microsoft Edge"),
        ("chrome", "Google Chrome"),
        ("firefox", "Firefox"),
    ] {
        if lower.contains(needle) {
            return label.into();
        }
    }
    id.into()
}
struct WatchedSession {
    session: Session,
    source_id: String,
    metadata_dirty: Arc<AtomicBool>,
    tokens: [i64; 3],
    clock: Option<Clock>,
    stamp: Option<(i64, i64, i64, i64)>,
    last_metadata: Instant,
}
impl Drop for WatchedSession {
    fn drop(&mut self) {
        let _ = self.session.RemoveMediaPropertiesChanged(self.tokens[0]);
        let _ = self.session.RemovePlaybackInfoChanged(self.tokens[1]);
        let _ = self.session.RemoveTimelinePropertiesChanged(self.tokens[2]);
    }
}
impl WatchedSession {
    fn new(
        session: Session,
        source_id: String,
        wake: &SyncSender<()>,
    ) -> windows::core::Result<Self> {
        let metadata_dirty = Arc::new(AtomicBool::new(true));
        let dirty = metadata_dirty.clone();
        let tx = wake.clone();
        let metadata = session.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
            dirty.store(true, Ordering::Relaxed);
            let _ = tx.try_send(());
            Ok(())
        }))?;
        let tx = wake.clone();
        let playback = session.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tx.try_send(());
            Ok(())
        }))?;
        let tx = wake.clone();
        let timeline =
            session.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
                let _ = tx.try_send(());
                Ok(())
            }))?;
        tracing::info!(event = "session_discovered", source = %source_id);
        Ok(Self {
            session,
            source_id,
            metadata_dirty,
            tokens: [metadata, playback, timeline],
            clock: None,
            stamp: None,
            last_metadata: Instant::now(),
        })
    }
    fn read(&mut self) -> windows::core::Result<Playback> {
        let mut p = self.clock.as_ref().map(Clock::snapshot).unwrap_or_default();
        let old_identity = (p.title.clone(), p.artists.clone(), p.album.clone());
        if self.clock.is_none()
            || self.metadata_dirty.swap(false, Ordering::Relaxed)
            || self.last_metadata.elapsed() >= Duration::from_secs(15)
        {
            let m = wait(self.session.TryGetMediaPropertiesAsync()?)?;
            p.title = m.Title()?.to_string();
            let artist = m.Artist()?.to_string();
            p.artists = if artist.trim().is_empty() {
                vec![]
            } else {
                vec![artist]
            };
            p.album = m.AlbumTitle()?.to_string();
            if old_identity != (p.title.clone(), p.artists.clone(), p.album.clone())
                || self.clock.is_none()
            {
                p.artwork = artwork(&m).ok().flatten();
                tracing::info!(event = "track_changed", source = %self.source_id);
            }
            self.last_metadata = Instant::now();
        }
        let changed_track = old_identity != (p.title.clone(), p.artists.clone(), p.album.clone());
        let info = self.session.GetPlaybackInfo()?;
        let status = match info.PlaybackStatus()? {
            Status::Playing => PlaybackStatus::Playing,
            Status::Paused => PlaybackStatus::Paused,
            _ => PlaybackStatus::Stopped,
        };
        let rate = info.PlaybackRate().and_then(|r| r.Value()).unwrap_or(1.0);
        let rate = if rate.is_finite() && rate > 0. && rate <= 8. {
            rate
        } else {
            1.0
        };
        let timeline = self.session.GetTimelineProperties()?;
        let position = timeline.Position()?.Duration / 10000;
        let start = timeline.StartTime()?.Duration / 10000;
        let end = timeline.EndTime()?.Duration / 10000;
        let updated = timeline.LastUpdatedTime()?.UniversalTime / 10000 - WINDOWS_EPOCH_MS;
        let stamp = (position, updated, start, end);
        p.duration_ms = (end - start).max(0);
        p.timeline_available = p.duration_ms > 0 || position > start;
        if self
            .clock
            .as_ref()
            .is_none_or(|c| c.playback.timeline_available != p.timeline_available)
        {
            tracing::info!(event = "timeline_capability", source = %self.source_id, available = p.timeline_available);
        }
        if self.stamp != Some(stamp) {
            let age = if updated > 0 {
                (now_ms() - updated).clamp(0, 86400000)
            } else {
                0
            };
            let authoritative = extrapolate(
                position - start,
                age,
                p.duration_ms,
                status == PlaybackStatus::Playing,
                rate,
            );
            let correction = authoritative - p.position_ms;
            if !changed_track && self.clock.is_some() && correction.abs() > 1500 {
                tracing::info!(event = "sync_correction", source = %self.source_id, correction_ms = correction);
            }
            p.position_ms = reconcile_position(p.position_ms, Some(authoritative), changed_track);
            self.stamp = Some(stamp);
        } else {
            p.position_ms = reconcile_position(p.position_ms, None, changed_track);
        }
        // Without a new authoritative timeline, preserve our monotonic clock on
        // pause/resume. Repeated stale Position values must not rewind playback.
        p.status = status;
        if !p.timeline_available {
            p.position_ms = 0;
        }
        p.playback_rate = rate;
        p.source_id = self.source_id.clone();
        p.source_app = app_name(&self.session.SourceAppUserModelId()?.to_string());
        p.snapshot_at_ms = now_ms();
        p.track_key = canonical_key(&p);
        self.clock = Some(Clock {
            playback: p.clone(),
            received: Instant::now(),
        });
        Ok(p)
    }
}
fn artwork(
    m: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
) -> windows::core::Result<Option<String>> {
    let thumbnail = match m.Thumbnail() {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };
    let stream = wait(thumbnail.OpenReadAsync()?)?;
    let size = stream.Size()?;
    if size == 0 || size > 2_000_000 {
        return Ok(None);
    }
    let content_type = stream.ContentType()?.to_string();
    if !["image/jpeg", "image/png", "image/webp", "image/bmp"].contains(&content_type.as_str()) {
        return Ok(None);
    }
    let reader = DataReader::CreateDataReader(&stream)?;
    let loaded = wait(reader.LoadAsync(size as u32)?)?;
    let mut data = vec![0u8; loaded as usize];
    reader.ReadBytes(&mut data)?;
    Ok(Some(format!(
        "data:{content_type};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(data)
    )))
}

pub fn watch(
    mut callback: impl FnMut(Vec<Playback>, Option<String>) + Send + 'static,
) -> windows::core::Result<()> {
    // This function runs on a new, dedicated thread, never on Tauri's STA UI thread.
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED)?;
    }
    let _apartment = Apartment;
    let manager = wait(SessionManager::RequestAsync()?)?;
    let (tx, rx) = sync_channel(1);
    let wake = tx.clone();
    let _sessions_token = manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
        let _ = wake.try_send(());
        Ok(())
    }))?;
    let wake = tx.clone();
    let _current_token = manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
        let _ = wake.try_send(());
        Ok(())
    }))?;
    let _events = ManagerEvents {
        manager: manager.clone(),
        sessions: _sessions_token,
        current: _current_token,
    };
    let mut watched: Vec<WatchedSession> = vec![];
    let mut serial = 0u64;
    loop {
        let sessions = manager.GetSessions()?;
        let sessions: Vec<_> = (0..sessions.Size()?)
            .filter_map(|i| sessions.GetAt(i).ok())
            .collect();
        watched.retain(|w| sessions.contains(&w.session));
        for session in sessions {
            if watched.iter().any(|w| w.session == session) {
                continue;
            }
            let base = session.SourceAppUserModelId()?.to_string();
            let id = if watched.iter().any(|w| w.source_id == base) {
                serial += 1;
                format!("{base}#{serial}")
            } else {
                base
            };
            match WatchedSession::new(session, id, &tx) {
                Ok(w) => watched.push(w),
                Err(error) => tracing::warn!(event = "session_subscription_failed", %error),
            }
        }
        let current = manager.GetCurrentSession().ok();
        let current_id = watched
            .iter()
            .find(|w| Some(&w.session) == current.as_ref())
            .map(|w| w.source_id.clone());
        let mut playback = vec![];
        for w in &mut watched {
            match w.read() {
                Ok(p) => playback.push(p),
                Err(error) => {
                    tracing::warn!(event = "session_read_failed", source = %w.source_id, %error);
                    w.metadata_dirty.store(true, Ordering::Relaxed);
                }
            }
        }
        callback(playback, current_id);
        // One-second safety poll covers players missing an event; updates normally
        // wake immediately. At most ten event batches/sec under event storms.
        let _ = rx.recv_timeout(Duration::from_secs(1));
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Read-only real SMTC diagnostic, used by `npm run probe:smtc`.
pub fn probe() -> windows::core::Result<Vec<Playback>> {
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED)?;
    }
    let _apartment = Apartment;
    let manager = wait(SessionManager::RequestAsync()?)?;
    let sessions = manager.GetSessions()?;
    let (tx, _) = sync_channel(1);
    let mut output = vec![];
    for i in 0..sessions.Size()? {
        let session = sessions.GetAt(i)?;
        let id = session.SourceAppUserModelId()?.to_string();
        let mut p = WatchedSession::new(session, id, &tx)?.read()?;
        p.artwork = None;
        output.push(p);
    }
    Ok(output)
}
pub fn fonts() -> Vec<String> {
    use winreg::{
        enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
        RegKey,
    };
    let mut fonts = vec![
        "Segoe UI".into(),
        "Microsoft YaHei UI".into(),
        "Microsoft JhengHei UI".into(),
        "SimHei".into(),
    ];
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        if let Ok(key) =
            RegKey::predef(hive).open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts")
        {
            for (name, _) in key.enum_values().flatten() {
                let name = name.replace(" (TrueType)", "").replace(" (OpenType)", "");
                fonts.extend(name.split(" & ").map(str::to_owned));
            }
        }
    }
    fonts.sort();
    fonts.dedup();
    fonts
}
