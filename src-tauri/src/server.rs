use crate::clock::now_ms;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, Result};
use axum::{
    Router,
    body::Body,
    extract::{
        Path, Query, Request, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use subtle::ConstantTimeEq;
use tokio::{net::TcpListener, sync::oneshot};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::{
    config::{AppConfig, OutputGeometry},
    credentials::load_or_create_relay_secret,
    model::{
        AudioPlaybackState, MediaKind, MusicEndedEvent, MusicPlaybackEvent, OutputConnectionStatus,
        OutputStatuses, RelayEvent, ServerStatus, TtsEvent,
    },
    state::{AppCore, ServerRuntime},
};

/// Display-only settings sent to overlay/notification/sticker clients.
/// The full AppConfig (channel IDs, lock snapshots, widget positions) is
/// reserved for the panel role.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct OverlayConfig {
    reaction_geometry: OutputGeometry,
    port: u16,
    display_duration_ms: u64,
    gif_duration_ms: u64,
    sticker_duration_ms: u64,
    notification_duration_ms: u64,
    media_volume: u8,
    tts_queue_limit: u8,
    tts_notifications_obs_enabled: bool,
    show_author: bool,
    show_media_text_obs: bool,
    show_media_text_widget: bool,
    widget_sound_enabled: bool,
    notification_sound_enabled: bool,
    notification_sound_obs_enabled: bool,
    media_obs_geometry: OutputGeometry,
    media_widget_geometry: OutputGeometry,
    notification_obs_geometry: OutputGeometry,
    notification_widget_geometry: OutputGeometry,
}

impl From<&AppConfig> for OverlayConfig {
    fn from(config: &AppConfig) -> Self {
        Self {
            reaction_geometry: config.reactions.geometry,
            port: config.port,
            display_duration_ms: config.display_duration_ms,
            gif_duration_ms: config.gif_duration_ms,
            sticker_duration_ms: config.sticker_duration_ms,
            notification_duration_ms: config.notification_duration_ms,
            media_volume: config.media_volume,
            tts_queue_limit: config.tts_queue_limit,
            tts_notifications_obs_enabled: config.tts_notifications_obs_enabled,
            show_author: config.show_author,
            show_media_text_obs: config.show_media_text_obs,
            show_media_text_widget: config.show_media_text_widget,
            widget_sound_enabled: config.widget_sound_enabled,
            notification_sound_enabled: config.notification_sound_enabled,
            notification_sound_obs_enabled: config.notification_sound_obs_enabled,
            media_obs_geometry: config.media_obs_geometry,
            media_widget_geometry: config.media_widget_geometry,
            notification_obs_geometry: config.notification_obs_geometry,
            notification_widget_geometry: config.notification_widget_geometry,
        }
    }
}

const HOST: &str = "127.0.0.1";
const OVERLAY_HTML: &str = include_str!("../../overlay/index.html");
const OVERLAY_CSS: &str = include_str!("../../overlay/overlay.css");
const OVERLAY_JS: &str = include_str!("../../overlay/overlay.js");
const OBS_VISUAL_HTML: &str = include_str!("../../overlay/obs-visual.html");
const OBS_VISUAL_CSS: &str = include_str!("../../overlay/obs-visual.css");
const OBS_AUDIO_HTML: &str = include_str!("../../overlay/obs-audio.html");
const OBS_AUDIO_CSS: &str = include_str!("../../overlay/obs-audio.css");
const RADAR_PNG: &[u8] = include_bytes!("../../gui/assets/relay-radar.png");
const NOTIFICATIONS_HTML: &str = include_str!("../../notifications/index.html");
const NOTIFICATIONS_CSS: &str = include_str!("../../notifications/notifications.css");
const NOTIFICATIONS_JS: &str = include_str!("../../notifications/notifications.js");
const STICKERS_HTML: &str = include_str!("../../stickers/index.html");
const STICKERS_CSS: &str = include_str!("../../stickers/stickers.css");
const STICKERS_JS: &str = include_str!("../../stickers/stickers.js");

#[derive(Clone)]
struct RelayServerState {
    core: Arc<AppCore>,
    port: u16,
    relay_secret: Arc<String>,
    client_shutdown: tokio::sync::broadcast::Sender<()>,
    media_clock_tx: tokio::sync::watch::Sender<MediaClockState>,
    media_clock_counts: Arc<Mutex<MediaClockCounts>>,
    stage_clock_tx: tokio::sync::watch::Sender<StageClockState>,
    stage_clock_counts: Arc<Mutex<StageClockCounts>>,
    music_clock: Arc<Mutex<MusicClockState>>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MediaClockState {
    video_busy: bool,
    audio_busy: bool,
}

#[derive(Default)]
struct MediaClockCounts {
    video_busy: usize,
    audio_busy: usize,
}

/// Cross-output stage lock so media / YouTube and TTS never overlap.
#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StageClockState {
    media_busy: bool,
    /// Authoritative YouTube/jukebox occupancy from the server music queue.
    /// Clients must prefer this over local musicPlay/musicIdle flags so a
    /// lagged WebSocket cannot leave TTS/notifications blocked forever.
    music_busy: bool,
    tts_busy: bool,
}

#[derive(Default)]
struct StageClockCounts {
    media_busy: usize,
    tts_busy: usize,
    music_busy: bool,
}

const MUSIC_CLOCK_CACHE_LIMIT: usize = 64;

#[derive(Default)]
struct MusicClockState {
    anchors: VecDeque<(String, u64)>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum StageLane {
    Media,
    Tts,
}

#[derive(Debug, Deserialize)]
struct AccessQuery {
    role: Option<String>,
    secret: Option<String>,
    token: Option<String>,
    source: Option<String>,
    client: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputSource {
    Reaction,
    Visual,
    Audio,
    Tts,
    Notification,
    Sticker,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputClient {
    Obs,
    Widget,
    Preview,
    Probe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutputConnection {
    source: OutputSource,
    client: OutputClient,
}

impl OutputConnection {
    fn is_tracked(self) -> bool {
        !matches!(self.client, OutputClient::Probe)
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
enum OutputClientMessage {
    ReactionEnded(String),
    AudioPlayback(Box<AudioPlaybackState>),
    MusicEnded(MusicEndedEvent),
    NotificationState(Box<NotificationStateReport>),
    MediaClock(MediaClockReport),
    StageClock(StageClockReport),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NotificationStateReport {
    visible: bool,
    #[serde(default)]
    notification: Option<TtsEvent>,
    #[serde(default)]
    id: Option<String>,
}

#[derive(Deserialize)]
struct MediaClockReport {
    busy: bool,
}

#[derive(Deserialize)]
struct StageClockReport {
    lane: StageLane,
    busy: bool,
}

pub async fn start_server(core: Arc<AppCore>) -> Result<()> {
    let port = core.config.read().await.port;
    let running_port = core
        .server_runtime
        .lock()
        .await
        .as_ref()
        .map(|runtime| runtime.port);
    // When moving to a different port, bind it before stopping the running
    // server so a failed bind leaves the current server (and its clients) intact.
    let listener = if running_port == Some(port) {
        stop_server(&core).await;
        TcpListener::bind((HOST, port))
            .await
            .with_context(|| format!("failed to bind {HOST}:{port}"))?
    } else {
        let listener = TcpListener::bind((HOST, port))
            .await
            .with_context(|| format!("failed to bind {HOST}:{port}"))?;
        stop_server(&core).await;
        listener
    };
    let (client_shutdown, _) = tokio::sync::broadcast::channel(1);
    let (media_clock_tx, _) = tokio::sync::watch::channel(MediaClockState::default());
    let (stage_clock_tx, _) = tokio::sync::watch::channel(StageClockState::default());
    let state = RelayServerState {
        core: core.clone(),
        port,
        relay_secret: Arc::new(load_or_create_relay_secret()?),
        client_shutdown: client_shutdown.clone(),
        media_clock_tx,
        media_clock_counts: Arc::new(Mutex::new(MediaClockCounts::default())),
        stage_clock_tx,
        stage_clock_counts: Arc::new(Mutex::new(StageClockCounts::default())),
        music_clock: Arc::new(Mutex::new(MusicClockState::default())),
    };
    // Keep musicBusy on the stage clock in sync with the server jukebox so
    // lagged overlay/TTS/notification sockets cannot strand musicActive=true.
    let music_clock_state = state.clone();
    let mut music_events = core.relay_tx.subscribe();
    tokio::spawn(async move {
        loop {
            match music_events.recv().await {
                Ok(RelayEvent::MusicPlay(playback)) => {
                    mark_music_clock(&music_clock_state, &playback.playback_id);
                    set_stage_music_busy(&music_clock_state, true);
                    sync_stage_scheduler(&music_clock_state).await;
                }
                Ok(RelayEvent::MusicIdle) | Ok(RelayEvent::Clear) => {
                    set_stage_music_busy(&music_clock_state, false);
                    sync_stage_scheduler(&music_clock_state).await;
                }
                Ok(RelayEvent::MusicStop(_)) => {
                    // Next event is MusicPlay (still busy) or MusicIdle/Clear.
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    let busy = music_clock_state.core.current_music().await.is_some();
                    set_stage_music_busy(&music_clock_state, busy);
                    sync_stage_scheduler(&music_clock_state).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
    let router = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/reactions", get(reaction_routes::page))
        .route("/reaction-assets/reactions.js", get(reaction_routes::script))
        .route("/reaction-assets/reactions.css", get(reaction_routes::style))
        .route("/reaction-sound/{id}", get(reaction_routes::sound))
        .route("/library-asset/{id}", get(reaction_routes::library_asset))
        .route("/overlay", get(overlay))
        .route("/medias", get(visual_overlay))
        .route("/audios", get(audio_overlay))
        .route("/youtube", get(youtube_overlay))
        .route("/obs/visual", get(obs_visual_page))
        .route("/obs/audio", get(obs_audio_page))
        .route("/obs-assets/obs-visual.css", get(obs_visual_css))
        .route("/obs-assets/obs-audio.css", get(obs_audio_css))
        .route("/overlay-assets/overlay.css", get(overlay_css))
        .route("/overlay-assets/audio-card.css", get(audio_card_css))
        .route("/overlay-assets/overlay.js", get(overlay_js))
        .route("/output-layout.js", get(output_layout))
        .route("/output-samples/{sample}", get(output_sample))
        .route("/overlay-assets/relay-radar.png", get(radar_png))
        .route("/tts-audio/{id}", get(tts_audio))
        .route("/media-artwork/{id}", get(media_artwork))
        .route("/media-audio/{id}", get(media_audio))
        .route("/media-cache/{id}", get(cached_media))
        .route("/notifications", get(notifications_page))
        .route("/notification-sound", get(notification_sound))
        .route("/stickers", get(stickers_page))
        .route("/sticker-assets/stickers.css", get(stickers_css))
        .route("/sticker-assets/stickers.js", get(stickers_js))
        .route(
            "/notification-assets/notifications.css",
            get(notifications_css),
        )
        .route(
            "/notification-assets/notifications.js",
            get(notifications_js),
        )
        .route("/ws", get(websocket))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            // YouTube IFrame embeds require a referrer (error 150/153 if stripped).
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            // frame-src 'self' allows /obs/* composites to embed legacy short pages.
            HeaderValue::from_static(
                "default-src 'none'; script-src 'self' https://www.youtube.com https://www.youtube-nocookie.com; style-src 'self'; img-src 'self' https://cdn.discordapp.com https://media.discordapp.net https://*.discordapp.net https://*.klipy.com https://media.tenor.com https://i.ytimg.com data:; media-src 'self' https://cdn.discordapp.com https://media.discordapp.net https://*.discordapp.net https://*.klipy.com https://media.tenor.com; connect-src 'self' ws://127.0.0.1:* ws://localhost:* https://www.youtube.com https://www.youtube-nocookie.com https://*.googlevideo.com https://youtubei.googleapis.com; frame-src 'self' https://www.youtube.com https://www.youtube-nocookie.com; frame-ancestors 'self' tauri://localhost http://tauri.localhost",
            ),
        ))
        .layer(middleware::from_fn(move |request: Request, next: Next| async move {
            if !host_allowed(request.headers(), port) {
                return StatusCode::FORBIDDEN.into_response();
            }
            next.run(request).await
        }))
        .with_state(state);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let status_core = core.clone();
    let task = tokio::spawn(async move {
        *status_core.server_status.write().await = ServerStatus {
            connected: true,
            overlay_clients: 0,
            outputs: OutputStatuses::default(),
            error: None,
        };
        let result = axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
        if let Err(error) = result {
            status_core.server_status.write().await.error = Some(error.to_string());
        }
        status_core.server_status.write().await.connected = false;
    });
    *core.server_runtime.lock().await = Some(ServerRuntime {
        shutdown: shutdown_tx,
        client_shutdown,
        task,
        port,
    });
    Ok(())
}

pub async fn stop_server(core: &Arc<AppCore>) {
    if let Some(runtime) = core.server_runtime.lock().await.take() {
        let _ = runtime.client_shutdown.send(());
        let _ = runtime.shutdown.send(());
        let mut task = runtime.task;
        if tokio::time::timeout(Duration::from_secs(3), &mut task)
            .await
            .is_err()
        {
            task.abort();
        }
    }
    *core.server_status.write().await = ServerStatus::default();
}

mod http_routes;
mod library_routes;
mod reaction_routes;
use http_routes::*;

async fn websocket(
    upgrade: WebSocketUpgrade,
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
    headers: HeaderMap,
) -> Response {
    if !origin_allowed(headers.get(header::ORIGIN), state.port) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let role = query.role.as_deref().unwrap_or("overlay");
    let authorized = match role {
        "overlay" | "tts" | "notification" | "sticker" | "reaction" => {
            request_secret_matches(query.secret.as_deref(), &headers, &state.relay_secret)
        }
        "panel" => secret_matches(query.token.as_deref(), &state.core.panel_token),
        _ => false,
    };
    if !authorized {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let output = match role {
        "overlay" | "tts" | "notification" | "sticker" | "reaction" => {
            let Some(output) = output_connection(role, &query) else {
                return StatusCode::BAD_REQUEST.into_response();
            };
            Some(output)
        }
        "panel" => None,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    upgrade.on_upgrade(move |socket| handle_socket(socket, state, output))
}

async fn handle_socket(
    socket: WebSocket,
    state: RelayServerState,
    output: Option<OutputConnection>,
) {
    let is_output = output.is_some();
    let tracked_clock_source = match output {
        Some(OutputConnection {
            source: OutputSource::Visual,
            client: OutputClient::Obs,
        }) => Some(OutputSource::Visual),
        Some(OutputConnection {
            source: OutputSource::Audio,
            client: OutputClient::Obs,
        }) => Some(OutputSource::Audio),
        _ => None,
    };
    let receives_music = output_receives_music(output);
    let receives_media_clock = tracked_clock_source.is_some();
    let receives_stage_clock = output_receives_stage_clock(output);
    let reports_stage_clock = output_reports_stage_clock(output);
    let mut reported_busy = false;
    let mut reported_stage_lane: Option<StageLane> = None;
    let mut relay_rx = state.core.relay_tx.subscribe();
    let mut media_clock_rx = state.media_clock_tx.subscribe();
    let mut stage_clock_rx = state.stage_clock_tx.subscribe();
    if let Some(output) = output {
        update_output_connection(&state, output, 1).await;
    }
    let (mut sender, mut receiver) = socket.split();
    let config = state.core.config.read().await.clone();
    let config_payload = if is_output {
        json!(OverlayConfig::from(&config))
    } else {
        json!(config)
    };
    if send_json(
        &mut sender,
        &json!({ "type": "config", "payload": config_payload }),
    )
    .await
    .is_err()
    {
        if let Some(output) = output {
            update_output_connection(&state, output, -1).await;
        }
        return;
    }
    let appearance = state.core.interface_preferences.read().await.clone();
    if send_json(
        &mut sender,
        &json!({ "type": "appearance", "payload": appearance }),
    )
    .await
    .is_err()
    {
        if let Some(output) = output {
            update_output_connection(&state, output, -1).await;
        }
        return;
    }
    if !is_output {
        let history = state.core.history.read().await.clone();
        let _ = send_json(
            &mut sender,
            &json!({ "type": "history", "payload": history }),
        )
        .await;
    } else if receives_media_clock
        && send_json(
            &mut sender,
            &json!({ "type": "mediaClock", "payload": *media_clock_rx.borrow_and_update() }),
        )
        .await
        .is_err()
    {
        if let Some(output) = output {
            update_output_connection(&state, output, -1).await;
        }
        return;
    }

    if receives_stage_clock
        && send_json(
            &mut sender,
            &json!({ "type": "stageClock", "payload": *stage_clock_rx.borrow_and_update() }),
        )
        .await
        .is_err()
    {
        if let Some(output) = output {
            update_output_connection(&state, output, -1).await;
        }
        return;
    }

    if receives_music
        && let Some(playback) = state.core.current_music().await
        && let Some(message) = music_play_message(&state, playback, false)
        && send_json(&mut sender, &message).await.is_err()
    {
        if let Some(output) = output {
            update_output_connection(&state, output, -1).await;
        }
        return;
    }

    let mut shutdown_rx = state.client_shutdown.subscribe();
    let active_reaction = state.core.reactions.lock().await.active.clone();
    let _ = send_json(&mut sender, &RelayEvent::Reaction(active_reaction)).await;
    if output_is_notification(output)
        && send_json(
            &mut sender,
            &RelayEvent::MessagePin(state.core.message_pin_event().await),
        )
        .await
        .is_err()
    {
        if let Some(output) = output {
            update_output_connection(&state, output, -1).await;
        }
        return;
    }
    let notification_client_id = if output_is_notification(output) {
        Some(state.core.register_notification_output().await)
    } else {
        None
    };
    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                let _ = sender.send(Message::Close(None)).await;
                break;
            }
            event = relay_rx.recv() => {
                match event {
                    Ok(RelayEvent::MusicHistory(_)) if is_output => {}
                    Ok(RelayEvent::MusicPlay(playback)) if receives_music => {
                        let message = music_play_message(&state, playback, true)
                            .expect("music clock must be established for dispatched playback");
                        if send_json(&mut sender, &message).await.is_err() {
                            break;
                        }
                    }
                    Ok(RelayEvent::MusicIdle) => {
                        if send_json(&mut sender, &RelayEvent::MusicIdle).await.is_err() {
                            break;
                        }
                    }
                    Ok(RelayEvent::Clear) => {
                        if send_json(&mut sender, &RelayEvent::Clear).await.is_err() {
                            break;
                        }
                    }
                    Ok(RelayEvent::Config(config)) if is_output => {
                        let payload = json!({ "type": "config", "payload": OverlayConfig::from(config.as_ref()) });
                        if send_json(&mut sender, &payload).await.is_err() { break; }
                    }
                    Ok(event) => {
                        if send_json(&mut sender, &event).await.is_err() { break; }
                    }
                    // A lagging client only misses events; keep the socket
                    // alive instead of interrupting the media it displays.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            changed = media_clock_rx.changed(), if receives_media_clock => {
                if changed.is_err() { break; }
                let clock = *media_clock_rx.borrow_and_update();
                if send_json(&mut sender, &json!({ "type": "mediaClock", "payload": clock })).await.is_err() { break; }
            }
            changed = stage_clock_rx.changed(), if receives_stage_clock => {
                if changed.is_err() { break; }
                let clock = *stage_clock_rx.borrow_and_update();
                if send_json(&mut sender, &json!({ "type": "stageClock", "payload": clock })).await.is_err() { break; }
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if sender.send(Message::Pong(payload)).await.is_err() { break; }
                    }
                    Some(Ok(Message::Text(text))) if is_output => {
                        let Ok(message) = serde_json::from_str(text.as_str()) else {
                            let _ = sender.send(Message::Close(None)).await;
                            break;
                        };
                        match message {
                            OutputClientMessage::ReactionEnded(id) if output.is_some_and(|o| o.source == OutputSource::Reaction && matches!(o.client, OutputClient::Obs | OutputClient::Widget)) => {
                                crate::reactions::stop(&state.core, Some(&id)).await;
                            }
                            OutputClientMessage::AudioPlayback(playback)
                                if matches!(playback.media.kind, MediaKind::Audio)
                                    && matches!(playback.target.as_str(), "obs" | "widget") =>
                            {
                                let _ = state
                                    .core
                                    .relay_tx
                                    .send(RelayEvent::AudioPlayback(*playback));
                            }
                            OutputClientMessage::MusicEnded(event) if receives_music => {
                                if event.completed {
                                    let _ = state.core.finish_music(&event.playback_id).await;
                                } else {
                                    let _ = state.core.stop_music_if_current(&event.playback_id).await;
                                }
                            }
                            OutputClientMessage::NotificationState(report)
                                if let Some(client_id) = notification_client_id =>
                            {
                                let report = *report;
                                if report.visible {
                                    if let Some(notification) = report.notification {
                                        state
                                            .core
                                            .report_notification_output(client_id, notification)
                                            .await;
                                    }
                                } else {
                                    state
                                        .core
                                        .clear_notification_output(client_id, report.id.as_deref())
                                        .await;
                                }
                            }
                            OutputClientMessage::MediaClock(clock) if tracked_clock_source.is_some() => {
                                let source = tracked_clock_source.expect("tracked clock source");
                                if clock.busy {
                                    let granted = try_acquire_output_lease(
                                        &state,
                                        source,
                                        &mut reported_busy,
                                    );
                                    let clock = media_clock_state(&state);
                                    if send_json(
                                        &mut sender,
                                        &json!({
                                            "type": "mediaGrant",
                                            "payload": { "granted": granted, "clock": clock },
                                        }),
                                    )
                                    .await
                                    .is_err()
                                    {
                                        break;
                                    }
                                } else {
                                    release_output_lease(&state, source, &mut reported_busy);
                                }
                            }
                            OutputClientMessage::StageClock(report) if reports_stage_clock => {
                                let granted = apply_stage_clock_report(
                                    &state,
                                    report.lane,
                                    report.busy,
                                    &mut reported_stage_lane,
                                );
                                sync_stage_scheduler(&state).await;
                                // Rejected busy claims do not change the watch value, so echo
                                // the clock to this claimant so pending UI can clear/retry.
                                if report.busy && !granted {
                                    let clock = stage_clock_state(&state);
                                    if send_json(
                                        &mut sender,
                                        &json!({
                                            "type": "stageClock",
                                            "payload": {
                                                "mediaBusy": clock.media_busy,
                                                "musicBusy": clock.music_busy,
                                                "ttsBusy": clock.tts_busy,
                                                "granted": false,
                                                "lane": match report.lane {
                                                    StageLane::Media => "media",
                                                    StageLane::Tts => "tts",
                                                },
                                            }
                                        }),
                                    )
                                    .await
                                    .is_err()
                                    {
                                        break;
                                    }
                                }
                            }
                            _ => {
                                let _ = sender.send(Message::Close(None)).await;
                                break;
                            }
                        }
                    }
                    Some(Ok(_)) => {
                        let _ = sender.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
        }
    }
    if let Some(source) = tracked_clock_source {
        release_output_lease(&state, source, &mut reported_busy);
    }
    if let Some(lane) = reported_stage_lane {
        apply_stage_clock_report(&state, lane, false, &mut reported_stage_lane);
        sync_stage_scheduler(&state).await;
    }
    if let Some(client_id) = notification_client_id {
        state.core.clear_notification_output(client_id, None).await;
    }
    if let Some(output) = output {
        update_output_connection(&state, output, -1).await;
    }
}

async fn send_json<T: serde::Serialize>(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    value: &T,
) -> Result<(), axum::Error> {
    let serialized = serde_json::to_string(value).expect("serializable server event");
    sender.send(Message::Text(serialized.into())).await
}

fn output_connection(role: &str, query: &AccessQuery) -> Option<OutputConnection> {
    let source = match (role, query.source.as_deref()) {
        ("reaction", None | Some("reaction")) => OutputSource::Reaction,
        ("overlay", None | Some("all")) => OutputSource::All,
        ("overlay", Some("visual")) => OutputSource::Visual,
        ("overlay", Some("audio") | Some("youtube")) => OutputSource::Audio,
        ("tts", None | Some("tts")) => OutputSource::Tts,
        ("notification", None | Some("notification")) => OutputSource::Notification,
        ("sticker", None | Some("sticker")) => OutputSource::Sticker,
        _ => return None,
    };
    let client = match query.client.as_deref().unwrap_or("obs") {
        "obs" => OutputClient::Obs,
        "widget" => OutputClient::Widget,
        "preview" => OutputClient::Preview,
        "probe" => OutputClient::Probe,
        _ => return None,
    };
    if matches!(client, OutputClient::Widget | OutputClient::Preview)
        && !matches!(
            source,
            OutputSource::Visual
                | OutputSource::Notification
                | OutputSource::All
                | OutputSource::Reaction
        )
    {
        return None;
    }
    Some(OutputConnection { source, client })
}

fn output_receives_music(output: Option<OutputConnection>) -> bool {
    matches!(
        output,
        Some(OutputConnection {
            // Visual OBS (/medias) needs MusicPlay/Idle hints so GIFs wait for
            // the jukebox even before the stageClock watch updates arrive.
            source: OutputSource::Audio
                | OutputSource::Visual
                | OutputSource::All
                | OutputSource::Notification
                | OutputSource::Tts,
            client: OutputClient::Obs | OutputClient::Widget,
        })
    )
}

fn output_is_notification(output: Option<OutputConnection>) -> bool {
    matches!(
        output,
        Some(OutputConnection {
            source: OutputSource::Notification,
            client: OutputClient::Obs | OutputClient::Widget,
        })
    )
}

fn output_receives_stage_clock(output: Option<OutputConnection>) -> bool {
    matches!(
        output,
        Some(OutputConnection {
            source: OutputSource::Visual
                | OutputSource::Audio
                | OutputSource::All
                | OutputSource::Tts
                | OutputSource::Notification
                | OutputSource::Sticker,
            client: OutputClient::Obs | OutputClient::Widget,
        })
    )
}

fn output_reports_stage_clock(output: Option<OutputConnection>) -> bool {
    output_receives_stage_clock(output)
}

async fn update_output_connection(
    state: &RelayServerState,
    connection: OutputConnection,
    delta: isize,
) {
    if !connection.is_tracked() {
        return;
    }
    let mut status = state.core.server_status.write().await;
    if matches!(connection.client, OutputClient::Obs) {
        adjust_count(&mut status.overlay_clients, delta);
    }
    for source in output_sources(connection.source) {
        let output = output_status_mut(&mut status.outputs, *source);
        let count = match connection.client {
            OutputClient::Obs => &mut output.obs_clients,
            OutputClient::Widget => &mut output.widget_clients,
            OutputClient::Preview => &mut output.preview_clients,
            OutputClient::Probe => continue,
        };
        adjust_count(count, delta);
        if delta > 0 {
            output.last_connected_at = Some(now_ms());
        }
    }
    drop(status);
    if delta < 0 && connection.source == OutputSource::Reaction {
        crate::reactions::stop_without_outputs(&state.core).await;
    }
}

fn try_acquire_output_lease(
    state: &RelayServerState,
    source: OutputSource,
    reported_busy: &mut bool,
) -> bool {
    if *reported_busy {
        return true;
    }
    let mut counts = state
        .media_clock_counts
        .lock()
        .expect("media clock mutex poisoned");
    let blocked = match source {
        OutputSource::Visual => counts.audio_busy > 0,
        OutputSource::Audio => counts.video_busy > 0,
        _ => return false,
    };
    if blocked {
        return false;
    }
    match source {
        OutputSource::Visual => counts.video_busy = counts.video_busy.saturating_add(1),
        OutputSource::Audio => counts.audio_busy = counts.audio_busy.saturating_add(1),
        _ => unreachable!("only split media outputs can acquire a lease"),
    }
    *reported_busy = true;
    drop(counts);
    broadcast_media_clock(state);
    true
}

fn release_output_lease(state: &RelayServerState, source: OutputSource, reported_busy: &mut bool) {
    if !*reported_busy {
        return;
    }
    let mut counts = state
        .media_clock_counts
        .lock()
        .expect("media clock mutex poisoned");
    match source {
        OutputSource::Visual => adjust_count(&mut counts.video_busy, -1),
        OutputSource::Audio => adjust_count(&mut counts.audio_busy, -1),
        _ => return,
    }
    *reported_busy = false;
    drop(counts);
    broadcast_media_clock(state);
}

fn media_clock_state(state: &RelayServerState) -> MediaClockState {
    let counts = state
        .media_clock_counts
        .lock()
        .expect("media clock mutex poisoned");
    MediaClockState {
        video_busy: counts.video_busy > 0,
        audio_busy: counts.audio_busy > 0,
    }
}

fn broadcast_media_clock(state: &RelayServerState) {
    let next = media_clock_state(state);
    state.media_clock_tx.send_if_modified(|current| {
        let changed = *current != next;
        *current = next;
        changed
    });
}

fn stage_clock_state(state: &RelayServerState) -> StageClockState {
    let counts = state
        .stage_clock_counts
        .lock()
        .expect("stage clock mutex poisoned");
    StageClockState {
        media_busy: counts.media_busy > 0,
        music_busy: counts.music_busy,
        tts_busy: counts.tts_busy > 0,
    }
}

async fn sync_stage_scheduler(state: &RelayServerState) {
    let clock = stage_clock_state(state);
    state
        .core
        .stage_scheduler
        .stage_state(clock.media_busy, clock.music_busy, clock.tts_busy)
        .await;
}

fn broadcast_stage_clock(state: &RelayServerState) {
    let next = stage_clock_state(state);
    // Every ownership change must wake claimants. The public booleans can stay
    // identical when OBS and the Windows widget overlap on the same media lane.
    state.stage_clock_tx.send_replace(next);
}

fn mark_music_clock(state: &RelayServerState, playback_id: &str) -> u64 {
    let mut clock = state
        .music_clock
        .lock()
        .expect("music clock mutex poisoned");
    mark_music_anchor(&mut clock, playback_id)
}

fn mark_music_anchor(clock: &mut MusicClockState, playback_id: &str) -> u64 {
    if let Some((_, started_at_ms)) = clock
        .anchors
        .iter()
        .find(|(known_id, _)| known_id == playback_id)
    {
        return *started_at_ms;
    }
    let started_at_ms = now_ms();
    if clock.anchors.len() >= MUSIC_CLOCK_CACHE_LIMIT {
        clock.anchors.pop_front();
    }
    clock
        .anchors
        .push_back((playback_id.to_owned(), started_at_ms));
    started_at_ms
}

fn music_play_message(
    state: &RelayServerState,
    playback: MusicPlaybackEvent,
    establish_clock: bool,
) -> Option<serde_json::Value> {
    let started_at_ms = if establish_clock {
        Some(mark_music_clock(state, &playback.playback_id))
    } else {
        current_music_clock(state, &playback.playback_id)
    }?;
    music_play_payload(playback, Some(started_at_ms))
}

fn music_play_payload(
    playback: MusicPlaybackEvent,
    started_at_ms: Option<u64>,
) -> Option<serde_json::Value> {
    let started_at_ms = started_at_ms?;
    let mut payload = serde_json::to_value(playback).expect("serializable music event");
    if let serde_json::Value::Object(object) = &mut payload {
        object.insert("serverStartedAtMs".into(), json!(started_at_ms));
    }
    Some(json!({ "type": "musicPlay", "payload": payload }))
}

fn current_music_clock(state: &RelayServerState, playback_id: &str) -> Option<u64> {
    let clock = state
        .music_clock
        .lock()
        .expect("music clock mutex poisoned");
    current_music_anchor(&clock, playback_id)
}

fn current_music_anchor(clock: &MusicClockState, playback_id: &str) -> Option<u64> {
    clock
        .anchors
        .iter()
        .find(|(known_id, _)| known_id == playback_id)
        .map(|(_, started_at_ms)| *started_at_ms)
}

fn set_stage_music_busy(state: &RelayServerState, busy: bool) {
    let changed = {
        let mut counts = state
            .stage_clock_counts
            .lock()
            .expect("stage clock mutex poisoned");
        if counts.music_busy == busy {
            false
        } else {
            counts.music_busy = busy;
            true
        }
    };
    if changed {
        broadcast_stage_clock(state);
    }
}

fn apply_stage_clock_report(
    state: &RelayServerState,
    lane: StageLane,
    busy: bool,
    reported_lane: &mut Option<StageLane>,
) -> bool {
    let mut counts = state
        .stage_clock_counts
        .lock()
        .expect("stage clock mutex poisoned");
    if busy {
        if *reported_lane == Some(lane) {
            return true;
        }
        // Exclusive stage: media, YouTube, and TTS must never overlap.
        let blocked = match lane {
            StageLane::Media => counts.tts_busy > 0 || counts.music_busy,
            StageLane::Tts => counts.media_busy > 0 || counts.music_busy,
        };
        if blocked {
            return false;
        }
        if let Some(previous) = reported_lane.take() {
            match previous {
                StageLane::Media => adjust_count(&mut counts.media_busy, -1),
                StageLane::Tts => adjust_count(&mut counts.tts_busy, -1),
            }
        }
        match lane {
            StageLane::Media => counts.media_busy = counts.media_busy.saturating_add(1),
            StageLane::Tts => counts.tts_busy = counts.tts_busy.saturating_add(1),
        }
        *reported_lane = Some(lane);
    } else {
        let Some(previous) = *reported_lane else {
            return true;
        };
        if previous != lane {
            return true;
        }
        match previous {
            StageLane::Media => adjust_count(&mut counts.media_busy, -1),
            StageLane::Tts => adjust_count(&mut counts.tts_busy, -1),
        }
        *reported_lane = None;
    }
    drop(counts);
    broadcast_stage_clock(state);
    true
}

fn output_sources(source: OutputSource) -> &'static [OutputSource] {
    match source {
        OutputSource::Reaction => &[OutputSource::Reaction],
        OutputSource::All => &[OutputSource::Visual, OutputSource::Audio],
        OutputSource::Visual => &[OutputSource::Visual],
        OutputSource::Audio => &[OutputSource::Audio],
        OutputSource::Tts => &[OutputSource::Tts],
        OutputSource::Notification => &[OutputSource::Notification],
        OutputSource::Sticker => &[OutputSource::Sticker],
    }
}

fn output_status_mut(
    statuses: &mut OutputStatuses,
    source: OutputSource,
) -> &mut OutputConnectionStatus {
    match source {
        OutputSource::Reaction => &mut statuses.reaction,
        OutputSource::Visual => &mut statuses.visual,
        OutputSource::Audio => &mut statuses.audio,
        OutputSource::Tts => &mut statuses.tts,
        OutputSource::Notification => &mut statuses.notification,
        OutputSource::Sticker => &mut statuses.sticker,
        OutputSource::All => unreachable!("combined output sources are expanded before tracking"),
    }
}

fn adjust_count(count: &mut usize, delta: isize) {
    if delta > 0 {
        *count = count.saturating_add(delta as usize);
    } else {
        *count = count.saturating_sub((-delta) as usize);
    }
}

fn secret_matches(candidate: Option<&str>, expected: &str) -> bool {
    candidate.is_some_and(|candidate| {
        candidate.len() == expected.len()
            && bool::from(candidate.as_bytes().ct_eq(expected.as_bytes()))
    })
}

fn request_secret_matches(candidate: Option<&str>, headers: &HeaderMap, expected: &str) -> bool {
    secret_matches(candidate, expected)
        || secret_matches(cookie_value(headers, "relay_secret"), expected)
}

fn cookie_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn host_allowed(headers: &HeaderMap, port: u16) -> bool {
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    host.eq_ignore_ascii_case(&format!("127.0.0.1:{port}"))
        || host.eq_ignore_ascii_case(&format!("localhost:{port}"))
}

fn origin_allowed(origin: Option<&HeaderValue>, port: u16) -> bool {
    let Some(origin) = origin.and_then(|value| value.to_str().ok()) else {
        return true;
    };
    if origin == "tauri://localhost" || origin == "http://tauri.localhost" {
        return true;
    }
    let Ok(url) = reqwest::Url::parse(origin) else {
        return false;
    };
    url.scheme() == "http"
        && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
        && url.port() == Some(port)
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none()
}

#[cfg(test)]
mod tests;
