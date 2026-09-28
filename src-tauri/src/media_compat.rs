use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use tokio::sync::Semaphore;

use crate::artwork;

const VIDEO_PROBE_BYTES: usize = 256 * 1024;
const MAX_VIDEO_INPUT_BYTES: usize = 50 * 1024 * 1024;
const MAX_VIDEO_OUTPUT_BYTES: usize = 60 * 1024 * 1024;
const TRANSCODE_TIMEOUT: Duration = Duration::from_secs(120);
static TRANSCODE_SLOT: Semaphore = Semaphore::const_new(1);

pub enum VideoCompatibility {
    Unchanged,
    Transcoded(Vec<u8>),
    HevcFallback,
}

pub async fn make_webview_compatible(
    url: &str,
    proxy_url: &str,
    filename: &str,
    content_type: &str,
) -> VideoCompatibility {
    if !is_mp4_candidate(filename, content_type) {
        return VideoCompatibility::Unchanged;
    }
    let Some(probe) = download_probe(url, proxy_url).await else {
        return VideoCompatibility::Unchanged;
    };
    if !is_hevc_mp4(&probe) {
        return VideoCompatibility::Unchanged;
    }
    let Ok(_permit) = TRANSCODE_SLOT.acquire().await else {
        return VideoCompatibility::HevcFallback;
    };
    let Some(input) = download_video(url, proxy_url).await else {
        return VideoCompatibility::HevcFallback;
    };
    transcode_video_bytes(input).await
}

/// Applies the same HEVC detection, size limits, transcode semaphore, and
/// fallback behavior to bytes that already belong to a trusted local asset.
/// The caller is responsible for obtaining those bytes from managed storage;
/// this function never opens or downloads a path or URL.
pub async fn make_webview_compatible_from_bytes(
    bytes: Vec<u8>,
    filename: &str,
    content_type: &str,
) -> VideoCompatibility {
    if !is_mp4_candidate(filename, content_type) {
        return VideoCompatibility::Unchanged;
    }
    if !is_hevc_mp4(&bytes) {
        return VideoCompatibility::Unchanged;
    }
    if bytes.len() > MAX_VIDEO_INPUT_BYTES {
        return VideoCompatibility::HevcFallback;
    }
    let Ok(_permit) = TRANSCODE_SLOT.acquire().await else {
        return VideoCompatibility::HevcFallback;
    };
    transcode_video_bytes(bytes).await
}

async fn transcode_video_bytes(input: Vec<u8>) -> VideoCompatibility {
    match tauri::async_runtime::spawn_blocking(move || transcode_to_h264(input)).await {
        Ok(Ok(output)) => VideoCompatibility::Transcoded(output),
        _ => VideoCompatibility::HevcFallback,
    }
}

async fn download_probe(url: &str, proxy_url: &str) -> Option<Vec<u8>> {
    match artwork::download_media_probe(url, VIDEO_PROBE_BYTES).await {
        Ok(bytes) => Some(bytes),
        Err(_) if !proxy_url.is_empty() && proxy_url != url => {
            artwork::download_media_probe(proxy_url, VIDEO_PROBE_BYTES)
                .await
                .ok()
        }
        Err(_) => None,
    }
}

async fn download_video(url: &str, proxy_url: &str) -> Option<Vec<u8>> {
    match artwork::download_video_bounded(url, MAX_VIDEO_INPUT_BYTES).await {
        Ok(bytes) => Some(bytes),
        Err(_) if !proxy_url.is_empty() && proxy_url != url => {
            artwork::download_video_bounded(proxy_url, MAX_VIDEO_INPUT_BYTES)
                .await
                .ok()
        }
        Err(_) => None,
    }
}

fn is_mp4_candidate(filename: &str, content_type: &str) -> bool {
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if matches!(mime.as_str(), "video/mp4" | "video/quicktime") {
        return true;
    }
    Path::new(filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "mp4" | "m4v" | "mov"
            )
        })
}

fn is_hevc_mp4(bytes: &[u8]) -> bool {
    is_mp4(bytes) && (contains_tag(bytes, b"hvc1") || contains_tag(bytes, b"hev1"))
}

fn is_mp4(bytes: &[u8]) -> bool {
    bytes.get(4..8) == Some(b"ftyp")
}

fn contains_tag(bytes: &[u8], tag: &[u8; 4]) -> bool {
    bytes.windows(tag.len()).any(|window| window == tag)
}

fn transcode_to_h264(input: Vec<u8>) -> Result<Vec<u8>> {
    if input.len() > MAX_VIDEO_INPUT_BYTES || !is_hevc_mp4(&input) {
        bail!("invalid HEVC input");
    }
    let temporary = TemporaryDirectory::create()?;
    let input_path = temporary.path().join("input.mp4");
    let output_path = temporary.path().join("output.mp4");
    fs::write(&input_path, input).context("failed to stage local video")?;

    // Same lookup as audio trimming: PATH, then a WinGet install that an
    // already-running Explorer PATH may not show yet.
    let ffmpeg = crate::reaction_trim::ffmpeg_executable().unwrap_or_else(|| "ffmpeg".into());
    let mut command = Command::new(ffmpeg);
    command
        .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&input_path)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "0:a:0?",
            "-map_metadata",
            "-1",
            "-map_chapters",
            "-1",
            "-sn",
            "-dn",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "22",
            "-pix_fmt",
            "yuv420p",
            "-threads",
            "2",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-movflags",
            "+faststart",
        ])
        .arg("-fs")
        .arg(MAX_VIDEO_OUTPUT_BYTES.to_string())
        .arg(&output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::hidden_process::hide_window(&mut command);
    let mut child = command.spawn().context("failed to start local FFmpeg")?;
    let deadline = Instant::now() + TRANSCODE_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().context("failed to monitor local FFmpeg")? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("local FFmpeg timed out");
        }
        thread::sleep(Duration::from_millis(100));
    };
    if !status.success() {
        bail!("local FFmpeg rejected the video");
    }
    let output_size = fs::metadata(&output_path)
        .context("missing local FFmpeg output")?
        .len() as usize;
    if output_size == 0 || output_size >= MAX_VIDEO_OUTPUT_BYTES {
        bail!("local FFmpeg output exceeds the cache limit");
    }
    let output = fs::read(output_path).context("failed to read local FFmpeg output")?;
    if !is_mp4(&output) || !contains_tag(&output, b"avc1") {
        bail!("local FFmpeg output is not H.264 MP4");
    }
    Ok(output)
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn create() -> Result<Self> {
        for _ in 0..8 {
            let path =
                env::temp_dir().join(format!("relay-transcode-{:016x}", rand::random::<u64>()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error).context("failed to create local media workspace"),
            }
        }
        bail!("failed to allocate local media workspace")
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests;
