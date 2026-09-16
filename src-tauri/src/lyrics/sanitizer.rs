use regex::Regex;

/// Nettoie les titres de pistes pour maximiser les correspondances avec les bases de paroles.
pub fn sanitize_track_title(title: &str) -> String {
    // Supprime les mentions de collaboration : (feat. ...), [ft. ...], etc.
    let re_feat = Regex::new(r"(?i)[\(\[\{]\s*(feat|ft)\.?\s+[^\)\]\}]+[\)\]\}]").unwrap();
    let cleaned = re_feat.replace_all(title, "");

    // Supprime les mentions de version / remaster / live / single / explicit
    let re_version = Regex::new(
        r"(?i)[\(\[\{]\s*(remastered|remaster|live|official\s+video|deluxe|bonus\s+track|radio\s+edit|single|explicit)[^\)\]\}]*[\)\]\}]"
    ).unwrap();
    let cleaned = re_version.replace_all(&cleaned, "");

    // Supprime les suffixes séparés par un tiret comme " - Remastered", " - Live", " - Single"
    let re_dash_suffix = Regex::new(r"(?i)\s*-\s*(?:remastered|remaster|live|deluxe|single|radio\s+edit)(?:\s+.*)?$").unwrap();
    let cleaned = re_dash_suffix.replace_all(&cleaned, "");

    cleaned.trim().to_string()
}

/// Normalise le nom d'artiste et produit des variantes de recherche (ex: artiste principal, alias).
pub fn get_artist_variants(artist: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let trimmed = artist.trim();
    if trimmed.is_empty() {
        return variants;
    }

    variants.push(trimmed.to_string());

    // Si l'artiste contient "Maître Gims", tester aussi "GIMS" / "Gims"
    if trimmed.to_lowercase().contains("maître gims") || trimmed.to_lowercase().contains("maitre gims") {
        variants.push("Gims".to_string());
    } else if trimmed.eq_ignore_ascii_case("gims") {
        variants.push("Maître Gims".to_string());
    }

    // Extraction du premier artiste en cas de featurings / séparateurs multiples
    let split_chars = [',', '&', ';', '/'];
    for sep in split_chars {
        if let Some((first, _)) = trimmed.split_once(sep) {
            let clean_first = first.trim().to_string();
            if !clean_first.is_empty() && !variants.contains(&clean_first) {
                variants.push(clean_first);
            }
        }
    }

    // Séparateur textuel " feat. " ou " ft. " ou " x "
    let re_split_words = Regex::new(r"(?i)\s+(?:feat\.?|ft\.?|featuring|x)\s+").unwrap();
    if let Some(mat) = re_split_words.find(trimmed) {
        let first = trimmed[..mat.start()].trim().to_string();
        if !first.is_empty() && !variants.contains(&first) {
            variants.push(first);
        }
    }

    variants
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
        assert_eq!(sanitize_track_title("NINAO - Single"), "NINAO");
    }

    #[test]
    fn test_artist_variants() {
        let vars = get_artist_variants("Gims, Dadju");
        assert!(vars.contains(&"Gims".to_string()));

        let vars2 = get_artist_variants("Maître Gims");
        assert!(vars2.contains(&"Gims".to_string()));

        let vars3 = get_artist_variants("David Guetta feat. Bebe Rexha");
        assert!(vars3.contains(&"David Guetta".to_string()));
    }
}
