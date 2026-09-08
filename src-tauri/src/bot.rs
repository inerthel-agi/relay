use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, bail};
use futures_util::StreamExt;
use serenity::{
    all::{
        ActionRowComponent, ButtonStyle, Channel, ChannelId, ChannelType, Colour, Command,
        CommandDataOptionValue, CommandInteraction, CommandOptionType, ComponentInteraction,
        ComponentInteractionDataKind, Context, CreateActionRow, CreateAllowedMentions,
        CreateButton, CreateChannel, CreateCommand, CreateCommandOption, CreateEmbed,
        CreateInputText, CreateInteractionResponse, CreateInteractionResponseMessage,
        CreateMessage, CreateModal, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
        EditInteractionResponse, EditMessage, EventHandler, GatewayIntents, GetMessages, GuildId,
        InputTextStyle, Interaction, Message, MessageId, MessageUpdateEvent, ModalInteraction,
        OnlineStatus, PermissionOverwrite, PermissionOverwriteType, Permissions, Ready,
        StickerFormatType, User, UserId,
    },
    async_trait,
    cache::Cache,
    client::Client,
    gateway::ActivityData,
    http::Http,
};

use crate::{
    artwork,
    commands::emit_output_test,
    config::{AppConfig, ChannelLockSnapshot, HoneypotAction, PermissionOverwriteSnapshot},
    credentials::{load_discord_credentials, load_youtube_api_key},
    custom_commands,
    model::{
        AuthorIdentity, BotStatus, ChannelSummary, GuildTagIdentity, MediaEvent, MediaKind,
        MusicPlaybackEvent, MusicPlaybackMode, OutputConnectionStatus, OutputTestTarget,
        ServerStatus, StickerEvent, VisualSegment,
    },
    music::{
        MusicSelection, MusicStartResult, SearchSelection, SelectionTake, cooldown_wait_seconds,
        parse_timestamp,
    },
    music_i18n::{self, MusicStrings},
    privacy,
    stage_scheduler::{StageLane, StageTicket},
    state::{AppCore, BotRuntime},
    youtube,
};

const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "bmp"];
const VIDEO_EXTENSIONS: [&str; 6] = ["mp4", "webm", "mov", "m4v", "ogv", "avi"];
const AUDIO_EXTENSIONS: [&str; 7] = ["mp3", "ogg", "wav", "m4a", "flac", "aac", "opus"];
const MEDIA_TEXT_LIMIT: usize = 180;
const MUSIC_COMPONENT_PREFIX: &str = "relay:music:";
const MUSIC_SEARCH_PREFIX: &str = "relay:music:search:";
const MUSIC_MODE_PREFIX: &str = "relay:music:mode:";
const MUSIC_CUSTOM_PREFIX: &str = "relay:music:custom:";
const MUSIC_SKIP_PREFIX: &str = "relay:music:skip:";
const MUSIC_LOOP_PREFIX: &str = "relay:music:loop:";
const MUSIC_CUSTOM_START_ID: &str = "start";
const MUSIC_CUSTOM_END_ID: &str = "end";

struct Handler {
    core: Arc<AppCore>,
}

const HONEYPOT_AUDIT_REASON: &str =
    "Relay compromised-account trap: message posted in the protected channel.";

fn honeypot_action_for_channel(config: &AppConfig, channel_id: &str) -> Option<HoneypotAction> {
    (!config.honeypot_channel_id.is_empty() && config.honeypot_channel_id == channel_id)
        .then_some(config.honeypot_action)
}

fn honeypot_notice(action: HoneypotAction) -> &'static str {
    match action {
        HoneypotAction::Kick => {
            "A message from your Discord account was posted in a protected honeypot channel used to identify compromised accounts. Your account may have been compromised, including by token-grabbing malware or a malicious application. As a precaution, you will be kicked from the server. Change your Discord password, enable two-factor authentication, and review Authorized Apps before rejoining."
        }
        HoneypotAction::Ban => {
            "A message from your Discord account was posted in a protected honeypot channel used to identify compromised accounts. Your account may have been compromised, including by token-grabbing malware or a malicious application. As a precaution, you will be banned from the server. Change your Discord password, enable two-factor authentication, and review Authorized Apps before contacting the server moderators."
        }
    }
}

async fn enforce_honeypot_action(
    core: &AppCore,
    context: &Context,
    message: &Message,
    action: HoneypotAction,
) {
    let Some(guild_id) = message.guild_id else {
        core.bot_status.write().await.error =
            Some("Compromised account trap ignored a non-server message.".into());
        return;
    };

    let dm_delivered = message
        .author
        .direct_message(
            &context.http,
            CreateMessage::new().content(honeypot_notice(action)),
        )
        .await
        .is_ok();
    let action_result = match action {
        HoneypotAction::Kick => {
            guild_id
                .kick_with_reason(&context.http, message.author.id, HONEYPOT_AUDIT_REASON)
                .await
        }
        HoneypotAction::Ban => {
            guild_id
                .ban_with_reason(&context.http, message.author.id, 0, HONEYPOT_AUDIT_REASON)
                .await
        }
    };

    let deletion_failed = message.delete(&context.http).await.is_err();
    core.bot_status.write().await.error =
        honeypot_outcome_error(dm_delivered, action_result.is_err(), deletion_failed);
}

