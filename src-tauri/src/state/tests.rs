use super::*;

fn history_music_selection() -> MusicSelection {
    MusicSelection {
        owner_id: 42,
        owner_name: "Requester".into(),
        channel_id: 123,
        track: crate::youtube::YouTubeTrack {
            video_id: "dQw4w9WgXcQ".into(),
            title: "History track".into(),
            channel_title: "Artist".into(),
            thumbnail: String::new(),
            duration_seconds: 216,
        },
    }
}

#[tokio::test]
async fn youtube_history_replays_each_mode_without_duplicate_rows() {
    for mode in [
        MusicPlaybackMode::Preview,
        MusicPlaybackMode::Full,
        MusicPlaybackMode::Custom,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let core = AppCore::load(directory.path().join("config.json")).unwrap();
        let mut events = core.relay_tx.subscribe();
        let result = if mode == MusicPlaybackMode::Custom {
            core.start_music_custom(history_music_selection(), 50, 95, 1, "1")
                .await
                .unwrap()
        } else {
            core.start_music(history_music_selection(), mode, 1, "1")
                .await
        };
        let crate::music::MusicStartResult::Started(original) = result else {
            panic!("not started")
        };
        let RelayEvent::MusicHistory(entry) = events.recv().await.unwrap() else {
            panic!("missing history update")
        };
        assert_eq!(entry.music, original);
        let serialized = serde_json::to_value(core.history.read().await.clone()).unwrap();
        assert_eq!(serialized[0]["music"]["videoId"], original.video_id);
        assert!(serialized[0].get("selection").is_none());
        core.stop_current_music().await;
        core.replay_music_history(&original.playback_id)
            .await
            .unwrap();
        let replay = core.current_music().await.unwrap();
        assert_ne!(replay.playback_id, original.playback_id);
        assert_eq!(replay.video_id, original.video_id);
        assert_eq!(replay.mode, mode);
        assert_eq!(replay.start_seconds, original.start_seconds);
        assert_eq!(replay.end_seconds, original.end_seconds);
        assert_eq!(replay.requested_by, original.requested_by);
        assert_eq!(core.history.read().await.len(), 1);
        // One replay can wait; a second must respect duplicate protection.
        core.replay_music_history(&original.playback_id)
            .await
            .unwrap();
        assert!(
            core.replay_music_history(&original.playback_id)
                .await
                .is_err()
        );
        assert_eq!(core.music.lock().await.pending_events().len(), 1);
        assert_eq!(core.history.read().await.len(), 1);
    }
}

#[tokio::test]
async fn youtube_history_records_queued_tracks_but_not_rejected_requests() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let mut selection = history_music_selection();
    core.start_music(selection.clone(), MusicPlaybackMode::Full, 1, "1")
        .await;
    selection.track.video_id = "abcdefghijk".into();
    assert!(matches!(
        core.start_music(selection.clone(), MusicPlaybackMode::Preview, 2, "2")
            .await,
        crate::music::MusicStartResult::Queued { .. }
    ));
    assert!(matches!(
        core.start_music(selection, MusicPlaybackMode::Full, 3, "3")
            .await,
        crate::music::MusicStartResult::DuplicatePending { .. }
    ));
    assert_eq!(core.history.read().await.len(), 2);
}

#[tokio::test]
async fn youtube_and_media_share_the_history_limit() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let crate::music::MusicStartResult::Started(original) = core
        .start_music(history_music_selection(), MusicPlaybackMode::Full, 1, "1")
        .await
    else {
        panic!("not started")
    };
    for index in 0..HISTORY_LIMIT {
        core.publish_media(media(MediaKind::Image, &format!("image-{index}")))
            .await;
    }
    assert_eq!(core.history.read().await.len(), HISTORY_LIMIT);
    assert!(
        core.replay_music_history(&original.playback_id)
            .await
            .is_err()
    );
    assert!(
        core.history
            .read()
            .await
            .iter()
            .all(|entry| entry.media().is_some())
    );
}

#[tokio::test]
async fn app_core_restores_persisted_interface_preferences() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let config = AppConfig {
        interface_preferences: InterfacePreferences {
            language: "ko".into(),
            theme: "light".into(),
            accent_rgb: [42, 84, 126],
            font_scale: 120,
        },
        ..AppConfig::default()
    };
    ConfigStore::new(path.clone()).save(&config).unwrap();

    let core = AppCore::load(path).unwrap();

    assert_eq!(
        *core.interface_preferences.read().await,
        config.interface_preferences
    );
}

