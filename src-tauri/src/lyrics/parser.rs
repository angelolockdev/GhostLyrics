use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LyricWord {
    pub text: String,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub start_time_ms: i64,
    pub end_time_ms: Option<i64>,
    pub text: String,
    #[serde(default)]
    pub words: Vec<LyricWord>,
}

/// Parse le contenu brut d'un fichier .lrc (standard ou Enhanced LRC) et retourne une liste triée de lignes avec mots synchronisés.
pub fn parse_lrc(content: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();

    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if !trimmed.starts_with('[') {
            continue;
        }

        // Ex: [01:23.45] ou [00:12] ou multiple tags [01:23.45][02:34.56] Text
        let parts: Vec<&str> = trimmed.split(']').collect();
        if parts.len() < 2 {
            continue;
        }

        let raw_text = parts.last().unwrap_or(&"").trim().to_string();

        for part in &parts[..parts.len() - 1] {
            if let Some(tag) = part.strip_prefix('[') {
                if let Some(ms) = parse_lrc_timestamp(tag) {
                    lines.push(LyricLine {
                        start_time_ms: ms,
                        end_time_ms: None,
                        text: raw_text.clone(),
                        words: Vec::new(),
                    });
                }
            }
        }
    }

    // Tri par ordre chronologique
    lines.sort_by_key(|line| line.start_time_ms);

    // Calcul des end_time_ms
    for i in 0..lines.len() {
        if i + 1 < lines.len() {
            lines[i].end_time_ms = Some(lines[i + 1].start_time_ms);
        } else {
            lines[i].end_time_ms = Some(lines[i].start_time_ms + 6000); // 6 secondes par défaut pour la dernière ligne
        }
    }

    // Traitement des mots (Enhanced LRC avec tags explicites ou estimation syllabique)
    for line in &mut lines {
        let end_ms = line.end_time_ms.unwrap_or(line.start_time_ms + 4000);

        if let Some((clean_text, parsed_words)) = parse_enhanced_lrc_words(&line.text, line.start_time_ms, end_ms) {
            line.text = clean_text;
            line.words = parsed_words;
        } else {
            line.words = estimate_syllables_for_line(&line.text, line.start_time_ms, end_ms);
        }
    }

    lines
}

/// Détecte et extrait les balises Enhanced LRC `<mm:ss.xx>` dans un texte.
/// Retourne `Some((texte_nettoyé, mots))` si au moins un tag valide a été trouvé.
fn parse_enhanced_lrc_words(raw_text: &str, line_start_ms: i64, line_end_ms: i64) -> Option<(String, Vec<LyricWord>)> {
    if !raw_text.contains('<') || !raw_text.contains('>') {
        return None;
    }

    let mut words = Vec::new();
    let mut clean_text = String::with_capacity(raw_text.len());
    let mut current_pos = 0;
    let chars: Vec<char> = raw_text.chars().collect();
    let len = chars.len();

    let mut current_tag_ms: Option<i64> = None;
    let mut current_word_buf = String::new();

    while current_pos < len {
        if chars[current_pos] == '<' {
            // Sauvegarder le mot précédent s'il y en avait un
            if !current_word_buf.is_empty() {
                let text = current_word_buf.trim().to_string();
                if !text.is_empty() {
                    let start_ms = current_tag_ms.unwrap_or(line_start_ms);
                    words.push((text, start_ms));
                }
                current_word_buf.clear();
            }

            // Chercher la fin du tag '>'
            if let Some(close_rel) = chars[current_pos..].iter().position(|&c| c == '>') {
                let tag_str: String = chars[current_pos + 1..current_pos + close_rel].iter().collect();
                if let Some(ms) = parse_lrc_timestamp(&tag_str) {
                    current_tag_ms = Some(ms);
                }
                current_pos += close_rel + 1;
                continue;
            }
        }

        let c = chars[current_pos];
        clean_text.push(c);
        current_word_buf.push(c);
        current_pos += 1;
    }

    if !current_word_buf.is_empty() {
        let text = current_word_buf.trim().to_string();
        if !text.is_empty() {
            let start_ms = current_tag_ms.unwrap_or(line_start_ms);
            words.push((text, start_ms));
        }
    }

    if words.is_empty() {
        return None;
    }

    // Calcul des end_time_ms pour chaque mot
    let mut result_words = Vec::with_capacity(words.len());
    for i in 0..words.len() {
        let (text, start_ms) = words[i].clone();
        let end_ms = if i + 1 < words.len() {
            words[i + 1].1
        } else {
            line_end_ms
        };

        result_words.push(LyricWord {
            text,
            start_time_ms: start_ms,
            end_time_ms: end_ms.max(start_ms + 80),
        });
    }

    let normalized_clean = clean_text.split_whitespace().collect::<Vec<_>>().join(" ");
    Some((normalized_clean, result_words))
}

