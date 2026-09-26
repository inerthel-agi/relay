//! Moderation presets: one click sets the existing switches to a coherent
//! level. A preset never touches channels, custom words, private data, the
//! allowlist or exempt roles.

use serde::{Deserialize, Serialize};

use super::WordPack;
use crate::config::AppConfig;
use crate::privacy::{PrivacyCategory, PrivacyClassification, ProtectionLevel};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModerationPreset {
    Relaxed,
    Standard,
    Strict,
    Event,
}

impl ModerationPreset {
    pub const ALL: [Self; 4] = [Self::Relaxed, Self::Standard, Self::Strict, Self::Event];
}

/// Every setting a preset controls, so it can also be captured and restored.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetFields {
    pub moderation_enabled: bool,
    pub allow_images: bool,
    pub allow_videos: bool,
    pub allow_audio: bool,
    pub review_only_selected: bool,
    pub privacy_scan_enabled: bool,
    pub protection_level: ProtectionLevel,
    pub block_threshold: PrivacyClassification,
    pub review_intermediate: bool,
    pub auto_delete_blocked_messages: bool,
    pub enabled_categories: Vec<PrivacyCategory>,
    pub word_packs: Vec<WordPack>,
    pub user_cooldown_seconds: u16,
    pub raid_limit_per_minute: u16,
    pub block_duplicates: bool,
    pub block_text_spam: bool,
    pub min_account_age_days: u16,
    pub block_invites: bool,
    pub block_shorteners: bool,
    pub block_scam_domains: bool,
}

pub fn fields(preset: ModerationPreset) -> PresetFields {
    let all_categories = PrivacyCategory::USER_CONFIGURABLE.to_vec();
    match preset {
        ModerationPreset::Relaxed => PresetFields {
            moderation_enabled: false,
            allow_images: true,
            allow_videos: true,
            allow_audio: true,
            review_only_selected: false,
            privacy_scan_enabled: true,
            protection_level: ProtectionLevel::Balanced,
            block_threshold: PrivacyClassification::High,
            review_intermediate: false,
            auto_delete_blocked_messages: true,
            enabled_categories: all_categories
                .into_iter()
                .filter(|category| *category != PrivacyCategory::LicensePlate)
                .collect(),
            word_packs: vec![WordPack::Hate],
            user_cooldown_seconds: 0,
            raid_limit_per_minute: 0,
            block_duplicates: false,
            block_text_spam: false,
            min_account_age_days: 0,
            block_invites: false,
            block_shorteners: false,
            block_scam_domains: true,
        },
        ModerationPreset::Standard => PresetFields {
            moderation_enabled: true,
            allow_images: false,
            allow_videos: true,
            allow_audio: true,
            review_only_selected: true,
            privacy_scan_enabled: true,
            protection_level: ProtectionLevel::Balanced,
            block_threshold: PrivacyClassification::High,
            review_intermediate: true,
            auto_delete_blocked_messages: true,
            enabled_categories: all_categories,
            word_packs: vec![WordPack::Hate, WordPack::Scams],
            user_cooldown_seconds: 10,
            raid_limit_per_minute: 0,
            block_duplicates: true,
            block_text_spam: true,
            min_account_age_days: 0,
            block_invites: true,
            block_shorteners: false,
            block_scam_domains: true,
        },
        ModerationPreset::Strict => PresetFields {
            moderation_enabled: true,
            allow_images: true,
            allow_videos: true,
            allow_audio: true,
            review_only_selected: false,
            privacy_scan_enabled: true,
            protection_level: ProtectionLevel::Strict,
            block_threshold: PrivacyClassification::High,
            review_intermediate: true,
            auto_delete_blocked_messages: true,
            enabled_categories: all_categories,
            word_packs: vec![WordPack::Hate, WordPack::Scams, WordPack::Sexual],
            user_cooldown_seconds: 30,
            raid_limit_per_minute: 20,
            block_duplicates: true,
            block_text_spam: true,
            min_account_age_days: 0,
            block_invites: true,
            block_shorteners: true,
            block_scam_domains: true,
        },
        ModerationPreset::Event => PresetFields {
            moderation_enabled: true,
            allow_images: true,
            allow_videos: true,
            allow_audio: true,
            review_only_selected: false,
            privacy_scan_enabled: true,
            protection_level: ProtectionLevel::Paranoid,
            block_threshold: PrivacyClassification::High,
            review_intermediate: true,
            auto_delete_blocked_messages: true,
            enabled_categories: all_categories,
            word_packs: WordPack::ALL.to_vec(),
            user_cooldown_seconds: 60,
            raid_limit_per_minute: 10,
            block_duplicates: true,
            block_text_spam: true,
            min_account_age_days: 7,
            block_invites: true,
            block_shorteners: true,
            block_scam_domains: true,
        },
    }
}

