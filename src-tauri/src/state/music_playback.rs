use super::*;

use crate::music::{MusicQueueDirection, MusicQueueMove, MusicQueuePolicy};

impl AppCore {
    pub async fn start_music(
        &self,
        selection: MusicSelection,
        mode: MusicPlaybackMode,
        order_timestamp: u64,
        order_id: &str,
    ) -> crate::music::MusicStartResult {
        let policy = {
            let config = self.config.read().await;
            MusicQueuePolicy::new(
                config.music_max_pending_per_user,
                config.music_reject_duplicate_pending,
            )
        };
        self.start_music_with_policy(selection, mode, policy, order_timestamp, order_id)
            .await
    }

    pub async fn start_music_with_policy(
        &self,
        selection: MusicSelection,
        mode: MusicPlaybackMode,
        policy: MusicQueuePolicy,
        order_timestamp: u64,
        order_id: &str,
    ) -> crate::music::MusicStartResult {
        // Keep admission and insertion under the same MusicState lock. The
        // scheduler reservation can wait while this lock is held; a rejected
        // request is then cancelled without exposing a race to another
        // request for the same member or video.
        let history_selection = selection.clone();
        let mut music = self.music.lock().await;
        let ticket = self
            .register_stage_output(order_timestamp, order_id, 300, StageLane::Music)
            .await;
        let result = music.start_with_policy(selection, mode, policy);
        if let crate::music::MusicStartResult::Queued { playback, .. } = &result {
            self.music_stage_tickets
                .lock()
                .await
                .insert(playback.playback_id.clone(), ticket);
        }
        drop(music);
        self.remember_music(&result, history_selection, order_timestamp)
            .await;
        self.emit_music_start(result, ticket).await
    }

    pub async fn start_music_custom(
        &self,
        selection: MusicSelection,
        start_seconds: u64,
        end_seconds: u64,
        order_timestamp: u64,
        order_id: &str,
    ) -> Result<crate::music::MusicStartResult, crate::music::CustomRangeError> {
        let policy = {
            let config = self.config.read().await;
            MusicQueuePolicy::new(
                config.music_max_pending_per_user,
                config.music_reject_duplicate_pending,
            )
        };
        self.start_music_custom_with_policy(
            selection,
            start_seconds,
            end_seconds,
            policy,
            order_timestamp,
            order_id,
        )
        .await
    }

    pub async fn start_music_custom_with_policy(
        &self,
        selection: MusicSelection,
        start_seconds: u64,
        end_seconds: u64,
        policy: MusicQueuePolicy,
        order_timestamp: u64,
        order_id: &str,
    ) -> Result<crate::music::MusicStartResult, crate::music::CustomRangeError> {
        let history_selection = selection.clone();
        let mut music = self.music.lock().await;
        let ticket = self
            .register_stage_output(order_timestamp, order_id, 300, StageLane::Music)
            .await;
        let result = music.start_custom_with_policy(selection, start_seconds, end_seconds, policy);
        if let Ok(crate::music::MusicStartResult::Queued { playback, .. }) = &result {
            self.music_stage_tickets
                .lock()
                .await
                .insert(playback.playback_id.clone(), ticket);
        }
        drop(music);
        match result {
            Ok(result) => {
                self.remember_music(&result, history_selection, order_timestamp)
                    .await;
                Ok(self.emit_music_start(result, ticket).await)
            }
            Err(error) => {
                self.cancel_stage_output(ticket).await;
                Err(error)
            }
        }
    }

    async fn remember_music(
        &self,
        result: &crate::music::MusicStartResult,
        selection: MusicSelection,
        timestamp: u64,
    ) {
        let (crate::music::MusicStartResult::Started(playback)
        | crate::music::MusicStartResult::Queued { playback, .. }) = result
        else {
            return;
        };
        let mut history = self.history.write().await;
        // Replays and repeat requests reuse the original row and selected range.
        if history.iter().any(|entry| {
            matches!(entry, HistoryEntry::Music(entry)
            if entry.selection == selection && entry.music.mode == playback.mode
                && entry.music.start_seconds == playback.start_seconds
                && entry.music.end_seconds == playback.end_seconds)
        }) {
            return;
        }
        let entry = crate::model::MusicHistoryEntry {
            music: playback.clone(),
            timestamp,
            selection,
        };
        history.push_front(HistoryEntry::Music(entry.clone()));
        history.truncate(HISTORY_LIMIT);
        let _ = self.relay_tx.send(RelayEvent::MusicHistory(entry));
    }

    pub async fn music_history_entry(
        &self,
        playback_id: &str,
    ) -> Option<crate::model::MusicHistoryEntry> {
        self.history
            .read()
            .await
            .iter()
            .find_map(|entry| match entry {
                HistoryEntry::Music(entry) if entry.music.playback_id == playback_id => {
                    Some(entry.clone())
                }
                _ => None,
            })
    }

