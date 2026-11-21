use super::*;

impl Handler {
    pub(super) async fn relay_message_update(
        &self,
        context: Context,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        let relays_media = self
            .core
            .config
            .read()
            .await
            .relays_media(&event.channel_id.to_string());
        if !relays_media
            || (new.is_none() && event.embeds.is_none())
            || !fresh_for_embeds(event.id, now_ms() / 1_000)
        {
            return;
        }
        let _observation = self.core.track_gif_preview(event.id).await;
        if event
            .embeds
            .as_ref()
            .is_some_and(|embeds| embeds.iter().any(|embed| embedded_gif(embed).is_some()))
        {
            self.core.observe_gif_embed(event.id, None).await;
        }
        // One media slot per message, taken where processing starts: the
        // slots are not reentrant, so a nested acquire would wait forever.
        let Ok(_media_slot) = self.core.media_ingest_slots.acquire().await else {
            return;
        };
        if let Some(message) = new {
            let staff = is_server_staff(&context.cache, &message);
            submit_embedded_gifs(&self.core, &context.http, &message, staff).await;
            return;
        }
        if let Ok(message) = context.http.get_message(event.channel_id, event.id).await {
            let staff = is_server_staff(&context.cache, &message);
            submit_embedded_gifs(&self.core, &context.http, &message, staff).await;
        }
        // A partial update cannot establish the member's moderation context.
    }
}

const GIF_PROVIDER_HOSTS: [&str; 3] = ["tenor.com", "giphy.com", "klipy.com"];

pub(super) fn has_gif_link(content: &str) -> bool {
    content
        .split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '<' | '>'))
        .any(|part| is_gif_link(part, &[]))
}

