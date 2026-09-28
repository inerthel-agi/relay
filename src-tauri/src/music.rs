use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

use crate::{
    model::{MusicPlaybackEvent, MusicPlaybackMode},
    youtube::YouTubeTrack,
};

const SEARCH_TTL: Duration = Duration::from_secs(120);
const SELECTION_TTL: Duration = Duration::from_secs(120);
const PREVIEW_DURATION_SECONDS: u64 = 30;
/// Per Discord user: minimum gap between YouTube Data API searches.
/// Mid-range of 5–8s — slows quota spam without blocking pick/play after results.
pub const MUSIC_SEARCH_COOLDOWN: Duration = Duration::from_secs(6);
pub const CUSTOM_MAX_WINDOW_SECONDS: u64 = 60;
pub const MUSIC_QUEUE_CAP: usize = 20;
pub const DEFAULT_MUSIC_MAX_PENDING_PER_USER: u8 = 3;
pub const MUSIC_MAX_PENDING_PER_USER_LIMIT: u8 = 10;

/// Admission rules for tracks waiting behind the currently playing track.
///
/// A value of zero disables the per-user pending limit. The upper bound is
/// normalized here as a safety net; persisted configuration validates the
/// same range before it reaches the runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MusicQueuePolicy {
    pub max_pending_per_user: u8,
    pub reject_duplicate_pending: bool,
}

impl Default for MusicQueuePolicy {
    fn default() -> Self {
        Self {
            max_pending_per_user: DEFAULT_MUSIC_MAX_PENDING_PER_USER,
            reject_duplicate_pending: true,
        }
    }
}

impl MusicQueuePolicy {
    pub const fn new(max_pending_per_user: u8, reject_duplicate_pending: bool) -> Self {
        Self {
            max_pending_per_user: if max_pending_per_user > MUSIC_MAX_PENDING_PER_USER_LIMIT {
                MUSIC_MAX_PENDING_PER_USER_LIMIT
            } else {
                max_pending_per_user
            },
            reject_duplicate_pending,
        }
    }

    /// Explicitly disable both optional admission guards.
    #[cfg(test)]
    pub const fn unlimited() -> Self {
        Self {
            max_pending_per_user: 0,
            reject_duplicate_pending: false,
        }
    }

    pub const fn normalized(self) -> Self {
        Self::new(self.max_pending_per_user, self.reject_duplicate_pending)
    }

    fn pending_limit(self) -> Option<usize> {
        (self.max_pending_per_user > 0).then_some(self.max_pending_per_user as usize)
    }
}

struct PendingSearch {
    owner_id: u64,
    channel_id: u64,
    query: String,
    results: Vec<YouTubeTrack>,
    expires_at: Instant,
}

struct PendingSelection {
    owner_id: u64,
    owner_name: String,
    channel_id: u64,
    track: YouTubeTrack,
    expires_at: Instant,
}

struct CurrentMusic {
    control_id: String,
    looping: bool,
    playback: MusicPlaybackEvent,
    owner_id: u64,
    channel_id: u64,
    now_playing_message_id: Option<u64>,
}

/// Result of stopping the current track, including Discord announce cleanup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoppedMusic {
    pub playback: MusicPlaybackEvent,
    pub channel_id: u64,
    pub now_playing_message_id: Option<u64>,
}

pub struct MusicCard {
    pub playback: MusicPlaybackEvent,
    pub control_id: String,
    pub looping: bool,
    pub channel_id: u64,
    pub message_id: u64,
    /// One-based position for pending cards; zero identifies the current card.
    pub position: usize,
}

