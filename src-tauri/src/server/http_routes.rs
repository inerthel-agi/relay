use super::*;

pub(super) async fn root() -> impl IntoResponse {
    (StatusCode::FOUND, [(header::LOCATION, "/overlay")])
}

pub(super) async fn health() -> impl IntoResponse {
    axum::Json(json!({
        "status": "ok",
    }))
}

pub(super) async fn overlay(
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
) -> Response {
    if !secret_matches(query.secret.as_deref(), &state.relay_secret) {
        return (StatusCode::UNAUTHORIZED, "Invalid relay secret.").into_response();
    }
    Html(OVERLAY_HTML).into_response()
}

pub(super) async fn visual_overlay(State(state): State<RelayServerState>) -> Response {
    short_overlay(&state, "visual")
}

pub(super) async fn audio_overlay(State(state): State<RelayServerState>) -> Response {
    short_overlay(&state, "audio")
}

pub(super) async fn youtube_overlay(
    State(state): State<RelayServerState>,
    headers: HeaderMap,
) -> Response {
    // Existing OBS sources may still point at 127.0.0.1; YouTube rejects that Referer.
    if let Some(response) = redirect_path_off_loopback_ip(&headers, "/youtube") {
        return response;
    }
    short_overlay(&state, "youtube")
}

pub(super) async fn obs_visual_page(
    State(state): State<RelayServerState>,
    headers: HeaderMap,
) -> Response {
    // Composite embeds /youtube; parent must also be localhost for a valid Referer chain.
    if let Some(response) = redirect_path_off_loopback_ip(&headers, "/obs/visual") {
        return response;
    }
    short_page(&state, OBS_VISUAL_HTML)
}

pub(super) async fn obs_audio_page(State(state): State<RelayServerState>) -> Response {
    short_page(&state, OBS_AUDIO_HTML)
}

pub(super) async fn obs_visual_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        OBS_VISUAL_CSS,
    )
}

pub(super) async fn obs_audio_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        OBS_AUDIO_CSS,
    )
}

/// YouTube error 150 rejects embeds whose page Referer is `http://127.0.0.1`.
/// Serve YouTube-bearing pages only via `http://localhost` (same loopback, accepted Referer).
pub(super) fn redirect_path_off_loopback_ip(headers: &HeaderMap, path: &str) -> Option<Response> {
    let host = headers.get(header::HOST)?.to_str().ok()?;
    let (hostname, port) = match host.split_once(':') {
        Some((hostname, port)) => (hostname, Some(port)),
        None => (host, None),
    };
    if !hostname.eq_ignore_ascii_case("127.0.0.1") {
        return None;
    }
    let embed_host = crate::widget::youtube_embed_host();
    let location = match port.filter(|value| !value.is_empty()) {
        Some(port) => format!("http://{embed_host}:{port}{path}"),
        None => format!("http://{embed_host}{path}"),
    };
    Some(
        (
            StatusCode::TEMPORARY_REDIRECT,
            [(header::LOCATION, location)],
        )
            .into_response(),
    )
}

pub(super) fn short_overlay(state: &RelayServerState, mode: &str) -> Response {
    let html = OVERLAY_HTML.replace(
        "<meta name=\"relay-mode\" content=\"all\">",
        &format!("<meta name=\"relay-mode\" content=\"{mode}\">"),
    );
    short_page(state, html)
}

/// Short OBS URLs have no query secret. The page embeds the secret in a meta
/// tag so overlay JS can authenticate WS/media requests without a host-wide
/// cookie (cookies on 127.0.0.1 are not port-scoped). A Max-Age=0 Set-Cookie
/// expires any leftover `relay_secret` cookie from earlier builds.
pub(super) fn short_page(state: &RelayServerState, html: impl AsRef<str>) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(
            header::SET_COOKIE,
            "relay_secret=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        )
        .body(inject_relay_secret(html.as_ref(), &state.relay_secret).into())
        .expect("valid short overlay response")
}

