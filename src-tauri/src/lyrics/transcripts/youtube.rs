use super::vtt_parser::{parse_vtt_or_srt, parse_youtube_json3};
use crate::lyrics::parser::LyricLine;
use regex::Regex;
use std::sync::OnceLock;

static VIDEO_ID_RE: OnceLock<Regex> = OnceLock::new();
static CAPTION_TRACKS_RE: OnceLock<Regex> = OnceLock::new();

fn get_video_id_re() -> &'static Regex {
    VIDEO_ID_RE.get_or_init(|| {
        Regex::new(r#"(?:v=|/v/|youtu\.be/|/embed/|/shorts/|/watch\?v=)([a-zA-Z0-9_-]{11})"#).unwrap()
    })
}

fn get_caption_tracks_re() -> &'static Regex {
    CAPTION_TRACKS_RE.get_or_init(|| {
        Regex::new(r#""captionTracks":\s*(\[[^\]]+\])"#).unwrap()
    })
}

#[derive(Debug, Clone)]
pub struct CaptionTrack {
    pub base_url: String,
    pub language_code: String,
    pub name: Option<String>,
    pub is_auto_generated: bool,
}

pub struct YouTubeTranscriptProvider {
    client: reqwest::Client,
}

impl YouTubeTranscriptProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Recherche et extrait la transcription anglaise d'une vidéo YouTube (via son ID ou son titre)
    pub async fn fetch_english_transcript(&self, query: &str) -> Option<Vec<LyricLine>> {
        let video_id = self.resolve_video_id(query).await?;
        self.fetch_transcript_by_id(&video_id).await
    }

    /// Extrait l'ID de la vidéo s'il est déjà présent ou effectue une recherche YouTube
    pub async fn resolve_video_id(&self, query: &str) -> Option<String> {
        let trimmed = query.trim();

        // 1. Détection directe si la chaîne contient un ID vidéo (ex: URL ou ID 11 caractères)
        if let Some(caps) = get_video_id_re().captures(trimmed) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
        }

        if trimmed.len() == 11 && trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Some(trimmed.to_string());
        }

        // 2. Recherche rapide sur YouTube via la page de résultats
        let search_url = format!(
            "https://www.youtube.com/results?search_query={}",
            urlencoding::encode(trimmed)
        );

        let resp = self
            .client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()?;

        // Cherche le premier "/watch?v=XXXXXXXXXXX"
        let re_match = Regex::new(r#"/watch\?v=([a-zA-Z0-9_-]{11})"#).ok()?;
        if let Some(caps) = re_match.captures(&resp) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
        }

        None
    }

    /// Télécharge et parse les sous-titres anglais pour un ID vidéo donné
    pub async fn fetch_transcript_by_id(&self, video_id: &str) -> Option<Vec<LyricLine>> {
        let watch_url = format!("https://www.youtube.com/watch?v={}", video_id);

        let html = self
            .client
            .get(&watch_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()?;

        let tracks = self.extract_caption_tracks(&html)?;

        // Priorisation des pistes anglaises :
        // 1. Piste manuelle officielle anglaise ("en", "en-US", "en-GB")
        // 2. Piste auto-générée anglaise (ASR)
        let selected_track = tracks
            .iter()
            .find(|t| !t.is_auto_generated && t.language_code.starts_with("en"))
            .or_else(|| tracks.iter().find(|t| t.language_code.starts_with("en")))?;

        let base_url = &selected_track.base_url;

        // Tenter le format natif JSON3 de YouTube pour une granularité mot-à-mot
        let json3_url = if base_url.contains("fmt=") {
            base_url.clone()
        } else {
            format!("{}&fmt=json3", base_url)
        };

        if let Ok(resp) = self.client.get(&json3_url).send().await {
            if let Ok(text) = resp.text().await {
                if let Some(lines) = parse_youtube_json3(&text) {
                    if !lines.is_empty() {
                        return Some(lines);
                    }
                }
            }
        }

        // Repli sur le format WebVTT
        let vtt_url = format!("{}&fmt=vtt", base_url);
        if let Ok(resp) = self.client.get(&vtt_url).send().await {
            if let Ok(text) = resp.text().await {
                let lines = parse_vtt_or_srt(&text);
                if !lines.is_empty() {
                    return Some(lines);
                }
            }
        }

        None
    }

    /// Extrait les pistes de sous-titres depuis le code source HTML de la page vidéo
    fn extract_caption_tracks(&self, html: &str) -> Option<Vec<CaptionTrack>> {
        let cap_re = get_caption_tracks_re();
        let json_array_str = cap_re.captures(html)?.get(1)?.as_str();

        let val: serde_json::Value = serde_json::from_str(json_array_str).ok()?;
        let arr = val.as_array()?;

        let mut tracks = Vec::new();
        for item in arr {
            let base_url = item.get("baseUrl").and_then(|v| v.as_str())?.to_string();
            let language_code = item
                .get("languageCode")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let vss_id = item.get("vssId").and_then(|v| v.as_str()).unwrap_or("");
            let is_auto_generated = vss_id.starts_with("a.") || item.get("kind").and_then(|v| v.as_str()) == Some("asr");

            let name = item
                .get("name")
                .and_then(|n| n.get("simpleText").or_else(|| n.get("runs").and_then(|r| r.get(0)).and_then(|r0| r0.get("text"))))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            tracks.push(CaptionTrack {
                base_url,
                language_code,
                name,
                is_auto_generated,
            });
        }

        if tracks.is_empty() {
            None
        } else {
            Some(tracks)
        }
    }
}
