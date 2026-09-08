use super::*;

fn track(video_id: &str, duration_seconds: u64) -> YouTubeTrack {
    YouTubeTrack {
        video_id: video_id.into(),
        title: "Test track".into(),
        channel_title: "Test channel".into(),
        thumbnail: "https://i.ytimg.com/vi/test/default.jpg".into(),
        duration_seconds,
    }
}

fn selection() -> MusicSelection {
    selection_with_id("video-1")
}

fn selection_with_id(video_id: &str) -> MusicSelection {
    MusicSelection {
        owner_id: 7,
        owner_name: "stealthy".into(),
        channel_id: 9,
        track: track(video_id, 90),
    }
}

#[test]
fn looping_yields_to_queued_tracks_and_rejects_duplicate_end_events() {
    let mut state = MusicState::default();
    let MusicStartResult::Started(first) = state.start(selection(), MusicPlaybackMode::Preview)
    else {
        panic!()
    };
    state.set_now_playing_message_id(&first.playback_id, 42);
    assert_eq!(
        state.toggle_loop(&first.playback_id, 8),
        Err(MusicSkipDecision::NotOwner)
    );
    assert_eq!(state.toggle_loop(&first.playback_id, 7), Ok(true));
    let MusicStartResult::Queued {
        playback: second, ..
    } = state.start(selection_with_id("video-2"), MusicPlaybackMode::Full)
    else {
        panic!()
    };
    assert!(state.finish_current(&first.playback_id).unwrap().1);
    assert_eq!(
        state.promote_next().unwrap().playback_id,
        second.playback_id
    );
    assert!(state.finish_current(&first.playback_id).is_none());
    state.stop_current();
    let repeated = state.promote_next().unwrap();
    assert_ne!(repeated.playback_id, first.playback_id);
    assert_eq!(repeated.end_seconds, Some(30));
    assert_eq!(state.active_message_ids(), vec![(9, 42)]);
    assert_eq!(state.toggle_loop(&first.playback_id, 7), Ok(false));
    assert!(!state.finish_current(&repeated.playback_id).unwrap().1);
    assert!(state.promote_next().is_none());
}

#[test]
fn skipping_a_loop_never_requeues_it() {
    let mut state = MusicState::default();
    let MusicStartResult::Started(first) = state.start(selection(), MusicPlaybackMode::Full) else {
        panic!()
    };
    state.toggle_loop(&first.playback_id, 7).unwrap();
    assert!(state.stop_if_current(&first.playback_id).is_some());
    assert!(state.promote_next().is_none());
    assert_eq!(
        state.toggle_loop(&first.playback_id, 7),
        Err(MusicSkipDecision::NotCurrent)
    );
}

#[test]
fn loop_off_cancels_a_waiting_repeat_but_preserves_an_original_queued_track() {
    let mut state = MusicState::default();
    let MusicStartResult::Started(first) = state.start(selection(), MusicPlaybackMode::Full) else {
        panic!()
    };
    state.toggle_loop(&first.playback_id, 7).unwrap();
    let MusicStartResult::Queued {
        playback: second, ..
    } = state.start(selection_with_id("video-2"), MusicPlaybackMode::Full)
    else {
        panic!()
    };
    assert!(state.waiting_repeat_id(&second.playback_id).is_none());
    state.finish_current(&first.playback_id).unwrap();
    state.promote_next();
    assert_eq!(state.toggle_loop(&first.playback_id, 7), Ok(false));
    let repeat = state.waiting_repeat_id(&first.playback_id).unwrap();
    assert!(state.remove_pending(&repeat).is_some());
    assert_eq!(
        state.current_event().unwrap().playback_id,
        second.playback_id
    );
    state.stop_current();
    assert!(state.promote_next().is_none());
}

#[test]
fn only_the_requester_can_select_and_take_a_track() {
    let mut state = MusicState::default();
    let search_id = state.insert_search(7, 9, "test".into(), vec![track("video-1", 90)]);
    assert_eq!(
        state.select_search(&search_id, 8, "other", "video-1"),
        SearchSelection::NotOwner
    );
    let SearchSelection::Selected(selection_id) =
        state.select_search(&search_id, 7, "stealthy", "video-1")
    else {
        panic!("expected a selection");
    };
    assert!(matches!(
        state.take_selection(&selection_id, 8),
        SelectionTake::NotOwner
    ));
    assert!(matches!(
        state.take_selection(&selection_id, 7),
        SelectionTake::Taken(_)
    ));
    assert_eq!(
        state.take_selection(&selection_id, 7),
        SelectionTake::NotFound
    );
}

