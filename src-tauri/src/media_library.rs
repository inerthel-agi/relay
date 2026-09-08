//! Persistent, local copies of media that can be replayed after a restart.
//!
//! The library deliberately stores generated asset paths instead of accepting
//! paths from the UI or from an HTTP request.  The JSON manifest contains only
//! metadata and every asset is addressed by a generated, fixed-width ID.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    config::AppConfig,
    model::MediaKind,
    privacy::{self, PrivacyAction},
};

pub const MEDIA_LIBRARY_ID_LENGTH: usize = 32;
pub const MEDIA_LIBRARY_MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
pub const MEDIA_LIBRARY_MAX_VIDEO_BYTES: usize = 50 * 1024 * 1024;
pub const MEDIA_LIBRARY_MAX_ITEMS: usize = 1_000;
pub const MEDIA_LIBRARY_MAX_NAME_CHARS: usize = 120;
pub const MEDIA_LIBRARY_MAX_INDEX_BYTES: u64 = 4 * 1024 * 1024;

const MANIFEST_VERSION: u32 = 1;
const INDEX_FILE_NAME: &str = "index.json";
const ASSETS_DIRECTORY_NAME: &str = "assets";

#[derive(Clone, Debug)]
pub enum MediaLibraryError {
    InvalidId,
    InvalidName,
    UnsupportedMedia,
    InvalidMedia,
    EmptyMedia,
    TooLarge { limit: usize },
    NotFound,
    PrivacyReview,
    PrivacyBlocked,
    CorruptManifest,
    AssetUnavailable,
    TooManyItems,
    Io,
    Serialization,
    Worker,
}

