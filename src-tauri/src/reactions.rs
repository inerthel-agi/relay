use crate::{config::OutputGeometry, model::RelayEvent, state::AppCore};
use anyhow::{Context, Result, bail};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    path::Path,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ReactionSettings {
    pub enabled: bool,
    pub global_cooldown_seconds: u16,
    pub member_cooldown_seconds: u16,
    pub music_percent: u8,
    pub allowed_channel_ids: Vec<String>,
    pub allowed_role_ids: Vec<String>,
    pub protected_message_id: String,
    pub protected_channel_id: String,
    pub geometry: OutputGeometry,
    pub definitions: Vec<ReactionDefinition>,
}
impl Default for ReactionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            global_cooldown_seconds: 10,
            member_cooldown_seconds: 30,
            music_percent: 25,
            allowed_channel_ids: vec![],
            allowed_role_ids: vec![],
            protected_message_id: String::new(),
            protected_channel_id: String::new(),
            geometry: OutputGeometry::default(),
            definitions: vec![],
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReactionDefinition {
    pub id: String,
    pub name: String,
    pub sound_id: String,
    pub visual_id: Option<String>,
    pub volume: u8,
    pub enabled: bool,
}
impl ReactionSettings {
    pub fn validate(&self) -> Result<()> {
        if self.definitions.len() > 25
            || self.music_percent > 100
            || self.global_cooldown_seconds > 3600
            || self.member_cooldown_seconds > 3600
        {
            bail!("Invalid reaction limits.");
        }
        crate::reaction_protection::validate(
            &self.protected_channel_id,
            &self.protected_message_id,
            &self.allowed_channel_ids,
        )?;
        self.geometry.validate()?;
        let mut ids = std::collections::HashSet::new();
        for item in &self.definitions {
            if !valid_id(&item.id)
                || !valid_id(&item.sound_id)
                || item.visual_id.as_ref().is_some_and(|id| !valid_id(id))
                || item.name.trim().is_empty()
                || item.name.chars().count() > 80
                || item.name.chars().any(char::is_control)
                || item.volume > 100
                || !ids.insert(&item.id)
            {
                bail!("Invalid reaction definition.");
            }
        }
        for list in [&self.allowed_channel_ids, &self.allowed_role_ids] {
            if list.len() > 100
                || list
                    .iter()
                    .any(|id| id.parse::<u64>().ok().is_none_or(|n| n == 0))
            {
                bail!("Invalid Discord access list.");
            }
        }
        Ok(())
    }
    pub fn authorize(&self, channel: &str, roles: &[String]) -> bool {
        self.enabled
            && self.allowed_channel_ids.iter().any(|id| id == channel)
            && roles
                .iter()
                .any(|role| self.allowed_role_ids.contains(role))
    }
}
pub fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn new_id() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Maximum duration of a reaction sound and its playback lease.
///
/// Keep this value as the single source of truth for both direct imports and
/// trimmed selections.  The trim module re-exports it for its command API.
pub const MAX_REACTION_SECONDS: f64 = 30.0;
pub const MAX_REACTION_DURATION_MS: u64 = (MAX_REACTION_SECONDS * 1000.0) as u64;

#[derive(Clone, Debug)]
pub struct ReactionPlayback {
    pub playback_id: String,
    pub reaction: ReactionDefinition,
    pub ends_at: u64,
    pub music_percent: u8,
}

impl Serialize for ReactionPlayback {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire<'a> {
            playback_id: &'a str,
            reaction: &'a ReactionDefinition,
            ends_at: u64,
            music_percent: u8,
            duration_ms: u64,
        }

        Wire {
            playback_id: &self.playback_id,
            reaction: &self.reaction,
            ends_at: self.ends_at,
            music_percent: self.music_percent,
            duration_ms: MAX_REACTION_DURATION_MS,
        }
        .serialize(serializer)
    }
}

