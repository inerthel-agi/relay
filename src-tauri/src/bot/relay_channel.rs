use super::*;

/// Names shown on stream go through the word filter ("Anonymous" when they match).
fn message_author(message: &Message, config: &AppConfig) -> AuthorIdentity {
    crate::moderation::anonymous_author(
        AuthorIdentity {
            username: message.author.name.clone(),
            display_avatar_url: message
                .author
                .avatar_url()
                .unwrap_or_else(|| message.author.default_avatar_url()),
        },
        config,
    )
}

/// Handles a message of the Relay channel, or of a former channel that waits
/// for the user's choice. Messages of any other channel are ignored.
pub(super) async fn route_relay_message(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    config: &AppConfig,
    role_ids: &[String],
    staff: bool,
) {
    let Some(feeds) = config.channel_feeds(&message.channel_id.to_string()) else {
        return;
    };
    let plan = RelayMessagePlan::new(message, feeds);
    if !apply_moderation_gate(core, message, role_ids, staff, &plan.lanes, config) {
        return;
    }
    if feeds.media {
        let _observation = if has_gif_link(&message.content)
            || message
                .embeds
                .iter()
                .any(|embed| embedded_gif(embed).is_some())
        {
            Some(core.track_gif_preview(message.id).await)
        } else {
            None
        };
        relay_channel_message(core, http, message, config, role_ids, staff, &plan).await;
    } else {
        former_message_channel(core, http, message, config, role_ids).await;
    }
}

/// What one message of a Relay channel feeds, decided before any download or scan.
#[derive(Debug, Default)]
pub(super) struct RelayMessagePlan {
    /// Moderation lanes the message is checked against, the notification first.
    pub(super) lanes: Vec<crate::moderation::Lane>,
    /// Notification text. `None` for media-only messages, commands, system
    /// messages, messages holding only GIF links and former media channels.
    pub(super) notification_text: Option<String>,
    /// Supported attachments as (index in the message, kind): Discord order, at most three.
    pub(super) attachments: Vec<(usize, MediaKind)>,
}

impl RelayMessagePlan {
    pub(super) fn new(message: &Message, feeds: crate::config::ChannelFeeds) -> Self {
        use crate::moderation::Lane;
        if !feeds.media {
            return Self {
                lanes: vec![Lane::Notifications],
                ..Self::default()
            };
        }
        let notification_text = if feeds.notifications {
            notification_text(message)
        } else {
            None
        };
        let attachments = message
            .attachments
            .iter()
            .enumerate()
            .filter_map(|(index, attachment)| {
                classify_attachment(attachment).map(|kind| (index, kind))
            })
            .take(3)
            .collect();
        // Links can still bring a GIF embed with a later update.
        let may_carry_media = !message.attachments.is_empty()
            || !message.sticker_items.is_empty()
            || !message.embeds.is_empty()
            || message.content.contains("https://")
            || message.content.contains("http://");
        let mut lanes = Vec::new();
        if notification_text.is_some() {
            lanes.push(Lane::Notifications);
        }
        if may_carry_media || notification_text.is_none() {
            lanes.push(Lane::Media);
        }
        Self {
            lanes,
            notification_text,
            attachments,
        }
    }
}

/// Notification text of a Relay channel message: regular messages and replies
/// only, never a command, and without the GIF links relayed as media.
pub(super) fn notification_text(message: &Message) -> Option<String> {
    if !matches!(
        message.kind,
        MessageType::Regular | MessageType::InlineReply
    ) {
        return None;
    }
    let text = without_gif_links(&message.content, &message.embeds);
    let text = text.trim();
    (!text.is_empty() && !looks_like_command(text)).then(|| text.to_owned())
}

