use super::{matching::confidence, parser::parse};
use crate::model::{Lyrics, LyricsKind, Playback};
use async_trait::async_trait;
use serde::Deserialize;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[async_trait]
pub trait LyricsProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn lookup(&self, track: &Playback) -> Result<Option<Lyrics>, String>;
}

pub struct LocalProvider {
    pub root: PathBuf,
}
impl LocalProvider {
    pub fn path(&self, key: &str) -> Result<PathBuf, String> {
        if key.len() != 64 || !key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid track identity".into());
        }
        Ok(self.root.join(format!("{key}.lrc")))
    }
    pub fn save(&self, track: &Playback, text: &str) -> Result<(), String> {
        if text.len() > 2_000_000 {
            return Err("歌词文件不能超过 2 MB".into());
        }
        let result = parse(text, track.duration_ms, self.name());
        if result.kind == LyricsKind::Unavailable {
            return Err("未找到有效的 LRC、YRC 或纯文本歌词".into());
        }
        std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        std::fs::write(self.path(&track.track_key)?, text).map_err(|e| e.to_string())
    }
}
#[async_trait]
impl LyricsProvider for LocalProvider {
    fn name(&self) -> &'static str {
        "Local"
    }
    async fn lookup(&self, track: &Playback) -> Result<Option<Lyrics>, String> {
        let path = self.path(&track.track_key)?;
        match std::fs::read_to_string(path) {
            Ok(text) if text.len() <= 2_000_000 => {
                let result = parse(&text, track.duration_ms, self.name());
                if result.kind == LyricsKind::Unavailable {
                    Err("本地歌词无效".into())
                } else {
                    Ok(Some(result))
                }
            }
            Ok(_) => Err("本地歌词超过 2 MB".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

pub struct LrclibProvider {
    client: reqwest::Client,
    next_request: Mutex<Instant>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    track_name: String,
    artist_name: String,
    #[serde(default)]
    album_name: String,
    duration: f64,
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}
impl LrclibProvider {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent("LyricGlass/0.1.0 (Windows desktop lyrics)")
                .timeout(Duration::from_secs(12))
                .build()
                .map_err(|e| e.to_string())?,
            next_request: Mutex::new(Instant::now()),
        })
    }
    async fn request(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<Option<Vec<u8>>, String> {
        let mut next = self.next_request.lock().await;
        let delay = next.saturating_duration_since(Instant::now());
        // A long Retry-After is respected across subsequent lookups without hanging the worker.
        if delay > Duration::from_secs(2) {
            return Err("LRCLIB 请求限流，请稍后重试".into());
        }
        tokio::time::sleep(delay).await;
        *next = Instant::now() + Duration::from_millis(350);
        let mut response = self
            .client
            .get(format!("https://lrclib.net/api/{path}"))
            .query(query)
            .send()
            .await
            .map_err(|e| e.without_url().to_string())?;
        if response.status().as_u16() == 404 {
            return Ok(None);
        }
        if response.status().as_u16() == 429 {
            let seconds = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);
            *next = Instant::now() + Duration::from_secs(seconds.max(1));
            return Err("LRCLIB 请求限流，请稍后重试".into());
        }
        response
            .error_for_status_ref()
            .map_err(|e| e.without_url().to_string())?;
        let mut data = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| e.without_url().to_string())?
        {
            if data.len() + chunk.len() > 2_000_000 {
                return Err("LRCLIB response too large".into());
            }
            data.extend_from_slice(&chunk);
        }
        Ok(Some(data))
    }
    fn convert(track: &Playback, record: Record) -> Option<Lyrics> {
        let score = confidence(
            track,
            &record.track_name,
            &record.artist_name,
            &record.album_name,
            (record.duration * 1000.) as i64,
        )?;
        if record.instrumental {
            return Some(Lyrics {
                kind: LyricsKind::Instrumental,
                lines: vec![],
                plain: None,
                provider: "LRCLIB".into(),
                confidence: score,
                note: Some("纯音乐".into()),
            });
        }
        let mut result = parse(
            record.synced_lyrics.as_deref().unwrap_or(""),
            track.duration_ms,
            "LRCLIB",
        );
        if result.kind == LyricsKind::Unavailable {
            result = parse(
                record.plain_lyrics.as_deref().unwrap_or(""),
                track.duration_ms,
                "LRCLIB",
            );
        }
        result.confidence = score;
        (result.kind != LyricsKind::Unavailable).then_some(result)
    }
}
#[async_trait]
impl LyricsProvider for LrclibProvider {
    fn name(&self) -> &'static str {
        "LRCLIB"
    }
    async fn lookup(&self, track: &Playback) -> Result<Option<Lyrics>, String> {
        if track.title.trim().is_empty() || track.artists.iter().all(|s| s.trim().is_empty()) {
            return Ok(None);
        }
        let mut query = vec![
            ("track_name".into(), track.title.clone()),
            ("artist_name".into(), track.artists.join(", ")),
        ];
        if !track.album.is_empty() {
            query.push(("album_name".into(), track.album.clone()));
        }
        if (1000..=3600000).contains(&track.duration_ms) {
            query.push(("duration".into(), (track.duration_ms / 1000).to_string()));
        }
        let mut candidates = vec![];
        if let Some(data) = self.request("get", &query).await? {
            let record: Record = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
            if let Some(result) = Self::convert(track, record) {
                if result.kind >= LyricsKind::Synced {
                    return Ok(Some(result));
                }
                candidates.push(result);
            }
        }
        // Search may recover an album spelling mismatch; every candidate still passes
        // the same strict title/artist/duration checks as an exact response.
        query.retain(|(k, _)| k == "track_name" || k == "artist_name");
        match self.request("search", &query).await {
            Ok(Some(data)) => {
                let records: Vec<Record> =
                    serde_json::from_slice(&data).map_err(|e| e.to_string())?;
                candidates.extend(records.into_iter().filter_map(|r| Self::convert(track, r)));
            }
            Err(e) if candidates.is_empty() => return Err(e),
            _ => {}
        }
        candidates.sort_by(|a, b| {
            b.kind
                .cmp(&a.kind)
                .then_with(|| b.confidence.total_cmp(&a.confidence))
        });
        Ok(candidates.into_iter().next())
    }
}

