use std::{path::Path, sync::Arc};
pub mod media_library;
pub mod message_pin;
pub mod moderation;
pub mod music_queue;
pub mod reactions;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::{
    artwork,
    bot::{
        apply_bot_presence, invite_url, refresh_channel_list, start_bot, sync_relay_command_schema,
    },
    config::{AppConfig, DEFAULT_SKIP_SHORTCUT, HoneypotAction, OutputGeometry},
    credentials::{
        CredentialStatus, DiscordCredentials, credential_status, load_discord_credentials,
        save_discord_credentials, save_youtube_api_key,
    },
    custom_commands::CustomCommandDefinition,
    model::{
        AudioControlAction, AudioControlEvent, AuthorIdentity, BotStatus, ChannelSummary,
        GuildTagIdentity, InterfacePreferences, MediaEvent, MediaKind, NotificationEvent,
        OutputTestEvent, OutputTestTarget, PendingMedia, RelayEvent, ServerStatus, StickerEvent,
        VisualSegment,
    },
    notification_widget::{self, NotificationWidgetState},
    server::start_server,
    state::AppCore,
    widget::{self, WidgetState},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    config: AppConfig,
    bot: BotStatus,
    server: ServerStatus,
    credentials: CredentialStatus,
    channels: Vec<ChannelSummary>,
    history: Vec<crate::model::HistoryEntry>,
    pending_media: Vec<PendingMedia>,
    overlay_url: String,
    audio_url: String,
    ws_url: String,
    invite_url: Option<String>,
    widget: WidgetState,
    notification_widget: NotificationWidgetState,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    bot: BotStatus,
    server: ServerStatus,
    channels: Vec<ChannelSummary>,
    widget: WidgetState,
    notification_widget: NotificationWidgetState,
    pending_media: Vec<PendingMedia>,
    queue: Vec<crate::stage_scheduler::QueueItem>,
    outputs_paused: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelConfig {
    /// The Relay channel. The former message channel is only changed by
    /// `choose_relay_channel`, so panel fields for it are ignored.
    watched_channel_id: String,
    #[serde(default)]
    media_cleanup_enabled: bool,
    #[serde(default)]
    media_welcome_message_id: String,
    #[serde(default)]
    music_channel_id: String,
    #[serde(default)]
    music_cleanup_enabled: bool,
    #[serde(default)]
    music_welcome_message_id: String,
    #[serde(default)]
    honeypot_channel_id: String,
    #[serde(default)]
    honeypot_action: HoneypotAction,
    port: u16,
    display_duration_ms: u64,
    gif_duration_ms: u64,
    sticker_duration_ms: u64,
    notification_duration_ms: u64,
    media_volume: u8,
    notification_character_limit: u32,
    notification_queue_limit: u8,
    notifications_obs_enabled: bool,
    bot_online_status: String,
    bot_activity_type: String,
    bot_activity_text: String,
    show_author: bool,
    show_media_text_obs: bool,
    show_media_text_widget: bool,
    widget_sound_enabled: bool,
    moderation_enabled: bool,
    moderation_allow_images: bool,
    moderation_allow_videos: bool,
    moderation_allow_audio: bool,
    privacy_scan_enabled: bool,
    privacy_similarity_boost: u8,
    privacy_concepts: Vec<crate::privacy::ForbiddenConcept>,
    #[serde(default)]
    privacy_filter_exempt_role_ids: Vec<String>,
    privacy_protection_level: crate::privacy::ProtectionLevel,
    privacy_enabled_categories: Vec<crate::privacy::PrivacyCategory>,
    privacy_block_threshold: crate::privacy::PrivacyClassification,
    privacy_review_intermediate: bool,
    privacy_auto_delete_blocked_messages: bool,
    privacy_allowlist: Vec<String>,
    privacy_custom_patterns: Vec<String>,
    /// Absent from older panels: keep the saved moderation settings then.
    #[serde(default)]
    moderation: Option<crate::moderation::ModerationSettings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSettings {
    channel: bool,
    url: bool,
    show: bool,
    status: bool,
    test: bool,
    regenerate: bool,
    clear: bool,
    nuke: bool,
    lock: bool,
    changelog: bool,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputTarget {
    MediaObs,
    MediaWidget,
    NotificationObs,
    NotificationWidget,
    MusicObs,
    MusicVideoObs,
}

#[tauri::command]
pub async fn get_bootstrap(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
) -> Result<Bootstrap, String> {
    build_bootstrap(&app, &core).await.map_err(display_error)
}

#[tauri::command]
pub async fn get_runtime_status(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
) -> Result<RuntimeStatus, String> {
    Ok(RuntimeStatus {
        bot: core.bot_status.read().await.clone(),
        server: core.server_status.read().await.clone(),
        channels: core.channels.read().await.clone(),
        widget: widget::state(&app, &core).await,
        notification_widget: notification_widget::state(&app, &core).await,
        pending_media: core.pending_media.read().await.iter().cloned().collect(),
        queue: queue_snapshot(&core).await,
        outputs_paused: core.outputs_paused(),
    })
}

/// Returns a newly typed OBS password (and whether it was typed), otherwise the stored one.
fn obs_password(password: Option<String>) -> Result<(Option<String>, bool), String> {
    match password.map(|value| value.trim().to_owned()) {
        Some(password) if !password.is_empty() => Ok((Some(password), true)),
        _ => Ok((
            crate::credentials::load_obs_password().map_err(display_error)?,
            false,
        )),
    }
}

/// Keeps a typed password only once OBS has accepted it.
fn remember_obs_password(password: Option<&str>, typed: bool) -> Result<(), String> {
    match password {
        Some(password) if typed => {
            crate::credentials::save_obs_password(password).map_err(display_error)
        }
        _ => Ok(()),
    }
}

#[tauri::command]
pub async fn obs_list_scenes(
    port: Option<u16>,
    password: Option<String>,
) -> Result<crate::obs::ObsScenes, String> {
    let (password, typed) = obs_password(password)?;
    let scenes = crate::obs::list_scenes(
        port.unwrap_or(crate::obs::DEFAULT_OBS_PORT),
        password.as_deref(),
    )
    .await
    .map_err(|error| format!("{error:#}"))?;
    remember_obs_password(password.as_deref(), typed)?;
    Ok(scenes)
}

#[tauri::command]
pub async fn obs_install_sources(
    core: State<'_, Arc<AppCore>>,
    port: Option<u16>,
    password: Option<String>,
    scene: String,
    include_reactions: bool,
) -> Result<Vec<crate::obs::InstalledSource>, String> {
    let scene = scene.trim();
    if scene.is_empty() || scene.chars().count() > 256 {
        return Err("Choose an OBS scene.".into());
    }
    let (password, typed) = obs_password(password)?;
    let relay_port = core.config.read().await.port;
    let sources = crate::obs::relay_sources(relay_port, include_reactions);
    let installed = crate::obs::install_sources(
        port.unwrap_or(crate::obs::DEFAULT_OBS_PORT),
        password.as_deref(),
        scene,
        &sources,
    )
    .await
    .map_err(|error| format!("{error:#}"))?;
    remember_obs_password(password.as_deref(), typed)?;
    Ok(installed)
}

#[tauri::command]
pub async fn forget_obs_password() -> Result<crate::credentials::CredentialStatus, String> {
    crate::credentials::save_obs_password("").map_err(display_error)?;
    crate::credentials::credential_status().map_err(display_error)
}

#[tauri::command]
pub async fn check_discord_setup(
    core: State<'_, Arc<AppCore>>,
) -> Result<crate::discord_check::DiscordSetupReport, String> {
    crate::discord_check::check(core.inner())
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
pub async fn panic_stop(app: AppHandle, core: State<'_, Arc<AppCore>>) -> Result<(), String> {
    crate::panic::trigger(&app, core.inner()).await;
    Ok(())
}

#[tauri::command]
pub async fn resume_outputs(core: State<'_, Arc<AppCore>>) -> Result<(), String> {
    crate::panic::resume(core.inner()).await;
    Ok(())
}

#[tauri::command]
pub async fn set_panic_shortcut(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
    shortcut: String,
) -> Result<AppConfig, String> {
    let shortcut = shortcut.trim().parse::<Shortcut>().map_err(|_| {
        "Invalid shortcut. Capture a key with at least one supported key combination.".to_string()
    })?;
    let previous = core.config.read().await.clone();
    let previous_shortcut = previous
        .panic_shortcut
        .parse::<Shortcut>()
        .or_else(|_| crate::config::DEFAULT_PANIC_SHORTCUT.parse::<Shortcut>())
        .map_err(|_| "The configured panic shortcut is invalid.".to_string())?;
    if shortcut == previous_shortcut {
        return Ok(previous);
    }
    let skip_shortcut = previous.skip_shortcut.parse::<Shortcut>().ok();
    if skip_shortcut == Some(shortcut) {
        return Err("The selected shortcut is already in use.".into());
    }

    let manager = app.global_shortcut();
    let _ = manager.unregister(previous_shortcut);
    if let Err(error) = register_panic_handler(manager, shortcut, core.inner().clone()) {
        let _ = register_panic_handler(manager, previous_shortcut, core.inner().clone());
        return Err(error);
    }
    match core
        .update_config(|config| config.panic_shortcut = shortcut.to_string())
        .await
    {
        Ok(config) => Ok(config),
        Err(_) => {
            let _ = manager.unregister(shortcut);
            let _ = register_panic_handler(manager, previous_shortcut, core.inner().clone());
            Err("The panic shortcut could not be saved.".into())
        }
    }
}

pub(crate) fn register_panic_handler(
    manager: &tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>,
    shortcut: Shortcut,
    core: Arc<AppCore>,
) -> Result<(), String> {
    manager
        .on_shortcut(shortcut, move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let app = app.clone();
                let core = core.clone();
                tauri::async_runtime::spawn(async move {
                    crate::panic::trigger(&app, &core).await;
                });
            }
        })
        .map_err(|_| "The selected shortcut is already in use.".to_string())
}

async fn queue_snapshot(core: &AppCore) -> Vec<crate::stage_scheduler::QueueItem> {
    let mut items = core.stage_scheduler.queue_snapshot().await;
    items.extend(
        core.music
            .lock()
            .await
            .pending_events()
            .into_iter()
            .map(|music| crate::stage_scheduler::QueueItem {
                id: format!("music:{}", music.playback_id),
                kind: "music".into(),
                title: music.title,
                author: music.requested_by,
                ready: true,
            }),
    );
    items
}

#[tauri::command]
pub async fn remove_queued_media(core: State<'_, Arc<AppCore>>, id: String) -> Result<(), String> {
    if let Some(ticket) = id
        .strip_prefix("stage:")
        .and_then(|value| value.parse::<u64>().ok())
    {
        if core.stage_scheduler.remove_queued(ticket).await {
            return Ok(());
        }
        if let Some(playback_id) = core.stage_scheduler.remove_queued_music(ticket).await {
            core.stop_music_if_current(&playback_id).await;
            return Ok(());
        }
    } else if let Some(playback_id) = id.strip_prefix("music:")
        && core
            .music
            .lock()
            .await
            .pending_events()
            .iter()
            .any(|item| item.playback_id == playback_id)
    {
        core.cancel_pending_music(playback_id).await;
        return Ok(());
    }
    Err("This item is no longer waiting in the queue.".into())
}

#[tauri::command]
pub async fn refresh_channels(
    core: State<'_, Arc<AppCore>>,
) -> Result<Vec<ChannelSummary>, String> {
    refresh_channel_list(&core).await.map_err(display_error)?;
    Ok(core.channels.read().await.clone())
}

#[tauri::command]
pub async fn set_interface_preferences(
    core: State<'_, Arc<AppCore>>,
    preferences: InterfacePreferences,
) -> Result<(), String> {
    crate::config::validate_interface_preferences(&preferences).map_err(display_error)?;
    core.update_config(|config| config.interface_preferences = preferences.clone())
        .await
        .map_err(display_error)?;
    *core.interface_preferences.write().await = preferences.clone();
    let _ = core.relay_tx.send(RelayEvent::Appearance(preferences));
    Ok(())
}

#[tauri::command]
pub async fn set_output_geometry(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
    target: OutputTarget,
    geometry: OutputGeometry,
    width: Option<f64>,
    height: Option<f64>,
    keep_aspect_ratio: Option<bool>,
) -> Result<AppConfig, String> {
    let current = core.config.read().await.clone();
    let media_size = if matches!(target, OutputTarget::MediaWidget)
        && (width.is_some() || height.is_some() || keep_aspect_ratio.is_some())
    {
        let keep_ratio = keep_aspect_ratio.unwrap_or(current.widget_keep_aspect_ratio);
        let (width, height) = widget::clamp_requested_size(
            &app,
            width.unwrap_or(current.widget_width),
            height.unwrap_or(current.widget_height),
            keep_ratio,
        )
        .map_err(display_error)?;
        Some((width, height, keep_ratio))
    } else {
        None
    };
    let notification_size = if matches!(target, OutputTarget::NotificationWidget)
        && (width.is_some() || height.is_some())
    {
        Some(
            notification_widget::clamp_requested_size(
                &app,
                width.unwrap_or(current.notification_widget_width),
                height.unwrap_or(current.notification_widget_height),
            )
            .map_err(display_error)?,
        )
    } else {
        None
    };
    let next = core
        .update_config(|config| {
            match target {
                OutputTarget::MediaObs => config.media_obs_geometry = geometry,
                OutputTarget::MediaWidget => config.media_widget_geometry = geometry,
                OutputTarget::NotificationObs => config.notification_obs_geometry = geometry,
                OutputTarget::NotificationWidget => {
                    config.notification_widget_geometry = geometry;
                }
                OutputTarget::MusicObs => config.music_obs_geometry = geometry,
                OutputTarget::MusicVideoObs => config.music_video_obs_geometry = geometry,
            }
            if let Some((width, height, keep_ratio)) = media_size {
                config.widget_width = width;
                config.widget_height = height;
                config.widget_keep_aspect_ratio = keep_ratio;
            }
            if let Some((width, height)) = notification_size {
                config.notification_widget_width = width;
                config.notification_widget_height = height;
            }
        })
        .await
        .map_err(display_error)?;
    if let Some((width, height, _)) = media_size {
        widget::apply_configured_size(&app, width, height).map_err(display_error)?;
    }
    if let Some((width, height)) = notification_size {
        notification_widget::apply_configured_size(&app, &core, width, height, true)
            .map_err(display_error)?;
    }
    Ok(next)
}

#[tauri::command]
pub async fn save_credentials(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
    client_id: String,
    token: String,
) -> Result<Bootstrap, String> {
    if token.trim().is_empty() {
        let Some((stored, _source)) = load_discord_credentials().map_err(display_error)? else {
            return Err("A Discord bot token is required for the first connection.".into());
        };
        if stored.client_id != client_id.trim() {
            return Err(
                "Enter the stored Discord client ID or provide the bot token again.".into(),
            );
        }
    } else {
        save_discord_credentials(&DiscordCredentials { client_id, token })
            .map_err(display_error)?;
    }
    // The YouTube API key has its own command (store_youtube_api_key).
    start_bot(core.inner().clone())
        .await
        .map_err(display_error)?;
    build_bootstrap(&app, &core).await.map_err(display_error)
}

#[tauri::command]
pub async fn store_youtube_api_key(youtube_api_key: String) -> Result<CredentialStatus, String> {
    save_youtube_api_key(&youtube_api_key).map_err(display_error)?;
    credential_status().map_err(display_error)
}

#[tauri::command]
pub async fn apply_config(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
    mut config: PanelConfig,
) -> Result<Bootstrap, String> {
    let previous = core.config.read().await.clone();
    if previous.relay_channel_conflict() && config.watched_channel_id != previous.watched_channel_id
    {
        return Err("Choose which former channel Relay keeps before changing it.".into());
    }
    config.music_welcome_message_id = crate::music_cleanup::protected_message_id(
        &config.music_welcome_message_id,
        &config.music_channel_id,
    )?;
    config.media_welcome_message_id = crate::channel_cleanup::welcome_message_id(
        &config.media_welcome_message_id,
        &config.watched_channel_id,
    )?;
    if config.media_cleanup_enabled
        && (!previous.media_cleanup_enabled
            || previous.watched_channel_id != config.watched_channel_id
            || previous.media_welcome_message_id != config.media_welcome_message_id)
    {
        crate::channel_cleanup::verify_welcome(
            &core,
            true,
            &config.watched_channel_id,
            &config.media_welcome_message_id,
        )
        .await?;
    }
    if crate::music_cleanup::requires_verification(
        &previous,
        config.music_cleanup_enabled,
        &config.music_channel_id,
        &config.music_welcome_message_id,
    ) {
        let http = core
            .discord_http()
            .await
            .ok_or("Connect the bot before enabling music cleanup.")?;
        let channel = config
            .music_channel_id
            .parse::<u64>()
            .map_err(|_| "Select a music channel.")?;
        if channel == 0 {
            return Err("Select a valid music channel.".into());
        }
        let protected =
            crate::music_cleanup::optional_protected_message(&config.music_welcome_message_id)?;
        crate::music_cleanup::verify_protected_message(
            &http,
            channel,
            protected,
            "The welcome message was not found in the selected music channel.",
        )
        .await?;
    }
    let next = core
        .update_config(|current| {
            current.watched_channel_id = config.watched_channel_id;
            current.media_cleanup_enabled = config.media_cleanup_enabled;
            current.media_welcome_message_id = config.media_welcome_message_id;
            current.unify_relay_channels();
            current.music_channel_id = config.music_channel_id;
            current.music_cleanup_enabled = config.music_cleanup_enabled;
            current.music_welcome_message_id = config.music_welcome_message_id;
            current.honeypot_channel_id = config.honeypot_channel_id;
            current.honeypot_action = config.honeypot_action;
            current.port = config.port;
            current.display_duration_ms = config.display_duration_ms;
            current.gif_duration_ms = config.gif_duration_ms;
            current.sticker_duration_ms = config.sticker_duration_ms;
            current.notification_duration_ms = config.notification_duration_ms;
            current.media_volume = config.media_volume;
            current.notification_character_limit = config.notification_character_limit;
            current.notification_queue_limit = config.notification_queue_limit;
            current.notifications_obs_enabled = config.notifications_obs_enabled;
            current.bot_online_status = config.bot_online_status;
            current.bot_activity_type = config.bot_activity_type;
            current.bot_activity_text = config.bot_activity_text.trim().to_owned();
            current.show_author = config.show_author;
            current.show_media_text_obs = config.show_media_text_obs;
            current.show_media_text_widget = config.show_media_text_widget;
            current.widget_sound_enabled = config.widget_sound_enabled;
            current.moderation_enabled = config.moderation_enabled;
            current.moderation_allow_images = config.moderation_allow_images;
            current.moderation_allow_videos = config.moderation_allow_videos;
            current.moderation_allow_audio = config.moderation_allow_audio;
            current.privacy_scan_enabled = config.privacy_scan_enabled;
            current.privacy_similarity_boost = config.privacy_similarity_boost;
            current.privacy_concepts = config.privacy_concepts;
            current.privacy_filter_exempt_role_ids = config.privacy_filter_exempt_role_ids;
            current.privacy_protection_level = config.privacy_protection_level;
            current.privacy_enabled_categories = config.privacy_enabled_categories;
            current.privacy_block_threshold = config.privacy_block_threshold;
            current.privacy_review_intermediate = config.privacy_review_intermediate;
            current.privacy_auto_delete_blocked_messages =
                config.privacy_auto_delete_blocked_messages;
            current.privacy_allowlist = config.privacy_allowlist;
            current.privacy_custom_patterns = config.privacy_custom_patterns;
            if let Some(moderation) = config.moderation {
                // The live-mode snapshot belongs to Relay, never to the panel.
                let live_restore = current.moderation.live_restore.take();
                current.moderation = moderation;
                current.moderation.live_restore = live_restore;
            }
        })
        .await
        .map_err(display_error)?;
    let port_changed = previous.port != next.port;
    let server_down = !core.server_status.read().await.connected;

    if (port_changed || server_down)
        && let Err(error) = start_server(core.inner().clone()).await
    {
        let _ = core.set_config(previous).await;
        if let Err(rollback_error) = start_server(core.inner().clone()).await {
            core.server_status.write().await.error = Some(rollback_error.to_string());
        }
        return Err(format!("Unable to use the requested local port: {error}"));
    }
    if port_changed || server_down {
        reactions::restore_audio(&app, &core).await?;
    }
    if previous.bot_online_status != next.bot_online_status
        || previous.bot_activity_type != next.bot_activity_type
        || previous.bot_activity_text != next.bot_activity_text
    {
        apply_bot_presence(&core, &next).await;
    }
    widget::refresh(&app, &core).await.map_err(display_error)?;
    notification_widget::refresh(&app, &core)
        .await
        .map_err(display_error)?;
    build_bootstrap(&app, &core).await.map_err(display_error)
}

/// Resolves two different former channels: the chosen one becomes the Relay channel.
#[tauri::command]
pub async fn choose_relay_channel(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
    channel_id: String,
) -> Result<Bootstrap, String> {
    // Refuse an invalid choice before saving anything.
    core.config
        .read()
        .await
        .clone()
        .choose_relay_channel(&channel_id)
        .map_err(display_error)?;
    core.update_config(|current| {
        // Checked above; if the channels changed meanwhile, nothing is chosen.
        let _ = current.choose_relay_channel(&channel_id);
    })
    .await
    .map_err(display_error)?;
    build_bootstrap(&app, &core).await.map_err(display_error)
}

#[tauri::command]
pub async fn set_media_caption_visibility(
    core: State<'_, Arc<AppCore>>,
    show_media_text_obs: bool,
    show_media_text_widget: bool,
) -> Result<AppConfig, String> {
    core.update_config(|config| {
        config.show_media_text_obs = show_media_text_obs;
        config.show_media_text_widget = show_media_text_widget;
    })
    .await
    .map_err(display_error)
}

#[tauri::command]
pub async fn clear_overlay(core: State<'_, Arc<AppCore>>) -> Result<(), String> {
    core.clear_all_music().await;
    core.stage_scheduler.clear().await;
    core.unpin_message().await;
    crate::reactions::stop(&core, None).await;
    let _ = core.relay_tx.send(RelayEvent::Clear);
    Ok(())
}

#[tauri::command]
pub async fn save_command_settings(
    core: State<'_, Arc<AppCore>>,
    settings: CommandSettings,
) -> Result<AppConfig, String> {
    core.update_config(|config| {
        config.command_channel_enabled = settings.channel;
        config.command_url_enabled = settings.url;
        config.command_show_enabled = settings.show;
        config.command_status_enabled = settings.status;
        config.command_test_enabled = settings.test;
        config.command_regenerate_enabled = settings.regenerate;
        config.command_clear_enabled = settings.clear;
        config.command_nuke_enabled = settings.nuke;
        config.command_lock_enabled = settings.lock || config.channel_lock.is_some();
        config.command_changelog_enabled = settings.changelog;
    })
    .await
    .map_err(display_error)
}

#[tauri::command]
pub async fn save_custom_commands(
    core: State<'_, Arc<AppCore>>,
    commands: Vec<CustomCommandDefinition>,
) -> Result<AppConfig, String> {
    let _sync = core.custom_command_sync.lock().await;
    let previous = core.config.read().await.clone();
    let mut candidate = previous.clone();
    candidate.custom_commands = commands.clone();
    candidate.validate().map_err(display_error)?;

    sync_relay_command_schema(&core, &candidate)
        .await
        .map_err(|_| "Unable to synchronize custom commands with Discord.".to_string())?;

    match core
        .update_config(|config| config.custom_commands = commands)
        .await
    {
        Ok(config) => Ok(config),
        Err(_) => {
            if sync_relay_command_schema(&core, &previous).await.is_err() {
                core.bot_status.write().await.error = Some(
                    "Custom command rollback failed. Reconnect the Discord bot before retrying."
                        .into(),
                );
                return Err(
                    "Unable to save custom commands or restore the previous Discord schema.".into(),
                );
            }
            Err("Unable to save custom commands. The previous Discord schema was restored.".into())
        }
    }
}

#[tauri::command]
pub async fn replay_media(core: State<'_, Arc<AppCore>>, message_id: String) -> Result<(), String> {
    if let Some(playback_id) = message_id.strip_prefix("youtube-") {
        return core
            .replay_music_history(playback_id)
            .await
            .map_err(|error| error.to_string());
    }
    if message_id.is_empty()
        || message_id.len() > 64
        || !message_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("Invalid Discord message ID.".into());
    }
    let events = core
        .history
        .read()
        .await
        .iter()
        .filter_map(crate::model::HistoryEntry::media)
        .filter(|event| event.message_id == message_id)
        .cloned()
        .collect::<Vec<_>>();
    if events.is_empty() {
        return Err("The media is no longer in history.".into());
    }
    for event in events.into_iter().rev() {
        core.replay_media_event(event)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn download_history_media(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
    message_id: String,
    media_url: String,
    format: Option<String>,
) -> Result<bool, String> {
    if let Some(playback_id) = message_id.strip_prefix("youtube-") {
        use tauri::Manager;
        let entry = core
            .music_history_entry(playback_id)
            .await
            .ok_or_else(|| "The music is no longer in history.".to_string())?;
        let tool_dir = app
            .path()
            .app_cache_dir()
            .map_err(|_| "The download tool folder is unavailable.".to_string())?;
        return crate::youtube_download::download(
            &entry.music.video_id,
            &entry.music.title,
            format.as_deref().unwrap_or_default(),
            &tool_dir,
        )
        .await;
    }
    validate_message_id(&message_id)?;
    if media_url.is_empty() || media_url.len() > 2_048 {
        return Err("Invalid media URL.".into());
    }
    let event = core
        .history
        .read()
        .await
        .iter()
        .filter_map(crate::model::HistoryEntry::media)
        .find(|event| {
            event.message_id == message_id
                && (event.url == media_url || event.proxy_url == media_url)
        })
        .cloned()
        .ok_or_else(|| "The media is no longer in history.".to_string())?;
    let filename = safe_media_filename(&event.filename, event.kind, &event.content_type);
    let (filter_name, extensions) = media_download_filter(event.kind, &event.content_type);
    let dialog = rfd::AsyncFileDialog::new()
        .add_filter(filter_name, extensions)
        .set_file_name(&filename);
    let Some(file) = dialog.save_file().await else {
        return Ok(false);
    };
    let (bytes, _) = history_media_bytes(&core, &event).await?;
    let path = file.path().to_path_buf();
    tauri::async_runtime::spawn_blocking(move || std::fs::write(path, bytes))
        .await
        .map_err(|_| "The media could not be saved.".to_string())?
        .map_err(|_| "The media could not be saved.".to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn set_skip_shortcut(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
    shortcut: String,
) -> Result<AppConfig, String> {
    let shortcut = shortcut.trim().parse::<Shortcut>().map_err(|_| {
        "Invalid shortcut. Capture a key with at least one supported key combination.".to_string()
    })?;
    let previous = core.config.read().await.clone();
    let previous_shortcut = previous
        .skip_shortcut
        .parse::<Shortcut>()
        .or_else(|_| DEFAULT_SKIP_SHORTCUT.parse::<Shortcut>())
        .map_err(|_| "The configured media skip shortcut is invalid.".to_string())?;
    if shortcut == previous_shortcut {
        return Ok(previous);
    }

    let manager = app.global_shortcut();
    let _ = manager.unregister(previous_shortcut);
    if let Err(error) = register_skip_handler(manager, shortcut, core.inner().clone()) {
        let _ = register_skip_handler(manager, previous_shortcut, core.inner().clone());
        return Err(error);
    }
    let next = match core
        .update_config(|config| config.skip_shortcut = shortcut.to_string())
        .await
    {
        Ok(config) => config,
        Err(_) => {
            let _ = manager.unregister(shortcut);
            let _ = register_skip_handler(manager, previous_shortcut, core.inner().clone());
            return Err("The media skip shortcut could not be saved.".into());
        }
    };
    Ok(next)
}

pub(crate) fn register_skip_handler(
    manager: &tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>,
    shortcut: Shortcut,
    core: Arc<AppCore>,
) -> Result<(), String> {
    manager
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let core = core.clone();
                tauri::async_runtime::spawn(async move {
                    core.skip_playback().await;
                });
            }
        })
        .map_err(|_| "The selected shortcut is already in use.".to_string())
}

fn validate_message_id(message_id: &str) -> Result<(), String> {
    if message_id.is_empty()
        || message_id.len() > 64
        || !message_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("Invalid Discord message ID.".into());
    }
    Ok(())
}

async fn history_media_bytes(
    core: &AppCore,
    event: &MediaEvent,
) -> Result<(Vec<u8>, String), String> {
    if let Some(audio_id) = event.audio_id.as_deref()
        && let Some(audio) = core
            .media_audio
            .read()
            .await
            .iter()
            .find(|audio| audio.id == audio_id)
            .cloned()
    {
        return Ok((audio.bytes.to_vec(), audio.content_type));
    }
    if let Some(cache_id) = event.cached_media_id.as_deref()
        && let Some(media) = core
            .cached_media
            .read()
            .await
            .iter()
            .find(|media| media.id == cache_id)
            .cloned()
    {
        return Ok((media.bytes.to_vec(), media.content_type));
    }

    let maximum_bytes = match event.kind {
        MediaKind::Audio => artwork::MAX_AUDIO_BYTES,
        MediaKind::Video => 50 * 1024 * 1024,
        MediaKind::Image | MediaKind::Gif => artwork::MAX_EMBED_MEDIA_BYTES,
    };
    let bytes = match event.kind {
        MediaKind::Video => artwork::download_video_bounded(&event.url, maximum_bytes).await,
        _ => artwork::download_bounded(&event.url, maximum_bytes).await,
    };
    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(_) if event.proxy_url != event.url => {
            let fallback = match event.kind {
                MediaKind::Video => {
                    artwork::download_video_bounded(&event.proxy_url, maximum_bytes).await
                }
                _ => artwork::download_bounded(&event.proxy_url, maximum_bytes).await,
            };
            fallback.map_err(|_| "The media is no longer available locally.".to_string())?
        }
        Err(_) => return Err("The media is no longer available locally.".into()),
    };
    Ok((bytes, event.content_type.clone()))
}

