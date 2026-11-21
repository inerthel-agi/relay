use super::*;

pub(super) fn relay_command(config: &AppConfig) -> CreateCommand {
    let mut command = CreateCommand::new("relay")
        .description("Configure the local OBS media relay")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "channel",
                "Set the channel whose messages and media are relayed to OBS",
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
            "Toggle the Relay channel lock",
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
    let http = core
        .discord_http()
        .await
        .context("the Discord bot is not running")?;
    if !core.bot_status.read().await.connected {
        bail!("the Discord bot is not connected");
    }
    Command::set_global_commands(&http, vec![relay_command(config)])
        .await
        .context("failed to synchronize the Relay command schema")?;
    Ok(())
}

pub(super) async fn handle_relay(
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
    let guild_id = command
        .guild_id
        .context("Relay commands require a server.")?;
    match relay_command_scope(&option.name, &config.watched_channel_id) {
        RelayCommandScope::InvokingServer => {}
        RelayCommandScope::RelayServer(watched) => {
            channel_in_guild(http, ChannelId::new(watched.parse()?), guild_id).await?;
        }
        RelayCommandScope::Unanchored => return Ok(RELAY_CHANNEL_REQUIRED.into()),
    }
    for argument in arguments {
        if let CommandDataOptionValue::Channel(channel_id) = argument.value {
            channel_in_guild(http, channel_id, guild_id).await?;
        }
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
            if config.relay_channel_conflict() {
                return Ok(RELAY_CHANNEL_CHOICE_PENDING.into());
            }
            core.update_config(|config| config.watched_channel_id = channel_id.clone())
                .await?;
            Ok(format!(
                "Relay channel set to <#{channel_id}>. Its messages and media are relayed."
            ))
        }
        "url" => {
            let config = core.config.read().await.clone();
            Ok(connection_details(&config))
        }
        "show" => {
            let config = core.config.read().await.clone();
            Ok(format!(
                "Relay channel: {}\n{}",
                relay_channel_summary(&config),
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
        "lock" => toggle_channel_lock(core, http, guild_id).await,
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

const CHANGELOG_URL: &str =
    "https://raw.githubusercontent.com/inerthel-agi/relay/main/CHANGELOG.md";
const CHANGELOG_PAGE_URL: &str = "https://github.com/inerthel-agi/relay/blob/main/CHANGELOG.md";
const CHANGELOG_MAX_BYTES: usize = 256 * 1024;
const CHANGELOG_EMBED_DESCRIPTION_LIMIT: usize = 3_900;
const CHANGELOG_MAX_EMBEDS: usize = 10;
const CHANGELOG_EMBED_COLOUR: u32 = 0x2F_B3_A8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChangelogSection {
    pub(super) heading: String,
    pub(super) version: String,
    pub(super) date: Option<String>,
    pub(super) body: String,
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
        || url.path() != "/inerthel-agi/relay/main/CHANGELOG.md"
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

pub(super) fn latest_changelog_section(changelog: &str) -> Option<ChangelogSection> {
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

pub(super) fn discord_format_changelog_body(body: &str) -> String {
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

pub(super) fn build_changelog_embeds(section: &ChangelogSection) -> (Vec<CreateEmbed>, bool) {
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

pub(super) fn char_len(value: &str) -> usize {
    value.chars().count()
}

pub(super) fn split_message_chunks(text: &str, limit: usize) -> Vec<String> {
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

pub(super) fn replace_configured_channel_id(
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
    if config.former_message_channel_id == old_channel_id {
        config.former_message_cleanup_enabled = false;
        config.former_message_welcome_message_id.clear();
    }
    if config.music_channel_id == old_channel_id {
        config.music_cleanup_enabled = false;
        config.music_welcome_message_id.clear();
    }
    for channel_id in [
        &mut config.watched_channel_id,
        &mut config.former_message_channel_id,
        &mut config.music_channel_id,
        &mut config.honeypot_channel_id,
    ]
    .into_iter()
    .chain(&mut config.extra_relay_channel_ids)
    .chain(&mut config.extra_music_channel_ids)
    .chain(&mut config.extra_honeypot_channel_ids)
    {
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
        let now = now_ms() / 1_000;
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

pub(super) fn clear_candidate(message_id: MessageId, protected: Option<u64>) -> bool {
    Some(message_id.get()) != protected
}

pub(super) fn is_bulk_deletable(message_id: MessageId, now_seconds: u64) -> bool {
    const SAFE_BULK_DELETE_AGE_SECONDS: u64 = 13 * 24 * 60 * 60 + 23 * 60 * 60;
    let created = message_id.created_at().unix_timestamp().max(0) as u64;
    now_seconds.saturating_sub(created) < SAFE_BULK_DELETE_AGE_SECONDS
}

pub(super) fn command_enabled(config: &AppConfig, command: &str) -> bool {
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

const RELAY_CHANNEL_REQUIRED: &str = "Choose the Relay channel in the Relay application first.";

#[derive(Debug, PartialEq)]
pub(super) enum RelayCommandScope<'a> {
    /// Acts on a channel of the invoking server, checked against that server.
    InvokingServer,
    /// Must run in the server of the configured Relay channel.
    RelayServer(&'a str),
    /// No Relay channel yet: nothing ties the command to the streamer's server,
    /// so an administrator of any other server running the bot would pass.
    Unanchored,
}

/// Lock validates its actual target itself, including a previous channel's snapshot.
pub(super) fn relay_command_scope<'a>(
    subcommand: &str,
    watched_channel_id: &'a str,
) -> RelayCommandScope<'a> {
    if matches!(subcommand, "clear" | "nuke" | "changelog" | "lock") {
        RelayCommandScope::InvokingServer
    } else if watched_channel_id.is_empty() {
        RelayCommandScope::Unanchored
    } else {
        RelayCommandScope::RelayServer(watched_channel_id)
    }
}

pub(super) fn default_command_authorized(
    guild_id: Option<GuildId>,
    permissions: Option<Permissions>,
) -> bool {
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
    ))
}

pub(super) fn format_relay_status(
    config: &AppConfig,
    bot: &BotStatus,
    server: &ServerStatus,
    moderation_pending: usize,
) -> String {
    let bot_state = if bot.connected {
        bot.username
            .as_deref()
            .map(|username| format!("connected as {username}"))
            .unwrap_or_else(|| "connected".into())
    } else {
        "disconnected".into()
    };
    let relay_channel = relay_channel_summary(config);
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
         Relay channel: {relay_channel}\n\
         Moderation: {moderation}\n\
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

pub(super) fn relay_channel_summary(config: &AppConfig) -> String {
    let summary = if config.relay_channel_conflict() {
        format!(
            "choice pending in the Relay application (media <#{}>, messages <#{}>)",
            config.watched_channel_id, config.former_message_channel_id
        )
    } else if config.watched_channel_id.is_empty() {
        "not configured".into()
    } else {
        format!("<#{}>", config.watched_channel_id)
    };
    if config.extra_relay_channel_ids.is_empty() {
        return summary;
    }
    let extras = config
        .extra_relay_channel_ids
        .iter()
        .map(|id| format!("<#{id}>"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{summary}, also {extras}")
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

pub(super) async fn relay_output_test(core: &AppCore, target: OutputTestTarget) -> Result<String> {
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
        OutputTestTarget::Notification => &server.outputs.notification,
        OutputTestTarget::Sticker => &server.outputs.sticker,
    };
    status.obs_clients + status.widget_clients
}

fn output_test_label(target: OutputTestTarget) -> &'static str {
    match target {
        OutputTestTarget::Visual => "media",
        OutputTestTarget::Audio => "audio",
        OutputTestTarget::Notification => "notification",
        OutputTestTarget::Sticker => "sticker",
    }
}

pub(super) fn require_same_guild(target: GuildId, invoking: GuildId) -> Result<()> {
    if target != invoking {
        bail!("This Relay command cannot control a channel in another Discord server.");
    }
    Ok(())
}

async fn channel_in_guild(
    http: &Http,
    channel_id: ChannelId,
    guild_id: GuildId,
) -> Result<serenity::all::GuildChannel> {
    let Channel::Guild(channel) = channel_id.to_channel(http).await? else {
        bail!("Relay commands require a server channel.");
    };
    require_same_guild(channel.guild_id, guild_id)?;
    Ok(channel)
}

async fn toggle_channel_lock(
    core: &Arc<AppCore>,
    http: &Http,
    guild_id: GuildId,
) -> Result<String> {
    let config = core.config.read().await.clone();
    if let Some(snapshot) = config.channel_lock.clone() {
        channel_in_guild(http, ChannelId::new(snapshot.channel_id.parse()?), guild_id).await?;
        restore_channel_permissions(http, &snapshot).await?;
        core.update_config(|next| next.channel_lock = None).await?;
        return Ok(format!("<#{0}> is unlocked.", snapshot.channel_id));
    }
    if config.watched_channel_id.is_empty() {
        bail!("configure the Relay channel before locking it");
    }

    let channel_id = ChannelId::new(config.watched_channel_id.parse()?);
    let channel = channel_in_guild(http, channel_id, guild_id).await?;
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

pub(super) fn snapshot_permission(
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

pub(super) fn connection_details(config: &AppConfig) -> String {
    format!(
        "Relay URL: `http://127.0.0.1:{}`\nVisual overlay: `{}`\nAudio overlay: `{}`",
        config.port,
        overlay_url(config),
        crate::widget::obs_audio_url(config.port)
    )
}

pub(super) fn overlay_url(config: &AppConfig) -> String {
    crate::widget::obs_visual_url(config.port)
}
