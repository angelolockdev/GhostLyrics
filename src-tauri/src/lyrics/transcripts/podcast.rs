use super::vtt_parser::parse_vtt_or_srt;
use crate::lyrics::parser::LyricLine;
use regex::Regex;
use serde::Deserialize;
use std::sync::OnceLock;

static TRANSCRIPT_TAG_RE: OnceLock<Regex> = OnceLock::new();

fn get_transcript_tag_re() -> &'static Regex {
    TRANSCRIPT_TAG_RE.get_or_init(|| {
        Regex::new(r#"(?i)<(?:podcast:)?transcript\s+[^>]*url\s*=\s*["']([^"']+)["'][^>]*>"#).unwrap()
    })
}

#[derive(Deserialize)]
struct ItunesSearchResult {
    pub results: Vec<ItunesPodcastItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItunesPodcastItem {
    #[allow(dead_code)]
    pub collection_name: Option<String>,
    pub feed_url: Option<String>,
}

pub struct PodcastTranscriptProvider {
    client: reqwest::Client,
}

impl PodcastTranscriptProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Recherche la transcription d'un épisode de podcast par son titre d'épisode et le nom du podcast
    pub async fn fetch_podcast_transcript(
        &self,
        episode_title: &str,
        show_name: &str,
    ) -> Option<Vec<LyricLine>> {
        // 1. Trouver le flux RSS du podcast via l'API publique iTunes
        let feed_url = self.resolve_podcast_feed(show_name).await?;

        // 2. Télécharger le flux RSS
        let feed_xml = self
            .client
            .get(&feed_url)
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()?;

        // 3. Trouver l'URL de transcription de l'épisode correspondant
        let transcript_url = self.extract_transcript_url_for_episode(&feed_xml, episode_title)?;

        // 4. Télécharger le fichier de transcription
        let transcript_content = self
            .client
            .get(&transcript_url)
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()?;

        // 5. Parser en LyricLine (format WebVTT / SRT)
        let lines = parse_vtt_or_srt(&transcript_content);
        if lines.is_empty() {
            None
        } else {
            Some(lines)
        }
    }

    /// Recherche le flux RSS d'une émission de podcast
    async fn resolve_podcast_feed(&self, show_name: &str) -> Option<String> {
        let clean_show = show_name.trim();
        if clean_show.is_empty() {
            return None;
        }

        let query_url = format!(
            "https://itunes.apple.com/search?term={}&media=podcast&entity=podcast&limit=3",
            urlencoding::encode(clean_show)
        );

        let resp = self
            .client
            .get(&query_url)
            .timeout(std::time::Duration::from_secs(6))
            .send()
            .await
            .ok()?
            .json::<ItunesSearchResult>()
            .await
            .ok()?;

        resp.results
            .into_iter()
            .find_map(|item| item.feed_url)
    }

    /// Extrait la balise <podcast:transcript url="..."> pour l'épisode ciblé dans le flux RSS
    fn extract_transcript_url_for_episode(&self, feed_xml: &str, episode_title: &str) -> Option<String> {
        let clean_target = episode_title.trim().to_lowercase();
        let target_keywords: Vec<&str> = clean_target
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() >= 3)
            .collect();

        let re_transcript = get_transcript_tag_re();

        // Parcourt les blocs <item>...</item>
        let item_blocks = feed_xml.split("<item>");
        for block in item_blocks.skip(1) {
            let item_content = block.split("</item>").next().unwrap_or("");
            let item_lower = item_content.to_lowercase();

            // Vérifier la correspondance du titre de l'épisode
            let matches = if clean_target.is_empty() {
                true
            } else if item_lower.contains(&clean_target) {
                true
            } else {
                // Au moins 65% des mots-clés présents dans l'item
                if target_keywords.is_empty() {
                    false
                } else {
                    let matching_count = target_keywords
                        .iter()
                        .filter(|&&kw| item_lower.contains(kw))
                        .count();
                    matching_count * 100 / target_keywords.len() >= 60
                }
            };

            if matches {
                if let Some(caps) = re_transcript.captures(item_content) {
                    if let Some(url_match) = caps.get(1) {
                        let url = url_match.as_str().trim().to_string();
                        if !url.is_empty() {
                            return Some(url);
                        }
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_transcript_url() {
        let provider = PodcastTranscriptProvider::new(reqwest::Client::new());
        let xml = r#"
        <rss>
          <channel>
            <item>
              <title>Episode 101 - The Future of AI</title>
              <podcast:transcript url="https://example.com/ep101.vtt" type="text/vtt" />
            </item>
            <item>
              <title>Episode 102 - Learning Languages</title>
              <podcast:transcript url="https://example.com/ep102.vtt" type="text/vtt" />
            </item>
          </channel>
        </rss>
        "#;

        let url = provider.extract_transcript_url_for_episode(xml, "The Future of AI");
        assert_eq!(url, Some("https://example.com/ep101.vtt".to_string()));

        let url2 = provider.extract_transcript_url_for_episode(xml, "Learning Languages");
        assert_eq!(url2, Some("https://example.com/ep102.vtt".to_string()));
    }
}
