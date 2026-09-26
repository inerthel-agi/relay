use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::privacy::PrivacyClassification;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorIdentity {
    pub username: String,
    pub display_avatar_url: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuildTagIdentity {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Image,
    Gif,
    Video,
    Audio,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaEvent {
    pub kind: MediaKind,
    pub url: String,
    pub proxy_url: String,
    pub filename: String,
    pub content_type: String,
    #[serde(default)]
    pub artwork_id: Option<String>,
    #[serde(default)]
    pub audio_id: Option<String>,
    #[serde(default)]
    pub cached_media_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub author: AuthorIdentity,
    pub timestamp: u64,
    pub message_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioPlaybackStatus {
    Playing,
    Paused,
    Idle,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioPlaybackState {
    pub status: AudioPlaybackStatus,
    pub target: String,
    pub media: MediaEvent,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioControlAction {
    Pause,
    Resume,
    Skip,
    Previous,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioControlEvent {
    pub action: AudioControlAction,
    pub media: Option<MediaEvent>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MusicPlaybackMode {
    Preview,
    Full,
    Custom,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicPlaybackEvent {
    pub playback_id: String,
    pub video_id: String,
    pub title: String,
    pub channel_title: String,
    pub thumbnail: String,
    pub duration_seconds: u64,
    pub mode: MusicPlaybackMode,
    pub start_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_seconds: Option<u64>,
    pub requested_by: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicHistoryEntry {
    pub music: MusicPlaybackEvent,
    pub timestamp: u64,
    #[serde(skip)]
    pub selection: crate::music::MusicSelection,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum HistoryEntry {
    Media(MediaEvent),
    Music(MusicHistoryEntry),
}

impl HistoryEntry {
    pub fn media(&self) -> Option<&MediaEvent> {
        match self {
            Self::Media(media) => Some(media),
            Self::Music(_) => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicStopEvent {
    pub playback_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicEndedEvent {
    pub playback_id: String,
    #[serde(default)]
    pub completed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingMedia {
    pub id: u64,
    pub media: MediaEvent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticker: Option<StickerEvent>,
    #[serde(skip)]
    pub sticker_bytes: Option<Arc<Vec<u8>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_classification: Option<PrivacyClassification>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub privacy_categories: Vec<crate::privacy::PrivacyCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_reason: Option<String>,
    /// Why moderation held it: "new_account", "raid", "safety_delay"...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_reason: Option<String>,
    /// Safety delay: published automatically at this time unless rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub queued_at: u64,
}

/// A notification message held for review instead of being dropped.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingText {
    pub id: u64,
    pub message_id: String,
    pub text: String,
    pub author: AuthorIdentity,
    #[serde(skip)]
    pub guild_tag: Option<GuildTagIdentity>,
    #[serde(skip)]
    pub segments: Vec<VisualSegment>,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    pub queued_at: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StickerEvent {
    pub id: String,
    pub name: String,
    pub format: String,
    pub url: String,
    pub cached_media_id: Option<String>,
    pub author: AuthorIdentity,
    pub timestamp: u64,
    pub message_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualSegment {
    pub kind: String,
    pub value: String,
    pub url: Option<String>,
    pub animated: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsEvent {
    pub id: String,
    pub text: String,
    pub author: AuthorIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_tag: Option<GuildTagIdentity>,
    pub content_type: String,
    pub timestamp: u64,
    pub visual_only: bool,
    pub segments: Vec<VisualSegment>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePinEvent {
    pub pinned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<TtsEvent>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputTestTarget {
    Visual,
    Audio,
    Tts,
    Notification,
    Sticker,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputTestEvent {
    pub target: OutputTestTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<MediaEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<TtsEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sticker: Option<StickerEvent>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BotStatus {
    pub connected: bool,
    pub username: Option<String>,
    pub display_avatar_url: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSummary {
    pub id: String,
    pub name: String,
    pub guild_name: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputConnectionStatus {
    pub obs_clients: usize,
    pub widget_clients: usize,
    pub preview_clients: usize,
    pub last_connected_at: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputStatuses {
    pub reaction: OutputConnectionStatus,
    pub visual: OutputConnectionStatus,
    pub audio: OutputConnectionStatus,
    pub tts: OutputConnectionStatus,
    pub notification: OutputConnectionStatus,
    pub sticker: OutputConnectionStatus,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub connected: bool,
    pub overlay_clients: usize,
    pub outputs: OutputStatuses,
    pub error: Option<String>,
}

/// Panel designs; outputs mirror the active one when their style is "auto".
pub const INTERFACE_DESIGNS: [&str; 6] = [
    "graphite",
    "paper",
    "neo-brutalism",
    "gridline",
    "lumen",
    "signal",
];
/// Output-only style that draws outlined text without a card.
pub const SUBTITLE_OUTPUT_STYLE: &str = "subtitle";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct InterfacePreferences {
    pub language: String,
    pub theme: String,
    pub accent_rgb: [u8; 3],
    pub font_scale: u8,
    pub design: String,
    /// "auto" follows `design`; otherwise a design or "subtitle".
    pub output_style: String,
    /// "auto" follows `theme`; otherwise "light" or "dark".
    pub output_background: String,
}

impl Default for InterfacePreferences {
    fn default() -> Self {
        Self {
            language: "en".into(),
            theme: "dark".into(),
            accent_rgb: [88, 185, 137],
            font_scale: 100,
            design: "graphite".into(),
            output_style: "auto".into(),
            output_background: "auto".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum RelayEvent {
    MusicHistory(MusicHistoryEntry),
    Reaction(Option<crate::reactions::ReactionPlayback>),
    MessagePin(MessagePinEvent),
    Media(MediaEvent),
    AudioPlayback(AudioPlaybackState),
    AudioControl(AudioControlEvent),
    MusicPlay(MusicPlaybackEvent),
    MusicStop(MusicStopEvent),
    /// Nothing left in the music queue; overlays may resume media.
    MusicIdle,
    Sticker(StickerEvent),
    Tts(TtsEvent),
    TestOutput(Box<OutputTestEvent>),
    Config(Box<AppConfig>),
    Clear,
    Skip,
    Appearance(InterfacePreferences),
    /// Panic button state; outputs ignore it, the panel and tray show it.
    OutputsPaused(bool),
}