fn media(kind: MediaKind, message_id: &str) -> MediaEvent {
    MediaEvent {
        kind,
        url: "https://cdn.discordapp.com/media".into(),
        proxy_url: "https://media.discordapp.net/media".into(),
        filename: "media.bin".into(),
        content_type: "application/octet-stream".into(),
        artwork_id: None,
        audio_id: None,
        cached_media_id: None,
        title: None,
        artist: None,
        text: None,
        author: crate::model::AuthorIdentity {
            username: "Moderator".into(),
            display_avatar_url: "https://cdn.discordapp.com/avatar.png".into(),
        },
        timestamp: 42,
        message_id: message_id.into(),
    }
}

fn sticker(message_id: &str) -> StickerEvent {
    StickerEvent {
        id: format!("sticker-{message_id}"),
        name: "Test sticker".into(),
        format: "png".into(),
        url: "https://cdn.discordapp.com/stickers/test.png".into(),
        cached_media_id: None,
        author: crate::model::AuthorIdentity {
            username: "Moderator".into(),
            display_avatar_url: "https://cdn.discordapp.com/avatar.png".into(),
        },
        timestamp: 42,
        message_id: message_id.into(),
    }
}

#[tokio::test]
async fn moderates_allowed_media_and_rejects_disabled_types() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let mut events = core.relay_tx.subscribe();
    core.set_config(AppConfig {
        moderation_enabled: true,
        moderation_allow_videos: false,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let _ = events.recv().await.unwrap();

    core.submit_media(media(MediaKind::Image, "1")).await;
    core.submit_media(media(MediaKind::Video, "2")).await;
    assert_eq!(core.pending_media.read().await.len(), 1);
    assert!(events.try_recv().is_err());

    let id = core.pending_media.read().await[0].id;
    let (delivery, mut requests) = mpsc::unbounded_channel();
    core.set_media_delivery(delivery).await;
    let ready = tokio::spawn(async move {
        let request = requests.recv().await.unwrap();
        assert!(matches!(request.kind, MediaKind::Image));
        request.ready.send(()).unwrap();
    });
    assert!(core.approve_media(id).await);
    ready.await.unwrap();
    assert!(matches!(events.recv().await.unwrap(), RelayEvent::Media(_)));
    assert_eq!(core.history.read().await.len(), 1);
    assert!(!core.reject_media(id).await);
}

#[tokio::test]
async fn sensitive_media_never_reaches_history_or_relay() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut events = core.relay_tx.subscribe();
    core.submit_analyzed_media(
        media(MediaKind::Image, "sensitive"),
        Some(PrivacyReport::sensitive("gps")),
    )
    .await;
    assert!(core.history.read().await.is_empty());
    assert!(core.pending_media.read().await.is_empty());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn medium_privacy_risk_uses_existing_queue_before_publication() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        privacy_review_intermediate: true,
        moderation_enabled: false,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut events = core.relay_tx.subscribe();

    core.submit_analyzed_media_with_text(
        media(MediaKind::Image, "medium-phone"),
        None,
        Some("Call 06 12 34 56 78"),
    )
    .await;

    let pending = core.pending_media.read().await;
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].privacy_classification,
        Some(privacy::PrivacyClassification::Medium)
    );
    assert!(
        pending[0]
            .privacy_categories
            .contains(&privacy::PrivacyCategory::Phone)
    );
    assert!(core.history.read().await.is_empty());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn custom_private_value_is_blocked_before_history_and_relay() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        privacy_custom_patterns: vec!["private-room-42".into()],
        moderation_enabled: false,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut events = core.relay_tx.subscribe();

    core.submit_analyzed_media_with_text(
        media(MediaKind::Image, "custom-pattern"),
        None,
        Some("private room 42"),
    )
    .await;

    assert!(core.pending_media.read().await.is_empty());
    assert!(core.history.read().await.is_empty());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn filter_words_block_without_scan_or_manual_moderation() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        moderation_enabled: false,
        privacy_scan_enabled: false,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "hitler".into(),
            aliases: Vec::new(),
            regexes: vec![r"\bsecret\b".into()],
        }],
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut events = core.relay_tx.subscribe();

    core.submit_analyzed_media_with_text(
        media(MediaKind::Image, "filter-exact"),
        Some(PrivacyReport::safe()),
        Some("hitler"),
    )
    .await;
    core.submit_analyzed_media_with_text(
        media(MediaKind::Image, "filter-regex"),
        Some(PrivacyReport::safe()),
        Some("a SECRET message"),
    )
    .await;
    assert!(core.history.read().await.is_empty());
    assert!(core.pending_media.read().await.is_empty());
    assert!(events.try_recv().is_err());

    core.submit_analyzed_media_with_text(
        media(MediaKind::Image, "filter-safe"),
        None,
        Some("public monument"),
    )
    .await;
    assert_eq!(core.history.read().await.len(), 1);
    assert!(matches!(events.recv().await.unwrap(), RelayEvent::Media(_)));
}

