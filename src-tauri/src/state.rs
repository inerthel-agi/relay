mod media_cache;

mod message_pin;

mod moderation_queue;

mod music_playback;

pub use message_pin::MessagePinStatus;

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, bail};
use axum::body::Bytes;
use serenity::{cache::Cache, gateway::ShardManager, http::Http};
use tokio::{
    sync::{Mutex, RwLock, broadcast, mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    artwork,
    artwork::EmbeddedArtwork,
    config::{AppConfig, ConfigStore},
    custom_commands::CustomCommandConfirmations,
    media_compat::{self, VideoCompatibility},
    model::{
        BotStatus, ChannelSummary, HistoryEntry, InterfacePreferences, MediaEvent, MediaKind,
        MusicPlaybackEvent, MusicPlaybackMode, MusicStopEvent, PendingMedia, PendingText,
        RelayEvent, ServerStatus, StickerEvent, TtsEvent, VisualSegment,
    },
    music::{MusicSelection, MusicState},
    privacy::{self, PrivacyAction, PrivacyReport},
    stage_scheduler::{StageLane, StageScheduler, StageTicket},
};

pub const HISTORY_LIMIT: usize = 50;
pub const MODERATION_QUEUE_LIMIT: usize = 50;
pub const ARTWORK_CACHE_LIMIT: usize = 50;
pub const MEDIA_AUDIO_CACHE_LIMIT: usize = 50;
pub const MEDIA_AUDIO_CACHE_BYTE_LIMIT: usize = 200 * 1024 * 1024;
pub const PROCESSED_EMBED_LIMIT: usize = 500;
pub const MEDIA_CACHE_ITEM_LIMIT: usize = 30;
pub const MEDIA_CACHE_BYTE_LIMIT: usize = 100 * 1024 * 1024;
const MEDIA_DELIVERY_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub struct TtsAudio {
    pub id: String,
    pub content_type: String,
    pub bytes: Bytes,
}

#[derive(Clone)]
pub struct MediaArtwork {
    pub id: String,
    pub content_type: String,
    pub bytes: Bytes,
}

#[derive(Clone)]
pub struct MediaAudio {
    pub id: String,
    pub content_type: String,
    pub bytes: Bytes,
}

#[derive(Clone)]
pub struct CachedMedia {
    pub id: String,
    pub content_type: String,
    pub bytes: Bytes,
}

pub struct BotRuntime {
    pub shard_manager: Arc<ShardManager>,
    pub task: JoinHandle<()>,
    pub http: Arc<Http>,
    pub cache: Arc<Cache>,
}

pub struct ServerRuntime {
    pub shutdown: tokio::sync::oneshot::Sender<()>,
    pub client_shutdown: broadcast::Sender<()>,
    pub task: JoinHandle<()>,
    pub port: u16,
}

pub struct MediaDeliveryRequest {
    pub kind: MediaKind,
    pub ready: oneshot::Sender<()>,
}

pub struct AppCore {
    pub media_library: Arc<crate::media_library::MediaLibrary>,
    pub data_directory: PathBuf,
    pub reactions: Mutex<crate::reactions::ReactionRuntime>,
    pub reaction_trim: Mutex<crate::reaction_trim::ReactionTrimState>,
    message_pin: Mutex<message_pin::MessagePinRuntime>,
    pub config: RwLock<AppConfig>,
    pub config_store: ConfigStore,
    config_mutation: Mutex<()>,
    tts_pending_count: AtomicUsize,
    pub bot_status: RwLock<BotStatus>,
    pub server_status: RwLock<ServerStatus>,
    pub channels: RwLock<Vec<ChannelSummary>>,
    pub history: RwLock<VecDeque<HistoryEntry>>,
    pub pending_media: RwLock<VecDeque<PendingMedia>>,
    /// Notification messages held for review (Moderation → review queue).
    pub pending_texts: RwLock<VecDeque<PendingText>>,
    pub moderation: std::sync::Mutex<crate::moderation::ModerationRuntime>,
    pub moderation_log: std::sync::Mutex<crate::moderation::log::ModerationLog>,
    pub tts_audio: RwLock<VecDeque<TtsAudio>>,
    pub media_artwork: RwLock<VecDeque<MediaArtwork>>,
    pub media_audio: RwLock<VecDeque<MediaAudio>>,
    pub music: Mutex<MusicState>,
    pub music_cleanup: Mutex<crate::music_cleanup::MusicCleanup>,
    music_stage_tickets: Mutex<HashMap<String, StageTicket>>,
    pub relay_tx: broadcast::Sender<RelayEvent>,
    pub stage_scheduler: StageScheduler,
    pub bot_runtime: Mutex<Option<BotRuntime>>,
    pub custom_command_sync: Mutex<()>,
    pub custom_command_confirmations: Mutex<CustomCommandConfirmations>,
    pub server_runtime: Mutex<Option<ServerRuntime>>,
    pub panel_token: String,
    pub widget_move_generation: AtomicU64,
    pub widget_resize_generation: AtomicU64,
    pub notification_widget_move_generation: AtomicU64,
    pub notification_widget_resize_generation: AtomicU64,
    /// Runtime-only marker for a media widget auto-woken by music playback.
    pub widget_ephemeral_wake: AtomicBool,
    pub interface_preferences: RwLock<InterfacePreferences>,
    pub processed_embed_ids: RwLock<VecDeque<String>>,
    pub cached_media: RwLock<VecDeque<CachedMedia>>,
    pending_privacy_roles: RwLock<HashMap<u64, Vec<String>>>,
    media_delivery: RwLock<Option<mpsc::UnboundedSender<MediaDeliveryRequest>>>,
    next_moderation_id: AtomicU64,
}

impl AppCore {
    pub fn load(config_path: PathBuf) -> Result<Arc<Self>> {
        let data_directory = config_path
            .parent()
            .context("Missing application data directory")?
            .to_path_buf();
        let config_store = ConfigStore::new(config_path);
        let config = config_store.load()?;
        let interface_preferences = config.interface_preferences.clone();
        let (relay_tx, _) = broadcast::channel(2_048);
        let moderation_log_path = data_directory.join("moderation-log.json");
        let stage_scheduler = StageScheduler::new(relay_tx.clone());
        Ok(Arc::new(Self {
            media_library: Arc::new(crate::media_library::MediaLibrary::open_or_unavailable(
                data_directory.join("library"),
            )),
            data_directory,
            reactions: Mutex::new(crate::reactions::ReactionRuntime::default()),
            reaction_trim: Mutex::new(crate::reaction_trim::ReactionTrimState::default()),
            message_pin: Mutex::new(message_pin::MessagePinRuntime::default()),
            config: RwLock::new(config),
            config_store,
            config_mutation: Mutex::new(()),
            tts_pending_count: AtomicUsize::new(0),
            bot_status: RwLock::new(BotStatus::default()),
            server_status: RwLock::new(ServerStatus::default()),
            channels: RwLock::new(Vec::new()),
            history: RwLock::new(VecDeque::with_capacity(HISTORY_LIMIT)),
            pending_media: RwLock::new(VecDeque::with_capacity(MODERATION_QUEUE_LIMIT)),
            pending_texts: RwLock::new(VecDeque::new()),
            moderation: std::sync::Mutex::new(crate::moderation::ModerationRuntime::default()),
            moderation_log: std::sync::Mutex::new(crate::moderation::log::ModerationLog::open(
                moderation_log_path,
                crate::clock::now_ms(),
            )),
            tts_audio: RwLock::new(VecDeque::new()),
            media_artwork: RwLock::new(VecDeque::with_capacity(ARTWORK_CACHE_LIMIT)),
            media_audio: RwLock::new(VecDeque::with_capacity(MEDIA_AUDIO_CACHE_LIMIT)),
            music: Mutex::new(MusicState::default()),
            music_cleanup: Mutex::new(crate::music_cleanup::MusicCleanup::default()),
            music_stage_tickets: Mutex::new(HashMap::new()),
            relay_tx,
            stage_scheduler,
            bot_runtime: Mutex::new(None),
            custom_command_sync: Mutex::new(()),
            custom_command_confirmations: Mutex::new(CustomCommandConfirmations::default()),
            server_runtime: Mutex::new(None),
            panel_token: random_session_token(),
            widget_move_generation: AtomicU64::new(0),
            widget_resize_generation: AtomicU64::new(0),
            notification_widget_move_generation: AtomicU64::new(0),
            notification_widget_resize_generation: AtomicU64::new(0),
            widget_ephemeral_wake: AtomicBool::new(false),
            interface_preferences: RwLock::new(interface_preferences),
            processed_embed_ids: RwLock::new(VecDeque::with_capacity(PROCESSED_EMBED_LIMIT)),
            cached_media: RwLock::new(VecDeque::with_capacity(MEDIA_CACHE_ITEM_LIMIT)),
            pending_privacy_roles: RwLock::new(HashMap::new()),
            media_delivery: RwLock::new(None),
            next_moderation_id: AtomicU64::new(1),
        }))
    }

    pub async fn set_media_delivery(&self, sender: mpsc::UnboundedSender<MediaDeliveryRequest>) {
        *self.media_delivery.write().await = Some(sender);
    }

    pub fn outputs_paused(&self) -> bool {
        self.stage_scheduler.is_paused()
    }

    pub async fn set_outputs_paused(&self, paused: bool) {
        self.stage_scheduler.set_paused(paused);
        let _ = self.relay_tx.send(RelayEvent::OutputsPaused(paused));
    }

    pub async fn register_stage_output(
        &self,
        timestamp_ms: u64,
        message_id: &str,
        part: u16,
        lane: StageLane,
    ) -> StageTicket {
        self.stage_scheduler
            .reserve(timestamp_ms, message_id, part, lane)
            .await
    }

    pub async fn complete_media(&self, ticket: StageTicket, event: MediaEvent) {
        self.stage_scheduler
            .ready(ticket, RelayEvent::Media(event))
            .await;
    }

    pub async fn complete_sticker(&self, ticket: StageTicket, event: StickerEvent) {
        self.stage_scheduler
            .ready(ticket, RelayEvent::Sticker(event))
            .await;
    }

    pub async fn complete_tts(&self, ticket: StageTicket, event: TtsEvent) {
        self.remember_authoritative_tts(&event).await;
        self.stage_scheduler
            .ready(ticket, RelayEvent::Tts(event))
            .await;
    }

    pub async fn complete_music(&self, ticket: StageTicket, event: MusicPlaybackEvent) {
        self.stage_scheduler
            .ready(ticket, RelayEvent::MusicPlay(event))
            .await;
    }

    pub async fn cancel_stage_output(&self, ticket: StageTicket) {
        self.stage_scheduler.cancel(ticket).await;
    }

    pub async fn set_config(&self, config: AppConfig) -> Result<()> {
        let _mutation = self.config_mutation.lock().await;
        self.persist_config(config).await
    }

    /// Applies a partial mutation while holding the mutation lock across the
    /// read-modify-save cycle, so concurrent writers cannot lose updates.
    pub async fn update_config<F>(&self, mutate: F) -> Result<AppConfig>
    where
        F: FnOnce(&mut AppConfig),
    {
        let _mutation = self.config_mutation.lock().await;
        let mut config = self.config.read().await.clone();
        mutate(&mut config);
        self.persist_config(config.clone()).await?;
        Ok(config)
    }

    async fn persist_config(&self, config: AppConfig) -> Result<()> {
        config.validate()?;
        let store = self.config_store.clone();
        let to_save = config.clone();
        tokio::task::spawn_blocking(move || store.save(&to_save)).await??;
        *self.config.write().await = config.clone();
        let mut pending = self.pending_media.write().await;
        if config.moderation_enabled {
            pending.retain(|item| {
                item.privacy_classification.is_some()
                    || item.hold_reason.is_some()
                    || media_type_allowed(&config, item.media.kind)
            });
        } else {
            // Privacy and moderation holds are independent from manual review
            // and must remain actionable when the manual switch is turned off.
            pending
                .retain(|item| item.privacy_classification.is_some() || item.hold_reason.is_some());
        }
        drop(pending);
        let pending_ids = self
            .pending_media
            .read()
            .await
            .iter()
            .map(|item| item.id)
            .collect::<std::collections::HashSet<_>>();
        self.pending_privacy_roles
            .write()
            .await
            .retain(|pending_id, _| pending_ids.contains(pending_id));
        let _ = self.relay_tx.send(RelayEvent::Config(Box::new(config)));
        Ok(())
    }

    #[cfg(test)]
    pub async fn submit_media(&self, media: MediaEvent) {
        self.submit_analyzed_media_with_text(media, None, None)
            .await;
    }

    #[cfg(test)]
    pub async fn submit_analyzed_media(&self, media: MediaEvent, report: Option<PrivacyReport>) {
        self.submit_analyzed_media_with_text(media, report, None)
            .await;
    }

    #[cfg(test)]
    pub async fn submit_analyzed_media_with_text(
        &self,
        media: MediaEvent,
        report: Option<PrivacyReport>,
        full_text: Option<&str>,
    ) {
        self.submit_analyzed_media_with_text_and_roles(media, report, full_text, &[])
            .await;
    }

    pub async fn submit_analyzed_media_with_text_and_roles(
        &self,
        media: MediaEvent,
        report: Option<PrivacyReport>,
        full_text: Option<&str>,
        role_ids: &[String],
    ) {
        let ticket = self
            .register_stage_output(media.timestamp, &media.message_id, 200, StageLane::Media)
            .await;
        self.submit_analyzed_media_with_ticket_and_roles(
            ticket, media, report, full_text, role_ids,
        )
        .await;
    }

    pub async fn submit_analyzed_media_with_ticket_and_roles(
        &self,
        ticket: StageTicket,
        media: MediaEvent,
        report: Option<PrivacyReport>,
        full_text: Option<&str>,
        role_ids: &[String],
    ) {
        let config = self.config.read().await.clone();
        let scoped_config = crate::moderation::scope_config(
            privacy::scoped_config_for_roles(&config, role_ids),
            crate::moderation::Lane::Media,
        );
        let role_exempt = privacy::has_exempt_role(&config, role_ids);
        let analysis_text = privacy_media_text(&media, full_text);
        // Always classify the message text again at this boundary. The bot's
        // asynchronous report is advisory, while this state gate is the last
        // decision point before history and relay publication.
        let mut authoritative_report = privacy::classify_text(Some(&analysis_text), &scoped_config);
        if let Some(report) = report {
            let mut report = if role_exempt {
                report.without_filter_signals()
            } else {
                report
            };
            if role_exempt {
                report.config_signature = Some(privacy::config_signature(&scoped_config));
            }
            let report_is_stale = privacy::privacy_rules_enabled(&scoped_config)
                && report
                    .config_signature
                    .is_none_or(|signature| signature != privacy::config_signature(&scoped_config));
            authoritative_report.merge(report);
            if report_is_stale
                && authoritative_report.classification == privacy::PrivacyClassification::Safe
            {
                authoritative_report.merge(PrivacyReport::suspicious("scan_config_changed"));
            }
        } else if config.privacy_scan_enabled
            && media_requires_image_scan(&media)
            && authoritative_report.classification == privacy::PrivacyClassification::Safe
        {
            authoritative_report = PrivacyReport::suspicious("image_scan_unavailable");
        }
        authoritative_report.apply_score_policy(&scoped_config);
        let privacy_action = if privacy::privacy_rules_enabled(&scoped_config) {
            privacy::action_for(&authoritative_report, &scoped_config)
        } else {
            PrivacyAction::Allow
        };
        privacy::log_decision(&authoritative_report, privacy_action);
        let verdict = self.moderation_verdict(&media.message_id);
        if matches!(privacy_action, PrivacyAction::Block) {
            self.record_moderation(
                Some(crate::moderation::Lane::Media),
                crate::moderation::log::LogAction::Blocked,
                authoritative_report.primary_reason().unwrap_or("privacy"),
                verdict.map(|verdict| verdict.author_id),
            );
            self.cancel_stage_output(ticket).await;
            return;
        }
        let Some(route) = review_route(&config, media.kind, privacy_action, verdict.as_ref())
        else {
            self.cancel_stage_output(ticket).await;
            return;
        };
        if let ReviewRoute::Publish = route {
            drop(config);
            self.publish_media_with_ticket(ticket, media).await;
            return;
        }
        self.cancel_stage_output(ticket).await;
        self.enqueue_pending(moderation_queue::PendingEntry {
            media,
            sticker: None,
            sticker_bytes: None,
            report: &authoritative_report,
            hold_reason: route.hold_reason(),
            release_at: route.release_at(),
            role_ids,
        })
        .await;
    }

    #[cfg(test)]
    pub async fn submit_sticker(
        &self,
        sticker: StickerEvent,
        text: Option<&str>,
        bytes: Option<Vec<u8>>,
        report: Option<PrivacyReport>,
    ) {
        self.submit_sticker_with_roles(sticker, text, bytes, report, &[])
            .await;
    }

    #[cfg(test)]
    pub async fn submit_sticker_with_roles(
        &self,
        sticker: StickerEvent,
        text: Option<&str>,
        bytes: Option<Vec<u8>>,
        report: Option<PrivacyReport>,
        role_ids: &[String],
    ) {
        let ticket = self
            .register_stage_output(
                sticker.timestamp,
                &sticker.message_id,
                100,
                StageLane::Media,
            )
            .await;
        self.submit_sticker_with_ticket_and_roles(ticket, sticker, text, bytes, report, role_ids)
            .await;
    }

    pub async fn submit_sticker_with_ticket_and_roles(
        &self,
        ticket: StageTicket,
        mut sticker: StickerEvent,
        text: Option<&str>,
        bytes: Option<Vec<u8>>,
        report: Option<PrivacyReport>,
        role_ids: &[String],
    ) {
        let config = self.config.read().await.clone();
        let scoped_config = crate::moderation::scope_config(
            privacy::scoped_config_for_roles(&config, role_ids),
            crate::moderation::Lane::Media,
        );
        let role_exempt = privacy::has_exempt_role(&config, role_ids);
        let analysis_text = format!("{}\n{}", text.unwrap_or_default(), sticker.name);
        let mut authoritative_report = privacy::classify_text(Some(&analysis_text), &scoped_config);
        if let Some(report) = report {
            let mut report = if role_exempt {
                report.without_filter_signals()
            } else {
                report
            };
            if role_exempt {
                report.config_signature = Some(privacy::config_signature(&scoped_config));
            }
            let report_is_stale = privacy::privacy_rules_enabled(&scoped_config)
                && report
                    .config_signature
                    .is_none_or(|signature| signature != privacy::config_signature(&scoped_config));
            authoritative_report.merge(report);
            if report_is_stale
                && authoritative_report.classification == privacy::PrivacyClassification::Safe
            {
                authoritative_report.merge(PrivacyReport::suspicious("scan_config_changed"));
            }
        } else if config.privacy_scan_enabled {
            match bytes.as_deref() {
                Some(bytes) => authoritative_report.merge(
                    privacy::analyze_image_bytes_async(bytes, Some(&analysis_text), &scoped_config)
                        .await,
                ),
                None => authoritative_report.merge(PrivacyReport::suspicious("scan_incomplete")),
            }
        }
        if config.privacy_scan_enabled
            && bytes.is_none()
            && authoritative_report.classification == privacy::PrivacyClassification::Safe
        {
            authoritative_report.merge(PrivacyReport::suspicious("scan_incomplete"));
        }
        authoritative_report.apply_score_policy(&scoped_config);
        let privacy_action = if privacy::privacy_rules_enabled(&scoped_config) {
            privacy::action_for(&authoritative_report, &scoped_config)
        } else {
            PrivacyAction::Allow
        };
        privacy::log_decision(&authoritative_report, privacy_action);
        let verdict = self.moderation_verdict(&sticker.message_id);
        if matches!(privacy_action, PrivacyAction::Block) {
            self.record_moderation(
                Some(crate::moderation::Lane::Media),
                crate::moderation::log::LogAction::Blocked,
                authoritative_report.primary_reason().unwrap_or("privacy"),
                verdict.map(|verdict| verdict.author_id),
            );
            self.cancel_stage_output(ticket).await;
            return;
        }
        let media_kind = if sticker.format == "gif" {
            MediaKind::Gif
        } else {
            MediaKind::Image
        };
        let content_type = match sticker.format.as_str() {
            "gif" => "image/gif",
            "apng" => "image/apng",
            _ => "image/png",
        };
        let media = MediaEvent {
            kind: media_kind,
            url: sticker.url.clone(),
            proxy_url: sticker.url.clone(),
            filename: sticker.name.clone(),
            content_type: content_type.into(),
            artwork_id: None,
            audio_id: None,
            cached_media_id: None,
            title: None,
            artist: None,
            text: None,
            author: sticker.author.clone(),
            timestamp: sticker.timestamp,
            message_id: sticker.message_id.clone(),
        };
        let Some(route) = review_route(&config, media.kind, privacy_action, verdict.as_ref())
        else {
            self.cancel_stage_output(ticket).await;
            return;
        };
        if !matches!(route, ReviewRoute::Publish) {
            self.cancel_stage_output(ticket).await;
            self.enqueue_pending(moderation_queue::PendingEntry {
                media,
                sticker: Some(sticker),
                sticker_bytes: bytes.map(Arc::new),
                report: &authoritative_report,
                hold_reason: route.hold_reason(),
                release_at: route.release_at(),
                role_ids,
            })
            .await;
            return;
        }
        if let Some(bytes) = bytes {
            let cache_id = format!("sticker-{}", sticker.id);
            self.cache_media(cache_id.clone(), content_type.into(), bytes)
                .await;
            sticker.cached_media_id = Some(cache_id);
        }
        self.publish_sticker_with_ticket(ticket, sticker).await;
    }

    pub async fn approve_media(&self, id: u64) -> bool {
        let approved = self.approve_media_checked(id, true).await;
        if approved {
            self.record_moderation(
                Some(crate::moderation::Lane::Media),
                crate::moderation::log::LogAction::Approved,
                "manual_review",
                None,
            );
        }
        approved
    }

    /// `rescan` is false for safety-delayed items, which were already checked.
    pub(crate) async fn approve_media_checked(&self, id: u64, rescan: bool) -> bool {
        let item = {
            let pending = self.pending_media.read().await;
            pending.iter().find(|item| item.id == id).cloned()
        };
        let Some(item) = item else {
            return false;
        };
        let config = self.config.read().await.clone();
        let role_ids = self
            .pending_privacy_roles
            .read()
            .await
            .get(&item.id)
            .cloned()
            .unwrap_or_default();
        let scoped_config = crate::moderation::scope_config(
            privacy::scoped_config_for_roles(&config, &role_ids),
            crate::moderation::Lane::Media,
        );
        if rescan && privacy::privacy_rules_enabled(&scoped_config) {
            let analysis_text = privacy_media_text(&item.media, None);
            let mut report = if scoped_config.privacy_scan_enabled
                && media_requires_image_scan(&item.media)
            {
                if let Some(bytes) = item.sticker_bytes.as_deref() {
                    privacy::analyze_image_bytes_async(bytes, Some(&analysis_text), &scoped_config)
                        .await
                } else {
                    privacy::analyze_remote_image(
                        &item.media.url,
                        &item.media.proxy_url,
                        Some(&analysis_text),
                        &scoped_config,
                    )
                    .await
                }
            } else {
                privacy::classify_text(Some(&analysis_text), &scoped_config)
            };
            if scoped_config.privacy_scan_enabled
                && let Some(artwork_id) = item.media.artwork_id.as_deref()
            {
                if let Some(bytes) = self.cached_artwork_bytes(artwork_id).await {
                    report.merge(
                        privacy::analyze_image_bytes_async(
                            &bytes,
                            Some(&analysis_text),
                            &scoped_config,
                        )
                        .await,
                    );
                } else {
                    report.merge(PrivacyReport::low("scan_incomplete"));
                }
            }
            report.apply_score_policy(&scoped_config);
            let action = privacy::action_for(&report, &scoped_config);
            if matches!(action, PrivacyAction::Block) {
                privacy::log_decision(&report, action);
                return false;
            }
        }
        let Some((media, sticker, sticker_bytes)) = ({
            let mut pending = self.pending_media.write().await;
            pending
                .iter()
                .position(|pending_item| pending_item.id == id)
                .and_then(|index| pending.remove(index))
                .map(|pending_item| {
                    (
                        pending_item.media,
                        pending_item.sticker,
                        pending_item.sticker_bytes,
                    )
                })
        }) else {
            return false;
        };
        self.pending_privacy_roles.write().await.remove(&item.id);
        if let Some(mut sticker_event) = sticker {
            if let Some(bytes) = sticker_bytes {
                let cache_id = format!("sticker-{}", sticker_event.id);
                self.cache_media(
                    cache_id.clone(),
                    media.content_type.clone(),
                    (*bytes).clone(),
                )
                .await;
                sticker_event.cached_media_id = Some(cache_id);
            }
            self.publish_sticker(sticker_event).await;
            return true;
        }
        self.publish_media(media).await;
        true
    }

    pub async fn reject_media(&self, id: u64) -> bool {
        let mut pending = self.pending_media.write().await;
        let Some(index) = pending.iter().position(|item| item.id == id) else {
            return false;
        };
        let Some(item) = pending.remove(index) else {
            return false;
        };
        drop(pending);
        self.pending_privacy_roles.write().await.remove(&item.id);
        self.record_moderation(
            Some(crate::moderation::Lane::Media),
            crate::moderation::log::LogAction::Rejected,
            item.hold_reason
                .as_deref()
                .or(item.privacy_reason.as_deref())
                .unwrap_or("manual_review"),
            item.author_id,
        );
        true
    }

    pub async fn clear_pending_media(&self) {
        let rejected =
            self.pending_media.read().await.len() + self.pending_texts.read().await.len();
        self.pending_media.write().await.clear();
        self.pending_texts.write().await.clear();
        self.pending_privacy_roles.write().await.clear();
        for _ in 0..rejected {
            self.record_moderation(
                None,
                crate::moderation::log::LogAction::Rejected,
                "reject_all",
                None,
            );
        }
    }

    pub async fn replay_media_event(&self, mut event: MediaEvent) -> Result<()> {
        let initial_config = self.config.read().await.clone();
        let analysis_text = privacy_media_text(&event, None);
        let mut scanned_bytes = None;
        if privacy::privacy_rules_enabled(&initial_config) {
            let mut report = if media_requires_image_scan(&event) {
                if let Some(cache_id) = event.cached_media_id.as_deref() {
                    if let Some(bytes) = self.cached_media_bytes(cache_id).await {
                        privacy::analyze_image_bytes_async(
                            &bytes,
                            Some(&analysis_text),
                            &initial_config,
                        )
                        .await
                    } else {
                        let bytes = download_replay_bytes(&event).await;
                        if let Some(bytes) = bytes {
                            let report = privacy::analyze_image_bytes_async(
                                &bytes,
                                Some(&analysis_text),
                                &initial_config,
                            )
                            .await;
                            scanned_bytes = Some(bytes);
                            report
                        } else {
                            privacy::analyze_remote_image(
                                &event.url,
                                &event.proxy_url,
                                Some(&analysis_text),
                                &initial_config,
                            )
                            .await
                        }
                    }
                } else {
                    let bytes = download_replay_bytes(&event).await;
                    if let Some(bytes) = bytes {
                        let report = privacy::analyze_image_bytes_async(
                            &bytes,
                            Some(&analysis_text),
                            &initial_config,
                        )
                        .await;
                        scanned_bytes = Some(bytes);
                        report
                    } else {
                        privacy::analyze_remote_image(
                            &event.url,
                            &event.proxy_url,
                            Some(&analysis_text),
                            &initial_config,
                        )
                        .await
                    }
                }
            } else {
                privacy::classify_text(Some(&analysis_text), &initial_config)
            };
            if initial_config.privacy_scan_enabled
                && let Some(artwork_id) = event.artwork_id.as_deref()
            {
                if let Some(bytes) = self.cached_artwork_bytes(artwork_id).await {
                    report.merge(
                        privacy::analyze_image_bytes_async(
                            &bytes,
                            Some(&analysis_text),
                            &initial_config,
                        )
                        .await,
                    );
                } else {
                    report.merge(PrivacyReport::low("scan_incomplete"));
                }
            }
            let current_config = self.config.read().await.clone();
            if !privacy::privacy_rules_enabled(&current_config) {
                // The user explicitly disabled scanning while the async scan
                // was in flight; preserve the documented bypass behavior.
            } else {
                let current_signature = privacy::config_signature(&current_config);
                if report.config_signature != Some(current_signature)
                    && report.classification == privacy::PrivacyClassification::Safe
                {
                    report.merge(PrivacyReport::suspicious("scan_config_changed"));
                }
                report.apply_score_policy(&current_config);
                let action = privacy::action_for(&report, &current_config);
                privacy::log_decision(&report, action);
                match action {
                    PrivacyAction::Allow => {}
                    PrivacyAction::Review => {
                        bail!("Media requires privacy review before replay.");
                    }
                    PrivacyAction::Block => {
                        bail!("Media blocked by the local privacy scan.");
                    }
                }
            }
        }
        if matches!(event.kind, MediaKind::Gif)
            && event.cached_media_id.is_none()
            && scanned_bytes.is_none()
        {
            let bytes = download_replay_bytes(&event).await;
            if let Some(bytes) = bytes {
                scanned_bytes = Some(bytes);
            }
        }
        if let Some(bytes) = scanned_bytes {
            if event.cached_media_id.is_none() {
                let cache_id = format!("{}-replay", event.message_id);
                self.cache_media(cache_id.clone(), event.content_type.clone(), bytes)
                    .await;
                event.cached_media_id = Some(cache_id);
            } else {
                drop(bytes);
            }
        }
        self.prepare_video_compatibility(&mut event).await;
        self.stage_scheduler
            .enqueue(RelayEvent::Media(event), StageLane::Media)
            .await;
        Ok(())
    }

    pub async fn publish_media(&self, media: MediaEvent) {
        let ticket = self
            .register_stage_output(media.timestamp, &media.message_id, 200, StageLane::Media)
            .await;
        self.publish_media_with_ticket(ticket, media).await;
    }

    pub async fn publish_media_with_ticket(&self, ticket: StageTicket, mut media: MediaEvent) {
        self.prepare_video_compatibility(&mut media).await;
        self.prepare_media_delivery(media.kind).await;
        {
            let mut history = self.history.write().await;
            history.push_front(HistoryEntry::Media(media.clone()));
            history.truncate(HISTORY_LIMIT);
        }
        self.complete_media(ticket, media).await;
    }

    async fn prepare_video_compatibility(&self, media: &mut MediaEvent) {
        let is_video = matches!(media.kind, MediaKind::Video)
            || (matches!(media.kind, MediaKind::Gif)
                && media
                    .content_type
                    .to_ascii_lowercase()
                    .starts_with("video/"));
        if !is_video {
            return;
        }
        let cached_media_available = if let Some(cache_id) = media.cached_media_id.as_deref() {
            self.cached_media
                .read()
                .await
                .iter()
                .any(|item| item.id == cache_id)
        } else {
            false
        };
        if cached_media_available {
            return;
        }
        media.cached_media_id = None;

        if let Some(id) = library_asset_id(media) {
            let library = self.media_library.clone();
            let asset = tokio::task::spawn_blocking(move || library.read_asset(&id))
                .await
                .ok()
                .and_then(Result::ok);
            if let Some(asset) = asset {
                let compatibility = media_compat::make_webview_compatible_from_bytes(
                    asset.bytes,
                    &asset.item.name,
                    &asset.content_type,
                )
                .await;
                self.apply_video_compatibility(media, compatibility).await;
            }
            return;
        }

        let compatibility = media_compat::make_webview_compatible(
            &media.url,
            &media.proxy_url,
            &media.filename,
            &media.content_type,
        )
        .await;
        self.apply_video_compatibility(media, compatibility).await;
    }

    async fn apply_video_compatibility(
        &self,
        media: &mut MediaEvent,
        compatibility: VideoCompatibility,
    ) {
        match compatibility {
            VideoCompatibility::Unchanged => {}
            VideoCompatibility::Transcoded(bytes) => {
                let cache_id = format!(
                    "h264-{:016x}{:016x}",
                    rand::random::<u64>(),
                    rand::random::<u64>()
                );
                self.cache_media(cache_id.clone(), "video/mp4".into(), bytes)
                    .await;
                media.content_type = "video/mp4".into();
                media.cached_media_id = Some(cache_id);
                eprintln!("[media] Codec: HEVC Action: TRANSCODED_LOCAL");
            }
            VideoCompatibility::HevcFallback => {
                eprintln!("[media] Codec: HEVC Action: SOURCE_FALLBACK");
            }
        }
    }

    async fn prepare_media_delivery(&self, kind: MediaKind) {
        let Some(sender) = self.media_delivery.read().await.clone() else {
            return;
        };
        let (ready, receiver) = oneshot::channel();
        if sender.send(MediaDeliveryRequest { kind, ready }).is_ok() {
            let _ = tokio::time::timeout(MEDIA_DELIVERY_TIMEOUT, receiver).await;
        }
    }

    pub fn tts_pending_count(&self) -> usize {
        self.tts_pending_count.load(Ordering::SeqCst)
    }

    async fn publish_sticker(&self, sticker: StickerEvent) {
        let ticket = self
            .register_stage_output(
                sticker.timestamp,
                &sticker.message_id,
                100,
                StageLane::Media,
            )
            .await;
        self.publish_sticker_with_ticket(ticket, sticker).await;
    }

    async fn publish_sticker_with_ticket(&self, ticket: StageTicket, sticker: StickerEvent) {
        self.complete_sticker(ticket, sticker).await;
    }

    #[cfg(test)]
    pub async fn publish_visual_tts(
        &self,
        id: String,
        text: String,
        author: crate::model::AuthorIdentity,
        guild_tag: Option<crate::model::GuildTagIdentity>,
        timestamp: u64,
        segments: Vec<VisualSegment>,
    ) {
        let ticket = self
            .register_stage_output(timestamp, &id, 0, StageLane::Tts)
            .await;
        self.publish_visual_tts_with_ticket(
            ticket, id, text, author, guild_tag, timestamp, segments,
        )
        .await;
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn publish_visual_tts_with_ticket(
        &self,
        ticket: StageTicket,
        id: String,
        text: String,
        author: crate::model::AuthorIdentity,
        guild_tag: Option<crate::model::GuildTagIdentity>,
        timestamp: u64,
        segments: Vec<VisualSegment>,
    ) {
        let event = TtsEvent {
            id,
            text,
            author,
            guild_tag,
            content_type: String::new(),
            timestamp,
            visual_only: true,
            segments,
        };
        self.complete_tts(ticket, event).await;
    }

    #[cfg(test)]
    pub async fn publish_visual_tts_if_allowed(
        &self,
        id: String,
        text: String,
        author: crate::model::AuthorIdentity,
        guild_tag: Option<crate::model::GuildTagIdentity>,
        timestamp: u64,
        segments: Vec<VisualSegment>,
    ) -> bool {
        self.publish_visual_tts_if_allowed_with_roles(
            id,
            text,
            author,
            guild_tag,
            timestamp,
            segments,
            &[],
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub async fn publish_visual_tts_if_allowed_with_roles(
        &self,
        id: String,
        text: String,
        author: crate::model::AuthorIdentity,
        guild_tag: Option<crate::model::GuildTagIdentity>,
        timestamp: u64,
        segments: Vec<VisualSegment>,
        role_ids: &[String],
    ) -> bool {
        let ticket = self
            .register_stage_output(timestamp, &id, 0, StageLane::Tts)
            .await;
        self.publish_visual_tts_if_allowed_with_ticket_and_roles(
            ticket, id, text, author, guild_tag, timestamp, segments, role_ids,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn publish_visual_tts_if_allowed_with_ticket_and_roles(
        &self,
        ticket: StageTicket,
        id: String,
        text: String,
        author: crate::model::AuthorIdentity,
        guild_tag: Option<crate::model::GuildTagIdentity>,
        timestamp: u64,
        segments: Vec<VisualSegment>,
        role_ids: &[String],
    ) -> bool {
        let config = self.config.read().await.clone();
        let scoped_config = crate::moderation::scope_config(
            privacy::scoped_config_for_roles(&config, role_ids),
            crate::moderation::Lane::Notifications,
        );
        let verdict = self.moderation_verdict(&id);
        let mut hold_reason = verdict
            .as_ref()
            .and_then(|verdict| verdict.hold)
            .map(str::to_owned);
        if privacy::privacy_rules_enabled(&scoped_config) {
            let report = classify_tts_privacy(&text, &segments, &scoped_config);
            let action = privacy::action_for(&report, &scoped_config);
            if !matches!(action, PrivacyAction::Allow) {
                privacy::log_decision(&report, action);
                let reason = report.primary_reason().unwrap_or("privacy");
                if matches!(action, PrivacyAction::Block) || !config.moderation.review_text {
                    self.record_moderation(
                        Some(crate::moderation::Lane::Notifications),
                        if matches!(action, PrivacyAction::Block) {
                            crate::moderation::log::LogAction::Blocked
                        } else {
                            crate::moderation::log::LogAction::Ignored
                        },
                        reason,
                        verdict.map(|verdict| verdict.author_id),
                    );
                    self.cancel_stage_output(ticket).await;
                    return false;
                }
                hold_reason = Some(reason.to_owned());
            }
        }
        if let Some(reason) = hold_reason {
            self.cancel_stage_output(ticket).await;
            self.hold_text(id, text, author, guild_tag, timestamp, segments, &reason)
                .await;
            return false;
        }
        self.publish_visual_tts_with_ticket(
            ticket, id, text, author, guild_tag, timestamp, segments,
        )
        .await;
        true
    }
}

const MAX_TTS_PRIVACY_SEGMENTS: usize = 64;
const MAX_TTS_PRIVACY_COMPOSITE_CHARS: usize = privacy::PRIVACY_TEXT_LIMIT + 1;

fn classify_tts_privacy(
    text: &str,
    segments: &[VisualSegment],
    config: &AppConfig,
) -> privacy::PrivacyReport {
    let mut composite = String::new();
    let mut truncated = false;
    append_tts_privacy_field(&mut composite, text, &mut truncated);
    for segment in segments.iter().take(MAX_TTS_PRIVACY_SEGMENTS) {
        append_tts_privacy_field(&mut composite, &segment.value, &mut truncated);
    }
    if segments.len() > MAX_TTS_PRIVACY_SEGMENTS {
        truncated = true;
    }
    let mut report = privacy::classify_text(Some(&composite), config);
    if truncated {
        report.merge(privacy::PrivacyReport::suspicious("scan_incomplete"));
    }
    report
}

fn append_tts_privacy_field(target: &mut String, value: &str, truncated: &mut bool) {
    if value.is_empty() {
        return;
    }
    if !target.is_empty() {
        append_tts_privacy_bounded(target, "\n", truncated);
    }
    append_tts_privacy_bounded(target, value, truncated);
}

fn append_tts_privacy_bounded(target: &mut String, value: &str, truncated: &mut bool) {
    let current = target.chars().count();
    if current >= MAX_TTS_PRIVACY_COMPOSITE_CHARS {
        *truncated = true;
        return;
    }
    let remaining = MAX_TTS_PRIVACY_COMPOSITE_CHARS - current;
    let mut appended = 0;
    for character in value.chars().take(remaining) {
        target.push(character);
        appended += 1;
    }
    if appended < value.chars().count() {
        *truncated = true;
    }
}

fn privacy_media_text(media: &MediaEvent, full_text: Option<&str>) -> String {
    let mut fields = Vec::with_capacity(7);
    if let Some(text) = full_text.filter(|text| !text.trim().is_empty()) {
        fields.push(text.to_owned());
    }
    if let Some(text) = media.text.as_deref().filter(|text| !text.trim().is_empty()) {
        fields.push(text.to_owned());
    }
    if !media.filename.trim().is_empty() {
        fields.push(media.filename.clone());
        if let Some(stem) = media.filename.rsplit_once('.').map(|(stem, _)| stem)
            && !stem.trim().is_empty()
        {
            fields.push(stem.to_owned());
        }
    }
    if let Some(title) = media
        .title
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        fields.push(title.to_owned());
    }
    if let Some(artist) = media
        .artist
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        fields.push(artist.to_owned());
    }
    fields.join("\n")
}

async fn download_replay_bytes(event: &MediaEvent) -> Option<Vec<u8>> {
    match artwork::download_bounded(&event.url, artwork::MAX_EMBED_MEDIA_BYTES).await {
        Ok(bytes) => Some(bytes),
        Err(_) if event.proxy_url != event.url => {
            artwork::download_bounded(&event.proxy_url, artwork::MAX_EMBED_MEDIA_BYTES)
                .await
                .ok()
        }
        Err(_) => None,
    }
}

/// Returns an ID only for events produced by the local library replay path.
/// Discord supplied URLs must continue through the HTTPS bounded downloader;
/// this guard must never turn an arbitrary loopback URL into a local file read.
fn library_asset_id(media: &MediaEvent) -> Option<String> {
    let id = media.message_id.strip_prefix("library-")?;
    if !crate::media_library::is_valid_id(id) || media.proxy_url != media.url {
        return None;
    }
    let url = reqwest::Url::parse(&media.url).ok()?;
    if url.scheme() != "http"
        || url.host_str() != Some("127.0.0.1")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let path_id = url.path().strip_prefix("/library-asset/")?;
    (path_id == id).then_some(id.to_owned())
}

/// Where a media item goes after the privacy scan allowed or reviewed it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReviewRoute {
    Publish,
    Review(Option<&'static str>),
    Delay(u64),
}

impl ReviewRoute {
    fn hold_reason(self) -> Option<String> {
        match self {
            Self::Review(reason) => reason.map(str::to_owned),
            Self::Delay(_) => Some("safety_delay".into()),
            Self::Publish => None,
        }
    }

    fn release_at(self) -> Option<u64> {
        match self {
            Self::Delay(at) => Some(at),
            _ => None,
        }
    }
}

/// `None` drops the item: manual review is on and its type is not reviewed.
fn review_route(
    config: &AppConfig,
    kind: MediaKind,
    privacy_action: PrivacyAction,
    verdict: Option<&crate::moderation::MessageVerdict>,
) -> Option<ReviewRoute> {
    let trusted = verdict.is_some_and(|verdict| verdict.trusted);
    let hold = verdict.and_then(|verdict| verdict.hold);
    if matches!(privacy_action, PrivacyAction::Review) || hold.is_some() {
        return Some(ReviewRoute::Review(hold));
    }
    let manually_moderated = config.moderation_enabled && !trusted;
    let type_allowed = media_type_allowed(config, kind);
    if manually_moderated && type_allowed {
        return Some(ReviewRoute::Review(None));
    }
    if manually_moderated && !config.moderation.review_only_selected {
        return None;
    }
    let delay = config.moderation.safety_delay_seconds;
    if delay > 0 && !trusted {
        return Some(ReviewRoute::Delay(
            crate::clock::now_ms() + u64::from(delay) * 1_000,
        ));
    }
    Some(ReviewRoute::Publish)
}

fn media_type_allowed(config: &AppConfig, kind: MediaKind) -> bool {
    match kind {
        MediaKind::Image | MediaKind::Gif => config.moderation_allow_images,
        MediaKind::Video => config.moderation_allow_videos,
        MediaKind::Audio => config.moderation_allow_audio,
    }
}

fn media_requires_image_scan(media: &MediaEvent) -> bool {
    matches!(media.kind, MediaKind::Image | MediaKind::Gif)
        && media
            .content_type
            .to_ascii_lowercase()
            .starts_with("image/")
}

fn random_session_token() -> String {
    use rand::{RngCore, rng};

    let mut bytes = [0_u8; 32];
    rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