#[derive(Default)]
pub struct MusicState {
    searches: HashMap<String, PendingSearch>,
    selections: HashMap<String, PendingSelection>,
    /// Last YouTube search attempt per Discord user id (API quota guard).
    search_cooldowns: HashMap<u64, Instant>,
    current: Option<CurrentMusic>,
    pending: VecDeque<CurrentMusic>,
    next_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MusicStartResult {
    Started(MusicPlaybackEvent),
    Queued {
        playback: MusicPlaybackEvent,
        position: usize,
    },
    QueueFull,
    UserQueueFull {
        limit: usize,
    },
    DuplicatePending {
        video_id: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MusicQueueDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MusicQueueMove {
    Moved { position: usize },
    AtBoundary,
    NotFound,
    Current,
    SchedulerUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MusicSkipDecision {
    Allowed,
    NotOwner,
    NotCurrent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicSelection {
    pub owner_id: u64,
    pub owner_name: String,
    pub channel_id: u64,
    pub track: YouTubeTrack,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchSelection {
    Selected(String),
    NotFound,
    NotOwner,
    InvalidVideo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionTake {
    Taken(MusicSelection),
    NotFound,
    NotOwner,
}

impl MusicState {
    pub fn pending_events(&self) -> Vec<MusicPlaybackEvent> {
        self.pending
            .iter()
            .map(|item| item.playback.clone())
            .collect()
    }

    pub fn pending_playback_ids(&self) -> Vec<String> {
        self.pending
            .iter()
            .map(|item| item.playback.playback_id.clone())
            .collect()
    }

    pub fn is_pending(&self, playback_id: &str) -> bool {
        self.pending
            .iter()
            .any(|entry| entry.playback.playback_id == playback_id)
    }

    pub fn pending_cards(&self) -> Vec<MusicCard> {
        self.pending
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                Some(MusicCard {
                    playback: entry.playback.clone(),
                    control_id: entry.control_id.clone(),
                    looping: entry.looping,
                    channel_id: entry.channel_id,
                    message_id: entry.now_playing_message_id?,
                    position: index + 1,
                })
            })
            .collect()
    }

    /// Move a waiting track one position while leaving the current track
    /// untouched. The caller is responsible for synchronizing the scheduler
    /// ticket order after this in-memory operation succeeds.
    pub fn move_pending(
        &mut self,
        playback_id: &str,
        direction: MusicQueueDirection,
    ) -> MusicQueueMove {
        if self
            .current
            .as_ref()
            .is_some_and(|entry| entry.playback.playback_id == playback_id)
        {
            return MusicQueueMove::Current;
        }
        let Some(index) = self
            .pending
            .iter()
            .position(|entry| entry.playback.playback_id == playback_id)
        else {
            return MusicQueueMove::NotFound;
        };
        let target = match direction {
            MusicQueueDirection::Up => index.checked_sub(1),
            MusicQueueDirection::Down => (index + 1 < self.pending.len()).then_some(index + 1),
        };
        let Some(target) = target else {
            return MusicQueueMove::AtBoundary;
        };
        self.pending.swap(index, target);
        MusicQueueMove::Moved {
            position: target + 1,
        }
    }

    /// Restore a previously captured order after a scheduler update fails.
    /// The set of IDs must match exactly, including cardinality.
    pub fn reorder_pending_by_ids(&mut self, playback_ids: &[String]) -> bool {
        if playback_ids.len() != self.pending.len() {
            return false;
        }
        if playback_ids.iter().any(|playback_id| {
            playback_ids
                .iter()
                .filter(|other| *other == playback_id)
                .count()
                != 1
        }) || self.pending.iter().any(|entry| {
            !playback_ids
                .iter()
                .any(|playback_id| playback_id == &entry.playback.playback_id)
        }) {
            return false;
        }
        let mut remaining = self.pending.drain(..).collect::<Vec<_>>();
        let mut reordered = VecDeque::with_capacity(remaining.len());
        for playback_id in playback_ids {
            let Some(index) = remaining
                .iter()
                .position(|entry| entry.playback.playback_id == *playback_id)
            else {
                unreachable!("pending IDs were validated before reordering");
            };
            reordered.push_back(remaining.swap_remove(index));
        }
        self.pending = reordered;
        true
    }

    /// Remaining wait before this user may call YouTube search again.
    /// Does not affect select / preview / full / custom / queue play.
    pub fn search_cooldown_remaining(&self, user_id: u64, now: Instant) -> Option<Duration> {
        remaining_search_cooldown(
            self.search_cooldowns.get(&user_id).copied(),
            now,
            MUSIC_SEARCH_COOLDOWN,
        )
    }

    /// Record a search attempt (call when about to hit the YouTube API).
    pub fn mark_search_attempt(&mut self, user_id: u64, now: Instant) {
        self.prune_search_cooldowns(now);
        self.search_cooldowns.insert(user_id, now);
    }

    pub fn insert_search(
        &mut self,
        owner_id: u64,
        channel_id: u64,
        query: String,
        results: Vec<YouTubeTrack>,
    ) -> String {
        self.prune(Instant::now());
        let search_id = self.next_id("s");
        self.searches.insert(
            search_id.clone(),
            PendingSearch {
                owner_id,
                channel_id,
                query,
                results,
                expires_at: Instant::now() + SEARCH_TTL,
            },
        );
        search_id
    }

    pub fn select_search(
        &mut self,
        search_id: &str,
        user_id: u64,
        owner_name: &str,
        video_id: &str,
    ) -> SearchSelection {
        self.prune(Instant::now());
        let Some(search) = self.searches.get(search_id) else {
            return SearchSelection::NotFound;
        };
        if search.owner_id != user_id {
            return SearchSelection::NotOwner;
        }
        let Some(track) = search
            .results
            .iter()
            .find(|track| track.video_id == video_id)
            .cloned()
        else {
            return SearchSelection::InvalidVideo;
        };
        let search = self
            .searches
            .remove(search_id)
            .expect("search was checked immediately before removal");
        let selection_id = self.next_id("t");
        let owner_name = owner_name.trim();
        let owner_name = if owner_name.is_empty() {
            format!("User {}", search.owner_id)
        } else {
            owner_name.to_owned()
        };
        self.selections.insert(
            selection_id.clone(),
            PendingSelection {
                owner_id: search.owner_id,
                owner_name,
                channel_id: search.channel_id,
                track,
                expires_at: Instant::now() + SELECTION_TTL,
            },
        );
        SearchSelection::Selected(selection_id)
    }

    pub fn selection_duration_seconds(&self, selection_id: &str) -> Option<u64> {
        self.selections
            .get(selection_id)
            .map(|selection| selection.track.duration_seconds)
    }

    pub fn peek_selection(&self, selection_id: &str, user_id: u64) -> SelectionTake {
        let Some(selection) = self.selections.get(selection_id) else {
            return SelectionTake::NotFound;
        };
        if selection.owner_id != user_id {
            return SelectionTake::NotOwner;
        }
        SelectionTake::Taken(MusicSelection {
            owner_id: selection.owner_id,
            owner_name: selection.owner_name.clone(),
            channel_id: selection.channel_id,
            track: selection.track.clone(),
        })
    }

    pub fn touch_selection(&mut self, selection_id: &str, user_id: u64) -> bool {
        self.prune(Instant::now());
        let Some(selection) = self.selections.get_mut(selection_id) else {
            return false;
        };
        if selection.owner_id != user_id {
            return false;
        }
        selection.expires_at = Instant::now() + SELECTION_TTL;
        true
    }

    pub fn take_selection(&mut self, selection_id: &str, user_id: u64) -> SelectionTake {
        self.prune(Instant::now());
        let Some(selection) = self.selections.get(selection_id) else {
            return SelectionTake::NotFound;
        };
        if selection.owner_id != user_id {
            return SelectionTake::NotOwner;
        }
        let selection = self
            .selections
            .remove(selection_id)
            .expect("selection was checked immediately before removal");
        SelectionTake::Taken(MusicSelection {
            owner_id: selection.owner_id,
            owner_name: selection.owner_name,
            channel_id: selection.channel_id,
            track: selection.track,
        })
    }

    pub fn restore_selection(&mut self, selection_id: &str, selection: MusicSelection) {
        self.prune(Instant::now());
        self.selections
            .entry(selection_id.to_owned())
            .or_insert(PendingSelection {
                owner_id: selection.owner_id,
                owner_name: selection.owner_name,
                channel_id: selection.channel_id,
                track: selection.track,
                expires_at: Instant::now() + SELECTION_TTL,
            });
    }

    pub fn cancel_selection(&mut self, selection_id: &str, user_id: u64) -> SelectionTake {
        self.take_selection(selection_id, user_id)
    }

    #[cfg(test)]
    pub fn start(
        &mut self,
        selection: MusicSelection,
        mode: MusicPlaybackMode,
    ) -> MusicStartResult {
        self.start_with_policy(selection, mode, MusicQueuePolicy::default())
    }

    pub fn start_with_policy(
        &mut self,
        selection: MusicSelection,
        mode: MusicPlaybackMode,
        policy: MusicQueuePolicy,
    ) -> MusicStartResult {
        match mode {
            MusicPlaybackMode::Preview => {
                let end = selection
                    .track
                    .duration_seconds
                    .min(PREVIEW_DURATION_SECONDS);
                self.start_range(selection, mode, 0, Some(end), policy)
            }
            MusicPlaybackMode::Full => self.start_range(selection, mode, 0, None, policy),
            MusicPlaybackMode::Custom => {
                unreachable!("custom clips must go through start_custom")
            }
        }
    }

    #[cfg(test)]
    pub fn start_custom(
        &mut self,
        selection: MusicSelection,
        start_seconds: u64,
        end_seconds: u64,
    ) -> Result<MusicStartResult, CustomRangeError> {
        self.start_custom_with_policy(
            selection,
            start_seconds,
            end_seconds,
            MusicQueuePolicy::default(),
        )
    }

    pub fn start_custom_with_policy(
        &mut self,
        selection: MusicSelection,
        start_seconds: u64,
        end_seconds: u64,
        policy: MusicQueuePolicy,
    ) -> Result<MusicStartResult, CustomRangeError> {
        let (start_seconds, end_seconds) =
            validate_custom_range(selection.track.duration_seconds, start_seconds, end_seconds)?;
        Ok(self.start_range(
            selection,
            MusicPlaybackMode::Custom,
            start_seconds,
            Some(end_seconds),
            policy,
        ))
    }

    fn start_range(
        &mut self,
        selection: MusicSelection,
        mode: MusicPlaybackMode,
        start_seconds: u64,
        end_seconds: Option<u64>,
        policy: MusicQueuePolicy,
    ) -> MusicStartResult {
        let policy = policy.normalized();
        if let Some(rejection) = self.admission_rejection(&selection, policy) {
            return rejection;
        }
        if self.current.is_some() && self.pending.len() >= MUSIC_QUEUE_CAP {
            return MusicStartResult::QueueFull;
        }
        let playback_id = self.next_id("p");
        let playback = MusicPlaybackEvent {
            playback_id,
            video_id: selection.track.video_id,
            title: selection.track.title,
            channel_title: selection.track.channel_title,
            thumbnail: selection.track.thumbnail,
            duration_seconds: selection.track.duration_seconds,
            mode,
            start_seconds,
            end_seconds,
            requested_by: selection.owner_name,
        };
        let entry = CurrentMusic {
            control_id: playback.playback_id.clone(),
            looping: false,
            playback: playback.clone(),
            owner_id: selection.owner_id,
            channel_id: selection.channel_id,
            now_playing_message_id: None,
        };
        if self.current.is_none() {
            self.current = Some(entry);
            MusicStartResult::Started(playback)
        } else {
            self.pending.push_back(entry);
            MusicStartResult::Queued {
                position: self.pending.len(),
                playback,
            }
        }
    }

    fn admission_rejection(
        &self,
        selection: &MusicSelection,
        policy: MusicQueuePolicy,
    ) -> Option<MusicStartResult> {
        if let Some(limit) = policy.pending_limit()
            && self
                .pending
                .iter()
                .filter(|entry| entry.owner_id == selection.owner_id)
                .count()
                >= limit
        {
            return Some(MusicStartResult::UserQueueFull { limit });
        }

        let video_id = canonical_music_id(&selection.track.video_id);
        if policy.reject_duplicate_pending
            && !video_id.is_empty()
            && self
                .pending
                .iter()
                .any(|entry| canonical_music_id(&entry.playback.video_id) == video_id)
        {
            return Some(MusicStartResult::DuplicatePending { video_id });
        }
        None
    }

    pub fn set_now_playing_message_id(&mut self, playback_id: &str, message_id: u64) -> bool {
        let Some(current) = self
            .current
            .iter_mut()
            .chain(self.pending.iter_mut())
            .find(|entry| entry.playback.playback_id == playback_id)
        else {
            return false;
        };
        if current.playback.playback_id != playback_id {
            return false;
        }
        current.now_playing_message_id = Some(message_id);
        true
    }

    pub fn current_event(&self) -> Option<MusicPlaybackEvent> {
        self.current
            .as_ref()
            .map(|current| current.playback.clone())
    }

    pub fn active_message_ids(&self) -> Vec<(u64, u64)> {
        self.current
            .iter()
            .chain(self.pending.iter())
            .filter_map(|entry| {
                entry
                    .now_playing_message_id
                    .map(|id| (entry.channel_id, id))
            })
            .collect()
    }

    pub fn current_card(&self) -> Option<MusicCard> {
        let entry = self.current.as_ref()?;
        Some(MusicCard {
            playback: entry.playback.clone(),
            control_id: entry.control_id.clone(),
            looping: entry.looping,
            channel_id: entry.channel_id,
            message_id: entry.now_playing_message_id?,
            position: 0,
        })
    }

    pub fn toggle_loop(
        &mut self,
        control_id: &str,
        user_id: u64,
    ) -> Result<bool, MusicSkipDecision> {
        let entry = self
            .current
            .iter_mut()
            .chain(self.pending.iter_mut())
            .find(|entry| entry.control_id == control_id)
            .ok_or(MusicSkipDecision::NotCurrent)?;
        if entry.owner_id != user_id {
            return Err(MusicSkipDecision::NotOwner);
        }
        entry.looping = !entry.looping;
        Ok(entry.looping)
    }

    pub fn control_playback_id(
        &self,
        control_id: &str,
        user_id: u64,
    ) -> Result<String, MusicSkipDecision> {
        let entry = self
            .current
            .iter()
            .chain(self.pending.iter())
            .find(|entry| entry.control_id == control_id)
            .ok_or(MusicSkipDecision::NotCurrent)?;
        if entry.owner_id != user_id {
            return Err(MusicSkipDecision::NotOwner);
        }
        if self
            .current
            .as_ref()
            .is_some_and(|current| current.control_id == control_id)
            && self.skip_decision(&entry.playback.playback_id, user_id)
                != MusicSkipDecision::Allowed
        {
            return Err(MusicSkipDecision::NotOwner);
        }
        Ok(entry.playback.playback_id.clone())
    }

    pub fn remove_pending(&mut self, playback_id: &str) -> Option<StoppedMusic> {
        let index = self
            .pending
            .iter()
            .position(|entry| entry.playback.playback_id == playback_id)?;
        let entry = self.pending.remove(index)?;
        Some(StoppedMusic {
            playback: entry.playback,
            channel_id: entry.channel_id,
            now_playing_message_id: entry.now_playing_message_id,
        })
    }

    pub fn waiting_repeat_id(&self, control_id: &str) -> Option<String> {
        self.pending
            .iter()
            .find(|entry| {
                entry.control_id == control_id
                    && entry.playback.playback_id != control_id
                    && !entry.looping
            })
            .map(|entry| entry.playback.playback_id.clone())
    }

    /// Only a natural end may repeat. A fresh ID makes duplicate end events harmless.
    pub fn finish_current(&mut self, playback_id: &str) -> Option<(StoppedMusic, bool)> {
        if self.current.as_ref()?.playback.playback_id != playback_id {
            return None;
        }
        let mut entry = self.current.take()?;
        let stopped = StoppedMusic {
            playback: entry.playback.clone(),
            channel_id: entry.channel_id,
            now_playing_message_id: entry.now_playing_message_id,
        };
        let repeat = entry.looping;
        if repeat {
            entry.playback.playback_id = self.next_id("p");
            self.pending.push_back(entry);
        }
        Some((stopped, repeat))
    }

    /// Clears the current track only (does not touch the pending queue).
    pub fn stop_current(&mut self) -> Option<StoppedMusic> {
        self.current.take().map(|current| StoppedMusic {
            playback: current.playback,
            channel_id: current.channel_id,
            now_playing_message_id: current.now_playing_message_id,
        })
    }

    pub fn stop_if_current(&mut self, playback_id: &str) -> Option<StoppedMusic> {
        if self
            .current
            .as_ref()
            .is_some_and(|current| current.playback.playback_id == playback_id)
        {
            return self.stop_current();
        }
        None
    }

    /// Promotes the next queued track to current, if any.
    pub fn promote_next(&mut self) -> Option<MusicPlaybackEvent> {
        let next = self.pending.pop_front()?;
        let playback = next.playback.clone();
        self.current = Some(next);
        Some(playback)
    }

    /// Stops the current track and drops the entire pending queue.
    pub fn clear_all(&mut self) -> Option<StoppedMusic> {
        self.pending.clear();
        self.stop_current()
    }

    pub fn skip_decision(&self, playback_id: &str, user_id: u64) -> MusicSkipDecision {
        match self.current.as_ref() {
            Some(current) if current.playback.playback_id == playback_id => {
                if current.owner_id == user_id {
                    MusicSkipDecision::Allowed
                } else {
                    MusicSkipDecision::NotOwner
                }
            }
            _ => MusicSkipDecision::NotCurrent,
        }
    }

    fn next_id(&mut self, prefix: &str) -> String {
        self.next_id = self.next_id.wrapping_add(1).max(1);
        format!("{prefix}{:x}", self.next_id)
    }

    fn prune(&mut self, now: Instant) {
        self.searches
            .retain(|_, search| search.expires_at > now && !search.query.is_empty());
        self.selections
            .retain(|_, selection| selection.expires_at > now);
        self.prune_search_cooldowns(now);
    }

    fn prune_search_cooldowns(&mut self, now: Instant) {
        self.search_cooldowns
            .retain(|_, last| now.saturating_duration_since(*last) < MUSIC_SEARCH_COOLDOWN);
    }
}

/// Pure helper: how long until `cooldown` elapses since `last_at`.
pub fn remaining_search_cooldown(
    last_at: Option<Instant>,
    now: Instant,
    cooldown: Duration,
) -> Option<Duration> {
    let last_at = last_at?;
    let elapsed = now.saturating_duration_since(last_at);
    if elapsed >= cooldown {
        None
    } else {
        Some(cooldown - elapsed)
    }
}

/// Whole seconds to show in the user-facing cooldown message (ceil, at least 1).
pub fn cooldown_wait_seconds(remaining: Duration) -> u64 {
    let millis = remaining.as_millis();
    (millis.div_ceil(1000) as u64).max(1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CustomRangeError {
    EmptyRange,
    WindowTooLong,
    OutsideTrack,
}

pub fn parse_timestamp(value: &str) -> Option<u64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(seconds);
    }
    let parts = value.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [minutes, seconds] => {
            let minutes = minutes.parse::<u64>().ok()?;
            let seconds = seconds.parse::<u64>().ok()?;
            if seconds >= 60 || minutes > 59 {
                return None;
            }
            Some(minutes.checked_mul(60)?.checked_add(seconds)?)
        }
        [hours, minutes, seconds] => {
            let hours = hours.parse::<u64>().ok()?;
            let minutes = minutes.parse::<u64>().ok()?;
            let seconds = seconds.parse::<u64>().ok()?;
            if seconds >= 60 || minutes >= 60 || hours > 2 {
                return None;
            }
            Some(
                hours
                    .checked_mul(3_600)?
                    .checked_add(minutes.checked_mul(60)?)?
                    .checked_add(seconds)?,
            )
        }
        _ => None,
    }
}

pub fn validate_custom_range(
    track_duration_seconds: u64,
    start_seconds: u64,
    end_seconds: u64,
) -> Result<(u64, u64), CustomRangeError> {
    if end_seconds <= start_seconds {
        return Err(CustomRangeError::EmptyRange);
    }
    if end_seconds - start_seconds > CUSTOM_MAX_WINDOW_SECONDS {
        return Err(CustomRangeError::WindowTooLong);
    }
    if start_seconds >= track_duration_seconds || end_seconds > track_duration_seconds {
        return Err(CustomRangeError::OutsideTrack);
    }
    Ok((start_seconds, end_seconds))
}

/// Return the stable YouTube video identifier used for queue comparisons.
/// Search results already contain the bare ID, but accepting the common watch
/// and short-link forms keeps the duplicate guard correct for imported tests or
/// future non-YouTube search providers. YouTube IDs remain case-sensitive.
pub fn canonical_music_id(value: &str) -> String {
    let value = value.trim();
    let value = value
        .strip_prefix("https://www.youtube.com/watch?v=")
        .or_else(|| value.strip_prefix("https://youtube.com/watch?v="))
        .or_else(|| value.strip_prefix("https://youtu.be/"))
        .unwrap_or(value);
    value
        .split(['&', '?', '#'])
        .next()
        .unwrap_or(value)
        .trim_end_matches('/')
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests;