fn honeypot_outcome_error(
    dm_delivered: bool,
    action_failed: bool,
    deletion_failed: bool,
) -> Option<String> {
    let mut errors = Vec::new();
    if !dm_delivered {
        errors.push("Discord DM could not be delivered.");
    }
    if action_failed {
        errors.push("Kick or ban failed; check the bot's permissions and role position.");
    }
    if deletion_failed {
        errors.push("Message deletion failed; check the bot's Manage Messages permission.");
    }
    (!errors.is_empty()).then(|| format!("Compromised account trap: {}", errors.join(" ")))
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, context: Context, ready: Ready) {
        let config = self.core.config.read().await.clone();
        let (activity, status) = presence_from_config(&config);
        context.set_presence(activity, status);
        let avatar = ready
            .user
            .avatar_url()
            .unwrap_or_else(|| ready.user.default_avatar_url());
        *self.core.bot_status.write().await = BotStatus {
            connected: true,
            username: Some(ready.user.name.clone()),
            display_avatar_url: Some(avatar),
            error: None,
        };

        let guild_ids = ready.guilds.iter().map(|guild| guild.id).collect();
        let channels =
            discover_channels(&context.http, &context.cache, ready.user.id, guild_ids).await;
        *self.core.channels.write().await = channels;
        warn_if_watched_channel_missing(&self.core).await;

        if let Err(error) =
            Command::set_global_commands(&context.http, vec![relay_command(&config)]).await
        {
            // Non-fatal: the gateway stays connected, so keep the online status.
            self.core.bot_status.write().await.error =
                Some(format!("Command registration failed: {error}"));
        }
    }

    async fn message(&self, context: Context, message: Message) {
        if message.author.bot {
            return;
        }
        let config = self.core.config.read().await.clone();
        let channel_id = message.channel_id.to_string();

        if let Some(action) = honeypot_action_for_channel(&config, &channel_id) {
            enforce_honeypot_action(&self.core, &context, &message, action).await;
            return;
        }

        let role_ids = message_role_ids(&message);
        let scoped_config = privacy::scoped_config_for_roles(&config, &role_ids);

        if !config.music_channel_id.is_empty() && channel_id == config.music_channel_id {
            if message.id.to_string() == config.music_welcome_message_id {
                return;
            }
            handle_music_message(&self.core, &context.http, &message, &scoped_config).await;
            crate::music_cleanup::delete(
                &self.core,
                &context.http,
                message.channel_id.get(),
                message.id.get(),
            )
            .await;
            return;
        }

        if !config.tts_channel_id.is_empty() && channel_id == config.tts_channel_id {
            let text_report = classify_message_privacy(&message, &scoped_config);
            if block_and_delete_message_if_needed(
                &self.core,
                &context.http,
                &message,
                &text_report,
                &scoped_config,
            )
            .await
            {
                return;
            }
            if privacy::privacy_rules_enabled(&scoped_config) {
                let action = privacy::action_for(&text_report, &scoped_config);
                if matches!(action, privacy::PrivacyAction::Review) {
                    privacy::log_decision(&text_report, action);
                    return;
                }
            }
            let stage_ticket =
                if !message.content.trim().is_empty() || !message.sticker_items.is_empty() {
                    Some(
                        self.core
                            .register_stage_output(
                                message_timestamp(&message),
                                &message.id.to_string(),
                                0,
                                StageLane::Tts,
                            )
                            .await,
                    )
                } else {
                    None
                };
            let author = message_author(&message);
            let guild_tag = guild_tag_from_user(&message.author);
            let mut sticker_segments = Vec::new();
            for sticker in message.sticker_items.iter().take(3) {
                let Some(url) = sticker.image_url() else {
                    continue;
                };
                let (format, _) = sticker_format(sticker.format_type);
                let sticker_text = format!("{}\n{}", message.content, sticker.name);
                let (mut report, _) =
                    inspect_sticker(&url, format, Some(&sticker_text), &scoped_config).await;
                let current_config = self.core.config.read().await.clone();
                let current_scoped_config =
                    privacy::scoped_config_for_roles(&current_config, &role_ids);
                report = reclassify_privacy_report(report, &sticker_text, &current_scoped_config);
                if block_and_delete_message_if_needed(
                    &self.core,
                    &context.http,
                    &message,
                    &report,
                    &current_scoped_config,
                )
                .await
                {
                    if let Some(ticket) = stage_ticket {
                        self.core.cancel_stage_output(ticket).await;
                    }
                    return;
                }
                if privacy::privacy_rules_enabled(&current_scoped_config) {
                    let action = privacy::action_for(&report, &current_scoped_config);
                    if matches!(action, privacy::PrivacyAction::Review) {
                        privacy::log_decision(&report, action);
                        if let Some(ticket) = stage_ticket {
                            self.core.cancel_stage_output(ticket).await;
                        }
                        return;
                    }
                }
                sticker_segments.push(sticker_visual_segment(
                    sticker.name.clone(),
                    Some(url),
                    sticker.format_type,
                ));
            }
            if !sticker_segments.is_empty() {
                let mut segments = match message.content.trim() {
                    "" => Vec::new(),
                    content => parse_visual_segments(content)
                        .unwrap_or_else(|| plain_text_segments(content.into())),
                };
                segments.append(&mut sticker_segments);
                let Some(ticket) = stage_ticket else {
                    return;
                };
                self.core
                    .publish_visual_tts_if_allowed_with_ticket_and_roles(
                        ticket,
                        message.id.to_string(),
                        message.content.clone(),
                        author,
                        guild_tag,
                        message_timestamp(&message),
                        segments,
                        &role_ids,
                    )
                    .await;
                return;
            }
            if let Some(segments) = parse_visual_segments(&message.content) {
                let Some(ticket) = stage_ticket else {
                    return;
                };
                self.core
                    .publish_visual_tts_if_allowed_with_ticket_and_roles(
                        ticket,
                        message.id.to_string(),
                        message.content.clone(),
                        author,
                        guild_tag,
                        message_timestamp(&message),
                        segments,
                        &role_ids,
                    )
                    .await;
                return;
            }
            if let Some(text) = prepare_tts_text(&message.content, config.tts_character_limit) {
                let Some(ticket) = stage_ticket else {
                    return;
                };
                self.core
                    .publish_visual_tts_if_allowed_with_ticket_and_roles(
                        ticket,
                        message.id.to_string(),
                        text.clone(),
                        author,
                        guild_tag,
                        message_timestamp(&message),
                        plain_text_segments(text),
                        &role_ids,
                    )
                    .await;
            } else if let Some(ticket) = stage_ticket {
                self.core.cancel_stage_output(ticket).await;
            }
            return;
        }

        if config.watched_channel_id.is_empty() || channel_id != config.watched_channel_id {
            return;
        }
        let message_report = classify_message_privacy(&message, &scoped_config);
        if block_and_delete_message_if_needed(
            &self.core,
            &context.http,
            &message,
            &message_report,
            &scoped_config,
        )
        .await
        {
            return;
        }
        let media_text = prepare_media_text(&message.content);
        let timestamp = message_timestamp(&message);
        let message_id = message.id.to_string();
        let mut stage_tickets = Vec::new();
        let mut sticker_tickets = Vec::new();
        for (index, sticker) in message.sticker_items.iter().take(3).enumerate() {
            if sticker.image_url().is_none() {
                continue;
            }
            let ticket = self
                .core
                .register_stage_output(timestamp, &message_id, 100 + index as u16, StageLane::Media)
                .await;
            stage_tickets.push(ticket);
            sticker_tickets.push((index, ticket));
        }
        let mut attachment_tickets = Vec::new();
        for (index, _) in message
            .attachments
            .iter()
            .filter_map(|item| classify_attachment(item).map(|kind| (item, kind)))
            .take(3)
            .enumerate()
        {
            let ticket = self
                .core
                .register_stage_output(timestamp, &message_id, 200 + index as u16, StageLane::Media)
                .await;
            stage_tickets.push(ticket);
            attachment_tickets.push(ticket);
        }

        for (index, sticker) in message.sticker_items.iter().take(3).enumerate() {
            let Some(url) = sticker.image_url() else {
                continue;
            };
            let (format, _) = sticker_format(sticker.format_type);
            let stage_ticket = sticker_tickets
                .iter()
                .find_map(|(ticket_index, ticket)| (*ticket_index == index).then_some(*ticket))
                .expect("recognized sticker has a stage ticket");
            let sticker_text = format!("{}\n{}", message.content, sticker.name);
            let (privacy_report, bytes) =
                inspect_sticker(&url, format, Some(&sticker_text), &scoped_config).await;
            let current_config = self.core.config.read().await.clone();
            let current_scoped_config =
                privacy::scoped_config_for_roles(&current_config, &role_ids);
            let privacy_report =
                reclassify_privacy_report(privacy_report, &sticker_text, &current_scoped_config);
            if block_and_delete_message_if_needed(
                &self.core,
                &context.http,
                &message,
                &privacy_report,
                &current_scoped_config,
            )
            .await
            {
                cancel_stage_tickets(&self.core, &stage_tickets).await;
                return;
            }
            self.core
                .submit_sticker_with_ticket_and_roles(
                    stage_ticket,
                    StickerEvent {
                        id: sticker.id.to_string(),
                        name: sticker.name.clone(),
                        format: format.into(),
                        url,
                        cached_media_id: None,
                        author: message_author(&message),
                        timestamp: message_timestamp(&message),
                        message_id: message.id.to_string(),
                    },
                    Some(&sticker_text),
                    bytes,
                    Some(privacy_report),
                    &role_ids,
                )
                .await;
        }

        for attachment in message
            .attachments
            .iter()
            .filter_map(|item| classify_attachment(item).map(|kind| (item, kind)))
            .take(3)
            .enumerate()
        {
            let (index, (attachment, kind)) = attachment;
            let stage_ticket = attachment_tickets[index];
            let mut audio_metadata = if matches!(kind, MediaKind::Audio) {
                artwork::extract(&attachment.url).await.ok()
            } else {
                None
            };
            let mut event = MediaEvent {
                kind,
                url: attachment.url.clone(),
                proxy_url: attachment.proxy_url.clone(),
                filename: attachment.filename.clone(),
                content_type: attachment.content_type.clone().unwrap_or_default(),
                artwork_id: None,
                audio_id: None,
                cached_media_id: None,
                title: audio_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.title.clone()),
                artist: audio_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.artist.clone()),
                text: media_text.clone(),
                author: AuthorIdentity {
                    username: message.author.name.clone(),
                    display_avatar_url: message
                        .author
                        .avatar_url()
                        .unwrap_or_else(|| message.author.default_avatar_url()),
                },
                timestamp: message.timestamp.unix_timestamp().max(0) as u64 * 1_000,
                message_id: message.id.to_string(),
            };
            let attachment_text = format!(
                "{}\n{}\n{}",
                attachment_privacy_text(&message.content, &event.filename),
                event.title.as_deref().unwrap_or_default(),
                event.artist.as_deref().unwrap_or_default()
            );
            let mut privacy_report = if matches!(kind, MediaKind::Image | MediaKind::Gif) {
                if attachment.size as usize > privacy::MAX_IMAGE_BYTES {
                    privacy::image_limit_report(Some(&attachment_text), &scoped_config)
                } else {
                    privacy::analyze_remote_image(
                        &event.url,
                        &event.proxy_url,
                        Some(&attachment_text),
                        &scoped_config,
                    )
                    .await
                }
            } else {
                privacy::classify_text(Some(&attachment_text), &scoped_config)
            };
            if let Some(embedded) = audio_metadata
                .as_ref()
                .and_then(|metadata| metadata.artwork.as_ref())
            {
                privacy_report.merge(
                    privacy::analyze_image_bytes_async(
                        &embedded.bytes,
                        Some(&attachment_text),
                        &scoped_config,
                    )
                    .await,
                );
            }
            let current_config = self.core.config.read().await.clone();
            let current_scoped_config =
                privacy::scoped_config_for_roles(&current_config, &role_ids);
            let privacy_report =
                reclassify_privacy_report(privacy_report, &attachment_text, &current_scoped_config);
            if block_and_delete_message_if_needed(
                &self.core,
                &context.http,
                &message,
                &privacy_report,
                &current_scoped_config,
            )
            .await
            {
                cancel_stage_tickets(&self.core, &stage_tickets).await;
                return;
            }
            if let Some(embedded) = audio_metadata
                .as_mut()
                .and_then(|metadata| metadata.artwork.take())
            {
                let id = attachment.id.to_string();
                self.core.cache_artwork(id.clone(), embedded).await;
                event.artwork_id = Some(id);
            }
            if let Some(metadata) = audio_metadata.as_mut() {
                let id = attachment.id.to_string();
                let content_type = attachment
                    .content_type
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".into());
                self.core
                    .cache_audio(
                        id.clone(),
                        content_type,
                        std::mem::take(&mut metadata.audio),
                    )
                    .await;
                event.audio_id = Some(id);
            }
            self.core
                .submit_analyzed_media_with_ticket_and_roles(
                    stage_ticket,
                    event,
                    Some(privacy_report),
                    Some(&message.content),
                    &role_ids,
                )
                .await;
        }

        submit_embedded_gifs(&self.core, &context.http, &message).await;
    }

    async fn message_update(
        &self,
        context: Context,
        _old: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        if let Some(message) = new {
            submit_embedded_gifs(&self.core, &context.http, &message).await;
            return;
        }
        let Some(embeds) = event.embeds else {
            return;
        };
        let watched_channel_id = self.core.config.read().await.watched_channel_id.clone();
        if watched_channel_id.is_empty() || event.channel_id.to_string() != watched_channel_id {
            return;
        }
        if let Ok(message) = context.http.get_message(event.channel_id, event.id).await {
            submit_embedded_gifs(&self.core, &context.http, &message).await;
            return;
        }
        if let Some(author) = event.author {
            let message = DeferredEmbedMessage {
                channel_id: event.channel_id.to_string(),
                message_id: event.id.to_string(),
                author,
                timestamp: event
                    .timestamp
                    .map(|timestamp| timestamp.unix_timestamp().max(0) as u64 * 1_000)
                    .unwrap_or_else(current_timestamp_ms),
                content: event.content.unwrap_or_default(),
                embeds,
                role_ids: Vec::new(),
            };
            submit_deferred_embeds(&self.core, &context.http, message).await;
            return;
        }
    }

    async fn interaction_create(&self, context: Context, interaction: Interaction) {
        let command = match interaction {
            Interaction::Component(component) => {
                if component.data.custom_id.starts_with(MUSIC_COMPONENT_PREFIX) {
                    handle_music_component(&self.core, &context, &component).await;
                } else {
                    custom_commands::handle_custom_component(&self.core, &context, &component)
                        .await;
                }
                return;
            }
            Interaction::Modal(modal) => {
                if modal.data.custom_id.starts_with(MUSIC_CUSTOM_PREFIX) {
                    handle_music_custom_modal(&self.core, &context, &modal).await;
                }
                return;
            }
            Interaction::Command(command) => command,
            _ => return,
        };
        if command.data.name != "relay" {
            return;
        }

        if let Some(response) =
            custom_commands::handle_custom_command(&self.core, &context, &command).await
        {
            if command
                .create_response(
                    &context.http,
                    CreateInteractionResponse::Message(response.into_message()),
                )
                .await
                .is_err()
            {
                self.core.bot_status.write().await.error =
                    Some("Discord did not accept a custom command response.".into());
            }
            return;
        }

        // Changelog fetches GitHub and posts embeds — defer so Discord does not time out.
        let defer_changelog = command
            .data
            .options
            .first()
            .is_some_and(|option| option.name == "changelog");
        if defer_changelog {
            let defer = CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            );
            if let Err(error) = command.create_response(&context.http, defer).await {
                self.core.bot_status.write().await.error =
                    Some(format!("Discord response failed: {error}"));
                return;
            }
            let content = handle_relay(&self.core, &context.http, &command)
                .await
                .unwrap_or_else(|error| format!("Unable to post the changelog: {error:#}"));
            let edit = EditInteractionResponse::new()
                .content(content)
                .allowed_mentions(CreateAllowedMentions::new());
            if let Err(error) = command.edit_response(&context.http, edit).await {
                self.core.bot_status.write().await.error =
                    Some(format!("Discord response failed: {error}"));
            }
            return;
        }

        let content = handle_relay(&self.core, &context.http, &command)
            .await
            .unwrap_or_else(|error| format!("Unable to update Relay: {error:#}"));
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(content)
                .ephemeral(true)
                .allowed_mentions(CreateAllowedMentions::new()),
        );
        if let Err(error) = command.create_response(&context.http, response).await {
            // Non-fatal: the gateway stays connected, so keep the online status.
            self.core.bot_status.write().await.error =
                Some(format!("Discord response failed: {error}"));
        }
    }
}

