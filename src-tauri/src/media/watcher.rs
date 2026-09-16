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
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };

    pub async fn get_current_session_state() -> Option<CurrentMediaState> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?.get().ok()?;

        // 1. Inspecte toutes les sessions pour trouver en priorité celle qui est en cours de lecture
        let sessions = manager.GetSessions().ok();
        let mut playing_session = None;
        let mut any_valid_session = None;

        if let Some(ref list) = sessions {
            let count = list.Size().unwrap_or(0);
            for i in 0..count {
                if let Ok(s) = list.GetAt(i) {
                    if let Ok(props) = s.TryGetMediaPropertiesAsync() {
                        if let Ok(p) = props.get() {
                            let title = p.Title().unwrap_or_default().to_string();
                            if !title.is_empty() {
                                let is_playing = s.GetPlaybackInfo().ok().map(|pb| {
                                    pb.PlaybackStatus().unwrap_or(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed)
                                        == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                                }).unwrap_or(false);

                                if is_playing && playing_session.is_none() {
                                    playing_session = Some(s.clone());
                                }
                                if any_valid_session.is_none() {
                                    any_valid_session = Some(s);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Priorité : 1. Session active en lecture, 2. GetCurrentSession(), 3. Première session avec titre
        let session = if let Some(ps) = playing_session {
            ps
        } else if let Ok(cur) = manager.GetCurrentSession() {
            cur
        } else if let Some(avs) = any_valid_session {
            avs
        } else {
            return None;
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
        let raw_position_ms = timeline.Position().unwrap_or_default().Duration / 10_000;

        let playback_info = session.GetPlaybackInfo().ok()?;
        let is_playing = playback_info.PlaybackStatus().unwrap_or(
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed,
        ) == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

        let playback_rate = playback_info
            .PlaybackRate()
            .ok()
            .and_then(|r| r.Value().ok())
            .unwrap_or(1.0);

        let now_unix_ms = chrono_or_instant_now();
        let last_updated_universal = timeline.LastUpdatedTime().map(|dt| dt.UniversalTime).unwrap_or(0);

        // Windows GSMTC rapporte Position au moment de LastUpdatedTime (100ns depuis 1601-01-01).
        // 116444736000000000 est le décalage vers le 1er janvier 1970 (Unix Epoch).
        let position_ms = if is_playing && last_updated_universal > 116_444_736_000_000_000 {
            let last_updated_unix_ms = (last_updated_universal - 116_444_736_000_000_000) / 10_000;
            let elapsed_ms = now_unix_ms - last_updated_unix_ms;
            if elapsed_ms >= 0 && elapsed_ms < 600_000 {
                let adjusted = raw_position_ms + (elapsed_ms as f64 * playback_rate) as i64;
                if duration_ms > 0 {
                    adjusted.min(duration_ms)
                } else {
                    adjusted
                }
            } else {
                raw_position_ms
            }
        } else {
            raw_position_ms
        };

        Some(CurrentMediaState {
            title,
            artist,
            album,
            duration_ms,
            position_ms,
            is_playing,
            playback_rate,
            last_updated_ms: now_unix_ms,
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
