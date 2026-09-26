//! Moderation layer on top of the privacy filter: built-in word lists, link
//! rules, spam and raid guards, account age, trusted and blocked members,
//! presets and the local decision log. Everything stays on this PC; nothing
//! here ever stores message text.

pub mod live;
pub mod log;
pub mod messages;
pub mod packs;
pub mod presets;
#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::privacy::ForbiddenConcept;
pub use packs::WordPack;
pub use presets::{ModerationPreset, PresetFields};

pub const MAX_MODERATION_IDS: usize = 200;
pub const MAX_PACK_EXCLUSIONS: usize = 200;
/// Discord epoch, used to read an account's creation date from its ID.
const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;
const DAY_MS: u64 = 86_400_000;
const RUNTIME_ENTRY_LIMIT: usize = 2_000;
const DUPLICATE_WINDOW_MS: u64 = 10 * 60 * 1_000;
const RAID_WINDOW_MS: u64 = 60 * 1_000;
const VERDICT_TTL_MS: u64 = 10 * 60 * 1_000;
const WARNING_INTERVAL_MS: u64 = 10 * 60 * 1_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Lane {
    Media,
    Notifications,
    Music,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FilterScopes {
    pub media: bool,
    pub notifications: bool,
    pub music: bool,
}

impl Default for FilterScopes {
    fn default() -> Self {
        Self {
            media: true,
            notifications: true,
            music: true,
        }
    }
}

impl FilterScopes {
    pub fn covers(self, lane: Lane) -> bool {
        match lane {
            Lane::Media => self.media,
            Lane::Notifications => self.notifications,
            Lane::Music => self.music,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ModerationSettings {
    pub word_packs: Vec<WordPack>,
    /// Pack words the streamer turned off after a false positive.
    pub pack_exclusions: Vec<String>,
    pub filter_scopes: FilterScopes,
    pub filter_usernames: bool,
    pub filter_music_titles: bool,
    pub user_cooldown_seconds: u16,
    pub raid_limit_per_minute: u16,
    pub block_duplicates: bool,
    pub block_text_spam: bool,
    pub min_account_age_days: u16,
    pub min_member_age_days: u16,
    pub trusted_role_ids: Vec<String>,
    pub trusted_user_ids: Vec<String>,
    pub blocked_user_ids: Vec<String>,
    pub block_invites: bool,
    pub block_shorteners: bool,
    pub block_scam_domains: bool,
    /// Media types left unchecked in manual moderation pass instead of being dropped.
    pub review_only_selected: bool,
    /// Messages flagged for review wait in the queue instead of disappearing.
    pub review_text: bool,
    pub pending_expiry_minutes: u16,
    pub safety_delay_seconds: u8,
    pub warn_on_block: bool,
    pub escalation_blocks: u8,
    pub escalation_window_minutes: u16,
    pub escalation_timeout_minutes: u16,
    pub honeypot_exempt_role_ids: Vec<String>,
    pub honeypot_timeout_minutes: u16,
    pub live_preset: Option<ModerationPreset>,
    pub live_obs_port: u16,
    /// Settings to restore when the stream ends, while the live preset is on.
    pub live_restore: Option<PresetFields>,
    pub max_media_seconds: u16,
    pub loudness_limiter: bool,
}

impl Default for ModerationSettings {
    fn default() -> Self {
        Self {
            word_packs: Vec::new(),
            pack_exclusions: Vec::new(),
            filter_scopes: FilterScopes::default(),
            filter_usernames: true,
            filter_music_titles: true,
            user_cooldown_seconds: 0,
            raid_limit_per_minute: 0,
            block_duplicates: false,
            block_text_spam: false,
            min_account_age_days: 0,
            min_member_age_days: 0,
            trusted_role_ids: Vec::new(),
            trusted_user_ids: Vec::new(),
            blocked_user_ids: Vec::new(),
            block_invites: false,
            block_shorteners: false,
            block_scam_domains: false,
            review_only_selected: false,
            review_text: true,
            pending_expiry_minutes: 0,
            safety_delay_seconds: 0,
            warn_on_block: false,
            escalation_blocks: 0,
            escalation_window_minutes: 10,
            escalation_timeout_minutes: 10,
            honeypot_exempt_role_ids: Vec::new(),
            honeypot_timeout_minutes: 1_440,
            live_preset: None,
            live_obs_port: crate::obs::DEFAULT_OBS_PORT,
            live_restore: None,
            max_media_seconds: 0,
            loudness_limiter: false,
        }
    }
}

impl ModerationSettings {
    pub fn validate(&self) -> Result<()> {
        let unique_packs = self
            .word_packs
            .iter()
            .collect::<std::collections::HashSet<_>>();
        if unique_packs.len() != self.word_packs.len() {
            bail!("Word lists are duplicated.");
        }
        if self.pack_exclusions.len() > MAX_PACK_EXCLUSIONS
            || self
                .pack_exclusions
                .iter()
                .any(|word| word.trim().is_empty() || word.chars().count() > 64)
        {
            bail!(
                "Word list exceptions must contain at most {MAX_PACK_EXCLUSIONS} words of 1 to 64 characters."
            );
        }
        if self.user_cooldown_seconds > 3_600 {
            bail!("The delay between two messages must be between 0 and 3600 seconds.");
        }
        if self.raid_limit_per_minute > 120 {
            bail!("The raid limit must be between 0 and 120 media per minute.");
        }
        if self.min_account_age_days > 365 || self.min_member_age_days > 365 {
            bail!("Account and member ages must be between 0 and 365 days.");
        }
        for (list, label) in [
            (&self.trusted_role_ids, "trusted role"),
            (&self.trusted_user_ids, "trusted member"),
            (&self.blocked_user_ids, "blocked member"),
            (&self.honeypot_exempt_role_ids, "trap exempt role"),
        ] {
            if list.len() > MAX_MODERATION_IDS {
                bail!("At most {MAX_MODERATION_IDS} {label} IDs may be configured.");
            }
            for id in list {
                if !(17..=20).contains(&id.len()) || !id.bytes().all(|byte| byte.is_ascii_digit()) {
                    bail!("Each {label} ID must be a Discord ID of 17 to 20 digits.");
                }
            }
        }
        if self.pending_expiry_minutes > 1_440 {
            bail!("Queue expiry must be between 0 and 1440 minutes.");
        }
        if self.safety_delay_seconds > 30 {
            bail!("The safety delay must be between 0 and 30 seconds.");
        }
        if self.escalation_blocks > 20
            || !(1..=1_440).contains(&self.escalation_window_minutes)
            || !(1..=40_320).contains(&self.escalation_timeout_minutes)
        {
            bail!(
                "Automatic timeouts need 0 to 20 blocks, a 1 to 1440 minute window and a 1 minute to 28 day timeout."
            );
        }
        if !(1..=40_320).contains(&self.honeypot_timeout_minutes) {
            bail!("The trap timeout must be between 1 minute and 28 days.");
        }
        if self.live_obs_port == 0 {
            bail!("The OBS WebSocket port is invalid.");
        }
        if self.max_media_seconds > 600 {
            bail!("The maximum media length must be between 0 and 600 seconds.");
        }
        Ok(())
    }

    pub fn links_filtered(&self) -> bool {
        self.block_invites || self.block_shorteners || self.block_scam_domains
    }
}

/// Custom filter words plus the enabled built-in lists, minus exceptions.
pub fn effective_concepts(config: &AppConfig) -> Cow<'_, [ForbiddenConcept]> {
    let settings = &config.moderation;
    if settings.word_packs.is_empty() {
        return Cow::Borrowed(&config.privacy_concepts);
    }
    let excluded = settings
        .pack_exclusions
        .iter()
        .map(|word| word.trim().to_lowercase())
        .collect::<Vec<_>>();
    let mut concepts = config.privacy_concepts.clone();
    for pack in &settings.word_packs {
        concepts.extend(
            pack.concepts()
                .iter()
                .filter(|concept| !excluded.contains(&concept.canonical.to_lowercase()))
                .cloned(),
        );
    }
    Cow::Owned(concepts)
}

/// Rules that apply to text even when the image scan is off.
pub fn content_rules_enabled(config: &AppConfig) -> bool {
    !config.moderation.word_packs.is_empty() || config.moderation.links_filtered()
}

/// Removes every text rule an exempt role or an out-of-scope channel skips.
pub fn clear_content_rules(config: &mut AppConfig) {
    config.privacy_concepts.clear();
    config.moderation.word_packs.clear();
    config.moderation.block_invites = false;
    config.moderation.block_shorteners = false;
    config.moderation.block_scam_domains = false;
}

/// Applies the per-channel filter scope (Moderation → word filters).
pub fn scope_config(mut config: AppConfig, lane: Lane) -> AppConfig {
    if !config.moderation.filter_scopes.covers(lane) {
        clear_content_rules(&mut config);
    }
    config
}

const SHORTENER_HOSTS: &[&str] = &[
    "bit.ly",
    "tinyurl.com",
    "goo.gl",
    "is.gd",
    "cutt.ly",
    "rb.gy",
    "shorturl.at",
    "ow.ly",
    "tiny.cc",
    "s.id",
    "rebrand.ly",
    "t.ly",
    "v.gd",
    "shorturl.gg",
];

/// Look-alike fragments seen in fake Nitro and Steam gift links.
const SCAM_HOST_FRAGMENTS: &[&str] = &[
    "dlscord",
    "discorcl",
    "disccord",
    "discordd",
    "dicsord",
    "discrod",
    "discorb",
    "discord-nitro",
    "discordnitro",
    "discord-gift",
    "discordgift",
    "nitro-gift",
    "steamcommunlty",
    "stearncommunity",
    "steamcomunity",
    "steamcommunitty",
    "steam-gift",
    "steamgift",
    "steampowered-gift",
];

/// Official domains that contain scam fragments by design.
const TRUSTED_HOSTS: &[&str] = &[
    "discord.com",
    "discord.gg",
    "discord.gift",
    "discordapp.com",
    "discordapp.net",
    "discord.media",
    "steamcommunity.com",
    "steampowered.com",
    "steamstatic.com",
];

/// Returns the reason code of the first link rule the text breaks.
pub fn link_violation(text: &str, settings: &ModerationSettings) -> Option<&'static str> {
    if !settings.links_filtered() {
        return None;
    }
    for host_and_path in link_candidates(text) {
        let (host, path) = host_and_path
            .split_once('/')
            .map_or((host_and_path.as_str(), ""), |(host, path)| (host, path));
        let host = host.trim_start_matches("www.");
        if settings.block_invites
            && (matches!(host, "discord.gg" | "dsc.gg")
                || (matches!(host, "discord.com" | "discordapp.com")
                    && path.starts_with("invite/")))
        {
            return Some("discord_invite");
        }
        if settings.block_shorteners && SHORTENER_HOSTS.contains(&host) {
            return Some("link_shortener");
        }
        if settings.block_scam_domains
            && !TRUSTED_HOSTS
                .iter()
                .any(|trusted| host == *trusted || host.ends_with(&format!(".{trusted}")))
            && SCAM_HOST_FRAGMENTS
                .iter()
                .any(|fragment| host.contains(fragment))
        {
            return Some("scam_domain");
        }
    }
    None
}

/// Lowercase "host/path" pieces for every link-like token in the text.
fn link_candidates(text: &str) -> Vec<String> {
    text.split(|character: char| {
        character.is_whitespace() || matches!(character, '<' | '>' | '(' | ')' | '"' | '\'')
    })
    .filter_map(|token| {
        let token = token
            .trim_matches(|character: char| matches!(character, '.' | ',' | ';' | '!' | '?'))
            .to_lowercase();
        let url = if token.starts_with("https://") || token.starts_with("http://") {
            reqwest::Url::parse(&token).ok()?
        } else {
            // Bare email addresses are not links. Explicit URLs may have userinfo.
            if token.contains('@') || token.contains("://") {
                return None;
            }
            reqwest::Url::parse(&format!("https://{token}")).ok()?
        };
        let host = url.host_str()?.trim_end_matches('.');
        let looks_like_host = host.contains('.')
            && host
                .rsplit('.')
                .next()
                .is_some_and(|tld| tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()))
            && host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'));
        looks_like_host.then(|| format!("{host}{}", url.path()))
    })
    .collect()
}

/// Spam patterns for notifications read on stream.
pub fn text_spam_reason(text: &str, mention_count: usize) -> Option<&'static str> {
    if mention_count > 5 {
        return Some("mention_spam");
    }
    let letters = text
        .chars()
        .filter(|character| character.is_alphabetic())
        .count();
    let uppercase = text
        .chars()
        .filter(|character| character.is_uppercase())
        .count();
    if letters >= 20 && uppercase * 4 >= letters * 3 {
        return Some("caps_spam");
    }
    let custom_emoji = text.matches("<:").count() + text.matches("<a:").count();
    let unicode_emoji = text
        .chars()
        .filter(|character| matches!(u32::from(*character), 0x1F300..=0x1FAFF | 0x2600..=0x27BF))
        .count();
    if custom_emoji + unicode_emoji > 12 {
        return Some("emoji_spam");
    }
    let mut run = 1;
    let mut previous = None;
    for character in text.chars() {
        if Some(character) == previous && !character.is_whitespace() {
            run += 1;
            if run >= 15 {
                return Some("repeated_characters");
            }
        } else {
            run = 1;
        }
        previous = Some(character);
    }
    None
}

