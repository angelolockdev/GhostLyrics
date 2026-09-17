use super::parser::{parse_lrc, LyricLine};
use super::sanitizer::{get_artist_variants, sanitize_track_title};
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
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default = "default_true")]
    pub is_synced: bool,
}

fn default_source() -> String {
    "LRCLIB".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Clone)]
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

#[derive(Deserialize)]
struct LyricsOvhResponse {
    pub lyrics: Option<String>,
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
                .user_agent("GhostLyrics/0.1.6 (https://github.com/angelolockdev/GhostLyrics)")
                .timeout(std::time::Duration::from_secs(10))
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
        let artist_variants = get_artist_variants(artist);
        let primary_artist = artist_variants.first().map(|s| s.as_str()).unwrap_or(artist);

        let cache_key = format!("{}_{}", sanitize_filename(&cleaned_title), sanitize_filename(primary_artist));
        let cache_file = self.cache_dir.join(format!("{}.json", cache_key));

        // 1. Vérifier le cache disque local
        if let Ok(cached_data) = fs::read_to_string(&cache_file) {
            if let Ok(parsed) = serde_json::from_str::<LyricsResponse>(&cached_data) {
                return Ok(parsed);
            }
        }

        // 2. Cascade de résolution LRCLIB
        let mut lrclib_result: Option<LrclibApiResponse> = None;
        let mut source_desc = "LRCLIB".to_string();

        // Étape 2a : LRCLIB /api/get (exact avec album si fourni)
        if let Some(alb) = album {
            if !alb.trim().is_empty() {
                if let Some(res) = self.query_lrclib_get(&cleaned_title, primary_artist, Some(alb)).await {
                    lrclib_result = Some(res);
                    source_desc = "LRCLIB (Album)".to_string();
                }
            }
        }

        // Étape 2b : LRCLIB /api/get sans album (résout les divergences de noms d'albums singles/compilations)
        if lrclib_result.is_none() {
            if let Some(res) = self.query_lrclib_get(&cleaned_title, primary_artist, None).await {
                lrclib_result = Some(res);
                source_desc = "LRCLIB (Exact)".to_string();
            }
        }

        // Étape 2c : LRCLIB /api/search (recherche floue par mots-clés)
        if lrclib_result.is_none() {
            for variant in &artist_variants {
                if let Some(res) = self.query_lrclib_search(&cleaned_title, variant).await {
                    lrclib_result = Some(res);
                    source_desc = "LRCLIB (Recherche)".to_string();
                    break;
                }
            }
        }

        // 3. Traitement du résultat LRCLIB si trouvé
        if let Some(api_data) = lrclib_result {
            let has_synced = api_data.synced_lyrics.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
            let has_plain = api_data.plain_lyrics.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
            let is_instrumental = api_data.instrumental.unwrap_or(false);

            let (lines, is_synced) = if has_synced {
                let synced_text = api_data.synced_lyrics.as_ref().unwrap();
                (parse_lrc(synced_text), true)
            } else if has_plain {
                let plain_text = api_data.plain_lyrics.as_ref().unwrap();
                (generate_paced_lines(plain_text, api_data.duration.or(duration_sec)), false)
            } else {
                (Vec::new(), false)
            };

            let final_response = LyricsResponse {
                id: api_data.id,
                track_name: api_data.track_name.or(api_data.track_name_alt).unwrap_or(cleaned_title),
                artist_name: api_data.artist_name.unwrap_or_else(|| primary_artist.to_string()),
                album_name: api_data.album_name,
                duration: api_data.duration.or(duration_sec),
                instrumental: is_instrumental,
                plain_lyrics: api_data.plain_lyrics,
                synced_lyrics: api_data.synced_lyrics,
                lines,
                source: if is_synced { format!("{} (Sync)", source_desc) } else { format!("{} (Texte)", source_desc) },
                is_synced,
            };

            if let Ok(serialized) = serde_json::to_string_pretty(&final_response) {
                let _ = fs::write(&cache_file, serialized);
            }
            return Ok(final_response);
        }

        // 4. Étape 4 : Fallback externe vers lyrics.ovh (paroles textuelles ouvertes)
        for variant in &artist_variants {
            if let Some(plain_text) = self.query_lyrics_ovh(&cleaned_title, variant).await {
                let lines = generate_paced_lines(&plain_text, duration_sec);
                let final_response = LyricsResponse {
                    id: None,
                    track_name: cleaned_title.clone(),
                    artist_name: variant.clone(),
                    album_name: album.map(|a| a.to_string()),
                    duration: duration_sec,
                    instrumental: false,
                    plain_lyrics: Some(plain_text),
                    synced_lyrics: None,
                    lines,
                    source: "lyrics.ovh (Texte)".to_string(),
                    is_synced: false,
                };

                if let Ok(serialized) = serde_json::to_string_pretty(&final_response) {
                    let _ = fs::write(&cache_file, serialized);
                }
                return Ok(final_response);
            }
        }

