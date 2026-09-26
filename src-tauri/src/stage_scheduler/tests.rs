use super::*;
use crate::model::{AuthorIdentity, MediaEvent, MediaKind, StickerEvent};

#[tokio::test]
async fn pinned_messages_wait_in_a_bounded_lane_without_delaying_media() {
    let (tx, mut rx) = broadcast::channel(16);
    let scheduler = StageScheduler::new(tx);
    scheduler.set_messages_pinned(true, 2).await;
    for id in 1..=3 {
        scheduler
            .enqueue(
                RelayEvent::Tts(TtsEvent {
                    id: id.to_string(),
                    text: "waiting".into(),
                    author: AuthorIdentity {
                        username: "user".into(),
                        display_avatar_url: String::new(),
                    },
                    guild_tag: None,
                    content_type: String::new(),
                    timestamp: id,
                    visual_only: true,
                    segments: vec![],
                }),
                StageLane::Tts,
            )
            .await;
    }
    scheduler.enqueue(media(4, "4"), StageLane::Media).await;
    assert!(matches!(rx.try_recv().unwrap(), RelayEvent::Media(_)));
    scheduler.stage_state(true, false, false).await;
    scheduler.stage_state(false, false, false).await;
    assert!(rx.try_recv().is_err());
    assert_eq!(scheduler.queue_snapshot().await.len(), 2);
    scheduler.set_messages_pinned(false, 2).await;
    assert!(matches!(rx.try_recv().unwrap(), RelayEvent::Tts(event) if event.id == "1"));
    scheduler.stage_state(false, false, true).await;
    scheduler.stage_state(false, false, false).await;
    assert!(matches!(rx.try_recv().unwrap(), RelayEvent::Tts(event) if event.id == "2"));
}

fn media(timestamp: u64, message_id: &str) -> RelayEvent {
    RelayEvent::Media(MediaEvent {
        kind: MediaKind::Image,
        url: format!("https://cdn.discordapp.com/{message_id}.png"),
        proxy_url: String::new(),
        filename: format!("{message_id}.png"),
        content_type: "image/png".into(),
        artwork_id: None,
        audio_id: None,
        cached_media_id: None,
        title: None,
        artist: None,
        text: None,
        author: AuthorIdentity {
            username: "user".into(),
            display_avatar_url: String::new(),
        },
        timestamp,
        message_id: message_id.into(),
    })
}

fn music(playback_id: &str, title: &str) -> RelayEvent {
    RelayEvent::MusicPlay(crate::model::MusicPlaybackEvent {
        playback_id: playback_id.into(),
        video_id: format!("video-{playback_id}"),
        title: title.into(),
        channel_title: "Relay".into(),
        thumbnail: String::new(),
        duration_seconds: 30,
        mode: crate::model::MusicPlaybackMode::Full,
        start_seconds: 0,
        end_seconds: None,
        requested_by: "Listener".into(),
    })
}

#[tokio::test]
async fn older_pending_ticket_blocks_newer_ready_media() {
    let (tx, mut rx) = broadcast::channel(16);
    let scheduler = StageScheduler::new(tx);
    let older = scheduler.reserve(22_000, "100", 0, StageLane::Tts).await;
    scheduler
        .ready(
            scheduler
                .reserve(22_001, "101", 200, StageLane::Media)
                .await,
            media(22_001, "101"),
        )
        .await;
    assert!(rx.try_recv().is_err());
    scheduler
        .ready(
            older,
            RelayEvent::Tts(TtsEvent {
                id: "100".into(),
                text: "older".into(),
                author: AuthorIdentity {
                    username: "user".into(),
                    display_avatar_url: String::new(),
                },
                guild_tag: None,
                content_type: String::new(),
                timestamp: 22_000,
                visual_only: true,
                segments: Vec::new(),
            }),
        )
        .await;
    assert!(matches!(rx.recv().await.unwrap(), RelayEvent::Tts(_)));
    scheduler.stage_state(false, false, true).await;
    scheduler.stage_state(false, false, false).await;
    assert!(matches!(rx.recv().await.unwrap(), RelayEvent::Media(_)));
}

#[tokio::test]
async fn active_event_is_never_preempted_by_an_older_late_arrival() {
    let (tx, mut rx) = broadcast::channel(16);
    let scheduler = StageScheduler::new(tx);
    scheduler
        .enqueue(media(22_001, "101"), StageLane::Media)
        .await;
    assert!(matches!(rx.recv().await.unwrap(), RelayEvent::Media(_)));
    scheduler.stage_state(true, false, false).await;
    scheduler
        .enqueue(media(22_000, "100"), StageLane::Media)
        .await;
    assert!(rx.try_recv().is_err());
    scheduler.stage_state(false, false, false).await;
    assert!(matches!(rx.recv().await.unwrap(), RelayEvent::Media(_)));
}

