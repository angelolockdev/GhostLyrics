use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackType {
    Music,
    Video,
    Unknown,
}

static YOUTUBE_SUFFIX_RE: OnceLock<Regex> = OnceLock::new();
static BRACKETED_NOISE_RE: OnceLock<Regex> = OnceLock::new();
static MUSIC_POSITIVE_REGEX: OnceLock<Regex> = OnceLock::new();
static NON_MUSIC_REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();

fn get_non_music_regexes() -> &'static [Regex] {
    NON_MUSIC_REGEXES.get_or_init(|| {
        vec![
            // Tutoriels / Formations / Guides
            Regex::new(r"(?i)\b(?:tutoriel|tutorial|how\s+to|comment\s+(?:faire|coder|créer|installer|réparer|utiliser|apprendre|débuter)|guide\s+(?:complet|pour|débutant)|cours\s+complet|formation)\b").unwrap(),
            // Gaming / Streams / Walkthroughs
            Regex::new(r"(?i)\b(?:gameplay|walkthrough|playthrough|let's\s*play|lets\s*play|speedrun|gaming|stream\s+(?:live|fr|en)|rediffusion)\b").unwrap(),
            // Reviews / Tests / Déballages
            Regex::new(r"(?i)\b(?:review|unboxing|test\s+(?:complet|rapide|matériel)|avis\s+sur|critique\b)").unwrap(),
            // Podcasts / Conférences / Interviews
            Regex::new(r"(?i)\b(?:podcast|interview|conférence|conference|débat|talk\s+show|tedx)\b").unwrap(),
            // Documentaires / Enquêtes / Actualités
            Regex::new(r"(?i)\b(?:documentaire|documentary|reportage|investigation|breaking\s+news|journal\s+télévisé|le\s+jt|actualité)\b").unwrap(),
            // Bandes-annonces / Teasers
            Regex::new(r"(?i)\b(?:official\s+trailer|bande-annonce|teaser\s+officiel)\b").unwrap(),
            // Vlogs / Réactions / Divertissement
            Regex::new(r"(?i)\b(?:vlog|prank|micro-trottoir|réaction\s+à|reaction\s+to|react\s+to)\b").unwrap(),
            // Épisodes / Séries (e.g. Episode 4, Ep. 2, Saison 1)
            Regex::new(r"(?i)\b(?:saison|season|épisode|episode|ep\.)\s*\d+\b").unwrap(),
            // Compilations / Best of / Highlights
            Regex::new(r"(?i)\b(?:compilation|best\s+of|highlights)\b").unwrap(),
        ]
    })
}

fn get_music_positive_regex() -> &'static Regex {
    MUSIC_POSITIVE_REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:official\s+(?:music\s+)?video|clip\s+officiel|official\s+audio|audio\s+officiel|lyric\s+video|visualizer|lyrics|paroles|remix|acoustic|live\s+session|prod\.|feat\.|ft\.)\b").unwrap()
    })
}

pub fn is_dedicated_music_app(raw_id: &str) -> bool {
    let lower = raw_id.to_lowercase();
    lower.contains("spotify")
        || lower.contains("deezer")
        || lower.contains("applemusic")
        || lower.contains("apple.music")
        || lower.contains("itunes")
        || lower.contains("tidal")
        || lower.contains("amazonmusic")
        || lower.contains("amazon.music")
        || lower.contains("qobuz")
        || lower.contains("musicbee")
        || lower.contains("foobar")
        || lower.contains("aimp")
        || lower.contains("winamp")
        || lower.contains("audacious")
        || lower.contains("clementine")
        || lower.contains("rhythmbox")
}

pub fn is_web_browser(raw_id: &str) -> bool {
    let lower = raw_id.to_lowercase();
    lower.contains("chrome")
        || lower.contains("msedge")
        || lower.contains("edge")
        || lower.contains("firefox")
        || lower.contains("brave")
        || lower.contains("opera")
        || lower.contains("vivaldi")
        || lower.contains("arc")
        || lower.contains("zen")
        || lower.contains("browser")
}