        Err(format!(
            "Paroles introuvables pour '{}' de '{}' (testé sur LRCLIB & lyrics.ovh)",
            cleaned_title, primary_artist
        ))
    }

    async fn query_lrclib_get(&self, title: &str, artist: &str, album: Option<&str>) -> Option<LrclibApiResponse> {
        let mut query = vec![
            ("track_name", title.to_string()),
            ("artist_name", artist.to_string()),
        ];
        if let Some(alb) = album {
            query.push(("album_name", alb.to_string()));
        }

        let res = self.send_request_with_retry("https://lrclib.net/api/get", &query).await.ok()?;
        if res.status().is_success() {
            res.json::<LrclibApiResponse>().await.ok()
        } else {
            None
        }
    }

    async fn query_lrclib_search(&self, title: &str, artist: &str) -> Option<LrclibApiResponse> {
        let query_str = format!("{} {}", title, artist);
        let query = vec![("q", query_str)];

        let res = self.send_request_with_retry("https://lrclib.net/api/search", &query).await.ok()?;
        if !res.status().is_success() {
            return None;
        }

        let list = res.json::<Vec<LrclibApiResponse>>().await.ok()?;
        if list.is_empty() {
            return None;
        }

        // Priorité 1 : Résultat contenant des paroles synchronisées (.lrc)
        if let Some(found) = list.iter().find(|item| {
            item.synced_lyrics.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
        }) {
            return Some(found.clone());
        }

        // Priorité 2 : Résultat avec paroles brutes
        if let Some(found) = list.iter().find(|item| {
            item.plain_lyrics.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
        }) {
            return Some(found.clone());
        }

        // Priorité 3 : Premier élément de la liste
        list.into_iter().next()
    }

    async fn query_lyrics_ovh(&self, title: &str, artist: &str) -> Option<String> {
        let encoded_artist = url_percent_encode(artist);
        let encoded_title = url_percent_encode(title);
        let url = format!("https://api.lyrics.ovh/v1/{}/{}", encoded_artist, encoded_title);

        let res = self.client.get(&url).send().await.ok()?;
        if res.status().is_success() {
            let data = res.json::<LyricsOvhResponse>().await.ok()?;
            data.lyrics.filter(|s| !s.trim().is_empty())
        } else {
            None
        }
    }

    async fn send_request_with_retry(&self, url: &str, query: &[(&str, String)]) -> Result<reqwest::Response, reqwest::Error> {
        let mut attempts = 0;
        loop {
            attempts += 1;
            let res = self.client.get(url).query(query).send().await;
            match res {
                Ok(response) => {
                    // Retry si 503 Server Overloaded
                    if response.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE && attempts < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                        continue;
                    }
                    return Ok(response);
                }
                Err(e) => {
                    if attempts < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }
}

/// Génère des lignes de paroles avec un rythme temporel progressif pour les paroles non synchronisées.
fn generate_paced_lines(plain_text: &str, duration_sec: Option<f64>) -> Vec<LyricLine> {
    let raw_lines: Vec<&str> = plain_text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if raw_lines.is_empty() {
        return Vec::new();
    }

    let count = raw_lines.len();
    let total_ms = duration_sec.unwrap_or(0.0) * 1000.0;

    let mut paced = Vec::with_capacity(count);

    if total_ms > 20000.0 {
        // Morceau avec durée connue : départ après 3s d'intro, fin ~5s avant la fin
        let start_offset = 3000.0;
        let end_offset = 5000.0;
        let usable_duration = (total_ms - start_offset - end_offset).max(10000.0);
        let interval = usable_duration / count as f64;

        for (i, line) in raw_lines.iter().enumerate() {
            let start_time_ms = (start_offset + (i as f64 * interval)).round() as i64;
            let end_time_ms = Some(((start_offset + ((i + 1) as f64 * interval)).round() as i64).min(total_ms as i64));
            paced.push(LyricLine {
                start_time_ms,
                end_time_ms,
                text: line.to_string(),
            });
        }
    } else {
        // Durée inconnue : pacing fixe à 3.5s par ligne
        for (i, line) in raw_lines.iter().enumerate() {
            let start_time_ms = (i as i64) * 3500;
            let end_time_ms = Some(start_time_ms + 3500);
            paced.push(LyricLine {
                start_time_ms,
                end_time_ms,
                text: line.to_string(),
            });
        }
    }

    paced
}

fn url_percent_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{:02X}", b));
        }
    }
    encoded
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "test réseau direct vers lrclib.net"]
    async fn test_live_fetch_lyrics_queen() {
        let service = LyricsService::new();
        let res = service.fetch_lyrics("Bohemian Rhapsody", "Queen", None, None).await;
        assert!(res.is_ok());
        let lyrics = res.unwrap();
        assert!(!lyrics.lines.is_empty());
        assert!(lyrics.is_synced);
    }

    #[tokio::test]
    #[ignore = "test réseau direct vers lrclib.net"]
    async fn test_live_fetch_ninao_gims() {
        let service = LyricsService::new();
        // Teste avec un nom d'album erroné/inconnu pour vérifier que la cascade rattrape la correspondance
        let res = service.fetch_lyrics("NINAO", "Gims", Some("Single Inconnu"), Some(168.0)).await;
        assert!(res.is_ok(), "NINAO de Gims doit être résolu avec succès: {:?}", res.err());
        let lyrics = res.unwrap();
        println!("Morceau trouvé : {} - {}", lyrics.track_name, lyrics.artist_name);
        println!("Source : {}", lyrics.source);
        println!("Synchronisé : {}", lyrics.is_synced);
        assert!(!lyrics.lines.is_empty());
        assert!(lyrics.is_synced);
    }

    #[tokio::test]
    async fn test_paced_lines_generator() {
        let text = "Ligne un\nLigne deux\nLigne trois";
        let lines = generate_paced_lines(text, Some(30.0));
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].start_time_ms, 3000);
        assert!(lines[1].start_time_ms > lines[0].start_time_ms);
        assert!(lines[2].start_time_ms > lines[1].start_time_ms);
    }

    #[tokio::test]
    #[ignore = "test réseau direct vers lyrics.ovh"]
    async fn test_query_lyrics_ovh() {
        let service = LyricsService::new();
        let lyrics = service.query_lyrics_ovh("Papaoutai", "Stromae").await;
        assert!(lyrics.is_some(), "lyrics.ovh doit renvoyer les paroles de Papaoutai");
        let text = lyrics.unwrap();
        assert!(text.contains("Où t'es, papa, où t'es") || text.contains("Dites-moi"));
    }
}

