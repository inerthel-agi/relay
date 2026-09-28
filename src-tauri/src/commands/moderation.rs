//! Moderation page commands: presets, message tester, decision log, held
//! messages, Discord actions from the queue and AutoMod synchronization.

use std::sync::Arc;

use serde::Serialize;
use serenity::all::{
    Action, ChannelId, EditAutoModRule, GuildId, Trigger, UserId, automod::EventType,
};
use tauri::State;

use crate::{
    config::AppConfig,
    model::PendingText,
    moderation::{
        Lane, ModerationPreset, WordPack,
        log::{LogAction, LogEntry, LogSummary},
        presets,
    },
    privacy::{self, PrivacyAction},
    state::AppCore,
};

const AUTOMOD_RULE_NAME: &str = "Relay word filter";
/// Discord limits: 1000 keywords of at most 60 characters.
const AUTOMOD_KEYWORD_LIMIT: usize = 1_000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModerationOverview {
    active_preset: Option<ModerationPreset>,
    summary: LogSummary,
    entries: Vec<LogEntry>,
    pending_texts: Vec<PendingText>,
    packs: Vec<PackInfo>,
    live_streaming: bool,
    raid_active: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInfo {
    id: WordPack,
    words: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    action: &'static str,
    level: privacy::PrivacyClassification,
    reasons: Vec<&'static str>,
    name_hidden: bool,
}

#[tauri::command]
pub async fn moderation_overview(
    core: State<'_, Arc<AppCore>>,
) -> Result<ModerationOverview, String> {
    let config = core.config.read().await.clone();
    let (entries, summary) = core.moderation_entries();
    Ok(ModerationOverview {
        active_preset: presets::matching(&config),
        summary,
        entries,
        pending_texts: core.pending_texts.read().await.iter().cloned().collect(),
        packs: WordPack::ALL
            .iter()
            .map(|pack| PackInfo {
                id: *pack,
                words: pack.words().len(),
            })
            .collect(),
        live_streaming: core
            .moderation
            .lock()
            .is_ok_and(|runtime| runtime.live_streaming),
        raid_active: core
            .moderation
            .lock()
            .is_ok_and(|runtime| runtime.raid_active(crate::clock::now_ms())),
    })
}

#[tauri::command]
pub async fn preview_moderation_preset(
    core: State<'_, Arc<AppCore>>,
    preset: ModerationPreset,
) -> Result<Vec<&'static str>, String> {
    Ok(presets::changed_keys(&*core.config.read().await, preset))
}

#[tauri::command]
pub async fn apply_moderation_preset(
    core: State<'_, Arc<AppCore>>,
    preset: ModerationPreset,
) -> Result<AppConfig, String> {
    core.update_config(|config| presets::apply(config, &presets::fields(preset)))
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Words of one built-in list, only when the streamer asks to see them.
#[tauri::command]
pub fn moderation_pack_words(pack: WordPack) -> Vec<&'static str> {
    pack.words().to_vec()
}

/// Dry run: what the current rules would do with this text. Nothing is published or logged.
#[tauri::command]
pub async fn test_moderation_text(
    core: State<'_, Arc<AppCore>>,
    text: String,
    lane: Option<Lane>,
) -> Result<TestResult, String> {
    if text.chars().count() > privacy::PRIVACY_TEXT_LIMIT {
        return Err("The test text must contain at most 4096 characters.".into());
    }
    let config = core.config.read().await.clone();
    let config = crate::moderation::scope_config(config, lane.unwrap_or(Lane::Media));
    let mut report = privacy::classify_text(Some(&text), &config);
    report.apply_score_policy(&config);
    let action = if privacy::privacy_rules_enabled(&config) {
        privacy::action_for(&report, &config)
    } else {
        PrivacyAction::Allow
    };
    Ok(TestResult {
        action: match action {
            PrivacyAction::Allow => "allow",
            PrivacyAction::Review => "review",
            PrivacyAction::Block => "block",
        },
        level: report.classification,
        reasons: report.reasons,
        name_hidden: config.moderation.filter_usernames
            && privacy::filter_words_match(&text, &config),
    })
}

