//! Review queue helpers shared by media, stickers and held text, plus the
//! decision log and the safety-delay/expiry tick.

use super::*;
use crate::clock::now_ms;
use crate::model::{AuthorIdentity, GuildTagIdentity, PendingText};
use crate::moderation::log::{LogAction, LogEntry, LogSummary};
use crate::moderation::{Lane, MessageVerdict};

pub(super) struct PendingEntry<'a> {
    pub media: MediaEvent,
    pub sticker: Option<StickerEvent>,
    pub sticker_bytes: Option<Arc<Vec<u8>>>,
    pub report: &'a PrivacyReport,
    pub hold_reason: Option<String>,
    pub release_at: Option<u64>,
    pub role_ids: &'a [String],
}

impl AppCore {
    /// Adds a line to the local decision log (never message content).
    pub fn record_moderation(
        &self,
        lane: Option<Lane>,
        action: LogAction,
        reason: &str,
        author_id: Option<String>,
    ) {
        if let Ok(mut log) = self.moderation_log.lock() {
            log.record(LogEntry {
                at: now_ms(),
                lane,
                action,
                reason: reason.to_owned(),
                author_id,
            });
        }
    }

    pub fn moderation_entries(&self) -> (Vec<LogEntry>, LogSummary) {
        self.moderation_log
            .lock()
            .map(|log| (log.entries(), log.summary(now_ms())))
            .unwrap_or_default()
    }

    pub fn clear_moderation_log(&self) {
        if let Ok(mut log) = self.moderation_log.lock() {
            log.clear();
        }
    }

    pub fn moderation_verdict(&self, message_id: &str) -> Option<MessageVerdict> {
        self.moderation
            .lock()
            .ok()
            .and_then(|runtime| runtime.verdict(message_id))
    }

    pub fn remember_moderation_verdict(&self, message_id: &str, verdict: MessageVerdict) {
        if let Ok(mut runtime) = self.moderation.lock() {
            runtime.remember_verdict(message_id, verdict, now_ms());
        }
    }

