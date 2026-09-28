use super::*;

#[test]
fn creates_and_round_trips_default_config() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::new(directory.path().join("config.json"));

    let config = store.load().unwrap();
    assert_eq!(config, AppConfig::default());

    let updated = AppConfig {
        music_max_pending_per_user: 3,
        music_reject_duplicate_pending: true,
        reactions: crate::reactions::ReactionSettings::default(),
        watched_channel_id: "123456789012345678".into(),
        former_message_channel_id: "223456789012345678".into(),
        media_cleanup_enabled: true,
        media_welcome_message_id: "623456789012345678".into(),
        former_message_cleanup_enabled: true,
        former_message_welcome_message_id: String::new(),
        music_channel_id: "323456789012345678".into(),
        music_cleanup_enabled: true,
        music_welcome_message_id: "523456789012345678".into(),
        honeypot_channel_id: "423456789012345678".into(),
        honeypot_action: HoneypotAction::Ban,
        port: 5_000,
        display_duration_ms: 4_000,
        gif_duration_ms: 6_000,
        sticker_duration_ms: 7_000,
        notification_duration_ms: 9_000,
        media_volume: 65,
        skip_shortcut: "control+shift+KeyK".into(),
        panic_shortcut: "control+shift+KeyP".into(),
        notification_character_limit: 280,
        notification_queue_limit: 24,
        notifications_obs_enabled: true,
        bot_online_status: "idle".into(),
        bot_activity_type: "watching".into(),
        bot_activity_text: "the media queue".into(),
        show_author: false,
        show_media_text_obs: true,
        show_media_text_widget: true,
        moderation_enabled: true,
        moderation_allow_images: true,
        moderation_allow_videos: false,
        moderation_allow_audio: true,
        privacy_scan_enabled: true,
        privacy_similarity_boost: 4,
        privacy_concepts: Vec::new(),
        privacy_filter_exempt_role_ids: Vec::new(),
        privacy_protection_level: ProtectionLevel::Strict,
        privacy_enabled_categories: default_privacy_categories(),
        privacy_block_threshold: PrivacyClassification::Critical,
        privacy_review_intermediate: true,
        privacy_auto_delete_blocked_messages: false,
        privacy_allowlist: vec!["public@example.com".into()],
        privacy_custom_patterns: vec!["private alias".into()],
        moderation: crate::moderation::ModerationSettings::default(),
        command_channel_enabled: true,
        command_url_enabled: false,
        command_show_enabled: true,
        command_status_enabled: false,
        command_test_enabled: false,
        command_regenerate_enabled: false,
        command_clear_enabled: true,
        command_nuke_enabled: true,
        command_lock_enabled: true,
        command_changelog_enabled: false,
        custom_commands: vec![CustomCommandDefinition {
            name: "announce".into(),
            description: "Post the configured announcement".into(),
            action: crate::custom_commands::CustomCommandAction::Reply {
                text: "Configured locally".into(),
                ephemeral: false,
            },
            ..CustomCommandDefinition::default()
        }],
        channel_lock: None,
        interface_preferences: InterfacePreferences {
            language: "fr".into(),
            theme: "light".into(),
            accent_rgb: [21, 126, 88],
            font_scale: 110,
            ..InterfacePreferences::default()
        },
        widget_x: Some(-640),
        widget_y: Some(120),
        widget_width: 960.0,
        widget_height: 540.0,
        widget_keep_aspect_ratio: false,
        widget_visible: true,
        widget_locked: true,
        widget_sound_enabled: true,
        notification_widget_x: Some(900),
        notification_widget_y: Some(40),
        notification_widget_width: 620.0,
        notification_widget_height: 180.0,
        notification_widget_visible: true,
        notification_widget_locked: true,
        media_obs_geometry: OutputGeometry {
            crop_top: 4,
            crop_right: 8,
            crop_bottom: 12,
            crop_left: 16,
            content_scale: 125,
            ..OutputGeometry::default()
        },
        media_widget_geometry: OutputGeometry {
            content_scale: 80,
            ..OutputGeometry::default()
        },
        notification_obs_geometry: OutputGeometry {
            crop_left: 10,
            content_scale: 150,
            ..OutputGeometry::default()
        },
        notification_widget_geometry: OutputGeometry {
            crop_bottom: 20,
            content_scale: 90,
            ..OutputGeometry::default()
        },
        music_obs_geometry: OutputGeometry {
            anchor: OutputAnchor::BottomCenter,
            content_scale: 250,
            ..OutputGeometry::default()
        },
        music_video_obs_geometry: OutputGeometry {
            anchor: OutputAnchor::TopRight,
            content_scale: 60,
            crop_top: 10,
            ..OutputGeometry::default()
        },
        notification_sound_enabled: true,
        notification_sound_obs_enabled: true,
        notification_sound_path: Some("C:/sounds/ping.mp3".into()),
    };
    store.save(&updated).unwrap();
    assert_eq!(store.load().unwrap(), updated);
}