pub fn is_generic_or_empty_artist(artist: &str) -> bool {
    let lower = artist.trim().to_lowercase();
    lower.is_empty()
        || lower == "youtube"
        || lower == "youtube music"
        || lower == "google chrome"
        || lower == "chrome"
        || lower == "microsoft edge"
        || lower == "edge"
        || lower == "mozilla firefox"
        || lower == "firefox"
        || lower == "brave"
        || lower == "opera"
        || lower == "unknown artist"
        || lower == "artiste inconnu"
}

pub fn clean_youtube_title(raw: &str) -> String {
    let re_suffix = YOUTUBE_SUFFIX_RE.get_or_init(|| {
        Regex::new(r"(?i)\s*-\s*YouTube(?:\s*Music)?\s*$").unwrap()
    });
    let re_noise = BRACKETED_NOISE_RE.get_or_init(|| {
        Regex::new(r"(?i)\s*[\(\[](?:official\s+(?:music\s+)?video|official\s+audio|clip\s+officiel|audio\s+officiel|lyric\s+video|lyrics|paroles|visualizer|audio|live(?:\s+session)?|hd|4k)[\)\]]").unwrap()
    });

    let without_suffix = re_suffix.replace(raw, "");
    let without_noise = re_noise.replace_all(&without_suffix, "");
    without_noise.trim().to_string()
}

pub fn extract_web_music_metadata(raw_title: &str, raw_artist: &str) -> (String, String) {
    let cleaned = clean_youtube_title(raw_title);

    // Si le navigateur a déjà un vrai artiste non-générique (ex: YouTube Music PWA)
    if !is_generic_or_empty_artist(raw_artist) {
        return (cleaned, raw_artist.trim().to_string());
    }

    // Essayer de séparer "Artiste - Titre"
    if let Some((artist_part, title_part)) = cleaned.split_once(" - ") {
        let artist = artist_part.trim();
        let title = title_part.trim();
        if !artist.is_empty() && !title.is_empty() {
            return (title.to_string(), artist.to_string());
        }
    }

    // Repli par défaut
    (cleaned, String::new())
}

pub fn is_valid_music_candidate(
    raw_app_id: &str,
    title: &str,
    artist: &str,
    playback_type: PlaybackType,
) -> bool {
    if is_dedicated_music_app(raw_app_id) {
        return !title.trim().is_empty();
    }

    let trimmed_title = title.trim();
    if trimmed_title.is_empty() {
        return false;
    }

    // Nettoyer le suffixe " - YouTube" avant d'évaluer les tirets et signaux
    let clean_title = if is_web_browser(raw_app_id) || trimmed_title.to_lowercase().contains("youtube") {
        clean_youtube_title(trimmed_title)
    } else {
        trimmed_title.to_string()
    };

    if clean_title.is_empty() {
        return false;
    }

    // Rejet strict des vidéos YouTube / Web non-musicales (tutos, gaming, podcasts...)
    for re in get_non_music_regexes() {
        if re.is_match(&clean_title) || re.is_match(artist) {
            return false;
        }
    }

    let has_music_signal = get_music_positive_regex().is_match(&clean_title)
        || clean_title.contains(" - ");

    if playback_type == PlaybackType::Video && !has_music_signal {
        return false;
    }

    if is_web_browser(raw_app_id) {
        let has_real_artist = !is_generic_or_empty_artist(artist);
        let has_split_title = clean_title.contains(" - ");
        if !has_real_artist && !has_split_title {
            return false;
        }
    }

    true
}