pub fn account_created_ms(user_id: u64) -> u64 {
    (user_id >> 22) + DISCORD_EPOCH_MS
}

pub fn anonymous_author(
    mut author: crate::model::AuthorIdentity,
    config: &AppConfig,
) -> crate::model::AuthorIdentity {
    if config.moderation.filter_usernames
        && crate::privacy::filter_words_match(&author.username, config)
    {
        author.username = "Anonymous".into();
        author.display_avatar_url = "https://cdn.discordapp.com/embed/avatars/0.png".into();
    }
    author
}

/// What the gate knows about one Discord message before any scan runs.
pub struct GateInput<'a> {
    pub lane: Lane,
    pub user_id: u64,
    pub role_ids: &'a [String],
    pub member_joined_ms: Option<u64>,
    pub mention_count: usize,
    pub text: &'a str,
    /// Stable keys of the attachments, stickers or links (duplicate detection).
    pub media_keys: Vec<String>,
    pub now_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateDecision {
    /// Nothing to add; `trusted` members also skip manual review.
    Pass { trusted: bool },
    /// Show only after a moderator approves it.
    Hold(&'static str),
    /// Ignore the message; the reason goes to the decision log.
    Drop(&'static str),
}

/// Remembered per message so the state layer can apply holds and trust.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MessageVerdict {
    pub rejected: bool,
    pub hold: Option<&'static str>,
    pub trusted: bool,
    pub author_id: String,
    pub channel_id: String,
    pub(crate) at_ms: u64,
}

#[derive(Default)]
pub struct ModerationRuntime {
    last_accepted: HashMap<(Lane, u64), u64>,
    media_window: VecDeque<u64>,
    raid_until_ms: u64,
    recent_media: VecDeque<(String, u64)>,
    blocks: HashMap<u64, VecDeque<u64>>,
    warned: HashMap<u64, u64>,
    verdicts: HashMap<String, MessageVerdict>,
    /// OBS stream state seen by the last live-mode check.
    pub live_streaming: bool,
}

/// What a block should trigger for the author.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BlockFollowUp {
    pub warn: bool,
    pub timeout_minutes: Option<u16>,
}

