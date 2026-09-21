use crate::lyrics::parser::{estimate_syllables_for_line, LyricLine, LyricWord};
use regex::Regex;
use std::sync::OnceLock;

static HTML_TAG_RE: OnceLock<Regex> = OnceLock::new();
static CUE_TIME_RE: OnceLock<Regex> = OnceLock::new();

fn get_html_tag_re() -> &'static Regex {
    HTML_TAG_RE.get_or_init(|| Regex::new(r"<[^>]+>").unwrap())
}

fn get_cue_time_re() -> &'static Regex {
    CUE_TIME_RE.get_or_init(|| {
        Regex::new(r"(?:(\d{1,2}):)?(\d{2}):(\d{2})[.,](\d{3})\s*-->\s*(?:(\d{1,2}):)?(\d{2}):(\d{2})[.,](\d{3})").unwrap()
    })
}

/// Nettoie les balises WebVTT (ex: <c.color>, <v Speaker>, <i>, entités HTML)
pub fn clean_subtitle_text(text: &str) -> String {
    let tag_re = get_html_tag_re();
    let stripped = tag_re.replace_all(text, "");
    stripped
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("\r", " ")
        .replace("\n", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parse un horodatage (hh:mm:ss.mmm ou mm:ss.mmm) en millisecondes
pub fn parse_timestamp_ms(hours: Option<&str>, minutes: &str, seconds: &str, millis: &str) -> i64 {
    let h: i64 = hours.and_then(|v| v.parse().ok()).unwrap_or(0);
    let m: i64 = minutes.parse().unwrap_or(0);
    let s: i64 = seconds.parse().unwrap_or(0);
    let ms: i64 = millis.parse().unwrap_or(0);

    h * 3600_000 + m * 60_000 + s * 1000 + ms
}

/// Parse du texte au format WebVTT ou SubRip (.srt)
pub fn parse_vtt_or_srt(content: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    let time_re = get_cue_time_re();

    let mut current_start_ms: Option<i64> = None;
    let mut current_end_ms: Option<i64> = None;
    let mut current_text_parts: Vec<String> = Vec::new();

    let flush_cue = |lines: &mut Vec<LyricLine>,
                     start: Option<i64>,
                     end: Option<i64>,
                     parts: &mut Vec<String>| {
        if let (Some(start_ms), Some(end_ms)) = (start, end) {
            let combined = parts.join(" ");
            let cleaned = clean_subtitle_text(&combined);
            if !cleaned.is_empty() {
                let words = estimate_syllables_for_line(&cleaned, start_ms, end_ms);
                lines.push(LyricLine {
                    start_time_ms: start_ms,
                    end_time_ms: Some(end_ms),
                    text: cleaned,
                    words,
                });
            }
        }
        parts.clear();
    };

    for raw_line in content.lines() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            flush_cue(&mut lines, current_start_ms, current_end_ms, &mut current_text_parts);
            current_start_ms = None;
            current_end_ms = None;
            continue;
        }

        // Ignorer l'en-tête WEBVTT et les notes
        if trimmed.starts_with("WEBVTT") || trimmed.starts_with("NOTE") {
            continue;
        }

        // Vérifier si c'est une ligne d'horodatage : 00:00:10.500 --> 00:00:14.200
        if let Some(caps) = time_re.captures(trimmed) {
            flush_cue(&mut lines, current_start_ms, current_end_ms, &mut current_text_parts);

            let start = parse_timestamp_ms(
                caps.get(1).map(|m| m.as_str()),
                caps.get(2).map(|m| m.as_str()).unwrap_or("0"),
                caps.get(3).map(|m| m.as_str()).unwrap_or("0"),
                caps.get(4).map(|m| m.as_str()).unwrap_or("0"),
            );

            let end = parse_timestamp_ms(
                caps.get(5).map(|m| m.as_str()),
                caps.get(6).map(|m| m.as_str()).unwrap_or("0"),
                caps.get(7).map(|m| m.as_str()).unwrap_or("0"),
                caps.get(8).map(|m| m.as_str()).unwrap_or("0"),
            );

            current_start_ms = Some(start);
            current_end_ms = Some(end);
            continue;
        }

        // Si c'est juste un index numérique SRT (ex: "1", "2"), l'ignorer
        if trimmed.chars().all(|c| c.is_ascii_digit()) && current_start_ms.is_none() {
            continue;
        }

        if current_start_ms.is_some() {
            current_text_parts.push(trimmed.to_string());
        }
    }

    flush_cue(&mut lines, current_start_ms, current_end_ms, &mut current_text_parts);

    // Dédupliquer ou fusionner les lignes consécutives avec le même texte
    lines.sort_by_key(|l| l.start_time_ms);
    lines
}

