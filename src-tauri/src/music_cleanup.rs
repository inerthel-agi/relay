use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use serde::Serialize;
use serenity::{
    all::{ChannelId, GetMessages, MessageId},
    http::Http,
};
use tauri::State;

use crate::state::AppCore;

const PREVIEW_LIMIT: usize = 1_000;
const TTL: Duration = Duration::from_secs(120);

#[derive(Default)]
pub struct MusicCleanup {
    active: HashMap<(u64, u64), Instant>,
    preview: Option<CleanupSnapshot>,
    generation: u64,
}

struct CleanupSnapshot {
    token: String,
    channel: u64,
    protected: Option<u64>,
    messages: Vec<u64>,
    expires: Instant,
}

impl MusicCleanup {
    fn take_preview(
        &mut self,
        token: &str,
        scope: (u64, Option<u64>),
        now: Instant,
    ) -> Result<CleanupSnapshot, String> {
        let snapshot = self
            .preview
            .take()
            .ok_or("Preview the cleanup again before confirming.")?;
        if snapshot.token != token || snapshot.expires <= now {
            return Err("The cleanup preview expired. Preview it again.".into());
        }
        if scope != (snapshot.channel, snapshot.protected) {
            return Err("Music settings changed. Preview the cleanup again.".into());
        }
        Ok(snapshot)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreview {
    token: String,
    count: usize,
    limit_reached: bool,
}

#[derive(Serialize)]
pub struct CleanupResult {
    deleted: usize,
    failed: usize,
    skipped: usize,
}

pub fn protected_message_id(value: &str, channel: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    let id = if value.starts_with("https://discord.com/channels/") {
        let parts: Vec<_> = value.trim_end_matches('/').split('/').collect();
        if parts.len() != 7 || parts[5] != channel || parts[4].parse::<u64>().is_err() {
            return Err(
                "The welcome message link must belong to the selected music channel.".into(),
            );
        }
        parts[6]
    } else {
        value
    };
    if id.parse::<u64>().ok().filter(|id| *id != 0).is_none() {
        return Err("Enter a valid Discord welcome message ID or link.".into());
    }
    Ok(id.to_owned())
}

pub(crate) fn requires_verification(
    previous: &crate::config::AppConfig,
    enabled: bool,
    channel: &str,
    welcome: &str,
) -> bool {
    enabled
        && !welcome.trim().is_empty()
        && (!previous.music_cleanup_enabled
            || previous.music_channel_id != channel
            || previous.music_welcome_message_id != welcome)
}

pub(crate) fn optional_protected_message(value: &str) -> Result<Option<u64>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<u64>()
        .ok()
        .filter(|id| *id != 0)
        .map(Some)
        .ok_or_else(|| "Enter a valid Discord welcome message ID or link.".into())
}

pub(crate) async fn verify_protected_message(
    http: &Http,
    channel: u64,
    protected: Option<u64>,
    error: &str,
) -> Result<(), String> {
    if let Some(message) = protected {
        ChannelId::new(channel)
            .message(http, MessageId::new(message))
            .await
            .map_err(|_| error.to_owned())?;
    }
    Ok(())
}

fn may_delete(id: u64, protected: Option<u64>, active: bool) -> bool {
    Some(id) != protected && !active
}

async fn http(core: &AppCore) -> Result<Arc<Http>, String> {
    core.discord_http()
        .await
        .ok_or_else(|| "Connect the Discord bot first.".into())
}

async fn configured(core: &AppCore) -> Result<(u64, Option<u64>), String> {
    let config = core.config.read().await;
    if !config.music_cleanup_enabled {
        return Err("Enable music channel cleanup first.".into());
    }
    let channel = config
        .music_channel_id
        .parse::<u64>()
        .map_err(|_| "Select a music channel first.")?;
    let protected = optional_protected_message(&config.music_welcome_message_id)?;
    Ok((channel, protected))
}

async fn active(core: &AppCore, channel: u64, message: u64) -> bool {
    if core
        .music_cleanup
        .lock()
        .await
        .active
        .get(&(channel, message))
        .is_some_and(|expires| *expires > Instant::now())
    {
        return true;
    }
    core.music
        .lock()
        .await
        .active_message_ids()
        .contains(&(channel, message))
}

async fn reaction_protected(core: &AppCore, channel: u64, message: u64) -> bool {
    let config = core.config.read().await;
    crate::reaction_protection::is_protected(
        &config.reactions.protected_channel_id,
        &config.reactions.protected_message_id,
        channel,
        message,
    )
}

pub async fn delete(core: &AppCore, http: &Http, channel: u64, message: u64) {
    let Ok((configured_channel, protected)) = configured(core).await else {
        return;
    };
    if channel != configured_channel
        || !may_delete(message, protected, false)
        || reaction_protected(core, channel, message).await
    {
        return;
    }
    core.music_cleanup
        .lock()
        .await
        .active
        .remove(&(channel, message));
    if ChannelId::new(channel)
        .delete_message(http, MessageId::new(message))
        .await
        .is_err()
    {
        core.bot_status.write().await.error = Some(
            "Music message cleanup failed. Check View Channel, Read Message History and Manage Messages permissions.".into());
    }
}

pub async fn expire_message(core: &Arc<AppCore>, http: &Arc<Http>, channel: u64, message: u64) {
    let Ok((configured_channel, protected)) = configured(core).await else {
        return;
    };
    if channel != configured_channel
        || Some(message) == protected
        || reaction_protected(core, channel, message).await
    {
        return;
    }
    core.music_cleanup
        .lock()
        .await
        .active
        .insert((channel, message), Instant::now() + TTL);
    let weak = Arc::downgrade(core);
    let http = http.clone();
    tokio::spawn(async move {
        tokio::time::sleep(TTL).await;
        if let Some(core) = weak.upgrade() {
            let tracked = core
                .music_cleanup
                .lock()
                .await
                .active
                .remove(&(channel, message))
                .is_some();
            if tracked {
                delete(&core, &http, channel, message).await;
            }
        }
    });
}

#[tauri::command]
pub async fn preview_music_cleanup(
    core: State<'_, Arc<AppCore>>,
) -> Result<CleanupPreview, String> {
    let (channel, protected) = configured(&core).await?;
    let http = http(&core).await?;
    verify_protected_message(
        &http,
        channel,
        protected,
        "The welcome message could not be found in this channel. Nothing was deleted.",
    )
    .await?;
    let mut messages = Vec::new();
    let mut cursor = None;
    let mut scanned = 0;
    while scanned < PREVIEW_LIMIT {
        let mut request = GetMessages::new().limit(100);
        if let Some(before) = cursor {
            request = request.before(before);
        }
        let page = ChannelId::new(channel)
            .messages(&http, request)
            .await
            .map_err(|_| "Unable to read music channel history.")?;
        if page.is_empty() {
            break;
        }
        cursor = page.iter().map(|message| message.id).min();
        scanned += page.len();
        for message in &page {
            if may_delete(
                message.id.get(),
                protected,
                active(&core, channel, message.id.get()).await,
            ) && !reaction_protected(&core, channel, message.id.get()).await
            {
                messages.push(message.id.get());
            }
        }
        if page.len() < 100 {
            break;
        }
    }
    let mut cleanup = core.music_cleanup.lock().await;
    cleanup.generation = cleanup.generation.wrapping_add(1);
    let token = format!("cleanup-{}", cleanup.generation);
    let count = messages.len();
    cleanup.preview = Some(CleanupSnapshot {
        token: token.clone(),
        channel,
        protected,
        messages,
        expires: Instant::now() + TTL,
    });
    Ok(CleanupPreview {
        token,
        count,
        limit_reached: scanned >= PREVIEW_LIMIT,
    })
}

#[tauri::command]
pub async fn confirm_music_cleanup(
    core: State<'_, Arc<AppCore>>,
    token: String,
) -> Result<CleanupResult, String> {
    let scope = configured(&core).await?;
    let snapshot = core
        .music_cleanup
        .lock()
        .await
        .take_preview(&token, scope, Instant::now())?;
    let http = http(&core).await?;
    verify_protected_message(
        &http,
        snapshot.channel,
        snapshot.protected,
        "The protected welcome message is no longer available. Cleanup cancelled.",
    )
    .await?;
    let mut result = CleanupResult {
        deleted: 0,
        failed: 0,
        skipped: 0,
    };
    for id in snapshot.messages {
        if configured(&core).await? != (snapshot.channel, snapshot.protected) {
            return Err("Music settings changed; cleanup stopped.".into());
        }
        if !may_delete(
            id,
            snapshot.protected,
            active(&core, snapshot.channel, id).await,
        ) || reaction_protected(&core, snapshot.channel, id).await
        {
            result.skipped += 1;
            continue;
        }
        match ChannelId::new(snapshot.channel)
            .delete_message(&http, MessageId::new(id))
            .await
        {
            Ok(()) => result.deleted += 1,
            Err(_) => {
                result.failed += 1;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    if result.failed > 0 {
        core.bot_status.write().await.error = Some(
            "Music cleanup stopped after a Discord deletion error. Check bot permissions.".into(),
        );
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
