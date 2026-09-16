use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentMediaState {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: i64,
    pub position_ms: i64,
    pub is_playing: bool,
    pub playback_rate: f64,
    pub last_updated_ms: i64,
    pub source_app: String,
}

#[cfg(windows)]
pub mod windows_gsmtc {
    use super::CurrentMediaState;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };

    pub async fn get_current_session_state() -> Option<CurrentMediaState> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?.get().ok()?;

        // 1. Tente d'obtenir la session active
        let session: GlobalSystemMediaTransportControlsSession = if let Ok(s) = manager.GetCurrentSession() {
            s
        } else {
            // 2. Si aucune session courante (ex: musique en pause), inspecter toutes les sessions ouvertes
            let sessions = manager.GetSessions().ok()?;
            let count = sessions.Size().unwrap_or(0);
            let mut candidate = None;
            for i in 0..count {
                if let Ok(s) = sessions.GetAt(i) {
                    if let Ok(props) = s.TryGetMediaPropertiesAsync() {
                        if let Ok(p) = props.get() {
                            if !p.Title().unwrap_or_default().to_string().is_empty() {
                                candidate = Some(s);
                                break;
                            }
                        }
                    }
                }
            }
            candidate?
        };

        let raw_app_id = session.SourceAppUserModelId().unwrap_or_default().to_string();
        let source_app = format_app_name(&raw_app_id);

        let media_props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
        let title = media_props.Title().unwrap_or_default().to_string();
        let artist = media_props.Artist().unwrap_or_default().to_string();
        let album = media_props.AlbumTitle().unwrap_or_default().to_string();

        if title.is_empty() && artist.is_empty() {
            return None;
        }

        let timeline = session.GetTimelineProperties().ok()?;
        let duration_ms = timeline.EndTime().unwrap_or_default().Duration / 10_000;
        let position_ms = timeline.Position().unwrap_or_default().Duration / 10_000;

        let playback_info = session.GetPlaybackInfo().ok()?;
        let is_playing = playback_info.PlaybackStatus().unwrap_or(
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed,
        ) == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

        let playback_rate = playback_info
            .PlaybackRate()
            .ok()
            .and_then(|r| r.Value().ok())
            .unwrap_or(1.0);

        Some(CurrentMediaState {
            title,
            artist,
            album,
            duration_ms,
            position_ms,
            is_playing,
            playback_rate,
            last_updated_ms: chrono_or_instant_now(),
            source_app,
        })
    }

    fn format_app_name(raw_id: &str) -> String {
        let lower = raw_id.to_lowercase();
        if lower.contains("spotify") {
            "Spotify".to_string()
        } else if lower.contains("chrome") {
            "Google Chrome".to_string()
        } else if lower.contains("msedge") || lower.contains("edge") {
            "Microsoft Edge".to_string()
        } else if lower.contains("applemusic") || lower.contains("apple") {
            "Apple Music".to_string()
        } else if lower.contains("vlc") {
            "VLC".to_string()
        } else if lower.contains("firefox") {
            "Firefox".to_string()
        } else if lower.contains("brave") {
            "Brave".to_string()
        } else if raw_id.is_empty() {
            "Lecteur Windows".to_string()
        } else {
            raw_id.split('.').next().unwrap_or(raw_id).to_string()
        }
    }

    fn chrono_or_instant_now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

#[cfg(not(windows))]
pub mod fallback {
    use super::CurrentMediaState;

    pub async fn get_current_session_state() -> Option<CurrentMediaState> {
        None
    }
}