impl ModerationRuntime {
    pub fn evaluate(&mut self, input: &GateInput, settings: &ModerationSettings) -> GateDecision {
        self.prune(input.now_ms);
        let user = input.user_id.to_string();
        if settings.blocked_user_ids.contains(&user) {
            return GateDecision::Drop("blocked_user");
        }
        let trusted = settings.trusted_user_ids.contains(&user)
            || input
                .role_ids
                .iter()
                .any(|role| settings.trusted_role_ids.contains(role));
        if trusted {
            self.last_accepted
                .insert((input.lane, input.user_id), input.now_ms);
            return GateDecision::Pass { trusted: true };
        }
        let cooldown_ms = u64::from(settings.user_cooldown_seconds) * 1_000;
        if cooldown_ms > 0
            && matches!(input.lane, Lane::Media | Lane::Notifications)
            && self
                .last_accepted
                .get(&(input.lane, input.user_id))
                .is_some_and(|last| input.now_ms.saturating_sub(*last) < cooldown_ms)
        {
            return GateDecision::Drop("cooldown");
        }
        if settings.block_text_spam
            && input.lane == Lane::Notifications
            && let Some(reason) = text_spam_reason(input.text, input.mention_count)
        {
            return GateDecision::Drop(reason);
        }
        if settings.block_duplicates && input.lane == Lane::Media && !input.media_keys.is_empty() {
            let duplicate = input.media_keys.iter().all(|key| {
                self.recent_media.iter().any(|(recent, at)| {
                    recent == key && input.now_ms.saturating_sub(*at) < DUPLICATE_WINDOW_MS
                })
            });
            if duplicate {
                return GateDecision::Drop("duplicate");
            }
        }
        self.last_accepted
            .insert((input.lane, input.user_id), input.now_ms);
        if input.lane == Lane::Media {
            for key in &input.media_keys {
                self.recent_media.push_back((key.clone(), input.now_ms));
            }
        }
        let hold = self.hold_reason(input, settings);
        match hold {
            Some(reason) => GateDecision::Hold(reason),
            None => GateDecision::Pass { trusted: false },
        }
    }

