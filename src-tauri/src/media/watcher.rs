use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentMediaState {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: i64,
    pub position_ms: i64,
    pub is_playing: boolean_compat::BoolCompat,
    pub playback_rate: f64,
    pub last_updated_ms: i64,
}

mod boolean_compat {
    pub type BoolCompat = bool;
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
        let session: GlobalSystemMediaTransportControlsSession = manager.GetCurrentSession().ok()?;

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
        })
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