fn safe_media_filename(filename: &str, kind: MediaKind, content_type: &str) -> String {
    let source = Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let mut safe = source
        .chars()
        .filter(|character| !character.is_control())
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => character,
        })
        .take(120)
        .collect::<String>();
    if safe.is_empty() || safe == "." || safe == ".." {
        safe = format!("relay-media.{}", media_extension(kind, content_type));
    }
    if matches!(kind, MediaKind::Gif) && !is_video_content_type(content_type) {
        let stem = safe
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .filter(|stem| !stem.is_empty())
            .unwrap_or(&safe);
        safe = format!("{stem}.gif");
    }
    safe
}

fn is_video_content_type(content_type: &str) -> bool {
    content_type
        .trim()
        .to_ascii_lowercase()
        .starts_with("video/")
}

fn media_download_filter(
    kind: MediaKind,
    content_type: &str,
) -> (&'static str, &'static [&'static str]) {
    const AUDIO_EXTENSIONS: &[&str] = &[
        "mp3", "flac", "wav", "ogg", "oga", "opus", "m4a", "aac", "webm", "weba",
    ];
    const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "mov", "mkv", "avi"];
    const GIF_VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm"];
    const GIF_EXTENSIONS: &[&str] = &["gif"];
    const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "apng"];

    match kind {
        MediaKind::Audio => ("Audio", AUDIO_EXTENSIONS),
        MediaKind::Video => ("Video", VIDEO_EXTENSIONS),
        MediaKind::Gif if is_video_content_type(content_type) => ("Video", GIF_VIDEO_EXTENSIONS),
        MediaKind::Gif => ("GIF", GIF_EXTENSIONS),
        MediaKind::Image => ("Image", IMAGE_EXTENSIONS),
    }
}

