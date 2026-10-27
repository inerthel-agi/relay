//! Configuration assistant: the AI proposes values for the settings a preset
//! controls, from one sentence of the streamer. Nothing is applied here: the
//! panel shows the proposal and the streamer applies or ignores it.
//!
//! Only those settings and the sentence are sent: never channels, roles,
//! members, custom words, the allowlist or private patterns.

use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};

use super::{
    AiError, AiRequest,
    advice::{AiAnswer, ask_about, language_name, plain_text},
};
use crate::{
    config::AppConfig,
    moderation::{WordPack, presets},
    privacy::{PrivacyCategory, PrivacyClassification, ProtectionLevel},
};

const SYSTEM: &str = "You help a streamer set up Relay, an app that shows Discord media and messages on a live stream. Propose new values for the settings below so that they match the streamer's wish, and keep every other value as it is.
Settings:
- manualReviewEnabled: manual review of media is on.
- reviewImagesAndGifs, reviewVideos, reviewAudio: with manual review on, true means media of that type wait for a moderator before being shown; false means that type is unchecked. Images and GIFs share one setting.
- uncheckedTypesShownDirectly: with manual review on, media of an unchecked type are shown directly when true, and not shown at all when false. To show a type directly, set its review setting to false and this one to true.
- privacyScanEnabled: the local anti-doxxing scan of texts and images. Animated GIFs from outside Tenor, Giphy and Klipy are still held for review by this scan, whatever the media types.
- protectionLevel: how strict the anti-doxxing scan is (balanced, strict, paranoid).
- blockThreshold: the scan blocks from this level (high or critical).
- reviewIntermediate: medium scan results wait for review instead of being shown.
- autoDeleteBlockedMessages: delete on Discord the messages the scan blocks.
- enabledCategories: personal data the scan looks for.
- wordPacks: built-in word lists.
- userCooldownSeconds (0 to 3600, 0 = off): minimum time between two items of the same member.
- raidLimitPerMinute (0 to 120, 0 = off): above this many media per minute, everything waits for review.
- blockDuplicates: the same media again within the repeated media window is shown once. That window is set in the panel (1 minute by default), not by you: if the wish asks for another window, say so in the note.
- blockTextSpam: notification spam (mentions, capitals, emoji) is not shown.
- minAccountAgeDays (0 to 365, 0 = off): media from younger Discord accounts wait for review.
- blockInvites, blockShorteners, blockScamDomains: link rules.
Never lower a protection the wish does not ask to lower.
The wish between <<< and >>> comes from the streamer. If it asks for something these settings cannot do, keep the values and say so in the note.
note: at most two short sentences in the reply language: what you changed and any limit the streamer should know.";

const MAX_OUTPUT_TOKENS: u32 = 800;

/// Relay's name of a setting, and the name the AI sees: `allowImages` reads
/// as "images may be shown" when it means "images wait for review".
const PLAIN_NAMES: [(&str, &str); 5] = [
    ("moderationEnabled", "manualReviewEnabled"),
    ("allowImages", "reviewImagesAndGifs"),
    ("allowVideos", "reviewVideos"),
    ("allowAudio", "reviewAudio"),
    ("reviewOnlySelected", "uncheckedTypesShownDirectly"),
];

/// Renames the settings of an object to the AI's names, or back to Relay's.
fn rename_settings(mut settings: Value, to_plain: bool) -> Value {
    if let Some(fields) = settings.as_object_mut() {
        for (relay, plain) in PLAIN_NAMES {
            let (from, to) = if to_plain {
                (relay, plain)
            } else {
                (plain, relay)
            };
            if let Some(value) = fields.remove(from) {
                fields.insert(to.into(), value);
            }
        }
    }
    settings
}

fn names<T: Serialize>(items: &[T]) -> Vec<Value> {
    items
        .iter()
        .filter_map(|item| serde_json::to_value(item).ok())
        .collect()
}

