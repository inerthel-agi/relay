use super::*;

#[test]
fn retention_preserves_recent_future_and_welcome_messages() {
    let now = 10_000_000;
    assert!(!expired(1, now - RETENTION_SECONDS + 1, now, None));
    assert!(expired(1, now - RETENTION_SECONDS, now, None));
    assert!(expired(
        1,
        now - 30 * RETENTION_SECONDS,
        now + 30 * RETENTION_SECONDS,
        None
    ));
    assert!(!expired(1, 0, now, Some(1)));
    assert!(!expired(1, now + 1, now, None));
}

#[test]
fn channels_are_independent_and_disabled_by_default() {
    let mut config = AppConfig::default();
    assert!(rule(&config, ChannelKind::Media).is_none());
    assert!(rule(&config, ChannelKind::FormerMessages).is_none());
    config.watched_channel_id = "2".into();
    config.media_cleanup_enabled = true;
    config.media_welcome_message_id = "7".into();
    assert_eq!(
        rule(&config, ChannelKind::Media),
        Some(CleanupRule {
            channel: 2,
            welcome: Some(7),
            reaction_protected: None,
        })
    );
    assert!(rule(&config, ChannelKind::FormerMessages).is_none());
    config.media_welcome_message_id = "invalid".into();
    assert!(rule(&config, ChannelKind::Media).is_none());
}

#[test]
fn welcome_link_must_belong_to_selected_channel_and_can_be_omitted() {
    assert_eq!(welcome_message_id("", "2"), Ok(String::new()));
    assert_eq!(
        welcome_message_id("https://discord.com/channels/1/2/7", "2"),
        Ok("7".into())
    );
    assert!(welcome_message_id("https://discord.com/channels/1/3/7", "2").is_err());
}

#[test]
fn cleanup_rule_tracks_the_optional_reaction_protection() {
    let config = AppConfig {
        watched_channel_id: "2".into(),
        media_cleanup_enabled: true,
        reactions: crate::reactions::ReactionSettings {
            protected_channel_id: "2".into(),
            protected_message_id: "9".into(),
            ..Default::default()
        },
        ..AppConfig::default()
    };
    assert_eq!(
        rule(&config, ChannelKind::Media)
            .unwrap()
            .reaction_protected,
        Some(9)
    );
}