#[test]
fn rejected_playback_can_restore_the_pending_selection() {
    let mut state = MusicState::default();
    let search_id = state.insert_search(7, 9, "test".into(), vec![track("video-1", 90)]);
    let SearchSelection::Selected(selection_id) =
        state.select_search(&search_id, 7, "stealthy", "video-1")
    else {
        panic!("expected a selection");
    };
    let SelectionTake::Taken(selection) = state.take_selection(&selection_id, 7) else {
        panic!("expected selection ownership");
    };

    state.restore_selection(&selection_id, selection);

    assert!(matches!(
        state.take_selection(&selection_id, 7),
        SelectionTake::Taken(_)
    ));
}

#[test]
fn preview_is_cut_at_thirty_seconds_and_full_has_no_cutoff() {
    let mut state = MusicState::default();
    let MusicStartResult::Started(preview) = state.start(selection(), MusicPlaybackMode::Preview)
    else {
        panic!("expected started");
    };
    assert_eq!(preview.end_seconds, Some(30));
    assert_eq!(preview.requested_by, "stealthy");
    // Second start queues while the first is still current.
    let MusicStartResult::Queued {
        playback: full,
        position,
    } = state.start(selection_with_id("video-2"), MusicPlaybackMode::Full)
    else {
        panic!("expected queued");
    };
    assert_eq!(position, 1);
    assert_eq!(full.end_seconds, None);
    assert_eq!(full.start_seconds, 0);
    assert_eq!(full.duration_seconds, 90);
}

#[test]
fn a_second_track_queues_instead_of_replacing() {
    let mut state = MusicState::default();
    let MusicStartResult::Started(first) = state.start(selection(), MusicPlaybackMode::Full) else {
        panic!("expected started");
    };
    let MusicStartResult::Queued {
        playback: second,
        position,
    } = state.start(selection_with_id("video-2"), MusicPlaybackMode::Full)
    else {
        panic!("expected queued");
    };
    assert_eq!(position, 1);
    assert_eq!(state.current_event(), Some(first.clone()));
    assert!(state.stop_if_current(&second.playback_id).is_none());
    let stopped = state
        .stop_if_current(&first.playback_id)
        .expect("first stopped");
    assert_eq!(stopped.playback, first);
    assert_eq!(state.promote_next(), Some(second.clone()));
    assert_eq!(state.current_event(), Some(second));
}

#[test]
fn queue_full_rejects_additional_tracks() {
    let mut state = MusicState::default();
    assert!(matches!(
        state.start_with_policy(
            selection(),
            MusicPlaybackMode::Full,
            MusicQueuePolicy::unlimited()
        ),
        MusicStartResult::Started(_)
    ));
    for index in 0..MUSIC_QUEUE_CAP {
        assert!(matches!(
            state.start_with_policy(
                selection_with_id(&format!("video-{}", index + 2)),
                MusicPlaybackMode::Full,
                MusicQueuePolicy::unlimited()
            ),
            MusicStartResult::Queued { .. }
        ));
    }
    assert_eq!(
        state.start_with_policy(
            selection_with_id("video-final"),
            MusicPlaybackMode::Full,
            MusicQueuePolicy::unlimited()
        ),
        MusicStartResult::QueueFull
    );
    // Current + MUSIC_QUEUE_CAP pending; stop+promote drains the FIFO.
    assert!(state.stop_current().is_some());
    for _ in 0..MUSIC_QUEUE_CAP {
        assert!(state.promote_next().is_some());
        assert!(state.stop_current().is_some());
    }
    assert!(state.promote_next().is_none());
}

#[test]
fn clear_all_drops_current_and_pending() {
    let mut state = MusicState::default();
    let MusicStartResult::Started(first) = state.start(selection(), MusicPlaybackMode::Full) else {
        panic!("expected started");
    };
    assert!(matches!(
        state.start(selection_with_id("video-2"), MusicPlaybackMode::Full),
        MusicStartResult::Queued { .. }
    ));
    assert_eq!(
        state.clear_all().map(|stopped| stopped.playback),
        Some(first)
    );
    assert!(state.current_event().is_none());
    assert!(state.promote_next().is_none());
}

