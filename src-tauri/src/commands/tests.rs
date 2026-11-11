use super::*;

#[test]
fn youtube_obs_url_uses_localhost_referrer_host() {
    assert_eq!(
        youtube_overlay_url(4590),
        format!("http://{}/youtube", "localhost:4590")
    );
    assert!(!youtube_overlay_url(4590).contains("127.0.0.1"));
}

#[test]
fn obs_visual_url_uses_localhost_and_composite_path() {
    assert_eq!(
        widget::obs_visual_url(4590),
        "http://localhost:4590/obs/visual"
    );
    assert!(!widget::obs_visual_url(4590).contains("127.0.0.1"));
}

#[test]
fn creates_an_audible_test_tone_wav() {
    let wav = test_tone_wav();
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 19_200);
    assert!(
        wav[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .any(|sample| { i16::from_le_bytes(*sample) != 0 })
    );
}

#[tokio::test]
async fn output_tests_bypass_history_and_cache_their_audio() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let mut events = core.relay_tx.subscribe();

    emit_output_test(&core, OutputTestTarget::Visual)
        .await
        .unwrap();
    let RelayEvent::TestOutput(visual) = events.recv().await.unwrap() else {
        panic!("expected visual output test");
    };
    assert_eq!(visual.target, OutputTestTarget::Visual);
    assert!(visual.media.is_some());
    assert!(core.history.read().await.is_empty());

    emit_output_test(&core, OutputTestTarget::Audio)
        .await
        .unwrap();
    let RelayEvent::TestOutput(audio) = events.recv().await.unwrap() else {
        panic!("expected audio output test");
    };
    assert_eq!(audio.target, OutputTestTarget::Audio);
    assert_eq!(
        audio.media.and_then(|media| media.audio_id).as_deref(),
        Some("999999999999999998")
    );
    assert_eq!(core.media_audio.read().await.len(), 1);

    emit_output_test(&core, OutputTestTarget::Notification)
        .await
        .unwrap();
    let RelayEvent::TestOutput(notification) = events.recv().await.unwrap() else {
        panic!("expected notification output test");
    };
    assert_eq!(notification.target, OutputTestTarget::Notification);
    assert_eq!(
        notification
            .notification
            .as_ref()
            .map(|event| event.id.as_str()),
        Some("relay-test-notification")
    );
    assert!(core.history.read().await.is_empty());
}

#[test]
fn download_filenames_are_safe_and_keep_media_extensions() {
    assert_eq!(
        safe_media_filename("C:\\private\\clip:01.mp4", MediaKind::Video, "video/mp4"),
        "clip_01.mp4"
    );
    assert_eq!(
        safe_media_filename("", MediaKind::Audio, "audio/flac"),
        "relay-media.flac"
    );
    assert_eq!(
        safe_media_filename("Discord GIF.jpg", MediaKind::Gif, "image/gif"),
        "Discord GIF.gif"
    );
    assert_eq!(
        safe_media_filename("Discord GIF.mp4", MediaKind::Gif, "video/mp4"),
        "Discord GIF.mp4"
    );
    assert_eq!(
        media_download_filter(MediaKind::Gif, "image/gif"),
        ("GIF", &["gif"] as &[&str])
    );
    assert_eq!(
        media_download_filter(MediaKind::Gif, "video/mp4"),
        ("Video", &["mp4", "webm"] as &[&str])
    );
    assert_eq!(
        media_download_filter(MediaKind::Image, "image/jpeg"),
        (
            "Image",
            &["png", "jpg", "jpeg", "gif", "webp", "apng"] as &[&str]
        )
    );
}

#[test]
fn notification_sound_plays_relay_sounds_or_the_kept_custom_copy() {
    let mut config = AppConfig::default();
    let (soft, soft_type) = notification_sound_bytes(&config).unwrap();
    assert_eq!(soft_type, "audio/wav");
    assert!(soft.starts_with(b"RIFF"));
    config.notification_sound_choice = Some(NotificationSound::Chat);
    let (chat, _) = notification_sound_bytes(&config).unwrap();
    assert!(chat.starts_with(b"RIFF"));
    assert_ne!(chat, soft);
    // Custom without a file plays nothing rather than a Relay sound.
    config.notification_sound_choice = Some(NotificationSound::Custom);
    assert!(notification_sound_bytes(&config).is_err());

    let directory = tempfile::tempdir().unwrap();
    let original = directory.path().join("my ping.mp3");
    std::fs::write(&original, b"first").unwrap();
    let store = directory
        .path()
        .join("relay")
        .join(NOTIFICATION_SOUND_FOLDER);
    let stored = store_custom_sound(&store, &original).unwrap();
    assert_eq!(stored, store.join("my ping.mp3"));
    // The original can go away: Relay plays its own copy.
    std::fs::remove_file(&original).unwrap();
    config.notification_sound_path = Some(stored.to_string_lossy().into_owned());
    assert_eq!(
        notification_sound_bytes(&config).unwrap(),
        (b"first".to_vec(), "audio/mpeg")
    );

    // A new file replaces the previous copy; choosing the copy itself keeps it.
    let second = directory.path().join("other.ogg");
    std::fs::write(&second, b"second").unwrap();
    let stored = store_custom_sound(&store, &second).unwrap();
    assert_eq!(store_custom_sound(&store, &stored).unwrap(), stored);
    let kept: Vec<_> = std::fs::read_dir(&store)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(kept, ["other.ogg"]);
    assert_eq!(std::fs::read(&stored).unwrap(), b"second");
}

#[tokio::test]
async fn the_runtime_status_counts_held_texts_for_the_tray() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.pending_texts
        .write()
        .await
        .push_back(crate::model::PendingText {
            id: 1,
            message_id: "1".into(),
            text: "held".into(),
            author: crate::model::AuthorIdentity {
                username: "Member".into(),
                display_avatar_url: String::new(),
            },
            guild_tag: None,
            segments: vec![],
            timestamp: 0,
            reason: None,
            author_id: None,
            channel_id: None,
            queued_at: 0,
            ai_advice: None,
            trusted: false,
        });
    let hidden = WidgetState {
        visible: false,
        locked: false,
    };
    let status = RuntimeStatus {
        bot: BotStatus::default(),
        server: ServerStatus::default(),
        channels: vec![],
        widget: hidden,
        notification_widget: NotificationWidgetState {
            visible: false,
            locked: false,
        },
        pending_media: vec![],
        pending_text_count: pending_text_count(&core).await,
        queue: vec![],
        outputs_paused: false,
    };
    let json = serde_json::to_value(status).unwrap();
    // The count only: the text itself never reaches the tray.
    assert_eq!(json["pendingTextCount"], 1);
    assert!(!json.to_string().contains("held"));
}