fn media_extension(kind: MediaKind, content_type: &str) -> &'static str {
    match content_type.to_ascii_lowercase().as_str() {
        "audio/mpeg" => "mp3",
        "audio/flac" => "flac",
        "audio/wav" | "audio/x-wav" => "wav",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => match kind {
            MediaKind::Audio => "mp3",
            MediaKind::Video => "mp4",
            MediaKind::Gif => "gif",
            MediaKind::Image => "png",
        },
    }
}

#[tauri::command]
pub async fn skip_media(core: State<'_, Arc<AppCore>>) -> Result<(), String> {
    core.skip_playback().await;
    Ok(())
}

#[tauri::command]
pub async fn test_output(
    core: State<'_, Arc<AppCore>>,
    target: OutputTestTarget,
) -> Result<(), String> {
    emit_output_test(&core, target).await.map_err(display_error)
}

#[tauri::command]
pub async fn preview_output_sample(
    core: State<'_, Arc<AppCore>>,
    sample: String,
) -> Result<(), String> {
    if sample == "notification" {
        return emit_output_test(&core, OutputTestTarget::Notification)
            .await
            .map_err(display_error);
    }
    if sample == "sticker" {
        return emit_output_test(&core, OutputTestTarget::Sticker)
            .await
            .map_err(display_error);
    }
    let kind = match sample.as_str() {
        "portrait" | "landscape" => MediaKind::Image,
        "gif" => MediaKind::Gif,
        "video" => MediaKind::Video,
        "audio" => MediaKind::Audio,
        _ => return Err("Unknown output sample.".into()),
    };
    let mut media = test_media(kind, "Relay sample", None);
    if sample == "audio" {
        let id = "999999999999999997";
        core.cache_audio(id.into(), "audio/wav".into(), test_tone_wav())
            .await;
        core.cache_artwork(
            id.into(),
            artwork::EmbeddedArtwork {
                content_type: "image/png".into(),
                bytes: include_bytes!("../../outputs/samples/landscape.png").to_vec(),
            },
        )
        .await;
        media.audio_id = Some(id.into());
        media.artwork_id = Some(id.into());
        media.title = Some("Relay audio test".into());
        media.artist = Some("Relay".into());
    } else {
        media.url = format!("/output-samples/{sample}");
        media.proxy_url = media.url.clone();
        media.content_type = match sample.as_str() {
            "gif" => "image/gif",
            "video" => "video/webm",
            _ => "image/png",
        }
        .into();
    }
    let _ = core
        .relay_tx
        .send(RelayEvent::TestOutput(Box::new(OutputTestEvent {
            target: if sample == "audio" {
                OutputTestTarget::Audio
            } else {
                OutputTestTarget::Visual
            },
            media: Some(media),
            notification: None,
            sticker: None,
        })));
    Ok(())
}