#[test]
fn stop_returns_now_playing_announce_for_cleanup() {
    let mut state = MusicState::default();
    let MusicStartResult::Started(playback) = state.start(selection(), MusicPlaybackMode::Full)
    else {
        panic!("expected started");
    };
    assert!(state.set_now_playing_message_id(&playback.playback_id, 99));
    let stopped = state
        .stop_if_current(&playback.playback_id)
        .expect("stopped");
    assert_eq!(stopped.playback, playback);
    assert_eq!(stopped.channel_id, 9);
    assert_eq!(stopped.now_playing_message_id, Some(99));
}

#[test]
fn skip_requires_the_requester_only() {
    let mut state = MusicState::default();
    let MusicStartResult::Started(playback) = state.start(selection(), MusicPlaybackMode::Full)
    else {
        panic!("expected started");
    };
    assert_eq!(
        state.skip_decision(&playback.playback_id, 7),
        MusicSkipDecision::Allowed
    );
    assert_eq!(
        state.skip_decision(&playback.playback_id, 8),
        MusicSkipDecision::NotOwner
    );
    assert_eq!(
        state.skip_decision("missing", 7),
        MusicSkipDecision::NotCurrent
    );
}

#[test]
fn parses_seconds_and_m_ss_timestamps() {
    assert_eq!(parse_timestamp("50"), Some(50));
    assert_eq!(parse_timestamp("0:50"), Some(50));
    assert_eq!(parse_timestamp("1:50"), Some(110));
    assert_eq!(parse_timestamp("1:60"), None);
    assert_eq!(parse_timestamp(""), None);
}

#[test]
fn custom_range_allows_a_one_minute_window_inside_the_track() {
    assert_eq!(validate_custom_range(180, 50, 110), Ok((50, 110)));
    assert_eq!(validate_custom_range(90, 0, 60), Ok((0, 60)));
    assert_eq!(
        validate_custom_range(90, 35, 95),
        Err(CustomRangeError::OutsideTrack)
    );
    assert_eq!(
        validate_custom_range(180, 10, 80),
        Err(CustomRangeError::WindowTooLong)
    );
    assert_eq!(
        validate_custom_range(180, 40, 40),
        Err(CustomRangeError::EmptyRange)
    );
}

#[test]
fn start_custom_sets_the_validated_window() {
    let mut state = MusicState::default();
    let selection = MusicSelection {
        owner_id: 7,
        owner_name: "stealthy".into(),
        channel_id: 9,
        track: track("video-1", 180),
    };
    let MusicStartResult::Started(playback) = state.start_custom(selection, 50, 110).unwrap()
    else {
        panic!("expected started");
    };
    assert_eq!(playback.mode, MusicPlaybackMode::Custom);
    assert_eq!(playback.start_seconds, 50);
    assert_eq!(playback.end_seconds, Some(110));
}

#[test]
fn default_policy_limits_pending_tracks_per_user_but_keeps_the_current_track_free() {
    let mut state = MusicState::default();
    assert!(matches!(
        state.start(selection(), MusicPlaybackMode::Full),
        MusicStartResult::Started(_)
    ));
    for index in 0..3 {
        assert!(matches!(
            state.start(
                selection_with_id(&format!("pending-{index}")),
                MusicPlaybackMode::Full
            ),
            MusicStartResult::Queued { .. }
        ));
    }
    assert_eq!(
        state.start(selection_with_id("pending-limit"), MusicPlaybackMode::Full),
        MusicStartResult::UserQueueFull { limit: 3 }
    );

    let mut other_user = selection_with_id("other-user");
    other_user.owner_id = 8;
    assert!(matches!(
        state.start(other_user, MusicPlaybackMode::Full),
        MusicStartResult::Queued { position: 4, .. }
    ));
}

#[test]
fn duplicate_pending_is_rejected_by_canonical_video_id_and_can_be_disabled() {
    let mut state = MusicState::default();
    assert!(matches!(
        state.start(selection_with_id("current"), MusicPlaybackMode::Full),
        MusicStartResult::Started(_)
    ));
    assert!(matches!(
        state.start(selection_with_id("video-dup"), MusicPlaybackMode::Full),
        MusicStartResult::Queued { .. }
    ));

    let duplicate = selection_with_id("https://youtu.be/video-dup");
    assert_eq!(
        state.start(duplicate.clone(), MusicPlaybackMode::Full),
        MusicStartResult::DuplicatePending {
            video_id: "video-dup".into()
        }
    );
    assert!(matches!(
        state.start_with_policy(
            duplicate,
            MusicPlaybackMode::Full,
            MusicQueuePolicy::new(0, false)
        ),
        MusicStartResult::Queued { .. }
    ));
}