#[tokio::test]
async fn exempt_role_publishes_filter_word_media_and_tts_but_not_gps() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let role_id = "123456789012345678".to_owned();
    core.set_config(AppConfig {
        moderation_enabled: false,
        privacy_scan_enabled: false,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "hitler".into(),
            aliases: Vec::new(),
            regexes: vec![r"\bsecret\b".into()],
        }],
        privacy_filter_exempt_role_ids: vec![role_id.clone()],
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut events = core.relay_tx.subscribe();

    core.submit_analyzed_media_with_text_and_roles(
        media(MediaKind::Image, "exempt-media"),
        Some(PrivacyReport::sensitive("forbidden_concept")),
        Some("hitler"),
        std::slice::from_ref(&role_id),
    )
    .await;
    assert_eq!(core.history.read().await.len(), 1);
    assert!(matches!(events.recv().await.unwrap(), RelayEvent::Media(_)));

    core.submit_analyzed_media_with_text_and_roles(
        media(MediaKind::Image, "exempt-regex"),
        Some(PrivacyReport::sensitive("forbidden_regex")),
        Some("secret"),
        std::slice::from_ref(&role_id),
    )
    .await;
    assert_eq!(core.history.read().await.len(), 2);
    assert!(matches!(events.recv().await.unwrap(), RelayEvent::Media(_)));

    assert!(
        core.publish_visual_tts_if_allowed_with_roles(
            "exempt-tts".into(),
            "hitler".into(),
            crate::model::AuthorIdentity {
                username: "Moderator".into(),
                display_avatar_url: String::new(),
            },
            None,
            42,
            vec![VisualSegment {
                kind: "text".into(),
                value: "hitler".into(),
                url: None,
                animated: false,
            }],
            std::slice::from_ref(&role_id),
        )
        .await
    );
    assert!(matches!(events.recv().await.unwrap(), RelayEvent::Tts(_)));

    core.update_config(|config| config.privacy_scan_enabled = true)
        .await
        .unwrap();
    assert!(matches!(
        events.recv().await.unwrap(),
        RelayEvent::Config(_)
    ));
    core.submit_analyzed_media_with_text_and_roles(
        media(MediaKind::Image, "exempt-gps"),
        Some(PrivacyReport::sensitive("gps")),
        None,
        std::slice::from_ref(&role_id),
    )
    .await;
    assert_eq!(core.history.read().await.len(), 2);
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn pending_role_scope_is_kept_per_moderation_item() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let role_id = "123456789012345678".to_owned();
    core.set_config(AppConfig {
        moderation_enabled: true,
        privacy_scan_enabled: false,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "hitler".into(),
            aliases: Vec::new(),
            regexes: Vec::new(),
        }],
        privacy_filter_exempt_role_ids: vec![role_id.clone()],
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let roles = std::slice::from_ref(&role_id);
    core.submit_analyzed_media_with_text_and_roles(
        media(MediaKind::Image, "shared-message"),
        None,
        Some("hitler"),
        roles,
    )
    .await;
    core.submit_analyzed_media_with_text_and_roles(
        media(MediaKind::Image, "shared-message"),
        None,
        Some("hitler"),
        roles,
    )
    .await;
    let ids = core
        .pending_media
        .read()
        .await
        .iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    assert!(core.approve_media(ids[0]).await);
    assert!(core.approve_media(ids[1]).await);
    assert_eq!(core.history.read().await.len(), 2);
}