    pub async fn replay_music_history(&self, playback_id: &str) -> Result<()> {
        let entry = self
            .music_history_entry(playback_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("The music is no longer in history."))?;
        let timestamp = crate::clock::now_ms();
        let result = match entry.music.mode {
            MusicPlaybackMode::Custom => self
                .start_music_custom(
                    entry.selection,
                    entry.music.start_seconds,
                    entry
                        .music
                        .end_seconds
                        .ok_or_else(|| anyhow::anyhow!("Missing music range."))?,
                    timestamp,
                    playback_id,
                )
                .await
                .map_err(|_| anyhow::anyhow!("The music range is no longer valid."))?,
            mode => {
                self.start_music(entry.selection, mode, timestamp, playback_id)
                    .await
            }
        };
        match result {
            crate::music::MusicStartResult::Started(_)
            | crate::music::MusicStartResult::Queued { .. } => Ok(()),
            crate::music::MusicStartResult::QueueFull => bail!("The music queue is full."),
            crate::music::MusicStartResult::UserQueueFull { .. } => {
                bail!("The requester's music queue is full.")
            }
            crate::music::MusicStartResult::DuplicatePending { .. } => {
                bail!("This music is already queued.")
            }
        }
    }

    pub(super) async fn emit_music_start(
        &self,
        result: crate::music::MusicStartResult,
        ticket: StageTicket,
    ) -> crate::music::MusicStartResult {
        match &result {
            crate::music::MusicStartResult::Started(playback) => {
                self.complete_music(ticket, playback.clone()).await;
            }
            // The ticket is linked while the MusicState lock is still held in
            // start_music(_custom), so a concurrent removal cannot orphan it.
            crate::music::MusicStartResult::Queued { .. } => {}
            crate::music::MusicStartResult::QueueFull
            | crate::music::MusicStartResult::UserQueueFull { .. }
            | crate::music::MusicStartResult::DuplicatePending { .. } => {
                self.cancel_stage_output(ticket).await;
            }
        }
        result
    }

    pub async fn current_music(&self) -> Option<MusicPlaybackEvent> {
        self.music.lock().await.current_event()
    }

    pub async fn cancel_pending_music(&self, playback_id: &str) {
        let _ = self.remove_pending_music(playback_id).await;
    }

    /// Remove a waiting track and its scheduler placeholder as one logical
    /// operation. Holding MusicState while taking the ticket-map lock closes
    /// the promotion/removal race: a track cannot become current after it has
    /// been accepted for removal.
    pub async fn remove_pending_music(&self, playback_id: &str) -> bool {
        let (stopped, ticket) = {
            let mut music = self.music.lock().await;
            let Some(stopped) = music.remove_pending(playback_id) else {
                return false;
            };
            let ticket = self.music_stage_tickets.lock().await.remove(playback_id);
            (stopped, ticket)
        };
        if let Some(ticket) = ticket {
            self.cancel_stage_output(ticket).await;
        }
        self.delete_now_playing_message(stopped.channel_id, stopped.now_playing_message_id)
            .await;
        true
    }

    /// Move one pending track and mirror the order in the shared scheduler.
    /// If the scheduler no longer has a matching placeholder, restore the
    /// original MusicState order and report the failed synchronization.
    pub async fn move_pending_music(
        &self,
        playback_id: &str,
        direction: MusicQueueDirection,
    ) -> MusicQueueMove {
        let mut music = self.music.lock().await;
        let old_order = music.pending_playback_ids();
        let result = music.move_pending(playback_id, direction);
        if !matches!(result, MusicQueueMove::Moved { .. }) {
            return result;
        }
        let new_order = music.pending_playback_ids();
        let ticket_guard = self.music_stage_tickets.lock().await;
        let Some(tickets) = new_order
            .iter()
            .map(|id| ticket_guard.get(id).copied())
            .collect::<Option<Vec<_>>>()
        else {
            let _ = music.reorder_pending_by_ids(&old_order);
            return MusicQueueMove::SchedulerUnavailable;
        };
        if !self.stage_scheduler.reorder_music(&tickets).await {
            let _ = music.reorder_pending_by_ids(&old_order);
            return MusicQueueMove::SchedulerUnavailable;
        }
        result
    }

    pub async fn stop_current_music(&self) -> Option<MusicPlaybackEvent> {
        let (stopped, next) = {
            let mut music = self.music.lock().await;
            let stopped = music.stop_current();
            let next = if stopped.is_some() {
                music.promote_next()
            } else {
                None
            };
            (stopped, next)
        };
        if let Some(stopped) = &stopped {
            self.delete_now_playing_message(stopped.channel_id, stopped.now_playing_message_id)
                .await;
            let _ = self.relay_tx.send(RelayEvent::MusicStop(MusicStopEvent {
                playback_id: stopped.playback.playback_id.clone(),
            }));
            self.emit_music_follow_up(next).await;
            return Some(stopped.playback.clone());
        }
        None
    }

