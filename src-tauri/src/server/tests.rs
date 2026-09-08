use super::*;

use std::io::{Read, Write};

use crate::{
    config::AppConfig,
    credentials::load_or_create_relay_secret,
    model::{AuthorIdentity, MediaEvent, MediaKind, RelayEvent},
    state::{CachedMedia, MediaArtwork, MediaAudio, TtsAudio},
};

#[test]
fn injects_hex_secret_into_html_head() {
    let html = "<html><head></head><body></body></html>";
    assert_eq!(
        inject_relay_secret(html, "abc123"),
        "<html><head><meta name=\"relay-secret\" content=\"abc123\"></head><body></body></html>"
    );
    assert_eq!(inject_relay_secret(html, "not hex!"), html);
    let already = "<html><head><meta name=\"relay-secret\" content=\"abc123\"></head></html>";
    assert_eq!(inject_relay_secret(already, "abc123"), already);
}

#[test]
fn compares_secrets_without_prefix_matches() {
    assert!(secret_matches(Some("private"), "private"));
    assert!(!secret_matches(Some("priv"), "private"));
    assert!(!secret_matches(None, "private"));

    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        HeaderValue::from_static("theme=dark; relay_secret=private"),
    );
    assert!(request_secret_matches(None, &headers, "private"));
}

#[test]
fn accepts_only_local_overlay_and_tauri_origins() {
    assert!(origin_allowed(None));
    assert!(origin_allowed(Some(&HeaderValue::from_static(
        "http://127.0.0.1:4590"
    ))));
    assert!(origin_allowed(Some(&HeaderValue::from_static(
        "http://localhost:4590"
    ))));
    assert!(origin_allowed(Some(&HeaderValue::from_static(
        "http://tauri.localhost"
    ))));
    assert!(!origin_allowed(Some(&HeaderValue::from_static(
        "https://example.com"
    ))));
}

#[test]
fn youtube_widget_pages_use_localhost_referrer_host() {
    // YouTube rejects embeds whose Referer is http://127.0.0.1 (error 150)
    // but accepts http://localhost on the same loopback interface.
    assert!(crate::widget::youtube_embed_host().starts_with("localhost"));
    assert_ne!(crate::widget::youtube_embed_host(), "127.0.0.1");
}

#[test]
fn youtube_loopback_ip_redirects_to_localhost() {
    let mut headers = HeaderMap::new();
    headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:4590"));
    let response = redirect_path_off_loopback_ip(&headers, "/youtube").expect("redirect");
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response.headers().get(header::LOCATION).unwrap(),
        "http://localhost:4590/youtube"
    );

    let mut localhost = HeaderMap::new();
    localhost.insert(header::HOST, HeaderValue::from_static("localhost:4590"));
    assert!(redirect_path_off_loopback_ip(&localhost, "/youtube").is_none());
}

#[test]
fn obs_visual_loopback_ip_redirects_to_localhost() {
    let mut headers = HeaderMap::new();
    headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:4590"));
    let response = redirect_path_off_loopback_ip(&headers, "/obs/visual").expect("redirect");
    assert_eq!(
        response.headers().get(header::LOCATION).unwrap(),
        "http://localhost:4590/obs/visual"
    );
}

#[test]
fn output_config_never_serializes_private_scanner_values() {
    let config = AppConfig {
        privacy_custom_patterns: vec!["private-value-marker".into()],
        privacy_allowlist: vec!["allowlist-value-marker".into()],
        ..AppConfig::default()
    };
    let serialized = serde_json::to_string(&OverlayConfig::from(&config)).unwrap();

    assert!(!serialized.contains("private-value-marker"));
    assert!(!serialized.contains("allowlist-value-marker"));
    assert!(!serialized.contains("privacyCustomPatterns"));
    assert!(!serialized.contains("privacyAllowlist"));
}