#[tokio::test]
async fn state_gate_rechecks_text_before_publication() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "hitler".into(),
            aliases: Vec::new(),
            regexes: Vec::new(),
        }],
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut media = media(MediaKind::Image, "concept");
    media.text = Some("h1tl3r".into());
    let mut events = core.relay_tx.subscribe();
    core.submit_analyzed_media(media, Some(PrivacyReport::safe()))
        .await;
    assert!(core.history.read().await.is_empty());
    assert!(core.pending_media.read().await.is_empty());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn state_gate_uses_full_discord_text_beyond_caption_limit() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "hitler".into(),
            aliases: Vec::new(),
            regexes: Vec::new(),
        }],
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut event = media(MediaKind::Image, "concept-after-caption");
    event.text = Some("a".repeat(180));
    let full_text = format!("{} h1tl3r", "a".repeat(180));
    let mut events = core.relay_tx.subscribe();
    core.submit_analyzed_media_with_text(event, Some(PrivacyReport::safe()), Some(&full_text))
        .await;
    assert!(core.history.read().await.is_empty());
    assert!(core.pending_media.read().await.is_empty());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn state_gate_scans_untrusted_media_fields_for_concepts() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "hitler".into(),
            aliases: Vec::new(),
            regexes: Vec::new(),
        }],
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut filename_media = media(MediaKind::Image, "filename-concept");
    filename_media.filename = "h1tl3r.png".into();
    let mut title_media = media(MediaKind::Image, "title-concept");
    title_media.title = Some("h1tl3r".into());
    core.submit_analyzed_media(filename_media, Some(PrivacyReport::safe()))
        .await;
    core.submit_analyzed_media(title_media, Some(PrivacyReport::safe()))
        .await;
    core.submit_sticker(
        StickerEvent {
            name: "h1tl3r".into(),
            ..sticker("sticker-concept")
        },
        None,
        None,
        Some(PrivacyReport::safe()),
    )
    .await;
    assert!(core.history.read().await.is_empty());
    assert!(core.pending_media.read().await.is_empty());
}

#[tokio::test]
async fn approval_rechecks_sensitive_pending_entries() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut pending_media = media(MediaKind::Image, "pending-sensitive");
    pending_media.content_type = "image/png".into();
    core.pending_media.write().await.push_back(PendingMedia {
        id: 99,
        media: pending_media,
        sticker: None,
        sticker_bytes: Some(Arc::new(b"not an image".to_vec())),
        privacy_classification: Some(privacy::PrivacyClassification::Sensitive),
        privacy_categories: vec![privacy::PrivacyCategory::GpsLocation],
        privacy_reason: Some("gps".into()),
    });
    let mut events = core.relay_tx.subscribe();
    assert!(!core.approve_media(99).await);
    assert!(core.history.read().await.is_empty());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn approval_rechecks_cached_audio_artwork_before_publication() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    core.cache_artwork(
        "audio-artwork".into(),
        EmbeddedArtwork {
            content_type: "image/png".into(),
            bytes: b"not an image".to_vec(),
        },
    )
    .await;
    let mut event = media(MediaKind::Audio, "pending-audio-artwork");
    event.artwork_id = Some("audio-artwork".into());
    core.pending_media.write().await.push_back(PendingMedia {
        id: 100,
        media: event,
        sticker: None,
        sticker_bytes: None,
        privacy_classification: Some(privacy::PrivacyClassification::Medium),
        privacy_categories: vec![privacy::PrivacyCategory::Ocr],
        privacy_reason: Some("ocr_text".into()),
    });
    let mut events = core.relay_tx.subscribe();

    assert!(!core.approve_media(100).await);
    assert!(core.history.read().await.is_empty());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn suspicious_media_uses_review_queue_even_when_manual_moderation_is_off() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        moderation_enabled: false,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut events = core.relay_tx.subscribe();
    core.submit_analyzed_media(
        media(MediaKind::Image, "suspicious"),
        Some(PrivacyReport::suspicious("privacy_signal")),
    )
    .await;
    assert_eq!(core.pending_media.read().await.len(), 1);
    assert!(core.history.read().await.is_empty());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn sensitive_sticker_never_reaches_cache_or_relay() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut events = core.relay_tx.subscribe();
    core.submit_sticker(
        sticker("sensitive"),
        Some("h1tl3r"),
        Some(vec![1, 2, 3]),
        Some(PrivacyReport::sensitive("forbidden_concept")),
    )
    .await;
    assert!(core.pending_media.read().await.is_empty());
    assert!(core.cached_media.read().await.is_empty());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn sticker_scan_review_uses_the_existing_queue() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        moderation_enabled: false,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut events = core.relay_tx.subscribe();
    core.submit_sticker(
        sticker("review"),
        Some("safe caption"),
        Some(vec![1, 2, 3]),
        Some(PrivacyReport::suspicious("scan_incomplete")),
    )
    .await;
    let pending = core.pending_media.read().await;
    assert_eq!(pending.len(), 1);
    assert!(pending[0].sticker.is_some());
    assert!(pending[0].sticker_bytes.is_some());
    assert!(core.cached_media.read().await.is_empty());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn privacy_review_pending_survives_manual_moderation_disable() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        moderation_enabled: true,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    core.submit_analyzed_media(
        media(MediaKind::Image, "privacy-review"),
        Some(PrivacyReport::suspicious("scan_incomplete")),
    )
    .await;
    assert_eq!(core.pending_media.read().await.len(), 1);
    core.update_config(|config| config.moderation_enabled = false)
        .await
        .unwrap();
    assert_eq!(core.pending_media.read().await.len(), 1);
}