pub const MAX_PENDING_REACTIONS: usize = 25;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TriggerResult {
    /// Zero means playing; one is the first waiting request.
    pub position: usize,
}

struct PendingReaction {
    definition: ReactionDefinition,
    member: Option<(u64, String, Vec<String>)>,
}

#[derive(Default)]
pub struct ReactionRuntime {
    pub active: Option<ReactionPlayback>,
    pending: VecDeque<PendingReaction>,
    last_global: Option<Instant>,
    last_members: HashMap<u64, Instant>,
}
impl ReactionRuntime {
    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }

    pub fn retain_pending(&mut self, settings: &ReactionSettings) {
        self.pending.retain(|request| {
            settings.enabled
                && settings.definitions.iter().any(|definition| {
                    definition.id == request.definition.id
                        && definition.enabled
                        && definition.sound_id == request.definition.sound_id
                })
                && request
                    .member
                    .as_ref()
                    .is_none_or(|(_, channel, roles)| settings.authorize(channel, roles))
        });
    }

    fn begin(
        &mut self,
        settings: &ReactionSettings,
        definition: ReactionDefinition,
    ) -> ReactionPlayback {
        let playback = ReactionPlayback {
            playback_id: new_id(),
            reaction: definition,
            ends_at: now_ms().saturating_add(MAX_REACTION_DURATION_MS),
            music_percent: settings.music_percent,
        };
        self.active = Some(playback.clone());
        playback
    }

    fn submit(
        &mut self,
        settings: &ReactionSettings,
        definition: ReactionDefinition,
        member: Option<(u64, String, Vec<String>)>,
        now: Instant,
    ) -> Result<TriggerResult> {
        if !settings.enabled || !definition.enabled {
            bail!("Reactions are disabled.");
        }
        if self.active.is_some() && self.pending.len() >= MAX_PENDING_REACTIONS {
            bail!("Reaction queue is full.");
        }
        if let Some((member, _, _)) = &member {
            if self.last_global.is_some_and(|last| {
                now.duration_since(last).as_secs() < u64::from(settings.global_cooldown_seconds)
            }) || self.last_members.get(member).is_some_and(|last| {
                now.duration_since(*last).as_secs() < u64::from(settings.member_cooldown_seconds)
            }) {
                bail!("Please wait before triggering another reaction.");
            }
            self.last_members
                .retain(|_, last| now.duration_since(*last) < Duration::from_secs(3600));
            if self.last_members.len() >= 4096 && !self.last_members.contains_key(member) {
                bail!("Reaction requests are temporarily full.");
            }
            self.last_global = Some(now);
            self.last_members.insert(*member, now);
        }
        if self.active.is_some() {
            self.pending
                .push_back(PendingReaction { definition, member });
            Ok(TriggerResult {
                position: self.pending.len(),
            })
        } else {
            self.begin(settings, definition);
            Ok(TriggerResult { position: 0 })
        }
    }
}

fn publish_start(core: &Arc<AppCore>, playback: ReactionPlayback) {
    let id = playback.playback_id.clone();
    let wait_ms = playback.ends_at.saturating_sub(now_ms());
    let _ = core.relay_tx.send(RelayEvent::Reaction(Some(playback)));
    let core = core.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        stop(&core, Some(&id)).await;
    });
}

pub async fn trigger(
    core: &Arc<AppCore>,
    id: &str,
    member: Option<(u64, String, Vec<String>)>,
) -> Result<TriggerResult> {
    let mut runtime = core.reactions.lock().await;
    let settings = core.config.read().await.reactions.clone();
    if !settings.enabled {
        bail!("Reactions are disabled.");
    }
    if let Some((_, channel, roles)) = &member
        && !settings.authorize(channel, roles)
    {
        bail!("This channel or your roles cannot trigger reactions.");
    }
    let definition = settings
        .definitions
        .iter()
        .find(|item| item.id == id)
        .cloned()
        .context("Reaction unavailable.")?;
    if !core
        .data_directory
        .join("reactions")
        .join(&definition.sound_id)
        .is_file()
    {
        bail!("Reaction sound unavailable.");
    }
    {
        let status = core.server_status.read().await;
        if status.outputs.reaction.obs_clients + status.outputs.reaction.widget_clients == 0 {
            bail!("No reaction output is connected.");
        }
    }
    let result = runtime.submit(&settings, definition, member, Instant::now())?;
    if result.position == 0 {
        publish_start(
            core,
            runtime.active.clone().expect("accepted reaction started"),
        );
    }
    Ok(result)
}

