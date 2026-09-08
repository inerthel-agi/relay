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
        tts_channel_id: "223456789012345678".into(),
        media_cleanup_enabled: true,
        media_welcome_message_id: "623456789012345678".into(),
        tts_cleanup_enabled: true,
        tts_welcome_message_id: String::new(),
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
        tts_character_limit: 280,
        tts_queue_limit: 24,
        tts_speech_enabled: false,
        tts_notifications_obs_enabled: true,
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
        music_widget_x: Some(880),
        music_widget_y: Some(20),
        music_widget_width: 540.0,
        music_widget_height: 360.0,
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
        },
        ..AppConfig::default()
    };

    store.save(&config).unwrap();
    let loaded = ConfigStore::new(path).load().unwrap();

    assert_eq!(loaded.interface_preferences, config.interface_preferences);
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
        tts_channel_id: "123456789012345678".into(),
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
            content_scale: 201,
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
        music_widget_width: 720.0,
        music_widget_height: 280.0,
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
fn migrates_missing_music_widget_size_to_compact_default() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.insert("notificationWidgetWidth".into(), serde_json::json!(540.0));
    object.insert("notificationWidgetHeight".into(), serde_json::json!(160.0));
    object.remove("musicWidgetWidth");
    object.remove("musicWidgetHeight");
    fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

    let migrated = ConfigStore::new(path.clone()).load().unwrap();
    assert_eq!(migrated.notification_widget_width, 540.0);
    assert_eq!(migrated.notification_widget_height, 160.0);
    assert_eq!(migrated.music_widget_width, DEFAULT_MUSIC_WIDGET_WIDTH);
    assert_eq!(migrated.music_widget_height, DEFAULT_MUSIC_WIDGET_HEIGHT);
    let persisted: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(persisted["musicWidgetWidth"], DEFAULT_MUSIC_WIDGET_WIDTH);
    assert_eq!(persisted["musicWidgetHeight"], DEFAULT_MUSIC_WIDGET_HEIGHT);
    assert_eq!(persisted["notificationWidgetWidth"], 540.0);
    assert_eq!(persisted["notificationWidgetHeight"], 160.0);
}

#[test]
fn migrates_video_first_music_default_to_compact_toast() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.insert(
        "musicWidgetWidth".into(),
        serde_json::json!(LEGACY_MUSIC_WIDGET_WIDTH),
    );
    object.insert(
        "musicWidgetHeight".into(),
        serde_json::json!(LEGACY_MUSIC_WIDGET_HEIGHT),
    );
    fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

    let migrated = ConfigStore::new(path.clone()).load().unwrap();
    assert_eq!(migrated.music_widget_width, DEFAULT_MUSIC_WIDGET_WIDTH);
    assert_eq!(migrated.music_widget_height, DEFAULT_MUSIC_WIDGET_HEIGHT);
    let persisted: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(persisted["musicWidgetWidth"], DEFAULT_MUSIC_WIDGET_WIDTH);
    assert_eq!(persisted["musicWidgetHeight"], DEFAULT_MUSIC_WIDGET_HEIGHT);
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
fn keeps_custom_music_widget_size() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut config = serde_json::to_value(AppConfig::default()).unwrap();
    let object = config.as_object_mut().unwrap();
    object.insert("musicWidgetWidth".into(), serde_json::json!(720.0));
    object.insert("musicWidgetHeight".into(), serde_json::json!(280.0));
    fs::write(&path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();

    let loaded = ConfigStore::new(path).load().unwrap();
    assert_eq!(loaded.music_widget_width, 720.0);
    assert_eq!(loaded.music_widget_height, 280.0);
}

#[test]
fn migrates_missing_music_widget_position_from_notification_position() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.insert("notificationWidgetX".into(), serde_json::json!(2020));
    object.insert("notificationWidgetY".into(), serde_json::json!(48));
    object.remove("musicWidgetX");
    object.remove("musicWidgetY");
    fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

    let migrated = ConfigStore::new(path.clone()).load().unwrap();
    assert_eq!(migrated.notification_widget_x, Some(2020));
    assert_eq!(migrated.notification_widget_y, Some(48));
    assert_eq!(migrated.music_widget_x, Some(2020));
    assert_eq!(migrated.music_widget_y, Some(48));
    let persisted: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(persisted["musicWidgetX"], 2020);
    assert_eq!(persisted["musicWidgetY"], 48);
    assert_eq!(persisted["notificationWidgetX"], 2020);
    assert_eq!(persisted["notificationWidgetY"], 48);
}

#[test]
fn keeps_existing_music_widget_position_during_load() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut config = serde_json::to_value(AppConfig::default()).unwrap();
    let object = config.as_object_mut().unwrap();
    object.insert("notificationWidgetX".into(), serde_json::json!(100));
    object.insert("notificationWidgetY".into(), serde_json::json!(20));
    object.insert("musicWidgetX".into(), serde_json::json!(880));
    object.insert("musicWidgetY".into(), serde_json::json!(40));
    fs::write(&path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();

    let loaded = ConfigStore::new(path).load().unwrap();
    assert_eq!(loaded.notification_widget_x, Some(100));
    assert_eq!(loaded.notification_widget_y, Some(20));
    assert_eq!(loaded.music_widget_x, Some(880));
    assert_eq!(loaded.music_widget_y, Some(40));
}

#[test]
fn migrates_legacy_config_without_overwriting_relay_config() {
    let root = tempfile::tempdir().unwrap();
    let legacy_directory = root.path().join(LEGACY_CONFIG_DIRECTORIES[0]);
    let relay_directory = root.path().join("eu.stealthylabs.relay");
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
        tts_channel_id: "423456789012345678".into(),
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
fn speech_enabled_installations_migrate_to_visual_messages() {
    let config = AppConfig {
        tts_speech_enabled: true,
        tts_channel_id: "123456789012345678".into(),
        tts_cleanup_enabled: true,
        ..Default::default()
    };
    let (loaded, migrated) = deserialize_config(&serde_json::to_vec(&config).unwrap()).unwrap();
    assert!(migrated);
    assert!(!loaded.tts_speech_enabled);
    assert_eq!(loaded.tts_channel_id, config.tts_channel_id);
    assert!(loaded.tts_cleanup_enabled);
}
