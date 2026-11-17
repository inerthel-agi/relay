use crate::clock::now_ms;
use std::{
    sync::Arc,
    time::{Duration, Instant},
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
        InputTextStyle, Interaction, Message, MessageId, MessageType, MessageUpdateEvent,
        ModalInteraction, OnlineStatus, PermissionOverwrite, PermissionOverwriteType, Permissions,
        Ready, StickerFormatType, User, UserId,
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
    config
        .is_honeypot_channel(channel_id)
        .then_some(config.honeypot_action)
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

    let config = core.config.read().await.clone();
    let role_ids = message_role_ids(message);
    // Exempt roles (moderators testing the trap) only get their message removed.
    if role_ids
        .iter()
        .any(|role| config.moderation.honeypot_exempt_role_ids.contains(role))
    {
        let _ = message.delete(&context.http).await;
        return;
    }
    let language = core.interface_preferences.read().await.language.clone();
    let dm_delivered = message
        .author
        .direct_message(
            &context.http,
            CreateMessage::new().content(crate::moderation::messages::honeypot_notice(
                action, &language,
            )),
        )
        .await
        .is_ok();
    let action_result = match action {
        HoneypotAction::Kick => guild_id
            .kick_with_reason(&context.http, message.author.id, HONEYPOT_AUDIT_REASON)
            .await
            .map_err(anyhow::Error::from),
        HoneypotAction::Ban => guild_id
            .ban_with_reason(&context.http, message.author.id, 0, HONEYPOT_AUDIT_REASON)
            .await
            .map_err(anyhow::Error::from),
        HoneypotAction::Timeout => timeout_member(
            &context.http,
            guild_id,
            message.author.id,
            config.moderation.honeypot_timeout_minutes,
        )
        .await
        .map(|_| ()),
    };
    if action_result.is_ok() {
        core.record_moderation(
            None,
            match action {
                HoneypotAction::Kick => crate::moderation::log::LogAction::Kicked,
                HoneypotAction::Ban => crate::moderation::log::LogAction::Banned,
                HoneypotAction::Timeout => crate::moderation::log::LogAction::TimedOut,
            },
            "honeypot",
            Some(message.author.id.to_string()),
        );
    }

    let deletion_failed = message.delete(&context.http).await.is_err();
    core.bot_status.write().await.error =
        honeypot_outcome_error(dm_delivered, action_result.is_err(), deletion_failed);
}

/// Discord timeout ("exclusion temporaire"), capped at the 28-day maximum.
pub(crate) async fn timeout_member(
    http: &Http,
    guild_id: serenity::model::id::GuildId,
    user_id: serenity::model::id::UserId,
    minutes: u16,
) -> Result<serenity::model::guild::Member> {
    let seconds = i64::from(minutes.clamp(1, 40_320)) * 60;
    let until = serenity::model::Timestamp::from_unix_timestamp(
        serenity::model::Timestamp::now().unix_timestamp() + seconds,
    )
    .unwrap_or_else(|_| serenity::model::Timestamp::now());
    guild_id
        .edit_member(
            http,
            user_id,
            serenity::builder::EditMember::new()
                .disable_communication_until_datetime(until)
                .audit_log_reason("Relay moderation"),
        )
        .await
        .map_err(Into::into)
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
        errors.push("Kick, ban or timeout failed; check the bot's permissions and role position.");
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
        let staff = is_server_staff(&context.cache, &message);
        if config.is_music_channel(&channel_id) {
            let lane = crate::moderation::Lane::Music;
            if !apply_moderation_gate(&self.core, &message, &role_ids, staff, &[lane], &config) {
                return;
            }
            let scoped_config = crate::moderation::scope_config(
                privacy::scoped_config_for_roles(&config, &role_ids),
                lane,
            );
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

        route_relay_message(
            &self.core,
            &context.http,
            &message,
            &config,
            &role_ids,
            staff,
        )
        .await;
    }

    async fn message_update(
        &self,
        context: Context,
        _old: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        self.relay_message_update(context, new, event).await;
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

mod deferred_embeds;
mod former_channel;
mod moderation_gate;
mod music_handlers;
mod reaction_access;
mod relay_channel;
mod relay_commands;
pub(crate) use deferred_embeds::fresh_for_embeds;
use deferred_embeds::*;
use former_channel::*;
pub(crate) use moderation_gate::report_action_result;
use moderation_gate::*;
use music_handlers::*;
pub(crate) use music_handlers::{refresh_music_card, refresh_pending_music_cards};
pub(crate) use reaction_access::{ReactionAccessOptions, get_reaction_access_options};
use relay_channel::*;
pub(crate) use relay_commands::clear_selected_channel;
pub use relay_commands::sync_relay_command_schema;
use relay_commands::*;

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

const RELAY_CHANNEL_CHOICE_PENDING: &str = "Relay found two former channels, one for media and one for messages. Choose the Relay channel in the Relay application first.";

const MISSING_CHANNEL_WARNING: &str = "A selected Discord channel is private or inaccessible. Add Relay or its role to the channel permissions.";

async fn warn_if_watched_channel_missing(core: &Arc<AppCore>) {
    let configured_channels = {
        let config = core.config.read().await;
        let mut channels = vec![
            config.watched_channel_id.clone(),
            config.former_message_channel_id.clone(),
            config.music_channel_id.clone(),
        ];
        channels.extend(config.extra_relay_channel_ids.iter().cloned());
        channels.extend(config.extra_music_channel_ids.iter().cloned());
        channels
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
    let honeypot = if config.honeypot_channel_id.is_empty() {
        Permissions::empty()
    } else {
        crate::discord_check::required_permissions("honeypot", config)
    };
    let permissions = (Permissions::VIEW_CHANNEL
        | Permissions::READ_MESSAGE_HISTORY
        | Permissions::SEND_MESSAGES
        | Permissions::EMBED_LINKS
        | honeypot
        // Automatic timeouts after repeated blocks (Moderation → sanctions).
        | if config.moderation.escalation_blocks > 0 {
            Permissions::MODERATE_MEMBERS
        } else {
            Permissions::empty()
        }
        | Permissions::MANAGE_CHANNELS
        | Permissions::MANAGE_ROLES
        | Permissions::MANAGE_MESSAGES
        | custom_commands::required_bot_permissions(&config.custom_commands))
    .bits();
    format!(
        "https://discord.com/oauth2/authorize?client_id={client_id}&permissions={permissions}&scope=bot%20applications.commands"
    )
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

async fn set_bot_error(core: &Arc<AppCore>, error: String) {
    let mut status = core.bot_status.write().await;
    status.connected = false;
    status.error = Some(error);
}

#[cfg(test)]
mod tests;