#[tauri::command]
pub fn clear_moderation_log(core: State<'_, Arc<AppCore>>) {
    core.clear_moderation_log();
}

#[tauri::command]
pub async fn approve_pending_text(core: State<'_, Arc<AppCore>>, id: u64) -> Result<(), String> {
    core.approve_text(id)
        .await
        .then_some(())
        .ok_or_else(|| "This message is no longer waiting for review.".into())
}

#[tauri::command]
pub async fn reject_pending_text(core: State<'_, Arc<AppCore>>, id: u64) -> Result<(), String> {
    core.reject_text(id)
        .await
        .map(|_| ())
        .ok_or_else(|| "This message is no longer waiting for review.".into())
}

async fn discord_http(core: &AppCore) -> Result<Arc<serenity::http::Http>, String> {
    let connected = core.bot_status.read().await.connected;
    core.discord_http()
        .await
        .filter(|_| connected)
        .ok_or_else(|| "Connect the Discord bot first.".into())
}

async fn guild_of(http: &serenity::http::Http, channel_id: &str) -> Result<GuildId, String> {
    let id = channel_id
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| "Choose the media channel first.".to_string())?;
    http.get_channel(ChannelId::new(id))
        .await
        .ok()
        .and_then(|channel| channel.guild())
        .map(|channel| channel.guild_id)
        .ok_or_else(|| "Relay cannot see this Discord channel.".into())
}

/// Rejects a pending item and applies a Discord action to its message or author.
/// `action`: "delete", "timeout", "ban" or "trust" (approve and trust the author).
#[tauri::command]
pub async fn moderate_pending(
    core: State<'_, Arc<AppCore>>,
    id: u64,
    action: String,
) -> Result<(), String> {
    let (author_id, channel_id, message_id) = core
        .pending_origin(id)
        .await
        .ok_or_else(|| "This item is no longer waiting for review.".to_string())?;
    if action == "trust" {
        let author_id =
            author_id.ok_or_else(|| "The author of this item is unknown.".to_string())?;
        core.update_config(|config| {
            let trusted = &mut config.moderation.trusted_user_ids;
            if !trusted.contains(&author_id)
                && trusted.len() < crate::moderation::MAX_MODERATION_IDS
            {
                trusted.push(author_id.clone());
            }
        })
        .await
        .map_err(|error| format!("{error:#}"))?;
        let approved = core.approve_media(id).await || core.approve_text(id).await;
        return approved
            .then_some(())
            .ok_or_else(|| "This item is no longer waiting for review.".into());
    }
    let http = discord_http(&core).await?;
    let channel_id =
        channel_id.ok_or_else(|| "The Discord channel of this item is unknown.".to_string())?;
    let parse = |value: &str| value.parse::<u64>().ok().filter(|id| *id > 0);
    let is_text = core
        .pending_texts
        .read()
        .await
        .iter()
        .any(|item| item.id == id);
    let lane = if is_text {
        Lane::Notifications
    } else {
        Lane::Media
    };
    let author = author_id.as_deref().and_then(parse).map(UserId::new);
    let log_author = author_id.clone();
    match action.as_str() {
        "delete" => {
            let (Some(channel), Some(message)) = (parse(&channel_id), parse(&message_id)) else {
                return Err("This Discord message cannot be found.".into());
            };
            ChannelId::new(channel)
                .delete_message(&http, message)
                .await
                .map_err(|_| {
                    "Deletion failed. Verify Manage Messages in this channel.".to_string()
                })?;
            core.record_moderation(
                Some(lane),
                LogAction::MessageDeleted,
                "manual_review",
                log_author,
            );
        }
        "timeout" | "ban" => {
            let author = author.ok_or_else(|| "The author of this item is unknown.".to_string())?;
            let guild = guild_of(&http, &channel_id).await?;
            if action == "ban" {
                guild
                    .ban_with_reason(&http, author, 0, "Relay moderation")
                    .await
                    .map_err(|_| {
                        "Ban failed. Verify Ban Members and the bot's role position.".to_string()
                    })?;
                core.record_moderation(Some(lane), LogAction::Banned, "manual_review", log_author);
            } else {
                let minutes = core
                    .config
                    .read()
                    .await
                    .moderation
                    .escalation_timeout_minutes;
                crate::bot::timeout_member(&http, guild, author, minutes)
                    .await
                    .map_err(|_| {
                        "Timeout failed. Verify Moderate Members and the bot's role position."
                            .to_string()
                    })?;
                core.record_moderation(
                    Some(lane),
                    LogAction::TimedOut,
                    "manual_review",
                    log_author,
                );
            }
        }
        _ => return Err("Unknown moderation action.".into()),
    }
    // The Discord action succeeded: remove the item from the queue.
    if is_text {
        core.reject_text(id).await;
    } else {
        core.reject_media(id).await;
    }
    Ok(())
}