pub(crate) async fn emit_output_test(
    core: &AppCore,
    target: OutputTestTarget,
) -> anyhow::Result<()> {
    const TEST_AUTHOR: &str = "Relay test";
    const TEST_AVATAR: &str = "/overlay-assets/relay-radar.png";
    const TEST_AUDIO_ID: &str = "999999999999999998";

    let author = AuthorIdentity {
        username: TEST_AUTHOR.into(),
        display_avatar_url: TEST_AVATAR.into(),
    };
    let event = match target {
        OutputTestTarget::Visual => OutputTestEvent {
            target,
            media: Some(test_media(MediaKind::Image, "Relay visual test", None)),
            notification: None,
            sticker: None,
        },
        OutputTestTarget::Audio => {
            core.cache_audio(TEST_AUDIO_ID.into(), "audio/wav".into(), test_tone_wav())
                .await;
            OutputTestEvent {
                target,
                media: Some(test_media(
                    MediaKind::Audio,
                    "Relay audio test",
                    Some(TEST_AUDIO_ID.into()),
                )),
                notification: None,
                sticker: None,
            }
        }
        OutputTestTarget::Notification => OutputTestEvent {
            target: OutputTestTarget::Notification,
            media: None,
            notification: Some(NotificationEvent {
                id: "relay-test-notification".into(),
                text: "Relay notification test".into(),
                author,
                guild_tag: Some(GuildTagIdentity {
                    name: "RE".into(),
                    badge_url: None,
                }),
                timestamp: 0,
                segments: vec![VisualSegment {
                    kind: "text".into(),
                    value: "Relay notification test".into(),
                    url: None,
                    animated: false,
                }],
            }),
            sticker: None,
        },
        OutputTestTarget::Sticker => OutputTestEvent {
            target,
            media: None,
            notification: None,
            sticker: Some(StickerEvent {
                id: "relay-test-sticker".into(),
                name: "Relay sticker test".into(),
                format: "png".into(),
                url: TEST_AVATAR.into(),
                cached_media_id: None,
                author,
                timestamp: 0,
                message_id: "relay-test-sticker".into(),
            }),
        },
    };
    if let Some(notification) = event.notification.as_ref() {
        core.remember_authoritative_notification(notification).await;
    }
    let _ = core.relay_tx.send(RelayEvent::TestOutput(Box::new(event)));
    Ok(())
}

