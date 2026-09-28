//! Validation and cleanup guards for the optional protected reaction message.

use anyhow::{Result, bail};
use serenity::{
    all::{ChannelId, MessageId},
    http::Http,
};

use crate::state::AppCore;

const DISCORD_MESSAGE_LINK_PREFIX: &str = "https://discord.com/channels/";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedProtection {
    pub channel_id: String,
    pub message_id: String,
}

/// Normalize a message ID or Discord message link into the two IDs stored by Relay.
///
/// A link supplies its channel. A bare message ID uses the selected channel and,
/// when the selector is temporarily blank, the previous protected channel.
pub fn normalize(
    value: &str,
    selected_channel: &str,
    previous_channel: &str,
    previous_message: &str,
    allowed_channels: &[String],
) -> Result<NormalizedProtection, String> {
    let value = value.trim();
    let selected_channel = selected_channel.trim();
    let previous_channel = previous_channel.trim();
    if value.is_empty() {
        return Ok(NormalizedProtection {
            channel_id: String::new(),
            message_id: String::new(),
        });
    }

    if let Some(path) = value.strip_prefix(DISCORD_MESSAGE_LINK_PREFIX) {
        let parts: Vec<_> = path.trim_end_matches('/').split('/').collect();
        if parts.len() != 3 {
            return Err("Enter a complete Discord message link or message ID.".into());
        }
        let guild = parse_id(parts[0])
            .ok_or("The protected message link must contain valid Discord IDs.")?;
        let channel = parse_id(parts[1])
            .ok_or("The protected message link must contain valid Discord IDs.")?;
        let message = parse_id(parts[2])
            .ok_or("The protected message link must contain valid Discord IDs.")?;
        if guild == 0 || channel == 0 || message == 0 {
            return Err("The protected message link must contain valid Discord IDs.".into());
        }
        if !contains_channel(allowed_channels, channel) {
            return Err(
                "The protected message channel must be one of the allowed Discord channels.".into(),
            );
        }
        return Ok(NormalizedProtection {
            channel_id: channel.to_string(),
            message_id: message.to_string(),
        });
    }

    if value.contains("://") || value.contains('/') {
        return Err(
            "Enter a Discord message ID or an https://discord.com/channels/... message link."
                .into(),
        );
    }
    let message = parse_id(value).ok_or("Enter a valid protected Discord message ID.")?;
    let selected = parse_id(selected_channel).filter(|id| contains_channel(allowed_channels, *id));
    let sole_allowed = (allowed_channels.len() == 1)
        .then(|| parse_id(allowed_channels[0].trim()))
        .flatten();
    let previous = parse_id(previous_channel)
        .filter(|id| contains_channel(allowed_channels, *id) && previous_message.trim() == value);
    let channel = selected.or(sole_allowed).or(previous).ok_or_else(|| {
        if allowed_channels.len() > 1 {
            "Choose a full Discord message link when multiple reaction channels are allowed."
                .to_owned()
        } else {
            "Choose an allowed Discord channel for the protected message.".to_owned()
        }
    })?;
    Ok(NormalizedProtection {
        channel_id: channel.to_string(),
        message_id: message.to_string(),
    })
}

pub fn validate(channel_id: &str, message_id: &str, allowed_channels: &[String]) -> Result<()> {
    let channel_id = channel_id.trim();
    let message_id = message_id.trim();
    if channel_id.is_empty() && message_id.is_empty() {
        return Ok(());
    }
    if channel_id.is_empty() {
        bail!("A protected message channel is required.");
    }
    if message_id.is_empty() {
        bail!("A protected message ID is required.");
    }
    if parse_id(channel_id).is_none_or(|id| id == 0) {
        bail!("The protected message channel ID is invalid.");
    }
    if parse_id(message_id).is_none_or(|id| id == 0) {
        bail!("The protected message ID is invalid.");
    }
    if !contains_channel(allowed_channels, parse_id(channel_id).unwrap_or_default()) {
        bail!("The protected message channel must be one of the allowed Discord channels.");
    }
    Ok(())
}

pub fn requires_verification(
    previous_channel: &str,
    previous_message: &str,
    next_channel: &str,
    next_message: &str,
) -> bool {
    !next_message.trim().is_empty()
        && (previous_channel.trim() != next_channel.trim()
            || previous_message.trim() != next_message.trim())
}

pub async fn verify(core: &AppCore, channel_id: &str, message_id: &str) -> Result<(), String> {
    let channel = parse_id(channel_id)
        .filter(|id| *id != 0)
        .ok_or("The protected message channel ID is invalid.")?;
    let message = parse_id(message_id)
        .filter(|id| *id != 0)
        .ok_or("The protected message ID is invalid.")?;
    let http = bot_http(core).await?;
    ChannelId::new(channel)
        .message(&http, MessageId::new(message))
        .await
        .map_err(|_| {
            String::from("The protected Discord message was not found in the selected channel.")
        })?;
    Ok(())
}

