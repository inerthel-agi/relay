use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::{
    music::{MusicQueueDirection, MusicQueueMove},
    state::AppCore,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicQueueItem {
    pub id: String,
    pub title: String,
    pub author: String,
}

#[tauri::command]
pub async fn get_music_queue(core: State<'_, Arc<AppCore>>) -> Result<Vec<MusicQueueItem>, String> {
    Ok(core
        .music
        .lock()
        .await
        .pending_events()
        .into_iter()
        .map(|music| MusicQueueItem {
            id: music.playback_id,
            title: music.title,
            author: music.requested_by,
        })
        .collect())
}

#[tauri::command]
pub async fn move_music_queue(
    core: State<'_, Arc<AppCore>>,
    id: String,
    direction: String,
) -> Result<(), String> {
    let direction = match direction.trim().to_ascii_lowercase().as_str() {
        "up" => MusicQueueDirection::Up,
        "down" => MusicQueueDirection::Down,
        _ => return Err("Queue direction must be up or down.".into()),
    };
    match core.move_pending_music(&id, direction).await {
        MusicQueueMove::Moved { .. } | MusicQueueMove::AtBoundary => {
            crate::bot::refresh_pending_music_cards(&core).await;
            Ok(())
        }
        MusicQueueMove::NotFound | MusicQueueMove::Current => {
            Err("This track is no longer waiting in the music queue.".into())
        }
        MusicQueueMove::SchedulerUnavailable => {
            Err("The music queue changed before the move could be applied.".into())
        }
    }
}

#[tauri::command]
pub async fn remove_music_queue(core: State<'_, Arc<AppCore>>, id: String) -> Result<(), String> {
    if !core.remove_pending_music(&id).await {
        return Err("This track is no longer waiting in the music queue.".into());
    }
    crate::bot::refresh_pending_music_cards(&core).await;
    Ok(())
}

#[tauri::command]
pub async fn save_music_queue_settings(
    core: State<'_, Arc<AppCore>>,
    max_pending_per_user: u8,
    reject_duplicate_pending: bool,
) -> Result<(), String> {
    if max_pending_per_user > crate::music::MUSIC_MAX_PENDING_PER_USER_LIMIT {
        return Err("Music requests per member must be between 0 and 10.".into());
    }
    core.update_config(|config| {
        config.music_max_pending_per_user = max_pending_per_user;
        config.music_reject_duplicate_pending = reject_duplicate_pending;
    })
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}