fn test_media(kind: MediaKind, filename: &str, audio_id: Option<String>) -> MediaEvent {
    MediaEvent {
        kind,
        url: "/overlay-assets/relay-radar.png".into(),
        proxy_url: "/overlay-assets/relay-radar.png".into(),
        filename: filename.into(),
        content_type: if matches!(kind, MediaKind::Audio) {
            "audio/wav".into()
        } else {
            "image/png".into()
        },
        artwork_id: None,
        audio_id,
        cached_media_id: None,
        title: None,
        artist: None,
        text: None,
        author: AuthorIdentity {
            username: "Relay test".into(),
            display_avatar_url: "/overlay-assets/relay-radar.png".into(),
        },
        timestamp: 0,
        message_id: format!("relay-test-{}", filename.replace(' ', "-")),
    }
}

fn test_tone_wav() -> Vec<u8> {
    const SAMPLE_RATE: u32 = 16_000;
    const SAMPLE_COUNT: u32 = SAMPLE_RATE * 3 / 5;
    const BYTES_PER_SAMPLE: u16 = 2;
    let data_length = SAMPLE_COUNT * u32::from(BYTES_PER_SAMPLE);
    let mut bytes = Vec::with_capacity(44 + data_length as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_length).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * u32::from(BYTES_PER_SAMPLE)).to_le_bytes());
    bytes.extend_from_slice(&BYTES_PER_SAMPLE.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_length.to_le_bytes());
    for index in 0..SAMPLE_COUNT {
        let elapsed = index as f32 / SAMPLE_RATE as f32;
        let fade_samples = SAMPLE_RATE / 50;
        let attack = (index as f32 / fade_samples as f32).min(1.0);
        let release = ((SAMPLE_COUNT - index) as f32 / fade_samples as f32).min(1.0);
        let envelope = attack.min(release);
        let sample = (f32::sin(std::f32::consts::TAU * 660.0 * elapsed)
            * envelope
            * 0.22
            * f32::from(i16::MAX)) as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

#[tauri::command]
pub async fn control_audio(
    core: State<'_, Arc<AppCore>>,
    action: String,
    current_url: Option<String>,
) -> Result<(), String> {
    let (action, media) = match action.as_str() {
        "pause" => (AudioControlAction::Pause, None),
        "resume" => (AudioControlAction::Resume, None),
        "skip" => (AudioControlAction::Skip, None),
        "previous" => {
            let history = core.history.read().await;
            let audio = history
                .iter()
                .filter_map(crate::model::HistoryEntry::media)
                .filter(|event| matches!(event.kind, crate::model::MediaKind::Audio))
                .collect::<Vec<_>>();
            let current_index = current_url
                .as_deref()
                .and_then(|url| audio.iter().position(|event| event.url == url));
            let previous = current_index
                .and_then(|index| audio.get(index + 1))
                .or_else(|| current_index.is_none().then(|| audio.first()).flatten())
                .ok_or_else(|| "No previous audio is available.".to_string())?;
            (AudioControlAction::Previous, Some((*previous).clone()))
        }
        _ => return Err("Invalid audio control action.".into()),
    };
    let _ = core
        .relay_tx
        .send(RelayEvent::AudioControl(AudioControlEvent {
            action,
            media,
        }));
    Ok(())
}

#[tauri::command]
pub async fn get_media_artwork(
    core: State<'_, Arc<AppCore>>,
    artwork_id: String,
) -> Result<tauri::ipc::Response, String> {
    if artwork_id.is_empty()
        || artwork_id.len() > 64
        || !artwork_id
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Err("Invalid media artwork ID.".into());
    }
    let cache = core.media_artwork.read().await;
    let artwork = cache
        .iter()
        .find(|artwork| artwork.id == artwork_id)
        .ok_or_else(|| "The media artwork is no longer available.".to_string())?;
    Ok(tauri::ipc::Response::new(artwork.bytes.to_vec()))
}