/// Deliberately isolated NetEase adapter boundary. SMTC has no dependable NetEase
/// song ID; an online adapter must first establish a strictly matched identity.
/// Local YRC is supported today without making startup depend on unofficial APIs.
pub mod netease {
    use crate::model::Lyrics;
    pub struct Payload {
        pub yrc: Option<String>,
        pub lrc: Option<String>,
        pub translation_lrc: Option<String>,
    }
    pub fn normalize(payload: Payload, duration_ms: i64) -> Lyrics {
        let mut lyrics = super::parse(payload.yrc.as_deref().unwrap_or(""), duration_ms, "NetEase");
        if lyrics.lines.is_empty() {
            lyrics = super::parse(payload.lrc.as_deref().unwrap_or(""), duration_ms, "NetEase");
        }
        if let Some(t) = payload.translation_lrc {
            let translation = super::parse(&t, duration_ms, "NetEase");
            for line in &mut lyrics.lines {
                line.translation = translation
                    .lines
                    .iter()
                    .find(|t| (t.start_ms - line.start_ms).abs() <= 100)
                    .map(|t| t.text.clone());
            }
        }
        lyrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn instrumental_and_plain() {
        let track = Playback {
            title: "Silence".into(),
            artists: vec!["Artist".into()],
            duration_ms: 100000,
            ..Default::default()
        };
        let record = Record {
            track_name: "Silence".into(),
            artist_name: "Artist".into(),
            album_name: "".into(),
            duration: 100.,
            instrumental: true,
            plain_lyrics: None,
            synced_lyrics: None,
        };
        assert_eq!(
            LrclibProvider::convert(&track, record).unwrap().kind,
            LyricsKind::Instrumental
        );
    }
    #[test]
    fn netease_translation_is_attached_by_time() {
        let result = netease::normalize(
            netease::Payload {
                yrc: Some("[1000,1000](1000,1000,0)你好".into()),
                lrc: None,
                translation_lrc: Some("[00:01]Hello".into()),
            },
            5000,
        );
        assert_eq!(result.lines[0].translation.as_deref(), Some("Hello"));
    }
}