#[test]
fn pending_tracks_can_move_without_touching_the_current_track() {
    let mut state = MusicState::default();
    let current = match state.start_with_policy(
        selection_with_id("current"),
        MusicPlaybackMode::Full,
        MusicQueuePolicy::unlimited(),
    ) {
        MusicStartResult::Started(playback) => playback,
        result => panic!("expected current playback, got {result:?}"),
    };
    let mut pending_ids = Vec::new();
    for id in ["a", "b", "c"] {
        let result = state.start_with_policy(
            selection_with_id(id),
            MusicPlaybackMode::Full,
            MusicQueuePolicy::unlimited(),
        );
        let MusicStartResult::Queued { playback, .. } = result else {
            panic!("expected queued track");
        };
        pending_ids.push(playback.playback_id);
    }
    assert_eq!(
        state.move_pending(&pending_ids[1], MusicQueueDirection::Up),
        MusicQueueMove::Moved { position: 1 }
    );
    assert_eq!(
        state.pending_playback_ids(),
        vec![
            pending_ids[1].clone(),
            pending_ids[0].clone(),
            pending_ids[2].clone()
        ]
    );
    assert_eq!(
        state.move_pending("missing", MusicQueueDirection::Up),
        MusicQueueMove::NotFound
    );
    assert_eq!(
        state.move_pending(&current.playback_id, MusicQueueDirection::Down),
        MusicQueueMove::Current
    );
}

#[test]
fn canonical_music_id_trims_common_youtube_links_without_changing_case() {
    assert_eq!(canonical_music_id(" video-1 "), "video-1");
    assert_eq!(
        canonical_music_id("https://www.youtube.com/watch?v=Video-1&t=4"),
        "Video-1"
    );
    assert_eq!(
        canonical_music_id("https://youtu.be/Video-1#t=4"),
        "Video-1"
    );
}

#[test]
fn remaining_search_cooldown_is_none_when_fresh_or_elapsed() {
    let now = Instant::now();
    assert!(remaining_search_cooldown(None, now, MUSIC_SEARCH_COOLDOWN).is_none());
    assert!(
        remaining_search_cooldown(
            Some(now - MUSIC_SEARCH_COOLDOWN),
            now,
            MUSIC_SEARCH_COOLDOWN
        )
        .is_none()
    );
    let remaining = remaining_search_cooldown(
        Some(now - Duration::from_secs(2)),
        now,
        MUSIC_SEARCH_COOLDOWN,
    )
    .expect("still cooling down");
    assert!(remaining <= Duration::from_secs(4));
    assert!(remaining >= Duration::from_millis(3_900));
}

#[test]
fn cooldown_wait_seconds_ceils_partial_seconds() {
    assert_eq!(cooldown_wait_seconds(Duration::from_millis(1)), 1);
    assert_eq!(cooldown_wait_seconds(Duration::from_millis(1000)), 1);
    assert_eq!(cooldown_wait_seconds(Duration::from_millis(1001)), 2);
    assert_eq!(cooldown_wait_seconds(Duration::from_secs(6)), 6);
}

#[test]
fn search_cooldown_is_per_user_and_ignores_select_play() {
    let mut state = MusicState::default();
    let now = Instant::now();
    state.mark_search_attempt(7, now);
    assert!(state.search_cooldown_remaining(7, now).is_some());
    assert!(state.search_cooldown_remaining(8, now).is_none());

    let search_id = state.insert_search(7, 9, "test".into(), vec![track("video-1", 90)]);
    let SearchSelection::Selected(selection_id) =
        state.select_search(&search_id, 7, "stealthy", "video-1")
    else {
        panic!("select must work during search cooldown");
    };
    let SelectionTake::Taken(selection) = state.take_selection(&selection_id, 7) else {
        panic!("take must work during search cooldown");
    };
    assert!(matches!(
        state.start(selection, MusicPlaybackMode::Preview),
        MusicStartResult::Started(_)
    ));
    // Cooldown still only keyed by the search attempt, not by play.
    assert!(
        state
            .search_cooldown_remaining(7, now + Duration::from_secs(1))
            .is_some()
    );
    assert!(
        state
            .search_cooldown_remaining(7, now + MUSIC_SEARCH_COOLDOWN)
            .is_none()
    );
}
