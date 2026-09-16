use super::parser::{parse_lrc, LyricLine};
use super::sanitizer::sanitize_track_title;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResponse {
    pub id: Option<i64>,
    pub track_name: String,
    pub artist_name: String,
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub instrumental: bool,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub lines: Vec<LyricLine>,
}

#[derive(Deserialize)]
struct LrclibApiResponse {
    pub id: Option<i64>,
    #[serde(rename = "name")]
    pub track_name: Option<String>,
    #[serde(rename = "trackName")]
    pub track_name_alt: Option<String>,
    #[serde(rename = "artistName")]
    pub artist_name: Option<String>,
    #[serde(rename = "albumName")]
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub instrumental: Option<bool>,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
}

pub struct LyricsService {
    client: reqwest::Client,
    cache_dir: PathBuf,
}

impl LyricsService {
    pub fn new() -> Self {
        let cache_dir = dirs_or_local_cache();
        if !cache_dir.exists() {
            let _ = fs::create_dir_all(&cache_dir);
        }

        Self {
            client: reqwest::Client::builder()
                .user_agent("GhostLyrics/0.1.0 (https://github.com/angelolockdev/GhostLyrics)")
                .build()
                .unwrap_or_default(),
            cache_dir,
        }
    }

    pub async fn fetch_lyrics(
        &self,
        title: &str,
        artist: &str,
        album: Option<&str>,
        duration_sec: Option<f64>,
    ) -> Result<LyricsResponse, String> {
        let cleaned_title = sanitize_track_title(title);
        let cache_key = format!("{}_{}", sanitize_filename(&cleaned_title), sanitize_filename(artist));
        let cache_file = self.cache_dir.join(format!("{}.json", cache_key));

        // 1. Vérifier le cache disque
        if let Ok(cached_data) = fs::read_to_string(&cache_file) {
            if let Ok(parsed) = serde_json::from_str::<LyricsResponse>(&cached_data) {
                return Ok(parsed);
            }
        }

        // 2. Requête vers l'API LRCLIB
        let mut query = vec![
            ("track_name", cleaned_title.clone()),
            ("artist_name", artist.to_string()),
        ];
        if let Some(alb) = album {
            if !alb.is_empty() {
                query.push(("album_name", alb.to_string()));
            }
        }

        let url = "https://lrclib.net/api/get";
        let res = self
            .client
            .get(url)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau LRCLIB: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("LRCLIB a retourné le statut: {}", res.status()));
        }

        let api_data = res
            .json::<LrclibApiResponse>()
            .await
            .map_err(|e| format!("Erreur de désérialisation LRCLIB: {}", e))?;

        let synced_text = api_data.synced_lyrics.clone().unwrap_or_default();
        let parsed_lines = parse_lrc(&synced_text);

        let final_response = LyricsResponse {
            id: api_data.id,
            track_name: api_data.track_name.or(api_data.track_name_alt).unwrap_or(cleaned_title),
            artist_name: api_data.artist_name.unwrap_or_else(|| artist.to_string()),
            album_name: api_data.album_name,
            duration: api_data.duration.or(duration_sec),
            instrumental: api_data.instrumental.unwrap_or(false),
            plain_lyrics: api_data.plain_lyrics,
            synced_lyrics: api_data.synced_lyrics,
            lines: parsed_lines,
        };

        // Sauvegarde dans le cache
        if let Ok(serialized) = serde_json::to_string_pretty(&final_response) {
            let _ = fs::write(&cache_file, serialized);
        }

        Ok(final_response)
    }
}

fn dirs_or_local_cache() -> PathBuf {
    if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(local_appdata).join("GhostLyrics").join("cache")
    } else {
        PathBuf::from("./local_cache")
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}
