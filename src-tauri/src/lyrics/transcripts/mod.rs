pub mod podcast;
pub mod vtt_parser;
pub mod youtube;

pub use podcast::PodcastTranscriptProvider;
pub use vtt_parser::{clean_subtitle_text, parse_vtt_or_srt, parse_youtube_json3};
pub use youtube::YouTubeTranscriptProvider;

use crate::lyrics::parser::LyricLine;

pub struct TranscriptService {
    youtube_provider: YouTubeTranscriptProvider,
    podcast_provider: PodcastTranscriptProvider,
}

impl TranscriptService {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            youtube_provider: YouTubeTranscriptProvider::new(client.clone()),
            podcast_provider: PodcastTranscriptProvider::new(client),
        }
    }

    /// Recherche une transcription soit via YouTube soit via les flux de Podcasts
    pub async fn fetch_transcript(
        &self,
        title: &str,
        artist: &str,
        is_youtube: bool,
    ) -> Option<(Vec<LyricLine>, &'static str)> {
        if is_youtube {
            if let Some(lines) = self.youtube_provider.fetch_english_transcript(title).await {
                return Some((lines, "YouTube CC"));
            }
        }

        // Tenter d'abord la recherche Podcast si un artiste/show est présent
        if !artist.trim().is_empty() {
            if let Some(lines) = self.podcast_provider.fetch_podcast_transcript(title, artist).await {
                return Some((lines, "Podcast RSS"));
            }
        }

        // Tenter également la recherche YouTube en fallback même si l'app source n'était pas un navigateur
        let combined_query = format!("{} {}", artist, title).trim().to_string();
        if let Some(lines) = self.youtube_provider.fetch_english_transcript(&combined_query).await {
            return Some((lines, "YouTube CC"));
        }

        None
    }
}