mod music_handlers;
mod reaction_access;
use music_handlers::*;
pub(crate) use music_handlers::{refresh_music_card, refresh_pending_music_cards};
pub(crate) use reaction_access::{ReactionAccessOptions, get_reaction_access_options};

fn bounded_privacy_text(value: &str) -> String {
    let value = value.trim();
    value
        .char_indices()
        .nth(privacy::PRIVACY_TEXT_LIMIT)
        .map_or_else(|| value.to_owned(), |(index, _)| value[..index].to_owned())
}

fn message_role_ids(message: &Message) -> Vec<String> {
    message
        .member
        .as_ref()
        .map(|member| member.roles.iter().map(ToString::to_string).collect())
        .unwrap_or_default()
}

fn classify_message_privacy(message: &Message, config: &AppConfig) -> privacy::PrivacyReport {
    classify_privacy_values(
        &message.content,
        message
            .sticker_items
            .iter()
            .take(3)
            .map(|sticker| sticker.name.as_str()),
        message
            .attachments
            .iter()
            .take(3)
            .map(|attachment| attachment.filename.as_str()),
        config,
    )
}

fn classify_privacy_values<'a>(
    content: &str,
    sticker_names: impl IntoIterator<Item = &'a str>,
    attachment_names: impl IntoIterator<Item = &'a str>,
    config: &AppConfig,
) -> privacy::PrivacyReport {
    if !privacy::privacy_rules_enabled(config) {
        return privacy::PrivacyReport::safe();
    }

    let mut report = privacy::PrivacyReport::safe();
    let mut classify = |value: &str| {
        let value = bounded_privacy_text(value);
        if !value.is_empty() {
            report.merge(privacy::classify_text(Some(&value), config));
        }
    };
    classify(content);
    for sticker_name in sticker_names.into_iter().take(3) {
        classify(sticker_name);
    }
    for attachment_name in attachment_names.into_iter().take(3) {
        classify(attachment_name);
        if let Some((stem, _extension)) = attachment_name.rsplit_once('.') {
            classify(stem);
        }
    }
    report.apply_score_policy(config);
    report
}

fn attachment_privacy_text(content: &str, filename: &str) -> String {
    let content = bounded_privacy_text(content);
    let filename = bounded_privacy_text(filename);
    if content.is_empty() {
        filename
    } else if filename.is_empty() {
        content
    } else {
        format!("{content}\n{filename}")
    }
}

fn reclassify_privacy_report(
    report: privacy::PrivacyReport,
    text: &str,
    config: &AppConfig,
) -> privacy::PrivacyReport {
    let signature = privacy::config_signature(config);
    if report
        .config_signature
        .is_some_and(|report_signature| report_signature != signature)
    {
        let mut current = privacy::classify_text(Some(text), config);
        if current.classification == privacy::PrivacyClassification::Safe {
            current.merge(privacy::PrivacyReport::suspicious("scan_config_changed"));
        }
        current.apply_score_policy(config);
        return current;
    }

    let mut current = report;
    current.merge(privacy::classify_text(Some(text), config));
    current.apply_score_policy(config);
    current
}

fn privacy_action_is_blocked(report: &privacy::PrivacyReport, config: &AppConfig) -> bool {
    if !privacy::privacy_rules_enabled(config) {
        return false;
    }
    let action = privacy::action_for(report, config);
    if matches!(action, privacy::PrivacyAction::Block) {
        privacy::log_decision(report, action);
        true
    } else {
        false
    }
}

fn should_auto_delete_blocked_message(report: &privacy::PrivacyReport, config: &AppConfig) -> bool {
    config.privacy_auto_delete_blocked_messages
        && matches!(
            privacy::action_for(report, config),
            privacy::PrivacyAction::Block
        )
        && report
            .categories
            .iter()
            .any(|category| !matches!(category, privacy::PrivacyCategory::MediaSafety))
}

async fn block_and_delete_message_if_needed(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    report: &privacy::PrivacyReport,
    config: &AppConfig,
) -> bool {
    if !privacy_action_is_blocked(report, config) {
        return false;
    }
    if should_auto_delete_blocked_message(report, config)
        && message
            .channel_id
            .delete_message(http, message.id)
            .await
            .is_err()
    {
        core.bot_status.write().await.error =
            Some("Privacy deletion failed. Verify Manage Messages in this channel.".into());
    }
    true
}

fn message_author(message: &Message) -> AuthorIdentity {
    AuthorIdentity {
        username: message.author.name.clone(),
        display_avatar_url: message
            .author
            .avatar_url()
            .unwrap_or_else(|| message.author.default_avatar_url()),
    }
}

fn guild_tag_from_user(user: &User) -> Option<GuildTagIdentity> {
    let primary_guild = user.primary_guild.as_ref()?;
    if primary_guild.identity_enabled != Some(true) {
        return None;
    }
    let name = primary_guild.tag.as_deref()?.trim();
    if name.is_empty() {
        return None;
    }
    Some(GuildTagIdentity {
        name: name.into(),
        badge_url: primary_guild.badge_url(),
    })
}

fn message_timestamp(message: &Message) -> u64 {
    message.timestamp.unix_timestamp().max(0) as u64 * 1_000
}

async fn cancel_stage_tickets(core: &AppCore, tickets: &[StageTicket]) {
    for ticket in tickets {
        core.cancel_stage_output(*ticket).await;
    }
}

fn sticker_format(format: StickerFormatType) -> (&'static str, &'static str) {
    match format {
        StickerFormatType::Png => ("png", "image/png"),
        StickerFormatType::Apng => ("apng", "image/png"),
        StickerFormatType::Lottie => ("lottie", "application/json"),
        StickerFormatType::Gif => ("gif", "image/gif"),
        StickerFormatType::Unknown(_) => ("unknown", "application/octet-stream"),
        _ => ("unknown", "application/octet-stream"),
    }
}

async fn inspect_sticker(
    url: &str,
    format: &str,
    text: Option<&str>,
    config: &AppConfig,
) -> (privacy::PrivacyReport, Option<Vec<u8>>) {
    let visual_bytes = matches!(format, "png" | "apng" | "gif");
    let bytes = if visual_bytes {
        artwork::download_bounded(url, artwork::MAX_ARTWORK_BYTES)
            .await
            .ok()
    } else {
        None
    };
    if !config.privacy_scan_enabled {
        return (privacy::classify_text(text, config), bytes);
    }
    let mut report = match bytes.as_deref() {
        Some(bytes) => privacy::analyze_image_bytes_async(bytes, text, config).await,
        None => privacy::classify_text(text, config),
    };
    if !visual_bytes || bytes.is_none() {
        report.merge(privacy::PrivacyReport::suspicious("scan_incomplete"));
    }
    report.apply_score_policy(config);
    (report, bytes)
}

fn sticker_visual_segment(
    name: String,
    url: Option<String>,
    format: StickerFormatType,
) -> VisualSegment {
    VisualSegment {
        kind: "sticker".into(),
        value: name,
        url: url.filter(|_| {
            matches!(
                format,
                StickerFormatType::Png | StickerFormatType::Apng | StickerFormatType::Gif
            )
        }),
        animated: matches!(format, StickerFormatType::Apng | StickerFormatType::Gif),
    }
}

fn parse_visual_segments(content: &str) -> Option<Vec<VisualSegment>> {
    let mut segments = Vec::new();
    let mut text = String::new();
    let mut cursor = 0;
    let mut found_emoji = false;

    while cursor < content.len() {
        let remainder = &content[cursor..];
        if let Some((consumed, value, url, animated)) = parse_custom_emoji(remainder) {
            push_text_segment(&mut segments, &mut text);
            segments.push(VisualSegment {
                kind: "emoji".into(),
                value,
                url: Some(url),
                animated,
            });
            cursor += consumed;
            found_emoji = true;
            continue;
        }

        let character = remainder
            .chars()
            .next()
            .expect("cursor is on a character boundary");
        if is_unicode_emoji(character) {
            push_text_segment(&mut segments, &mut text);
            segments.push(VisualSegment {
                kind: "emoji".into(),
                value: character.to_string(),
                url: None,
                animated: false,
            });
            found_emoji = true;
        } else {
            text.push(character);
        }
        cursor += character.len_utf8();
    }
    push_text_segment(&mut segments, &mut text);
    found_emoji.then_some(segments)
}

