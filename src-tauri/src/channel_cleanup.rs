use std::{sync::Arc, time::Duration};

use serenity::{
    all::{ChannelId, GetMessages, MessageId},
    http::Http,
};

use crate::{config::AppConfig, state::AppCore};

const RETENTION_SECONDS: u64 = 24 * 60 * 60;
const SWEEP_INTERVAL: Duration = Duration::from_secs(5 * 60);
const MAX_PAGES: usize = 10;

#[derive(Clone, Copy)]
enum ChannelKind {
    Media,
    FormerMessages,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CleanupRule {
    channel: u64,
    welcome: Option<u64>,
    reaction_protected: Option<u64>,
}

#[derive(Default)]
struct Scan {
    rule: Option<CleanupRule>,
    before: Option<MessageId>,
}

fn rule(config: &AppConfig, kind: ChannelKind) -> Option<CleanupRule> {
    let (enabled, channel, welcome) = match kind {
        ChannelKind::Media => (
            config.media_cleanup_enabled,
            &config.watched_channel_id,
            &config.media_welcome_message_id,
        ),
        ChannelKind::FormerMessages => (
            config.former_message_cleanup_enabled,
            &config.former_message_channel_id,
            &config.former_message_welcome_message_id,
        ),
    };
    if !enabled {
        return None;
    }
    let channel = channel.parse::<u64>().ok().filter(|id| *id > 0)?;
    let welcome = if welcome.is_empty() {
        None
    } else {
        Some(welcome.parse::<u64>().ok().filter(|id| *id > 0)?)
    };
    let reaction_protected = crate::reaction_protection::protected_message_for_channel(
        &config.reactions.protected_channel_id,
        &config.reactions.protected_message_id,
        channel,
    );
    Some(CleanupRule {
        channel,
        welcome,
        reaction_protected,
    })
}

fn expired(message_id: u64, timestamp: u64, now: u64, welcome: Option<u64>) -> bool {
    welcome != Some(message_id)
        && now
            .checked_sub(timestamp)
            .is_some_and(|age| age >= RETENTION_SECONDS)
}

async fn unchanged(core: &AppCore, kind: ChannelKind, expected: &CleanupRule) -> bool {
    rule(&*core.config.read().await, kind).as_ref() == Some(expected)
}

async fn sweep(
    core: &AppCore,
    http: &Http,
    kind: ChannelKind,
    scan: &mut Scan,
) -> Result<(), &'static str> {
    let current = rule(&*core.config.read().await, kind);
    if scan.rule != current {
        scan.before = None;
        scan.rule = current.clone();
    }
    let Some(current) = current else {
        return Ok(());
    };
    let channel = ChannelId::new(current.channel);
    if let Some(welcome) = current.welcome {
        channel
            .message(http, MessageId::new(welcome))
            .await
            .map_err(
                |_| "24-hour cleanup paused: the protected welcome message cannot be verified.",
            )?;
    }
    let now = crate::clock::now_secs();
    for _ in 0..MAX_PAGES {
        if !unchanged(core, kind, &current).await {
            scan.before = None;
            return Ok(());
        }
        let mut request = GetMessages::new().limit(100);
        if let Some(before) = scan.before {
            request = request.before(before);
        }
        let page = channel.messages(http, request).await.map_err(
            |_| "24-hour cleanup failed: check View Channel and Read Message History permissions.",
        )?;
        if page.is_empty() {
            scan.before = None;
            return Ok(());
        }
        let oldest = page.iter().map(|message| message.id).min();
        for message in &page {
            let timestamp = message.timestamp.unix_timestamp().max(0) as u64;
            if !expired(message.id.get(), timestamp, now, current.welcome)
                || current.reaction_protected == Some(message.id.get())
            {
                continue;
            }
            if !unchanged(core, kind, &current).await {
                scan.before = None;
                return Ok(());
            }
            message.delete(http).await
                .map_err(|_| "24-hour cleanup stopped after a deletion error. Check Manage Messages permission.")?;
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        // Resume older history on the next pass instead of repeatedly scanning busy recent pages.
        scan.before = oldest;
        if page.len() < 100 {
            scan.before = None;
            return Ok(());
        }
    }
    Ok(())
}

/// Polled alongside the Discord gateway; stopping the bot cancels this future too.
pub async fn run(core: Arc<AppCore>, http: Arc<Http>) {
    let mut interval = tokio::time::interval(SWEEP_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut scans = [Scan::default(), Scan::default()];
    loop {
        interval.tick().await;
        if !core.bot_status.read().await.connected {
            continue;
        }
        for (kind, scan) in [ChannelKind::Media, ChannelKind::FormerMessages]
            .into_iter()
            .zip(&mut scans)
        {
            if let Err(error) = sweep(&core, &http, kind, scan).await {
                core.bot_status.write().await.error = Some(error.into());
            }
        }
    }
}

/// Accept the same message-link format used by music cleanup, with a channel-neutral error.
pub fn welcome_message_id(value: &str, channel: &str) -> Result<String, String> {
    crate::music_cleanup::protected_message_id(value, channel).map_err(|_| {
        "Enter a valid welcome message ID or a Discord link from the selected channel.".into()
    })
}

pub async fn verify_welcome(
    core: &AppCore,
    enabled: bool,
    channel: &str,
    welcome: &str,
) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }
    let channel = channel
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or("Select a channel before enabling 24-hour cleanup.")?;
    // Same checks as the music channel's welcome message, with this page's wording.
    let Some(welcome) = crate::music_cleanup::optional_protected_message(welcome)
        .map_err(|_| "Enter a valid welcome message ID.")?
    else {
        return Ok(());
    };
    let http = core
        .discord_http()
        .await
        .ok_or("Connect the bot to verify the welcome message.")?;
    crate::music_cleanup::verify_protected_message(
        &http,
        channel,
        Some(welcome),
        "The welcome message was not found in the selected channel.",
    )
    .await
}

#[cfg(test)]
mod tests;