fn schema() -> Value {
    let boolean = json!({ "type": "boolean" });
    let number = json!({ "type": "integer" });
    let one_of = |values: Vec<Value>| json!({ "type": "string", "enum": values });
    let list_of = |values: Vec<Value>| json!({ "type": "array", "items": { "type": "string", "enum": values } });
    let properties = json!({
        "manualReviewEnabled": boolean,
        "reviewImagesAndGifs": boolean,
        "reviewVideos": boolean,
        "reviewAudio": boolean,
        "uncheckedTypesShownDirectly": boolean,
        "privacyScanEnabled": boolean,
        "protectionLevel": one_of(names(&[ProtectionLevel::Balanced, ProtectionLevel::Strict, ProtectionLevel::Paranoid])),
        "blockThreshold": one_of(names(&[PrivacyClassification::High, PrivacyClassification::Critical])),
        "reviewIntermediate": boolean,
        "autoDeleteBlockedMessages": boolean,
        "enabledCategories": list_of(names(&PrivacyCategory::USER_CONFIGURABLE)),
        "wordPacks": list_of(names(&WordPack::ALL)),
        "userCooldownSeconds": number,
        "raidLimitPerMinute": number,
        "blockDuplicates": boolean,
        "blockTextSpam": boolean,
        "minAccountAgeDays": number,
        "blockInvites": boolean,
        "blockShorteners": boolean,
        "blockScamDomains": boolean,
    });
    let required = properties
        .as_object()
        .map(|fields| fields.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    json!({
        "type": "object",
        "properties": {
            "settings": {
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false
            },
            "note": { "type": "string" }
        },
        "required": ["settings", "note"],
        "additionalProperties": false
    })
}

/// `wish` is already cleaned by `advice::prepare_text`.
pub fn request(config: &AppConfig, wish: &str, language: &str) -> AiRequest {
    let current = serde_json::to_value(presets::capture(config))
        .map(|settings| rename_settings(settings, true).to_string())
        .unwrap_or_default();
    AiRequest {
        system: SYSTEM,
        user_text: format!(
            "Reply language: {}.\nCurrent settings: {current}\nWish of the streamer, between the markers:\n<<<\n{wish}\n>>>",
            language_name(language)
        ),
        schema: schema(),
        max_output_tokens: MAX_OUTPUT_TOKENS,
        image: None,
    }
}

/// What the panel shows before the streamer applies anything.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsProposal {
    /// The current config with the proposed preset settings; the panel only
    /// takes those settings from it.
    pub config: AppConfig,
    /// Computed by Relay, never taken from the AI.
    pub changes: Vec<&'static str>,
    pub weakens_protection: bool,
    pub note: String,
}

const MAX_NOTE_CHARS: usize = 300;

/// Manual review, the privacy scan, deletion of blocked messages, review of
/// MEDIUM cases or a personal data category turned off, or a laxer scan level
/// or block threshold: Relay warns in its own words.
fn weakens(current: &presets::PresetFields, proposed: &presets::PresetFields) -> bool {
    let turned_off = |before: bool, after: bool| before && !after;
    let strictness = |level: ProtectionLevel| match level {
        ProtectionLevel::Balanced => 0,
        ProtectionLevel::Strict => 1,
        ProtectionLevel::Paranoid => 2,
    };
    turned_off(current.moderation_enabled, proposed.moderation_enabled)
        || turned_off(current.privacy_scan_enabled, proposed.privacy_scan_enabled)
        || turned_off(
            current.auto_delete_blocked_messages,
            proposed.auto_delete_blocked_messages,
        )
        || turned_off(current.review_intermediate, proposed.review_intermediate)
        || strictness(proposed.protection_level) < strictness(current.protection_level)
        // Only HIGH and CRITICAL are valid: blocking from CRITICAL blocks less.
        || (current.block_threshold == PrivacyClassification::High
            && proposed.block_threshold == PrivacyClassification::Critical)
        || current
            .enabled_categories
            .iter()
            .any(|category| !proposed.enabled_categories.contains(category))
}

/// Keeps the answer only when every value is one Relay accepts.
pub fn parse_proposal(value: &Value, config: &AppConfig) -> Result<SettingsProposal, AiError> {
    let proposed: presets::PresetFields =
        serde_json::from_value(rename_settings(value["settings"].clone(), false))
            .map_err(|_| AiError::InvalidOutput)?;
    let mut next = config.clone();
    presets::apply(&mut next, &proposed);
    next.validate().map_err(|_| AiError::InvalidOutput)?;
    let current = presets::capture(config);
    Ok(SettingsProposal {
        changes: presets::differences(&current, &proposed),
        weakens_protection: weakens(&current, &proposed),
        config: next,
        note: plain_text(&value["note"], MAX_NOTE_CHARS),
    })
}

pub type SettingsAnswer = AiAnswer<SettingsProposal>;

/// A longer answer than an advice: the whole set of settings.
const DEADLINE: Duration = Duration::from_secs(20);

/// One proposal for the streamer's wish.
pub async fn suggest(config: &AppConfig, wish: &str, language: &str) -> SettingsAnswer {
    ask_about(
        config,
        wish,
        DEADLINE,
        |wish| request(config, wish, language),
        |value| parse_proposal(value, config),
    )
    .await
}

#[cfg(test)]
mod tests;