pub fn calculate_music_score(
    raw_app_id: &str,
    title: &str,
    artist: &str,
    album: &str,
    is_playing: bool,
    playback_type: PlaybackType,
) -> i32 {
    let mut score = 0;

    let clean_title = if is_web_browser(raw_app_id) || title.to_lowercase().contains("youtube") {
        clean_youtube_title(title)
    } else {
        title.to_string()
    };

    // 1. Priorité applicative
    if is_dedicated_music_app(raw_app_id) {
        score += 1500;
    } else if is_web_browser(raw_app_id) {
        score += 200;
        let lower_title = title.to_lowercase();
        let lower_app = raw_app_id.to_lowercase();
        if lower_title.contains("youtube music") || lower_app.contains("youtubemusic") {
            score += 500;
        }
    } else {
        // Autres lecteurs multimédias locaux (ex: VLC)
        score += 600;
    }

    // 2. Statut de lecture : un média en cours de lecture reçoit un fort bonus
    if is_playing {
        score += 2500;
    }

    // 3. Qualité des métadonnées
    if !is_generic_or_empty_artist(artist) {
        score += 300;
    }
    if !album.trim().is_empty() {
        score += 200;
    }

    // 4. Type de média rapporté par le système
    match playback_type {
        PlaybackType::Music => score += 400,
        PlaybackType::Video => {
            if is_web_browser(raw_app_id) {
                score -= 300;
            } else {
                score -= 100;
            }
        }
        PlaybackType::Unknown => {}
    }

    // 5. Signaux de titre
    if clean_title.contains(" - ") {
        score += 250;
    }
    if get_music_positive_regex().is_match(&clean_title) {
        score += 200;
    }
    for re in get_non_music_regexes() {
        if re.is_match(&clean_title) || re.is_match(artist) {
            score -= 2000;
            break;
        }
    }

    score
}

