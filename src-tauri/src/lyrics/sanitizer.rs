use regex::Regex;

/// Nettoie les titres de pistes pour maximiser les correspondances avec les bases de paroles.
pub fn sanitize_track_title(title: &str) -> String {
    // Supprime les mentions de collaboration : (feat. ...), [ft. ...], etc.
    let re_feat = Regex::new(r"(?i)[\(\[\{]\s*(feat|ft)\.?\s+[^\)\]\}]+[\)\]\}]").unwrap();
    let cleaned = re_feat.replace_all(title, "");

    // Supprime les mentions de version / remaster / live : (Remastered 2021), [Live at ...], etc.
    let re_version = Regex::new(
        r"(?i)[\(\[\{]\s*(remastered|remaster|live|official\s+video|deluxe|bonus\s+track|radio\s+edit)[^\)\]\}]*[\)\]\}]"
    ).unwrap();
    let cleaned = re_version.replace_all(&cleaned, "");

    // Supprime les suffixes séparés par un tiret comme " - Remastered" ou " - Live"
    let re_dash_suffix = Regex::new(r"(?i)\s*-\s*(remastered|remaster|live|deluxe).+$").unwrap();
    let cleaned = re_dash_suffix.replace_all(&cleaned, "");

    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_feat() {
        assert_eq!(sanitize_track_title("Song Title (feat. Artist)"), "Song Title");
        assert_eq!(sanitize_track_title("Song Title [ft. Artist B]"), "Song Title");
    }

    #[test]
    fn test_sanitize_remaster() {
        assert_eq!(sanitize_track_title("Bohemian Rhapsody - Remastered 2011"), "Bohemian Rhapsody");
        assert_eq!(sanitize_track_title("Yesterday (Remastered)"), "Yesterday");
    }
}