#[test]
fn interface_preferences_survive_a_config_round_trip() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let store = ConfigStore::new(path.clone());
    let config = AppConfig {
        interface_preferences: crate::model::InterfacePreferences {
            language: "ja".into(),
            theme: "light".into(),
            accent_rgb: [12, 34, 56],
            font_scale: 125,
            design: "signal".into(),
            output_style: "subtitle".into(),
            output_background: "light".into(),
        },
        ..AppConfig::default()
    };

    store.save(&config).unwrap();
    let loaded = ConfigStore::new(path).load().unwrap();

    assert_eq!(loaded.interface_preferences, config.interface_preferences);
}

#[test]
fn preferences_saved_before_output_styles_follow_the_design() {
    let preferences: InterfacePreferences = serde_json::from_value(serde_json::json!({
        "language": "fr",
        "theme": "light",
        "accentRgb": [1, 2, 3],
        "fontScale": 100
    }))
    .unwrap();

    assert_eq!(preferences.design, "graphite");
    assert_eq!(preferences.output_style, "auto");
    assert_eq!(preferences.output_background, "auto");
    assert!(validate_interface_preferences(&preferences).is_ok());
}

#[test]
fn rejects_unknown_designs_and_output_styles() {
    let valid = InterfacePreferences::default();
    for (design, style, background) in [
        ("openai", "auto", "auto"),
        ("graphite", "psn", "auto"),
        ("graphite", "auto", "grey"),
    ] {
        let preferences = InterfacePreferences {
            design: design.into(),
            output_style: style.into(),
            output_background: background.into(),
            ..valid.clone()
        };
        assert!(validate_interface_preferences(&preferences).is_err());
    }
    for style in ["auto", "signal", "subtitle", "paper"] {
        let preferences = InterfacePreferences {
            output_style: style.into(),
            ..valid.clone()
        };
        assert!(validate_interface_preferences(&preferences).is_ok());
    }
}

