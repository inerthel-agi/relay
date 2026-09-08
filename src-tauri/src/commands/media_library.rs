use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tauri::State;

use crate::{
    media_library::{self, MediaLibraryAsset, MediaLibraryItem, MediaLibraryUsage},
    model::{AuthorIdentity, MediaEvent},
    state::AppCore,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaLibrarySnapshot {
    pub items: Vec<MediaLibraryItem>,
    pub total_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaLibraryPreview {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

#[tauri::command]
pub fn get_media_library(core: State<'_, Arc<AppCore>>) -> Result<MediaLibrarySnapshot, String> {
    let library = &core.inner().media_library;
    let items = library.list().map_err(super::display_error)?;
    let usage = library.usage().map_err(super::display_error)?;
    Ok(snapshot(items, usage))
}

#[tauri::command]
pub async fn import_library_media(
    core: State<'_, Arc<AppCore>>,
) -> Result<Option<MediaLibraryItem>, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter(
            "Images and videos",
            &[
                "png", "jpg", "jpeg", "gif", "webp", "bmp", "apng", "mp4", "webm", "mov", "m4v",
                "ogv",
            ],
        )
        .pick_file()
        .await;
    let Some(file) = file else {
        return Ok(None);
    };
    let path = file.path().to_path_buf();
    let config = core.inner().config.read().await.clone();
    let library = Arc::clone(&core.inner().media_library);
    library
        .import_file(path, Some(config), None)
        .await
        .map(Some)
        .map_err(super::display_error)
}

#[tauri::command]
pub async fn save_history_to_library(
    core: State<'_, Arc<AppCore>>,
    message_id: String,
    media_url: String,
) -> Result<MediaLibraryItem, String> {
    super::validate_message_id(&message_id)?;
    if media_url.is_empty() || media_url.len() > 2_048 {
        return Err("Invalid media URL.".into());
    }
    let event = core
        .inner()
        .history
        .read()
        .await
        .iter()
        .find(|event| {
            event.message_id == message_id
                && (event.url == media_url || event.proxy_url == media_url)
        })
        .cloned()
        .ok_or_else(|| "The media is no longer in history.".to_string())?;
    if matches!(event.kind, crate::model::MediaKind::Audio) {
        return Err("Only images, GIFs, and videos can be saved in the media library.".into());
    }
    let bytes_and_type = super::history_media_bytes(core.inner(), &event).await?;
    let config = core.inner().config.read().await.clone();
    Arc::clone(&core.inner().media_library)
        .import_bytes(
            event.filename,
            event.kind,
            bytes_and_type.1,
            bytes_and_type.0,
            Some(config),
            event.text,
        )
        .await
        .map_err(super::display_error)
}

#[tauri::command]
pub fn rename_library_media(
    core: State<'_, Arc<AppCore>>,
    id: String,
    name: String,
) -> Result<MediaLibraryItem, String> {
    core.inner()
        .media_library
        .rename(&id, &name)
        .map_err(super::display_error)
}

#[tauri::command]
pub async fn delete_library_media(core: State<'_, Arc<AppCore>>, id: String) -> Result<(), String> {
    if core
        .inner()
        .config
        .read()
        .await
        .reactions
        .definitions
        .iter()
        .any(|reaction| reaction.visual_id.as_deref() == Some(id.as_str()))
    {
        return Err("Remove this media from its reaction before deleting it.".into());
    }
    core.inner()
        .media_library
        .delete(&id)
        .map_err(super::display_error)
}

#[tauri::command]
pub async fn play_library_media(core: State<'_, Arc<AppCore>>, id: String) -> Result<(), String> {
    let library = Arc::clone(&core.inner().media_library);
    let asset = library.read_asset(&id).map_err(super::display_error)?;
    let config = core.inner().config.read().await.clone();
    media_library::validate_media_bytes(
        &asset.item.name,
        asset.item.kind,
        &asset.content_type,
        &asset.bytes,
        Some(&config),
        None,
    )
    .await
    .map_err(super::display_error)?;
    let url = library_asset_url(config.port, &asset.item.id);
    core.inner()
        .publish_media(library_event(asset.item, url))
        .await;
    Ok(())
}

#[tauri::command]
pub fn preview_library_asset(
    core: State<'_, Arc<AppCore>>,
    id: String,
) -> Result<MediaLibraryPreview, String> {
    let asset = core
        .inner()
        .media_library
        .read_asset(&id)
        .map_err(super::display_error)?;
    Ok(preview(asset))
}

pub fn library_asset_url(port: u16, id: &str) -> String {
    format!("http://127.0.0.1:{port}/library-asset/{id}")
}

fn snapshot(items: Vec<MediaLibraryItem>, usage: MediaLibraryUsage) -> MediaLibrarySnapshot {
    MediaLibrarySnapshot {
        items,
        total_bytes: usage.bytes,
    }
}

fn preview(asset: MediaLibraryAsset) -> MediaLibraryPreview {
    MediaLibraryPreview {
        bytes: asset.bytes,
        content_type: asset.content_type,
    }
}

fn library_event(item: MediaLibraryItem, url: String) -> MediaEvent {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    MediaEvent {
        kind: item.kind,
        url: url.clone(),
        proxy_url: url,
        filename: item.name,
        content_type: item.content_type,
        artwork_id: None,
        audio_id: None,
        cached_media_id: None,
        title: None,
        artist: None,
        text: None,
        author: AuthorIdentity {
            username: "Relay library".into(),
            display_avatar_url: String::new(),
        },
        timestamp,
        message_id: format!("library-{}", item.id),
    }
}
