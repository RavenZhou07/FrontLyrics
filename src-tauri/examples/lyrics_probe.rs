use lyricglass_lib::{
    lyrics::providers::{LrclibProvider, LyricsProvider},
    model::{LyricsKind, Playback},
};
#[tokio::main]
async fn main() {
    // Public metadata from LRCLIB's documentation; no lyrics are printed or bundled.
    let track = Playback {
        title: "I Want to Live".into(),
        artists: vec!["Borislav Slavov".into()],
        album: "Baldur's Gate 3 (Original Game Soundtrack)".into(),
        duration_ms: 233000,
        ..Default::default()
    };
    match LrclibProvider::new().unwrap().lookup(&track).await {
        Ok(Some(l)) if l.kind != LyricsKind::Unavailable => println!(
            "provider={} kind={:?} lines={} confidence={}",
            l.provider,
            l.kind,
            l.lines.len(),
            l.confidence
        ),
        result => {
            eprintln!("Live lookup failed: {result:?}");
            std::process::exit(1);
        }
    }
}
