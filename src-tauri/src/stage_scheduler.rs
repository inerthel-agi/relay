use crate::clock::now_ms;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use serde::Serialize;
use tokio::sync::{Mutex, broadcast};

use crate::model::{NotificationEvent, RelayEvent};

const READY_TIMEOUT: Duration = Duration::from_secs(20);
const CLAIM_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StageLane {
    Media,
    Notification,
    Music,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct StageOrderKey {
    pub timestamp_ms: u64,
    pub message_id: u64,
    pub part: u16,
    pub insertion: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StageTicket(u64);

/// What became of an item handed to the stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Readiness {
    /// Waits for its turn on screen.
    Queued,
    /// Discarded, with the reason for the decision log: outputs paused, or
    /// pinned messages whose text queue is full.
    Dropped(&'static str),
    /// Its ticket was already cancelled.
    Cancelled,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub author: String,
    pub ready: bool,
}

#[derive(Clone)]
pub struct StageScheduler {
    inner: Arc<Inner>,
}

struct Inner {
    relay_tx: broadcast::Sender<RelayEvent>,
    next_id: AtomicU64,
    next_insertion: AtomicU64,
    ready_timeout: Duration,
    claim_timeout: Duration,
    /// Set by the panic button: new content is dropped until resumed.
    paused: AtomicBool,
    state: Mutex<SchedulerState>,
}

#[derive(Default)]
struct SchedulerState {
    messages_pinned: bool,
    message_queue_limit: usize,
    ordered: BTreeMap<StageOrderKey, StageTicket>,
    entries: HashMap<StageTicket, TicketEntry>,
    /// Text cards have their own place on screen; media, stickers and music
    /// share the other one. At most one item per place.
    active: Vec<ActiveTicket>,
    media_busy: bool,
    music_busy: bool,
    notification_busy: bool,
}

impl SchedulerState {
    fn is_active(&self, ticket: StageTicket) -> bool {
        self.active.iter().any(|active| active.ticket == ticket)
    }

    /// The item left the screen: it leaves the scheduler.
    fn finish(&mut self, ticket: StageTicket) {
        self.active.retain(|active| active.ticket != ticket);
        self.entries.remove(&ticket);
    }

    fn place_free(&self, text: bool) -> bool {
        let busy = match text {
            true => self.notification_busy,
            false => self.media_busy || self.music_busy,
        };
        !busy
            && !self
                .active
                .iter()
                .any(|active| is_text(active.lane) == text)
    }

    /// The oldest ready item of each place, in Discord order. A part never
    /// passes an older part of its own message that is still being checked:
    /// a message shows only once every part passed.
    fn next_ready(&self) -> Option<(StageOrderKey, StageTicket)> {
        let mut seen_places = [false; 2];
        let mut unchecked_messages = HashSet::new();
        for (key, ticket) in &self.ordered {
            let Some(entry) = self.entries.get(ticket) else {
                continue;
            };
            let group = message_group(entry.original_key);
            if entry.event.is_none() {
                unchecked_messages.insert(group);
            }
            let text = is_text(entry.lane);
            // Pinned messages wait in their own bounded lane.
            if text && self.messages_pinned {
                continue;
            }
            if std::mem::replace(&mut seen_places[usize::from(text)], true) {
                continue;
            }
            if entry.event.is_some()
                && self.place_free(text)
                && !unchecked_messages.contains(&group)
            {
                return Some((*key, *ticket));
            }
        }
        None
    }
}

struct TicketEntry {
    original_key: StageOrderKey,
    key: Option<StageOrderKey>,
    lane: StageLane,
    event: Option<RelayEvent>,
}

#[derive(Clone, Copy)]
struct ActiveTicket {
    ticket: StageTicket,
    lane: StageLane,
    saw_busy: bool,
}

impl StageScheduler {
    pub async fn set_messages_pinned(&self, pinned: bool, queue_limit: usize) {
        {
            let mut state = self.inner.state.lock().await;
            state.messages_pinned = pinned;
            state.message_queue_limit = queue_limit.max(1);
            if pinned
                && let Some(active) = state
                    .active
                    .iter()
                    .find(|active| is_text(active.lane))
                    .copied()
            {
                state.finish(active.ticket);
            }
        }
        self.try_dispatch().await;
    }
    pub fn new(relay_tx: broadcast::Sender<RelayEvent>) -> Self {
        Self::with_timeouts(relay_tx, READY_TIMEOUT, CLAIM_TIMEOUT)
    }

    fn with_timeouts(
        relay_tx: broadcast::Sender<RelayEvent>,
        ready_timeout: Duration,
        claim_timeout: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                relay_tx,
                next_id: AtomicU64::new(1),
                next_insertion: AtomicU64::new(1),
                ready_timeout,
                claim_timeout,
                paused: AtomicBool::new(false),
                state: Mutex::new(SchedulerState::default()),
            }),
        }
    }

    pub async fn reserve(
        &self,
        timestamp_ms: u64,
        message_id: &str,
        part: u16,
        lane: StageLane,
    ) -> StageTicket {
        let ticket = StageTicket(self.inner.next_id.fetch_add(1, Ordering::Relaxed));
        let key = StageOrderKey {
            timestamp_ms,
            message_id: message_id.parse().unwrap_or(u64::MAX),
            part,
            insertion: self.inner.next_insertion.fetch_add(1, Ordering::Relaxed),
        };
        {
            let mut state = self.inner.state.lock().await;
            state.ordered.insert(key, ticket);
            state.entries.insert(
                ticket,
                TicketEntry {
                    original_key: key,
                    key: Some(key),
                    lane,
                    event: None,
                },
            );
        }
        self.spawn_ready_timeout(ticket);
        self.try_dispatch().await;
        ticket
    }

    pub async fn enqueue(&self, event: RelayEvent, lane: StageLane) -> StageTicket {
        let (timestamp, message_id, part) = event_order(&event);
        let numeric_message_id = message_id.parse().unwrap_or(u64::MAX);
        let reserved = {
            let state = self.inner.state.lock().await;
            state.ordered.iter().find_map(|(key, ticket)| {
                let entry = state.entries.get(ticket)?;
                (key.timestamp_ms == timestamp
                    && key.message_id == numeric_message_id
                    && entry.original_key.part == part
                    && entry.lane == lane
                    && entry.event.is_none())
                .then_some(*ticket)
            })
        };
        if let Some(ticket) = reserved {
            self.ready(ticket, event).await;
            return ticket;
        }
        let ticket = self.reserve(timestamp, &message_id, part, lane).await;
        self.ready(ticket, event).await;
        ticket
    }

    /// True while media, a sticker or music is on screen.
    pub async fn media_on_screen(&self) -> bool {
        !self.inner.state.lock().await.place_free(false)
    }

    pub fn set_paused(&self, paused: bool) {
        self.inner.paused.store(paused, Ordering::SeqCst);
    }

    pub fn is_paused(&self) -> bool {
        self.inner.paused.load(Ordering::SeqCst)
    }

    pub async fn ready(&self, ticket: StageTicket, event: RelayEvent) -> Readiness {
        let readiness = {
            let mut state = self.inner.state.lock().await;
            if !state.entries.contains_key(&ticket) {
                return Readiness::Cancelled;
            }
            // While paused, content that becomes ready is discarded rather than queued,
            // so resuming does not flood the stream with everything sent meanwhile.
            let dropped = if self.is_paused() {
                Some("outputs_paused")
            } else if state.messages_pinned
                && matches!(&event, RelayEvent::Notification(_))
                && state
                    .entries
                    .values()
                    .filter(|entry| is_text(entry.lane) && entry.event.is_some())
                    .count()
                    >= state.message_queue_limit
            {
                Some("queue_full")
            } else {
                None
            };
            if let Some(reason) = dropped {
                if let Some(entry) = state.entries.remove(&ticket) {
                    if let Some(key) = entry.key {
                        state.ordered.remove(&key);
                    }
                    // The other parts of the message no longer wait for it.
                    self.reinsert_ready_group_for_key(&mut state, entry.original_key);
                }
                Readiness::Dropped(reason)
            } else {
                let entry = state
                    .entries
                    .get_mut(&ticket)
                    .expect("checked stage ticket");
                entry.event = Some(event);
                let demoted = entry.key.is_none();
                if demoted {
                    self.reinsert_ready_group(&mut state, ticket);
                }
                Readiness::Queued
            }
        };
        self.try_dispatch().await;
        readiness
    }

    pub async fn cancel(&self, ticket: StageTicket) {
        self.cancel_many(&[ticket]).await;
    }

    /// Withdraws every part of the ticket's message that is not on screen yet.
    /// A held or dropped text must not leave its message's ready media to show
    /// on their own once the text no longer gates them.
    pub async fn cancel_message(&self, ticket: StageTicket) {
        let tickets = {
            let state = self.inner.state.lock().await;
            let Some(group) = state
                .entries
                .get(&ticket)
                .map(|entry| message_group(entry.original_key))
            else {
                return;
            };
            state
                .entries
                .iter()
                .filter(|(other, entry)| {
                    message_group(entry.original_key) == group && !state.is_active(**other)
                })
                .map(|(other, _)| *other)
                .collect::<Vec<_>>()
        };
        self.cancel_many(&tickets).await;
    }

    /// Cancels several tickets in one step, so a part that is already ready
    /// never reaches the stage while its siblings are still being removed.
    pub async fn cancel_many(&self, tickets: &[StageTicket]) {
        {
            let mut state = self.inner.state.lock().await;
            let mut original_keys = Vec::with_capacity(tickets.len());
            for ticket in tickets {
                if let Some(entry) = state.entries.remove(ticket) {
                    if let Some(key) = entry.key {
                        state.ordered.remove(&key);
                    }
                    original_keys.push(entry.original_key);
                }
                state.active.retain(|active| active.ticket != *ticket);
            }
            for key in original_keys {
                self.reinsert_ready_group_for_key(&mut state, key);
            }
        }
        self.try_dispatch().await;
    }

    pub async fn stage_state(&self, media_busy: bool, music_busy: bool, notification_busy: bool) {
        {
            let mut state = self.inner.state.lock().await;
            state.media_busy = media_busy;
            state.music_busy = music_busy;
            state.notification_busy = notification_busy;
            let mut completed = Vec::new();
            for active in state.active.iter_mut() {
                let busy = lane_busy(active.lane, media_busy, music_busy, notification_busy);
                if busy {
                    active.saw_busy = true;
                } else if active.saw_busy {
                    completed.push(active.ticket);
                }
            }
            for ticket in completed {
                state.finish(ticket);
            }
        }
        self.try_dispatch().await;
    }

    pub async fn queue_snapshot(&self) -> Vec<QueueItem> {
        let state = self.inner.state.lock().await;
        state
            .ordered
            .values()
            .filter_map(|ticket| {
                let entry = state.entries.get(ticket)?;
                let (kind, title, author) = match &entry.event {
                    Some(RelayEvent::Media(media)) => (
                        format!("{:?}", media.kind).to_lowercase(),
                        media
                            .title
                            .clone()
                            .unwrap_or_else(|| media.filename.clone()),
                        media.author.username.clone(),
                    ),
                    Some(RelayEvent::Notification(notification)) => (
                        "notification".into(),
                        "Message".into(),
                        notification.author.username.clone(),
                    ),
                    Some(RelayEvent::Sticker(sticker)) => (
                        "sticker".into(),
                        sticker.name.clone(),
                        sticker.author.username.clone(),
                    ),
                    Some(RelayEvent::MusicPlay(music)) => (
                        "music".into(),
                        music.title.clone(),
                        music.requested_by.clone(),
                    ),
                    None if entry.lane == StageLane::Music => {
                        return None;
                    }
                    _ => ("preparing".into(), String::new(), String::new()),
                };
                Some(QueueItem {
                    id: format!("stage:{}", ticket.0),
                    kind,
                    title,
                    author,
                    ready: entry.event.is_some(),
                })
            })
            .collect()
    }

    pub async fn remove_queued_music(&self, id: u64) -> Option<String> {
        let ticket = StageTicket(id);
        let playback_id = {
            let mut state = self.inner.state.lock().await;
            if state.is_active(ticket) {
                return None;
            }
            let entry = state.entries.get(&ticket)?;
            let Some(RelayEvent::MusicPlay(music)) = &entry.event else {
                return None;
            };
            let playback_id = music.playback_id.clone();
            let entry = state.entries.remove(&ticket)?;
            if let Some(key) = entry.key {
                state.ordered.remove(&key);
            }
            self.reinsert_ready_group_for_key(&mut state, entry.original_key);
            playback_id
        };
        self.try_dispatch().await;
        Some(playback_id)
    }

    /// Reorder the requested non-active Music tickets while keeping every
    /// non-Music ticket in its existing scheduler slot. A current Music ticket
    /// can already be ready but waiting behind a busy lane, so it remains in
    /// place when it is not included in `ticket_order`. A queued track whose
    /// reservation was demoted after the ready delay has no slot to swap: its
    /// reservation is replaced at promotion, so it is accepted as it is.
    pub async fn reorder_music(&self, ticket_order: &[StageTicket]) -> bool {
        let accepted = {
            let mut state = self.inner.state.lock().await;
            let mut seen = HashSet::with_capacity(ticket_order.len());
            if ticket_order.is_empty() || ticket_order.iter().any(|ticket| !seen.insert(*ticket)) {
                false
            } else {
                let valid_tickets = ticket_order.iter().all(|ticket| {
                    state.entries.get(ticket).is_some_and(|entry| {
                        entry.lane == StageLane::Music
                            && !state.is_active(*ticket)
                            && entry
                                .key
                                .is_none_or(|key| state.ordered.get(&key) == Some(ticket))
                    })
                });
                if !valid_tickets {
                    false
                } else {
                    let slotted = ticket_order
                        .iter()
                        .copied()
                        .filter(|ticket| state.entries.get(ticket).is_some_and(|e| e.key.is_some()))
                        .collect::<Vec<_>>();
                    let mut slot_keys = slotted
                        .iter()
                        .map(|ticket| {
                            state
                                .entries
                                .get(ticket)
                                .and_then(|entry| entry.key)
                                .expect("validated Music ticket key")
                        })
                        .collect::<Vec<_>>();
                    slot_keys.sort_unstable();
                    for key in &slot_keys {
                        state.ordered.remove(key);
                    }
                    for (key, ticket) in slot_keys.into_iter().zip(slotted) {
                        state.ordered.insert(key, ticket);
                        if let Some(entry) = state.entries.get_mut(&ticket) {
                            entry.key = Some(key);
                        }
                    }
                    true
                }
            }
        };
        if accepted {
            self.try_dispatch().await;
        }
        accepted
    }

    pub async fn remove_queued(&self, id: u64) -> bool {
        let ticket = StageTicket(id);
        {
            let mut state = self.inner.state.lock().await;
            if state.is_active(ticket) {
                return false;
            }
            let Some(entry) = state.entries.get(&ticket) else {
                return false;
            };
            if entry.lane == StageLane::Music || entry.event.is_none() {
                return false;
            }
            let entry = state.entries.remove(&ticket).expect("checked ticket");
            if let Some(key) = entry.key {
                state.ordered.remove(&key);
            }
            self.reinsert_ready_group_for_key(&mut state, entry.original_key);
        }
        self.try_dispatch().await;
        true
    }

    pub async fn clear(&self) {
        let mut state = self.inner.state.lock().await;
        state.ordered.clear();
        state.entries.clear();
        state.active.clear();
    }

    /// Skip ends everything on screen: the pages hide the text card too.
    pub async fn skip_active(&self) {
        {
            let mut state = self.inner.state.lock().await;
            if state.active.is_empty() {
                return;
            }
            for active in std::mem::take(&mut state.active) {
                state.entries.remove(&active.ticket);
            }
        }
        self.try_dispatch().await;
    }

    async fn try_dispatch(&self) {
        loop {
            let (ticket, event) = {
                let mut state = self.inner.state.lock().await;
                let Some((key, ticket)) = state.next_ready() else {
                    return;
                };
                let entry = state.entries.get_mut(&ticket).expect("ready stage ticket");
                let event = entry.event.take().expect("ready stage event");
                let lane = entry.lane;
                state.ordered.remove(&key);
                state.active.push(ActiveTicket {
                    ticket,
                    lane,
                    saw_busy: false,
                });
                (ticket, event)
            };
            let _ = self.inner.relay_tx.send(event);
            self.spawn_claim_timeout(ticket);
        }
    }

    fn spawn_ready_timeout(&self, ticket: StageTicket) {
        let scheduler = self.clone();
        let ready_timeout = self.inner.ready_timeout;
        tokio::spawn(async move {
            tokio::time::sleep(ready_timeout).await;
            let demoted = {
                let mut state = scheduler.inner.state.lock().await;
                let active_ticket = state.is_active(ticket);
                let original_key = match state.entries.get(&ticket) {
                    Some(entry) if entry.event.is_none() && !active_ticket => entry.original_key,
                    Some(_) => return,
                    None => return,
                };
                scheduler.demote_group(&mut state, original_key)
            };
            if demoted {
                scheduler.try_dispatch().await;
            }
        });
    }

    fn spawn_claim_timeout(&self, ticket: StageTicket) {
        let scheduler = self.clone();
        let claim_timeout = self.inner.claim_timeout;
        tokio::spawn(async move {
            tokio::time::sleep(claim_timeout).await;
            let released = {
                let mut state = scheduler.inner.state.lock().await;
                if !state
                    .active
                    .iter()
                    .any(|active| active.ticket == ticket && !active.saw_busy)
                {
                    return;
                }
                state.finish(ticket);
                true
            };
            if released {
                scheduler.try_dispatch().await;
            }
        });
    }

    fn demote_group(&self, state: &mut SchedulerState, original_key: StageOrderKey) -> bool {
        let group = message_group(original_key);
        // The active part is already on stage: demoting it would leave a
        // key-less entry that no later reinsertion can ever release.
        let tickets = state
            .entries
            .iter()
            .filter_map(|(ticket, entry)| {
                (message_group(entry.original_key) == group && !state.is_active(*ticket))
                    .then_some(*ticket)
            })
            .collect::<Vec<_>>();
        let mut demoted = false;
        for ticket in tickets {
            let Some(entry) = state.entries.get_mut(&ticket) else {
                continue;
            };
            if let Some(key) = entry.key.take() {
                state.ordered.remove(&key);
                demoted = true;
            }
        }
        demoted
    }

    fn reinsert_ready_group(&self, state: &mut SchedulerState, ticket: StageTicket) {
        let Some(entry) = state.entries.get(&ticket) else {
            return;
        };
        self.reinsert_ready_group_for_key(state, entry.original_key);
    }

    fn reinsert_ready_group_for_key(
        &self,
        state: &mut SchedulerState,
        original_key: StageOrderKey,
    ) {
        let group = message_group(original_key);
        // Only demoted tickets wait for each other. A part reserved after the
        // demotion (a late GIF embed) keeps its slot and may already be active.
        let mut tickets = state
            .entries
            .iter()
            .filter_map(|(ticket, entry)| {
                (message_group(entry.original_key) == group && entry.key.is_none())
                    .then_some(*ticket)
            })
            .collect::<Vec<_>>();
        if tickets.is_empty()
            || !tickets.iter().all(|ticket| {
                state
                    .entries
                    .get(ticket)
                    .is_some_and(|entry| entry.event.is_some())
            })
        {
            return;
        }
        tickets.sort_by_key(|ticket| {
            state
                .entries
                .get(ticket)
                .expect("registered stage ticket")
                .original_key
        });
        let readiness_timestamp = now_ms();
        for ticket in tickets {
            let original_key = state
                .entries
                .get(&ticket)
                .expect("registered stage ticket")
                .original_key;
            let key = StageOrderKey {
                timestamp_ms: readiness_timestamp,
                message_id: original_key.message_id,
                part: original_key.part,
                insertion: self.inner.next_insertion.fetch_add(1, Ordering::Relaxed),
            };
            let entry = state
                .entries
                .get_mut(&ticket)
                .expect("registered stage ticket");
            if entry.key.is_none() {
                entry.key = Some(key);
                state.ordered.insert(key, ticket);
            }
        }
    }
}

fn event_order(event: &RelayEvent) -> (u64, String, u16) {
    match event {
        RelayEvent::Notification(NotificationEvent { timestamp, id, .. }) => {
            (*timestamp, id.clone(), 0)
        }
        RelayEvent::Sticker(event) => (event.timestamp, event.message_id.clone(), 100),
        RelayEvent::Media(event) => (event.timestamp, event.message_id.clone(), 200),
        RelayEvent::MusicPlay(event) => (now_ms(), event.playback_id.clone(), 300),
        _ => (now_ms(), String::new(), u16::MAX),
    }
}

fn message_group(key: StageOrderKey) -> (u64, u64) {
    (key.timestamp_ms, key.message_id)
}

fn is_text(lane: StageLane) -> bool {
    lane == StageLane::Notification
}

fn lane_busy(lane: StageLane, media_busy: bool, music_busy: bool, notification_busy: bool) -> bool {
    match lane {
        StageLane::Media => media_busy,
        StageLane::Notification => notification_busy,
        StageLane::Music => music_busy,
    }
}

#[cfg(test)]
mod tests;
