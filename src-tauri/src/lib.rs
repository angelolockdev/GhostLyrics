pub mod commands;
pub mod lyrics;
pub mod media;
pub mod window;

use commands::{
    fetch_song_lyrics, get_media_state, set_overlay_click_through, show_settings_window, AppState,
};
use lyrics::LyricsService;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let lyrics_service = LyricsService::new();

    tauri::Builder::default()
        .manage(AppState { lyrics_service })
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_media_state,
            fetch_song_lyrics,
            set_overlay_click_through,
            show_settings_window,
        ])
        .setup(|app| {
            // Empêche la fermeture brutale et garde l'overlay prêt
            if let Some(overlay_win) = app.get_webview_window("overlay") {
                let _ = overlay_win.set_shadow(false);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Erreur lors de l'exécution de l'application GhostLyrics");
}