#[test]
fn rejects_invalid_config() {
    let config = AppConfig {
        port: 80,
        ..AppConfig::default()
    };
    assert!(config.validate().is_err());

    let duplicate_channels = AppConfig {
        watched_channel_id: "123456789012345678".into(),
        former_message_channel_id: "123456789012345678".into(),
        ..AppConfig::default()
    };
    assert!(duplicate_channels.validate().is_err());

    let duplicate_honeypot = AppConfig {
        watched_channel_id: "123456789012345678".into(),
        honeypot_channel_id: "123456789012345678".into(),
        ..AppConfig::default()
    };
    assert!(duplicate_honeypot.validate().is_err());

    let invalid_geometry = AppConfig {
        media_obs_geometry: OutputGeometry {
            crop_top: 41,
            ..OutputGeometry::default()
        },
        ..AppConfig::default()
    };
    assert!(invalid_geometry.validate().is_err());

    let invalid_scale = AppConfig {
        notification_widget_geometry: OutputGeometry {
            content_scale: 401,
            ..OutputGeometry::default()
        },
        ..AppConfig::default()
    };
    assert!(invalid_scale.validate().is_err());

    let invalid_size = AppConfig {
        widget_width: 159.0,
        ..AppConfig::default()
    };
    assert!(invalid_size.validate().is_err());

    let invalid_presence = AppConfig {
        bot_online_status: "offline".into(),
        ..AppConfig::default()
    };
    assert!(invalid_presence.validate().is_err());

    let invalid_skip_shortcut = AppConfig {
        skip_shortcut: "Control+Shift".into(),
        ..AppConfig::default()
    };
    assert!(invalid_skip_shortcut.validate().is_err());

    let invalid_activity = AppConfig {
        bot_activity_type: "streaming".into(),
        ..AppConfig::default()
    };
    assert!(invalid_activity.validate().is_err());

    let activity_too_long = AppConfig {
        bot_activity_text: "x".repeat(129),
        ..AppConfig::default()
    };
    assert!(activity_too_long.validate().is_err());

    let invalid_similarity_boost = AppConfig {
        privacy_similarity_boost: 0,
        ..AppConfig::default()
    };
    assert!(invalid_similarity_boost.validate().is_err());

    let invalid_exempt_role = AppConfig {
        privacy_filter_exempt_role_ids: vec!["123".into()],
        ..AppConfig::default()
    };
    assert!(invalid_exempt_role.validate().is_err());
    let zero_exempt_role = AppConfig {
        privacy_filter_exempt_role_ids: vec!["00000000000000000".into()],
        ..AppConfig::default()
    };
    assert!(zero_exempt_role.validate().is_err());

    let valid_exempt_role = AppConfig {
        privacy_filter_exempt_role_ids: vec!["123456789012345678".into()],
        ..AppConfig::default()
    };
    assert!(valid_exempt_role.validate().is_ok());

    let too_many_exempt_roles = AppConfig {
        privacy_filter_exempt_role_ids: (0..=MAX_PRIVACY_EXEMPT_ROLE_IDS)
            .map(|index| format!("{index:017}"))
            .collect(),
        ..AppConfig::default()
    };
    assert!(too_many_exempt_roles.validate().is_err());

    let invalid_block_threshold = AppConfig {
        privacy_block_threshold: PrivacyClassification::Medium,
        ..AppConfig::default()
    };
    assert!(invalid_block_threshold.validate().is_err());

    let duplicate_categories = AppConfig {
        privacy_enabled_categories: vec![PrivacyCategory::Email, PrivacyCategory::Email],
        ..AppConfig::default()
    };
    assert!(duplicate_categories.validate().is_err());

    let invalid_private_value = AppConfig {
        privacy_custom_patterns: vec!["ab".into()],
        ..AppConfig::default()
    };
    assert!(invalid_private_value.validate().is_err());

    let invalid_allowlist_value = AppConfig {
        privacy_allowlist: vec!["public\nvalue".into()],
        ..AppConfig::default()
    };
    assert!(invalid_allowlist_value.validate().is_err());
}

#[test]
fn migrates_missing_gif_duration_from_the_image_duration() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
    legacy.as_object_mut().unwrap().remove("gifDurationMs");
    legacy["displayDurationMs"] = serde_json::json!(15_000);
    fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

    let migrated = ConfigStore::new(path.clone()).load().unwrap();
    assert_eq!(migrated.display_duration_ms, 15_000);
    assert_eq!(migrated.gif_duration_ms, 15_000);
    let persisted: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(persisted["gifDurationMs"], 15_000);
}

#[test]
fn migrates_missing_sticker_duration_to_eight_seconds() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
    legacy.as_object_mut().unwrap().remove("stickerDurationMs");
    fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

    let migrated = ConfigStore::new(path.clone()).load().unwrap();
    assert_eq!(migrated.sticker_duration_ms, 8_000);
    let persisted: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(persisted["stickerDurationMs"], 8_000);
}

#[test]
fn preserves_custom_notification_widget_size_and_output_geometry() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::new(directory.path().join("config.json"));
    let updated = AppConfig {
        notification_widget_width: 540.0,
        notification_widget_height: 144.0,
        media_obs_geometry: OutputGeometry {
            content_scale: 140,
            crop_left: 8,
            ..OutputGeometry::default()
        },
        media_widget_geometry: OutputGeometry {
            content_scale: 120,
            ..OutputGeometry::default()
        },
        notification_obs_geometry: OutputGeometry {
            content_scale: 105,
            crop_top: 4,
            ..OutputGeometry::default()
        },
        notification_widget_geometry: OutputGeometry {
            content_scale: 112,
            crop_bottom: 6,
            ..OutputGeometry::default()
        },
        widget_width: 800.0,
        widget_height: 450.0,
        ..AppConfig::default()
    };
    store.save(&updated).unwrap();
    assert_eq!(store.load().unwrap(), updated);
}