#[tokio::test]
async fn replay_blocks_sensitive_media_when_filter_words_are_enabled() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: false,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "hitler".into(),
            aliases: Vec::new(),
            regexes: Vec::new(),
        }],
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut event = media(MediaKind::Video, "replay-sensitive");
    event.text = Some("h1tl3r".into());
    let mut events = core.relay_tx.subscribe();
    assert!(core.replay_media_event(event).await.is_err());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn replay_keeps_the_privacy_off_bypass() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let mut event = media(MediaKind::Video, "replay-disabled");
    event.text = Some("h1tl3r".into());
    let mut events = core.relay_tx.subscribe();
    core.replay_media_event(event).await.unwrap();
    assert!(matches!(events.recv().await.unwrap(), RelayEvent::Media(_)));
}

#[tokio::test]
async fn stale_safe_report_enters_the_current_privacy_policy() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: true,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let old_config = core.config.read().await.clone();
    let report = privacy::classify_text(Some("landscape"), &old_config);
    core.update_config(|config| config.privacy_similarity_boost = 3)
        .await
        .unwrap();
    core.submit_analyzed_media(media(MediaKind::Image, "stale-safe"), Some(report))
        .await;
    let pending = core.pending_media.read().await;
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].privacy_reason.as_deref(),
        Some("scan_config_changed")
    );
}

#[tokio::test]
async fn disabled_privacy_scan_preserves_existing_immediate_flow() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: false,
        privacy_concepts: Vec::new(),
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut events = core.relay_tx.subscribe();
    core.submit_analyzed_media(
        media(MediaKind::Image, "disabled"),
        Some(PrivacyReport::sensitive("forbidden_concept")),
    )
    .await;
    assert_eq!(core.history.read().await.len(), 1);
    assert!(matches!(events.recv().await.unwrap(), RelayEvent::Media(_)));
}

#[tokio::test]
async fn broadcasts_visual_tts_without_touching_the_audio_cache() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let mut events = core.relay_tx.subscribe();

    core.publish_visual_tts(
        "123456789012345678".into(),
        "test".into(),
        crate::model::AuthorIdentity {
            username: "Silent tester".into(),
            display_avatar_url: "https://cdn.discordapp.com/avatar.png".into(),
        },
        Some(crate::model::GuildTagIdentity {
            name: "RE".into(),
            badge_url: None,
        }),
        42,
        vec![VisualSegment {
            kind: "text".into(),
            value: "test".into(),
            url: None,
            animated: false,
        }],
    )
    .await;

    let RelayEvent::Tts(event) = events.recv().await.unwrap() else {
        panic!("expected a TTS relay event");
    };
    assert!(event.visual_only);
    assert_eq!(event.text, "test");
    assert_eq!(event.guild_tag.unwrap().name, "RE");
    assert_eq!(event.segments.len(), 1);
    assert!(core.tts_audio.read().await.is_empty());
}

