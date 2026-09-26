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

use crate::model::{RelayEvent, TtsEvent};

const READY_TIMEOUT: Duration = Duration::from_secs(20);
const CLAIM_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StageLane {
    Media,
    Tts,
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
    active: Option<ActiveTicket>,
    media_busy: bool,
    music_busy: bool,
    tts_busy: bool,
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
                && state
                    .active
                    .is_some_and(|active| active.lane == StageLane::Tts)
            {
                let active = state.active.take().expect("checked active message");
                state.entries.remove(&active.ticket);
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

    pub fn set_paused(&self, paused: bool) {
        self.inner.paused.store(paused, Ordering::SeqCst);
    }

    pub fn is_paused(&self) -> bool {
        self.inner.paused.load(Ordering::SeqCst)
    }

    pub async fn ready(&self, ticket: StageTicket, event: RelayEvent) {
        {
            let mut state = self.inner.state.lock().await;
            // While paused, content that becomes ready is discarded rather than queued,
            // so resuming does not flood the stream with everything sent meanwhile.
            if self.is_paused() {
                if let Some(entry) = state.entries.remove(&ticket)
                    && let Some(key) = entry.key
                {
                    state.ordered.remove(&key);
                }
                return;
            }
            if state.messages_pinned
                && matches!(&event, RelayEvent::Tts(_))
                && state
                    .entries
                    .values()
                    .filter(|entry| entry.lane == StageLane::Tts && entry.event.is_some())
                    .count()
                    >= state.message_queue_limit
            {
                if let Some(entry) = state.entries.remove(&ticket)
                    && let Some(key) = entry.key
                {
                    state.ordered.remove(&key);
                }
                return;
            }
            let demoted = match state.entries.get_mut(&ticket) {
                Some(entry) => {
                    entry.event = Some(event);
                    entry.key.is_none()
                }
                None => return,
            };
            if demoted {
                self.reinsert_ready_group(&mut state, ticket);
            }
        }
        self.try_dispatch().await;
    }

    pub async fn cancel(&self, ticket: StageTicket) {
        {
            let mut state = self.inner.state.lock().await;
            if let Some(entry) = state.entries.remove(&ticket) {
                if let Some(key) = entry.key {
                    state.ordered.remove(&key);
                }
                self.reinsert_ready_group_for_key(&mut state, entry.original_key);
            }
            if state.active.is_some_and(|active| active.ticket == ticket) {
                state.active = None;
            }
        }
        self.try_dispatch().await;
    }

    pub async fn stage_state(&self, media_busy: bool, music_busy: bool, tts_busy: bool) {
        let should_dispatch = {
            let mut state = self.inner.state.lock().await;
            state.media_busy = media_busy;
            state.music_busy = music_busy;
            state.tts_busy = tts_busy;
            let mut completed = None;
            if let Some(active) = state.active.as_mut() {
                let busy = lane_busy(active.lane, media_busy, music_busy, tts_busy);
                if busy {
                    active.saw_busy = true;
                } else if active.saw_busy {
                    completed = Some(active.ticket);
                }
            }
            if let Some(ticket) = completed {
                state.active = None;
                state.entries.remove(&ticket);
                true
            } else {
                state.active.is_none()
            }
        };
        if should_dispatch {
            self.try_dispatch().await;
        }
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
                    Some(RelayEvent::Tts(tts)) => (
                        "notification".into(),
                        "Message".into(),
                        tts.author.username.clone(),
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
            if state.active.is_some_and(|active| active.ticket == ticket) {
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
    /// place when it is not included in `ticket_order`.
    pub async fn reorder_music(&self, ticket_order: &[StageTicket]) -> bool {
        let accepted = {
            let mut state = self.inner.state.lock().await;
            let active_ticket = state.active.map(|active| active.ticket);
            let mut seen = HashSet::with_capacity(ticket_order.len());
            if ticket_order.is_empty() || ticket_order.iter().any(|ticket| !seen.insert(*ticket)) {
                false
            } else {
                let valid_tickets = ticket_order.iter().all(|ticket| {
                    state.entries.get(ticket).is_some_and(|entry| {
                        entry.lane == StageLane::Music
                            && !active_ticket.is_some_and(|active| active == *ticket)
                            && entry
                                .key
                                .is_some_and(|key| state.ordered.get(&key) == Some(ticket))
                    })
                });
                if !valid_tickets {
                    false
                } else {
                    let mut slot_keys = ticket_order
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
                    for (key, ticket) in slot_keys.into_iter().zip(ticket_order.iter().copied()) {
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
            if state.active.is_some_and(|active| active.ticket == ticket) {
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
        state.active = None;
    }

    pub async fn skip_active(&self) {
        let should_dispatch = {
            let mut state = self.inner.state.lock().await;
            let Some(active) = state.active.take() else {
                return;
            };
            state.entries.remove(&active.ticket);
            true
        };
        if should_dispatch {
            self.try_dispatch().await;
        }
    }

    async fn try_dispatch(&self) {
        let dispatched = {
            let mut state = self.inner.state.lock().await;
            if state.active.is_some()
                || state.media_busy
                || state.music_busy
                || (state.tts_busy && !state.messages_pinned)
            {
                return;
            }
            let Some((&key, &ticket)) = state.ordered.iter().find(|(_, ticket)| {
                !state.messages_pinned
                    || state
                        .entries
                        .get(ticket)
                        .is_none_or(|entry| entry.lane != StageLane::Tts)
            }) else {
                return;
            };
            let Some(entry) = state.entries.get_mut(&ticket) else {
                state.ordered.remove(&key);
                return;
            };
            let Some(event) = entry.event.take() else {
                return;
            };
            let lane = entry.lane;
            state.ordered.remove(&key);
            state.active = Some(ActiveTicket {
                ticket,
                lane,
                saw_busy: false,
            });
            Some((ticket, event))
        };
        if let Some((ticket, event)) = dispatched {
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
                let active_ticket = state.active.is_some_and(|active| active.ticket == ticket);
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
                    .is_some_and(|active| active.ticket == ticket && !active.saw_busy)
                {
                    return;
                }
                state.active = None;
                state.entries.remove(&ticket);
                true
            };
            if released {
                scheduler.try_dispatch().await;
            }
        });
    }

    fn demote_group(&self, state: &mut SchedulerState, original_key: StageOrderKey) -> bool {
        let group = message_group(original_key);
        let tickets = state
            .entries
            .iter()
            .filter_map(|(ticket, entry)| {
                (message_group(entry.original_key) == group).then_some(*ticket)
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
        let mut tickets = state
            .entries
            .iter()
            .filter_map(|(ticket, entry)| {
                (message_group(entry.original_key) == group).then_some(*ticket)
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
        RelayEvent::Tts(TtsEvent { timestamp, id, .. }) => (*timestamp, id.clone(), 0),
        RelayEvent::Sticker(event) => (event.timestamp, event.message_id.clone(), 100),
        RelayEvent::Media(event) => (event.timestamp, event.message_id.clone(), 200),
        RelayEvent::MusicPlay(event) => (now_ms(), event.playback_id.clone(), 300),
        _ => (now_ms(), String::new(), u16::MAX),
    }
}

fn message_group(key: StageOrderKey) -> (u64, u64) {
    (key.timestamp_ms, key.message_id)
}

fn lane_busy(lane: StageLane, media_busy: bool, music_busy: bool, tts_busy: bool) -> bool {
    match lane {
        StageLane::Media => media_busy,
        StageLane::Tts => tts_busy,
        StageLane::Music => music_busy,
    }
}

#[cfg(test)]
mod tests;