pub(super) fn inject_relay_secret(html: &str, secret: &str) -> String {
    if !secret.bytes().all(|byte| byte.is_ascii_hexdigit())
        || html.contains("name=\"relay-secret\"")
    {
        return html.to_owned();
    }
    let meta = format!("<meta name=\"relay-secret\" content=\"{secret}\">");
    if let Some(index) = html.find("</head>") {
        let mut injected = String::with_capacity(html.len() + meta.len());
        injected.push_str(&html[..index]);
        injected.push_str(&meta);
        injected.push_str(&html[index..]);
        injected
    } else {
        format!("{meta}{html}")
    }
}

pub(super) async fn notification_sound(
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
    headers: HeaderMap,
) -> Response {
    if !request_secret_matches(query.secret.as_deref(), &headers, &state.relay_secret) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(path) = state
        .core
        .config
        .read()
        .await
        .notification_sound_path
        .clone()
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let bytes = tokio::task::spawn_blocking(move || -> anyhow::Result<(Vec<u8>, String)> {
        let metadata = std::fs::metadata(&path)?;
        if metadata.len() > crate::commands::NOTIFICATION_SOUND_MAX_BYTES {
            anyhow::bail!("notification sound exceeds the size limit");
        }
        let bytes = std::fs::read(&path)?;
        let extension = std::path::Path::new(&path)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let content_type = match extension.as_str() {
            "mp3" => "audio/mpeg",
            "flac" => "audio/flac",
            "wav" => "audio/wav",
            "ogg" | "oga" | "opus" => "audio/ogg",
            "m4a" => "audio/mp4",
            "aac" => "audio/aac",
            "webm" | "weba" => "audio/webm",
            _ => "application/octet-stream",
        };
        Ok((bytes, content_type.to_owned()))
    })
    .await;
    let Ok(Ok((bytes, content_type))) = bytes else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from(bytes))
        .expect("valid notification sound response")
}

pub(super) async fn overlay_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        OVERLAY_CSS,
    )
}

pub(super) async fn output_sample(Path(sample): Path<String>) -> Response {
    let (content_type, bytes): (&str, &[u8]) = match sample.as_str() {
        "portrait" => (
            "image/png",
            include_bytes!("../../../outputs/samples/portrait.png"),
        ),
        "landscape" => (
            "image/png",
            include_bytes!("../../../outputs/samples/landscape.png"),
        ),
        "gif" => (
            "image/gif",
            include_bytes!("../../../outputs/samples/motion.gif"),
        ),
        "video" => (
            "video/webm",
            include_bytes!("../../../outputs/samples/motion.webm"),
        ),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    ([(header::CONTENT_TYPE, content_type)], bytes).into_response()
}

pub(super) async fn output_layout() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../outputs/layout.js"),
    )
}

pub(super) async fn overlay_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        OVERLAY_JS,
    )
}

pub(super) async fn radar_png() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/png")], RADAR_PNG)
}

pub(super) async fn notifications_page(
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
) -> Response {
    if query.secret.is_some() && !secret_matches(query.secret.as_deref(), &state.relay_secret) {
        return (StatusCode::UNAUTHORIZED, "Invalid relay secret.").into_response();
    }
    short_page(&state, NOTIFICATIONS_HTML)
}

pub(super) async fn notifications_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        NOTIFICATIONS_CSS,
    )
}

pub(super) async fn notifications_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        NOTIFICATIONS_JS,
    )
}

pub(super) async fn stickers_page(
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
) -> Response {
    if query.secret.is_some() && !secret_matches(query.secret.as_deref(), &state.relay_secret) {
        return (StatusCode::UNAUTHORIZED, "Invalid relay secret.").into_response();
    }
    short_page(&state, STICKERS_HTML)
}

pub(super) async fn stickers_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STICKERS_CSS,
    )
}

