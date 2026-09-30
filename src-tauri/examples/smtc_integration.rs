//! Pair with tools/smtc-fixture.ps1; this verifies the production event adapter and
//! network provider against the OS. It is not a Spotify/NetEase compatibility claim.
use lyricglass_lib::{
    lyrics::providers::{LrclibProvider, LyricsProvider},
    model::{LyricsKind, PlaybackStatus},
};
use std::time::{Duration, Instant};
fn main() {
    #[cfg(windows)]
    {
        let started = Instant::now();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let provider = LrclibProvider::new().unwrap();
        let mut resolved = false;
        let mut paused: Option<i64> = None;
        let mut resumed = false;
        let mut sought = false;
        let mut previous = 0;
        let result = lyricglass_lib::platform::windows::watch(move |sessions, _| {
            if started.elapsed() > Duration::from_secs(75) {
                eprintln!("Timed out waiting for synthetic session, pause/resume, or seek");
                std::process::exit(1);
            }
            let Some(p) = sessions
                .iter()
                .find(|p| p.source_id == "powershell.exe" && p.title == "I Want to Live")
            else {
                return;
            };
            assert!(p.timeline_available);
            assert_eq!(p.duration_ms, 233000);
            if !resolved {
                let lyrics = runtime
                    .block_on(provider.lookup(p))
                    .expect("live LRCLIB lookup")
                    .expect("trusted lyrics");
                assert_eq!(lyrics.kind, LyricsKind::Synced);
                assert!(lyrics.lines.len() > 10);
                println!(
                    "SMTC → normalized metadata → LRCLIB: synced, {} lines, confidence {}",
                    lyrics.lines.len(),
                    lyrics.confidence
                );
                resolved = true;
            }
            if p.status == PlaybackStatus::Paused {
                if let Some(position) = paused {
                    assert!(
                        (p.position_ms - position).abs() < 500,
                        "paused position drifted"
                    );
                } else {
                    paused = Some(p.position_ms);
                    println!("Pause: frozen authoritative position");
                }
            } else if paused.is_some() && p.status == PlaybackStatus::Playing && !resumed {
                resumed = true;
                println!("Resume: playing again");
            }
            if previous > 0 && p.position_ms - previous > 15000 {
                sought = true;
                println!("Seek: authoritative position correction");
            }
            previous = p.position_ms;
            if resolved && resumed && sought {
                println!("PASS: native SMTC, network resolution, pause, resume and seek");
                std::process::exit(0);
            }
        });
        if let Err(error) = result {
            eprintln!("SMTC failed: {error}");
            std::process::exit(1);
        }
    }
    #[cfg(not(windows))]
    eprintln!("Windows required");
}
