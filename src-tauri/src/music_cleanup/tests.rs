use super::*;

#[test]
fn welcome_message_links_must_match_the_music_channel() {
    assert_eq!(
        protected_message_id("https://discord.com/channels/1/2/3", "2"),
        Ok("3".into())
    );
    assert!(protected_message_id("https://discord.com/channels/1/4/3", "2").is_err());
    assert!(protected_message_id("0", "2").is_err());
    assert!(protected_message_id("not-an-id", "2").is_err());
    assert_eq!(protected_message_id("3", "2"), Ok("3".into()));
}

#[test]
fn cleanup_preserves_welcome_and_active_messages() {
    assert!(!may_delete(7, Some(7), false));
    assert!(!may_delete(8, Some(7), true));
    assert!(may_delete(8, Some(7), false));
}

#[test]
fn confirmation_is_single_use_and_bound_to_channel_message_and_expiration() {
    let now = Instant::now();
    for (token, scope, time, valid) in [
        ("one", (2, Some(7)), now, true),
        ("wrong", (2, Some(7)), now, false),
        ("one", (3, Some(7)), now, false),
        ("one", (2, Some(8)), now, false),
        ("one", (2, Some(7)), now + TTL, false),
    ] {
        let mut cleanup = MusicCleanup {
            preview: Some(CleanupSnapshot {
                token: "one".into(),
                channel: 2,
                protected: Some(7),
                messages: vec![8, 9],
                expires: now + TTL,
            }),
            ..Default::default()
        };
        let result = cleanup.take_preview(token, scope, time);
        assert_eq!(result.is_ok(), valid);
        if let Ok(snapshot) = result {
            assert_eq!(snapshot.messages, vec![8, 9]);
        }
        assert!(cleanup.take_preview("one", (2, Some(7)), now).is_err());
    }
}

#[tokio::test]
async fn automatic_cleanup_does_not_request_deletion_for_protected_or_foreign_messages() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    {
        let mut config = core.config.write().await;
        config.music_cleanup_enabled = true;
        config.music_channel_id = "2".into();
        config.music_welcome_message_id = "7".into();
    }
    let http = Http::new("unused-test-token");
    delete(&core, &http, 2, 7).await;
    delete(&core, &http, 3, 8).await;
    assert!(core.bot_status.read().await.error.is_none());
}

#[tokio::test]
async fn missing_welcome_message_is_optional_for_configuration_and_verification() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    {
        let mut config = core.config.write().await;
        config.music_cleanup_enabled = true;
        config.music_channel_id = "2".into();
    }
    assert_eq!(configured(&core).await.unwrap(), (2, None));
    let http = Http::new("unused-test-token");
    assert!(
        verify_protected_message(&http, 2, None, "unused")
            .await
            .is_ok()
    );
    assert!(may_delete(8, None, false));
    assert!(!may_delete(8, None, true));
    assert_eq!(optional_protected_message("  ").unwrap(), None);
    assert!(optional_protected_message("0").is_err());
    assert!(optional_protected_message("invalid").is_err());
}

#[tokio::test]
async fn reaction_protection_keeps_its_message_out_of_music_cleanup() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    {
        let mut config = core.config.write().await;
        config.reactions.protected_channel_id = "2".into();
        config.reactions.protected_message_id = "9".into();
    }
    assert!(reaction_protected(&core, 2, 9).await);
    assert!(!reaction_protected(&core, 2, 8).await);
    assert!(!reaction_protected(&core, 3, 9).await);
}

#[test]
fn changing_optional_protection_invalidates_the_preview() {
    let now = Instant::now();
    for scope in [(2, None), (2, Some(7)), (3, None)] {
        let mut cleanup = MusicCleanup {
            preview: Some(CleanupSnapshot {
                token: "one".into(),
                channel: 2,
                protected: None,
                messages: vec![8],
                expires: now + TTL,
            }),
            ..Default::default()
        };
        assert_eq!(
            cleanup.take_preview("one", scope, now).is_ok(),
            scope == (2, None)
        );
    }
}

#[test]
fn unrelated_saves_do_not_reverify_unchanged_music_settings() {
    let previous = crate::config::AppConfig {
        music_cleanup_enabled: true,
        music_channel_id: "2".into(),
        music_welcome_message_id: "7".into(),
        ..Default::default()
    };
    assert!(!requires_verification(&previous, true, "2", "7"));
    assert!(requires_verification(&previous, true, "3", "7"));
    assert!(!requires_verification(&previous, true, "2", ""));
    assert!(!requires_verification(&previous, false, "2", ""));
    let disabled = crate::config::AppConfig {
        music_cleanup_enabled: false,
        ..previous
    };
    assert!(requires_verification(&disabled, true, "2", "7"));
    assert!(!requires_verification(&disabled, true, "2", ""));
}
