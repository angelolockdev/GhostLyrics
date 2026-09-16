use tauri::{AppHandle, Manager};

pub fn set_click_through(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("overlay") {
        window
            .set_ignore_cursor_events(enabled)
            .map_err(|e| format!("Impossible de changer le mode click-through: {}", e))?;
        Ok(())
    } else {
        Err("Fenêtre overlay non trouvée".to_string())
    }
}