impl std::fmt::Display for MediaLibraryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidId => "Invalid media library ID.",
            Self::InvalidName => "The media name is invalid.",
            Self::UnsupportedMedia => "Only images, GIFs, and videos are supported.",
            Self::InvalidMedia => "The file does not match a supported media format.",
            Self::EmptyMedia => "The media file is empty.",
            Self::TooLarge { limit } => {
                if *limit >= 50 * 1024 * 1024 {
                    "The video must stay under 50 MB."
                } else {
                    "The image or GIF must stay under 20 MB."
                }
            }
            Self::NotFound => "The media library item no longer exists.",
            Self::PrivacyReview => "The media requires privacy review before it can be saved.",
            Self::PrivacyBlocked => "The media was blocked by the local privacy scan.",
            Self::CorruptManifest => "The media library index is invalid.",
            Self::AssetUnavailable => "The saved media file is unavailable.",
            Self::TooManyItems => "The media library has reached its item limit.",
            Self::Io => "The media library could not access its files.",
            Self::Serialization => "The media library index could not be saved.",
            Self::Worker => "The media library operation could not be completed.",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for MediaLibraryError {}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaLibraryItem {
    pub id: String,
    pub name: String,
    pub kind: MediaKind,
    pub content_type: String,
    pub extension: String,
    pub size_bytes: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaLibraryUsage {
    pub item_count: usize,
    pub bytes: u64,
}

#[derive(Clone, Debug)]
pub struct MediaLibraryAsset {
    pub item: MediaLibraryItem,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ValidatedMedia {
    pub name: String,
    pub kind: MediaKind,
    pub content_type: String,
    pub extension: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MediaLibraryManifest {
    version: u32,
    #[serde(default)]
    items: Vec<MediaLibraryItem>,
}

struct LibraryState {
    items: Vec<MediaLibraryItem>,
    load_error: Option<MediaLibraryError>,
}

pub struct MediaLibrary {
    index_path: PathBuf,
    assets_path: PathBuf,
    state: Mutex<LibraryState>,
}

impl MediaLibrary {
    /// Opens a library rooted in the application's data directory.
    #[allow(dead_code)]
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, MediaLibraryError> {
        let library = Self::open_or_unavailable(root);
        let error = library
            .state
            .lock()
            .map_err(|_| MediaLibraryError::Worker)?
            .load_error
            .clone();
        error.map_or(Ok(library), Err)
    }

    /// Creates a usable manager even when an existing index is corrupt or
    /// temporarily unavailable. The error is retained and returned by every
    /// operation, so the rest of Relay can still start without overwriting the
    /// user's library data.
    pub fn open_or_unavailable(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let assets_path = root.join(ASSETS_DIRECTORY_NAME);
        let index_path = root.join(INDEX_FILE_NAME);
        let (items, load_error) = match fs::create_dir_all(&assets_path) {
            Ok(()) => match load_manifest(&index_path, &assets_path) {
                Ok(items) => (items, None),
                Err(error) => (Vec::new(), Some(error)),
            },
            Err(_) => (Vec::new(), Some(MediaLibraryError::Io)),
        };
        Self {
            index_path,
            assets_path,
            state: Mutex::new(LibraryState { items, load_error }),
        }
    }

    /// Returns the current items, newest first.
    pub fn list(&self) -> Result<Vec<MediaLibraryItem>, MediaLibraryError> {
        let state = self.state.lock().map_err(|_| MediaLibraryError::Worker)?;
        ensure_available(&state)?;
        let mut items = state.items.clone();
        items.sort_by(|left, right| {
            right
                .created_at_ms
                .cmp(&left.created_at_ms)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(items)
    }

    pub fn usage(&self) -> Result<MediaLibraryUsage, MediaLibraryError> {
        let state = self.state.lock().map_err(|_| MediaLibraryError::Worker)?;
        ensure_available(&state)?;
        Ok(MediaLibraryUsage {
            item_count: state.items.len(),
            bytes: state.items.iter().map(|item| item.size_bytes).sum(),
        })
    }

    pub fn get(&self, id: &str) -> Result<MediaLibraryItem, MediaLibraryError> {
        validate_id(id)?;
        let state = self.state.lock().map_err(|_| MediaLibraryError::Worker)?;
        ensure_available(&state)?;
        state
            .items
            .iter()
            .find(|item| item.id == id)
            .cloned()
            .ok_or(MediaLibraryError::NotFound)
    }

    /// Reads a managed asset.  The path is reconstructed from manifest data;
    /// no caller supplied path is ever opened.
    pub fn read_asset(&self, id: &str) -> Result<MediaLibraryAsset, MediaLibraryError> {
        let item = self.get(id)?;
        let path = self.asset_path(&item);
        let bytes = read_managed_asset(&path, &item)?;
        Ok(MediaLibraryAsset {
            content_type: item.content_type.clone(),
            item,
            bytes,
        })
    }

    /// Copies and validates a local file into the library.
    pub async fn import_file(
        self: &std::sync::Arc<Self>,
        path: PathBuf,
        config: Option<AppConfig>,
        privacy_text: Option<String>,
    ) -> Result<MediaLibraryItem, MediaLibraryError> {
        let (filename, kind, content_type, bytes) =
            tokio::task::spawn_blocking(move || read_local_media(path))
                .await
                .map_err(|_| MediaLibraryError::Worker)??;
        self.import_bytes(filename, kind, content_type, bytes, config, privacy_text)
            .await
    }

    /// Validates media, including the configured local privacy gate, before
    /// writing an owned copy to disk.
    pub async fn import_bytes(
        self: &std::sync::Arc<Self>,
        filename: String,
        kind: MediaKind,
        content_type: String,
        bytes: Vec<u8>,
        config: Option<AppConfig>,
        privacy_text: Option<String>,
    ) -> Result<MediaLibraryItem, MediaLibraryError> {
        let validated = validate_media_bytes(
            &filename,
            kind,
            &content_type,
            &bytes,
            config.as_ref(),
            privacy_text.as_deref(),
        )
        .await?;
        let library = std::sync::Arc::clone(self);
        tokio::task::spawn_blocking(move || library.store_bytes(validated, bytes))
            .await
            .map_err(|_| MediaLibraryError::Worker)?
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<MediaLibraryItem, MediaLibraryError> {
        validate_id(id)?;
        let mut state = self.state.lock().map_err(|_| MediaLibraryError::Worker)?;
        ensure_available(&state)?;
        let Some(index) = state.items.iter().position(|item| item.id == id) else {
            return Err(MediaLibraryError::NotFound);
        };
        let item = &state.items[index];
        let name = sanitize_name(name, &item.extension)?;
        let mut next = state.items.clone();
        next[index].name = name;
        next[index].updated_at_ms = now_millis();
        persist_manifest(&self.index_path, &next)?;
        state.items = next;
        state
            .items
            .iter()
            .find(|candidate| candidate.id == id)
            .cloned()
            .ok_or(MediaLibraryError::Worker)
    }

    pub fn delete(&self, id: &str) -> Result<(), MediaLibraryError> {
        validate_id(id)?;
        let mut state = self.state.lock().map_err(|_| MediaLibraryError::Worker)?;
        ensure_available(&state)?;
        let Some(index) = state.items.iter().position(|item| item.id == id) else {
            return Err(MediaLibraryError::NotFound);
        };
        let item = state.items[index].clone();
        let asset = self.asset_path(&item);
        let tombstone = self.tombstone_path(&item);
        let asset_was_moved = if asset.exists() {
            fs::rename(&asset, &tombstone).map_err(|_| MediaLibraryError::Io)?;
            true
        } else {
            false
        };

        let mut next = state.items.clone();
        next.remove(index);
        if let Err(error) = persist_manifest(&self.index_path, &next) {
            if asset_was_moved {
                let _ = fs::rename(&tombstone, &asset);
            }
            return Err(error);
        }
        state.items = next;
        drop(state);
        if asset_was_moved {
            let _ = fs::remove_file(tombstone);
        }
        Ok(())
    }

    fn store_bytes(
        &self,
        validated: ValidatedMedia,
        bytes: Vec<u8>,
    ) -> Result<MediaLibraryItem, MediaLibraryError> {
        let mut state = self.state.lock().map_err(|_| MediaLibraryError::Worker)?;
        ensure_available(&state)?;
        if state.items.len() >= MEDIA_LIBRARY_MAX_ITEMS {
            return Err(MediaLibraryError::TooManyItems);
        }
        let id = next_id(&state.items);
        let now = now_millis();
        let item = MediaLibraryItem {
            id: id.clone(),
            name: validated.name,
            kind: validated.kind,
            content_type: validated.content_type,
            extension: validated.extension,
            size_bytes: bytes.len() as u64,
            created_at_ms: now,
            updated_at_ms: now,
        };
        let asset = self.asset_path(&item);
        let temporary = self.asset_temp_path(&item);
        write_asset(&temporary, &asset, &bytes)?;

        let mut next = state.items.clone();
        next.push(item.clone());
        if let Err(error) = persist_manifest(&self.index_path, &next) {
            let _ = fs::remove_file(&asset);
            return Err(error);
        }
        state.items = next;
        Ok(item)
    }

    fn asset_path(&self, item: &MediaLibraryItem) -> PathBuf {
        self.assets_path
            .join(format!("{}.{}", item.id, item.extension))
    }

    fn asset_temp_path(&self, item: &MediaLibraryItem) -> PathBuf {
        self.assets_path.join(format!("{}.tmp", item.id))
    }

    fn tombstone_path(&self, item: &MediaLibraryItem) -> PathBuf {
        self.assets_path.join(format!("{}.delete", item.id))
    }
}

fn read_managed_asset(path: &Path, item: &MediaLibraryItem) -> Result<Vec<u8>, MediaLibraryError> {
    let limit = maximum_bytes(item.kind, &item.content_type);
    let metadata = fs::metadata(path).map_err(|_| MediaLibraryError::AssetUnavailable)?;
    if metadata.len() != item.size_bytes || metadata.len() > limit as u64 {
        return Err(MediaLibraryError::AssetUnavailable);
    }
    let file = File::open(path).map_err(|_| MediaLibraryError::AssetUnavailable)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| MediaLibraryError::AssetUnavailable)?;
    if bytes.len() as u64 != item.size_bytes {
        return Err(MediaLibraryError::AssetUnavailable);
    }
    Ok(bytes)
}

pub fn is_valid_id(id: &str) -> bool {
    id.len() == MEDIA_LIBRARY_ID_LENGTH
        && id.bytes().all(|byte| byte.is_ascii_hexdigit())
        && id.bytes().all(|byte| !byte.is_ascii_uppercase())
}

pub fn validate_id(id: &str) -> Result<(), MediaLibraryError> {
    is_valid_id(id)
        .then_some(())
        .ok_or(MediaLibraryError::InvalidId)
}

/// Shared validation used by imports and replays.  Callers may pass the live
/// `AppConfig`; when privacy is disabled only the existing media type and size
/// rules apply.
pub async fn validate_media_bytes(
    filename: &str,
    kind: MediaKind,
    content_type: &str,
    bytes: &[u8],
    config: Option<&AppConfig>,
    privacy_text: Option<&str>,
) -> Result<ValidatedMedia, MediaLibraryError> {
    if !is_visual_kind(kind) {
        return Err(MediaLibraryError::UnsupportedMedia);
    }
    if bytes.is_empty() {
        return Err(MediaLibraryError::EmptyMedia);
    }

    let requested_type = canonical_content_type(kind, content_type, filename)
        .ok_or(MediaLibraryError::UnsupportedMedia)?;
    let sniffed_type = sniff_content_type(bytes);
    let effective_type = sniffed_type.unwrap_or(requested_type.as_str());
    let effective_kind = effective_kind(kind, effective_type);
    if !kind_matches_content(effective_kind, effective_type)
        || (sniffed_type.is_some() && !kind_matches_content(kind, effective_type))
    {
        return Err(MediaLibraryError::InvalidMedia);
    }
    let limit = maximum_bytes(effective_kind, effective_type);
    if bytes.len() > limit {
        return Err(MediaLibraryError::TooLarge { limit });
    }
    if !signature_matches(effective_kind, effective_type, bytes) {
        return Err(MediaLibraryError::InvalidMedia);
    }
    let extension = extension_for(effective_kind, effective_type, filename)
        .ok_or(MediaLibraryError::UnsupportedMedia)?;
    let name = sanitize_name(filename, extension)?;

    if let Some(config) = config.filter(|config| privacy::privacy_rules_enabled(config)) {
        let analysis_text = combined_privacy_text(&name, privacy_text);
        let report = if effective_type.starts_with("image/") {
            privacy::analyze_image_bytes_async(bytes, Some(&analysis_text), config).await
        } else {
            privacy::classify_text(Some(&analysis_text), config)
        };
        let action = privacy::action_for(&report, config);
        if !matches!(action, PrivacyAction::Allow) {
            privacy::log_decision(&report, action);
            return Err(match action {
                PrivacyAction::Review => MediaLibraryError::PrivacyReview,
                PrivacyAction::Block => MediaLibraryError::PrivacyBlocked,
                PrivacyAction::Allow => MediaLibraryError::Worker,
            });
        }
    }

    Ok(ValidatedMedia {
        name,
        kind: effective_kind,
        content_type: effective_type.to_owned(),
        extension: extension.to_owned(),
    })
}

fn load_manifest(
    index_path: &Path,
    assets_path: &Path,
) -> Result<Vec<MediaLibraryItem>, MediaLibraryError> {
    let metadata = match fs::metadata(index_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(MediaLibraryError::Io),
    };
    if metadata.len() > MEDIA_LIBRARY_MAX_INDEX_BYTES {
        return Err(MediaLibraryError::CorruptManifest);
    }
    let bytes = fs::read(index_path).map_err(|_| MediaLibraryError::Io)?;
    let manifest: MediaLibraryManifest =
        serde_json::from_slice(&bytes).map_err(|_| MediaLibraryError::CorruptManifest)?;
    if manifest.version != MANIFEST_VERSION || manifest.items.len() > MEDIA_LIBRARY_MAX_ITEMS {
        return Err(MediaLibraryError::CorruptManifest);
    }
    let mut seen = std::collections::HashSet::with_capacity(manifest.items.len());
    for item in &manifest.items {
        validate_manifest_item(item, assets_path)?;
        if !seen.insert(item.id.clone()) {
            return Err(MediaLibraryError::CorruptManifest);
        }
    }
    Ok(manifest.items)
}

fn ensure_available(state: &LibraryState) -> Result<(), MediaLibraryError> {
    state.load_error.clone().map_or(Ok(()), Err)
}

fn validate_manifest_item(
    item: &MediaLibraryItem,
    assets_path: &Path,
) -> Result<(), MediaLibraryError> {
    validate_id(&item.id).map_err(|_| MediaLibraryError::CorruptManifest)?;
    if item.name
        != sanitize_name(&item.name, &item.extension)
            .map_err(|_| MediaLibraryError::CorruptManifest)?
        || item.name.chars().count() > MEDIA_LIBRARY_MAX_NAME_CHARS
    {
        return Err(MediaLibraryError::CorruptManifest);
    }
    if item.size_bytes == 0
        || item.size_bytes > maximum_bytes(item.kind, &item.content_type) as u64
        || extension_for(item.kind, &item.content_type, &item.name)
            .is_none_or(|extension| extension != item.extension)
    {
        return Err(MediaLibraryError::CorruptManifest);
    }
    if !kind_matches_content(item.kind, &item.content_type)
        || !allowed_extension(item.kind, &item.extension, &item.content_type)
    {
        return Err(MediaLibraryError::CorruptManifest);
    }
    let expected_prefix = format!("{}.", item.id);
    if !assets_path
        .join(format!("{}.{}", item.id, item.extension))
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(&expected_prefix))
    {
        return Err(MediaLibraryError::CorruptManifest);
    }
    Ok(())
}

fn persist_manifest(
    index_path: &Path,
    items: &[MediaLibraryItem],
) -> Result<(), MediaLibraryError> {
    let parent = index_path.parent().ok_or(MediaLibraryError::Io)?;
    fs::create_dir_all(parent).map_err(|_| MediaLibraryError::Io)?;
    let manifest = MediaLibraryManifest {
        version: MANIFEST_VERSION,
        items: items.to_vec(),
    };
    let bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|_| MediaLibraryError::Serialization)?;
    let temporary = temporary_path(index_path);
    let write_result = (|| -> Result<(), MediaLibraryError> {
        let mut file = File::create(&temporary).map_err(|_| MediaLibraryError::Io)?;
        file.write_all(&bytes).map_err(|_| MediaLibraryError::Io)?;
        file.write_all(b"\n").map_err(|_| MediaLibraryError::Io)?;
        file.sync_all().map_err(|_| MediaLibraryError::Io)?;
        drop(file);
        fs::rename(&temporary, index_path).map_err(|_| MediaLibraryError::Io)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

fn write_asset(
    temporary: &Path,
    destination: &Path,
    bytes: &[u8],
) -> Result<(), MediaLibraryError> {
    let write_result = (|| -> Result<(), MediaLibraryError> {
        let mut file = File::create(temporary).map_err(|_| MediaLibraryError::Io)?;
        file.write_all(bytes).map_err(|_| MediaLibraryError::Io)?;
        file.sync_all().map_err(|_| MediaLibraryError::Io)?;
        drop(file);
        fs::rename(temporary, destination).map_err(|_| MediaLibraryError::Io)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    write_result
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    PathBuf::from(temporary)
}

fn read_local_media(
    path: PathBuf,
) -> Result<(String, MediaKind, String, Vec<u8>), MediaLibraryError> {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(MediaLibraryError::InvalidName)?
        .to_owned();
    let (kind, content_type) =
        kind_and_type_from_filename(&filename).ok_or(MediaLibraryError::UnsupportedMedia)?;
    let limit = maximum_bytes(kind, &content_type);
    let metadata = fs::metadata(&path).map_err(|_| MediaLibraryError::Io)?;
    if !metadata.is_file() {
        return Err(MediaLibraryError::Io);
    }
    if metadata.len() > limit as u64 {
        return Err(MediaLibraryError::TooLarge { limit });
    }
    let mut file = File::open(path).map_err(|_| MediaLibraryError::Io)?;
    let mut bytes = Vec::with_capacity(metadata.len().min(limit as u64) as usize);
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| MediaLibraryError::Io)?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) > limit {
            return Err(MediaLibraryError::TooLarge { limit });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok((filename, kind, content_type, bytes))
}

fn kind_and_type_from_filename(filename: &str) -> Option<(MediaKind, String)> {
    let extension = filename
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())?;
    let result = match extension.as_str() {
        "png" => (MediaKind::Image, "image/png"),
        "jpg" | "jpeg" => (MediaKind::Image, "image/jpeg"),
        "gif" => (MediaKind::Gif, "image/gif"),
        "webp" => (MediaKind::Image, "image/webp"),
        "bmp" => (MediaKind::Image, "image/bmp"),
        "apng" => (MediaKind::Image, "image/apng"),
        "mp4" | "m4v" => (MediaKind::Video, "video/mp4"),
        "webm" => (MediaKind::Video, "video/webm"),
        "mov" => (MediaKind::Video, "video/quicktime"),
        "ogv" => (MediaKind::Video, "video/ogg"),
        _ => return None,
    };
    Some((result.0, result.1.to_owned()))
}

fn canonical_content_type(kind: MediaKind, content_type: &str, filename: &str) -> Option<String> {
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let from_mime = match mime.as_str() {
        "image/png" => Some("image/png"),
        "image/apng" => Some("image/apng"),
        "image/jpeg" | "image/jpg" => Some("image/jpeg"),
        "image/gif" => Some("image/gif"),
        "image/webp" => Some("image/webp"),
        "image/bmp" | "image/x-ms-bmp" => Some("image/bmp"),
        "video/mp4" | "video/x-m4v" => Some("video/mp4"),
        "video/webm" => Some("video/webm"),
        "video/quicktime" => Some("video/quicktime"),
        "video/ogg" => Some("video/ogg"),
        _ => None,
    };
    from_mime
        .map(str::to_owned)
        .or_else(|| kind_and_type_from_filename(filename).map(|(_, mime)| mime))
        .filter(|mime| kind_matches_content(kind, mime))
}

fn effective_kind(kind: MediaKind, content_type: &str) -> MediaKind {
    if matches!(kind, MediaKind::Image) && content_type == "image/gif" {
        MediaKind::Gif
    } else {
        kind
    }
}

fn is_visual_kind(kind: MediaKind) -> bool {
    matches!(kind, MediaKind::Image | MediaKind::Gif | MediaKind::Video)
}

fn kind_matches_content(kind: MediaKind, content_type: &str) -> bool {
    match kind {
        MediaKind::Image => content_type.starts_with("image/") && content_type != "image/gif",
        MediaKind::Gif => content_type == "image/gif" || content_type.starts_with("video/"),
        MediaKind::Video => content_type.starts_with("video/"),
        MediaKind::Audio => false,
    }
}

fn maximum_bytes(kind: MediaKind, content_type: &str) -> usize {
    if content_type.starts_with("video/") || matches!(kind, MediaKind::Video) {
        MEDIA_LIBRARY_MAX_VIDEO_BYTES
    } else {
        MEDIA_LIBRARY_MAX_IMAGE_BYTES
    }
}

fn allowed_extension(kind: MediaKind, extension: &str, content_type: &str) -> bool {
    let extension = extension.to_ascii_lowercase();
    match kind {
        MediaKind::Image => {
            ["png", "jpg", "jpeg", "webp", "bmp", "apng"].contains(&extension.as_str())
        }
        MediaKind::Gif => {
            if content_type.starts_with("video/") {
                ["mp4", "webm", "mov", "m4v", "ogv"].contains(&extension.as_str())
            } else {
                extension == "gif"
            }
        }
        MediaKind::Video => ["mp4", "webm", "mov", "m4v", "ogv"].contains(&extension.as_str()),
        MediaKind::Audio => false,
    }
}

fn extension_for(kind: MediaKind, content_type: &str, filename: &str) -> Option<&'static str> {
    let by_type = match content_type {
        "image/png" | "image/apng" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/bmp" => Some("bmp"),
        "video/mp4" => Some("mp4"),
        "video/webm" => Some("webm"),
        "video/quicktime" => Some("mov"),
        "video/ogg" => Some("ogv"),
        _ => None,
    };
    by_type.or_else(|| {
        let extension = filename
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())?;
        let allowed = allowed_extension(kind, &extension, content_type);
        allowed.then_some(match extension.as_str() {
            "jpeg" => "jpg",
            "m4v" => "mp4",
            "apng" => "png",
            "ogv" => "ogv",
            "mov" => "mov",
            "webp" => "webp",
            "bmp" => "bmp",
            "webm" => "webm",
            "gif" => "gif",
            _ => "png",
        })
    })
}

fn sniff_content_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP".as_slice()) {
        Some("image/webp")
    } else if bytes.starts_with(b"BM") {
        Some("image/bmp")
    } else if bytes.get(4..8) == Some(b"ftyp".as_slice()) {
        Some("video/mp4")
    } else if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        Some("video/webm")
    } else if bytes.starts_with(b"OggS") {
        Some("video/ogg")
    } else {
        None
    }
}

fn signature_matches(kind: MediaKind, content_type: &str, bytes: &[u8]) -> bool {
    match content_type {
        "image/png" | "image/apng" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8]),
        "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP".as_slice()),
        "image/bmp" => bytes.starts_with(b"BM"),
        "video/mp4" => bytes.get(4..8) == Some(b"ftyp".as_slice()),
        "video/webm" => bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]),
        "video/ogg" => bytes.starts_with(b"OggS"),
        _ => is_visual_kind(kind),
    }
}