pub(super) fn without_gif_links<'a>(
    content: &'a str,
    embeds: &[serenity::all::Embed],
) -> std::borrow::Cow<'a, str> {
    let is_gif_token = |token: &str| {
        token
            .split(['(', ')', '<', '>'])
            .any(|part| is_gif_link(part, embeds))
    };
    if !content.split_whitespace().any(is_gif_token) {
        return std::borrow::Cow::Borrowed(content);
    }
    std::borrow::Cow::Owned(
        content
            .lines()
            .map(|line| {
                line.split_whitespace()
                    .filter(|token| !is_gif_token(token))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// A GIF served by a public GIF library over HTTPS (not a file uploaded to Discord).
pub(super) fn is_gif_library_url(value: &str) -> bool {
    reqwest::Url::parse(value).is_ok_and(|url| url.scheme() == "https" && gif_library_host(&url))
}

fn gif_library_host(url: &reqwest::Url) -> bool {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    GIF_PROVIDER_HOSTS
        .iter()
        .any(|provider| host == *provider || host.ends_with(&format!(".{provider}")))
}

fn is_gif_link(value: &str, embeds: &[serenity::all::Embed]) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    url_has_extension(url.as_str(), "gif")
        || gif_library_host(&url)
        || embeds
            .iter()
            .any(|embed| embed.url.as_deref() == Some(value) && embedded_gif(embed).is_some())
}

/// Discord adds a GIF preview within seconds of the message. A later update
/// is an edit: after a restart nothing remembers its GIF was shown, so it
/// would come back on stream.
pub(crate) fn fresh_for_embeds(message_id: MessageId, now_seconds: u64) -> bool {
    const EMBED_UPDATE_WINDOW_SECONDS: u64 = 2 * 60;
    let created = message_id.created_at().unix_timestamp().max(0) as u64;
    now_seconds.saturating_sub(created) <= EMBED_UPDATE_WINDOW_SECONDS
}

pub(super) struct EmbeddedGif {
    pub(super) url: String,
    pub(super) proxy_url: String,
    pub(super) title: Option<String>,
    pub(super) content_type: &'static str,
}

pub(super) struct DeferredEmbedMessage {
    pub(super) channel_id: String,
    pub(super) message_id: String,
    pub(super) author: serenity::all::User,
    pub(super) timestamp: u64,
    pub(super) content: String,
    /// Media card text; `None` when the notification already shows the text.
    pub(super) caption: Option<String>,
    pub(super) embeds: Vec<serenity::all::Embed>,
    pub(super) role_ids: Vec<String>,
}

/// Relays the GIF embeds and links of a media channel message. Returns true
/// when one of them blocked the message.
pub(super) async fn submit_embedded_gifs(
    core: &Arc<AppCore>,
    http: &Http,
    message: &Message,
    staff: bool,
) -> bool {
    let config = core.config.read().await.clone();
    if message.author.bot || !config.relays_media(&message.channel_id.to_string()) {
        return false;
    }
    let role_ids = message_role_ids(message);
    if core.moderation_verdict(&message.id.to_string()).is_none()
        && config.moderation.min_member_age_days > 0
        && message
            .member
            .as_ref()
            .and_then(|member| member.joined_at)
            .is_none()
    {
        // Logged once per message: an edited message can bring many updates.
        if core
            .claim_embed(format!("member-age-unknown:{}", message.id))
            .await
        {
            core.record_moderation(
                Some(crate::moderation::Lane::Media),
                crate::moderation::log::LogAction::Ignored,
                "member_age_unknown",
                Some(message.author.id.to_string()),
            );
        }
        return false;
    }
    if !apply_moderation_gate(
        core,
        message,
        &role_ids,
        staff,
        &[crate::moderation::Lane::Media],
        &config,
    ) {
        return false;
    }
    let notifies = config
        .channel_feeds(&message.channel_id.to_string())
        .is_some_and(|feeds| feeds.notifications)
        && notification_text(message).is_some();
    submit_deferred_embeds(
        core,
        http,
        (message, staff),
        download_embedded_gif,
        DeferredEmbedMessage {
            channel_id: message.channel_id.to_string(),
            message_id: message.id.to_string(),
            author: message.author.clone(),
            timestamp: message.timestamp.unix_timestamp().max(0) as u64 * 1_000,
            content: message.content.clone(),
            caption: if notifies {
                None
            } else {
                prepare_media_text(&message.content)
            },
            embeds: message.embeds.clone(),
            role_ids: message_role_ids(message),
        },
    )
    .await
}

/// Returns true when the message was blocked. `source` and `staff` (the
/// Discord message and whether its author is a moderator) are for the block
/// follow-up (log, warning, escalation) and the AI image check.
pub(super) async fn submit_deferred_embeds(
    core: &Arc<AppCore>,
    http: &Http,
    (source, staff): (&Message, bool),
    download: impl AsyncFn(EmbeddedGif, Vec<EmbeddedGif>) -> (EmbeddedGif, Option<Vec<u8>>),
    message: DeferredEmbedMessage,
) -> bool {
    if message.author.bot {
        return false;
    }
    let config = core.config.read().await.clone();
    let scoped_config = crate::moderation::scope_config(
        privacy::scoped_config_for_roles(&config, &message.role_ids),
        crate::moderation::Lane::Media,
    );
    if !config.relays_media(&message.channel_id) {
        return false;
    }
    let message_report = classify_privacy_values(
        &message.content,
        std::iter::empty::<&str>(),
        std::iter::empty::<&str>(),
        &scoped_config,
    );
    if block_and_delete_message_if_needed(core, http, source, &message_report, &scoped_config).await
    {
        return true;
    }
    let media_text = message.caption.clone();
    let direct_link = direct_gif_link(&message.content).is_some();
    let gifs = message_gifs(&message.content, &message.embeds);
    if gifs.is_empty() && has_gif_link(&message.content) {
        core.observe_gif_embed(source.id, Some(source.author.id))
            .await;
    }
    for (index, gif) in gifs.into_iter().enumerate() {
        let event_id = format!("{}-embed-{index}", message.message_id);
        if core.embed_claimed(&event_id).await {
            continue;
        }
        let fallbacks = if direct_link {
            link_embed_gifs(&gif.url, &message.embeds)
        } else {
            Vec::new()
        };
        let (embed, downloaded) = download(gif, fallbacks).await;
        // Claimed only now, so the embed update can still retry a failed link.
        if downloaded.is_none() && wait_for_link_embed(direct_link, &message.embeds) {
            core.observe_gif_embed(source.id, Some(source.author.id))
                .await;
            continue;
        }
        core.observe_gif_embed(source.id, None).await;
        if !core.claim_embed(event_id).await {
            continue;
        }
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
                Some(bytes) if is_gif_library_url(&event.url) => {
                    privacy::analyze_library_gif_async(
                        bytes,
                        Some(&message.content),
                        &scoped_config,
                    )
                    .await
                }
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
        let current =
            current_lane_config(core, &message.role_ids, crate::moderation::Lane::Media).await;
        let gif = (
            MediaKind::Gif,
            event.url.as_str(),
            event.proxy_url.as_str(),
            downloaded.as_deref(),
        );
        let Some(privacy_report) = check_media_part(
            core,
            http,
            source,
            staff,
            &current,
            (&message.content, privacy_report),
            Some(gif),
        )
        .await
        else {
            return true;
        };
        core.submit_analyzed_media_with_text_and_roles(
            event,
            Some(privacy_report),
            Some(&message.content),
            &message.role_ids,
        )
        .await;
    }
    false
}

/// Downloads the GIF, then the URLs of the embeds given as fallbacks, and
/// returns the source that worked (the GIF itself when none did).
async fn download_embedded_gif(
    gif: EmbeddedGif,
    fallbacks: Vec<EmbeddedGif>,
) -> (EmbeddedGif, Option<Vec<u8>>) {
    let mut candidates = vec![gif];
    candidates.extend(fallbacks);
    for index in 0..candidates.len() {
        let candidate = &candidates[index];
        let mut urls = vec![candidate.url.as_str()];
        if candidate.proxy_url != candidate.url {
            urls.push(candidate.proxy_url.as_str());
        }
        for url in urls {
            if let Ok(bytes) = artwork::download_bounded(url, artwork::MAX_EMBED_MEDIA_BYTES).await
            {
                return (candidates.swap_remove(index), Some(bytes));
            }
        }
    }
    (candidates.swap_remove(0), None)
}

/// The GIFs of the embeds Discord made for `link`. A pasted Discord CDN link
/// is signed and expires; its embed carries fresh and proxied URLs.
pub(super) fn link_embed_gifs(link: &str, embeds: &[serenity::all::Embed]) -> Vec<EmbeddedGif> {
    let without_query = |url: &str| {
        reqwest::Url::parse(url).ok().map(|mut url| {
            url.set_query(None);
            url.set_fragment(None);
            url
        })
    };
    let Some(link) = without_query(link) else {
        return Vec::new();
    };
    embeds
        .iter()
        .filter(|embed| embed.url.as_deref().and_then(without_query).as_ref() == Some(&link))
        .filter_map(embedded_gif)
        .collect()
}

/// A typed GIF link that could not be downloaded before Discord attached its
/// embed: leave it unclaimed so the embed update (fresh messages only) retries.
pub(super) fn wait_for_link_embed(direct_link: bool, embeds: &[serenity::all::Embed]) -> bool {
    direct_link && embeds.is_empty()
}

pub(super) fn message_gifs(content: &str, embeds: &[serenity::all::Embed]) -> Vec<EmbeddedGif> {
    direct_gif_link(content)
        .map(|gif| vec![gif])
        .unwrap_or_else(|| embeds.iter().filter_map(embedded_gif).collect())
}

pub(super) fn direct_gif_link(content: &str) -> Option<EmbeddedGif> {
    let url = content
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '(' | ')' | '<' | '>')
        })
        .filter_map(|part| reqwest::Url::parse(part).ok())
        .find(|url| artwork::url_allowed(url) && url_has_extension(url.as_str(), "gif"))?
        .to_string();
    Some(EmbeddedGif {
        proxy_url: url.clone(),
        url,
        title: None,
        content_type: "image/gif",
    })
}

pub(super) fn embedded_gif(embed: &serenity::all::Embed) -> Option<EmbeddedGif> {
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
    } else if let Some(thumbnail) = embed.thumbnail.as_ref() {
        (
            thumbnail.url.clone(),
            thumbnail
                .proxy_url
                .clone()
                .unwrap_or_else(|| thumbnail.url.clone()),
        )
    } else {
        let url = embed
            .url
            .as_deref()
            .filter(|url| url_has_extension(url, "gif"))?;
        (url.to_owned(), url.to_owned())
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

pub(super) fn sniff_media_type(bytes: &[u8], fallback: &'static str) -> &'static str {
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