fn parse_custom_emoji(content: &str) -> Option<(usize, String, String, bool)> {
    if !content.starts_with("<:") && !content.starts_with("<a:") {
        return None;
    }
    let end = content.find('>')?;
    let token = &content[..=end];
    let animated = token.starts_with("<a:");
    let body = token
        .strip_prefix(if animated { "<a:" } else { "<:" })?
        .strip_suffix('>')?;
    let (name, id) = body.rsplit_once(':')?;
    if name.is_empty() || id.len() > 20 || !id.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let url = format!("https://cdn.discordapp.com/emojis/{id}.webp?size=128&animated={animated}");
    Some((token.len(), format!(":{name}:"), url, animated))
}

fn plain_text_segments(text: String) -> Vec<VisualSegment> {
    vec![VisualSegment {
        kind: "text".into(),
        value: text,
        url: None,
        animated: false,
    }]
}

fn push_text_segment(segments: &mut Vec<VisualSegment>, text: &mut String) {
    if !text.is_empty() {
        segments.push(VisualSegment {
            kind: "text".into(),
            value: std::mem::take(text),
            url: None,
            animated: false,
        });
    }
}

fn is_unicode_emoji(character: char) -> bool {
    matches!(
        character as u32,
        0x1F000..=0x1FAFF
            | 0x2600..=0x27BF
            | 0x2300..=0x23FF
            | 0x2B00..=0x2BFF
            | 0xFE0F
    )
}

pub async fn start_bot(core: Arc<AppCore>) -> Result<bool> {
    let Some((credentials, _source)) = load_discord_credentials()? else {
        stop_bot(&core).await;
        *core.bot_status.write().await = BotStatus::default();
        return Ok(false);
    };

    let intents =
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    // Build the new client before stopping the running bot, so a build
    // failure leaves the current connection untouched.
    let mut client = Client::builder(&credentials.token, intents)
        .event_handler(Handler { core: core.clone() })
        .await
        .context("failed to create the Discord client")?;
    stop_bot(&core).await;
    let shard_manager = client.shard_manager.clone();
    let http = client.http.clone();
    let cache = client.cache.clone();
    let status_core = core.clone();
    let cleanup_http = http.clone();
    let task = tokio::spawn(async move {
        tokio::select! {
            result = client.start() => {
                if let Err(error) = result {
                    set_bot_error(&status_core, format!("Discord connection failed: {error}")).await;
                }
            }
            _ = crate::channel_cleanup::run(status_core.clone(), cleanup_http) => {}
        }
        status_core.bot_status.write().await.connected = false;
    });
    *core.bot_runtime.lock().await = Some(BotRuntime {
        shard_manager,
        task,
        http,
        cache,
    });
    Ok(true)
}

pub async fn refresh_channel_list(core: &Arc<AppCore>) -> Result<()> {
    let (http, cache) = {
        let runtime = core.bot_runtime.lock().await;
        let runtime = runtime.as_ref().context("the Discord bot is not running")?;
        (runtime.http.clone(), runtime.cache.clone())
    };
    if !core.bot_status.read().await.connected {
        bail!("the Discord bot is not connected");
    }
    let bot_id = cache.current_user().id;
    let guild_ids = cache.guilds();
    let channels = discover_channels(&http, &cache, bot_id, guild_ids).await;
    *core.channels.write().await = channels;
    warn_if_watched_channel_missing(core).await;
    Ok(())
}

pub async fn apply_bot_presence(core: &Arc<AppCore>, config: &AppConfig) {
    let shard_manager = {
        let runtime = core.bot_runtime.lock().await;
        runtime
            .as_ref()
            .map(|runtime| Arc::clone(&runtime.shard_manager))
    };
    let Some(shard_manager) = shard_manager else {
        return;
    };
    let messengers = shard_manager
        .runners
        .lock()
        .await
        .values()
        .map(|runner| runner.runner_tx.clone())
        .collect::<Vec<_>>();
    let (activity, status) = presence_from_config(config);
    for messenger in messengers {
        messenger.set_presence(activity.clone(), status);
    }
}

fn presence_from_config(config: &AppConfig) -> (Option<ActivityData>, OnlineStatus) {
    let status = match config.bot_online_status.as_str() {
        "idle" => OnlineStatus::Idle,
        "dnd" => OnlineStatus::DoNotDisturb,
        "invisible" => OnlineStatus::Invisible,
        _ => OnlineStatus::Online,
    };
    let text = config.bot_activity_text.trim();
    let activity = if text.is_empty() || config.bot_activity_type == "none" {
        None
    } else {
        Some(match config.bot_activity_type.as_str() {
            "playing" => ActivityData::playing(text),
            "listening" => ActivityData::listening(text),
            "watching" => ActivityData::watching(text),
            "competing" => ActivityData::competing(text),
            _ => ActivityData::custom(text),
        })
    };
    (activity, status)
}

async fn discover_channels(
    http: &Arc<Http>,
    cache: &Arc<Cache>,
    bot_id: UserId,
    guild_ids: Vec<GuildId>,
) -> Vec<ChannelSummary> {
    let mut channels = Vec::new();
    for guild_id in guild_ids {
        let guild_name = guild_id
            .to_partial_guild(http)
            .await
            .map(|guild| guild.name)
            .unwrap_or_else(|_| guild_id.to_string());
        if let Ok(guild_channels) = guild_id.channels(http).await {
            channels.extend(guild_channels.into_values().filter_map(|channel| {
                (matches!(channel.kind, ChannelType::Text | ChannelType::News)
                    && bot_can_view_channel(cache, &channel, bot_id))
                .then(|| ChannelSummary {
                    id: channel.id.to_string(),
                    name: channel.name,
                    guild_name: guild_name.clone(),
                })
            }));
        }
    }
    channels.sort_by(|left, right| {
        left.guild_name
            .cmp(&right.guild_name)
            .then(left.name.cmp(&right.name))
    });
    channels
}

const MISSING_CHANNEL_WARNING: &str = "A selected Discord channel is private or inaccessible. Add Relay or its role to the channel permissions.";

async fn warn_if_watched_channel_missing(core: &Arc<AppCore>) {
    let configured_channels = {
        let config = core.config.read().await;
        [
            config.watched_channel_id.clone(),
            config.tts_channel_id.clone(),
            config.music_channel_id.clone(),
        ]
    };
    let available_channels = core.channels.read().await;
    let missing = configured_channels.iter().any(|configured_channel| {
        !configured_channel.is_empty()
            && !available_channels
                .iter()
                .any(|channel| channel.id == *configured_channel)
    });
    let mut status = core.bot_status.write().await;
    if missing {
        status.error = Some(MISSING_CHANNEL_WARNING.into());
    } else if status.error.as_deref() == Some(MISSING_CHANNEL_WARNING) {
        status.error = None;
    }
}

pub async fn stop_bot(core: &Arc<AppCore>) {
    if let Some(runtime) = core.bot_runtime.lock().await.take() {
        runtime.shard_manager.shutdown_all().await;
        runtime.task.abort();
    }
    *core.bot_status.write().await = BotStatus::default();
    core.channels.write().await.clear();
}

pub fn invite_url(client_id: &str, config: &AppConfig) -> String {
    let permissions = (Permissions::VIEW_CHANNEL
        | Permissions::READ_MESSAGE_HISTORY
        | Permissions::MANAGE_CHANNELS
        | Permissions::MANAGE_ROLES
        | Permissions::MANAGE_MESSAGES
        | custom_commands::required_bot_permissions(&config.custom_commands))
    .bits();
    format!(
        "https://discord.com/oauth2/authorize?client_id={client_id}&permissions={permissions}&scope=bot%20applications.commands"
    )
}

fn relay_command(config: &AppConfig) -> CreateCommand {
    let mut command = CreateCommand::new("relay")
        .description("Configure the local OBS media relay")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "channel",
                "Set the channel whose media is relayed to OBS",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Channel, "channel", "Channel to watch")
                    .channel_types(vec![ChannelType::Text, ChannelType::News])
                    .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "nuke",
                "Recreate a channel to delete all of its messages",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Channel,
                    "channel",
                    "Channel to recreate",
                )
                .channel_types(vec![ChannelType::Text, ChannelType::News])
                .required(true),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "url",
            "Show the local relay and overlay URLs",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "show",
            "Show relay configuration and connection details",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "status",
            "Show live Relay, OBS, queue, and widget status",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "test",
                "Send a local test to one connected output",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "output",
                    "Output to test locally",
                )
                .add_string_choice("Media", "visual")
                .add_string_choice("Audio", "audio")
                .add_string_choice("Notification", "notification")
                .add_string_choice("Sticker", "sticker")
                .required(true),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "regenerate",
            "Reconnect local relay outputs without changing their URLs",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "clear",
                "Delete a chosen number of messages from one channel",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Channel,
                    "channel",
                    "Channel whose messages will be deleted",
                )
                .channel_types(vec![ChannelType::Text, ChannelType::News])
                .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "count",
                    "Number of messages to delete (1-1000)",
                )
                .min_int_value(1)
                .max_int_value(1_000)
                .required(true),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "lock",
            "Toggle the configured media channel lock",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "changelog",
                "Post the latest Relay release notes from GitHub",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Channel,
                    "channel",
                    "Channel that receives the release notes",
                )
                .channel_types(vec![ChannelType::Text, ChannelType::News])
                .required(true),
            ),
        );
    if config.reactions.enabled {
        let mut choice =
            CreateCommandOption::new(CommandOptionType::String, "name", "Reaction to play")
                .required(true);
        for reaction in config
            .reactions
            .definitions
            .iter()
            .filter(|item| item.enabled)
            .take(25)
        {
            choice = choice.add_string_choice(&reaction.name, &reaction.id);
        }
        command = command.add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "reaction",
                "Play a prepared sound or reaction",
            )
            .add_sub_option(choice),
        );
    }
    for custom in config
        .custom_commands
        .iter()
        .filter(|command| command.enabled)
    {
        command = command.add_option(custom.command_option());
    }
    command
}

