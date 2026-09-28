//! Checks what the Discord bot can actually do for every configured feature,
//! so the panel can say exactly which permission or setting is missing.

use std::sync::Arc;

use anyhow::{Context, Result};
use serde::Serialize;
use serenity::all::{ApplicationFlags, Channel, ChannelId, Permissions};

use crate::{config::AppConfig, config::HoneypotAction, state::AppCore};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IntentState {
    Enabled,
    Missing,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    Ok,
    MissingPermissions,
    ChannelNotFound,
    NotConfigured,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureCheck {
    pub feature: &'static str,
    pub channel_name: Option<String>,
    pub status: CheckStatus,
    /// Discord permission names, for example `MANAGE_MESSAGES`.
    pub missing: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordSetupReport {
    pub bot_connected: bool,
    pub message_content_intent: IntentState,
    pub checks: Vec<FeatureCheck>,
}

/// Permissions each feature needs in its channel, derived from the current settings.
pub fn required_permissions(feature: &str, config: &AppConfig) -> Permissions {
    let read = Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY;
    // Blocked messages are deleted by filter words and link rules too, not only by the scan.
    let privacy_delete = config.privacy_auto_delete_blocked_messages
        && crate::privacy::privacy_rules_enabled(config);
    let escalation = if config.moderation.escalation_blocks > 0 {
        Permissions::MODERATE_MEMBERS
    } else {
        Permissions::empty()
    };
    let delete_if = |needed: bool| {
        if needed {
            Permissions::MANAGE_MESSAGES
        } else {
            Permissions::empty()
        }
    };
    match feature {
        "relay" | "media" => {
            read | delete_if(config.media_cleanup_enabled || privacy_delete) | escalation
        }
        "notifications" => {
            read | delete_if(config.former_message_cleanup_enabled || privacy_delete) | escalation
        }
        "music" => {
            read | Permissions::SEND_MESSAGES
                | Permissions::EMBED_LINKS
                | delete_if(config.music_cleanup_enabled)
        }
        "honeypot" => {
            read | Permissions::MANAGE_MESSAGES
                | match config.honeypot_action {
                    HoneypotAction::Kick => Permissions::KICK_MEMBERS,
                    HoneypotAction::Ban => Permissions::BAN_MEMBERS,
                    HoneypotAction::Timeout => Permissions::MODERATE_MEMBERS,
                }
        }
        _ => Permissions::empty(),
    }
}

/// Features to check with their channel. Two former channels awaiting the
/// user's choice are still checked separately, in their former roles.
fn checked_features(config: &AppConfig) -> Vec<(&'static str, String)> {
    let mut features = if config.relay_channel_conflict() {
        vec![
            ("media", config.watched_channel_id.clone()),
            ("notifications", config.former_message_channel_id.clone()),
        ]
    } else {
        vec![("relay", config.watched_channel_id.clone())]
    };
    features.push(("music", config.music_channel_id.clone()));
    features.push(("honeypot", config.honeypot_channel_id.clone()));
    features
}

pub fn missing_permission_names(required: Permissions, granted: Permissions) -> Vec<String> {
    if granted.contains(Permissions::ADMINISTRATOR) {
        return Vec::new();
    }
    (required - granted)
        .iter_names()
        .map(|(name, _)| name.to_owned())
        .collect()
}

/// A gateway refusal (close code 4014) is the only reliable sign when the bot is offline.
fn intent_from_error(error: Option<&str>) -> IntentState {
    match error {
        Some(error) if error.to_ascii_lowercase().contains("disallowed") => IntentState::Missing,
        _ => IntentState::Unknown,
    }
}

pub async fn check(core: &Arc<AppCore>) -> Result<DiscordSetupReport> {
    let status = core.bot_status.read().await.clone();
    let config = core.config.read().await.clone();
    let features = checked_features(&config);
    let runtime = {
        let runtime = core.bot_runtime.lock().await;
        runtime
            .as_ref()
            .map(|runtime| (runtime.http.clone(), runtime.cache.clone()))
    };
    let Some((http, cache)) = runtime.filter(|_| status.connected) else {
        return Ok(DiscordSetupReport {
            bot_connected: false,
            message_content_intent: intent_from_error(status.error.as_deref()),
            checks: Vec::new(),
        });
    };

    let application = http
        .get_current_application_info()
        .await
        .context("Discord did not return the application settings")?;
    let flags = application.flags.unwrap_or(ApplicationFlags::empty());
    let message_content_intent = if flags.intersects(
        ApplicationFlags::GATEWAY_MESSAGE_CONTENT
            | ApplicationFlags::GATEWAY_MESSAGE_CONTENT_LIMITED,
    ) {
        IntentState::Enabled
    } else {
        IntentState::Missing
    };

    let bot_id = cache.current_user().id;
    let mut checks = Vec::new();
    for (feature, channel_id) in features {
        let Some(id) = channel_id.parse::<u64>().ok().filter(|id| *id != 0) else {
            checks.push(FeatureCheck {
                feature,
                channel_name: None,
                status: CheckStatus::NotConfigured,
                missing: Vec::new(),
            });
            continue;
        };
        let channel = match ChannelId::new(id).to_channel(&http).await {
            Ok(Channel::Guild(channel)) => channel,
            _ => {
                checks.push(FeatureCheck {
                    feature,
                    channel_name: None,
                    status: CheckStatus::ChannelNotFound,
                    missing: Vec::new(),
                });
                continue;
            }
        };
        #[allow(deprecated)]
        let granted = channel
            .permissions_for_user(&cache, bot_id)
            .unwrap_or(Permissions::empty());
        let missing = missing_permission_names(required_permissions(feature, &config), granted);
        checks.push(FeatureCheck {
            feature,
            channel_name: Some(channel.name.clone()),
            status: if missing.is_empty() {
                CheckStatus::Ok
            } else {
                CheckStatus::MissingPermissions
            },
            missing,
        });
    }
    Ok(DiscordSetupReport {
        bot_connected: true,
        message_content_intent,
        checks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirements_follow_the_enabled_options() {
        let mut config = AppConfig::default();
        assert!(!required_permissions("media", &config).contains(Permissions::MANAGE_MESSAGES));
        config.media_cleanup_enabled = true;
        assert!(required_permissions("media", &config).contains(Permissions::MANAGE_MESSAGES));
        assert!(required_permissions("music", &config).contains(Permissions::EMBED_LINKS));
        config.honeypot_action = HoneypotAction::Ban;
        let honeypot = required_permissions("honeypot", &config);
        assert!(honeypot.contains(Permissions::BAN_MEMBERS));
        assert!(!honeypot.contains(Permissions::KICK_MEMBERS));
    }

    #[test]
    fn one_relay_channel_is_checked_unless_a_choice_is_pending() {
        let mut config = AppConfig {
            watched_channel_id: "123456789012345678".into(),
            ..AppConfig::default()
        };
        let features = checked_features(&config);
        assert_eq!(features[0], ("relay", "123456789012345678".to_owned()));
        assert!(
            !features
                .iter()
                .any(|(feature, _)| *feature == "notifications")
        );
        config.media_cleanup_enabled = true;
        assert!(required_permissions("relay", &config).contains(Permissions::MANAGE_MESSAGES));

        config.former_message_channel_id = "223456789012345678".into();
        let features = checked_features(&config);
        assert_eq!(features[0], ("media", "123456789012345678".to_owned()));
        assert_eq!(
            features[1],
            ("notifications", "223456789012345678".to_owned())
        );
    }

    #[test]
    fn missing_names_ignore_granted_and_administrator() {
        let required = Permissions::VIEW_CHANNEL | Permissions::MANAGE_MESSAGES;
        assert_eq!(
            missing_permission_names(required, Permissions::VIEW_CHANNEL),
            vec!["MANAGE_MESSAGES".to_owned()]
        );
        assert!(missing_permission_names(required, Permissions::ADMINISTRATOR).is_empty());
    }

    #[test]
    fn a_refused_gateway_means_the_intent_is_off() {
        assert_eq!(
            intent_from_error(Some(
                "Discord connection failed: Disallowed gateway intents"
            )),
            IntentState::Missing
        );
        assert_eq!(
            intent_from_error(Some("network down")),
            IntentState::Unknown
        );
        assert_eq!(intent_from_error(None), IntentState::Unknown);
    }
}