#[tauri::command]
pub async fn approve_pending_media(core: State<'_, Arc<AppCore>>, id: u64) -> Result<(), String> {
    core.approve_media(id)
        .await
        .then_some(())
        .ok_or_else(|| "The media is no longer pending.".into())
}

#[tauri::command]
pub async fn reject_pending_media(core: State<'_, Arc<AppCore>>, id: u64) -> Result<(), String> {
    core.reject_media(id)
        .await
        .then_some(())
        .ok_or_else(|| "The media is no longer pending.".into())
}

#[tauri::command]
pub async fn clear_pending_media(core: State<'_, Arc<AppCore>>) -> Result<(), String> {
    core.clear_pending_media().await;
    Ok(())
}

#[tauri::command]
pub async fn regenerate_secret(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
) -> Result<Bootstrap, String> {
    start_server(core.inner().clone())
        .await
        .map_err(display_error)?;
    reactions::restore_audio(&app, &core).await?;
    widget::refresh(&app, &core).await.map_err(display_error)?;
    notification_widget::refresh(&app, &core)
        .await
        .map_err(display_error)?;
    build_bootstrap(&app, &core).await.map_err(display_error)
}

#[tauri::command]
pub async fn toggle_widget(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
) -> Result<WidgetState, String> {
    widget::toggle(&app, core.inner().clone())
        .await
        .map_err(display_error)
}