pub async fn sync_relay_command_schema(core: &Arc<AppCore>, config: &AppConfig) -> Result<()> {
    let http = {
        let runtime = core.bot_runtime.lock().await;
        runtime
            .as_ref()
            .map(|runtime| runtime.http.clone())
            .context("the Discord bot is not running")?
    };
    if !core.bot_status.read().await.connected {
        bail!("the Discord bot is not connected");
    }
    Command::set_global_commands(&http, vec![relay_command(config)])
        .await
        .context("failed to synchronize the Relay command schema")?;
    Ok(())
}

async fn handle_relay(
    core: &Arc<AppCore>,
    http: &Http,
    command: &CommandInteraction,
) -> Result<String> {
    if let Some(option) = command
        .data
        .options
        .first()
        .filter(|option| option.name == "reaction")
    {
        let CommandDataOptionValue::SubCommand(arguments) = &option.value else {
            anyhow::bail!("Invalid reaction command.");
        };
        if command.guild_id.is_none() {
            anyhow::bail!("Reactions require a server.");
        }
        let id = arguments
            .iter()
            .find_map(|argument| match &argument.value {
                CommandDataOptionValue::String(value) if argument.name == "name" => {
                    Some(value.as_str())
                }
                _ => None,
            })
            .context("Choose a reaction.")?;
        let roles = command
            .member
            .as_ref()
            .map(|member| member.roles.iter().map(ToString::to_string).collect())
            .unwrap_or_default();
        let result = crate::reactions::trigger(
            core,
            id,
            Some((command.user.id.get(), command.channel_id.to_string(), roles)),
        )
        .await;
        let language = core.interface_preferences.read().await.language.clone();
        return Ok(crate::reactions_messages::reply(&language, result));
    }
    if !default_command_authorized(
        command.guild_id,
        command
            .member
            .as_deref()
            .and_then(|member| member.permissions),
    ) {
        return Ok("Default Relay commands require Discord Administrator permission.".into());
    }
    let Some(option) = command.data.options.first() else {
        return Ok("Choose a Relay subcommand.".into());
    };
    let CommandDataOptionValue::SubCommand(arguments) = &option.value else {
        return Ok("Invalid Relay command.".into());
    };
    let config = core.config.read().await.clone();
    let lock_can_restore = option.name == "lock" && config.channel_lock.is_some();
    if !command_enabled(&config, &option.name) && !lock_can_restore {
        return Ok(format!(
            "`/relay {}` is disabled on the Commands page in the Relay application.",
            option.name
        ));
    }

    match option.name.as_str() {
        "channel" => {
            let channel_id = arguments
                .iter()
                .find_map(|argument| match argument.value {
                    CommandDataOptionValue::Channel(channel_id) => Some(channel_id.to_string()),
                    _ => None,
                })
                .context("a channel is required")?;
            core.update_config(|config| config.watched_channel_id = channel_id.clone())
                .await?;
            Ok(format!("Relay channel set to <#{channel_id}>."))
        }
        "url" => {
            let config = core.config.read().await.clone();
            Ok(connection_details(&config))
        }
        "show" => {
            let config = core.config.read().await.clone();
            let channel = if config.watched_channel_id.is_empty() {
                "not configured".to_owned()
            } else {
                format!("<#{}>", config.watched_channel_id)
            };
            Ok(format!(
                "Channel: {channel}\n{}",
                connection_details(&config)
            ))
        }
        "status" => relay_status(core).await,
        "test" => {
            let target = arguments
                .iter()
                .find_map(|argument| match &argument.value {
                    CommandDataOptionValue::String(value) => output_test_target(value),
                    _ => None,
                })
                .context("an output is required")?;
            relay_output_test(core, target).await
        }
        "regenerate" => {
            let config = core.config.read().await.clone();
            Ok(format!(
                "The permanent relay URL was preserved. No OBS update is required:\n{}",
                overlay_url(&config)
            ))
        }
        "clear" => {
            let channel_id = arguments
                .iter()
                .find_map(|argument| match argument.value {
                    CommandDataOptionValue::Channel(channel_id) => Some(channel_id),
                    _ => None,
                })
                .context("a channel is required")?;
            let count = arguments
                .iter()
                .find_map(|argument| match argument.value {
                    CommandDataOptionValue::Integer(value) => usize::try_from(value).ok(),
                    _ => None,
                })
                .filter(|count| (1..=1_000).contains(count))
                .context("a message count between 1 and 1000 is required")?;
            clear_selected_channel(core, http, channel_id, count).await
        }
        "nuke" => {
            let channel_id = arguments
                .iter()
                .find_map(|argument| match argument.value {
                    CommandDataOptionValue::Channel(channel_id) => Some(channel_id),
                    _ => None,
                })
                .context("a channel is required")?;
            nuke_selected_channel(core, http, channel_id).await
        }
        "lock" => toggle_channel_lock(core, http).await,
        "changelog" => {
            let channel_id = arguments
                .iter()
                .find_map(|argument| match argument.value {
                    CommandDataOptionValue::Channel(channel_id) => Some(channel_id),
                    _ => None,
                })
                .context("a channel is required")?;
            post_changelog(core, http, channel_id).await
        }
        _ => Ok("Unknown Relay subcommand.".into()),
    }
}

const CHANGELOG_URL: &str = "https://raw.githubusercontent.com/stealthsrc/relay/main/CHANGELOG.md";
const CHANGELOG_PAGE_URL: &str = "https://github.com/stealthsrc/relay/blob/main/CHANGELOG.md";
const CHANGELOG_MAX_BYTES: usize = 256 * 1024;
const CHANGELOG_EMBED_DESCRIPTION_LIMIT: usize = 3_900;
const CHANGELOG_MAX_EMBEDS: usize = 10;
const CHANGELOG_EMBED_COLOUR: u32 = 0x2F_B3_A8;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChangelogSection {
    heading: String,
    version: String,
    date: Option<String>,
    body: String,
}

async fn post_changelog(core: &Arc<AppCore>, http: &Http, channel_id: ChannelId) -> Result<String> {
    let language = core.interface_preferences.read().await.language.clone();
    let changelog = fetch_changelog_markdown()
        .await
        .context("failed to download CHANGELOG.md from GitHub")?;
    let mut section = latest_changelog_section(&changelog)
        .context("no published release section was found in CHANGELOG.md")?;
    section.body = crate::changelog::changelog_body_for_language(&section.body, &language);
    let (embeds, truncated) = build_changelog_embeds(&section);
    if embeds.is_empty() {
        bail!("the latest changelog section is empty");
    }

    for embed in embeds {
        channel_id
            .send_message(
                http,
                CreateMessage::new()
                    .embed(embed)
                    .allowed_mentions(CreateAllowedMentions::new()),
            )
            .await
            .context(
                "failed to post the changelog — ensure the bot can View Channel, Send Messages, and Embed Links in that channel",
            )?;
    }

    let mut confirmation = format!(
        "Posted Relay **{}** release notes as embed(s) to <#{}>.",
        section.version, channel_id
    );
    if truncated {
        confirmation.push_str(&format!(
            "\nSome content was truncated. Full notes: <{CHANGELOG_PAGE_URL}>"
        ));
    }
    Ok(confirmation)
}

