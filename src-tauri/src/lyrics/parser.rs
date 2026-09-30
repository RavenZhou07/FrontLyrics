use crate::model::{LyricLine, Lyrics, LyricsKind, Word};
use regex::Regex;
use std::sync::LazyLock;
static STAMP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[(\d{1,3}):([0-5]\d)(?:[.:](\d{1,3}))?\]").unwrap());
static WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<(\d{1,3}):([0-5]\d)(?:[.:](\d{1,3}))?>").unwrap());
static OFFSET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\[offset:([+-]?\d+)\]").unwrap());
static YRC_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[(\d+),(\d+)\]").unwrap());
static YRC_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\((\d+),(\d+),\d+\)").unwrap());
fn time(c: &regex::Captures<'_>) -> i64 {
    let fraction = c
        .get(3)
        .map(|v| format!("{:0<3}", v.as_str()).parse::<i64>().unwrap_or(0))
        .unwrap_or(0);
    c[1].parse::<i64>().unwrap_or(0) * 60000 + c[2].parse::<i64>().unwrap_or(0) * 1000 + fraction
}
fn line(start: i64, text: String) -> LyricLine {
    LyricLine {
        start_ms: start,
        end_ms: 0,
        text,
        translation: None,
        words: vec![],
    }
}
pub fn parse(text: &str, duration: i64, provider: &str) -> Lyrics {
    let text = text.trim_start_matches('\u{feff}');
    let offset = OFFSET
        .captures(text)
        .and_then(|c| c[1].parse::<i64>().ok())
        .unwrap_or(0)
        .clamp(-600000, 600000);
    let mut lines = vec![];
    let mut plain = vec![];
    for raw in text.lines().map(str::trim).filter(|v| !v.is_empty()) {
        if let Some(c) = YRC_LINE.captures(raw) {
            let start = c[1].parse::<i64>().unwrap_or(0);
            let length = c[2].parse::<i64>().unwrap_or(0);
            if start > 86400000 || length > 86400000 {
                continue;
            }
            let captures: Vec<_> = YRC_WORD.captures_iter(raw).collect();
            let mut words = vec![];
            for (i, w) in captures.iter().enumerate() {
                let s = w[1].parse::<i64>().unwrap_or(-1);
                let d = w[2].parse::<i64>().unwrap_or(0);
                let content = &raw[w.get(0).unwrap().end()
                    ..captures
                        .get(i + 1)
                        .map(|v| v.get(0).unwrap().start())
                        .unwrap_or(raw.len())];
                if s >= start
                    && d > 0
                    && s.saturating_add(d) <= start + length + 5
                    && !content.is_empty()
                {
                    words.push(Word {
                        start_ms: (s - offset).max(0),
                        end_ms: (s + d - offset).max(0),
                        text: content.into(),
                    });
                }
            }
            if !words.is_empty() {
                let mut l = line(
                    (start - offset).max(0),
                    words.iter().map(|w| w.text.as_str()).collect(),
                );
                l.end_ms = (start + length - offset).max(0);
                l.words = words;
                lines.push(l);
            }
            continue;
        }
        let stamps: Vec<_> = STAMP.captures_iter(raw).collect();
        if stamps.is_empty() {
            if !raw.starts_with('[') {
                plain.push(raw);
            }
            continue;
        }
        let content = &raw[stamps.last().unwrap().get(0).unwrap().end()..];
        for stamp in &stamps {
            let mut l = line(
                (time(stamp) - offset).max(0),
                WORD.replace_all(content, "").trim().into(),
            );
            let words: Vec<_> = WORD.captures_iter(content).collect();
            // Enhanced LRC supplies absolute word boundaries. A closing timestamp is
            // required for the last word; never invent word durations from text length.
            for pair in words.windows(2) {
                let start = time(&pair[0]) - offset;
                let end = time(&pair[1]) - offset;
                let t = &content[pair[0].get(0).unwrap().end()..pair[1].get(0).unwrap().start()];
                if start >= l.start_ms && end > start && !t.is_empty() {
                    l.words.push(Word {
                        start_ms: start.max(0),
                        end_ms: end.max(0),
                        text: t.into(),
                    });
                }
            }
            if l.words
                .iter()
                .map(|w| w.text.as_str())
                .collect::<String>()
                .trim()
                != l.text.trim()
            {
                l.words.clear();
            }
            lines.push(l);
        }
    }
    lines.sort_by_key(|l| l.start_ms);
    let mut merged: Vec<LyricLine> = vec![];
    for l in lines {
        if let Some(previous) = merged.last_mut().filter(|p| p.start_ms == l.start_ms) {
            if previous.text.is_empty() {
                *previous = l;
            } else if previous.text != l.text && !l.text.is_empty() {
                match &mut previous.translation {
                    Some(t) if t != &l.text => {
                        t.push_str(" / ");
                        t.push_str(&l.text);
                    }
                    None => previous.translation = Some(l.text),
                    _ => {}
                }
            }
        } else {
            merged.push(l);
        }
    }
    for i in 0..merged.len() {
        let next =
            merged
                .get(i + 1)
                .map(|v| v.start_ms)
                .unwrap_or(if duration > merged[i].start_ms {
                    duration
                } else {
                    merged[i].start_ms + 10000
                });
        let l = &mut merged[i];
        l.end_ms = if l.end_ms > l.start_ms {
            l.end_ms.min(next)
        } else {
            next
        };
        if l.words.iter().any(|w| w.end_ms > l.end_ms)
            || l.words.windows(2).any(|w| w[0].end_ms > w[1].start_ms)
        {
            l.words.clear();
        }
    }
    let kind = if merged.iter().all(|l| l.text.is_empty()) {
        if plain.is_empty() {
            LyricsKind::Unavailable
        } else {
            LyricsKind::Plain
        }
    } else if merged.iter().any(|l| !l.words.is_empty()) {
        LyricsKind::WordTimed
    } else {
        LyricsKind::Synced
    };
    Lyrics {
        kind,
        lines: merged,
        plain: if plain.is_empty() {
            None
        } else {
            Some(plain.join("\n"))
        },
        provider: provider.into(),
        confidence: 1.0,
        note: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timestamps_utf8_duplicates_and_translation() {
        let l = parse(
            "[ar:test]\n[00:01.2][00:03.045]你好\n[00:01.200]Hello\n[00:01.200]你好\n[00:04]",
            5000,
            "local",
        );
        assert_eq!(l.lines.len(), 3);
        assert_eq!(l.lines[0].start_ms, 1200);
        assert_eq!(l.lines[0].translation.as_deref(), Some("Hello"));
        assert_eq!(l.lines[0].end_ms, 3045);
        assert!(l.lines[2].text.is_empty());
    }
    #[test]
    fn malformed_empty_plain_and_offsets() {
        assert_eq!(
            parse("[99:99]bad\n[ti:foo]", 0, "test").kind,
            LyricsKind::Unavailable
        );
        assert_eq!(parse("", 0, "test").kind, LyricsKind::Unavailable);
        assert_eq!(parse("你好\n世界", 0, "test").kind, LyricsKind::Plain);
        assert_eq!(
            parse("[offset:200]\n[00:01]x", 0, "test").lines[0].start_ms,
            800
        );
    }
    #[test]
    fn enhanced_requires_real_word_boundaries() {
        let l = parse("[00:01]<00:01>你<00:01.5>好<00:02>", 3000, "test");
        assert_eq!(l.kind, LyricsKind::WordTimed);
        assert_eq!(l.lines[0].words[1].end_ms, 2000);
        assert!(parse("[00:01]<00:01>你好", 3000, "test").lines[0]
            .words
            .is_empty());
    }
    #[test]
    fn yrc_uses_absolute_segment_times() {
        let l = parse("[1000,1000](1000,400,0)你(1400,600,0)好", 3000, "local");
        assert_eq!(l.kind, LyricsKind::WordTimed);
        assert_eq!(l.lines[0].text, "你好");
        assert_eq!(l.lines[0].words[1].start_ms, 1400);
    }
}