#[tokio::test]
async fn slow_head_is_demoted_then_reinserted_without_being_lost() {
    let (tx, mut rx) = broadcast::channel(16);
    let scheduler =
        StageScheduler::with_timeouts(tx, Duration::from_millis(20), Duration::from_millis(20));
    let slow_text = scheduler.reserve(22_000, "100", 0, StageLane::Tts).await;
    let image = scheduler
        .reserve(22_001, "101", 200, StageLane::Media)
        .await;
    scheduler.ready(image, media(22_001, "101")).await;

    assert!(
        tokio::time::timeout(Duration::from_millis(10), rx.recv())
            .await
            .is_err()
    );
    assert!(matches!(rx.recv().await.unwrap(), RelayEvent::Media(_)));
    scheduler.stage_state(true, false, false).await;
    scheduler.stage_state(false, false, false).await;

    scheduler
        .ready(
            slow_text,
            RelayEvent::Tts(TtsEvent {
                id: "100".into(),
                text: "slow text".into(),
                author: AuthorIdentity {
                    username: "user".into(),
                    display_avatar_url: String::new(),
                },
                guild_tag: None,
                content_type: String::new(),
                timestamp: 22_000,
                visual_only: true,
                segments: Vec::new(),
            }),
        )
        .await;
    assert!(matches!(rx.recv().await.unwrap(), RelayEvent::Tts(_)));
}

#[tokio::test]
async fn reinserts_a_demoted_message_in_its_original_part_order() {
    let (tx, mut rx) = broadcast::channel(16);
    let scheduler =
        StageScheduler::with_timeouts(tx, Duration::from_millis(20), Duration::from_millis(20));
    let text = scheduler.reserve(22_000, "100", 0, StageLane::Tts).await;
    let sticker = scheduler
        .reserve(22_000, "100", 100, StageLane::Media)
        .await;
    let newer_image = scheduler
        .reserve(22_001, "101", 200, StageLane::Media)
        .await;
    scheduler
        .ready(
            sticker,
            RelayEvent::Sticker(StickerEvent {
                id: "sticker".into(),
                name: "sticker".into(),
                format: "png".into(),
                url: "https://cdn.discordapp.com/stickers/sticker.png".into(),
                cached_media_id: None,
                author: AuthorIdentity {
                    username: "user".into(),
                    display_avatar_url: String::new(),
                },
                timestamp: 22_000,
                message_id: "100".into(),
            }),
        )
        .await;
    scheduler.ready(newer_image, media(22_001, "101")).await;

    assert!(
        matches!(rx.recv().await.unwrap(), RelayEvent::Media(event) if event.message_id == "101")
    );
    scheduler.stage_state(true, false, false).await;
    scheduler.stage_state(false, false, false).await;

    scheduler
        .ready(
            text,
            RelayEvent::Tts(TtsEvent {
                id: "100".into(),
                text: "slow text".into(),
                author: AuthorIdentity {
                    username: "user".into(),
                    display_avatar_url: String::new(),
                },
                guild_tag: None,
                content_type: String::new(),
                timestamp: 22_000,
                visual_only: true,
                segments: Vec::new(),
            }),
        )
        .await;
    assert!(matches!(rx.recv().await.unwrap(), RelayEvent::Tts(event) if event.id == "100"));
    scheduler.stage_state(false, false, true).await;
    scheduler.stage_state(false, false, false).await;
    assert!(
        matches!(rx.recv().await.unwrap(), RelayEvent::Sticker(event) if event.id == "sticker")
    );
}

#[tokio::test]
async fn queue_removal_preserves_active_playback_and_order() {
    let (tx, mut rx) = broadcast::channel(16);
    let scheduler = StageScheduler::new(tx);
    let active = scheduler.enqueue(media(1, "1"), StageLane::Media).await;
    let _ = rx.recv().await.unwrap();
    let waiting = scheduler.enqueue(media(2, "2"), StageLane::Media).await;
    let next = scheduler.enqueue(media(3, "3"), StageLane::Media).await;
    assert!(!scheduler.remove_queued(active.0).await);
    let snapshot = scheduler.queue_snapshot().await;
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0].id, format!("stage:{}", waiting.0));
    assert!(scheduler.remove_queued(waiting.0).await);
    assert_eq!(
        scheduler.queue_snapshot().await[0].id,
        format!("stage:{}", next.0)
    );
    scheduler.stage_state(true, false, false).await;
    scheduler.stage_state(false, false, false).await;
    assert!(
        matches!(rx.recv().await.unwrap(), RelayEvent::Media(value) if value.message_id == "3")
    );
}

