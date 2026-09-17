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
    use crate::media::matcher::{
        calculate_music_score, extract_web_music_metadata, format_app_name,
        is_valid_music_candidate, is_web_browser, PlaybackType,
    };
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };

    struct SessionCandidate {
        session: GlobalSystemMediaTransportControlsSession,
        raw_app_id: String,
        raw_title: String,
        raw_artist: String,
        album: String,
        score: i32,
    }

    pub async fn get_current_session_state() -> Option<CurrentMediaState> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?.get().ok()?;

        // 1. Collecte et évaluation intelligente de toutes les sessions
        let mut candidates = Vec::new();

        let evaluate_session = |s: GlobalSystemMediaTransportControlsSession| -> Option<SessionCandidate> {
            let raw_app_id = s.SourceAppUserModelId().unwrap_or_default().to_string();
            let props_async = s.TryGetMediaPropertiesAsync().ok()?;
            let p = props_async.get().ok()?;

            let raw_title = p.Title().unwrap_or_default().to_string();
            let raw_artist = p.Artist().unwrap_or_default().to_string();
            let album = p.AlbumTitle().unwrap_or_default().to_string();

            if raw_title.trim().is_empty() && raw_artist.trim().is_empty() {
                return None;
            }

            let pb_type = match p.PlaybackType().ok().and_then(|r| r.Value().ok()) {
                Some(windows::Media::MediaPlaybackType::Music) => PlaybackType::Music,
                Some(windows::Media::MediaPlaybackType::Video) => PlaybackType::Video,
                _ => PlaybackType::Unknown,
            };

            let is_playing = s.GetPlaybackInfo().ok().map(|pb| {
                pb.PlaybackStatus().unwrap_or(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed)
                    == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
            }).unwrap_or(false);

            // Rejet catégorique si la session n'est pas un candidat musical valable (ex: tutoriels, gaming, vlogs)
            if !is_valid_music_candidate(&raw_app_id, &raw_title, &raw_artist, pb_type) {
                return None;
            }

            let score = calculate_music_score(
                &raw_app_id,
                &raw_title,
                &raw_artist,
                &album,
                is_playing,
                pb_type,
            );

            Some(SessionCandidate {
                session: s,
                raw_app_id,
                raw_title,
                raw_artist,
                album,
                score,
            })
        };

        if let Ok(sessions) = manager.GetSessions() {
            let count = sessions.Size().unwrap_or(0);
            for i in 0..count {
                if let Ok(s) = sessions.GetAt(i) {
                    if let Some(candidate) = evaluate_session(s) {
                        candidates.push(candidate);
                    }
                }
            }
        }

        // Si GetSessions() n'a rien trouvé, tenter GetCurrentSession()
        if candidates.is_empty() {
            if let Ok(cur) = manager.GetCurrentSession() {
                if let Some(candidate) = evaluate_session(cur) {
                    candidates.push(candidate);
                }
            }
        }

        // Aucun candidat musical valide en cours (ex: uniquement une vidéo de dev/tuto sur YouTube)
        if candidates.is_empty() {
            return None;
        }

        // Trier par score décroissant : le meilleur candidat musical l'emporte toujours
        candidates.sort_by(|a, b| b.score.cmp(&a.score));
        let best = candidates.remove(0);

        let source_app = format_app_name(&best.raw_app_id, &best.raw_title);
        let (title, artist) = if is_web_browser(&best.raw_app_id) {
            extract_web_music_metadata(&best.raw_title, &best.raw_artist)
        } else {
            (best.raw_title.trim().to_string(), best.raw_artist.trim().to_string())
        };

        if title.is_empty() && artist.is_empty() {
            return None;
        }

        let timeline = best.session.GetTimelineProperties().ok()?;
        let duration_ms = timeline.EndTime().unwrap_or_default().Duration / 10_000;
        let raw_position_ms = timeline.Position().unwrap_or_default().Duration / 10_000;

        let playback_info = best.session.GetPlaybackInfo().ok()?;
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
            album: best.album,
            duration_ms,
            position_ms,
            is_playing,
            playback_rate,
            last_updated_ms: now_unix_ms,
            source_app,
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