pub(super) async fn stickers_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        STICKERS_JS,
    )
}

pub(super) async fn tts_audio(
    Path(id): Path<String>,
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
    headers: HeaderMap,
) -> Response {
    if !request_secret_matches(query.secret.as_deref(), &headers, &state.relay_secret)
        || id.is_empty()
        || id.len() > 20
        || !id.chars().all(|character| character.is_ascii_digit())
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let audio = state
        .core
        .tts_audio
        .read()
        .await
        .iter()
        .find(|item| item.id == id)
        .cloned();
    let Some(audio) = audio else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Response::builder()
        .header(header::CONTENT_TYPE, audio.content_type)
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from(audio.bytes))
        .expect("valid TTS audio response")
}

pub(super) async fn media_artwork(
    Path(id): Path<String>,
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
    headers: HeaderMap,
) -> Response {
    if !request_secret_matches(query.secret.as_deref(), &headers, &state.relay_secret)
        || id.is_empty()
        || id.len() > 20
        || !id.chars().all(|character| character.is_ascii_digit())
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let artwork = state
        .core
        .media_artwork
        .read()
        .await
        .iter()
        .find(|item| item.id == id)
        .cloned();
    let Some(artwork) = artwork else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Response::builder()
        .header(header::CONTENT_TYPE, artwork.content_type)
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from(artwork.bytes))
        .expect("valid media artwork response")
}

pub(super) async fn media_audio(
    Path(id): Path<String>,
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
    headers: HeaderMap,
) -> Response {
    if !request_secret_matches(query.secret.as_deref(), &headers, &state.relay_secret)
        || id.is_empty()
        || id.len() > 20
        || !id.chars().all(|character| character.is_ascii_digit())
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let audio = state
        .core
        .media_audio
        .read()
        .await
        .iter()
        .find(|item| item.id == id)
        .cloned();
    let Some(audio) = audio else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Response::builder()
        .header(header::CONTENT_TYPE, audio.content_type)
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from(audio.bytes))
        .expect("valid media audio response")
}

pub(super) async fn cached_media(
    Path(id): Path<String>,
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
    headers: HeaderMap,
) -> Response {
    let authorized = request_secret_matches(query.secret.as_deref(), &headers, &state.relay_secret)
        || secret_matches(query.token.as_deref(), &state.core.panel_token);
    if !authorized
        || id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let media = state
        .core
        .cached_media
        .read()
        .await
        .iter()
        .find(|item| item.id == id)
        .cloned();
    let Some(media) = media else {
        return StatusCode::NOT_FOUND.into_response();
    };
    ranged_media_response(media, headers.get(header::RANGE))
}

pub(super) fn ranged_media_response(
    media: crate::state::CachedMedia,
    range: Option<&HeaderValue>,
) -> Response {
    let total = media.bytes.len();
    let requested = range
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("bytes="))
        .and_then(|value| value.split_once('-'))
        .and_then(|(start, end)| {
            let start = start.parse::<usize>().ok()?;
            let end = if end.is_empty() {
                total.checked_sub(1)?
            } else {
                end.parse::<usize>().ok()?.min(total.checked_sub(1)?)
            };
            (start <= end && start < total).then_some((start, end))
        });
    let (status, body, content_range) = if let Some((start, end)) = requested {
        (
            StatusCode::PARTIAL_CONTENT,
            media.bytes.slice(start..=end),
            Some(format!("bytes {start}-{end}/{total}")),
        )
    } else {
        (StatusCode::OK, media.bytes, None)
    };
    let mut response = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, media.content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(header::CONTENT_LENGTH, body.len());
    if let Some(content_range) = content_range {
        response = response.header(header::CONTENT_RANGE, content_range);
    }
    response
        .body(Body::from(body))
        .expect("valid cached media response")
}

pub(super) async fn audio_card_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../../../overlay/audio-card.css"),
    )
}