#[test]
fn migrates_stretched_notification_default_to_compact_toast() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.insert(
        "notificationWidgetWidth".into(),
        serde_json::json!(LEGACY_NOTIFICATION_WIDGET_WIDTH),
    );
    object.insert(
        "notificationWidgetHeight".into(),
        serde_json::json!(LEGACY_NOTIFICATION_WIDGET_HEIGHT),
    );
    object.insert("notificationWidgetX".into(), serde_json::json!(940));
    object.insert("notificationWidgetY".into(), serde_json::json!(12));
    fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

    let migrated = ConfigStore::new(path.clone()).load().unwrap();
    assert_eq!(
        migrated.notification_widget_width,
        DEFAULT_NOTIFICATION_WIDGET_WIDTH
    );
    assert_eq!(
        migrated.notification_widget_height,
        DEFAULT_NOTIFICATION_WIDGET_HEIGHT
    );
    assert_eq!(migrated.notification_widget_x, Some(940));
    assert_eq!(migrated.notification_widget_y, Some(12));
    let persisted: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(
        persisted["notificationWidgetWidth"],
        DEFAULT_NOTIFICATION_WIDGET_WIDTH
    );
    assert_eq!(
        persisted["notificationWidgetHeight"],
        DEFAULT_NOTIFICATION_WIDGET_HEIGHT
    );
    assert_eq!(persisted["notificationWidgetX"], 940);
}

#[test]
fn migrates_previous_compact_notification_size() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut previous = serde_json::to_value(AppConfig::default()).unwrap();
    let object = previous.as_object_mut().unwrap();
    object.insert(
        "notificationWidgetWidth".into(),
        serde_json::json!(PREVIOUS_NOTIFICATION_WIDGET_WIDTH),
    );
    object.insert(
        "notificationWidgetHeight".into(),
        serde_json::json!(PREVIOUS_NOTIFICATION_WIDGET_HEIGHT),
    );
    object.insert("notificationWidgetX".into(), serde_json::json!(1440));
    fs::write(&path, serde_json::to_vec_pretty(&previous).unwrap()).unwrap();

    let migrated = ConfigStore::new(path).load().unwrap();
    assert_eq!(
        migrated.notification_widget_width,
        DEFAULT_NOTIFICATION_WIDGET_WIDTH
    );
    assert_eq!(
        migrated.notification_widget_height,
        DEFAULT_NOTIFICATION_WIDGET_HEIGHT
    );
    assert_eq!(migrated.notification_widget_x, Some(1440));
}

#[test]
fn keeps_custom_notification_widget_size() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut config = serde_json::to_value(AppConfig::default()).unwrap();
    let object = config.as_object_mut().unwrap();
    object.insert("notificationWidgetWidth".into(), serde_json::json!(720.0));
    object.insert("notificationWidgetHeight".into(), serde_json::json!(160.0));
    object.insert("notificationWidgetX".into(), serde_json::json!(940));
    fs::write(&path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();

    let loaded = ConfigStore::new(path).load().unwrap();
    assert_eq!(loaded.notification_widget_width, 720.0);
    assert_eq!(loaded.notification_widget_height, 160.0);
    assert_eq!(loaded.notification_widget_x, Some(940));
}

#[test]
fn migrates_legacy_config_without_overwriting_relay_config() {
    let root = tempfile::tempdir().unwrap();
    let legacy_directory = root.path().join(LEGACY_CONFIG_DIRECTORIES[0]);
    let relay_directory = root.path().join(APP_IDENTIFIER);
    let legacy = AppConfig {
        watched_channel_id: "123456789012345678".into(),
        ..AppConfig::default()
    };
    ConfigStore::new(legacy_directory.join("config.json"))
        .save(&legacy)
        .unwrap();
    ConfigStore::new(relay_directory.join("config.json"))
        .save(&AppConfig::default())
        .unwrap();

    migrate_legacy_config(&relay_directory).unwrap();
    assert_eq!(
        ConfigStore::new(relay_directory.join("config.json"))
            .load()
            .unwrap(),
        legacy
    );

    let relay = AppConfig {
        watched_channel_id: "323456789012345678".into(),
        former_message_channel_id: "423456789012345678".into(),
        ..AppConfig::default()
    };
    ConfigStore::new(relay_directory.join("config.json"))
        .save(&relay)
        .unwrap();
    migrate_legacy_config(&relay_directory).unwrap();
    assert_eq!(
        ConfigStore::new(relay_directory.join("config.json"))
            .load()
            .unwrap(),
        relay
    );
}

