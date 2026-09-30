use crate::model::Playback;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

/// Preserve edition qualifiers (live, remix, remaster), CJK and artist punctuation.
pub fn normalize(s: &str) -> String {
    s.nfkc()
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn canonical_key(p: &Playback) -> String {
    let mut artists: Vec<_> = p.artists.iter().map(|s| normalize(s)).collect();
    artists.sort();
    let signature = serde_json::to_vec(&(
        normalize(&p.title),
        artists,
        normalize(&p.album),
        p.duration_ms.max(0) / 1000,
    ))
    .unwrap();
    format!("{:x}", Sha256::digest(signature))
}
pub fn confidence(
    p: &Playback,
    title: &str,
    artist: &str,
    album: &str,
    duration_ms: i64,
) -> Option<f64> {
    if normalize(&p.title) != normalize(title) || p.artists.is_empty() {
        return None;
    }
    let a = normalize(artist);
    let exact_artist = p
        .artists
        .iter()
        .any(|v| !v.trim().is_empty() && normalize(v) == a)
        || normalize(&p.artists.join(", ")) == a
        || normalize(&p.artists.join("; ")) == a;
    if !exact_artist {
        return None;
    }
    let has_duration = p.duration_ms > 0 && duration_ms > 0;
    if has_duration && (p.duration_ms - duration_ms).abs() > 2500 {
        return None;
    }
    let album_matches = !p.album.is_empty() && normalize(&p.album) == normalize(album);
    if !has_duration && !album_matches {
        return None;
    }
    Some(if album_matches && has_duration {
        1.0
    } else if has_duration {
        0.94
    } else {
        0.88
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn track() -> Playback {
        Playback {
            title: "夜曲".into(),
            artists: vec!["周杰伦".into()],
            album: "十一月的萧邦".into(),
            duration_ms: 226000,
            ..Default::default()
        }
    }
    #[test]
    fn conservative_normalization() {
        assert_eq!(normalize("  ＡＢＣ  Café  "), "abc café");
        assert_ne!(normalize("Song (Live)"), normalize("Song"));
    }
    #[test]
    fn reject_wrong_version_artist_and_duration() {
        let p = track();
        assert_eq!(
            confidence(&p, "夜曲", "周杰伦", "十一月的萧邦", 227000),
            Some(1.0)
        );
        assert_eq!(confidence(&p, "夜曲 (Live)", "周杰伦", "", 226000), None);
        assert_eq!(confidence(&p, "夜曲", "cover artist", "", 226000), None);
        assert_eq!(confidence(&p, "夜曲", "周杰伦", "", 220000), None);
        let p = Playback {
            duration_ms: 0,
            album: String::new(),
            ..p
        };
        assert_eq!(confidence(&p, "夜曲", "周杰伦", "", 0), None);
    }
    #[test]
    fn canonical_independent_of_player_and_case() {
        let a = track();
        let b = Playback {
            source_id: "other".into(),
            title: " 夜曲 ".into(),
            ..a.clone()
        };
        assert_eq!(canonical_key(&a), canonical_key(&b));
        assert_ne!(
            canonical_key(&a),
            canonical_key(&Playback {
                duration_ms: 90000,
                ..a
            })
        );
    }
}