/// A slash command typed as text, such as `/relay status`.
fn looks_like_command(text: &str) -> bool {
    let Some(name) = text
        .split_whitespace()
        .next()
        .and_then(|token| token.strip_prefix('/'))
    else {
        return false;
    };
    name.chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

/// Relays one Relay channel message: the text as a notification, then each
/// supported sticker, attachment and GIF once, in Discord order. The
/// notification is released after every media check, so a blocked part keeps
/// the whole message off stream.
async fn relay_channel_message(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    config: &AppConfig,
    role_ids: &[String],
    staff: bool,
    plan: &RelayMessagePlan,
) {
    let role_config = privacy::scoped_config_for_roles(config, role_ids);
    // The message text is checked with the rules of every output it feeds.
    let message_config =
        crate::moderation::scope_config_for_lanes(role_config.clone(), &plan.lanes);
    let message_report = classify_message_privacy(message, &message_config);
    if block_and_delete_message_if_needed(core, http, message, &message_report, &message_config)
        .await
    {
        return;
    }
    let scoped_config =
        crate::moderation::scope_config(role_config, crate::moderation::Lane::Media);
    // The notification already shows the text: media cards do not repeat it.
    let media_text = if plan.notification_text.is_some() {
        None
    } else {
        prepare_media_text(&message.content)
    };
    let timestamp = message_timestamp(message);
    let message_id = message.id.to_string();
    let notification_ticket = match plan.notification_text {
        Some(_) => Some(
            core.register_stage_output(timestamp, &message_id, 0, StageLane::Notification)
                .await,
        ),
        None => None,
    };
    let mut stage_tickets = notification_ticket.into_iter().collect::<Vec<_>>();
    let mut sticker_tickets = Vec::new();
    for (index, sticker) in message.sticker_items.iter().take(3).enumerate() {
        let Some(url) = sticker.image_url() else {
            continue;
        };
        let ticket = core
            .register_stage_output(timestamp, &message_id, 100 + index as u16, StageLane::Media)
            .await;
        stage_tickets.push(ticket);
        sticker_tickets.push((sticker, url, ticket));
    }
    let mut attachment_tickets = Vec::new();
    for index in 0..plan.attachments.len() {
        let ticket = core
            .register_stage_output(timestamp, &message_id, 200 + index as u16, StageLane::Media)
            .await;
        stage_tickets.push(ticket);
        attachment_tickets.push(ticket);
    }
    // Only a message that downloads media takes one of the shared slots: a
    // plain text never waits behind slow media checks. Its stage places are
    // reserved first, so a later message never shows before it.
    let carries_media = !plan.attachments.is_empty()
        || !message.sticker_items.is_empty()
        || !message_gifs(&message.content, &message.embeds).is_empty();
    let mut _media_slot = match carries_media {
        true => match core.media_ingest_slots.acquire().await {
            Ok(slot) => Some(slot),
            Err(_) => {
                cancel_stage_tickets(core, &stage_tickets).await;
                return;
            }
        },
        false => None,
    };

    for (sticker, url, stage_ticket) in sticker_tickets {
        let (format, _) = sticker_format(sticker.format_type);
        let sticker_text = format!("{}\n{}", message.content, sticker.name);
        let (privacy_report, bytes) =
            inspect_sticker(&url, format, Some(&sticker_text), &scoped_config).await;
        let current = current_lane_config(core, role_ids, crate::moderation::Lane::Media).await;
        let image = (format != "lottie").then_some((
            MediaKind::Image,
            url.as_str(),
            url.as_str(),
            bytes.as_deref(),
        ));
        let Some(privacy_report) = check_media_part(
            core,
            http,
            message,
            staff,
            &current,
            (&sticker_text, privacy_report),
            image,
        )
        .await
        else {
            cancel_stage_tickets(core, &stage_tickets).await;
            return;
        };
        core.submit_sticker_with_ticket_and_roles(
            stage_ticket,
            StickerEvent {
                id: sticker.id.to_string(),
                name: sticker.name.clone(),
                format: format.into(),
                url,
                cached_media_id: None,
                author: message_author(message, config),
                timestamp: message_timestamp(message),
                message_id: message.id.to_string(),
            },
            Some(&sticker_text),
            bytes,
            Some(privacy_report),
            role_ids,
        )
        .await;
    }

    for (index, (attachment_index, kind)) in plan.attachments.iter().copied().enumerate() {
        let attachment = &message.attachments[attachment_index];
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
            author: message_author(message, config),
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
        let current = current_lane_config(core, role_ids, crate::moderation::Lane::Media).await;
        let media = (
            kind,
            event.url.as_str(),
            event.proxy_url.as_str(),
            audio_metadata.as_ref().map(|m| m.audio.as_slice()),
        );
        let Some(privacy_report) = check_media_part(
            core,
            http,
            message,
            staff,
            &current,
            (&attachment_text, privacy_report),
            Some(media),
        )
        .await
        else {
            cancel_stage_tickets(core, &stage_tickets).await;
            return;
        };
        if let Some(embedded) = audio_metadata
            .as_mut()
            .and_then(|metadata| metadata.artwork.take())
        {
            let id = attachment.id.to_string();
            core.cache_artwork(id.clone(), embedded).await;
            event.artwork_id = Some(id);
        }
        if let Some(metadata) = audio_metadata.as_mut() {
            let id = attachment.id.to_string();
            let content_type = attachment
                .content_type
                .clone()
                .unwrap_or_else(|| "application/octet-stream".into());
            core.cache_audio(
                id.clone(),
                content_type,
                std::mem::take(&mut metadata.audio),
            )
            .await;
            event.audio_id = Some(id);
        }
        // An HEVC video can be transcoded for up to two minutes: other messages
        // use the slot meanwhile, and the stage places keep the Discord order.
        let transcodable =
            crate::media_compat::is_mp4_candidate(&event.filename, &event.content_type);
        if transcodable {
            drop(_media_slot.take());
        }
        core.submit_analyzed_media_with_ticket_and_roles(
            stage_ticket,
            event,
            Some(privacy_report),
            Some(&message.content),
            role_ids,
        )
        .await;
        let media_follows = index + 1 < plan.attachments.len()
            || !message_gifs(&message.content, &message.embeds).is_empty();
        if transcodable && media_follows {
            // Taken again only after the release: never two slots for one message.
            match core.media_ingest_slots.acquire().await {
                Ok(slot) => _media_slot = Some(slot),
                Err(_) => {
                    cancel_stage_tickets(core, &stage_tickets).await;
                    return;
                }
            }
        }
    }

    if submit_embedded_gifs(core, http, message, staff).await {
        cancel_stage_tickets(core, &stage_tickets).await;
        return;
    }
    if let (Some(text), Some(ticket)) = (plan.notification_text.as_deref(), notification_ticket) {
        publish_notification(core, message, config, role_ids, ticket, text, Vec::new()).await;
    }
}

pub(super) async fn publish_notification(
    core: &AppCore,
    message: &Message,
    config: &AppConfig,
    role_ids: &[String],
    ticket: StageTicket,
    text: &str,
    stickers: Vec<VisualSegment>,
) {
    let text =
        prepare_notification_text(text, config.notification_character_limit).unwrap_or_default();
    let segments = if text.is_empty() {
        if stickers.is_empty() {
            core.cancel_stage_output(ticket).await;
            return;
        }
        stickers
    } else {
        let mut segments =
            parse_visual_segments(&text).unwrap_or_else(|| plain_text_segments(text.clone()));
        segments.extend(stickers);
        segments
    };
    core.publish_notification_card_if_allowed_with_ticket_and_roles(
        ticket,
        message.id.to_string(),
        text,
        message_author(message, config),
        guild_tag_from_user(&message.author),
        message_timestamp(message),
        segments,
        role_ids,
    )
    .await;
}

pub(super) fn guild_tag_from_user(user: &User) -> Option<GuildTagIdentity> {
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

pub(super) fn message_timestamp(message: &Message) -> u64 {
    message.timestamp.unix_timestamp().max(0) as u64 * 1_000
}

async fn cancel_stage_tickets(core: &AppCore, tickets: &[StageTicket]) {
    core.cancel_stage_outputs(tickets).await;
}

pub(super) fn sticker_format(format: StickerFormatType) -> (&'static str, &'static str) {
    match format {
        StickerFormatType::Png => ("png", "image/png"),
        StickerFormatType::Apng => ("apng", "image/png"),
        StickerFormatType::Lottie => ("lottie", "application/json"),
        StickerFormatType::Gif => ("gif", "image/gif"),
        StickerFormatType::Unknown(_) => ("unknown", "application/octet-stream"),
        _ => ("unknown", "application/octet-stream"),
    }
}

pub(super) async fn inspect_sticker(
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
    // Outputs never draw a Lottie sticker (they show the Relay logo instead),
    // so its name and the message text are all a viewer sees: both are scanned.
    if format != "lottie" && (!visual_bytes || bytes.is_none()) {
        report.merge(privacy::PrivacyReport::suspicious("scan_incomplete"));
    }
    report.apply_score_policy(config);
    (report, bytes)
}

pub(super) fn sticker_visual_segment(
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

pub(super) fn parse_visual_segments(content: &str) -> Option<Vec<VisualSegment>> {
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

pub(super) fn plain_text_segments(text: String) -> Vec<VisualSegment> {
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

fn classify_attachment(attachment: &serenity::all::Attachment) -> Option<MediaKind> {
    classify_media(&attachment.filename, attachment.content_type.as_deref())
}

pub(super) fn classify_media(filename: &str, content_type: Option<&str>) -> Option<MediaKind> {
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

pub(super) fn prepare_notification_text(content: &str, character_limit: u32) -> Option<String> {
    let content = content.trim();
    if content.is_empty() {
        return None;
    }
    if character_limit == 0 {
        return Some(content.to_owned());
    }
    use unicode_segmentation::UnicodeSegmentation;
    let mut end = 0;
    for _ in 0..character_limit {
        let remaining = &content[end..];
        if remaining.is_empty() {
            break;
        }
        end += match parse_custom_emoji(remaining) {
            Some((bytes, ..)) => bytes,
            None => remaining.graphemes(true).next()?.len(),
        };
    }
    Some(content[..end].to_owned())
}

pub(super) fn prepare_media_text(content: &str) -> Option<String> {
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
