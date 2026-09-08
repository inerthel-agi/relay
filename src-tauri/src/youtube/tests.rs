use super::*;

fn snippet(title: &str) -> VideoSnippet {
    VideoSnippet {
        title: title.into(),
        channel_title: "Channel".into(),
        thumbnails: ThumbnailSet {
            default: Some(Thumbnail {
                url: "https://i.ytimg.com/default.jpg".into(),
            }),
            medium: None,
            high: None,
        },
    }
}

fn details(duration_seconds: u64, category_id: Option<&str>) -> VideoDetails {
    VideoDetails {
        duration_seconds,
        category_id: category_id.map(str::to_owned),
    }
}

#[test]
fn parses_youtube_durations() {
    assert_eq!(parse_iso8601_duration("PT2M57S"), Some(177));
    assert_eq!(parse_iso8601_duration("PT1H2M3S"), Some(3_723));
    assert_eq!(parse_iso8601_duration("PT0S"), Some(0));
    assert_eq!(parse_iso8601_duration("P2D"), None);
    assert_eq!(parse_iso8601_duration("PT2M30"), None);
}

#[test]
fn normalizes_queries_without_allowing_controls() {
    assert_eq!(normalize_query("  daft   punk  ").unwrap(), "daft   punk");
    assert!(normalize_query("song\nname").is_err());
}

#[test]
fn decodes_basic_html_entities_in_titles() {
    assert_eq!(
        clean_text("JENNIE&#39;s Mantra &amp; Friends", 80),
        "JENNIE's Mantra & Friends"
    );
}

#[test]
fn merge_keeps_relevance_ahead_of_recent_uploads() {
    let relevance = (1..=6)
        .map(|index| {
            (
                format!("rel-{index}"),
                snippet(&format!("Relevant {index}")),
            )
        })
        .collect::<Vec<_>>();
    let recent = vec![
        ("new-1".into(), snippet("Brand new")),
        ("rel-2".into(), snippet("Already in relevance")),
        ("new-2".into(), snippet("Also new")),
    ];
    let merged = merge_search_candidates(relevance, recent);
    let ids = merged
        .iter()
        .map(|(video_id, _)| video_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "rel-1", "rel-2", "rel-3", "rel-4", "rel-5", "rel-6", "new-1", "new-2"
        ]
    );
}

#[test]
fn shorts_spam_titles_are_detected() {
    assert!(is_shorts_spam_title(
        "jisoo dance #blackpink #fyp #ytshorts"
    ));
    assert!(is_shorts_spam_title("CLIP #shorts"));
    assert!(!is_shorts_spam_title(
        "JISOO - FLOWER (Official Music Video)"
    ));
    assert!(!is_shorts_spam_title("JISOO – FLOWER (Lyrics)"));
}

#[test]
fn jukebox_duration_rejects_shorts_length_and_marathons() {
    assert!(!is_jukebox_duration(48));
    assert!(is_jukebox_duration(60));
    assert!(is_jukebox_duration(174));
    assert!(is_jukebox_duration(300));
    assert!(!is_jukebox_duration(301));
}

#[test]
fn map_search_results_prefers_relevance_and_filters_spam() {
    let mut items = Vec::new();
    // First 10 slots simulate relevance primary window.
    for index in 1..=10 {
        items.push((
            format!("rel-{index}"),
            snippet(&format!("Relevant song {index}")),
        ));
    }
    items.push((
        "short-spam".into(),
        snippet("jisoo edit #blackpink #fyp #ytshorts"),
    ));
    items.push(("recent-music".into(), snippet("Fresh official audio")));
    items.push(("recent-other".into(), snippet("Interview clip")));

    let mut details_map = HashMap::new();
    for index in 1..=10 {
        details_map.insert(format!("rel-{index}"), details(150, Some("10")));
    }
    details_map.insert("short-spam".into(), details(42, Some("22")));
    details_map.insert("recent-music".into(), details(175, Some("10")));
    details_map.insert("recent-other".into(), details(120, Some("24")));

    let tracks = map_search_results(items, details_map);
    let ids = tracks
        .iter()
        .map(|track| track.video_id.as_str())
        .collect::<Vec<_>>();

    assert!(!ids.contains(&"short-spam"));
    assert_eq!(
        &ids[..10],
        &[
            "rel-1", "rel-2", "rel-3", "rel-4", "rel-5", "rel-6", "rel-7", "rel-8", "rel-9",
            "rel-10",
        ]
    );
    assert_eq!(ids[10], "recent-music");
    assert_eq!(ids[11], "recent-other");
}

#[test]
fn map_search_results_keeps_streamable_tracks_only() {
    let items = vec![
        ("short".into(), snippet("Too short")),
        ("ok".into(), snippet("Good track")),
        ("long".into(), snippet("Too long")),
        ("spam".into(), snippet("clip #shorts #fyp #ytshorts")),
    ];
    let details_map = HashMap::from([
        ("short".into(), details(12, Some("10"))),
        ("ok".into(), details(148, Some("10"))),
        ("long".into(), details(400, Some("10"))),
        ("spam".into(), details(90, Some("22"))),
    ]);
    let tracks = map_search_results(items, details_map);
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].video_id, "ok");
    assert_eq!(tracks[0].duration_seconds, 148);
}

#[test]
fn youtube_error_message_detects_quota_without_leaking_bodies_unparsed() {
    let body = br#"{"error":{"message":"Quota exceeded","errors":[{"reason":"quotaExceeded"}]}}"#;
    let message = youtube_error_message(body, 403, "search");
    assert!(message.contains("quota exceeded"));
    assert!(!message.contains("AIza"));
}
