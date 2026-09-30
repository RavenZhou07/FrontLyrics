use serde::{Deserialize, Serialize};

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatus {
    Playing,
    Paused,
    #[default]
    Stopped,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playback {
    pub source_id: String,
    pub source_app: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_ms: i64,
    pub position_ms: i64,
    pub status: PlaybackStatus,
    pub playback_rate: f64,
    pub timeline_available: bool,
    pub artwork: Option<String>,
    pub snapshot_at_ms: i64,
    pub track_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub app: String,
    pub title: String,
    pub status: PlaybackStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub translation: Option<String>,
    pub words: Vec<Word>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum LyricsKind {
    Unavailable,
    Plain,
    Synced,
    WordTimed,
    Instrumental,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    pub kind: LyricsKind,
    pub lines: Vec<LyricLine>,
    pub plain: Option<String>,
    pub provider: String,
    pub confidence: f64,
    pub note: Option<String>,
}
impl Lyrics {
    pub fn unavailable(note: impl Into<String>) -> Self {
        Self {
            kind: LyricsKind::Unavailable,
            lines: vec![],
            plain: None,
            provider: String::new(),
            confidence: 0.0,
            note: Some(note.into()),
        }
    }
}