#[tauri::command]
pub async fn set_widget_locked(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
    locked: bool,
) -> Result<WidgetState, String> {
    widget::set_locked(&app, core.inner().clone(), locked)
        .await
        .map_err(display_error)
}

#[tauri::command]
pub async fn set_notification_widget_visible(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
    visible: bool,
) -> Result<NotificationWidgetState, String> {
    notification_widget::set_visible(&app, core.inner().clone(), visible)
        .await
        .map_err(display_error)
}

pub const NOTIFICATION_SOUND_MAX_SECONDS: u64 = 10;
pub const NOTIFICATION_SOUND_MAX_BYTES: u64 = 15 * 1024 * 1024;

#[tauri::command]
pub async fn set_notification_sound_enabled(
    core: State<'_, Arc<AppCore>>,
    enabled: bool,
) -> Result<AppConfig, String> {
    core.update_config(|config| config.notification_sound_enabled = enabled)
        .await
        .map_err(display_error)
}

#[tauri::command]
pub async fn set_notification_sound_obs_enabled(
    core: State<'_, Arc<AppCore>>,
    enabled: bool,
) -> Result<AppConfig, String> {
    core.update_config(|config| config.notification_sound_obs_enabled = enabled)
        .await
        .map_err(display_error)
}

#[tauri::command]
pub async fn pick_notification_sound(
    core: State<'_, Arc<AppCore>>,
) -> Result<Option<AppConfig>, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter(
            "Audio",
            &[
                "mp3", "flac", "wav", "ogg", "oga", "opus", "m4a", "aac", "webm", "weba",
            ],
        )
        .pick_file()
        .await;
    let Some(file) = file else {
        return Ok(None);
    };
    let path = file.path().to_path_buf();
    validate_notification_sound(path.clone())
        .await
        .map_err(display_error)?;
    let path_string = path.to_string_lossy().into_owned();
    let config = core
        .update_config(|config| config.notification_sound_path = Some(path_string))
        .await
        .map_err(display_error)?;
    Ok(Some(config))
}

