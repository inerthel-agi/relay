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
fn search_uses_a_single_search_list_call() {
    // Quota guard: each search.list costs 100 units, so search() must issue exactly one.
    let source = include_str!("../youtube.rs");
    let body = source
        .split("pub async fn search(")
        .nth(1)
        .and_then(|rest| rest.split("\nasync fn ").next())
        .expect("search body");
    assert_eq!(body.matches("search_page(").count(), 1);
    assert!(!body.contains("tokio::join!"));
}

#[test]
fn pasted_youtube_links_resolve_to_their_video_id() {
    for link in [
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://youtube.com/watch?list=PL1&v=dQw4w9WgXcQ&t=42s",
        "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://music.youtube.com/watch?v=dQw4w9WgXcQ&si=abc",
        "https://youtu.be/dQw4w9WgXcQ?si=abc",
        "https://www.youtube.com/shorts/dQw4w9WgXcQ",
        "https://www.youtube.com/embed/dQw4w9WgXcQ",
        "https://www.youtube.com/live/dQw4w9WgXcQ",
        "  https://youtu.be/dQw4w9WgXcQ  ",
    ] {
        assert_eq!(
            video_id_from_link(link).as_deref(),
            Some("dQw4w9WgXcQ"),
            "{link}"
        );
    }
    for text in [
        "red velvet psycho",
        "https://youtu.be/dQw4w9WgXcQ please",
        "https://evil.example/watch?v=dQw4w9WgXcQ",
        "https://youtube.com.evil.example/watch?v=dQw4w9WgXcQ",
        "https://www.youtube.com/watch?v=short",
        "https://www.youtube.com/watch?v=dQw4w9WgXc!",
        "https://www.youtube.com/channel/UC1234567890",
        "javascript:alert(1)",
    ] {
        assert_eq!(video_id_from_link(text), None, "{text}");
    }
}

fn lookup_item(id: &str, duration: &str, embeddable: Option<bool>) -> LookupItem {
    LookupItem {
        id: id.into(),
        snippet: LookupSnippet {
            video: snippet("Pasted song"),
            category_id: Some("10".into()),
        },
        content_details: VideoContentDetails {
            duration: duration.into(),
        },
        status: embeddable.map(|embeddable| LookupStatus { embeddable }),
    }
}

#[test]
fn a_pasted_video_follows_the_search_rules() {
    let tracks = lookup_results(vec![lookup_item("dQw4w9WgXcQ", "PT3M33S", Some(true))]);
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].video_id, "dQw4w9WgXcQ");
    assert_eq!(tracks[0].duration_seconds, 213);

    // Not embeddable, unknown embed status, too long or too short: refused like in search.
    for item in [
        lookup_item("aaaaaaaaaaa", "PT3M", Some(false)),
        lookup_item("bbbbbbbbbbb", "PT3M", None),
        lookup_item("ccccccccccc", "PT12M", Some(true)),
        lookup_item("ddddddddddd", "PT30S", Some(true)),
    ] {
        assert!(lookup_results(vec![item]).is_empty());
    }
}

#[test]
fn lookup_response_parses_videos_list() {
    let body = br#"{"items":[{"id":"dQw4w9WgXcQ","snippet":{"title":"Song","channelTitle":"Artist","categoryId":"10","thumbnails":{"high":{"url":"https://i.ytimg.com/vi/x/hq.jpg"}}},"contentDetails":{"duration":"PT3M"},"status":{"embeddable":true}}]}"#;
    let response = serde_json::from_slice::<LookupResponse>(body).unwrap();
    let tracks = lookup_results(response.items);
    assert_eq!(tracks[0].title, "Song");
    assert_eq!(tracks[0].channel_title, "Artist");
}

fn track(video_id: &str) -> YouTubeTrack {
    YouTubeTrack {
        video_id: video_id.into(),
        title: "Song".into(),
        channel_title: "Channel".into(),
        thumbnail: "https://i.ytimg.com/default.jpg".into(),
        duration_seconds: 180,
    }
}

#[test]
fn repeated_searches_reuse_cached_results_until_they_expire() {
    let mut cache = SearchCache::default();
    let start = Instant::now();
    assert_eq!(
        SearchCache::key("  Red   Velvet PSYCHO "),
        "red velvet psycho"
    );
    cache.insert(
        SearchCache::key("red velvet psycho"),
        vec![track("a")],
        start,
    );

    let hit = cache.get(
        &SearchCache::key("RED velvet  psycho"),
        start + Duration::from_secs(60),
    );
    assert_eq!(hit.unwrap()[0].video_id, "a");
    assert!(cache.get("other query", start).is_none());
    assert!(
        cache
            .get(
                "red velvet psycho",
                start + SEARCH_CACHE_TTL + Duration::from_secs(1)
            )
            .is_none()
    );
    assert!(cache.entries.is_empty(), "expired entries are dropped");
}

#[test]
fn search_cache_keeps_the_most_recent_queries_only() {
    let mut cache = SearchCache::default();
    let now = Instant::now();
    for index in 0..=SEARCH_CACHE_LIMIT {
        cache.insert(format!("query {index}"), vec![track("a")], now);
    }
    assert_eq!(cache.entries.len(), SEARCH_CACHE_LIMIT);
    assert!(cache.get("query 0", now).is_none());
    assert!(
        cache
            .get(&format!("query {SEARCH_CACHE_LIMIT}"), now)
            .is_some()
    );
    // Re-inserting a query replaces it instead of duplicating it.
    cache.insert("query 5".into(), vec![track("b")], now);
    assert_eq!(cache.entries.len(), SEARCH_CACHE_LIMIT);
    assert_eq!(cache.get("query 5", now).unwrap()[0].video_id, "b");
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