    /// Clear current music and the entire pending queue (force-clear / overlay clear).
    pub async fn clear_all_music(&self) -> Option<MusicPlaybackEvent> {
        let (cards, stopped) = {
            let mut music = self.music.lock().await;
            (music.active_message_ids(), music.clear_all())
        };
        let tickets = self
            .music_stage_tickets
            .lock()
            .await
            .drain()
            .map(|(_, ticket)| ticket)
            .collect::<Vec<_>>();
        for ticket in tickets {
            self.cancel_stage_output(ticket).await;
        }
        for (channel, message) in cards {
            self.delete_now_playing_message(channel, Some(message))
                .await;
        }
        if let Some(stopped) = &stopped {
            let _ = self.relay_tx.send(RelayEvent::MusicStop(MusicStopEvent {
                playback_id: stopped.playback.playback_id.clone(),
            }));
            // Always idle after a full clear so notification clients resume
            // even when the following Clear event is coalesced or lagged.
            let _ = self.relay_tx.send(RelayEvent::MusicIdle);
            return Some(stopped.playback.clone());
        }
        None
    }

    pub async fn stop_music_if_current(&self, playback_id: &str) -> Option<MusicPlaybackEvent> {
        let (stopped, next) = {
            let mut music = self.music.lock().await;
            let stopped = music.stop_if_current(playback_id);
            let next = if stopped.is_some() {
                music.promote_next()
            } else {
                None
            };
            (stopped, next)
        };
        if let Some(stopped) = &stopped {
            self.delete_now_playing_message(stopped.channel_id, stopped.now_playing_message_id)
                .await;
            let _ = self.relay_tx.send(RelayEvent::MusicStop(MusicStopEvent {
                playback_id: stopped.playback.playback_id.clone(),
            }));
            self.emit_music_follow_up(next).await;
            return Some(stopped.playback.clone());
        }
        None
    }

    /// GUI / hotkey skip: reaction first, then YouTube, then media.
    pub async fn skip_playback(self: &Arc<Self>) {
        if crate::reactions::skip(self).await {
            return;
        }
        if self.stop_current_music().await.is_some() {
            return;
        }
        let _ = self.relay_tx.send(RelayEvent::MusicStop(MusicStopEvent {
            playback_id: String::new(),
        }));
        let _ = self.relay_tx.send(RelayEvent::Skip);
        self.stage_scheduler.skip_active().await;
    }

    pub async fn finish_music(&self, playback_id: &str) -> bool {
        let (stopped, repeated, next) = {
            let mut music = self.music.lock().await;
            let Some((stopped, repeated)) = music.finish_current(playback_id) else {
                return false;
            };
            let next = music.promote_next();
            (stopped, repeated, next)
        };
        if !repeated {
            self.delete_now_playing_message(stopped.channel_id, stopped.now_playing_message_id)
                .await;
        }
        let _ = self.relay_tx.send(RelayEvent::MusicStop(MusicStopEvent {
            playback_id: stopped.playback.playback_id.clone(),
        }));
        // Promote next or signal idle so overlays can resume media.
        self.emit_music_follow_up(next).await;
        true
    }

    pub(crate) async fn delete_now_playing_message(
        &self,
        channel_id: u64,
        message_id: Option<u64>,
    ) {
        let Some(message_id) = message_id else {
            return;
        };
        let config = self.config.read().await;
        if config.music_channel_id == channel_id.to_string()
            && config.music_welcome_message_id == message_id.to_string()
        {
            return;
        }
        if crate::reaction_protection::is_protected(
            &config.reactions.protected_channel_id,
            &config.reactions.protected_message_id,
            channel_id,
            message_id,
        ) {
            return;
        }
        drop(config);
        let http = self.discord_http().await;
        let Some(http) = http else {
            return;
        };
        if serenity::model::id::ChannelId::new(channel_id)
            .delete_message(&*http, serenity::model::id::MessageId::new(message_id))
            .await
            .is_err()
        {
            // Soft-fail: missing Manage Messages (or already deleted) must not break playback.
            eprintln!(
                "relay: failed to delete now-playing message {message_id} in channel {channel_id}"
            );
        }
    }

    pub(super) async fn emit_music_follow_up(&self, next: Option<MusicPlaybackEvent>) {
        if let Some(playback) = next {
            // Release the previous scheduler ticket before the next queued track
            // competes for the shared stage.
            let _ = self.relay_tx.send(RelayEvent::MusicIdle);
            let ticket = self
                .music_stage_tickets
                .lock()
                .await
                .remove(&playback.playback_id);
            if let Some(ticket) = ticket {
                self.complete_music(ticket, playback).await;
            } else {
                self.stage_scheduler
                    .enqueue(RelayEvent::MusicPlay(playback), StageLane::Music)
                    .await;
            }
            crate::bot::refresh_music_card(self).await;
            crate::bot::refresh_pending_music_cards(self).await;
        } else {
            let _ = self.relay_tx.send(RelayEvent::MusicIdle);
        }
    }
}