#[tauri::command]
pub async fn clear_notification_sound(core: State<'_, Arc<AppCore>>) -> Result<AppConfig, String> {
    core.update_config(|config| config.notification_sound_path = None)
        .await
        .map_err(display_error)
}

async fn validate_notification_sound(path: std::path::PathBuf) -> anyhow::Result<()> {
    use lofty::prelude::AudioFile;

    let metadata = std::fs::metadata(&path)?;
    if metadata.len() > NOTIFICATION_SOUND_MAX_BYTES {
        anyhow::bail!("The notification sound must stay under 15 MB.");
    }
    let duration = tokio::task::spawn_blocking(move || -> anyhow::Result<std::time::Duration> {
        let tagged = lofty::probe::Probe::open(&path)?.read()?;
        Ok(tagged.properties().duration())
    })
    .await??;
    if duration > std::time::Duration::from_secs(NOTIFICATION_SOUND_MAX_SECONDS) {
        anyhow::bail!(
            "The notification sound must last {NOTIFICATION_SOUND_MAX_SECONDS} seconds or less."
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn set_notification_widget_locked(
    app: AppHandle,
    core: State<'_, Arc<AppCore>>,
    locked: bool,
) -> Result<NotificationWidgetState, String> {
    notification_widget::set_locked(&app, core.inner().clone(), locked)
        .await
        .map_err(display_error)
}

async fn build_bootstrap(app: &AppHandle, core: &Arc<AppCore>) -> anyhow::Result<Bootstrap> {
    let config = core.config.read().await.clone();
    let credentials = credential_status()?;
    let invite_url = credentials
        .client_id
        .as_deref()
        .map(|client_id| invite_url(client_id, &config));
    Ok(Bootstrap {
        overlay_url: widget::obs_visual_url(config.port),
        audio_url: widget::obs_audio_url(config.port),
        ws_url: format!(
            "ws://127.0.0.1:{}/ws?role=panel&token={}",
            config.port, core.panel_token
        ),
        invite_url,
        widget: widget::state(app, core).await,
        notification_widget: notification_widget::state(app, core).await,
        config,
        bot: core.bot_status.read().await.clone(),
        server: core.server_status.read().await.clone(),
        credentials,
        channels: core.channels.read().await.clone(),
        history: core.history.read().await.iter().cloned().collect(),
        pending_media: core.pending_media.read().await.iter().cloned().collect(),
    })
}

/// Legacy dedicated YouTube URL (still served). Kept for tests and migration docs;
/// the panel recommends [`widget::obs_visual_url`] instead.
#[cfg(test)]
fn youtube_overlay_url(port: u16) -> String {
    format!("http://{}:{port}/youtube", widget::youtube_embed_host())
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests;
