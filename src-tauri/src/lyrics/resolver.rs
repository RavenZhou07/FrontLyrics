use super::{
    cache::Cache,
    providers::{LocalProvider, LyricsProvider},
};
use crate::model::{now_ms, Lyrics, LyricsKind, Playback};
use std::path::PathBuf;
pub struct Resolver {
    pub local: LocalProvider,
    pub cache: Cache,
    pub providers: Vec<Box<dyn LyricsProvider>>,
}
impl Resolver {
    pub fn new(root: PathBuf, providers: Vec<Box<dyn LyricsProvider>>) -> Self {
        Self {
            local: LocalProvider {
                root: root.join("local-lyrics"),
            },
            cache: Cache::new(root.join("cache")),
            providers,
        }
    }
    pub async fn resolve(&self, track: &Playback, refresh: bool) -> Lyrics {
        let mut failures = vec![];
        match self.local.lookup(track).await {
            Ok(Some(result)) => return result,
            Err(e) => failures.push(e),
            _ => {}
        }
        if !refresh {
            if let Some(result) = self.cache.get(&track.track_key, now_ms()) {
                return result;
            }
        }
        let mut best: Option<Lyrics> = None;
        for provider in &self.providers {
            match provider.lookup(track).await {
                Ok(Some(result)) if result.kind != LyricsKind::Unavailable => {
                    if best.as_ref().is_none_or(|v| {
                        result.kind > v.kind
                            || (result.kind == v.kind && result.confidence > v.confidence)
                    }) {
                        best = Some(result);
                    }
                }
                Err(error) => {
                    tracing::warn!(event = "provider_failure", provider = provider.name(), %error);
                    failures.push(format!("{}: {error}", provider.name()));
                }
                _ => {}
            }
        }
        let result = best.unwrap_or_else(|| {
            Lyrics::unavailable(if failures.is_empty() {
                "未找到可信匹配。可以导入本地歌词。".into()
            } else {
                failures.join("; ")
            })
        });
        // Network errors are transient, not a negative match. Do not cache them.
        if failures.is_empty() || result.kind != LyricsKind::Unavailable {
            if let Err(error) = self.cache.put(&track.track_key, &result, now_ms()) {
                tracing::warn!(event = "cache_write_failure", %error);
            }
        }
        tracing::info!(event = "lyrics_resolved", provider = %result.provider, kind = ?result.kind, confidence = result.confidence);
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    struct Fake {
        kind: LyricsKind,
        fail: bool,
    }
    #[async_trait]
    impl LyricsProvider for Fake {
        fn name(&self) -> &'static str {
            "test"
        }
        async fn lookup(&self, _: &Playback) -> Result<Option<Lyrics>, String> {
            if self.fail {
                return Err("offline".into());
            }
            let mut l = Lyrics::unavailable("test");
            l.kind = self.kind.clone();
            Ok(Some(l))
        }
    }
    fn track() -> Playback {
        Playback {
            track_key: "a".repeat(64),
            ..Default::default()
        }
    }
    #[tokio::test]
    async fn failure_fallback_quality_and_local_override() {
        let d = tempfile::tempdir().unwrap();
        let r = Resolver::new(
            d.path().into(),
            vec![
                Box::new(Fake {
                    kind: LyricsKind::WordTimed,
                    fail: true,
                }),
                Box::new(Fake {
                    kind: LyricsKind::Plain,
                    fail: false,
                }),
                Box::new(Fake {
                    kind: LyricsKind::Synced,
                    fail: false,
                }),
            ],
        );
        assert_eq!(r.resolve(&track(), false).await.kind, LyricsKind::Synced);
        r.local.save(&track(), "my own text").unwrap();
        assert_eq!(r.resolve(&track(), false).await.kind, LyricsKind::Plain);
    }
    #[tokio::test]
    async fn failures_not_negative_cached() {
        let d = tempfile::tempdir().unwrap();
        let r = Resolver::new(
            d.path().into(),
            vec![Box::new(Fake {
                kind: LyricsKind::Plain,
                fail: true,
            })],
        );
        assert_eq!(
            r.resolve(&track(), false).await.kind,
            LyricsKind::Unavailable
        );
        assert!(r.cache.get(&track().track_key, now_ms()).is_none());
    }
}