#[test]
fn classifies_valid_output_sources_and_client_contexts() {
    let visual_preview =
        output_connection("overlay", &access_query(Some("visual"), Some("preview")))
            .expect("visual preview should be accepted");
    assert_eq!(visual_preview.source, OutputSource::Visual);
    assert_eq!(visual_preview.client, OutputClient::Preview);

    let combined = output_connection("overlay", &access_query(None, None))
        .expect("legacy overlay should remain supported");
    assert_eq!(combined.source, OutputSource::All);
    assert_eq!(combined.client, OutputClient::Obs);
    let widget = output_connection("overlay", &access_query(None, Some("widget")))
        .expect("the Windows widget should be accepted for the combined overlay");
    assert!(output_receives_music(Some(widget)));

    assert!(output_connection("tts", &access_query(Some("audio"), None)).is_none());
    assert!(output_connection("tts", &access_query(None, Some("widget"))).is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn serves_authenticated_overlay_and_broadcasts_under_load() {
    let port = free_local_port();
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        port,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    start_server(core.clone()).await.unwrap();
    let secret = load_or_create_relay_secret().unwrap();

    assert!(http_status(port, "/overlay").starts_with("HTTP/1.1 401"));
    assert!(http_status(port, &format!("/overlay?secret={secret}")).starts_with("HTTP/1.1 200"));
    let visual_response = http_response(port, "/medias");
    assert!(visual_response.starts_with("HTTP/1.1 200"));
    assert!(visual_response.contains(&format!(
        "<meta name=\"relay-secret\" content=\"{secret}\">"
    )));
    assert!(!visual_response.contains(&format!("relay_secret={secret}")));
    assert!(
        visual_response
            .to_ascii_lowercase()
            .contains("set-cookie: relay_secret=; path=/; httponly; samesite=strict; max-age=0")
    );
    assert!(visual_response.contains("content=\"visual\""));
    assert!(visual_response.contains("https://*.discordapp.net"));
    assert!(visual_response.contains("https://*.klipy.com"));
    assert!(visual_response.contains("https://i.ytimg.com"));
    let visual_headers = visual_response.to_ascii_lowercase();
    assert!(visual_headers.contains("referrer-policy: strict-origin-when-cross-origin"));
    assert!(!visual_headers.contains("referrer-policy: no-referrer"));
    let audio_response = http_response(port, "/audios");
    assert!(audio_response.starts_with("HTTP/1.1 200"));
    assert!(audio_response.contains("content=\"audio\""));
    // /youtube on 127.0.0.1 must redirect: YouTube rejects that Referer (error 150).
    let youtube_redirect = http_response(port, "/youtube");
    assert!(youtube_redirect.starts_with("HTTP/1.1 307"));
    assert!(youtube_redirect.contains(&format!("location: http://localhost:{port}/youtube")));
    let youtube_response = http_response_with_host(port, "/youtube", "localhost");
    assert!(youtube_response.starts_with("HTTP/1.1 200"));
    assert!(youtube_response.contains("content=\"youtube\""));
    let obs_visual_redirect = http_response(port, "/obs/visual");
    assert!(obs_visual_redirect.starts_with("HTTP/1.1 307"));
    assert!(obs_visual_redirect.contains(&format!("location: http://localhost:{port}/obs/visual")));
    let obs_visual = http_response_with_host(port, "/obs/visual", "localhost");
    assert!(obs_visual.starts_with("HTTP/1.1 200"));
    assert!(obs_visual.contains("src=\"/medias\""));
    assert!(obs_visual.contains("src=\"/stickers\""));
    assert!(obs_visual.contains("src=\"/notifications\""));
    assert!(obs_visual.contains("src=\"/youtube\""));
    assert!(obs_visual.contains("allowtransparency=\"true\""));
    assert!(!obs_visual.contains("color-scheme\" content=\"light only\""));
    let obs_audio = http_response(port, "/obs/audio");
    assert!(obs_audio.starts_with("HTTP/1.1 200"));
    assert!(obs_audio.contains("src=\"/audios\""));
    assert!(!obs_audio.contains("src=\"/tts\""));
    assert!(http_status(port, "/tts").starts_with("HTTP/1.1 200"));
    assert!(http_status(port, "/tts?secret=wrong").starts_with("HTTP/1.1 401"));
    assert!(http_status(port, &format!("/tts?secret={secret}")).starts_with("HTTP/1.1 200"));
    assert!(http_status(port, "/notifications").starts_with("HTTP/1.1 200"));
    assert!(http_status(port, "/notifications?secret=wrong").starts_with("HTTP/1.1 401"));
    assert!(
        http_status(port, &format!("/notifications?secret={secret}")).starts_with("HTTP/1.1 200")
    );
    core.tts_audio.write().await.push_front(TtsAudio {
        id: "123456789012345678".into(),
        content_type: "audio/wav".into(),
        bytes: axum::body::Bytes::from_static(b"RIFF-test"),
    });
    assert!(http_status(port, "/tts-audio/123456789012345678").starts_with("HTTP/1.1 401"));
    assert!(
        http_status(
            port,
            &format!("/tts-audio/123456789012345678?secret={secret}")
        )
        .starts_with("HTTP/1.1 200")
    );
    core.media_artwork.write().await.push_front(MediaArtwork {
        id: "223456789012345678".into(),
        content_type: "image/png".into(),
        bytes: axum::body::Bytes::from_static(b"PNG-test"),
    });
    assert!(http_status(port, "/media-artwork/223456789012345678").starts_with("HTTP/1.1 401"));
    assert!(
        http_status(
            port,
            &format!("/media-artwork/223456789012345678?secret={secret}")
        )
        .starts_with("HTTP/1.1 200")
    );
    core.media_audio.write().await.push_front(MediaAudio {
        id: "323456789012345678".into(),
        content_type: "audio/mpeg".into(),
        bytes: axum::body::Bytes::from_static(b"ID3-original-audio"),
    });
    assert!(http_status(port, "/media-audio/323456789012345678").starts_with("HTTP/1.1 401"));
    assert!(
        http_status(
            port,
            &format!("/media-audio/323456789012345678?secret={secret}")
        )
        .starts_with("HTTP/1.1 200")
    );
    core.cached_media.write().await.push_front(CachedMedia {
        id: "423456789012345678-embed-0".into(),
        content_type: "video/mp4".into(),
        bytes: axum::body::Bytes::from_static(b"0123456789"),
    });
    assert!(
        http_status(port, "/media-cache/423456789012345678-embed-0").starts_with("HTTP/1.1 401")
    );
    assert!(
        http_status(
            port,
            &format!("/media-cache/423456789012345678-embed-0?secret={secret}")
        )
        .starts_with("HTTP/1.1 200")
    );

    let mut clients = Vec::new();
    for _ in 0..8 {
        let (mut client, _) = tokio_tungstenite::connect_async(format!(
            "ws://127.0.0.1:{port}/ws?role=overlay&source=visual&client=obs&secret={secret}"
        ))
        .await
        .unwrap();
        let initial = client.next().await.unwrap().unwrap();
        assert!(initial.to_text().unwrap().contains("\"type\":\"config\""));
        let appearance = client.next().await.unwrap().unwrap();
        assert!(
            appearance
                .to_text()
                .unwrap()
                .contains("\"type\":\"appearance\"")
        );
        let clock = client.next().await.unwrap().unwrap();
        assert!(clock.to_text().unwrap().contains("\"type\":\"mediaClock\""));
        let stage = client.next().await.unwrap().unwrap();
        assert!(stage.to_text().unwrap().contains("\"type\":\"stageClock\""));
        let reaction = client.next().await.unwrap().unwrap();
        assert!(
            reaction
                .to_text()
                .unwrap()
                .contains("\"type\":\"reaction\"")
        );
        clients.push(client);
    }
    let (mut preview_client, _) = tokio_tungstenite::connect_async(format!(
        "ws://127.0.0.1:{port}/ws?role=overlay&source=visual&client=preview&secret={secret}"
    ))
    .await
    .unwrap();
    let initial = preview_client.next().await.unwrap().unwrap();
    assert!(initial.to_text().unwrap().contains("\"type\":\"config\""));
    let appearance = preview_client.next().await.unwrap().unwrap();
    assert!(
        appearance
            .to_text()
            .unwrap()
            .contains("\"type\":\"appearance\"")
    );
    let reaction = preview_client.next().await.unwrap().unwrap();
    assert!(
        reaction
            .to_text()
            .unwrap()
            .contains("\"type\":\"reaction\"")
    );
    clients.push(preview_client);
    let (mut widget_client, _) = tokio_tungstenite::connect_async(format!(
        "ws://127.0.0.1:{port}/ws?role=overlay&source=visual&client=widget&secret={secret}"
    ))
    .await
    .unwrap();
    let initial = widget_client.next().await.unwrap().unwrap();
    assert!(initial.to_text().unwrap().contains("\"type\":\"config\""));
    let appearance = widget_client.next().await.unwrap().unwrap();
    assert!(
        appearance
            .to_text()
            .unwrap()
            .contains("\"type\":\"appearance\"")
    );
    let stage = widget_client.next().await.unwrap().unwrap();
    assert!(stage.to_text().unwrap().contains("\"type\":\"stageClock\""));
    let reaction = widget_client.next().await.unwrap().unwrap();
    assert!(
        reaction
            .to_text()
            .unwrap()
            .contains("\"type\":\"reaction\"")
    );
    clients.push(widget_client);
    let (mut tts_client, _) = tokio_tungstenite::connect_async(format!(
        "ws://127.0.0.1:{port}/ws?role=tts&source=tts&client=obs&secret={secret}"
    ))
    .await
    .unwrap();
    let initial = tts_client.next().await.unwrap().unwrap();
    assert!(initial.to_text().unwrap().contains("\"type\":\"config\""));
    let appearance = tts_client.next().await.unwrap().unwrap();
    assert!(
        appearance
            .to_text()
            .unwrap()
            .contains("\"type\":\"appearance\"")
    );
    let stage = tts_client.next().await.unwrap().unwrap();
    assert!(stage.to_text().unwrap().contains("\"type\":\"stageClock\""));
    let reaction = tts_client.next().await.unwrap().unwrap();
    assert!(
        reaction
            .to_text()
            .unwrap()
            .contains("\"type\":\"reaction\"")
    );
    clients.push(tts_client);
    let (mut notification_client, _) = tokio_tungstenite::connect_async(format!(
        "ws://127.0.0.1:{port}/ws?role=notification&source=notification&client=obs&secret={secret}"
    ))
    .await
    .unwrap();
    let initial = notification_client.next().await.unwrap().unwrap();
    assert!(initial.to_text().unwrap().contains("\"type\":\"config\""));
    let appearance = notification_client.next().await.unwrap().unwrap();
    assert!(
        appearance
            .to_text()
            .unwrap()
            .contains("\"type\":\"appearance\"")
    );
    let stage = notification_client.next().await.unwrap().unwrap();
    assert!(stage.to_text().unwrap().contains("\"type\":\"stageClock\""));
    let reaction = notification_client.next().await.unwrap().unwrap();
    assert!(
        reaction
            .to_text()
            .unwrap()
            .contains("\"type\":\"reaction\"")
    );
    let pin = notification_client.next().await.unwrap().unwrap();
    assert!(pin.to_text().unwrap().contains("\"type\":\"messagePin\""));
    clients.push(notification_client);

    for index in 0..300 {
        let event = MediaEvent {
            kind: MediaKind::Image,
            url: format!("https://cdn.discordapp.com/test/{index}.png"),
            proxy_url: format!("https://media.discordapp.net/test/{index}.png"),
            filename: format!("{index}.png"),
            content_type: "image/png".into(),
            artwork_id: None,
            audio_id: None,
            cached_media_id: None,
            title: None,
            artist: None,
            text: None,
            author: AuthorIdentity {
                username: "stability".into(),
                display_avatar_url: "https://cdn.discordapp.com/avatar.png".into(),
            },
            timestamp: index,
            message_id: format!("10000000000000{index:04}"),
        };
        {
            let mut history = core.history.write().await;
            history.push_front(event.clone());
            history.truncate(crate::state::HISTORY_LIMIT);
        }
        let _ = core.relay_tx.send(RelayEvent::Media(event));
    }

    for client in &mut clients {
        for _ in 0..300 {
            let message = tokio::time::timeout(Duration::from_secs(5), client.next())
                .await
                .expect("broadcast timed out")
                .expect("socket closed")
                .expect("websocket error");
            assert!(message.to_text().unwrap().contains("\"type\":\"media\""));
        }
    }
    assert_eq!(core.history.read().await.len(), 50);
    let status = core.server_status.read().await.clone();
    assert_eq!(status.overlay_clients, 10);
    assert_eq!(status.outputs.visual.obs_clients, 8);
    assert_eq!(status.outputs.visual.preview_clients, 1);
    assert_eq!(status.outputs.visual.widget_clients, 1);
    assert!(status.outputs.visual.last_connected_at.is_some());
    assert_eq!(status.outputs.tts.obs_clients, 1);
    assert_eq!(status.outputs.notification.obs_clients, 1);

    start_server(core.clone()).await.unwrap();
    for client in &mut clients {
        let closed = tokio::time::timeout(Duration::from_secs(5), client.next())
            .await
            .expect("old socket did not close after restart");
        assert!(
            closed.is_none()
                || closed.is_some_and(|message| message.is_ok_and(|value| value.is_close()))
        );
    }
    drop(clients);
    stop_server(&core).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reaction_and_library_routes_keep_assets_private() {
    let port = free_local_port();
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        port,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    start_server(core.clone()).await.unwrap();
    let secret = load_or_create_relay_secret().unwrap();

    let reactions = http_response(port, "/reactions");
    assert!(reactions.starts_with("HTTP/1.1 200"));
    assert!(reactions.contains(&format!(
        "<meta name=\"relay-secret\" content=\"{secret}\">"
    )));
    assert!(
        reactions
            .to_ascii_lowercase()
            .contains("set-cookie: relay_secret=; path=/; httponly; samesite=strict; max-age=0")
    );

    assert!(http_status(port, "/reaction-sound/bad").starts_with("HTTP/1.1 401"));
    assert!(http_status(port, "/reaction-sound/bad?secret=wrong").starts_with("HTTP/1.1 401"));
    assert!(
        http_status(port, &format!("/reaction-sound/bad?secret={secret}"))
            .starts_with("HTTP/1.1 404")
    );

    assert!(http_status(port, "/library-asset/bad").starts_with("HTTP/1.1 401"));
    assert!(http_status(port, "/library-asset/bad?secret=wrong").starts_with("HTTP/1.1 401"));
    assert!(
        http_status(port, &format!("/library-asset/bad?secret={secret}"))
            .starts_with("HTTP/1.1 404")
    );

    let (mut reaction_client, _) = tokio_tungstenite::connect_async(format!(
        "ws://127.0.0.1:{port}/ws?role=reaction&source=reaction&client=obs&secret={secret}"
    ))
    .await
    .unwrap();
    assert!(
        reaction_client
            .next()
            .await
            .unwrap()
            .unwrap()
            .to_text()
            .unwrap()
            .contains("\"type\":\"config\"")
    );
    assert!(
        reaction_client
            .next()
            .await
            .unwrap()
            .unwrap()
            .to_text()
            .unwrap()
            .contains("\"type\":\"appearance\"")
    );
    assert!(
        reaction_client
            .next()
            .await
            .unwrap()
            .unwrap()
            .to_text()
            .unwrap()
            .contains("\"type\":\"reaction\"")
    );

    stop_server(&core).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn coordinates_split_video_and_audio_outputs() {
    let port = free_local_port();
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        port,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    start_server(core.clone()).await.unwrap();
    let secret = load_or_create_relay_secret().unwrap();

    let mut audio = connect_test_output(port, &secret, "audio").await;
    let initial_audio_clock = next_test_event(&mut audio, "mediaClock").await;
    assert_eq!(initial_audio_clock["payload"]["videoBusy"], false);
    assert_eq!(initial_audio_clock["payload"]["audioBusy"], false);

    let mut visual = connect_test_output(port, &secret, "visual").await;
    let initial_visual_clock = next_test_event(&mut visual, "mediaClock").await;
    assert_eq!(initial_visual_clock["payload"]["videoBusy"], false);
    assert_eq!(initial_visual_clock["payload"]["audioBusy"], false);

    send_test_clock(&mut visual, true).await;
    let visual_grant = next_test_event(&mut visual, "mediaGrant").await;
    assert_eq!(visual_grant["payload"]["granted"], true);
    let visual_busy = next_test_event(&mut visual, "mediaClock").await;
    assert_eq!(visual_busy["payload"]["videoBusy"], true);
    let visual_busy = next_test_event(&mut audio, "mediaClock").await;
    assert_eq!(visual_busy["payload"]["videoBusy"], true);

    send_test_clock(&mut audio, true).await;
    let audio_grant = next_test_event(&mut audio, "mediaGrant").await;
    assert_eq!(audio_grant["payload"]["granted"], false);

    core.publish_media(test_media(MediaKind::Video, "video"))
        .await;
    let video_event = next_test_event(&mut audio, "media").await;
    assert_eq!(video_event["payload"]["kind"], "video");
    let video_event = next_test_event(&mut visual, "media").await;
    assert_eq!(video_event["payload"]["kind"], "video");

    core.publish_media(test_media(MediaKind::Audio, "audio"))
        .await;
    let audio_event = next_test_event(&mut visual, "media").await;
    assert_eq!(audio_event["payload"]["kind"], "audio");
    let audio_event = next_test_event(&mut audio, "media").await;
    assert_eq!(audio_event["payload"]["kind"], "audio");

    drop(visual);
    let video_idle = next_test_event(&mut audio, "mediaClock").await;
    assert_eq!(video_idle["payload"]["videoBusy"], false);

    send_test_clock(&mut audio, true).await;
    let audio_grant = next_test_event(&mut audio, "mediaGrant").await;
    assert_eq!(audio_grant["payload"]["granted"], true);
    let audio_busy = next_test_event(&mut audio, "mediaClock").await;
    assert_eq!(audio_busy["payload"]["audioBusy"], true);

    send_test_clock(&mut audio, false).await;
    let audio_idle = next_test_event(&mut audio, "mediaClock").await;
    assert_eq!(audio_idle["payload"]["audioBusy"], false);

    stop_server(&core).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn coordinates_media_and_tts_stage_clock() {
    let port = free_local_port();
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(AppConfig {
        port,
        ..AppConfig::default()
    })
    .await
    .unwrap();
    start_server(core.clone()).await.unwrap();
    let secret = load_or_create_relay_secret().unwrap();

    let mut overlay = connect_test_output(port, &secret, "all").await;
    let overlay_stage = next_test_event(&mut overlay, "stageClock").await;
    assert_eq!(overlay_stage["payload"]["mediaBusy"], false);
    assert_eq!(overlay_stage["payload"]["musicBusy"], false);
    assert_eq!(overlay_stage["payload"]["ttsBusy"], false);

    let (mut notification, _) = tokio_tungstenite::connect_async(format!(
        "ws://127.0.0.1:{port}/ws?role=notification&source=notification&client=widget&secret={secret}"
    ))
    .await
    .unwrap();
    assert_eq!(
        next_test_event(&mut notification, "config").await["type"],
        "config"
    );
    assert_eq!(
        next_test_event(&mut notification, "appearance").await["type"],
        "appearance"
    );
    let notification_stage = next_test_event(&mut notification, "stageClock").await;
    assert_eq!(notification_stage["payload"]["mediaBusy"], false);
    assert_eq!(notification_stage["payload"]["musicBusy"], false);
    assert_eq!(notification_stage["payload"]["ttsBusy"], false);

    notification
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({ "type": "stageClock", "payload": { "lane": "tts", "busy": true } })
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let busy = next_test_event(&mut overlay, "stageClock").await;
    assert_eq!(busy["payload"]["ttsBusy"], true);
    assert_eq!(busy["payload"]["mediaBusy"], false);
    let busy = next_test_event(&mut notification, "stageClock").await;
    assert_eq!(busy["payload"]["ttsBusy"], true);

    overlay
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({ "type": "stageClock", "payload": { "lane": "media", "busy": true } })
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    // Media claim is rejected while TTS holds the exclusive stage.
    let rejected = next_test_event(&mut overlay, "stageClock").await;
    assert_eq!(rejected["payload"]["mediaBusy"], false);
    assert_eq!(rejected["payload"]["ttsBusy"], true);
    assert_eq!(rejected["payload"]["granted"], false);

    notification
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({ "type": "stageClock", "payload": { "lane": "tts", "busy": false } })
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let idle = next_test_event(&mut overlay, "stageClock").await;
    assert_eq!(idle["payload"]["ttsBusy"], false);
    let idle = next_test_event(&mut notification, "stageClock").await;
    assert_eq!(idle["payload"]["ttsBusy"], false);

    overlay
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({ "type": "stageClock", "payload": { "lane": "media", "busy": true } })
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let media_only = next_test_event(&mut overlay, "stageClock").await;
    assert_eq!(media_only["payload"]["mediaBusy"], true);
    assert_eq!(media_only["payload"]["ttsBusy"], false);
    let media_only = next_test_event(&mut notification, "stageClock").await;
    assert_eq!(media_only["payload"]["mediaBusy"], true);
    assert_eq!(media_only["payload"]["ttsBusy"], false);

    let mut peer_overlay = connect_test_output(port, &secret, "all").await;
    let peer_initial = next_test_event(&mut peer_overlay, "stageClock").await;
    assert_eq!(peer_initial["payload"]["mediaBusy"], true);
    peer_overlay
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({ "type": "stageClock", "payload": { "lane": "media", "busy": true } })
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    // A same-lane grant must wake the claimant even though the global
    // mediaBusy boolean was already true because another output owns it.
    let peer_grant = next_test_event(&mut peer_overlay, "stageClock").await;
    assert_eq!(peer_grant["payload"]["mediaBusy"], true);
    assert_eq!(peer_grant["payload"]["ttsBusy"], false);

    stop_server(&core).await;
}

type TestWebSocket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect_test_output(port: u16, secret: &str, source: &str) -> TestWebSocket {
    let (mut client, _) = tokio_tungstenite::connect_async(format!(
        "ws://127.0.0.1:{port}/ws?role=overlay&source={source}&client=obs&secret={secret}"
    ))
    .await
    .unwrap();
    assert_eq!(
        next_test_event(&mut client, "config").await["type"],
        "config"
    );
    assert_eq!(
        next_test_event(&mut client, "appearance").await["type"],
        "appearance"
    );
    client
}

async fn next_test_event(client: &mut TestWebSocket, expected_type: &str) -> serde_json::Value {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(5), client.next())
            .await
            .expect("websocket event timed out")
            .expect("websocket closed")
            .expect("websocket error");
        let event: serde_json::Value =
            serde_json::from_str(message.to_text().expect("text websocket event")).unwrap();
        if event["type"] == expected_type {
            return event;
        }
    }
}

async fn send_test_clock(client: &mut TestWebSocket, busy: bool) {
    client
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({ "type": "mediaClock", "payload": { "busy": busy } })
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
}

fn test_media(kind: MediaKind, id: &str) -> MediaEvent {
    MediaEvent {
        kind,
        url: format!("https://cdn.discordapp.com/{id}"),
        proxy_url: format!("https://media.discordapp.net/{id}"),
        filename: id.into(),
        content_type: "application/octet-stream".into(),
        artwork_id: None,
        audio_id: None,
        cached_media_id: None,
        title: None,
        artist: None,
        text: None,
        author: AuthorIdentity {
            username: "clock-test".into(),
            display_avatar_url: "https://cdn.discordapp.com/avatar.png".into(),
        },
        timestamp: 1,
        message_id: id.into(),
    }
}

fn free_local_port() -> u16 {
    std::net::TcpListener::bind((HOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn access_query(source: Option<&str>, client: Option<&str>) -> AccessQuery {
    AccessQuery {
        role: None,
        secret: None,
        token: None,
        source: source.map(str::to_owned),
        client: client.map(str::to_owned),
    }
}

#[test]
fn serves_cached_video_byte_ranges_inline() {
    let response = ranged_media_response(
        CachedMedia {
            id: "gif".into(),
            content_type: "video/mp4".into(),
            bytes: axum::body::Bytes::from_static(b"0123456789"),
        },
        Some(&HeaderValue::from_static("bytes=2-5")),
    );
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes 2-5/10");
    assert_eq!(response.headers()[header::CONTENT_TYPE], "video/mp4");
    assert!(
        response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .is_none()
    );
}

fn http_status(port: u16, path: &str) -> String {
    http_response(port, path)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn http_response(port: u16, path: &str) -> String {
    http_response_with_host(port, path, HOST)
}

fn http_response_with_host(port: u16, path: &str, hostname: &str) -> String {
    let mut stream = std::net::TcpStream::connect((HOST, port)).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {hostname}:{port}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}
