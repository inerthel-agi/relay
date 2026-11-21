use super::*;

fn bounded_privacy_text(value: &str) -> String {
    let value = value.trim();
    value
        .char_indices()
        .nth(privacy::PRIVACY_TEXT_LIMIT)
        .map_or_else(|| value.to_owned(), |(index, _)| value[..index].to_owned())
}

pub(super) fn message_role_ids(message: &Message) -> Vec<String> {
    message
        .member
        .as_ref()
        .map(|member| member.roles.iter().map(ToString::to_string).collect())
        .unwrap_or_default()
}

pub(super) fn classify_message_privacy(
    message: &Message,
    config: &AppConfig,
) -> privacy::PrivacyReport {
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

pub(super) fn classify_privacy_values<'a>(
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

pub(super) fn attachment_privacy_text(content: &str, filename: &str) -> String {
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

/// Re-read after media I/O so a policy change during a scan is observed.
pub(super) async fn current_lane_config(
    core: &AppCore,
    roles: &[String],
    lane: crate::moderation::Lane,
) -> AppConfig {
    let config = core.config.read().await;
    crate::moderation::scope_config(privacy::scoped_config_for_roles(&config, roles), lane)
}

type MediaCandidate<'a> = (MediaKind, &'a str, &'a str, Option<&'a [u8]>);

pub(super) async fn check_media_part(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    staff: bool,
    current: &AppConfig,
    (text, report): (&str, privacy::PrivacyReport),
    candidate: Option<MediaCandidate<'_>>,
) -> Option<privacy::PrivacyReport> {
    let mut report = reclassify_privacy_report(report, text, current);
    if block_and_delete_message_if_needed(core, http, message, &report, current).await {
        return None;
    }
    if let Some(media) = candidate
        && (voice_check(core, http, message, media, current, &mut report).await
            || ai_image_check(core, http, message, staff, media, &mut report).await)
    {
        return None;
    }
    Some(report)
}

pub(super) fn reclassify_privacy_report(
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

pub(super) fn privacy_action_is_blocked(
    report: &privacy::PrivacyReport,
    config: &AppConfig,
) -> bool {
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

pub(super) fn should_auto_delete_blocked_message(
    report: &privacy::PrivacyReport,
    config: &AppConfig,
) -> bool {
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

pub(super) async fn block_and_delete_message_if_needed(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    report: &privacy::PrivacyReport,
    config: &AppConfig,
) -> bool {
    if !privacy_action_is_blocked(report, config) {
        return false;
    }
    if should_auto_delete_blocked_message(report, config) {
        let failed = message
            .channel_id
            .delete_message(http, message.id)
            .await
            .is_err();
        report_action_result(core, failed, PRIVACY_DELETION_FAILED).await;
    }
    follow_up_block(core, http, message, report, config).await;
    true
}

pub(super) const PRIVACY_DELETION_FAILED: &str =
    "Privacy deletion failed. Verify Manage Messages in this channel.";
pub(super) const AI_IMAGE_DELETION_FAILED: &str =
    "AI image deletion failed. Verify Manage Messages in this channel.";
pub(super) const AUTOMATIC_TIMEOUT_FAILED: &str =
    "Automatic timeout failed. Verify Moderate Members and the bot's role position.";

/// Shows a failed Discord action in the panel until the same action succeeds
/// again. Any other error message is left alone.
pub(crate) async fn report_action_result(core: &AppCore, failed: bool, error: &'static str) {
    let mut status = core.bot_status.write().await;
    if failed {
        status.error = Some(error.into());
    } else if status.error.as_deref() == Some(error) {
        status.error = None;
    }
}

/// AI image check of one media of a member, after the local scan. Returns
/// true when it blocked the whole message. A hold applies to the parts of the
/// message not yet submitted; a relay that cleared an unfinished local scan
/// settles `report`.
pub(super) async fn ai_image_check(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    staff: bool,
    candidate: MediaCandidate<'_>,
    report: &mut privacy::PrivacyReport,
) -> bool {
    use crate::ai::image_guard::{self, Candidate, Outcome};
    let message_id = message.id.to_string();
    let trusted = core
        .moderation_verdict(&message_id)
        .is_some_and(|verdict| verdict.trusted);
    if staff || trusted {
        return false;
    }
    let (kind, url, proxy_url, bytes) = candidate;
    let config = {
        let config = core.config.read().await;
        if !image_guard::active(&config.ai) {
            return false;
        }
        let mut config = config.clone();
        // A public library GIF cannot show the poster's personal data; it is
        // still checked for harmful content.
        if is_gif_library_url(url) {
            config.ai.personal_data_relay = false;
        }
        config
    };
    let language = core.interface_preferences.read().await.language.clone();
    let member = message.author.id.to_string();
    let outcome = image_guard::check(
        &config,
        Candidate {
            kind,
            url,
            proxy_url,
            bytes,
            report,
            member: &member,
        },
        &language,
    )
    .await;
    match outcome {
        Outcome::Skip | Outcome::Clear => false,
        Outcome::Release => {
            *report = privacy::PrivacyReport {
                config_signature: report.config_signature,
                ..privacy::PrivacyReport::safe()
            };
            false
        }
        Outcome::Hold(reason) => {
            core.hold_message_for_review(&message_id, reason);
            false
        }
        Outcome::Block(reason) => {
            if config.ai.image_guard_delete_message {
                let failed = message
                    .channel_id
                    .delete_message(http, message.id)
                    .await
                    .is_err();
                report_action_result(core, failed, AI_IMAGE_DELETION_FAILED).await;
            }
            let report = privacy::PrivacyReport::suspicious(reason);
            follow_up_block(core, http, message, &report, &config).await;
            true
        }
    }
}

/// Voice check of an audio file or a video's sound, on this PC. Returns true
/// when it blocked the whole message; a sound that cannot be checked holds the
/// message for review. Moderators are checked too: it protects personal data.
/// With the AI watch on, the transcript of a member's sound gets a second
/// reading by the chosen provider, which can only hold it.
pub(super) async fn voice_check(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    candidate: MediaCandidate<'_>,
    config: &AppConfig,
    report: &mut privacy::PrivacyReport,
) -> bool {
    use crate::voice_scan::{self, VoiceCheck};
    let (kind, url, proxy_url, bytes) = candidate;
    if !voice_scan::applies(config, kind) {
        return false;
    }
    let directory = voice_scan::model_directory(&core.data_directory);
    let source = (url, proxy_url, bytes.map(<[u8]>::to_vec));
    match voice_scan::check(directory, source, config).await {
        VoiceCheck::Silent => false,
        VoiceCheck::Unchecked => {
            core.hold_message_for_review(&message.id.to_string(), voice_scan::HOLD_UNCHECKED);
            false
        }
        VoiceCheck::Checked {
            report: voice,
            transcript,
        } => {
            report.merge(voice);
            report.apply_score_policy(config);
            if block_and_delete_message_if_needed(core, http, message, report, config).await {
                return true;
            }
            let message_id = message.id.to_string();
            if core
                .ai_holds_transcript(&message_id, &transcript, config)
                .await
            {
                core.hold_message_for_review(&message_id, crate::ai::watch::HOLD_REASON);
            }
            false
        }
    }
}

/// Logs a block, then warns or times out the author when those options are on.
async fn follow_up_block(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    report: &privacy::PrivacyReport,
    config: &AppConfig,
) {
    use crate::moderation::log::LogAction;
    let lane = decision_log_lane(config, message);
    let author_id = message.author.id.to_string();
    core.record_moderation(
        lane,
        LogAction::Blocked,
        report.primary_reason().unwrap_or("privacy"),
        Some(author_id.clone()),
    );
    let follow_up = match core.moderation.lock() {
        Ok(mut runtime) => {
            runtime.record_block(message.author.id.get(), &config.moderation, now_ms())
        }
        Err(_) => return,
    };
    if follow_up.warn {
        let language = core.interface_preferences.read().await.language.clone();
        if message
            .author
            .direct_message(
                http,
                CreateMessage::new().content(crate::moderation::messages::block_warning(&language)),
            )
            .await
            .is_ok()
        {
            core.record_moderation(
                lane,
                LogAction::Warned,
                "block_warning",
                Some(author_id.clone()),
            );
        }
    }
    if let (Some(minutes), Some(guild_id)) = (follow_up.timeout_minutes, message.guild_id) {
        let result = timeout_member(http, guild_id, message.author.id, minutes).await;
        if result.is_ok() {
            core.record_moderation(lane, LogAction::TimedOut, "escalation", Some(author_id));
        }
        report_action_result(core, result.is_err(), AUTOMATIC_TIMEOUT_FAILED).await;
    }
}

/// Lane recorded in the decision log for a message of this channel.
pub(super) fn decision_log_lane(
    config: &AppConfig,
    message: &Message,
) -> Option<crate::moderation::Lane> {
    use crate::moderation::Lane;
    let channel_id = message.channel_id.to_string();
    if config.is_music_channel(&channel_id) {
        return Some(Lane::Music);
    }
    let feeds = config.channel_feeds(&channel_id)?;
    let has_media = !message.attachments.is_empty()
        || !message.sticker_items.is_empty()
        || !message.embeds.is_empty();
    Some(if feeds.media && (has_media || !feeds.notifications) {
        Lane::Media
    } else {
        Lane::Notifications
    })
}

/// Permissions that let a member moderate the server. Such members skip the
/// spam, cooldown and review checks like trusted members. Privacy blocks still
/// apply, but a privacy review or hold never sends their messages to manual review.
const STAFF_PERMISSIONS: Permissions = Permissions::ADMINISTRATOR
    .union(Permissions::MANAGE_GUILD)
    .union(Permissions::MANAGE_MESSAGES);

/// Server owner, or member whose roles grant a staff permission (from the cache).
pub(super) fn is_server_staff(cache: &Cache, message: &Message) -> bool {
    let (Some(guild_id), Some(member)) = (message.guild_id, message.member.as_ref()) else {
        return false;
    };
    let Some(guild) = cache.guild(guild_id) else {
        return false;
    };
    let everyone = serenity::all::RoleId::new(guild_id.get());
    let permissions = std::iter::once(&everyone)
        .chain(member.roles.iter())
        .filter_map(|role| guild.roles.get(role))
        .fold(Permissions::empty(), |granted, role| {
            granted | role.permissions
        });
    staff_member(guild.owner_id == message.author.id, permissions)
}

pub(super) fn staff_member(owner: bool, permissions: Permissions) -> bool {
    owner || permissions.intersects(STAFF_PERMISSIONS)
}

/// Runs the spam, trust and account checks of every lane the message feeds.
/// Returns false when the message is ignored.
pub(super) fn apply_moderation_gate(
    core: &Arc<AppCore>,
    message: &Message,
    role_ids: &[String],
    staff: bool,
    lanes: &[crate::moderation::Lane],
    config: &AppConfig,
) -> bool {
    use crate::moderation::{GateDecision, GateInput, Lane, MessageVerdict};
    let message_id = message.id.to_string();
    let mut media_keys = message
        .attachments
        .iter()
        .map(|attachment| {
            format!(
                "a:{}:{}",
                attachment.filename.to_lowercase(),
                attachment.size
            )
        })
        .collect::<Vec<_>>();
    media_keys.extend(
        message
            .sticker_items
            .iter()
            .map(|sticker| format!("s:{}", sticker.id)),
    );
    // Only the embeds shown as a GIF: a link preview is not media.
    media_keys.extend(
        message
            .embeds
            .iter()
            .filter(|embed| embedded_gif(embed).is_some())
            .filter_map(|embed| embed.url.as_ref().map(|url| format!("e:{url}"))),
    );
    let mention_count = message.mentions.len()
        + message.mention_roles.len()
        + if message.mention_everyone { 10 } else { 0 };
    let input = GateInput {
        lane: lanes.first().copied().unwrap_or(Lane::Media),
        user_id: message.author.id.get(),
        role_ids,
        staff,
        member_joined_ms: message
            .member
            .as_ref()
            .and_then(|member| member.joined_at)
            .map(|joined| joined.unix_timestamp().max(0) as u64 * 1_000),
        mention_count,
        text: &message.content,
        media_keys,
        now_ms: now_ms(),
    };
    #[cfg(test)]
    tests::wait_for_other_gate();
    let mut runtime = core.moderation_runtime();
    if let Some(verdict) = runtime.verdict(&message_id) {
        return !verdict.rejected
            && !config
                .moderation
                .blocked_user_ids
                .contains(&message.author.id.to_string());
    }
    let (decision, lane) = runtime.evaluate_lanes(&input, lanes, &config.moderation);
    let mut verdict = MessageVerdict {
        author_id: message.author.id.to_string(),
        channel_id: message.channel_id.to_string(),
        ..Default::default()
    };
    match decision {
        GateDecision::Drop(_) => verdict.rejected = true,
        GateDecision::Pass { trusted } => verdict.trusted = trusted,
        GateDecision::Hold(reason) if !lanes.contains(&Lane::Music) => verdict.hold = Some(reason),
        GateDecision::Hold(_) => {}
    }
    runtime.remember_verdict(&message_id, verdict, input.now_ms);
    drop(runtime);
    if let GateDecision::Drop(reason) = decision {
        core.record_moderation(
            Some(lane),
            crate::moderation::log::LogAction::Ignored,
            reason,
            Some(message.author.id.to_string()),
        );
        if reason == "cooldown" {
            spawn_cooldown_notice(core, message, config);
        }
        return false;
    }
    true
}

/// How long the cooldown notice stays in the channel.
const COOLDOWN_NOTICE_LIFETIME: Duration = Duration::from_secs(10);
/// Older messages are re-checks (an edit after the verdict expired), not new sends.
const COOLDOWN_NOTICE_FRESH_MS: u64 = 30_000;

/// Tells a member that a message sent too soon was not shown. Discord has no
/// ephemeral reply to a plain message, so the notice deletes itself.
fn spawn_cooldown_notice(core: &Arc<AppCore>, message: &Message, config: &AppConfig) {
    let settings = &config.moderation;
    let author = message.author.id;
    // Only a message the member just typed: not a system message (boost, pin,
    // thread), and not an old message checked again because it was edited.
    let typed = matches!(
        message.kind,
        MessageType::Regular | MessageType::InlineReply
    ) && now_ms().saturating_sub(message_timestamp(message)) < COOLDOWN_NOTICE_FRESH_MS;
    if !settings.cooldown_notice
        || !typed
        || !core
            .moderation
            .lock()
            .is_ok_and(|mut runtime| runtime.cooldown_notice_due(author.get(), now_ms()))
    {
        return;
    }
    let core = core.clone();
    let channel_id = message.channel_id;
    let seconds = settings.user_cooldown_seconds;
    tauri::async_runtime::spawn(async move {
        let Some(http) = core.discord_http().await else {
            return;
        };
        let language = core.interface_preferences.read().await.language.clone();
        let notice = CreateMessage::new()
            .content(format!(
                "<@{author}> {}",
                crate::moderation::messages::cooldown_notice(&language, seconds)
            ))
            .allowed_mentions(CreateAllowedMentions::new().users([author]));
        let Ok(sent) = channel_id.send_message(&*http, notice).await else {
            return;
        };
        tokio::time::sleep(COOLDOWN_NOTICE_LIFETIME).await;
        let _ = channel_id.delete_message(&*http, sent.id).await;
    });
}