pub fn capture(config: &AppConfig) -> PresetFields {
    let moderation = &config.moderation;
    PresetFields {
        moderation_enabled: config.moderation_enabled,
        allow_images: config.moderation_allow_images,
        allow_videos: config.moderation_allow_videos,
        allow_audio: config.moderation_allow_audio,
        review_only_selected: moderation.review_only_selected,
        privacy_scan_enabled: config.privacy_scan_enabled,
        protection_level: config.privacy_protection_level,
        block_threshold: config.privacy_block_threshold,
        review_intermediate: config.privacy_review_intermediate,
        auto_delete_blocked_messages: config.privacy_auto_delete_blocked_messages,
        enabled_categories: config.privacy_enabled_categories.clone(),
        word_packs: moderation.word_packs.clone(),
        user_cooldown_seconds: moderation.user_cooldown_seconds,
        raid_limit_per_minute: moderation.raid_limit_per_minute,
        block_duplicates: moderation.block_duplicates,
        block_text_spam: moderation.block_text_spam,
        min_account_age_days: moderation.min_account_age_days,
        block_invites: moderation.block_invites,
        block_shorteners: moderation.block_shorteners,
        block_scam_domains: moderation.block_scam_domains,
    }
}

pub fn apply(config: &mut AppConfig, fields: &PresetFields) {
    config.moderation_enabled = fields.moderation_enabled;
    config.moderation_allow_images = fields.allow_images;
    config.moderation_allow_videos = fields.allow_videos;
    config.moderation_allow_audio = fields.allow_audio;
    config.privacy_scan_enabled = fields.privacy_scan_enabled;
    config.privacy_protection_level = fields.protection_level;
    config.privacy_block_threshold = fields.block_threshold;
    config.privacy_review_intermediate = fields.review_intermediate;
    config.privacy_auto_delete_blocked_messages = fields.auto_delete_blocked_messages;
    config.privacy_enabled_categories = fields.enabled_categories.clone();
    let moderation = &mut config.moderation;
    moderation.review_only_selected = fields.review_only_selected;
    moderation.word_packs = fields.word_packs.clone();
    moderation.user_cooldown_seconds = fields.user_cooldown_seconds;
    moderation.raid_limit_per_minute = fields.raid_limit_per_minute;
    moderation.block_duplicates = fields.block_duplicates;
    moderation.block_text_spam = fields.block_text_spam;
    moderation.min_account_age_days = fields.min_account_age_days;
    moderation.block_invites = fields.block_invites;
    moderation.block_shorteners = fields.block_shorteners;
    moderation.block_scam_domains = fields.block_scam_domains;
}

/// Compares sets as sets: the order of lists does not make a preset different.
fn same(left: &PresetFields, right: &PresetFields) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    for fields in [&mut left, &mut right] {
        fields
            .enabled_categories
            .sort_by_key(|category| format!("{category:?}"));
        fields.word_packs.sort_by_key(|pack| format!("{pack:?}"));
    }
    left == right
}

/// The preset the current settings match exactly, if any ("Custom" otherwise).
pub fn matching(config: &AppConfig) -> Option<ModerationPreset> {
    let current = capture(config);
    ModerationPreset::ALL
        .into_iter()
        .find(|preset| same(&current, &fields(*preset)))
}

/// Stable setting keys a preset would change, translated by the panel.
pub fn changed_keys(config: &AppConfig, preset: ModerationPreset) -> Vec<&'static str> {
    let current = capture(config);
    let target = fields(preset);
    let mut keys = Vec::new();
    let mut check = |changed: bool, key: &'static str| {
        if changed {
            keys.push(key);
        }
    };
    check(
        current.moderation_enabled != target.moderation_enabled,
        "moderationEnabled",
    );
    check(
        current.allow_images != target.allow_images
            || current.allow_videos != target.allow_videos
            || current.allow_audio != target.allow_audio
            || current.review_only_selected != target.review_only_selected,
        "reviewedTypes",
    );
    check(
        current.privacy_scan_enabled != target.privacy_scan_enabled,
        "privacyScan",
    );
    check(
        current.protection_level != target.protection_level,
        "protectionLevel",
    );
    check(
        current.block_threshold != target.block_threshold,
        "blockThreshold",
    );
    check(
        current.review_intermediate != target.review_intermediate,
        "reviewIntermediate",
    );
    check(
        current.auto_delete_blocked_messages != target.auto_delete_blocked_messages,
        "autoDelete",
    );
    let mut current_categories = current.enabled_categories.clone();
    let mut target_categories = target.enabled_categories.clone();
    current_categories.sort_by_key(|category| format!("{category:?}"));
    target_categories.sort_by_key(|category| format!("{category:?}"));
    check(current_categories != target_categories, "categories");
    let mut current_packs = current.word_packs.clone();
    let mut target_packs = target.word_packs.clone();
    current_packs.sort_by_key(|pack| format!("{pack:?}"));
    target_packs.sort_by_key(|pack| format!("{pack:?}"));
    check(current_packs != target_packs, "wordPacks");
    check(
        current.user_cooldown_seconds != target.user_cooldown_seconds,
        "cooldown",
    );
    check(
        current.raid_limit_per_minute != target.raid_limit_per_minute,
        "raidLimit",
    );
    check(
        current.block_duplicates != target.block_duplicates,
        "duplicates",
    );
    check(
        current.block_text_spam != target.block_text_spam,
        "textSpam",
    );
    check(
        current.min_account_age_days != target.min_account_age_days,
        "accountAge",
    );
    check(
        current.block_invites != target.block_invites
            || current.block_shorteners != target.block_shorteners
            || current.block_scam_domains != target.block_scam_domains,
        "links",
    );
    keys
}
