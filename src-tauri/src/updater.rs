use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_name: String,
    pub release_notes: String,
    pub download_url: String,
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f64,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    #[allow(dead_code)]
    size: u64,
}

/// Vérifie la présence d'une nouvelle version sur GitHub Releases
pub async fn check_github_update(current_version: &str) -> Result<UpdateInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent("GhostLyrics-Updater/0.1.4 (https://github.com/angelolockdev/GhostLyrics)")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Erreur création client HTTP: {}", e))?;

    let url = "https://api.github.com/repos/angelolockdev/GhostLyrics/releases/latest";
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Impossible de joindre GitHub Releases: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("GitHub API a retourné le statut: {}", res.status()));
    }

    let release = res
        .json::<GitHubRelease>()
        .await
        .map_err(|e| format!("Erreur décodage JSON GitHub: {}", e))?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    let has_update = is_newer_version(current_version, &latest_version);

    // Recherche de l'asset d'installation Windows (.exe NSIS de préférence, sinon .msi)
    let download_url = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".exe") && a.name.contains("setup"))
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".exe")))
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".msi")))
        .map(|a| a.browser_download_url.clone())
        .unwrap_or_default();

    Ok(UpdateInfo {
        current_version: current_version.to_string(),
        latest_version,
        has_update,
        release_name: release.name.unwrap_or_else(|| release.tag_name.clone()),
        release_notes: release.body.unwrap_or_default(),
        download_url,
        published_at: release.published_at.unwrap_or_default(),
    })
}

/// Télécharge le fichier d'installation en émettant la progression, puis le lance et quitte l'app
pub async fn download_and_install<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    download_url: &str,
) -> Result<(), String> {
    if download_url.is_empty() {
        return Err("URL de téléchargement introuvable pour cette version".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("GhostLyrics-Updater/0.1.4")
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Erreur création client HTTP: {}", e))?;

    let mut res = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("Erreur lancement téléchargement: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Échec téléchargement: {}", res.status()));
    }

    let total_bytes = res.content_length().unwrap_or(0);
    let mut downloaded_bytes: u64 = 0;

    let temp_dir = std::env::temp_dir();
    let file_ext = if download_url.ends_with(".msi") { ".msi" } else { ".exe" };
    let installer_path: PathBuf = temp_dir.join(format!("GhostLyrics_Update_Setup{}", file_ext));

    let mut file = File::create(&installer_path)
        .map_err(|e| format!("Impossible de créer le fichier temporaire: {}", e))?;

    while let Some(chunk) = res.chunk().await.map_err(|e| format!("Erreur transfert: {}", e))? {
        file.write_all(&chunk)
            .map_err(|e| format!("Erreur écriture disque: {}", e))?;

        downloaded_bytes += chunk.len() as u64;
        let percent = if total_bytes > 0 {
            (downloaded_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        let _ = app.emit(
            "update_download_progress",
            DownloadProgress {
                downloaded_bytes,
                total_bytes,
                percent: (percent * 10.0).round() / 10.0,
            },
        );
    }

    drop(file);

    // Lancement de l'installeur Windows
    #[cfg(target_os = "windows")]
    {
        if file_ext == ".msi" {
            std::process::Command::new("msiexec")
                .args(["/i", &installer_path.to_string_lossy(), "/passive"])
                .spawn()
                .map_err(|e| format!("Impossible de démarrer l'installeur MSI: {}", e))?;
        } else {
            std::process::Command::new(&installer_path)
                .args(["/passive"])
                .spawn()
                .map_err(|e| format!("Impossible de démarrer l'installeur EXE: {}", e))?;
        }

        // Émission d'un événement final avant fermeture
        let _ = app.emit("update_ready_to_restart", ());

        // Pause de 500ms puis fermeture pour laisser l'installeur prendre la main
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        app.exit(0);
    }

    Ok(())
}

/// Compare deux versions SemVer (ex: "0.1.5" > "0.1.4")
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse_parts = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['.', '-'])
            .filter_map(|part| part.parse::<u64>().ok())
            .collect()
    };

    let cur_parts = parse_parts(current);
    let lat_parts = parse_parts(latest);

    let max_len = cur_parts.len().max(lat_parts.len());
    for i in 0..max_len {
        let c = cur_parts.get(i).copied().unwrap_or(0);
        let l = lat_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.1.3", "0.1.4"));
        assert!(is_newer_version("0.1.4", "0.1.5"));
        assert!(is_newer_version("0.1.4", "1.0.0"));
        assert!(is_newer_version("v0.1.4", "v0.2.0"));
        assert!(is_newer_version("0.1.4", "0.1.10"));

        assert!(!is_newer_version("0.1.4", "0.1.4"));
        assert!(!is_newer_version("0.1.4", "0.1.3"));
        assert!(!is_newer_version("1.0.0", "0.9.9"));
    }
}