pub async fn skip(core: &Arc<AppCore>) -> bool {
    let current = core
        .reactions
        .lock()
        .await
        .active
        .as_ref()
        .map(|active| active.playback_id.clone());
    if let Some(id) = current {
        stop(core, Some(&id)).await;
        true
    } else {
        false
    }
}

pub async fn stop(core: &Arc<AppCore>, expected: Option<&str>) {
    let mut runtime = core.reactions.lock().await;
    if expected.is_none() {
        runtime.clear_pending();
        if runtime.active.take().is_some() {
            let _ = core.relay_tx.send(RelayEvent::Reaction(None));
        }
        return;
    }
    if runtime
        .active
        .as_ref()
        .is_none_or(|active| Some(active.playback_id.as_str()) != expected)
    {
        return;
    }
    runtime.active = None;
    let settings = core.config.read().await.reactions.clone();
    runtime.retain_pending(&settings);
    let connected = {
        let status = core.server_status.read().await;
        status.outputs.reaction.obs_clients + status.outputs.reaction.widget_clients > 0
    };
    if !connected {
        runtime.clear_pending();
    }
    while let Some(request) = runtime.pending.pop_front() {
        // Settings may have changed while this request was waiting.
        let Some(definition) = settings
            .definitions
            .iter()
            .find(|definition| {
                definition.id == request.definition.id
                    && definition.enabled
                    && definition.sound_id == request.definition.sound_id
            })
            .cloned()
        else {
            continue;
        };
        if !core
            .data_directory
            .join("reactions")
            .join(&definition.sound_id)
            .is_file()
        {
            continue;
        }
        let playback = runtime.begin(&settings, definition);
        publish_start(core, playback);
        return;
    }
    let _ = core.relay_tx.send(RelayEvent::Reaction(None));
}

pub async fn stop_without_outputs(core: &AppCore) {
    let mut runtime = core.reactions.lock().await;
    let status = core.server_status.read().await;
    if status.outputs.reaction.obs_clients + status.outputs.reaction.widget_clients == 0 {
        runtime.clear_pending();
        if runtime.active.take().is_some() {
            let _ = core.relay_tx.send(RelayEvent::Reaction(None));
        }
    }
}

