use std::sync::Arc;

use tauri::State;

use crate::state::{AppCore, MessagePinStatus};

#[tauri::command]
pub async fn get_message_status(core: State<'_, Arc<AppCore>>) -> Result<MessagePinStatus, String> {
    Ok(core.message_status().await)
}

#[tauri::command]
pub async fn pin_message(
    core: State<'_, Arc<AppCore>>,
    message_id: Option<String>,
) -> Result<MessagePinStatus, String> {
    core.pin_current_message(message_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn unpin_message(core: State<'_, Arc<AppCore>>) -> Result<MessagePinStatus, String> {
    Ok(core.unpin_message().await)
}
