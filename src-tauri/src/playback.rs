use crate::model::{Playback, PlaybackStatus};
use std::time::Instant;

pub fn extrapolate(position: i64, elapsed: i64, duration: i64, playing: bool, rate: f64) -> i64 {
    let position = position
        .saturating_add(if playing {
            (elapsed.max(0) as f64 * rate) as i64
        } else {
            0
        })
        .max(0);
    if duration > 0 {
        position.min(duration)
    } else {
        position
    }
}

/// An unchanged SMTC sample is not a fresh position. Preserve the monotonic
/// estimate through pause/resume; reset on new metadata awaiting a new timeline.
pub fn reconcile_position(estimated: i64, authoritative: Option<i64>, track_changed: bool) -> i64 {
    authoritative
        .unwrap_or(if track_changed { 0 } else { estimated })
        .max(0)
}

#[derive(Clone)]
pub struct Clock {
    pub playback: Playback,
    pub received: Instant,
}
impl Clock {
    pub fn snapshot(&self) -> Playback {
        let mut p = self.playback.clone();
        p.position_ms = extrapolate(
            p.position_ms,
            self.received.elapsed().as_millis() as i64,
            p.duration_ms,
            p.status == PlaybackStatus::Playing && p.timeline_available,
            p.playback_rate,
        );
        p.snapshot_at_ms = crate::model::now_ms();
        p
    }
}

/// Explicit preference wins when present; otherwise retain a playing session before
/// consulting the OS hint. A paused OS-current browser never displaces a player.
pub fn arbitrate(
    sessions: &[Playback],
    active: Option<&str>,
    preferred: &str,
    os_current: Option<&str>,
) -> Option<usize> {
    let valid = |p: &&Playback| !p.title.trim().is_empty();
    if !preferred.is_empty() {
        if let Some(i) = sessions.iter().position(|p| {
            !p.title.is_empty()
                && (p.source_id == preferred || p.source_app.eq_ignore_ascii_case(preferred))
        }) {
            return Some(i);
        }
    }
    let mut candidates: Vec<_> = sessions
        .iter()
        .enumerate()
        .filter(|(_, p)| valid(p))
        .collect();
    candidates.sort_by_key(|(_, p)| {
        std::cmp::Reverse((
            p.status == PlaybackStatus::Playing,
            Some(p.source_id.as_str()) == active,
            Some(p.source_id.as_str()) == os_current,
            p.status == PlaybackStatus::Paused,
        ))
    });
    candidates.first().map(|(i, _)| *i)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(id: &str, status: PlaybackStatus) -> Playback {
        Playback {
            source_id: id.into(),
            title: "song".into(),
            status,
            ..Default::default()
        }
    }
    #[test]
    fn clock_pause_rate_and_bounds() {
        assert_eq!(extrapolate(1000, 2000, 9000, true, 1.0), 3000);
        assert_eq!(extrapolate(1000, 2000, 9000, false, 1.0), 1000);
        assert_eq!(extrapolate(1000, 2000, 9000, true, 2.0), 5000);
        assert_eq!(extrapolate(1000, 2000, 2000, true, 1.0), 2000);
        assert_eq!(extrapolate(-500, 0, 0, true, 1.0), 0);
    }
    #[test]
    fn paused_browser_does_not_steal_playing_session() {
        let s = [
            p("browser", PlaybackStatus::Paused),
            p("spotify", PlaybackStatus::Playing),
        ];
        assert_eq!(arbitrate(&s, Some("spotify"), "", Some("browser")), Some(1));
        assert_eq!(arbitrate(&s, Some("spotify"), "browser", None), Some(0));
    }
    #[test]
    fn switching_and_disappearing_sessions() {
        let s = [
            p("netease", PlaybackStatus::Playing),
            p("spotify", PlaybackStatus::Playing),
        ];
        assert_eq!(arbitrate(&s, Some("spotify"), "", Some("netease")), Some(1));
        assert_eq!(arbitrate(&s[..1], Some("spotify"), "", None), Some(0));
        assert_eq!(arbitrate(&[], None, "", None), None);
    }
    #[test]
    fn seek_replaces_clock_and_resume_starts_here() {
        let mut c = Clock {
            playback: Playback {
                position_ms: 60000,
                status: PlaybackStatus::Paused,
                playback_rate: 1.0,
                ..Default::default()
            },
            received: Instant::now(),
        };
        assert_eq!(c.snapshot().position_ms, 60000);
        c.playback.position_ms = 12000;
        assert_eq!(c.snapshot().position_ms, 12000);
    }
    #[test]
    fn stale_samples_pause_resume_seek_and_new_track() {
        assert_eq!(reconcile_position(30000, None, false), 30000);
        let paused = extrapolate(30000, 20000, 100000, false, 1.);
        assert_eq!(reconcile_position(paused, None, false), 30000);
        assert_eq!(
            reconcile_position(extrapolate(paused, 1000, 100000, true, 1.), None, false),
            31000
        );
        assert_eq!(reconcile_position(31000, Some(75000), false), 75000);
        assert_eq!(reconcile_position(75000, Some(10000), false), 10000);
        assert_eq!(reconcile_position(75000, None, true), 0);
        assert_eq!(reconcile_position(75000, Some(500), true), 500);
    }
}