pub fn import_sound(directory: &Path, path: &Path) -> Result<String> {
    use lofty::prelude::AudioFile;
    use std::io::Write;
    let bytes = read_sound_path(path)?;
    let tagged = lofty::probe::Probe::new(std::io::Cursor::new(&bytes))
        .guess_file_type()?
        .read()?;
    let duration = tagged.properties().duration();
    if duration.is_zero() || duration > Duration::from_secs_f64(MAX_REACTION_SECONDS) {
        bail!("Reaction sound must last between 0 and 30 seconds.");
    }
    std::fs::create_dir_all(directory)?;
    let id = new_id();
    let destination = directory.join(&id);
    let temporary = directory.join(format!("{id}.tmp"));
    let result = (|| -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result?;
    Ok(id)
}

const MAX_SOUND_BYTES: usize = 15 * 1024 * 1024;
fn read_sound_path(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_SOUND_BYTES as u64 {
        bail!("Reaction sound must be under 15 MB.");
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_SOUND_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > MAX_SOUND_BYTES {
        bail!("Reaction sound must be under 15 MB.");
    }
    Ok(bytes)
}
pub fn read_sound(directory: &Path, id: &str) -> Result<Vec<u8>> {
    if !valid_id(id) {
        bail!("Invalid sound ID.");
    }
    read_sound_path(&directory.join(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definition() -> ReactionDefinition {
        ReactionDefinition {
            id: new_id(),
            name: "Applause".into(),
            sound_id: new_id(),
            visual_id: None,
            volume: 70,
            enabled: true,
        }
    }
    #[test]
    fn access_is_explicit() {
        let mut settings = ReactionSettings {
            enabled: true,
            ..Default::default()
        };
        assert!(!settings.authorize("1", &["2".into()]));
        settings.allowed_channel_ids.push("1".into());
        settings.allowed_role_ids.push("2".into());
        assert!(settings.authorize("1", &["2".into()]));
        assert!(!settings.authorize("3", &["2".into()]));
    }
    #[test]
    fn single_playback_and_local_cooldown_bypass() {
        let settings = ReactionSettings {
            enabled: true,
            ..Default::default()
        };
        let mut runtime = ReactionRuntime::default();
        let now = Instant::now();
        runtime
            .submit(
                &settings,
                definition(),
                Some((1, "1".into(), vec!["2".into()])),
                now,
            )
            .unwrap();
        assert_eq!(
            runtime
                .submit(&settings, definition(), None, now)
                .unwrap()
                .position,
            1
        );
        runtime.clear_pending();
        runtime.active = None;
        assert!(
            runtime
                .submit(
                    &settings,
                    definition(),
                    Some((1, "1".into(), vec!["2".into()])),
                    now
                )
                .is_err()
        );
        runtime.submit(&settings, definition(), None, now).unwrap();
    }

    #[test]
    fn queue_is_bounded_and_rejected_requests_do_not_consume_cooldowns() {
        let settings = ReactionSettings {
            enabled: true,
            ..Default::default()
        };
        let mut runtime = ReactionRuntime::default();
        let now = Instant::now();
        assert_eq!(
            runtime
                .submit(&settings, definition(), None, now)
                .unwrap()
                .position,
            0
        );
        for position in 1..=MAX_PENDING_REACTIONS {
            assert_eq!(
                runtime
                    .submit(&settings, definition(), None, now)
                    .unwrap()
                    .position,
                position
            );
        }
        assert!(
            runtime
                .submit(
                    &settings,
                    definition(),
                    Some((7, "1".into(), vec!["2".into()])),
                    now
                )
                .unwrap_err()
                .to_string()
                .contains("queue is full")
        );
        assert!(!runtime.last_members.contains_key(&7));
        assert_eq!(runtime.pending.len(), MAX_PENDING_REACTIONS);
    }

    async fn fifo_fixture() -> (std::path::PathBuf, Arc<AppCore>, Vec<ReactionDefinition>) {
        let directory = std::env::temp_dir().join(format!("relay-reaction-fifo-{}", new_id()));
        let core = AppCore::load(directory.join("config.json")).unwrap();
        let definitions: Vec<_> = (0..4).map(|_| definition()).collect();
        std::fs::create_dir_all(directory.join("reactions")).unwrap();
        for definition in &definitions {
            std::fs::write(
                directory.join("reactions").join(&definition.sound_id),
                wav(1),
            )
            .unwrap();
        }
        core.config.write().await.reactions = ReactionSettings {
            enabled: true,
            global_cooldown_seconds: 0,
            member_cooldown_seconds: 0,
            allowed_channel_ids: vec!["1".into()],
            allowed_role_ids: vec!["2".into()],
            definitions: definitions.clone(),
            ..Default::default()
        };
        core.server_status
            .write()
            .await
            .outputs
            .reaction
            .widget_clients = 1;
        (directory, core, definitions)
    }

    #[tokio::test]
    async fn fifo_advances_in_order_and_duplicate_completion_cannot_skip_a_sound() {
        let (directory, core, definitions) = fifo_fixture().await;
        let mut events = core.relay_tx.subscribe();
        for (index, definition) in definitions.iter().take(3).enumerate() {
            let result = trigger(
                &core,
                &definition.id,
                Some((index as u64 + 10, "1".into(), vec!["2".into()])),
            )
            .await
            .unwrap();
            assert_eq!(result.position, index);
        }
        assert!(matches!(
            events.try_recv().unwrap(),
            RelayEvent::Reaction(Some(_))
        ));
        assert!(
            events.try_recv().is_err(),
            "queued requests must not interrupt playback"
        );
        let first = core.reactions.lock().await.active.clone().unwrap();
        stop(&core, Some(&first.playback_id)).await;
        let second = core.reactions.lock().await.active.clone().unwrap();
        assert_eq!(second.reaction.id, definitions[1].id);
        assert_ne!(first.playback_id, second.playback_id);
        assert!(matches!(
            events.try_recv().unwrap(),
            RelayEvent::Reaction(Some(_))
        ));
        stop(&core, Some(&first.playback_id)).await;
        assert_eq!(
            core.reactions
                .lock()
                .await
                .active
                .as_ref()
                .unwrap()
                .playback_id,
            second.playback_id
        );
        assert!(events.try_recv().is_err());
        stop(&core, Some(&second.playback_id)).await;
        let third = core.reactions.lock().await.active.clone().unwrap();
        assert_eq!(third.reaction.id, definitions[2].id);
        stop(&core, Some(&third.playback_id)).await;
        assert!(core.reactions.lock().await.active.is_none());
        assert!(core.reactions.lock().await.pending.is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn shared_skip_advances_only_the_current_reaction_and_keeps_the_fifo() {
        let (directory, core, definitions) = fifo_fixture().await;
        for definition in definitions.iter().take(3) {
            trigger(&core, &definition.id, None).await.unwrap();
        }
        let mut events = core.relay_tx.subscribe();
        core.skip_playback().await;
        assert_eq!(
            core.reactions
                .lock()
                .await
                .active
                .as_ref()
                .unwrap()
                .reaction
                .id,
            definitions[1].id
        );
        assert_eq!(core.reactions.lock().await.pending.len(), 1);
        assert!(matches!(
            events.try_recv().unwrap(),
            RelayEvent::Reaction(Some(_))
        ));
        assert!(
            events.try_recv().is_err(),
            "reaction skip must not send music/media skip events"
        );
        stop(&core, None).await;
        let _ = events.try_recv();
        core.skip_playback().await;
        let mut media_skipped = false;
        while let Ok(event) = events.try_recv() {
            if matches!(event, RelayEvent::Skip) {
                media_skipped = true;
            }
        }
        assert!(
            media_skipped,
            "ordinary media skip still works with no active reaction"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn queued_sounds_respect_changes_missing_files_and_manual_stop() {
        let (directory, core, definitions) = fifo_fixture().await;
        for definition in &definitions {
            trigger(&core, &definition.id, None).await.unwrap();
        }
        let first = core.reactions.lock().await.active.clone().unwrap();
        std::fs::remove_file(directory.join("reactions").join(&definitions[1].sound_id)).unwrap();
        {
            let mut config = core.config.write().await;
            config.reactions.definitions[2].enabled = false;
            config.reactions.definitions[3].volume = 35;
        }
        stop(&core, Some(&first.playback_id)).await;
        let current = core.reactions.lock().await.active.clone().unwrap();
        assert_eq!(current.reaction.id, definitions[3].id);
        assert_eq!(current.reaction.volume, 35);
        trigger(&core, &definitions[0].id, None).await.unwrap();
        stop(&core, None).await;
        assert!(core.reactions.lock().await.active.is_none());
        assert!(core.reactions.lock().await.pending.is_empty());
        stop(&core, Some(&current.playback_id)).await;
        assert!(core.reactions.lock().await.active.is_none());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn removed_access_and_disabled_module_discard_queued_reactions() {
        let (directory, core, definitions) = fifo_fixture().await;
        trigger(&core, &definitions[0].id, None).await.unwrap();
        trigger(
            &core,
            &definitions[1].id,
            Some((10, "1".into(), vec!["2".into()])),
        )
        .await
        .unwrap();
        core.config.write().await.reactions.allowed_role_ids.clear();
        let id = core
            .reactions
            .lock()
            .await
            .active
            .as_ref()
            .unwrap()
            .playback_id
            .clone();
        stop(&core, Some(&id)).await;
        assert!(core.reactions.lock().await.active.is_none());
        assert!(core.reactions.lock().await.pending.is_empty());
        trigger(&core, &definitions[0].id, None).await.unwrap();
        trigger(&core, &definitions[1].id, None).await.unwrap();
        core.config.write().await.reactions.enabled = false;
        let id = core
            .reactions
            .lock()
            .await
            .active
            .as_ref()
            .unwrap()
            .playback_id
            .clone();
        stop(&core, Some(&id)).await;
        assert!(core.reactions.lock().await.active.is_none());
        assert!(core.reactions.lock().await.pending.is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn timeout_advances_fifo_and_output_loss_discards_pending_requests() {
        let (directory, core, definitions) = fifo_fixture().await;
        trigger(&core, &definitions[0].id, None).await.unwrap();
        trigger(&core, &definitions[1].id, None).await.unwrap();
        let mut expiring = core.reactions.lock().await.active.clone().unwrap();
        expiring.ends_at = now_ms();
        publish_start(&core, expiring);
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if core
                    .reactions
                    .lock()
                    .await
                    .active
                    .as_ref()
                    .is_some_and(|active| active.reaction.id == definitions[1].id)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        trigger(&core, &definitions[2].id, None).await.unwrap();
        core.server_status
            .write()
            .await
            .outputs
            .reaction
            .widget_clients = 0;
        stop_without_outputs(&core).await;
        assert!(core.reactions.lock().await.active.is_none());
        assert!(core.reactions.lock().await.pending.is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn playback_wire_payload_includes_duration_for_reconnects() {
        let playback = ReactionPlayback {
            playback_id: new_id(),
            reaction: definition(),
            ends_at: now_ms().saturating_add(MAX_REACTION_DURATION_MS),
            music_percent: 25,
        };
        let payload = serde_json::to_value(&playback).unwrap();
        assert_eq!(payload["durationMs"], MAX_REACTION_DURATION_MS);
        assert_eq!(payload["endsAt"], playback.ends_at);
    }

    #[test]
    fn rejects_traversal_and_duplicates() {
        assert!(!valid_id("../../config.json"));
        let item = definition();
        let settings = ReactionSettings {
            definitions: vec![item.clone(), item],
            ..Default::default()
        };
        assert!(settings.validate().is_err());
    }
    fn wav(seconds: u32) -> Vec<u8> {
        wav_millis(seconds * 1000)
    }
    fn wav_millis(milliseconds: u32) -> Vec<u8> {
        let length = milliseconds * 16;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(length + 36).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8000u32.to_le_bytes());
        bytes.extend_from_slice(&16000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.resize(44 + length as usize, 0);
        bytes
    }
    #[test]
    fn sound_import_owns_copy_and_rejects_long_or_oversized_files() {
        let directory = std::env::temp_dir().join(format!("relay-reaction-test-{}", new_id()));
        std::fs::create_dir_all(&directory).unwrap();
        let original = directory.join("original.wav");
        let storage = directory.join("managed");
        std::fs::write(&original, wav(1)).unwrap();
        let id = import_sound(&storage, &original).unwrap();
        std::fs::remove_file(&original).unwrap();
        assert_eq!(read_sound(&storage, &id).unwrap(), wav(1));
        std::fs::write(&original, wav_millis(30_000)).unwrap();
        let maximum_id = import_sound(&storage, &original).unwrap();
        assert_eq!(
            read_sound(&storage, &maximum_id).unwrap(),
            wav_millis(30_000)
        );
        std::fs::write(&original, wav_millis(30_001)).unwrap();
        let error = import_sound(&storage, &original).unwrap_err().to_string();
        assert!(error.contains("30 seconds"));
        assert_eq!(std::fs::read_dir(&storage).unwrap().count(), 2);
        let corrupt = storage.join(new_id());
        std::fs::File::create(&corrupt)
            .unwrap()
            .set_len(MAX_SOUND_BYTES as u64 + 1)
            .unwrap();
        assert!(read_sound(&storage, corrupt.file_name().unwrap().to_str().unwrap()).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[tokio::test]
    async fn stale_completion_does_not_stop_new_playback() {
        let directory = std::env::temp_dir().join(format!("relay-reaction-runtime-{}", new_id()));
        let core = AppCore::load(directory.join("config.json")).unwrap();
        let settings = ReactionSettings {
            enabled: true,
            ..Default::default()
        };
        let playback = core.reactions.lock().await.begin(&settings, definition());
        stop(&core, Some("old-playback")).await;
        assert!(core.reactions.lock().await.active.is_some());
        stop(&core, Some(&playback.playback_id)).await;
        assert!(core.reactions.lock().await.active.is_none());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn concurrent_triggers_queue_and_missing_outputs_do_not_start() {
        let directory =
            std::env::temp_dir().join(format!("relay-reaction-concurrent-{}", new_id()));
        let core = AppCore::load(directory.join("config.json")).unwrap();
        let definition = definition();
        std::fs::create_dir_all(directory.join("reactions")).unwrap();
        std::fs::write(
            directory.join("reactions").join(&definition.sound_id),
            wav(1),
        )
        .unwrap();
        core.config.write().await.reactions = ReactionSettings {
            enabled: true,
            definitions: vec![definition.clone()],
            ..Default::default()
        };
        assert!(
            trigger(&core, &definition.id, None)
                .await
                .unwrap_err()
                .to_string()
                .contains("No reaction output")
        );
        assert!(core.reactions.lock().await.active.is_none());
        core.server_status
            .write()
            .await
            .outputs
            .reaction
            .obs_clients = 1;
        let (first, second) = tokio::join!(
            trigger(&core, &definition.id, None),
            trigger(&core, &definition.id, None)
        );
        let mut positions = [first.unwrap().position, second.unwrap().position];
        positions.sort();
        assert_eq!(positions, [0, 1]);
        stop_without_outputs(&core).await;
        assert!(core.reactions.lock().await.active.is_some());
        core.server_status
            .write()
            .await
            .outputs
            .reaction
            .obs_clients = 0;
        stop_without_outputs(&core).await;
        assert!(core.reactions.lock().await.active.is_none());
        // Discord reactions also run with only the invisible Windows receiver.
        {
            let mut config = core.config.write().await;
            config.reactions.allowed_channel_ids = vec!["1".into()];
            config.reactions.allowed_role_ids = vec!["2".into()];
        }
        core.server_status
            .write()
            .await
            .outputs
            .reaction
            .widget_clients = 1;
        trigger(
            &core,
            &definition.id,
            Some((3, "1".into(), vec!["2".into()])),
        )
        .await
        .unwrap();
        stop_without_outputs(&core).await;
        assert!(core.reactions.lock().await.active.is_some());
        core.server_status
            .write()
            .await
            .outputs
            .reaction
            .widget_clients = 0;
        stop_without_outputs(&core).await;
        assert!(core.reactions.lock().await.active.is_none());
        let lock = core.reactions.lock().await;
        let waiting_core = core.clone();
        let id = definition.id.clone();
        let waiting = tokio::spawn(async move { trigger(&waiting_core, &id, None).await });
        core.config.write().await.reactions.enabled = false;
        drop(lock);
        assert!(
            waiting
                .await
                .unwrap()
                .unwrap_err()
                .to_string()
                .contains("disabled")
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