    fn hold_reason(
        &mut self,
        input: &GateInput,
        settings: &ModerationSettings,
    ) -> Option<&'static str> {
        let min_account_ms = u64::from(settings.min_account_age_days) * DAY_MS;
        if min_account_ms > 0
            && input
                .now_ms
                .saturating_sub(account_created_ms(input.user_id))
                < min_account_ms
        {
            return Some("new_account");
        }
        let min_member_ms = u64::from(settings.min_member_age_days) * DAY_MS;
        if min_member_ms > 0
            && input
                .member_joined_ms
                .is_some_and(|joined| input.now_ms.saturating_sub(joined) < min_member_ms)
        {
            return Some("new_member");
        }
        if input.lane == Lane::Media && settings.raid_limit_per_minute > 0 {
            self.media_window.push_back(input.now_ms);
            while self
                .media_window
                .front()
                .is_some_and(|at| input.now_ms.saturating_sub(*at) >= RAID_WINDOW_MS)
            {
                self.media_window.pop_front();
            }
            if self.media_window.len() > usize::from(settings.raid_limit_per_minute) {
                self.raid_until_ms = input.now_ms + RAID_WINDOW_MS;
            }
            if input.now_ms < self.raid_until_ms {
                return Some("raid");
            }
        }
        None
    }

    pub fn raid_active(&self, now_ms: u64) -> bool {
        now_ms < self.raid_until_ms
    }

    pub fn remember_verdict(&mut self, message_id: &str, mut verdict: MessageVerdict, now_ms: u64) {
        verdict.at_ms = now_ms;
        if self.verdicts.len() >= RUNTIME_ENTRY_LIMIT {
            self.prune(now_ms);
        }
        self.verdicts.insert(message_id.to_owned(), verdict);
    }

    pub fn verdict(&self, message_id: &str) -> Option<MessageVerdict> {
        self.verdicts.get(message_id).cloned()
    }

    /// Counts a block for escalation and decides on a warning.
    pub fn record_block(
        &mut self,
        user_id: u64,
        settings: &ModerationSettings,
        now_ms: u64,
    ) -> BlockFollowUp {
        let window_ms = u64::from(settings.escalation_window_minutes) * 60_000;
        let blocks = self.blocks.entry(user_id).or_default();
        blocks.push_back(now_ms);
        while blocks
            .front()
            .is_some_and(|at| now_ms.saturating_sub(*at) >= window_ms)
        {
            blocks.pop_front();
        }
        let timeout_minutes = (settings.escalation_blocks > 0
            && blocks.len() >= usize::from(settings.escalation_blocks))
        .then_some(settings.escalation_timeout_minutes);
        if timeout_minutes.is_some() {
            blocks.clear();
        }
        let warn = settings.warn_on_block
            && self
                .warned
                .get(&user_id)
                .is_none_or(|at| now_ms.saturating_sub(*at) >= WARNING_INTERVAL_MS);
        if warn {
            self.warned.insert(user_id, now_ms);
        }
        BlockFollowUp {
            warn,
            timeout_minutes,
        }
    }

    fn prune(&mut self, now_ms: u64) {
        let keep_ms = 3_600_000;
        self.last_accepted
            .retain(|_, at| now_ms.saturating_sub(*at) < keep_ms);
        while self
            .recent_media
            .front()
            .is_some_and(|(_, at)| now_ms.saturating_sub(*at) >= DUPLICATE_WINDOW_MS)
            || self.recent_media.len() > RUNTIME_ENTRY_LIMIT
        {
            self.recent_media.pop_front();
        }
        self.blocks.retain(|_, blocks| {
            blocks
                .back()
                .is_some_and(|at| now_ms.saturating_sub(*at) < keep_ms * 24)
        });
        self.warned
            .retain(|_, at| now_ms.saturating_sub(*at) < WARNING_INTERVAL_MS);
        self.verdicts
            .retain(|_, verdict| now_ms.saturating_sub(verdict.at_ms) < VERDICT_TTL_MS);
        if self.last_accepted.len() > RUNTIME_ENTRY_LIMIT {
            self.last_accepted.clear();
        }
    }
}