pub fn protected_message_for_channel(
    protected_channel_id: &str,
    protected_message_id: &str,
    channel_id: u64,
) -> Option<u64> {
    let protected_channel = parse_id(protected_channel_id).filter(|id| *id != 0)?;
    if protected_channel != channel_id {
        return None;
    }
    parse_id(protected_message_id).filter(|id| *id != 0)
}

pub fn is_protected(
    protected_channel_id: &str,
    protected_message_id: &str,
    channel_id: u64,
    message_id: u64,
) -> bool {
    protected_message_for_channel(protected_channel_id, protected_message_id, channel_id)
        == Some(message_id)
}

pub fn is_channel_protected(
    protected_channel_id: &str,
    protected_message_id: &str,
    channel_id: u64,
) -> bool {
    protected_message_for_channel(protected_channel_id, protected_message_id, channel_id).is_some()
}

async fn bot_http(core: &AppCore) -> Result<std::sync::Arc<Http>, String> {
    if !core.bot_status.read().await.connected {
        return Err("Connect the Discord bot to verify the protected message.".into());
    }
    core.discord_http()
        .await
        .ok_or_else(|| "Connect the Discord bot to verify the protected message.".into())
}

fn parse_id(value: &str) -> Option<u64> {
    value.parse::<u64>().ok().filter(|id| *id > 0)
}

fn contains_channel(allowed_channels: &[String], channel_id: u64) -> bool {
    allowed_channels
        .iter()
        .any(|allowed| parse_id(allowed.trim()) == Some(channel_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_protection_is_optional_and_clears_both_ids() {
        let allowed = vec!["2".into()];
        assert_eq!(
            normalize("", "", "2", "9", &allowed).unwrap(),
            NormalizedProtection {
                channel_id: String::new(),
                message_id: String::new(),
            }
        );
        assert!(validate("", "", &allowed).is_ok());
    }

    #[test]
    fn links_are_normalized_independently_of_a_stale_stored_channel() {
        let allowed = vec!["2".into(), "3".into()];
        assert_eq!(
            normalize("https://discord.com/channels/1/2/3", "2", "", "", &allowed).unwrap(),
            NormalizedProtection {
                channel_id: "2".into(),
                message_id: "3".into(),
            }
        );
        assert_eq!(
            normalize("https://discord.com/channels/1/3/4", "2", "", "", &allowed).unwrap(),
            NormalizedProtection {
                channel_id: "3".into(),
                message_id: "4".into(),
            }
        );
        let restricted = vec!["2".into()];
        assert!(
            normalize(
                "https://discord.com/channels/1/3/4",
                "2",
                "",
                "",
                &restricted
            )
            .is_err()
        );
    }

    #[test]
    fn bare_ids_use_the_selected_or_previous_channel() {
        let allowed = vec!["2".into(), "1".into()];
        assert_eq!(
            normalize("9", "2", "1", "8", &allowed).unwrap(),
            NormalizedProtection {
                channel_id: "2".into(),
                message_id: "9".into(),
            }
        );
        assert_eq!(
            normalize("9", "", "1", "9", &allowed).unwrap(),
            NormalizedProtection {
                channel_id: "1".into(),
                message_id: "9".into(),
            }
        );
        assert!(normalize("9", "", "", "", &allowed).is_err());
        let sole_allowed = vec!["4".into()];
        assert_eq!(
            normalize("9", "", "1", "8", &sole_allowed).unwrap(),
            NormalizedProtection {
                channel_id: "4".into(),
                message_id: "9".into(),
            }
        );
    }

    #[test]
    fn validation_rejects_a_protected_channel_outside_the_access_list() {
        let allowed = vec!["2".into()];
        assert!(validate("2", "9", &allowed).is_ok());
        assert!(validate("3", "9", &allowed).is_err());
        assert!(validate("", "9", &allowed).is_err());
    }

    #[test]
    fn only_the_configured_message_in_the_configured_channel_is_protected() {
        assert!(is_protected("2", "9", 2, 9));
        assert!(!is_protected("2", "9", 2, 8));
        assert!(!is_protected("2", "9", 3, 9));
        assert!(is_channel_protected("2", "9", 2));
        assert!(!is_channel_protected("2", "", 2));
    }

    #[test]
    fn verification_is_required_only_for_nonempty_changes() {
        assert!(!requires_verification("2", "9", "2", "9"));
        assert!(!requires_verification("2", "9", "", ""));
        assert!(requires_verification("2", "9", "3", "9"));
        assert!(requires_verification("2", "9", "2", "10"));
    }
}
