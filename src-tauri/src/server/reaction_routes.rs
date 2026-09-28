use super::*;
pub(super) async fn library_asset(
    Path(id): Path<String>,
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
    headers: HeaderMap,
) -> Response {
    if !request_secret_matches(query.secret.as_deref(), &headers, &state.relay_secret) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !crate::media_library::is_valid_id(&id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let library = state.core.media_library.clone();
    let Ok(Ok(asset)) = tokio::task::spawn_blocking(move || library.read_asset(&id)).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    library_routes::asset_response(asset, headers.get(header::RANGE))
}
pub(super) async fn page(State(state): State<RelayServerState>) -> Response {
    short_page(&state, include_str!("../../../reactions/index.html"))
}
pub(super) async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../reactions/reactions.js"),
    )
}
pub(super) async fn style() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../../../reactions/reactions.css"),
    )
}
pub(super) async fn sound(
    Path(id): Path<String>,
    State(state): State<RelayServerState>,
    Query(query): Query<AccessQuery>,
    headers: HeaderMap,
) -> Response {
    if !request_secret_matches(query.secret.as_deref(), &headers, &state.relay_secret) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !crate::reactions::valid_id(&id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let directory = state.core.data_directory.join("reactions");
    let Ok(Ok(bytes)) =
        tokio::task::spawn_blocking(move || crate::reactions::read_sound(&directory, &id)).await
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mime = if bytes.starts_with(b"RIFF") {
        "audio/wav"
    } else if bytes.starts_with(b"OggS") {
        "audio/ogg"
    } else if bytes.starts_with(b"fLaC") {
        "audio/flac"
    } else if bytes.get(4..8) == Some(b"ftyp") {
        "audio/mp4"
    } else if bytes.first() == Some(&0xff) && bytes.get(1).is_some_and(|byte| byte & 0xf6 == 0xf0) {
        "audio/aac"
    } else {
        "audio/mpeg"
    };
    (
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response()
}
