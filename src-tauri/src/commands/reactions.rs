use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::{
    bot::ReactionAccessOptions,
    config::AppConfig,
    model::RelayEvent,
    reaction_protection, reaction_trim,
    reactions::{self, ReactionRuntime, ReactionSettings},
    state::AppCore,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReactionsResult {
    pub discord_pending: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReactionSoundResult {
    pub sound_id: Option<String>,
    pub trim_token: Option<String>,
    pub filename: String,
    pub duration_seconds: f64,
}

#[tauri::command]
pub async fn get_reactions(core: State<'_, Arc<AppCore>>) -> Result<ReactionSettings, String> {
    Ok(core.config.read().await.reactions.clone())
}

#[tauri::command]
pub async fn get_reaction_access_options(
    core: State<'_, Arc<AppCore>>,
) -> Result<ReactionAccessOptions, String> {
    crate::bot::get_reaction_access_options(&core)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn save_reactions(
    core: State<'_, Arc<AppCore>>,
    mut settings: ReactionSettings,
) -> Result<SaveReactionsResult, String> {
    let _schema_sync = core.custom_command_sync.lock().await;
    let mut reaction_runtime = core.reactions.lock().await;
    let previous = core.config.read().await.clone();
    let normalized = reaction_protection::normalize(
        &settings.protected_message_id,
        &settings.protected_channel_id,
        &previous.reactions.protected_channel_id,
        &previous.reactions.protected_message_id,
        &settings.allowed_channel_ids,
    )?;
    settings.protected_channel_id = normalized.channel_id;
    settings.protected_message_id = normalized.message_id;
    if reaction_protection::requires_verification(
        &previous.reactions.protected_channel_id,
        &previous.reactions.protected_message_id,
        &settings.protected_channel_id,
        &settings.protected_message_id,
    ) {
        reaction_protection::verify(
            &core,
            &settings.protected_channel_id,
            &settings.protected_message_id,
        )
        .await?;
    }
    let mut candidate = previous.clone();
    candidate.reactions = settings.clone();
    candidate.validate().map_err(|error| error.to_string())?;
    for definition in &settings.definitions {
        if !core
            .data_directory
            .join("reactions")
            .join(&definition.sound_id)
            .is_file()
        {
            return Err("Reaction sound unavailable.".into());
        }
        if let Some(id) = &definition.visual_id {
            let item = core
                .media_library
                .get(id)
                .map_err(|error| error.to_string())?;
            if !matches!(
                item.kind,
                crate::model::MediaKind::Image | crate::model::MediaKind::Gif
            ) || !item.content_type.starts_with("image/")
            {
                return Err("Reaction visuals must be library images or GIFs.".into());
            }
        }
    }

    let mut discord_pending = false;
    let mut discord_schema_synced = false;
    if discord_sync_available(&core).await {
        match crate::bot::sync_relay_command_schema(&core, &candidate).await {
            Ok(()) => discord_schema_synced = true,
            Err(_) if !discord_sync_available(&core).await => {
                discord_pending = true;
            }
            Err(_) => {
                return Err("Unable to synchronize reaction commands with Discord.".into());
            }
        }
    } else {
        discord_pending = true;
    }

    if core
        .update_config(|config| config.reactions = settings.clone())
        .await
        .is_err()
    {
        let schema_error = rollback_discord_schema(&core, &previous, discord_schema_synced).await;
        if let Some(error) = schema_error {
            core.bot_status.write().await.error = Some(error.clone());
            return Err(format!(
                "Unable to save reactions and restore the previous runtime state: {error}"
            ));
        }
        return Err("Unable to save reaction settings.".into());
    }

    reaction_runtime.retain_pending(&settings);
    if active_reaction_must_stop(&reaction_runtime, &settings) {
        reaction_runtime.clear_pending();
        reaction_runtime.active = None;
        let _ = core.relay_tx.send(RelayEvent::Reaction(None));
    }
    Ok(SaveReactionsResult { discord_pending })
}
#[tauri::command]
pub async fn import_reaction_sound(
    core: State<'_, Arc<AppCore>>,
) -> Result<Option<ImportReactionSoundResult>, String> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .add_filter(
            "Audio",
            &["mp3", "flac", "wav", "ogg", "opus", "m4a", "aac"],
        )
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    let path = file.path().to_path_buf();
    let source = tokio::task::spawn_blocking(move || reaction_trim::inspect_source(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    if source.duration_seconds() <= reaction_trim::MAX_REACTION_SECONDS {
        let directory = core.data_directory.join("reactions");
        let source_path = source.path().to_path_buf();
        let sound_id =
            tokio::task::spawn_blocking(move || reactions::import_sound(&directory, &source_path))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
        core.reaction_trim.lock().await.clear();
        return Ok(Some(ImportReactionSoundResult {
            sound_id: Some(sound_id),
            trim_token: None,
            filename: source.filename().to_owned(),
            duration_seconds: source.duration_seconds(),
        }));
    }

    let filename = source.filename().to_owned();
    let duration_seconds = source.duration_seconds();
    let trim_token = core.reaction_trim.lock().await.replace(source);
    Ok(Some(ImportReactionSoundResult {
        sound_id: None,
        trim_token: Some(trim_token),
        filename,
        duration_seconds,
    }))
}
#[tauri::command]
pub async fn preview_reaction_trim(
    core: State<'_, Arc<AppCore>>,
    token: String,
    start_seconds: f64,
    end_seconds: f64,
) -> Result<Vec<u8>, String> {
    let pending = core
        .reaction_trim
        .lock()
        .await
        .resolve(&token)
        .map_err(|e| e.to_string())?;
    let source = pending.source().clone();
    let bytes = tokio::task::spawn_blocking(move || {
        reaction_trim::render_pcm_wav(&source, start_seconds, end_seconds)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    core.reaction_trim
        .lock()
        .await
        .resolve(&token)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}
#[tauri::command]
pub async fn finish_reaction_trim(
    core: State<'_, Arc<AppCore>>,
    token: String,
    start_seconds: f64,
    end_seconds: f64,
) -> Result<String, String> {
    let pending = core
        .reaction_trim
        .lock()
        .await
        .resolve(&token)
        .map_err(|e| e.to_string())?;
    let source = pending.source().clone();
    let bytes = tokio::task::spawn_blocking(move || {
        reaction_trim::render_pcm_wav(&source, start_seconds, end_seconds)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let pending = core
        .reaction_trim
        .lock()
        .await
        .take(&token)
        .map_err(|e| e.to_string())?;
    let directory = core.data_directory.join("reactions");
    let result = tokio::task::spawn_blocking(move || {
        reaction_trim::import_rendered_sound(&directory, &bytes)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string());
    if result.is_err() {
        core.reaction_trim.lock().await.restore_if_empty(pending);
    }
    result
}
#[tauri::command]
pub async fn cancel_reaction_trim(
    core: State<'_, Arc<AppCore>>,
    token: String,
) -> Result<(), String> {
    core.reaction_trim
        .lock()
        .await
        .cancel(&token)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn trigger_reaction(
    core: State<'_, Arc<AppCore>>,
    id: String,
) -> Result<reactions::TriggerResult, String> {
    reactions::trigger(&core, &id, None)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn stop_reaction(core: State<'_, Arc<AppCore>>) -> Result<(), String> {
    reactions::stop(&core, None).await;
    Ok(())
}
#[tauri::command]
pub async fn preview_reaction_sound(
    core: State<'_, Arc<AppCore>>,
    id: String,
) -> Result<Vec<u8>, String> {
    if !reactions::valid_id(&id) {
        return Err("Invalid sound ID.".into());
    }
    let directory = core.data_directory.join("reactions");
    tokio::task::spawn_blocking(move || reactions::read_sound(&directory, &id))
        .await
        .map_err(|_| "Sound unavailable.".to_string())?
        .map_err(|_| "Sound unavailable.".to_string())
}
// Keep a local audio receiver alive even when the main panel is hidden.
// This window never displays reaction visuals or appears in the taskbar.
pub async fn restore_audio(app: &AppHandle, core: &AppCore) -> Result<(), String> {
    let port = core.config.read().await.port;
    let url = format!("http://127.0.0.1:{port}/reactions?client=widget&audioOnly=1")
        .parse()
        .map_err(|_| "Invalid reaction audio URL.")?;
    if let Some(window) = app.get_webview_window("reaction-widget") {
        return window.navigate(url).map_err(|error| error.to_string());
    }
    tauri::WebviewWindowBuilder::new(app, "reaction-widget", tauri::WebviewUrl::External(url))
        .title("Relay reaction audio")
        .inner_size(1.0, 1.0)
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .decorations(false)
        .resizable(false)
        .build()
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn discord_sync_available(core: &AppCore) -> bool {
    if !core.bot_status.read().await.connected {
        return false;
    }
    core.bot_runtime.lock().await.is_some()
}

async fn rollback_discord_schema(
    core: &Arc<AppCore>,
    previous: &AppConfig,
    synced: bool,
) -> Option<String> {
    if !synced {
        return None;
    }
    crate::bot::sync_relay_command_schema(core, previous)
        .await
        .err()
        .map(|_| "Unable to restore the previous Discord command schema.".to_string())
}

fn active_reaction_must_stop(runtime: &ReactionRuntime, settings: &ReactionSettings) -> bool {
    let Some(active) = runtime.active.as_ref() else {
        return false;
    };
    !settings.enabled
        || !settings.definitions.iter().any(|definition| {
            definition.id == active.reaction.id
                && definition.enabled
                && definition.sound_id == active.reaction.sound_id
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactions::{ReactionDefinition, ReactionPlayback};

    fn definition(enabled: bool, sound_id: &str) -> ReactionDefinition {
        ReactionDefinition {
            id: "a".repeat(32),
            name: "Test".into(),
            sound_id: sound_id.into(),
            visual_id: None,
            volume: 70,
            enabled,
        }
    }

    #[test]
    fn active_reaction_is_stopped_when_removed_or_disabled() {
        let sound_id = "b".repeat(32);
        let mut runtime = ReactionRuntime::default();
        runtime.active = Some(ReactionPlayback {
            playback_id: "c".repeat(32),
            reaction: definition(true, &sound_id),
            ends_at: 1,
            music_percent: 25,
        });

        let settings = ReactionSettings {
            enabled: true,
            definitions: vec![definition(false, &sound_id)],
            ..ReactionSettings::default()
        };
        assert!(active_reaction_must_stop(&runtime, &settings));

        let settings = ReactionSettings {
            enabled: true,
            definitions: vec![definition(true, &"d".repeat(32))],
            ..ReactionSettings::default()
        };
        assert!(active_reaction_must_stop(&runtime, &settings));
    }

    #[test]
    fn legacy_widget_setting_is_ignored_and_removed_on_save() {
        let settings: ReactionSettings = serde_json::from_value(serde_json::json!({
            "enabled": true, "widgetEnabled": true, "globalCooldownSeconds": 42
        }))
        .unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.global_cooldown_seconds, 42);
        assert!(
            serde_json::to_value(settings)
                .unwrap()
                .get("widgetEnabled")
                .is_none()
        );
    }

    #[test]
    fn save_result_uses_camel_case_pending_flag() {
        let value = serde_json::to_value(SaveReactionsResult {
            discord_pending: true,
        })
        .expect("serializable save result");
        assert_eq!(value["discordPending"], true);
    }
}