/// Parse le format natif JSON3 de YouTube
pub fn parse_youtube_json3(json_content: &str) -> Option<Vec<LyricLine>> {
    let parsed: serde_json::Value = serde_json::from_str(json_content).ok()?;
    let events = parsed.get("events")?.as_array()?;

    let mut lines = Vec::new();

    for ev in events {
        let start_ms = ev.get("tStartMs").and_then(|v| v.as_i64())?;
        let duration_ms = ev.get("dDurationMs").and_then(|v| v.as_i64()).unwrap_or(2500);
        let end_ms = start_ms + duration_ms;

        let segs = match ev.get("segs").and_then(|v| v.as_array()) {
            Some(s) => s,
            None => continue,
        };

        let mut full_text = String::new();
        let mut words = Vec::new();

        for seg in segs {
            let utf8 = seg.get("utf8").and_then(|v| v.as_str()).unwrap_or("");
            if utf8.trim().is_empty() || utf8 == "\n" {
                continue;
            }

            let word_offset_ms = seg.get("tOffsetMs").and_then(|v| v.as_i64()).unwrap_or(0);
            let seg_start = start_ms + word_offset_ms;

            let cleaned_part = clean_subtitle_text(utf8);
            if !cleaned_part.is_empty() {
                full_text.push_str(&cleaned_part);
                full_text.push(' ');

                words.push(LyricWord {
                    text: cleaned_part,
                    start_time_ms: seg_start,
                    end_time_ms: seg_start + 400, // Durée indicative, recalculée ci-dessous
                });
            }
        }

        let cleaned_full = clean_subtitle_text(&full_text);
        if !cleaned_full.is_empty() {
            // Si pas de mots individuels horodatés dans segs, estimer les syllabes
            let final_words = if words.len() <= 1 {
                estimate_syllables_for_line(&cleaned_full, start_ms, end_ms)
            } else {
                // Ajuster les end_time_ms des mots consécutifs
                for i in 0..words.len() {
                    let next_start = if i + 1 < words.len() {
                        words[i + 1].start_time_ms
                    } else {
                        end_ms
                    };
                    words[i].end_time_ms = next_start.max(words[i].start_time_ms + 100);
                }
                words
            };

            lines.push(LyricLine {
                start_time_ms: start_ms,
                end_time_ms: Some(end_ms),
                text: cleaned_full,
                words: final_words,
            });
        }
    }

    lines.sort_by_key(|l| l.start_time_ms);
    if lines.is_empty() {
        None
    } else {
        Some(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vtt_basic() {
        let vtt = r#"WEBVTT

1
00:00:01.500 --> 00:00:04.000
Welcome to this English podcast.

2
00:00:04.200 --> 00:00:07.800
Today we are learning new vocabulary.
"#;
        let lines = parse_vtt_or_srt(vtt);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start_time_ms, 1500);
        assert_eq!(lines[0].end_time_ms, Some(4000));
        assert_eq!(lines[0].text, "Welcome to this English podcast.");
        assert!(!lines[0].words.is_empty());

        assert_eq!(lines[1].start_time_ms, 4200);
        assert_eq!(lines[1].end_time_ms, Some(7800));
        assert_eq!(lines[1].text, "Today we are learning new vocabulary.");
    }

    #[test]
    fn test_parse_srt_with_html_tags() {
        let srt = r##"1
00:01:10,250 --> 00:01:15,000
<font color="#ffff00"><i>Never give up</i></font> on your dreams.
"##;
        let lines = parse_vtt_or_srt(srt);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].start_time_ms, 70250);
        assert_eq!(lines[0].end_time_ms, Some(75000));
        assert_eq!(lines[0].text, "Never give up on your dreams.");
    }

    #[test]
    fn test_parse_youtube_json3() {
        let json3 = r#"{
            "events": [
                {
                    "tStartMs": 2000,
                    "dDurationMs": 3000,
                    "segs": [
                        { "utf8": "Hello ", "tOffsetMs": 0 },
                        { "utf8": "world", "tOffsetMs": 500 }
                    ]
                }
            ]
        }"#;

        let lines = parse_youtube_json3(json3).expect("Doit parser le json3");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].start_time_ms, 2000);
        assert_eq!(lines[0].end_time_ms, Some(5000));
        assert_eq!(lines[0].text, "Hello world");
        assert_eq!(lines[0].words.len(), 2);
        assert_eq!(lines[0].words[0].text, "Hello");
        assert_eq!(lines[0].words[1].text, "world");
    }
}
