use axum::{http::HeaderValue, response::Response};

use crate::media_library::MediaLibraryAsset;

/// Serves one library file (the server authenticates the request first).
pub(super) fn asset_response(asset: MediaLibraryAsset, range: Option<&HeaderValue>) -> Response {
    super::http_routes::ranged_bytes_response(asset.content_type, asset.bytes.into(), range)
}