async fn fetch_changelog_markdown() -> Result<String> {
    let url = reqwest::Url::parse(CHANGELOG_URL).context("invalid changelog URL")?;
    if url.scheme() != "https"
        || url.host_str() != Some("raw.githubusercontent.com")
        || url.path() != "/stealthsrc/relay/main/CHANGELOG.md"
    {
        bail!("changelog URL is not the expected GitHub raw path");
    }

    let client = reqwest::Client::builder()
        .user_agent(concat!("Relay/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("unable to create the changelog HTTP client")?;
    let response = client
        .get(url)
        .send()
        .await
        .context("unable to reach GitHub")?
        .error_for_status()
        .context("GitHub rejected the changelog request")?;
    if response
        .content_length()
        .is_some_and(|length| length > CHANGELOG_MAX_BYTES as u64)
    {
        bail!("CHANGELOG.md exceeds the download size limit");
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("failed while reading CHANGELOG.md")?;
        if body.len() + chunk.len() > CHANGELOG_MAX_BYTES {
            bail!("CHANGELOG.md exceeds the download size limit");
        }
        body.extend_from_slice(&chunk);
    }

    String::from_utf8(body).context("CHANGELOG.md is not valid UTF-8")
}

fn latest_changelog_section(changelog: &str) -> Option<ChangelogSection> {
    let mut heading = None;
    let mut lines = Vec::new();
    let mut in_release = false;
    for line in changelog.lines() {
        if line.starts_with("## ") {
            if in_release {
                break;
            }
            in_release = line.starts_with("## [") && !line.starts_with("## [Unreleased]");
            if !in_release {
                continue;
            }
            heading = Some(line.to_owned());
            continue;
        } else if line.starts_with('[') && line.contains("]: http") {
            continue;
        }
        if in_release {
            lines.push(line);
        }
    }
    let heading = heading?;
    let (version, date) = parse_changelog_heading(&heading)?;
    let body = lines.join("\n").trim().to_owned();
    if body.is_empty() {
        return None;
    }
    Some(ChangelogSection {
        heading,
        version,
        date,
        body,
    })
}

fn parse_changelog_heading(heading: &str) -> Option<(String, Option<String>)> {
    let rest = heading.strip_prefix("## [")?;
    let (version, after) = rest.split_once(']')?;
    let version = version.trim();
    if version.is_empty() || version.eq_ignore_ascii_case("unreleased") {
        return None;
    }
    let date = after
        .trim()
        .strip_prefix('-')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    Some((version.to_owned(), date))
}

fn discord_format_changelog_body(body: &str) -> String {
    let mut formatted = String::new();
    for line in body.lines() {
        if let Some(title) = line.strip_prefix("#### ") {
            if !formatted.is_empty() {
                formatted.push('\n');
            }
            formatted.push_str("**");
            formatted.push_str(title.trim());
            formatted.push_str("**\n");
            continue;
        }
        if let Some(title) = line.strip_prefix("### ") {
            if !formatted.is_empty() {
                formatted.push('\n');
            }
            formatted.push_str("**");
            formatted.push_str(title.trim());
            formatted.push_str("**\n");
            continue;
        }
        if let Some(item) = line.strip_prefix("- ") {
            formatted.push('•');
            formatted.push(' ');
            formatted.push_str(item);
            formatted.push('\n');
            continue;
        }
        if line.starts_with("## ") {
            continue;
        }
        formatted.push_str(line);
        formatted.push('\n');
    }
    formatted.trim().to_owned()
}

fn build_changelog_embeds(section: &ChangelogSection) -> (Vec<CreateEmbed>, bool) {
    let colour = Colour::new(CHANGELOG_EMBED_COLOUR);
    let formatted = discord_format_changelog_body(&section.body);
    let mut descriptions = split_message_chunks(&formatted, CHANGELOG_EMBED_DESCRIPTION_LIMIT);
    if descriptions.is_empty() {
        descriptions.push(String::new());
    }

    let truncated = descriptions.len() > CHANGELOG_MAX_EMBEDS;
    if truncated {
        descriptions.truncate(CHANGELOG_MAX_EMBEDS);
        if let Some(last) = descriptions.last_mut() {
            let notice = format!("\n\n…truncated. Full changelog: {CHANGELOG_PAGE_URL}");
            while char_len(last) + char_len(&notice) > CHANGELOG_EMBED_DESCRIPTION_LIMIT
                && !last.is_empty()
            {
                last.pop();
            }
            last.push_str(&notice);
        }
    }

    let total = descriptions.len();
    let footer_text = match &section.date {
        Some(date) => format!("Released {date} · Synced from GitHub CHANGELOG.md"),
        None => "Synced from GitHub CHANGELOG.md".to_owned(),
    };
    let embeds = descriptions
        .into_iter()
        .enumerate()
        .map(|(index, description)| {
            let title = if total == 1 {
                format!("Relay {}", section.version)
            } else {
                format!("Relay {} ({}/{})", section.version, index + 1, total)
            };
            let mut embed = CreateEmbed::new()
                .colour(colour)
                .title(title)
                .url(CHANGELOG_PAGE_URL)
                .description(description);
            if index + 1 == total {
                embed = embed.footer(serenity::all::CreateEmbedFooter::new(footer_text.clone()));
            }
            embed
        })
        .collect();

    (embeds, truncated)
}

fn char_len(value: &str) -> usize {
    value.chars().count()
}

fn split_message_chunks(text: &str, limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        let mut remaining = line;
        while char_len(remaining) > limit {
            let (head, tail) = split_at_char_boundary(remaining, limit);
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            chunks.push(head.to_owned());
            remaining = tail;
        }
        if !current.is_empty() && char_len(&current) + char_len(remaining) + 1 > limit {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(remaining);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn split_at_char_boundary(value: &str, char_count: usize) -> (&str, &str) {
    match value.char_indices().nth(char_count) {
        Some((index, _)) => (&value[..index], &value[index..]),
        None => (value, ""),
    }
}

pub(crate) async fn clear_selected_channel(
    core: &AppCore,
    http: &Http,
    channel_id: ChannelId,
    count: usize,
) -> Result<String> {
    let protected = {
        let config = core.config.read().await;
        crate::reaction_protection::protected_message_for_channel(
            &config.reactions.protected_channel_id,
            &config.reactions.protected_message_id,
            channel_id.get(),
        )
    };
    let deleted = clear_channel_messages(http, channel_id, count, protected).await?;
    Ok(format!(
        "Cleared {deleted} message(s) from <#{channel_id}>."
    ))
}

async fn nuke_selected_channel(
    core: &Arc<AppCore>,
    http: &Http,
    channel_id: ChannelId,
) -> Result<String> {
    let protected = {
        let config = core.config.read().await;
        crate::reaction_protection::is_channel_protected(
            &config.reactions.protected_channel_id,
            &config.reactions.protected_message_id,
            channel_id.get(),
        )
    };
    if protected {
        bail!(
            "Cannot recreate this channel while it contains the protected reaction message. Clear or change the protected message first."
        );
    }
    let Channel::Guild(channel) = channel_id.to_channel(http).await? else {
        bail!("only text and announcement channels can be recreated");
    };
    if !matches!(channel.kind, ChannelType::Text | ChannelType::News) {
        bail!("only text and announcement channels can be recreated");
    }

    let mut replacement = CreateChannel::new(channel.name.clone())
        .kind(channel.kind)
        .position(channel.position)
        .permissions(channel.permission_overwrites.clone())
        .nsfw(channel.nsfw)
        .rate_limit_per_user(channel.rate_limit_per_user.unwrap_or_default());
    if let Some(parent_id) = channel.parent_id {
        replacement = replacement.category(parent_id);
    }
    if let Some(topic) = channel.topic.as_deref() {
        replacement = replacement.topic(topic);
    }
    let replacement = channel.guild_id.create_channel(http, replacement).await?;
    let replacement_id = replacement.id;
    let old_id = channel.id;

    core.update_config(|config| {
        replace_configured_channel_id(config, old_id, replacement_id);
    })
    .await?;
    channel.delete(http).await?;
    Ok(format!(
        "Recreated <#{old_id}> as <#{replacement_id}>. Its message history was deleted."
    ))
}

fn replace_configured_channel_id(
    config: &mut AppConfig,
    old_channel_id: ChannelId,
    replacement_channel_id: ChannelId,
) {
    let old_channel_id = old_channel_id.to_string();
    let replacement_channel_id = replacement_channel_id.to_string();
    if config.watched_channel_id == old_channel_id {
        config.media_cleanup_enabled = false;
        config.media_welcome_message_id.clear();
    }
    if config.tts_channel_id == old_channel_id {
        config.tts_cleanup_enabled = false;
        config.tts_welcome_message_id.clear();
    }
    if config.music_channel_id == old_channel_id {
        config.music_cleanup_enabled = false;
        config.music_welcome_message_id.clear();
    }
    for channel_id in [
        &mut config.watched_channel_id,
        &mut config.tts_channel_id,
        &mut config.music_channel_id,
        &mut config.honeypot_channel_id,
    ] {
        if *channel_id == old_channel_id {
            *channel_id = replacement_channel_id.clone();
        }
    }
    if let Some(lock) = config.channel_lock.as_mut()
        && lock.channel_id == old_channel_id
    {
        lock.channel_id = replacement_channel_id;
    }
}

async fn clear_channel_messages(
    http: &Http,
    channel_id: ChannelId,
    limit: usize,
    protected: Option<u64>,
) -> Result<usize> {
    let mut before = None;
    let mut deleted = 0;
    while deleted < limit {
        let page_limit = (limit - deleted).min(100) as u8;
        let mut request = GetMessages::new().limit(page_limit);
        if let Some(message_id) = before {
            request = request.before(message_id);
        }
        let messages = channel_id.messages(http, request).await?;
        if messages.is_empty() {
            break;
        }
        before = messages.last().map(|message| message.id);
        let now = current_timestamp_ms() / 1_000;
        let (recent, old): (Vec<MessageId>, Vec<MessageId>) = messages
            .iter()
            .take(limit - deleted)
            .filter(|message| clear_candidate(message.id, protected))
            .map(|message| message.id)
            .partition(|message_id| is_bulk_deletable(*message_id, now));

        if recent.len() >= 2 {
            channel_id.delete_messages(http, &recent).await?;
            deleted += recent.len();
        } else if let Some(message_id) = recent.first() {
            channel_id.delete_message(http, message_id).await?;
            deleted += 1;
        }
        for message_id in old {
            channel_id.delete_message(http, message_id).await?;
            deleted += 1;
        }
        if messages.len() < usize::from(page_limit) {
            break;
        }
    }
    Ok(deleted)
}

fn clear_candidate(message_id: MessageId, protected: Option<u64>) -> bool {
    Some(message_id.get()) != protected
}

fn is_bulk_deletable(message_id: MessageId, now_seconds: u64) -> bool {
    const SAFE_BULK_DELETE_AGE_SECONDS: u64 = 13 * 24 * 60 * 60 + 23 * 60 * 60;
    let created = message_id.created_at().unix_timestamp().max(0) as u64;
    now_seconds.saturating_sub(created) < SAFE_BULK_DELETE_AGE_SECONDS
}

fn command_enabled(config: &AppConfig, command: &str) -> bool {
    match command {
        "channel" => config.command_channel_enabled,
        "url" => config.command_url_enabled,
        "show" => config.command_show_enabled,
        "status" => config.command_status_enabled,
        "test" => config.command_test_enabled,
        "regenerate" => config.command_regenerate_enabled,
        "clear" => config.command_clear_enabled,
        "nuke" => config.command_nuke_enabled,
        "lock" => config.command_lock_enabled,
        "changelog" => config.command_changelog_enabled,
        _ => false,
    }
}

fn default_command_authorized(guild_id: Option<GuildId>, permissions: Option<Permissions>) -> bool {
    guild_id.is_some()
        && permissions.is_some_and(|permissions| permissions.contains(Permissions::ADMINISTRATOR))
}

async fn relay_status(core: &AppCore) -> Result<String> {
    let config = core.config.read().await.clone();
    let bot = core.bot_status.read().await.clone();
    let server = core.server_status.read().await.clone();
    let moderation_pending = core.pending_media.read().await.len();
    Ok(format_relay_status(
        &config,
        &bot,
        &server,
        moderation_pending,
        core.tts_pending_count(),
    ))
}

fn format_relay_status(
    config: &AppConfig,
    bot: &BotStatus,
    server: &ServerStatus,
    moderation_pending: usize,
    tts_pending: usize,
) -> String {
    let bot_state = if bot.connected {
        bot.username
            .as_deref()
            .map(|username| format!("connected as {username}"))
            .unwrap_or_else(|| "connected".into())
    } else {
        "disconnected".into()
    };
    let media_channel = if config.watched_channel_id.is_empty() {
        "not configured".into()
    } else {
        format!("<#{}>", config.watched_channel_id)
    };
    let tts_channel = if config.tts_channel_id.is_empty() {
        "disabled".into()
    } else {
        format!("<#{}>", config.tts_channel_id)
    };
    let moderation = if config.moderation_enabled {
        format!("enabled ({moderation_pending} pending)")
    } else {
        "disabled".into()
    };
    let media_widget = widget_status(config.widget_visible, config.widget_locked);
    let notification_widget = widget_status(
        config.notification_widget_visible,
        config.notification_widget_locked,
    );

    format!(
        "**Relay status**\n\
         Bot: {bot_state}\n\
         Local server: {}\n\
         Media channel: {media_channel}\n\
         Message channel: {tts_channel}\n\
         Moderation: {moderation}\n\
         Messages preparing: {tts_pending}\n\
         Media widget: {media_widget}\n\
         Notification widget: {notification_widget}\n\
         **Connected outputs (OBS / widget / preview)**\n\
         Visual: {}\n\
         Audio: {}\n\
         Notifications: {}\n\
         Stickers: {}",
        if server.connected {
            "online"
        } else {
            "offline"
        },
        output_status(&server.outputs.visual),
        output_status(&server.outputs.audio),
        output_status(&server.outputs.notification),
        output_status(&server.outputs.sticker),
    )
}

fn widget_status(visible: bool, locked: bool) -> &'static str {
    match (visible, locked) {
        (false, _) => "hidden",
        (true, true) => "visible and locked",
        (true, false) => "visible and movable",
    }
}

fn output_status(status: &OutputConnectionStatus) -> String {
    format!(
        "{} / {} / {}",
        status.obs_clients, status.widget_clients, status.preview_clients
    )
}

async fn relay_output_test(core: &AppCore, target: OutputTestTarget) -> Result<String> {
    let connected = {
        let server = core.server_status.read().await;
        connected_output_count(&server, target)
    };
    let label = output_test_label(target);
    if connected == 0 {
        return Ok(format!(
            "No live {label} output is connected. Connect the matching OBS source or Windows output first."
        ));
    }
    emit_output_test(core, target).await?;
    Ok(format!(
        "Local {label} test sent to {connected} connected output(s). Nothing was posted to Discord or added to Relay history."
    ))
}

fn output_test_target(value: &str) -> Option<OutputTestTarget> {
    match value {
        "visual" | "media" => Some(OutputTestTarget::Visual),
        "audio" => Some(OutputTestTarget::Audio),
        "tts" => Some(OutputTestTarget::Notification),
        "notification" => Some(OutputTestTarget::Notification),
        "sticker" => Some(OutputTestTarget::Sticker),
        _ => None,
    }
}

fn connected_output_count(server: &ServerStatus, target: OutputTestTarget) -> usize {
    if !server.connected {
        return 0;
    }
    let status = match target {
        OutputTestTarget::Visual => &server.outputs.visual,
        OutputTestTarget::Audio => &server.outputs.audio,
        OutputTestTarget::Tts => &server.outputs.tts,
        OutputTestTarget::Notification => &server.outputs.notification,
        OutputTestTarget::Sticker => &server.outputs.sticker,
    };
    status.obs_clients + status.widget_clients
}

fn output_test_label(target: OutputTestTarget) -> &'static str {
    match target {
        OutputTestTarget::Visual => "media",
        OutputTestTarget::Audio => "audio",
        OutputTestTarget::Tts => "notifications",
        OutputTestTarget::Notification => "notification",
        OutputTestTarget::Sticker => "sticker",
    }
}

async fn toggle_channel_lock(core: &Arc<AppCore>, http: &Http) -> Result<String> {
    let config = core.config.read().await.clone();
    if let Some(snapshot) = config.channel_lock.clone() {
        restore_channel_permissions(http, &snapshot).await?;
        core.update_config(|next| next.channel_lock = None).await?;
        return Ok(format!("<#{0}> is unlocked.", snapshot.channel_id));
    }
    if config.watched_channel_id.is_empty() {
        bail!("configure a media channel before locking it");
    }

    let channel_id = ChannelId::new(config.watched_channel_id.parse()?);
    let Channel::Guild(channel) = channel_id.to_channel(http).await? else {
        bail!("the configured media channel is not a server text channel");
    };
    let roles = channel.guild_id.roles(http).await?;
    let everyone = channel.guild_id.everyone_role();
    let mut targets = vec![PermissionOverwriteType::Role(everyone)];
    targets.extend(roles.values().filter_map(|role| {
        (role.id != everyone
            && role.permissions.intersects(
                Permissions::ADMINISTRATOR
                    | Permissions::MANAGE_CHANNELS
                    | Permissions::MANAGE_MESSAGES,
            ))
        .then_some(PermissionOverwriteType::Role(role.id))
    }));

    let snapshot = ChannelLockSnapshot {
        channel_id: channel.id.to_string(),
        overwrites: targets
            .iter()
            .filter_map(|kind| snapshot_permission(&channel.permission_overwrites, *kind))
            .collect(),
    };
    let lock_snapshot = snapshot.clone();
    core.update_config(|next| next.channel_lock = Some(lock_snapshot))
        .await?;

    for kind in targets {
        let existing = channel
            .permission_overwrites
            .iter()
            .find(|overwrite| overwrite.kind == kind);
        let mut allow = existing.map_or(Permissions::empty(), |overwrite| overwrite.allow);
        let mut deny = existing.map_or(Permissions::empty(), |overwrite| overwrite.deny);
        if kind == PermissionOverwriteType::Role(everyone) {
            allow.remove(Permissions::SEND_MESSAGES);
            deny.insert(Permissions::SEND_MESSAGES);
        } else {
            deny.remove(Permissions::SEND_MESSAGES);
            allow.insert(Permissions::SEND_MESSAGES);
        }
        if let Err(error) = channel
            .create_permission(http, PermissionOverwrite { allow, deny, kind })
            .await
        {
            if restore_channel_permissions(http, &snapshot).await.is_ok() {
                let _ = core
                    .update_config(|rollback| rollback.channel_lock = None)
                    .await;
            }
            bail!("Discord refused the channel lock: {error}");
        }
    }
    Ok(format!(
        "<#{0}> is locked. Administrators and moderation roles can still write.",
        snapshot.channel_id
    ))
}

fn snapshot_permission(
    overwrites: &[PermissionOverwrite],
    kind: PermissionOverwriteType,
) -> Option<PermissionOverwriteSnapshot> {
    let (target_kind, target_id) = match kind {
        PermissionOverwriteType::Member(id) => ("member", id.to_string()),
        PermissionOverwriteType::Role(id) => ("role", id.to_string()),
        _ => return None,
    };
    let existing = overwrites.iter().find(|overwrite| overwrite.kind == kind);
    Some(PermissionOverwriteSnapshot {
        target_id,
        target_kind: target_kind.into(),
        allow: existing.map_or(0, |overwrite| overwrite.allow.bits()),
        deny: existing.map_or(0, |overwrite| overwrite.deny.bits()),
        existed: existing.is_some(),
    })
}

async fn restore_channel_permissions(http: &Http, snapshot: &ChannelLockSnapshot) -> Result<()> {
    let channel_id = ChannelId::new(snapshot.channel_id.parse()?);
    let Channel::Guild(channel) = channel_id.to_channel(http).await? else {
        bail!("the locked channel is no longer a server channel");
    };
    for saved in &snapshot.overwrites {
        let id = saved.target_id.parse::<u64>()?;
        let kind = match saved.target_kind.as_str() {
            "member" => PermissionOverwriteType::Member(UserId::new(id)),
            "role" => PermissionOverwriteType::Role(serenity::all::RoleId::new(id)),
            _ => bail!("the saved channel permission target is invalid"),
        };
        if saved.existed {
            channel
                .create_permission(
                    http,
                    PermissionOverwrite {
                        allow: Permissions::from_bits_truncate(saved.allow),
                        deny: Permissions::from_bits_truncate(saved.deny),
                        kind,
                    },
                )
                .await?;
        } else if channel
            .permission_overwrites
            .iter()
            .any(|overwrite| overwrite.kind == kind)
        {
            channel.delete_permission(http, kind).await?;
        }
    }
    Ok(())
}

fn connection_details(config: &AppConfig) -> String {
    format!(
        "Relay URL: `http://127.0.0.1:{}`\nVisual overlay: `{}`\nAudio overlay: `{}`",
        config.port,
        overlay_url(config),
        audio_overlay_url(config)
    )
}

fn overlay_url(config: &AppConfig) -> String {
    format!(
        "http://{}:{}/obs/visual",
        crate::widget::youtube_embed_host(),
        config.port
    )
}

fn audio_overlay_url(config: &AppConfig) -> String {
    format!("http://127.0.0.1:{}/obs/audio", config.port)
}

fn classify_attachment(attachment: &serenity::all::Attachment) -> Option<MediaKind> {
    classify_media(&attachment.filename, attachment.content_type.as_deref())
}

struct EmbeddedGif {
    url: String,
    proxy_url: String,
    title: Option<String>,
    content_type: &'static str,
}

struct DeferredEmbedMessage {
    channel_id: String,
    message_id: String,
    author: serenity::all::User,
    timestamp: u64,
    content: String,
    embeds: Vec<serenity::all::Embed>,
    role_ids: Vec<String>,
}

async fn submit_embedded_gifs(core: &Arc<AppCore>, http: &Http, message: &Message) {
    submit_deferred_embeds(
        core,
        http,
        DeferredEmbedMessage {
            channel_id: message.channel_id.to_string(),
            message_id: message.id.to_string(),
            author: message.author.clone(),
            timestamp: message.timestamp.unix_timestamp().max(0) as u64 * 1_000,
            content: message.content.clone(),
            embeds: message.embeds.clone(),
            role_ids: message_role_ids(message),
        },
    )
    .await;
}

async fn submit_deferred_embeds(core: &Arc<AppCore>, http: &Http, message: DeferredEmbedMessage) {
    if message.author.bot {
        return;
    }
    let config = core.config.read().await.clone();
    let scoped_config = privacy::scoped_config_for_roles(&config, &message.role_ids);
    if config.watched_channel_id.is_empty() || message.channel_id != config.watched_channel_id {
        return;
    }
    let message_report = classify_privacy_values(
        &message.content,
        std::iter::empty::<&str>(),
        std::iter::empty::<&str>(),
        &scoped_config,
    );
    if privacy_action_is_blocked(&message_report, &scoped_config) {
        delete_deferred_message_if_needed(core, http, &message, &message_report, &scoped_config)
            .await;
        return;
    }
    let media_text = prepare_media_text(&message.content);
    for (index, embed) in message.embeds.iter().filter_map(embedded_gif).enumerate() {
        let event_id = format!("{}-embed-{index}", message.message_id);
        if !core.claim_embed(event_id.clone()).await {
            continue;
        }
        let downloaded =
            match artwork::download_bounded(&embed.url, artwork::MAX_EMBED_MEDIA_BYTES).await {
                Ok(bytes) => Some(bytes),
                Err(_) if embed.proxy_url != embed.url => {
                    artwork::download_bounded(&embed.proxy_url, artwork::MAX_EMBED_MEDIA_BYTES)
                        .await
                        .ok()
                }
                Err(_) => None,
            };
        let mut content_type = embed.content_type;
        if let Some(bytes) = downloaded.as_deref() {
            content_type = sniff_media_type(bytes, content_type);
        }
        let event = MediaEvent {
            kind: MediaKind::Gif,
            url: embed.url,
            proxy_url: embed.proxy_url,
            filename: embed.title.unwrap_or_else(|| "Discord GIF".into()),
            content_type: content_type.into(),
            artwork_id: None,
            audio_id: None,
            cached_media_id: None,
            title: None,
            artist: None,
            text: media_text.clone(),
            author: AuthorIdentity {
                username: message.author.name.clone(),
                display_avatar_url: message
                    .author
                    .avatar_url()
                    .unwrap_or_else(|| message.author.default_avatar_url()),
            },
            timestamp: message.timestamp,
            message_id: message.message_id.clone(),
        };
        let privacy_report = if event
            .content_type
            .to_ascii_lowercase()
            .starts_with("image/")
        {
            match downloaded.as_deref() {
                Some(bytes) => {
                    privacy::analyze_image_bytes_async(
                        bytes,
                        Some(&message.content),
                        &scoped_config,
                    )
                    .await
                }
                None => {
                    privacy::analyze_remote_image(
                        &event.url,
                        &event.proxy_url,
                        Some(&message.content),
                        &scoped_config,
                    )
                    .await
                }
            }
        } else {
            privacy::classify_text(Some(&message.content), &scoped_config)
        };
        let current_config = core.config.read().await.clone();
        let current_scoped_config =
            privacy::scoped_config_for_roles(&current_config, &message.role_ids);
        let privacy_report =
            reclassify_privacy_report(privacy_report, &message.content, &current_scoped_config);
        if privacy_action_is_blocked(&privacy_report, &current_scoped_config) {
            delete_deferred_message_if_needed(
                core,
                http,
                &message,
                &privacy_report,
                &current_scoped_config,
            )
            .await;
            return;
        }
        core.submit_analyzed_media_with_text_and_roles(
            event,
            Some(privacy_report),
            Some(&message.content),
            &message.role_ids,
        )
        .await;
    }
}

async fn delete_deferred_message_if_needed(
    core: &Arc<AppCore>,
    http: &Http,
    message: &DeferredEmbedMessage,
    report: &privacy::PrivacyReport,
    config: &AppConfig,
) {
    let (Ok(channel_id), Ok(message_id)) = (
        message.channel_id.parse::<u64>(),
        message.message_id.parse::<u64>(),
    ) else {
        return;
    };
    if should_auto_delete_blocked_message(report, config)
        && ChannelId::new(channel_id)
            .delete_message(http, MessageId::new(message_id))
            .await
            .is_err()
    {
        core.bot_status.write().await.error =
            Some("Privacy deletion failed. Verify Manage Messages in this channel.".into());
    }
}

fn embedded_gif(embed: &serenity::all::Embed) -> Option<EmbeddedGif> {
    let is_gifv = embed.kind.as_deref() == Some("gifv");
    let image_is_gif = [
        embed.url.as_deref(),
        embed.image.as_ref().map(|image| image.url.as_str()),
        embed
            .image
            .as_ref()
            .and_then(|image| image.proxy_url.as_deref()),
        embed
            .thumbnail
            .as_ref()
            .map(|thumbnail| thumbnail.url.as_str()),
        embed
            .thumbnail
            .as_ref()
            .and_then(|thumbnail| thumbnail.proxy_url.as_deref()),
    ]
    .into_iter()
    .flatten()
    .any(|url| url_has_extension(url, "gif"));
    let known_provider = [
        embed.url.as_deref(),
        embed
            .provider
            .as_ref()
            .and_then(|provider| provider.name.as_deref()),
        embed
            .provider
            .as_ref()
            .and_then(|provider| provider.url.as_deref()),
        embed.image.as_ref().map(|image| image.url.as_str()),
        embed.video.as_ref().map(|video| video.url.as_str()),
    ]
    .into_iter()
    .flatten()
    .any(|value| {
        let value = value.to_ascii_lowercase();
        value.contains("klipy") || value.contains("tenor") || value.contains("giphy")
    });
    if !is_gifv && !image_is_gif && !known_provider {
        return None;
    }
    if let Some(video) = embed.video.as_ref() {
        let content_type = video_mime_type(&video.url)
            .or_else(|| video.proxy_url.as_deref().and_then(video_mime_type))
            .unwrap_or("video/mp4");
        return Some(EmbeddedGif {
            url: video.url.clone(),
            proxy_url: video.proxy_url.clone().unwrap_or_else(|| video.url.clone()),
            title: embed.title.clone(),
            content_type,
        });
    }
    let video_url = [
        embed.image.as_ref().map(|media| media.url.as_str()),
        embed
            .image
            .as_ref()
            .and_then(|media| media.proxy_url.as_deref()),
        embed.thumbnail.as_ref().map(|media| media.url.as_str()),
        embed
            .thumbnail
            .as_ref()
            .and_then(|media| media.proxy_url.as_deref()),
    ]
    .into_iter()
    .flatten()
    .find_map(|url| video_mime_type(url).map(|content_type| (url, content_type)));
    if let Some((url, content_type)) = video_url {
        return Some(EmbeddedGif {
            url: url.to_owned(),
            proxy_url: url.to_owned(),
            title: embed.title.clone(),
            content_type,
        });
    }
    let (url, proxy_url) = if let Some(image) = embed.image.as_ref() {
        (
            image.url.clone(),
            image.proxy_url.clone().unwrap_or_else(|| image.url.clone()),
        )
    } else {
        let thumbnail = embed.thumbnail.as_ref()?;
        (
            thumbnail.url.clone(),
            thumbnail
                .proxy_url
                .clone()
                .unwrap_or_else(|| thumbnail.url.clone()),
        )
    };
    Some(EmbeddedGif {
        url: url.clone(),
        proxy_url,
        title: embed.title.clone(),
        content_type: if url_has_extension(&url, "webp") {
            "image/webp"
        } else {
            "image/gif"
        },
    })
}

fn video_mime_type(url: &str) -> Option<&'static str> {
    if url_has_extension(url, "mp4") || url_has_extension(url, "m4v") {
        Some("video/mp4")
    } else if url_has_extension(url, "webm") {
        Some("video/webm")
    } else {
        None
    }
}

fn sniff_media_type(bytes: &[u8], fallback: &'static str) -> &'static str {
    if bytes.get(4..8) == Some(b"ftyp".as_slice()) {
        "video/mp4"
    } else if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        "video/webm"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP".as_slice()) {
        "image/webp"
    } else {
        fallback
    }
}

#[allow(deprecated)]
fn bot_can_view_channel(
    cache: &Arc<Cache>,
    channel: &serenity::all::GuildChannel,
    bot_id: UserId,
) -> bool {
    channel
        .permissions_for_user(cache, bot_id)
        .map(|permissions| permissions.contains(Permissions::VIEW_CHANNEL))
        .unwrap_or(true)
}

fn url_has_extension(url: &str, extension: &str) -> bool {
    url.split(['?', '#']).next().is_some_and(|path| {
        path.to_ascii_lowercase()
            .ends_with(&format!(".{extension}"))
    })
}

fn classify_media(filename: &str, content_type: Option<&str>) -> Option<MediaKind> {
    let content_type = content_type.unwrap_or_default().to_ascii_lowercase();
    if content_type == "image/gif" {
        return Some(MediaKind::Gif);
    }
    if content_type.starts_with("image/") {
        return Some(MediaKind::Image);
    }
    if content_type.starts_with("video/") {
        return Some(MediaKind::Video);
    }
    if content_type.starts_with("audio/") {
        return Some(MediaKind::Audio);
    }

    let extension = filename
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())?;
    if extension == "gif" {
        Some(MediaKind::Gif)
    } else if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Image)
    } else if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Video)
    } else if AUDIO_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Audio)
    } else {
        None
    }
}

fn prepare_tts_text(content: &str, character_limit: u32) -> Option<String> {
    let content = content.trim();
    if content.is_empty() {
        return None;
    }
    if character_limit == 0 {
        return Some(content.to_owned());
    }
    Some(content.chars().take(character_limit as usize).collect())
}

fn prepare_media_text(content: &str) -> Option<String> {
    let printable = content
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let normalized = printable
        .split_whitespace()
        .filter(|segment| !segment.starts_with("https://") && !segment.starts_with("http://"))
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        return None;
    }
    let mut characters = normalized.chars();
    let mut text = characters
        .by_ref()
        .take(MEDIA_TEXT_LIMIT)
        .collect::<String>();
    if characters.next().is_some() {
        text.pop();
        text.push('…');
    }
    Some(text)
}

async fn set_bot_error(core: &Arc<AppCore>, error: String) {
    let mut status = core.bot_status.write().await;
    status.connected = false;
    status.error = Some(error);
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests;
