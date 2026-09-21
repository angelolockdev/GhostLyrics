use crate::lyrics::{LyricsResponse, LyricsService};
use crate::media::{get_current_session_state, CurrentMediaState};
use crate::window::set_click_through;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub lyrics_service: LyricsService,
}

#[tauri::command]
pub async fn get_media_state() -> Result<Option<CurrentMediaState>, String> {
    Ok(get_current_session_state().await)
}

#[tauri::command]
pub async fn fetch_song_lyrics(
    title: String,
    artist: String,
    album: Option<String>,
    duration_sec: Option<f64>,
    state: State<'_, AppState>,
) -> Result<LyricsResponse, String> {
    state
        .lyrics_service
        .fetch_lyrics(&title, &artist, album.as_deref(), duration_sec)
        .await
}

#[tauri::command]
pub fn set_overlay_click_through(app: AppHandle, enabled: bool) -> Result<(), String> {
    set_click_through(&app, enabled)
}

#[tauri::command]
pub fn show_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.unminimize();
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        // Recréation dynamique si la fenêtre a été fermée ou détruite
        let window = tauri::WebviewWindowBuilder::new(
            &app,
            "settings",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("GhostLyrics — Paramètres")
        .inner_size(660.0, 660.0)
        .min_inner_size(480.0, 400.0)
        .resizable(true)
        .decorations(true)
        .build()
        .map_err(|e| format!("Impossible de créer la fenêtre des paramètres : {}", e))?;

        let _ = window.unminimize();
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[tauri::command]
pub fn minimize_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("overlay") {
        window.hide().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Fenêtre overlay introuvable".to_string())
    }
}

#[tauri::command]
pub fn toggle_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("overlay") {
        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
        } else {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
        }
        Ok(())
    } else {
        Err("Fenêtre overlay introuvable".to_string())
    }
}

#[tauri::command]
pub fn close_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn check_for_updates() -> Result<crate::updater::UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION");
    crate::updater::check_github_update(current_version).await
}

#[tauri::command]
pub async fn download_and_install_update(app: AppHandle, download_url: String) -> Result<(), String> {
    crate::updater::download_and_install(app, &download_url).await
}
