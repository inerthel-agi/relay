use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tauri_plugin_global_shortcut::Shortcut;

use crate::custom_commands::{CustomCommandDefinition, validate_custom_commands};
use crate::model::InterfacePreferences;
use crate::privacy::{
    ForbiddenConcept, MAX_CONFIGURED_REGEXES, MAX_PRIVACY_LIST_ENTRIES,
    MAX_PRIVACY_LIST_VALUE_CHARS, PrivacyCategory, PrivacyClassification, ProtectionLevel,
    default_privacy_categories,
};

pub const DEFAULT_PORT: u16 = 4590;
pub const DEFAULT_DISPLAY_DURATION_MS: u64 = 8_000;
pub const DEFAULT_GIF_DURATION_MS: u64 = 8_000;
pub const DEFAULT_STICKER_DURATION_MS: u64 = 8_000;
pub const DEFAULT_NOTIFICATION_DURATION_MS: u64 = 8_000;
pub const DEFAULT_WIDGET_WIDTH: f64 = 640.0;
pub const DEFAULT_WIDGET_HEIGHT: f64 = 360.0;
pub const DEFAULT_NOTIFICATION_WIDGET_WIDTH: f64 = 400.0;
pub const DEFAULT_NOTIFICATION_WIDGET_HEIGHT: f64 = 104.0;
const LEGACY_NOTIFICATION_WIDGET_WIDTH: f64 = 980.0;
const LEGACY_NOTIFICATION_WIDGET_HEIGHT: f64 = 180.0;
const PREVIOUS_NOTIFICATION_WIDGET_WIDTH: f64 = 480.0;
const PREVIOUS_NOTIFICATION_WIDGET_HEIGHT: f64 = 112.0;
pub const DEFAULT_SKIP_SHORTCUT: &str = "control+alt+KeyS";
pub const DEFAULT_PANIC_SHORTCUT: &str = "control+alt+KeyP";
pub const MAX_PRIVACY_EXEMPT_ROLE_IDS: usize = 100;
pub const MIN_WIDGET_WIDTH: f64 = 160.0;
pub const MIN_WIDGET_HEIGHT: f64 = 90.0;
const MAX_WIDGET_DIMENSION: f64 = 16_384.0;
const LEGACY_CONFIG_DIRECTORIES: [&str; 2] =
    ["eu.stealthylabs.discord-obs-relay", "discord-obs-relay"];

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOverwriteSnapshot {
    pub target_id: String,
    pub target_kind: String,
    pub allow: u64,
    pub deny: u64,
    pub existed: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelLockSnapshot {
    pub channel_id: String,
    pub overwrites: Vec<PermissionOverwriteSnapshot>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputAnchor {
    #[default]
    Legacy,
    TopLeft,
    TopCenter,
    TopRight,
    Center,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OutputGeometry {
    pub crop_top: u8,
    pub crop_right: u8,
    pub crop_bottom: u8,
    pub crop_left: u8,
    pub content_scale: u16,
    pub anchor: OutputAnchor,
    pub margin_x: u16,
    pub margin_y: u16,
}

impl Default for OutputGeometry {
    fn default() -> Self {
        Self {
            crop_top: 0,
            crop_right: 0,
            crop_bottom: 0,
            crop_left: 0,
            content_scale: 100,
            anchor: OutputAnchor::Legacy,
            margin_x: 12,
            margin_y: 16,
        }
    }
}

/// The OBS YouTube video can shrink to a small corner player.
pub const YOUTUBE_VIDEO_MIN_SCALE: u16 = 10;

impl OutputGeometry {
    pub fn validate(&self) -> Result<()> {
        self.validate_with_min_scale(50)
    }

    pub fn validate_with_min_scale(&self, min_scale: u16) -> Result<()> {
        if [
            self.crop_top,
            self.crop_right,
            self.crop_bottom,
            self.crop_left,
        ]
        .into_iter()
        .any(|crop| crop > 40)
        {
            bail!("Output crop values must be between 0 and 40 percent.");
        }
        if self.margin_x > 200 || self.margin_y > 200 {
            bail!("Output margins must be between 0 and 200 pixels.");
        }
        if !(min_scale..=400).contains(&self.content_scale) {
            bail!("Output content scale must be between {min_scale} and 400 percent.");
        }
        Ok(())
    }
}

/// What Relay takes from the messages of one Discord channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelFeeds {
    pub notifications: bool,
    pub media: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HoneypotAction {
    #[default]
    Kick,
    Ban,
    /// Temporary timeout; the duration lives in `moderation.honeypot_timeout_minutes`.
    Timeout,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub music_max_pending_per_user: u8,
    pub music_reject_duplicate_pending: bool,
    pub reactions: crate::reactions::ReactionSettings,
    /// The Relay channel: its messages become notifications and its media go to OBS.
    pub watched_channel_id: String,
    /// Former separate message channel. Only kept, with its cleanup settings,
    /// while it differs from `watched_channel_id` and the user has not chosen one.
    pub former_message_channel_id: String,
    pub media_cleanup_enabled: bool,
    pub media_welcome_message_id: String,
    pub former_message_cleanup_enabled: bool,
    pub former_message_welcome_message_id: String,
    pub music_channel_id: String,
    pub music_cleanup_enabled: bool,
    pub music_welcome_message_id: String,
    pub honeypot_channel_id: String,
    pub honeypot_action: HoneypotAction,
    pub port: u16,
    pub display_duration_ms: u64,
    pub gif_duration_ms: u64,
    pub sticker_duration_ms: u64,
    pub notification_duration_ms: u64,
    pub media_volume: u8,
    pub skip_shortcut: String,
    pub panic_shortcut: String,
    pub notification_character_limit: u32,
    pub notification_queue_limit: u8,
    pub notifications_obs_enabled: bool,
    pub bot_online_status: String,
    pub bot_activity_type: String,
    pub bot_activity_text: String,
    pub show_author: bool,
    pub show_media_text_obs: bool,
    pub show_media_text_widget: bool,
    pub moderation_enabled: bool,
    pub moderation_allow_images: bool,
    pub moderation_allow_videos: bool,
    pub moderation_allow_audio: bool,
    pub privacy_scan_enabled: bool,
    pub privacy_similarity_boost: u8,
    pub privacy_concepts: Vec<ForbiddenConcept>,
    pub privacy_filter_exempt_role_ids: Vec<String>,
    pub privacy_protection_level: ProtectionLevel,
    pub privacy_enabled_categories: Vec<PrivacyCategory>,
    pub privacy_block_threshold: PrivacyClassification,
    pub privacy_review_intermediate: bool,
    pub privacy_auto_delete_blocked_messages: bool,
    pub privacy_allowlist: Vec<String>,
    pub privacy_custom_patterns: Vec<String>,
    pub moderation: crate::moderation::ModerationSettings,
    pub command_channel_enabled: bool,
    pub command_url_enabled: bool,
    pub command_show_enabled: bool,
    pub command_status_enabled: bool,
    pub command_test_enabled: bool,
    pub command_regenerate_enabled: bool,
    pub command_clear_enabled: bool,
    pub command_nuke_enabled: bool,
    pub command_lock_enabled: bool,
    pub command_changelog_enabled: bool,
    pub custom_commands: Vec<CustomCommandDefinition>,
    pub channel_lock: Option<ChannelLockSnapshot>,
    pub interface_preferences: InterfacePreferences,
    pub widget_x: Option<i32>,
    pub widget_y: Option<i32>,
    pub widget_width: f64,
    pub widget_height: f64,
    pub widget_keep_aspect_ratio: bool,
    pub widget_visible: bool,
    pub widget_locked: bool,
    pub widget_sound_enabled: bool,
    pub notification_widget_x: Option<i32>,
    pub notification_widget_y: Option<i32>,
    pub notification_widget_width: f64,
    pub notification_widget_height: f64,
    pub notification_widget_visible: bool,
    pub notification_widget_locked: bool,
    pub media_obs_geometry: OutputGeometry,
    pub media_widget_geometry: OutputGeometry,
    pub notification_obs_geometry: OutputGeometry,
    pub notification_widget_geometry: OutputGeometry,
    /// Size and place of the YouTube "Now playing" card in OBS (`/youtube`).
    pub music_obs_geometry: OutputGeometry,
    /// Size, place and crop of the YouTube video itself in OBS (`/youtube`).
    pub music_video_obs_geometry: OutputGeometry,
    pub notification_sound_enabled: bool,
    pub notification_sound_obs_enabled: bool,
    pub notification_sound_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            music_max_pending_per_user: 3,
            music_reject_duplicate_pending: true,
            reactions: crate::reactions::ReactionSettings::default(),
            watched_channel_id: String::new(),
            former_message_channel_id: String::new(),
            media_cleanup_enabled: false,
            media_welcome_message_id: String::new(),
            former_message_cleanup_enabled: false,
            former_message_welcome_message_id: String::new(),
            music_channel_id: String::new(),
            music_cleanup_enabled: false,
            music_welcome_message_id: String::new(),
            honeypot_channel_id: String::new(),
            honeypot_action: HoneypotAction::Kick,
            port: DEFAULT_PORT,
            display_duration_ms: DEFAULT_DISPLAY_DURATION_MS,
            gif_duration_ms: DEFAULT_GIF_DURATION_MS,
            sticker_duration_ms: DEFAULT_STICKER_DURATION_MS,
            notification_duration_ms: DEFAULT_NOTIFICATION_DURATION_MS,
            media_volume: 50,
            skip_shortcut: DEFAULT_SKIP_SHORTCUT.into(),
            panic_shortcut: DEFAULT_PANIC_SHORTCUT.into(),
            notification_character_limit: 0,
            notification_queue_limit: 50,
            notifications_obs_enabled: true,
            bot_online_status: "online".into(),
            bot_activity_type: "custom".into(),
            bot_activity_text: String::new(),
            show_author: true,
            show_media_text_obs: false,
            show_media_text_widget: false,
            moderation_enabled: false,
            moderation_allow_images: true,
            moderation_allow_videos: true,
            moderation_allow_audio: true,
            privacy_scan_enabled: false,
            privacy_similarity_boost: 4,
            privacy_concepts: Vec::new(),
            privacy_filter_exempt_role_ids: Vec::new(),
            privacy_protection_level: ProtectionLevel::Balanced,
            privacy_enabled_categories: default_privacy_categories(),
            privacy_block_threshold: PrivacyClassification::High,
            privacy_review_intermediate: true,
            privacy_auto_delete_blocked_messages: true,
            privacy_allowlist: Vec::new(),
            privacy_custom_patterns: Vec::new(),
            moderation: crate::moderation::ModerationSettings::default(),
            command_channel_enabled: true,
            command_url_enabled: true,
            command_show_enabled: true,
            command_status_enabled: true,
            command_test_enabled: true,
            command_regenerate_enabled: true,
            command_clear_enabled: true,
            command_nuke_enabled: true,
            command_lock_enabled: true,
            command_changelog_enabled: true,
            custom_commands: Vec::new(),
            channel_lock: None,
            interface_preferences: InterfacePreferences::default(),
            widget_x: None,
            widget_y: None,
            widget_width: DEFAULT_WIDGET_WIDTH,
            widget_height: DEFAULT_WIDGET_HEIGHT,
            widget_keep_aspect_ratio: true,
            widget_visible: false,
            widget_locked: false,
            widget_sound_enabled: false,
            notification_widget_x: None,
            notification_widget_y: None,
            notification_widget_width: DEFAULT_NOTIFICATION_WIDGET_WIDTH,
            notification_widget_height: DEFAULT_NOTIFICATION_WIDGET_HEIGHT,
            notification_widget_visible: false,
            notification_widget_locked: false,
            media_obs_geometry: OutputGeometry::default(),
            media_widget_geometry: OutputGeometry::default(),
            notification_obs_geometry: OutputGeometry::default(),
            notification_widget_geometry: OutputGeometry::default(),
            music_obs_geometry: OutputGeometry::default(),
            music_video_obs_geometry: OutputGeometry::default(),
            notification_sound_enabled: false,
            notification_sound_obs_enabled: false,
            notification_sound_path: None,
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<()> {
        if self.music_max_pending_per_user > 10 {
            bail!("Music requests per member must be between 0 and 10.");
        }
        self.reactions.validate()?;
        if self.reactions.enabled
            && self
                .custom_commands
                .iter()
                .filter(|command| command.enabled)
                .count()
                > 14
        {
            bail!(
                "Reactions support at most 14 enabled custom commands alongside the default commands."
            );
        }
        validate_channel_id(&self.watched_channel_id, "watched")?;
        validate_channel_id(&self.former_message_channel_id, "message")?;
        for (enabled, channel, welcome) in [
            (
                self.media_cleanup_enabled,
                &self.watched_channel_id,
                &self.media_welcome_message_id,
            ),
            (
                self.former_message_cleanup_enabled,
                &self.former_message_channel_id,
                &self.former_message_welcome_message_id,
            ),
        ] {
            if enabled && channel.parse::<u64>().ok().filter(|id| *id > 0).is_none() {
                bail!("24-hour cleanup requires a valid channel.");
            }
            if !welcome.is_empty() && welcome.parse::<u64>().ok().filter(|id| *id > 0).is_none() {
                bail!("The protected welcome message ID is invalid.");
            }
        }
        validate_channel_id(&self.music_channel_id, "music")?;
        validate_channel_id(&self.music_welcome_message_id, "welcome message")?;
        if self.music_cleanup_enabled && self.music_channel_id.is_empty() {
            bail!("Music cleanup requires a music channel.");
        }
        validate_channel_id(&self.honeypot_channel_id, "honeypot")?;
        let configured_channels = [
            ("media", &self.watched_channel_id),
            ("messages", &self.former_message_channel_id),
            ("music", &self.music_channel_id),
            ("honeypot", &self.honeypot_channel_id),
        ];
        for (index, (_, left_id)) in configured_channels.iter().enumerate() {
            if left_id.is_empty() {
                continue;
            }
            if configured_channels[index + 1..]
                .iter()
                .any(|(_, right_id)| left_id == right_id && !right_id.is_empty())
            {
                bail!("The Relay channel, music, and honeypot must use separate Discord channels.");
            }
        }
        if self.port < 1024 {
            bail!("The local port must be between 1024 and 65535.");
        }
        if !(1_000..=60_000).contains(&self.display_duration_ms) {
            bail!("The image duration must be between 1 and 60 seconds.");
        }
        if !(1_000..=60_000).contains(&self.gif_duration_ms) {
            bail!("The GIF duration must be between 1 and 60 seconds.");
        }
        if !(1_000..=60_000).contains(&self.sticker_duration_ms) {
            bail!("The sticker duration must be between 1 and 60 seconds.");
        }
        if !(1_000..=60_000).contains(&self.notification_duration_ms) {
            bail!("The notification duration must be between 1 and 60 seconds.");
        }
        if self.media_volume > 100 {
            bail!("The media volume must be between 0 and 100 percent.");
        }
        if self.skip_shortcut.trim().parse::<Shortcut>().is_err() {
            bail!("The media skip shortcut is invalid.");
        }
        if self.panic_shortcut.trim().parse::<Shortcut>().is_err() {
            bail!("The panic shortcut is invalid.");
        }
        if !(1..=50).contains(&self.notification_queue_limit) {
            bail!("The message queue limit must be between 1 and 50.");
        }
        if !(1..=100).contains(&self.privacy_similarity_boost) {
            bail!("Privacy similarity boost must be between 1 and 100.");
        }
        if self.privacy_concepts.len() > 100 {
            bail!("At most 100 forbidden concepts may be configured.");
        }
        if self
            .privacy_concepts
            .iter()
            .map(|concept| concept.regexes.len())
            .sum::<usize>()
            > MAX_CONFIGURED_REGEXES
        {
            bail!("At most {MAX_CONFIGURED_REGEXES} filter regular expressions may be configured.");
        }
        for concept in &self.privacy_concepts {
            concept.validate()?;
        }
        if self.privacy_filter_exempt_role_ids.len() > MAX_PRIVACY_EXEMPT_ROLE_IDS {
            bail!(
                "At most {MAX_PRIVACY_EXEMPT_ROLE_IDS} privacy filter exempt roles may be configured."
            );
        }
        for role_id in &self.privacy_filter_exempt_role_ids {
            validate_snowflake_id(role_id, "privacy filter exempt role")?;
        }
        if !matches!(
            self.privacy_block_threshold,
            PrivacyClassification::High | PrivacyClassification::Critical
        ) {
            bail!("The privacy block threshold must be HIGH or CRITICAL.");
        }
        if self.privacy_enabled_categories.len() > PrivacyCategory::USER_CONFIGURABLE.len() {
            bail!("Too many privacy detection categories were configured.");
        }
        let mut unique_categories = std::collections::HashSet::new();
        for category in &self.privacy_enabled_categories {
            if !PrivacyCategory::USER_CONFIGURABLE.contains(category)
                || !unique_categories.insert(*category)
            {
                bail!("Privacy detection categories are invalid or duplicated.");
            }
        }
        validate_privacy_list(&self.privacy_allowlist, "privacy allowlist")?;
        validate_privacy_list(&self.privacy_custom_patterns, "private data list")?;
        self.moderation.validate()?;
        if !matches!(
            self.bot_online_status.as_str(),
            "online" | "idle" | "dnd" | "invisible"
        ) {
            bail!("The Discord bot status is invalid.");
        }
        if !matches!(
            self.bot_activity_type.as_str(),
            "none" | "custom" | "playing" | "listening" | "watching" | "competing"
        ) {
            bail!("The Discord bot activity type is invalid.");
        }
        if self.bot_activity_text.chars().count() > 128
            || self.bot_activity_text.chars().any(char::is_control)
        {
            bail!("The Discord bot activity must contain at most 128 printable characters.");
        }
        validate_interface_preferences(&self.interface_preferences)?;
        validate_custom_commands(&self.custom_commands)?;
        validate_widget_size(self.widget_width, self.widget_height)?;
        validate_widget_size(
            self.notification_widget_width,
            self.notification_widget_height,
        )?;
        self.media_obs_geometry.validate()?;
        self.media_widget_geometry.validate()?;
        self.notification_obs_geometry.validate()?;
        self.notification_widget_geometry.validate()?;
        self.music_obs_geometry.validate()?;
        self.music_video_obs_geometry
            .validate_with_min_scale(YOUTUBE_VIDEO_MIN_SCALE)?;
        Ok(())
    }

    /// True while two different former channels wait for the user's choice.
    /// Until then each one keeps its former single role.
    pub fn relay_channel_conflict(&self) -> bool {
        !self.watched_channel_id.is_empty()
            && !self.former_message_channel_id.is_empty()
            && self.watched_channel_id != self.former_message_channel_id
    }

    pub fn channel_feeds(&self, channel_id: &str) -> Option<ChannelFeeds> {
        if channel_id.is_empty() {
            return None;
        }
        let media = self.watched_channel_id == channel_id;
        let notifications = if self.relay_channel_conflict() {
            self.former_message_channel_id == channel_id
        } else {
            media
        };
        (media || notifications).then_some(ChannelFeeds {
            notifications,
            media,
        })
    }

    /// Folds the former message channel into the Relay channel when that loses
    /// nothing: only one was set, or both were the same. Two different channels
    /// stay untouched until `choose_relay_channel`. Returns whether it changed.
    pub fn unify_relay_channels(&mut self) -> bool {
        if self.former_message_channel_id.is_empty() {
            return false;
        }
        if self.watched_channel_id.is_empty() {
            self.watched_channel_id = std::mem::take(&mut self.former_message_channel_id);
            self.media_cleanup_enabled = self.former_message_cleanup_enabled;
            self.media_welcome_message_id =
                std::mem::take(&mut self.former_message_welcome_message_id);
        } else if self.watched_channel_id == self.former_message_channel_id {
            self.media_cleanup_enabled |= self.former_message_cleanup_enabled;
            if self.media_welcome_message_id.is_empty() {
                self.media_welcome_message_id =
                    std::mem::take(&mut self.former_message_welcome_message_id);
            }
        } else {
            return false;
        }
        self.clear_former_message_channel();
        true
    }

    /// Keeps one of the two former channels as the Relay channel, with its cleanup settings.
    pub fn choose_relay_channel(&mut self, channel_id: &str) -> Result<()> {
        if !self.relay_channel_conflict() {
            bail!("There is no Relay channel choice to make.");
        }
        if channel_id == self.former_message_channel_id {
            self.watched_channel_id = std::mem::take(&mut self.former_message_channel_id);
            self.media_cleanup_enabled = self.former_message_cleanup_enabled;
            self.media_welcome_message_id =
                std::mem::take(&mut self.former_message_welcome_message_id);
        } else if channel_id != self.watched_channel_id {
            bail!("Choose one of the two former Relay channels.");
        }
        self.clear_former_message_channel();
        Ok(())
    }

    fn clear_former_message_channel(&mut self) {
        self.former_message_channel_id.clear();
        self.former_message_cleanup_enabled = false;
        self.former_message_welcome_message_id.clear();
    }
}

pub(crate) fn validate_interface_preferences(preferences: &InterfacePreferences) -> Result<()> {
    if !matches!(
        preferences.language.as_str(),
        "en" | "fr" | "es" | "de" | "ru" | "zh" | "ko" | "ja" | "id"
    ) {
        bail!("The interface language is invalid.");
    }
    if !matches!(preferences.theme.as_str(), "light" | "dark") {
        bail!("The interface theme is invalid.");
    }
    if !(80..=140).contains(&preferences.font_scale) {
        bail!("The interface font scale must be between 80 and 140 percent.");
    }
    if !crate::model::INTERFACE_DESIGNS.contains(&preferences.design.as_str()) {
        bail!("The interface design is invalid.");
    }
    let output_style = preferences.output_style.as_str();
    if output_style != "auto"
        && output_style != crate::model::SUBTITLE_OUTPUT_STYLE
        && !crate::model::INTERFACE_DESIGNS.contains(&output_style)
    {
        bail!("The notification style is invalid.");
    }
    if !matches!(
        preferences.output_background.as_str(),
        "auto" | "light" | "dark"
    ) {
        bail!("The notification background is invalid.");
    }
    Ok(())
}

fn validate_privacy_list(values: &[String], label: &str) -> Result<()> {
    if values.len() > MAX_PRIVACY_LIST_ENTRIES {
        bail!("The {label} may contain at most {MAX_PRIVACY_LIST_ENTRIES} entries.");
    }
    for value in values {
        let value = value.trim();
        if !(3..=MAX_PRIVACY_LIST_VALUE_CHARS).contains(&value.chars().count())
            || value.chars().any(char::is_control)
        {
            bail!(
                "Each {label} entry must contain 3 to {MAX_PRIVACY_LIST_VALUE_CHARS} printable characters."
            );
        }
    }
    Ok(())
}

fn validate_widget_size(width: f64, height: f64) -> Result<()> {
    if !width.is_finite()
        || !height.is_finite()
        || !(MIN_WIDGET_WIDTH..=MAX_WIDGET_DIMENSION).contains(&width)
        || !(MIN_WIDGET_HEIGHT..=MAX_WIDGET_DIMENSION).contains(&height)
    {
        bail!("Widget size is outside the supported bounds.");
    }
    Ok(())
}

fn validate_channel_id(channel_id: &str, label: &str) -> Result<()> {
    if !channel_id.is_empty()
        && (channel_id.len() > 20
            || !channel_id
                .chars()
                .all(|character| character.is_ascii_digit()))
    {
        bail!("The {label} channel ID is invalid.");
    }
    Ok(())
}

pub(crate) fn validate_snowflake_id(value: &str, label: &str) -> Result<()> {
    if !(17..=20).contains(&value.len())
        || !value.chars().all(|character| character.is_ascii_digit())
        || value.parse::<u64>().map_or(true, |id| id == 0)
    {
        bail!("The {label} ID is invalid.");
    }
    Ok(())
}

/// Must match `identifier` in `tauri.conf.json` (checked by a test).
pub const APP_IDENTIFIER: &str = "eu.inerthel.relay";
/// Identifier used up to 1.4.0 for the configuration, data and WebView folders.
pub const PREVIOUS_APP_IDENTIFIER: &str = "eu.stealthylabs.relay";
/// Where WebView2 keeps the panel's local preferences (localStorage).
pub const WEBVIEW_STORAGE_PATH: [&str; 3] = ["EBWebView", "Default", "Local Storage"];

/// Copies a folder of the previous app identifier to its new place, once:
/// nothing happens when the destination exists or the source is missing.
/// The copy goes through a staging folder, so an interrupted copy is retried
/// on the next launch. The source stays in place for older builds.
pub fn copy_previous_app_folder(from: &Path, to: &Path) -> Result<bool> {
    if to.exists() || !from.is_dir() {
        return Ok(false);
    }
    let mut staging = to.as_os_str().to_owned();
    staging.push(".migrating");
    let staging = PathBuf::from(staging);
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .with_context(|| format!("failed to clear {}", staging.display()))?;
    }
    copy_folder(from, &staging)?;
    fs::rename(&staging, to).with_context(|| format!("failed to create {}", to.display()))?;
    Ok(true)
}

fn copy_folder(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).with_context(|| format!("failed to create {}", to.display()))?;
    for entry in fs::read_dir(from).with_context(|| format!("failed to read {}", from.display()))? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_folder(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target)
                .with_context(|| format!("failed to copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}

pub fn migrate_legacy_config(config_directory: &Path) -> Result<()> {
    let destination = config_directory.join("config.json");
    let Some(parent) = config_directory.parent() else {
        return Ok(());
    };
    for directory in LEGACY_CONFIG_DIRECTORIES {
        let legacy_path = parent.join(directory).join("config.json");
        if legacy_path.is_file() {
            let bytes = fs::read(&legacy_path)
                .with_context(|| format!("failed to read {}", legacy_path.display()))?;
            // Merge the former channels as saved, then unify: a channel that
            // only differs from the current one must stay visible for the choice.
            let (mut config, _) = deserialize_former_config(&bytes)
                .with_context(|| format!("failed to parse {}", legacy_path.display()))?;
            if destination.exists() {
                // This runs on every launch: only a configuration without a
                // Relay channel takes the legacy ones. An empty former message
                // channel is the normal state since 1.4.1 and is never refilled.
                let store = ConfigStore::new(destination.clone());
                let mut current = store.load()?;
                if current.watched_channel_id.is_empty()
                    && (!config.watched_channel_id.is_empty()
                        || !config.former_message_channel_id.is_empty())
                {
                    current.watched_channel_id = config.watched_channel_id;
                    if current.former_message_channel_id.is_empty() {
                        current.former_message_channel_id = config.former_message_channel_id;
                    }
                    current.unify_relay_channels();
                    store.save(&current)?;
                }
            } else {
                config.unify_relay_channels();
                ConfigStore::new(destination.clone()).save(&config)?;
            }
            break;
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<AppConfig> {
        if !self.path.exists() {
            let config = AppConfig::default();
            self.save(&config)?;
            return Ok(config);
        }

        let bytes = fs::read(&self.path)
            .with_context(|| format!("failed to read {}", self.path.display()))?;
        let (config, migrated) = deserialize_config(&bytes)
            .with_context(|| format!("failed to parse {}", self.path.display()))?;
        config.validate()?;
        if migrated {
            self.save(&config)?;
        }
        Ok(config)
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        config.validate()?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let temporary_path = temporary_path(&self.path);
        let bytes = serde_json::to_vec_pretty(config)?;
        let mut file = fs::File::create(&temporary_path)
            .with_context(|| format!("failed to create {}", temporary_path.display()))?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary_path, &self.path)
            .with_context(|| format!("failed to replace {}", self.path.display()))?;
        Ok(())
    }
}

fn deserialize_config(bytes: &[u8]) -> Result<(AppConfig, bool)> {
    let (mut config, mut migrated) = deserialize_former_config(bytes)?;
    // 1.4.1 reads messages and media from one channel. Two different former
    // channels are kept as they are until the user chooses in Relay.
    migrated |= config.unify_relay_channels();
    Ok((config, migrated))
}

/// Settings renamed in 1.4.1: they were named after the removed speech feature.
const RENAMED_KEYS: [(&str, &str); 6] = [
    ("ttsChannelId", "formerMessageChannelId"),
    ("ttsCleanupEnabled", "formerMessageCleanupEnabled"),
    ("ttsWelcomeMessageId", "formerMessageWelcomeMessageId"),
    ("ttsCharacterLimit", "notificationCharacterLimit"),
    ("ttsQueueLimit", "notificationQueueLimit"),
    ("ttsNotificationsObsEnabled", "notificationsObsEnabled"),
];

/// Moves former keys to their new names. When a file holds both, the new
/// name wins, so a stray old key can never make the whole file unreadable.
fn rename_former_keys(value: &mut serde_json::Value) -> bool {
    let Some(config) = value.as_object_mut() else {
        return false;
    };
    let mut renamed = false;
    for (old, new) in RENAMED_KEYS {
        if let Some(previous) = config.remove(old) {
            renamed = true;
            config.entry(new).or_insert(previous);
        }
    }
    renamed
}

/// Reads a saved configuration with every migration except the channel unification.
fn deserialize_former_config(bytes: &[u8]) -> Result<(AppConfig, bool)> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)?;
    let renamed = rename_former_keys(&mut value);
    let missing_gif_duration = value.get("gifDurationMs").is_none();
    let missing_sticker_duration = value.get("stickerDurationMs").is_none();
    let mut migrated = renamed || missing_gif_duration || missing_sticker_duration;
    if missing_gif_duration {
        let duration = value
            .get("displayDurationMs")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(DEFAULT_DISPLAY_DURATION_MS));
        if let Some(config) = value.as_object_mut() {
            config.insert("gifDurationMs".into(), duration);
        }
    }
    if missing_sticker_duration && let Some(config) = value.as_object_mut() {
        config.insert(
            "stickerDurationMs".into(),
            serde_json::json!(DEFAULT_STICKER_DURATION_MS),
        );
    }
    // Migrate the two generated notification sizes that preceded the denser
    // toast. Explicitly customized dimensions remain untouched.
    let notification_width = value
        .get("notificationWidgetWidth")
        .and_then(|width| width.as_f64());
    let notification_height = value
        .get("notificationWidgetHeight")
        .and_then(|height| height.as_f64());
    let generated_notification_size = matches!(
        (notification_width, notification_height),
        (Some(width), Some(height))
            if (width, height)
                == (
                    LEGACY_NOTIFICATION_WIDGET_WIDTH,
                    LEGACY_NOTIFICATION_WIDGET_HEIGHT,
                )
                || (width, height)
                    == (
                        PREVIOUS_NOTIFICATION_WIDGET_WIDTH,
                        PREVIOUS_NOTIFICATION_WIDGET_HEIGHT,
                    )
    );
    if generated_notification_size {
        if let Some(config) = value.as_object_mut() {
            config.insert(
                "notificationWidgetWidth".into(),
                serde_json::json!(DEFAULT_NOTIFICATION_WIDGET_WIDTH),
            );
            config.insert(
                "notificationWidgetHeight".into(),
                serde_json::json!(DEFAULT_NOTIFICATION_WIDGET_HEIGHT),
            );
            // Saved positions are physical pixels while widget sizes are
            // logical units. Preserve the top-left here: shifting without a
            // monitor scale would misplace the window on high-DPI displays.
        }
        migrated = true;
    }
    // Missing notification widget size keys still pick up AppConfig::default
    // via serde(default). Never rewrite an explicitly saved custom size —
    // earlier builds forced any width < 900 / height < 170 back to 980×180
    // on every launch, which wiped custom card length from Overlay.
    Ok((serde_json::from_value(value)?, migrated))
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    PathBuf::from(temporary)
}

#[cfg(test)]
mod tests;