#[test]
fn legacy_output_geometry_keeps_its_layout_and_rejects_invalid_margins() {
    let geometry: OutputGeometry = serde_json::from_str(r#"{"contentScale":125}"#).unwrap();
    assert_eq!(geometry.anchor, OutputAnchor::Legacy);
    assert_eq!(geometry.margin_x, 12);
    assert!(geometry.validate().is_ok());
    assert!(
        OutputGeometry {
            margin_x: 201,
            ..geometry
        }
        .validate()
        .is_err()
    );
}

#[test]
fn only_the_youtube_video_can_shrink_below_half_size() {
    let small = OutputGeometry {
        content_scale: 20,
        ..OutputGeometry::default()
    };
    let config = AppConfig {
        music_video_obs_geometry: small,
        ..Default::default()
    };
    assert!(config.validate().is_ok());
    let too_small = OutputGeometry {
        content_scale: YOUTUBE_VIDEO_MIN_SCALE - 1,
        ..small
    };
    assert!(
        AppConfig {
            music_video_obs_geometry: too_small,
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    // Every other output keeps its 50% floor, the YouTube card included.
    assert!(
        AppConfig {
            music_obs_geometry: small,
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn music_cleanup_accepts_an_optional_welcome_message() {
    let mut config = AppConfig {
        music_cleanup_enabled: true,
        music_channel_id: "123456789012345678".into(),
        ..Default::default()
    };
    assert!(config.validate().is_ok());
    config.music_welcome_message_id = "invalid".into();
    assert!(config.validate().is_err());
    config.music_welcome_message_id = "223456789012345678".into();
    assert!(config.validate().is_ok());
    config.music_channel_id.clear();
    assert!(config.validate().is_err());
}

#[test]
fn legacy_speech_setting_is_ignored_on_load() {
    let config = AppConfig {
        former_message_channel_id: "123456789012345678".into(),
        former_message_cleanup_enabled: true,
        ..Default::default()
    };
    let mut value = serde_json::to_value(&config).unwrap();
    value["ttsSpeechEnabled"] = serde_json::json!(true);
    let (loaded, _) = deserialize_config(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        serde_json::to_value(&loaded)
            .unwrap()
            .get("ttsSpeechEnabled")
            .is_none()
    );
    // The only former channel becomes the Relay channel with its cleanup.
    assert_eq!(loaded.watched_channel_id, config.former_message_channel_id);
    assert!(loaded.media_cleanup_enabled);
    assert!(loaded.former_message_channel_id.is_empty());
}

const MEDIA_CHANNEL: &str = "123456789012345678";
const MESSAGE_CHANNEL: &str = "223456789012345678";
const MEDIA_WELCOME: &str = "323456789012345678";
const MESSAGE_WELCOME: &str = "423456789012345678";

/// Writes a pre-1.4.1 file as it was saved, without the current validation.
fn write_former_config(path: &Path, fields: serde_json::Value) {
    let mut value = serde_json::to_value(AppConfig::default()).unwrap();
    // A pre-1.4.1 file only has the former names of the renamed settings.
    for (old, new) in RENAMED_KEYS {
        let previous = value.as_object_mut().unwrap().remove(new).unwrap();
        value[old] = previous;
    }
    for (key, field) in fields.as_object().unwrap() {
        value[key] = field.clone();
    }
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

/// The first load rewrites the settings renamed in 1.4.1, and only those.
fn assert_saved_with_current_names(path: &Path) {
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    for (old, new) in RENAMED_KEYS {
        assert!(saved.get(old).is_none(), "{old} is renamed");
        assert!(saved.get(new).is_some(), "{new} is saved");
    }
}

/// Loads the file like three application restarts and checks it settles after the first.
fn load_three_times(path: &Path) -> AppConfig {
    let store = ConfigStore::new(path.to_path_buf());
    let first = store.load().unwrap();
    let saved = fs::read(path).unwrap();
    for _ in 0..2 {
        assert_eq!(store.load().unwrap(), first);
        assert_eq!(fs::read(path).unwrap(), saved);
    }
    first
}

#[test]
fn a_single_former_message_channel_becomes_the_relay_channel() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    write_former_config(
        &path,
        serde_json::json!({
            "ttsChannelId": MESSAGE_CHANNEL,
            "ttsCleanupEnabled": true,
            "ttsWelcomeMessageId": MESSAGE_WELCOME,
        }),
    );
    let config = load_three_times(&path);
    assert_eq!(config.watched_channel_id, MESSAGE_CHANNEL);
    assert!(config.media_cleanup_enabled);
    assert_eq!(config.media_welcome_message_id, MESSAGE_WELCOME);
    assert!(config.former_message_channel_id.is_empty());
    assert!(!config.former_message_cleanup_enabled);
    assert!(config.former_message_welcome_message_id.is_empty());
    assert!(!config.relay_channel_conflict());
}

#[test]
fn a_single_former_media_channel_stays_the_relay_channel() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    write_former_config(
        &path,
        serde_json::json!({
            "watchedChannelId": MEDIA_CHANNEL,
            "mediaCleanupEnabled": true,
            "mediaWelcomeMessageId": MEDIA_WELCOME,
        }),
    );
    let config = load_three_times(&path);
    assert_saved_with_current_names(&path);
    assert_eq!(config.watched_channel_id, MEDIA_CHANNEL);
    assert!(config.media_cleanup_enabled);
    assert_eq!(config.media_welcome_message_id, MEDIA_WELCOME);
}

#[test]
fn two_identical_former_channels_are_unified() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    write_former_config(
        &path,
        serde_json::json!({
            "watchedChannelId": MEDIA_CHANNEL,
            "ttsChannelId": MEDIA_CHANNEL,
            "ttsCleanupEnabled": true,
            "ttsWelcomeMessageId": MESSAGE_WELCOME,
        }),
    );
    // Rejected before 1.4.1, so such a file could only come from a manual edit.
    assert!(
        AppConfig {
            watched_channel_id: MEDIA_CHANNEL.into(),
            former_message_channel_id: MEDIA_CHANNEL.into(),
            ..AppConfig::default()
        }
        .validate()
        .is_err()
    );
    let config = load_three_times(&path);
    assert_eq!(config.watched_channel_id, MEDIA_CHANNEL);
    assert!(config.former_message_channel_id.is_empty());
    assert!(config.media_cleanup_enabled);
    assert_eq!(config.media_welcome_message_id, MESSAGE_WELCOME);
}

#[test]
fn two_different_former_channels_wait_for_an_explicit_choice() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    write_former_config(
        &path,
        serde_json::json!({
            "watchedChannelId": MEDIA_CHANNEL,
            "mediaWelcomeMessageId": MEDIA_WELCOME,
            "ttsChannelId": MESSAGE_CHANNEL,
            "ttsCleanupEnabled": true,
            "ttsWelcomeMessageId": MESSAGE_WELCOME,
        }),
    );
    let config = load_three_times(&path);
    assert_saved_with_current_names(&path);
    assert!(config.relay_channel_conflict());
    assert_eq!(config.watched_channel_id, MEDIA_CHANNEL);
    assert_eq!(config.former_message_channel_id, MESSAGE_CHANNEL);
    assert!(config.former_message_cleanup_enabled);
    // Until the choice, each channel keeps its former single role.
    assert_eq!(
        config.channel_feeds(MEDIA_CHANNEL),
        Some(ChannelFeeds {
            notifications: false,
            media: true
        })
    );
    assert_eq!(
        config.channel_feeds(MESSAGE_CHANNEL),
        Some(ChannelFeeds {
            notifications: true,
            media: false
        })
    );

    let mut refused = config.clone();
    assert!(refused.choose_relay_channel("523456789012345678").is_err());
    assert_eq!(refused, config);

    let mut keep_messages = config.clone();
    keep_messages.choose_relay_channel(MESSAGE_CHANNEL).unwrap();
    assert_eq!(keep_messages.watched_channel_id, MESSAGE_CHANNEL);
    assert!(keep_messages.media_cleanup_enabled);
    assert_eq!(keep_messages.media_welcome_message_id, MESSAGE_WELCOME);
    assert!(keep_messages.former_message_channel_id.is_empty());
    assert!(!keep_messages.former_message_cleanup_enabled);
    assert!(keep_messages.choose_relay_channel(MESSAGE_CHANNEL).is_err());

    let mut keep_media = config.clone();
    keep_media.choose_relay_channel(MEDIA_CHANNEL).unwrap();
    assert_eq!(keep_media.watched_channel_id, MEDIA_CHANNEL);
    assert!(!keep_media.media_cleanup_enabled);
    assert_eq!(keep_media.media_welcome_message_id, MEDIA_WELCOME);
    assert!(keep_media.former_message_channel_id.is_empty());

    // The choice is saved once and survives restarts.
    ConfigStore::new(path.clone()).save(&keep_messages).unwrap();
    let chosen = load_three_times(&path);
    assert_eq!(chosen, keep_messages);
    assert_eq!(
        chosen.channel_feeds(MESSAGE_CHANNEL),
        Some(ChannelFeeds {
            notifications: true,
            media: true
        })
    );
    assert_eq!(chosen.channel_feeds(MEDIA_CHANNEL), None);
}

#[test]
fn only_the_relay_channel_feeds_the_unified_flow() {
    let config = AppConfig {
        watched_channel_id: MEDIA_CHANNEL.into(),
        music_channel_id: MESSAGE_CHANNEL.into(),
        ..AppConfig::default()
    };
    assert_eq!(
        config.channel_feeds(MEDIA_CHANNEL),
        Some(ChannelFeeds {
            notifications: true,
            media: true
        })
    );
    assert_eq!(config.channel_feeds(MESSAGE_CHANNEL), None);
    assert_eq!(config.channel_feeds("523456789012345678"), None);
    assert_eq!(config.channel_feeds(""), None);
    assert_eq!(AppConfig::default().channel_feeds(""), None);
}

#[test]
fn legacy_directory_migration_never_brings_back_a_former_message_channel() {
    let root = tempfile::tempdir().unwrap();
    let legacy_directory = root.path().join(LEGACY_CONFIG_DIRECTORIES[0]);
    let relay_directory = root.path().join(APP_IDENTIFIER);
    fs::create_dir_all(&legacy_directory).unwrap();
    write_former_config(
        &legacy_directory.join("config.json"),
        serde_json::json!({
            "watchedChannelId": "523456789012345678",
            "ttsChannelId": MESSAGE_CHANNEL,
        }),
    );
    ConfigStore::new(relay_directory.join("config.json"))
        .save(&AppConfig {
            watched_channel_id: MEDIA_CHANNEL.into(),
            ..AppConfig::default()
        })
        .unwrap();
    // The migration runs on every launch; the Relay channel stays unified.
    for _ in 0..3 {
        migrate_legacy_config(&relay_directory).unwrap();
    }
    let config = load_three_times(&relay_directory.join("config.json"));
    assert_eq!(config.watched_channel_id, MEDIA_CHANNEL);
    assert!(config.former_message_channel_id.is_empty());
    assert!(!config.relay_channel_conflict());

    // A configuration without a Relay channel keeps both legacy channels for the choice.
    ConfigStore::new(relay_directory.join("config.json"))
        .save(&AppConfig::default())
        .unwrap();
    migrate_legacy_config(&relay_directory).unwrap();
    let config = load_three_times(&relay_directory.join("config.json"));
    assert_eq!(config.watched_channel_id, "523456789012345678");
    assert_eq!(config.former_message_channel_id, MESSAGE_CHANNEL);
    assert!(config.relay_channel_conflict());
    // Once chosen, later launches keep the choice.
    let mut chosen = config;
    chosen.choose_relay_channel(MESSAGE_CHANNEL).unwrap();
    ConfigStore::new(relay_directory.join("config.json"))
        .save(&chosen)
        .unwrap();
    migrate_legacy_config(&relay_directory).unwrap();
    assert_eq!(
        load_three_times(&relay_directory.join("config.json")),
        chosen
    );

    // A fresh install takes the legacy channels, unified when only one is set.
    write_former_config(
        &legacy_directory.join("config.json"),
        serde_json::json!({ "ttsChannelId": MESSAGE_CHANNEL }),
    );
    fs::remove_file(relay_directory.join("config.json")).unwrap();
    migrate_legacy_config(&relay_directory).unwrap();
    let config = load_three_times(&relay_directory.join("config.json"));
    assert_eq!(config.watched_channel_id, MESSAGE_CHANNEL);
    assert!(config.former_message_channel_id.is_empty());
}

#[test]
fn the_app_identifier_matches_the_tauri_configuration() {
    let tauri: serde_json::Value =
        serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    assert_eq!(tauri["identifier"], APP_IDENTIFIER);
    assert_ne!(APP_IDENTIFIER, PREVIOUS_APP_IDENTIFIER);
}

#[test]
fn previous_app_folders_are_copied_once_and_kept() {
    let root = tempfile::tempdir().unwrap();
    let previous = root.path().join(PREVIOUS_APP_IDENTIFIER);
    let current = root.path().join(APP_IDENTIFIER);
    fs::create_dir_all(previous.join("library").join("items")).unwrap();
    fs::write(previous.join("config.json"), b"{}").unwrap();
    fs::write(
        previous.join("library").join("items").join("a.bin"),
        b"media",
    )
    .unwrap();

    assert!(copy_previous_app_folder(&previous, &current).unwrap());
    assert_eq!(fs::read(current.join("config.json")).unwrap(), b"{}");
    assert_eq!(
        fs::read(current.join("library").join("items").join("a.bin")).unwrap(),
        b"media"
    );
    assert!(
        previous.join("config.json").is_file(),
        "the old folder stays"
    );

    // Later launches never overwrite the new folder.
    fs::write(current.join("config.json"), b"new").unwrap();
    assert!(!copy_previous_app_folder(&previous, &current).unwrap());
    assert_eq!(fs::read(current.join("config.json")).unwrap(), b"new");
    // Nothing to copy on a fresh install.
    assert!(
        !copy_previous_app_folder(&root.path().join("missing"), &root.path().join("other"))
            .unwrap()
    );
}

#[test]
fn an_interrupted_app_folder_copy_is_retried() {
    let root = tempfile::tempdir().unwrap();
    let previous = root.path().join(PREVIOUS_APP_IDENTIFIER);
    let current = root.path().join(APP_IDENTIFIER);
    fs::create_dir_all(&previous).unwrap();
    fs::write(previous.join("config.json"), b"{}").unwrap();
    let staging = root.path().join(format!("{APP_IDENTIFIER}.migrating"));
    fs::create_dir_all(&staging).unwrap();
    fs::write(staging.join("partial.json"), b"half").unwrap();

    assert!(copy_previous_app_folder(&previous, &current).unwrap());
    assert!(current.join("config.json").is_file());
    assert!(!current.join("partial.json").exists());
    assert!(!staging.exists());
}

#[test]
fn output_content_scale_reaches_four_hundred_percent() {
    let geometry = |content_scale| OutputGeometry {
        content_scale,
        ..OutputGeometry::default()
    };
    assert!(geometry(400).validate().is_ok());
    assert!(geometry(50).validate().is_ok());
    assert!(geometry(401).validate().is_err());
    assert!(geometry(49).validate().is_err());
}

#[test]
fn a_file_with_both_names_of_a_renamed_setting_still_loads() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut value = serde_json::to_value(AppConfig::default()).unwrap();
    value["notificationQueueLimit"] = serde_json::json!(7);
    value["ttsQueueLimit"] = serde_json::json!(40);
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    // The current name wins and the stray former key is dropped.
    let config = load_three_times(&path);
    assert_eq!(config.notification_queue_limit, 7);
    assert_saved_with_current_names(&path);
}