fn sanitize_name(name: &str, extension: &str) -> Result<String, MediaLibraryError> {
    if extension.is_empty() || !extension.bytes().all(|byte| byte.is_ascii_lowercase()) {
        return Err(MediaLibraryError::InvalidName);
    }
    let basename = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let source_stem = basename
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(basename);
    let mut stem = source_stem
        .chars()
        .filter(|character| !character.is_control())
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            _ => character,
        })
        .collect::<String>();
    stem = stem.trim().to_owned();
    if stem.is_empty() || stem == "." || stem == ".." {
        stem = "relay-media".into();
    }
    let extension_length = extension.chars().count() + 1;
    let max_stem = MEDIA_LIBRARY_MAX_NAME_CHARS.saturating_sub(extension_length);
    stem = stem.chars().take(max_stem.max(1)).collect();
    let name = format!("{stem}.{extension}");
    if name.chars().count() > MEDIA_LIBRARY_MAX_NAME_CHARS {
        return Err(MediaLibraryError::InvalidName);
    }
    Ok(name)
}

fn combined_privacy_text(name: &str, privacy_text: Option<&str>) -> String {
    let Some(text) = privacy_text.filter(|text| !text.trim().is_empty()) else {
        return name.to_owned();
    };
    format!("{text}\n{name}")
}

fn next_id(items: &[MediaLibraryItem]) -> String {
    loop {
        let candidate = random_id();
        if !items.iter().any(|item| item.id == candidate) {
            return candidate;
        }
    }
}

fn random_id() -> String {
    static COUNTER: OnceLock<std::sync::atomic::AtomicU64> = OnceLock::new();
    let counter = COUNTER
        .get_or_init(|| std::sync::atomic::AtomicU64::new(0))
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let entropy = rand::random::<u64>();
    let mut hasher = Sha256::new();
    hasher.update(timestamp.to_le_bytes());
    hasher.update(counter.to_le_bytes());
    hasher.update(entropy.to_le_bytes());
    format!("{:x}", hasher.finalize())[..MEDIA_LIBRARY_ID_LENGTH].to_owned()
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests;