/// Creates or updates one AutoMod keyword rule from the enabled lists and custom words.
#[tauri::command]
pub async fn sync_discord_automod(core: State<'_, Arc<AppCore>>) -> Result<usize, String> {
    let config = core.config.read().await.clone();
    let http = discord_http(&core).await?;
    let guild = guild_of(&http, &config.watched_channel_id).await?;
    let keywords = automod_keywords(&config);
    if keywords.is_empty() {
        return Err("Add filter words or turn on a word list first.".into());
    }
    let rules = guild.automod_rules(&http).await.map_err(|_| {
        "Discord refused. Relay needs the Manage Server permission for AutoMod.".to_string()
    })?;
    let builder = EditAutoModRule::new()
        .name(AUTOMOD_RULE_NAME)
        .event_type(EventType::MessageSend)
        .trigger(Trigger::Keyword {
            strings: keywords.clone(),
            regex_patterns: Vec::new(),
            allow_list: config.privacy_allowlist.iter().take(100).cloned().collect(),
        })
        .actions(vec![Action::BlockMessage {
            custom_message: Some("Blocked by the server's word filter.".into()),
        }])
        .enabled(true);
    let result = match rules.iter().find(|rule| rule.name == AUTOMOD_RULE_NAME) {
        Some(rule) => guild
            .edit_automod_rule(&http, rule.id, builder)
            .await
            .map(|_| ()),
        None => guild.create_automod_rule(&http, builder).await.map(|_| ()),
    };
    result.map_err(|_| {
        "Discord refused. Relay needs the Manage Server permission for AutoMod.".to_string()
    })?;
    Ok(keywords.len())
}

#[tauri::command]
pub async fn remove_discord_automod(core: State<'_, Arc<AppCore>>) -> Result<bool, String> {
    let config = core.config.read().await.clone();
    let http = discord_http(&core).await?;
    let guild = guild_of(&http, &config.watched_channel_id).await?;
    let rules = guild.automod_rules(&http).await.map_err(|_| {
        "Discord refused. Relay needs the Manage Server permission for AutoMod.".to_string()
    })?;
    let Some(rule) = rules.iter().find(|rule| rule.name == AUTOMOD_RULE_NAME) else {
        return Ok(false);
    };
    guild
        .delete_automod_rule(&http, rule.id)
        .await
        .map_err(|_| "Discord refused to remove the AutoMod rule.".to_string())?;
    Ok(true)
}

/// Canonical words and aliases as Discord keywords (at most 60 characters each).
pub(crate) fn automod_keywords(config: &AppConfig) -> Vec<String> {
    let mut keywords = Vec::new();
    for concept in crate::moderation::effective_concepts(config).iter() {
        for word in std::iter::once(&concept.canonical).chain(concept.aliases.iter()) {
            let word = word.trim().to_lowercase();
            if !word.is_empty() && word.chars().count() <= 60 && !keywords.contains(&word) {
                keywords.push(word);
            }
        }
    }
    keywords.truncate(AUTOMOD_KEYWORD_LIMIT);
    keywords
}
