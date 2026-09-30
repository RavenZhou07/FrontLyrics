use crate::{
    model::{Lyrics, LyricsKind},
    settings::write_json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
const POSITIVE_TTL: i64 = 30 * 86400000;
const NEGATIVE_TTL: i64 = 10 * 60000;
#[derive(Serialize, Deserialize)]
struct Entry {
    version: u32,
    saved_at_ms: i64,
    lyrics: Lyrics,
}
pub struct Cache {
    root: PathBuf,
}
impl Cache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn path(&self, key: &str) -> Option<PathBuf> {
        (key.len() == 64 && key.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| self.root.join(format!("{key}.json")))
    }
    pub fn get(&self, key: &str, now: i64) -> Option<Lyrics> {
        let e: Entry = serde_json::from_slice(&std::fs::read(self.path(key)?).ok()?).ok()?;
        let ttl = if e.lyrics.kind == LyricsKind::Unavailable {
            NEGATIVE_TTL
        } else {
            POSITIVE_TTL
        };
        if e.version != 1 || now < e.saved_at_ms || now - e.saved_at_ms >= ttl {
            None
        } else {
            Some(e.lyrics)
        }
    }
    pub fn put(&self, key: &str, lyrics: &Lyrics, now: i64) -> Result<(), String> {
        let path = self.path(key).ok_or("Invalid cache key")?;
        write_json(
            &path,
            &Entry {
                version: 1,
                saved_at_ms: now,
                lyrics: lyrics.clone(),
            },
        )
    }
    pub fn remove(&self, key: &str) {
        if let Some(path) = self.path(key) {
            let _ = std::fs::remove_file(path);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expiry_negative_shorter_corruption_and_traversal() {
        let d = tempfile::tempdir().unwrap();
        let c = Cache::new(d.path().into());
        let key = "a".repeat(64);
        c.put(&key, &Lyrics::unavailable("missing"), 100).unwrap();
        assert!(c.get(&key, 200).is_some());
        assert!(c.get(&key, 100 + NEGATIVE_TTL).is_none());
        let mut l = Lyrics::unavailable("");
        l.kind = LyricsKind::Instrumental;
        c.put(&key, &l, 100).unwrap();
        assert!(c.get(&key, 100 + NEGATIVE_TTL).is_some());
        assert!(c.get(&key, 99).is_none());
        assert!(c.get("../settings", 200).is_none());
        std::fs::write(d.path().join(format!("{key}.json")), "{bad}").unwrap();
        assert!(c.get(&key, 200).is_none());
    }
}