#[tokio::test]
async fn tts_privacy_gate_blocks_filter_concepts_and_holds_medium_risk() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        privacy_scan_enabled: false,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "hitler".into(),
            aliases: Vec::new(),
            regexes: Vec::new(),
        }],
        ..AppConfig::default()
    })
    .await
    .unwrap();
    let mut events = core.relay_tx.subscribe();
    let author = crate::model::AuthorIdentity {
        username: "TTS tester".into(),
        display_avatar_url: "https://cdn.discordapp.com/avatar.png".into(),
    };
    let blocked_concept = core
        .publish_visual_tts_if_allowed(
            "tts-concept".into(),
            "sticker h1tl3r".into(),
            author.clone(),
            None,
            42,
            Vec::new(),
        )
        .await;
    assert!(!blocked_concept);
    let blocked_segment_concept = core
        .publish_visual_tts_if_allowed(
            "tts-segment-concept".into(),
            "safe text".into(),
            author.clone(),
            None,
            42,
            vec![VisualSegment {
                kind: "text".into(),
                value: "h1tl3r".into(),
                url: None,
                animated: false,
            }],
        )
        .await;
    assert!(!blocked_segment_concept);
    assert!(events.try_recv().is_err());
    let blocked_cross_field_concept = core
        .publish_visual_tts_if_allowed(
            "tts-cross-field-concept".into(),
            "hi".into(),
            author.clone(),
            None,
            42,
            vec![VisualSegment {
                kind: "sticker".into(),
                value: "tler".into(),
                url: None,
                animated: false,
            }],
        )
        .await;
    assert!(!blocked_cross_field_concept);
    assert!(events.try_recv().is_err());
    let blocked_cross_segment_concept = core
        .publish_visual_tts_if_allowed(
            "tts-cross-segment-concept".into(),
            "safe".into(),
            author.clone(),
            None,
            42,
            vec![
                VisualSegment {
                    kind: "text".into(),
                    value: "hi".into(),
                    url: None,
                    animated: false,
                },
                VisualSegment {
                    kind: "text".into(),
                    value: "tler".into(),
                    url: None,
                    animated: false,
                },
            ],
        )
        .await;
    assert!(!blocked_cross_segment_concept);
    assert!(events.try_recv().is_err());
    let unrelated_split = core
        .publish_visual_tts_if_allowed(
            "tts-unrelated-split".into(),
            "safe".into(),
            author.clone(),
            None,
            42,
            vec![VisualSegment {
                kind: "text".into(),
                value: "hello world".into(),
                url: None,
                animated: false,
            }],
        )
        .await;
    assert!(unrelated_split);
    assert!(matches!(events.try_recv(), Ok(RelayEvent::Tts(_))));
    core.update_config(|config| {
        config.privacy_scan_enabled = true;
        config.privacy_review_intermediate = true;
    })
    .await
    .unwrap();
    let held_for_review = core
        .publish_visual_tts_if_allowed(
            "tts-medium".into(),
            "call 06 12 34 56 78".into(),
            author.clone(),
            None,
            42,
            Vec::new(),
        )
        .await;
    assert!(!held_for_review);

    core.update_config(|config| config.privacy_review_intermediate = false)
        .await
        .unwrap();
    let allowed_medium = core
        .publish_visual_tts_if_allowed(
            "tts-medium-allowed".into(),
            "call 06 12 34 56 78".into(),
            author,
            None,
            42,
            Vec::new(),
        )
        .await;
    assert!(allowed_medium);
}

#[tokio::test]
async fn claims_delayed_embeds_only_once() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    assert!(core.claim_embed("message-embed-0".into()).await);
    assert!(!core.claim_embed("message-embed-0".into()).await);
}

#[tokio::test]
async fn explicit_stage_tickets_keep_delayed_text_ahead_of_newer_ready_media() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let mut events = core.relay_tx.subscribe();

    let video_ticket = core
        .register_stage_output(21_590, "100", 200, StageLane::Media)
        .await;
    let mut video = media(MediaKind::Video, "video");
    video.timestamp = 21_590;
    core.complete_media(video_ticket, video).await;
    assert!(
        matches!(events.recv().await.unwrap(), RelayEvent::Media(event) if event.message_id == "video")
    );
    core.stage_scheduler.stage_state(true, false, false).await;

    let text_ticket = core
        .register_stage_output(22_000, "101", 0, StageLane::Tts)
        .await;
    let image_ticket = core
        .register_stage_output(22_010, "102", 200, StageLane::Media)
        .await;
    let mut image = media(MediaKind::Image, "image");
    image.timestamp = 22_010;
    core.complete_media(image_ticket, image).await;
    core.complete_tts(
        text_ticket,
        TtsEvent {
            id: "text".into(),
            text: "older text".into(),
            author: crate::model::AuthorIdentity {
                username: "tester".into(),
                display_avatar_url: String::new(),
            },
            guild_tag: None,
            content_type: String::new(),
            timestamp: 22_000,
            visual_only: true,
            segments: Vec::new(),
        },
    )
    .await;

    core.stage_scheduler.stage_state(false, false, false).await;
    assert!(matches!(events.recv().await.unwrap(), RelayEvent::Tts(event) if event.id == "text"));
    core.stage_scheduler.stage_state(false, false, true).await;
    core.stage_scheduler.stage_state(false, false, false).await;
    assert!(
        matches!(events.recv().await.unwrap(), RelayEvent::Media(event) if event.message_id == "image")
    );
}
