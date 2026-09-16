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
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Fenêtre de paramètres introuvable".to_string())
    }
}
