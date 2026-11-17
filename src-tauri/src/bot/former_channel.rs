use super::*;

/// A former message channel kept until the user chooses the Relay channel:
/// its messages stay notifications only, as before 1.4.1.
pub(super) async fn former_message_channel(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    config: &AppConfig,
    role_ids: &[String],
) {
    let scoped_config = crate::moderation::scope_config(
        privacy::scoped_config_for_roles(config, role_ids),
        crate::moderation::Lane::Notifications,
    );
    let text_report = classify_message_privacy(message, &scoped_config);
    if block_and_delete_message_if_needed(core, http, message, &text_report, &scoped_config).await {
        return;
    }
    // Staff and trusted members never wait for review: only a block stops them.
    let trusted =
        crate::state::skips_review(core.moderation_verdict(&message.id.to_string()).as_ref());
    if !trusted
        && privacy::privacy_rules_enabled(&scoped_config)
        && !scoped_config.moderation.review_text
    {
        let action = privacy::action_for(&text_report, &scoped_config);
        if matches!(action, privacy::PrivacyAction::Review) {
            privacy::log_decision(&text_report, action);
            core.record_moderation(
                Some(crate::moderation::Lane::Notifications),
                crate::moderation::log::LogAction::Ignored,
                text_report.primary_reason().unwrap_or("privacy"),
                Some(message.author.id.to_string()),
            );
            return;
        }
    }
    // Same text as a Relay channel notification: no system message, no typed
    // command, no GIF link.
    let text = notification_text(message).unwrap_or_default();
    let stage_ticket = if !text.is_empty() || !message.sticker_items.is_empty() {
        Some(
            core.register_stage_output(
                message_timestamp(message),
                &message.id.to_string(),
                0,
                StageLane::Notification,
            )
            .await,
        )
    } else {
        None
    };
    let mut sticker_segments = Vec::new();
    for sticker in message.sticker_items.iter().take(3) {
        let Some(url) = sticker.image_url() else {
            continue;
        };
        let Some(segment) = former_sticker_segment(
            core,
            http,
            message,
            role_ids,
            &scoped_config,
            (sticker, url),
        )
        .await
        else {
            if let Some(ticket) = stage_ticket {
                core.cancel_stage_output(ticket).await;
            }
            return;
        };
        sticker_segments.push(segment);
    }
    if let Some(ticket) = stage_ticket {
        publish_notification(
            core,
            message,
            config,
            role_ids,
            ticket,
            &text,
            sticker_segments,
        )
        .await;
    }
}

pub(super) async fn former_sticker_segment(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    role_ids: &[String],
    scoped_config: &AppConfig,
    (sticker, url): (&serenity::all::StickerItem, String),
) -> Option<VisualSegment> {
    let (format, _) = sticker_format(sticker.format_type);
    let sticker_text = format!("{}\n{}", message.content, sticker.name);
    let (report, bytes) = inspect_sticker(&url, format, Some(&sticker_text), scoped_config).await;
    let current = current_lane_config(core, role_ids, crate::moderation::Lane::Notifications).await;
    let trusted =
        crate::state::skips_review(core.moderation_verdict(&message.id.to_string()).as_ref());
    let image = (format != "lottie").then_some((
        MediaKind::Image,
        url.as_str(),
        url.as_str(),
        bytes.as_deref(),
    ));
    let report = check_media_part(
        core,
        http,
        message,
        trusted,
        &current,
        (&sticker_text, report),
        image,
    )
    .await?;
    if !trusted && privacy::privacy_rules_enabled(&current) {
        let action = privacy::action_for(&report, &current);
        if matches!(action, privacy::PrivacyAction::Review) {
            privacy::log_decision(&report, action);
            return None;
        }
    }
    Some(sticker_visual_segment(
        sticker.name.clone(),
        Some(url),
        sticker.format_type,
    ))
}
