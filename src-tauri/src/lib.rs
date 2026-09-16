pub mod commands;
pub mod lyrics;
pub mod media;
pub mod updater;
pub mod window;

use commands::{
    check_for_updates, close_app, download_and_install_update, fetch_song_lyrics, get_media_state,
    minimize_overlay, set_overlay_click_through, show_settings_window, toggle_overlay, AppState,
};
use lyrics::LyricsService;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
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
            minimize_overlay,
            toggle_overlay,
            close_app,
            check_for_updates,
            download_and_install_update,
        ])
        .setup(|app| {
            // Configuration de l'overlay au démarrage
            if let Some(overlay_win) = app.get_webview_window("overlay") {
                let _ = overlay_win.set_shadow(false);
            }

            // Construction du menu System Tray (zone de notification Windows)
            let toggle_i = MenuItem::with_id(
                app,
                "toggle_overlay",
                "Afficher / Masquer l'overlay",
                true,
                None::<&str>,
            )?;
            let settings_i = MenuItem::with_id(
                app,
                "settings",
                "Paramètres",
                true,
                None::<&str>,
            )?;
            let quit_i = MenuItem::with_id(
                app,
                "quit",
                "Quitter GhostLyrics",
                true,
                None::<&str>,
            )?;
            let tray_menu = Menu::with_items(app, &[&toggle_i, &settings_i, &quit_i])?;

            // Initialisation de l'icône dans la zone des icônes cachées
            if let Some(icon) = app.default_window_icon() {
                let _tray = TrayIconBuilder::new()
                    .icon(icon.clone())
                    .tooltip("GhostLyrics")
                    .menu(&tray_menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "toggle_overlay" => {
                            if let Some(win) = app.get_webview_window("overlay") {
                                if win.is_visible().unwrap_or(false) {
                                    let _ = win.hide();
                                } else {
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                }
                            }
                        }
                        "settings" => {
                            if let Some(win) = app.get_webview_window("settings") {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(win) = app.get_webview_window("overlay") {
                                if win.is_visible().unwrap_or(false) {
                                    let _ = win.hide();
                                } else {
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                }
                            }
                        }
                    })
                    .build(app)?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Erreur lors de l'exécution de l'application GhostLyrics");
}
