use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub start_time_ms: i64,
    pub end_time_ms: Option<i64>,
    pub text: String,
}

/// Parse le contenu brut d'un fichier .lrc et retourne une liste triée de lignes avec timestamps en millisecondes.
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

        let text = parts.last().unwrap_or(&"").trim().to_string();

        for part in &parts[..parts.len() - 1] {
            if let Some(tag) = part.strip_prefix('[') {
                if let Some(ms) = parse_lrc_timestamp(tag) {
                    lines.push(LyricLine {
                        start_time_ms: ms,
                        end_time_ms: None,
                        text: text.clone(),
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

    lines
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
    fn test_parse_lrc_basic() {
        let content = "[00:12.50] Première ligne\n[00:18.00] Deuxième ligne";
        let parsed = parse_lrc(content);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].start_time_ms, 12500);
        assert_eq!(parsed[0].end_time_ms, Some(18000));
        assert_eq!(parsed[0].text, "Première ligne");
        assert_eq!(parsed[1].start_time_ms, 18000);
    }
}