    pub(super) async fn enqueue_pending(&self, entry: PendingEntry<'_>) {
        let verdict = self.moderation_verdict(&entry.media.message_id);
        let author_id = verdict.as_ref().map(|verdict| verdict.author_id.clone());
        let channel_id = verdict.as_ref().map(|verdict| verdict.channel_id.clone());
        let mut pending = self.pending_media.write().await;
        // Evict the oldest unreviewed item instead of silently dropping new media.
        let evicted = if pending.len() >= MODERATION_QUEUE_LIMIT {
            pending.pop_front()
        } else {
            None
        };
        let pending_id = self.next_moderation_id.fetch_add(1, Ordering::Relaxed);
        let reason = entry
            .hold_reason
            .clone()
            .or_else(|| entry.report.primary_reason().map(str::to_owned))
            .unwrap_or_else(|| "manual_review".into());
        let delayed = entry.release_at.is_some();
        pending.push_back(PendingMedia {
            id: pending_id,
            media: entry.media,
            sticker: entry.sticker,
            sticker_bytes: entry.sticker_bytes,
            privacy_classification: (entry.report.classification
                != privacy::PrivacyClassification::Safe)
                .then_some(entry.report.classification),
            privacy_categories: entry.report.categories.clone(),
            privacy_reason: entry.report.primary_reason().map(str::to_owned),
            hold_reason: entry.hold_reason,
            release_at: entry.release_at,
            author_id: author_id.clone(),
            channel_id,
            queued_at: now_ms(),
        });
        drop(pending);
        if let Some(evicted) = evicted {
            self.pending_privacy_roles.write().await.remove(&evicted.id);
            self.record_moderation(
                Some(Lane::Media),
                LogAction::Evicted,
                "queue_full",
                evicted.author_id,
            );
        }
        if !entry.role_ids.is_empty() {
            self.pending_privacy_roles
                .write()
                .await
                .insert(pending_id, entry.role_ids.to_vec());
        }
        if !delayed {
            self.record_moderation(Some(Lane::Media), LogAction::Held, &reason, author_id);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn hold_text(
        &self,
        message_id: String,
        text: String,
        author: AuthorIdentity,
        guild_tag: Option<GuildTagIdentity>,
        timestamp: u64,
        segments: Vec<VisualSegment>,
        reason: &str,
    ) {
        let verdict = self.moderation_verdict(&message_id);
        let author_id = verdict.as_ref().map(|verdict| verdict.author_id.clone());
        let mut pending = self.pending_texts.write().await;
        if pending.len() >= MODERATION_QUEUE_LIMIT {
            pending.pop_front();
        }
        pending.push_back(PendingText {
            id: self.next_moderation_id.fetch_add(1, Ordering::Relaxed),
            message_id,
            text,
            author,
            guild_tag,
            segments,
            timestamp,
            reason: Some(reason.to_owned()),
            author_id: author_id.clone(),
            channel_id: verdict.map(|verdict| verdict.channel_id),
            queued_at: now_ms(),
        });
        drop(pending);
        self.record_moderation(
            Some(Lane::Notifications),
            LogAction::Held,
            reason,
            author_id,
        );
    }

    pub async fn approve_text(&self, id: u64) -> bool {
        let item = {
            let mut pending = self.pending_texts.write().await;
            let Some(index) = pending.iter().position(|item| item.id == id) else {
                return false;
            };
            pending.remove(index)
        };
        let Some(item) = item else {
            return false;
        };
        self.record_moderation(
            Some(Lane::Notifications),
            LogAction::Approved,
            item.reason.as_deref().unwrap_or("manual_review"),
            item.author_id.clone(),
        );
        let ticket = self
            .register_stage_output(item.timestamp, &item.message_id, 0, StageLane::Notification)
            .await;
        self.publish_notification_card_with_ticket(
            ticket,
            item.message_id,
            item.text,
            item.author,
            item.guild_tag,
            item.timestamp,
            item.segments,
        )
        .await;
        true
    }

    pub async fn reject_text(&self, id: u64) -> Option<PendingText> {
        let mut pending = self.pending_texts.write().await;
        let index = pending.iter().position(|item| item.id == id)?;
        let item = pending.remove(index)?;
        drop(pending);
        self.record_moderation(
            Some(Lane::Notifications),
            LogAction::Rejected,
            item.reason.as_deref().unwrap_or("manual_review"),
            item.author_id.clone(),
        );
        Some(item)
    }

    /// Author and Discord location of a pending item, for actions from the queue.
    pub async fn pending_origin(
        &self,
        id: u64,
    ) -> Option<(Option<String>, Option<String>, String)> {
        if let Some(item) = self
            .pending_media
            .read()
            .await
            .iter()
            .find(|item| item.id == id)
        {
            return Some((
                item.author_id.clone(),
                item.channel_id.clone(),
                item.media.message_id.clone(),
            ));
        }
        self.pending_texts
            .read()
            .await
            .iter()
            .find(|item| item.id == id)
            .map(|item| {
                (
                    item.author_id.clone(),
                    item.channel_id.clone(),
                    item.message_id.clone(),
                )
            })
    }

    /// Publishes safety-delayed media that nobody cancelled and expires old items.
    pub async fn moderation_tick(&self) {
        let now = now_ms();
        let due = self
            .pending_media
            .read()
            .await
            .iter()
            .filter(|item| item.release_at.is_some_and(|at| at <= now))
            .map(|item| item.id)
            .collect::<Vec<_>>();
        for id in due {
            if self.approve_media_checked(id, false).await {
                self.record_moderation(
                    Some(Lane::Media),
                    LogAction::Released,
                    "safety_delay",
                    None,
                );
            }
        }
        let expiry_minutes = self.config.read().await.moderation.pending_expiry_minutes;
        if expiry_minutes == 0 {
            return;
        }
        let limit = u64::from(expiry_minutes) * 60_000;
        let expired_media = {
            let mut pending = self.pending_media.write().await;
            let (expired, kept): (VecDeque<_>, VecDeque<_>) = pending.drain(..).partition(|item| {
                item.release_at.is_none() && now.saturating_sub(item.queued_at) >= limit
            });
            *pending = kept;
            expired
        };
        for item in expired_media {
            self.pending_privacy_roles.write().await.remove(&item.id);
            self.record_moderation(
                Some(Lane::Media),
                LogAction::Expired,
                "expired",
                item.author_id,
            );
        }
        let expired_texts = {
            let mut pending = self.pending_texts.write().await;
            let (expired, kept): (VecDeque<_>, VecDeque<_>) = pending
                .drain(..)
                .partition(|item| now.saturating_sub(item.queued_at) >= limit);
            *pending = kept;
            expired
        };
        for item in expired_texts {
            self.record_moderation(
                Some(Lane::Notifications),
                LogAction::Expired,
                "expired",
                item.author_id,
            );
        }
    }
}