/// Estimateur syllabique heuristique pour découper une ligne classique en mots rythmés de façon naturelle.
pub fn estimate_syllables_for_line(text: &str, start_time_ms: i64, end_time_ms: i64) -> Vec<LyricWord> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let raw_tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if raw_tokens.is_empty() {
        return Vec::new();
    }

    let total_duration = (end_time_ms - start_time_ms).max(300);
    // Marge de respiration en fin de phrase (12% de la durée, min 100ms, max 350ms)
    let breath_pause_ms = ((total_duration as f64) * 0.12).clamp(100.0, 350.0) as i64;
    let singing_duration = (total_duration - breath_pause_ms).max(180);

    // Calcul du poids syllabique de chaque mot
    let mut weights: Vec<f64> = Vec::with_capacity(raw_tokens.len());
    for token in &raw_tokens {
        let mut vowels = 0;
        let mut others = 0;

        for c in token.chars() {
            let lower = c.to_lowercase().next().unwrap_or(c);
            if matches!(
                lower,
                'a' | 'e' | 'i' | 'o' | 'u' | 'y'
                    | 'à' | 'á' | 'â' | 'ã' | 'ä' | 'å'
                    | 'è' | 'é' | 'ê' | 'ë'
                    | 'ì' | 'í' | 'î' | 'ï'
                    | 'ò' | 'ó' | 'ô' | 'õ' | 'ö'
                    | 'ù' | 'ú' | 'û' | 'ü'
                    | 'ý' | 'ÿ'
            ) {
                vowels += 1;
            } else if c.is_alphabetic() {
                others += 1;
            }
        }

        // Poids phonétique : les voyelles sont chantées plus longuement que les consonnes
        let mut weight = (vowels as f64 * 2.0) + (others as f64 * 0.6);
        if weight < 1.0 {
            weight = 1.0;
        }

        // Bonus pour la ponctuation de fin de mot (virgule, point) qui induit une micro-pause
        if token.ends_with(',') || token.ends_with(';') {
            weight += 0.8;
        } else if token.ends_with('.') || token.ends_with('!') || token.ends_with('?') {
            weight += 1.2;
        }

        weights.push(weight);
    }

    let total_weight: f64 = weights.iter().sum();
    let mut result = Vec::with_capacity(raw_tokens.len());
    let mut current_word_start = start_time_ms;

    for (i, token) in raw_tokens.iter().enumerate() {
        let share = if total_weight > 0.0 {
            weights[i] / total_weight
        } else {
            1.0 / (raw_tokens.len() as f64)
        };

        let raw_duration = ((singing_duration as f64) * share).round() as i64;
        let word_duration = raw_duration.max(90); // Plancher minimal à 90ms

        let is_last = i == raw_tokens.len() - 1;
        let word_end = if is_last {
            (start_time_ms + singing_duration).max(current_word_start + 90)
        } else {
            current_word_start + word_duration
        };

        result.push(LyricWord {
            text: token.to_string(),
            start_time_ms: current_word_start,
            end_time_ms: word_end,
        });

        current_word_start = word_end;
    }

    result
}

fn parse_lrc_timestamp(tag: &str) -> Option<i64> {
    // Format attendu : mm:ss.xx ou mm:ss
    let parts: Vec<&str> = tag.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let minutes: i64 = parts[0].parse().ok()?;
    let normalized_sec = parts[1].replace(',', ".");
    let sec_parts: Vec<&str> = normalized_sec.split('.').collect();
    let seconds: i64 = sec_parts[0].parse().ok()?;

    let mut millis: i64 = 0;
    if sec_parts.len() > 1 {
        let frac_str = sec_parts[1];
        if frac_str.len() == 2 {
            millis = frac_str.parse::<i64>().ok()? * 10;
        } else if frac_str.len() == 3 {
            millis = frac_str.parse::<i64>().ok()?;
        } else if let Ok(val) = frac_str.parse::<i64>() {
            millis = val;
        }
    }

    Some(minutes * 60 * 1000 + seconds * 1000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lrc_basic_and_syllable_estimation() {
        let content = "[00:12.50] Première ligne\n[00:18.00] Deuxième ligne";
        let parsed = parse_lrc(content);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].start_time_ms, 12500);
        assert_eq!(parsed[0].end_time_ms, Some(18000));
        assert_eq!(parsed[0].text, "Première ligne");

        // Vérification de l'estimation syllabique
        assert_eq!(parsed[0].words.len(), 2);
        assert_eq!(parsed[0].words[0].text, "Première");
        assert_eq!(parsed[0].words[1].text, "ligne");
        assert_eq!(parsed[0].words[0].start_time_ms, 12500);
        assert!(parsed[0].words[0].end_time_ms > parsed[0].words[0].start_time_ms);
        assert_eq!(parsed[0].words[1].start_time_ms, parsed[0].words[0].end_time_ms);
        assert!(parsed[0].words[1].end_time_ms <= 18000);
    }

    #[test]
    fn test_parse_enhanced_lrc() {
        let content = "[00:10.00]<00:10.00>Hello <00:11.20>darkness <00:12.50>my\n[00:15.00] Next";
        let parsed = parse_lrc(content);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].text, "Hello darkness my");
        assert_eq!(parsed[0].words.len(), 3);
        assert_eq!(parsed[0].words[0].text, "Hello");
        assert_eq!(parsed[0].words[0].start_time_ms, 10000);
        assert_eq!(parsed[0].words[0].end_time_ms, 11200);

        assert_eq!(parsed[0].words[1].text, "darkness");
        assert_eq!(parsed[0].words[1].start_time_ms, 11200);
        assert_eq!(parsed[0].words[1].end_time_ms, 12500);

        assert_eq!(parsed[0].words[2].text, "my");
        assert_eq!(parsed[0].words[2].start_time_ms, 12500);
        assert_eq!(parsed[0].words[2].end_time_ms, 15000);
    }

    #[test]
    fn test_estimate_syllables_duration_bounds() {
        let words = estimate_syllables_for_line("Is this the real life? Is this just fantasy?", 0, 5000);
        assert!(!words.is_empty());
        assert_eq!(words[0].start_time_ms, 0);

        let last_word = words.last().unwrap();
        assert!(last_word.end_time_ms <= 5000);

        for w in &words {
            assert!(w.end_time_ms >= w.start_time_ms + 90);
        }
    }
}