#[tokio::test]
async fn queued_music_can_be_removed_but_dispatched_music_cannot() {
    let (tx, mut rx) = broadcast::channel(16);
    let scheduler = StageScheduler::new(tx);
    let event = RelayEvent::MusicPlay(crate::model::MusicPlaybackEvent {
        playback_id: "queued-song".into(),
        video_id: "sample".into(),
        title: "Sample".into(),
        channel_title: "Relay".into(),
        thumbnail: String::new(),
        duration_seconds: 30,
        mode: crate::model::MusicPlaybackMode::Full,
        start_seconds: 0,
        end_seconds: None,
        requested_by: "Listener".into(),
    });
    scheduler.stage_state(true, false, false).await;
    let queued = scheduler.enqueue(event.clone(), StageLane::Music).await;
    assert_eq!(scheduler.queue_snapshot().await[0].title, "Sample");
    assert_eq!(
        scheduler.remove_queued_music(queued.0).await.as_deref(),
        Some("queued-song")
    );
    scheduler.stage_state(false, false, false).await;
    let active = scheduler.enqueue(event, StageLane::Music).await;
    let _ = rx.recv().await.unwrap();
    assert_eq!(scheduler.remove_queued_music(active.0).await, None);
}

#[tokio::test]
async fn reorders_music_tickets_inside_their_existing_slots() {
    let (tx, _rx) = broadcast::channel(16);
    let scheduler = StageScheduler::new(tx);
    scheduler.stage_state(true, false, false).await;

    let first = scheduler.reserve(10, "1", 300, StageLane::Music).await;
    scheduler.ready(first, music("first", "First")).await;
    let media_ticket = scheduler.reserve(11, "2", 200, StageLane::Media).await;
    scheduler.ready(media_ticket, media(11, "middle")).await;
    let second = scheduler.reserve(12, "3", 300, StageLane::Music).await;
    scheduler.ready(second, music("second", "Second")).await;

    let before = scheduler.queue_snapshot().await;
    assert_eq!(
        before
            .iter()
            .map(|item| item.title.as_str())
            .collect::<Vec<_>>(),
        vec!["First", "middle.png", "Second"]
    );
    assert!(scheduler.reorder_music(&[second, first]).await);
    let after = scheduler.queue_snapshot().await;
    assert_eq!(
        after
            .iter()
            .map(|item| item.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Second", "middle.png", "First"]
    );
}

#[tokio::test]
async fn pending_music_reorders_around_a_ready_current_ticket() {
    let (tx, _rx) = broadcast::channel(16);
    let scheduler = StageScheduler::new(tx);
    scheduler.stage_state(true, false, false).await;

    // The first track is current in MusicState, but its stage ticket has not
    // dispatched yet because a media output is still busy.
    let current = scheduler.reserve(10, "1", 300, StageLane::Music).await;
    scheduler.ready(current, music("current", "Current")).await;
    let middle = scheduler.reserve(11, "2", 200, StageLane::Media).await;
    scheduler.ready(middle, media(11, "middle")).await;
    let first_pending = scheduler.reserve(12, "3", 300, StageLane::Music).await;
    let second_pending = scheduler.reserve(13, "4", 300, StageLane::Music).await;

    assert!(
        scheduler
            .reorder_music(&[second_pending, first_pending])
            .await
    );
    scheduler
        .ready(first_pending, music("first", "First"))
        .await;
    scheduler
        .ready(second_pending, music("second", "Second"))
        .await;
    let snapshot = scheduler.queue_snapshot().await;
    assert_eq!(
        snapshot
            .iter()
            .map(|item| item.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Current", "middle.png", "Second", "First"]
    );
}

#[tokio::test]
async fn scheduler_rejects_reordering_an_active_or_incomplete_music_set() {
    let (tx, mut rx) = broadcast::channel(16);
    let scheduler = StageScheduler::new(tx);
    let active = scheduler
        .enqueue(music("active", "Active"), StageLane::Music)
        .await;
    assert!(matches!(rx.recv().await.unwrap(), RelayEvent::MusicPlay(_)));
    assert!(!scheduler.reorder_music(&[active]).await);

    scheduler.stage_state(false, true, false).await;
    let waiting = scheduler.reserve(20, "2", 300, StageLane::Music).await;
    scheduler.ready(waiting, music("waiting", "Waiting")).await;
    assert!(!scheduler.reorder_music(&[]).await);
    assert_eq!(scheduler.queue_snapshot().await[0].title, "Waiting");
}

#[tokio::test]
async fn paused_scheduler_drops_ready_content_until_resumed() {
    let (tx, mut rx) = broadcast::channel(16);
    let scheduler = StageScheduler::new(tx);
    scheduler.set_paused(true);
    let dropped = scheduler
        .reserve(30_000, "300", 200, StageLane::Media)
        .await;
    scheduler.ready(dropped, media(30_000, "300")).await;
    assert!(rx.try_recv().is_err());
    assert!(scheduler.queue_snapshot().await.is_empty());

    scheduler.set_paused(false);
    let shown = scheduler
        .reserve(30_001, "301", 200, StageLane::Media)
        .await;
    scheduler.ready(shown, media(30_001, "301")).await;
    assert!(matches!(rx.try_recv(), Ok(RelayEvent::Media(_))));
}