pub fn format_app_name(raw_id: &str, title: &str) -> String {
    let lower_id = raw_id.to_lowercase();
    let lower_title = title.to_lowercase();

    if lower_id.contains("spotify") {
        "Spotify".to_string()
    } else if lower_id.contains("deezer") {
        "Deezer".to_string()
    } else if lower_id.contains("applemusic") || lower_id.contains("apple.music") || lower_id.contains("itunes") {
        "Apple Music".to_string()
    } else if lower_id.contains("tidal") {
        "Tidal".to_string()
    } else if lower_id.contains("amazonmusic") {
        "Amazon Music".to_string()
    } else if lower_id.contains("qobuz") {
        "Qobuz".to_string()
    } else if lower_title.contains("youtube music") {
        "YouTube Music".to_string()
    } else if lower_title.contains("youtube") || lower_id.contains("youtube") {
        "YouTube".to_string()
    } else if lower_id.contains("chrome") {
        "Google Chrome".to_string()
    } else if lower_id.contains("msedge") || lower_id.contains("edge") {
        "Microsoft Edge".to_string()
    } else if lower_id.contains("firefox") {
        "Firefox".to_string()
    } else if lower_id.contains("brave") {
        "Brave".to_string()
    } else if lower_id.contains("opera") {
        "Opera".to_string()
    } else if lower_id.contains("vlc") {
        "VLC".to_string()
    } else if raw_id.is_empty() {
        "Lecteur Windows".to_string()
    } else {
        raw_id.split('.').next().unwrap_or(raw_id).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_classification() {
        assert!(is_dedicated_music_app("Spotify.exe"));
        assert!(is_dedicated_music_app("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"));
        assert!(is_dedicated_music_app("Deezer.exe"));
        assert!(is_dedicated_music_app("AppleMusic.exe"));
        assert!(is_dedicated_music_app("Tidal.exe"));
        assert!(!is_dedicated_music_app("chrome.exe"));
        assert!(!is_dedicated_music_app("msedge.exe"));

        assert!(is_web_browser("chrome.exe"));
        assert!(is_web_browser("msedge.exe"));
        assert!(is_web_browser("firefox.exe"));
        assert!(is_web_browser("brave.exe"));
        assert!(!is_web_browser("Spotify.exe"));
    }

    #[test]
    fn test_clean_youtube_title() {
        assert_eq!(
            clean_youtube_title("GIMS - NINAO (Clip Officiel) - YouTube"),
            "GIMS - NINAO"
        );
        assert_eq!(
            clean_youtube_title("Daft Punk - Get Lucky (Official Audio) - YouTube"),
            "Daft Punk - Get Lucky"
        );
        assert_eq!(
            clean_youtube_title("The Weeknd - Blinding Lights [Official Music Video] - YouTube"),
            "The Weeknd - Blinding Lights"
        );
    }

    #[test]
    fn test_extract_web_music_metadata() {
        let (title, artist) = extract_web_music_metadata(
            "GIMS - NINAO (Clip Officiel) - YouTube",
            "",
        );
        assert_eq!(artist, "GIMS");
        assert_eq!(title, "NINAO");

        let (title, artist) = extract_web_music_metadata(
            "Bohemian Rhapsody - YouTube Music",
            "Queen",
        );
        assert_eq!(artist, "Queen");
        assert_eq!(title, "Bohemian Rhapsody");
    }

    #[test]
    fn test_rejects_non_music_youtube_videos() {
        // Tutoriel
        assert!(!is_valid_music_candidate(
            "chrome.exe",
            "Comment apprendre Rust en 2026 (Tutoriel débutant) - YouTube",
            "",
            PlaybackType::Video,
        ));
        // Gaming / Gameplay
        assert!(!is_valid_music_candidate(
            "msedge.exe",
            "Minecraft Hardcore Ep. 12 (Gameplay FR) - YouTube",
            "",
            PlaybackType::Video,
        ));
        // Review tech
        assert!(!is_valid_music_candidate(
            "chrome.exe",
            "iPhone 16 Pro Review: Don't Buy Yet! - YouTube",
            "",
            PlaybackType::Video,
        ));
        // Vidéo sans format artiste - titre ni artiste
        assert!(!is_valid_music_candidate(
            "chrome.exe",
            "Mon chat fait des acrobaties hilarantes - YouTube",
            "",
            PlaybackType::Video,
        ));
    }

    #[test]
    fn test_accepts_valid_music_youtube() {
        assert!(is_valid_music_candidate(
            "chrome.exe",
            "GIMS - NINAO (Clip Officiel) - YouTube",
            "",
            PlaybackType::Video,
        ));
        assert!(is_valid_music_candidate(
            "msedge.exe",
            "Daft Punk - Get Lucky (Official Audio) - YouTube",
            "",
            PlaybackType::Video,
        ));
    }

    #[test]
    fn test_spotify_vs_youtube_priorities() {
        // 1. Spotify joue vs YouTube clip musical qui joue :
        // Spotify a la priorité applicative absolue (4900 vs 2650)
        let spotify_score = calculate_music_score(
            "Spotify.exe",
            "NINAO",
            "GIMS",
            "Le Fléau",
            true,
            PlaybackType::Music,
        );
        let youtube_music_score = calculate_music_score(
            "chrome.exe",
            "GIMS - NINAO (Clip Officiel) - YouTube",
            "",
            "",
            true,
            PlaybackType::Video,
        );
        assert!(spotify_score > youtube_music_score, "Spotify ({}) doit battre YouTube clip ({})", spotify_score, youtube_music_score);

        // 2. Spotify en pause vs YouTube clip musical qui joue :
        // Si Spotify est en pause et que YouTube joue de la vraie musique, YouTube doit l'emporter (2650 vs 2400)
        let spotify_paused_score = calculate_music_score(
            "Spotify.exe",
            "NINAO",
            "GIMS",
            "Le Fléau",
            false,
            PlaybackType::Music,
        );
        assert!(youtube_music_score > spotify_paused_score, "YouTube clip en cours de lecture ({}) doit battre Spotify en pause ({})", youtube_music_score, spotify_paused_score);
    }
}
