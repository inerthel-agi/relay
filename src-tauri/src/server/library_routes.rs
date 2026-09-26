use axum::{
    body::Body,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::media_library::{MediaLibraryAsset, is_valid_id};

/// Library URLs contain generated IDs only.  The server should authenticate
/// the request before calling [`asset_response`].
pub(super) fn valid_asset_id(id: &str) -> bool {
    is_valid_id(id)
}

/// Builds the same range-capable private response used by the in-memory media
/// cache.  Keeping range support matters for Browser Source video playback.
pub(super) fn asset_response(asset: MediaLibraryAsset, range: Option<&HeaderValue>) -> Response {
    let total = asset.bytes.len();
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
            asset.bytes[start..=end].to_vec(),
            Some(format!("bytes {start}-{end}/{total}")),
        )
    } else {
        (StatusCode::OK, asset.bytes, None)
    };
    let mut response = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, asset.content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(header::CONTENT_LENGTH, body.len());
    if let Some(content_range) = content_range {
        response = response.header(header::CONTENT_RANGE, content_range);
    }
    match response.body(Body::from(body)) {
        Ok(response) => response,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
