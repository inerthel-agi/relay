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
/// Frames checked per GIF or video by the AI image check.
pub const CHECKED_FRAMES: usize = 3;
const FRAMES_TIMEOUT: Duration = Duration::from_secs(30);
static FRAMES_SLOT: Semaphore = Semaphore::const_new(1);
/// About 2 to 3 minutes of sound with the small model on a recent processor.
const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(120);
static TRANSCRIBE_SLOT: Semaphore = Semaphore::const_new(1);
/// Written by FFmpeg in the transcription's own folder, read, then deleted with it.
const TRANSCRIPT_FILE: &str = "transcript.txt";

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

/// Downloads a GIF or video from Discord and returns three still frames
/// (start, one third, two thirds), or every frame of a media that has fewer
/// (a still GIF), as PNG, at most 1280 pixels wide. `None` when only part of
/// the media could be sampled.
pub async fn frames_from_url(url: &str, proxy_url: &str) -> Option<Vec<Vec<u8>>> {
    let input = download_video(url, proxy_url).await?;
    frames_from_bytes(input).await
}

/// Same frames from bytes already in memory (an animated sticker).
pub async fn frames_from_bytes(input: Vec<u8>) -> Option<Vec<Vec<u8>>> {
    if input.is_empty() || input.len() > MAX_VIDEO_INPUT_BYTES {
        return None;
    }
    let _permit = FRAMES_SLOT.acquire().await.ok()?;
    tauri::async_runtime::spawn_blocking(move || extract_frames(input))
        .await
        .ok()?
        .ok()
        .filter(|frames| !frames.is_empty())
}

/// Speech of an audio file or of a video's sound, transcribed on this PC by
/// FFmpeg's whisper filter: processor only, low priority, one at a time.
/// `None` when the media has no sound track.
pub async fn transcribe(
    input: Vec<u8>,
    model_directory: PathBuf,
    model_file: &'static str,
) -> Result<Option<String>> {
    if input.is_empty() || input.len() > MAX_VIDEO_INPUT_BYTES {
        bail!("the media is empty or too large to transcribe");
    }
    in_slot(&TRANSCRIBE_SLOT, move || {
        transcribe_file(input, &model_directory, model_file)
    })
    .await?
}

/// Runs blocking work while holding `slot`. The slot stays taken until the
/// work really ends, even if the waiting task is dropped (the bot restarting
/// mid-check): blocking work cannot be cancelled, and two transcriptions would
/// share the transcript file.
async fn in_slot<T: Send + 'static>(
    slot: &'static Semaphore,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T> {
    let permit = slot.acquire().await?;
    Ok(tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await?)
}

/// Same, from Discord. The download is bounded like a video's.
pub async fn transcribe_url(
    url: &str,
    proxy_url: &str,
    model_directory: PathBuf,
    model_file: &'static str,
) -> Result<Option<String>> {
    let input = download_video(url, proxy_url)
        .await
        .context("the media could not be downloaded")?;
    transcribe(input, model_directory, model_file).await
}

/// True when the FFmpeg Relay uses was built with whisper.cpp (FFmpeg 8, full builds).
pub fn ffmpeg_has_whisper() -> bool {
    let ffmpeg = crate::reaction_trim::ffmpeg_executable().unwrap_or_else(|| "ffmpeg".into());
    let mut command = Command::new(ffmpeg);
    command
        .args(["-hide_banner", "-filters"])
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    crate::hidden_process::hide_window(&mut command);
    command.output().is_ok_and(|output| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.split_whitespace().nth(1) == Some("whisper"))
    })
}

/// Filter options have their own escaping, which a Windows path breaks: FFmpeg
/// runs in the model folder and names its files relative to it. The processor
/// only, so the graphics card stays free for the game and the stream encoder;
/// 30-second chunks, so a phone number is not cut in two.
fn whisper_filter(model_file: &str) -> String {
    format!(
        "whisper=model={model_file}:language=auto:use_gpu=false:queue=30:destination={TRANSCRIPT_FILE}:format=text"
    )
}

fn transcribe_file(
    input: Vec<u8>,
    model_directory: &Path,
    model_file: &str,
) -> Result<Option<String>> {
    let deadline = Instant::now() + TRANSCRIBE_TIMEOUT;
    let temporary = TemporaryDirectory::create()?;
    let input_path = temporary.path().join("input");
    fs::write(&input_path, input).context("failed to stage local media")?;
    let errors_path = temporary.path().join("errors.txt");
    let transcript_path = model_directory.join(TRANSCRIPT_FILE);
    let _ = fs::remove_file(&transcript_path);

    let mut command = member_media_command(&input_path);
    crate::hidden_process::hide_window_low_priority(&mut command);
    command
        .current_dir(model_directory)
        .args(["-loglevel", "error", "-vn", "-af"])
        .arg(whisper_filter(model_file))
        .args(["-f", "null", "-"])
        // A file, not a pipe: a full pipe would stall FFmpeg until the deadline.
        .stderr(fs::File::create(&errors_path)?);
    let mut child = command.spawn().context("failed to start local FFmpeg")?;
    let (succeeded, transcript) = with_transcript_file(transcript_path, |path| {
        wait_until(&mut child, deadline)?;
        let succeeded = child.wait()?.success();
        let transcript = fs::read_to_string(path).unwrap_or_default();
        Ok((succeeded, transcript))
    })?;
    if !succeeded {
        let errors = fs::read_to_string(&errors_path).unwrap_or_default();
        if errors.contains("does not contain any stream") {
            return Ok(None);
        }
        bail!("local FFmpeg could not transcribe the media");
    }
    Ok(Some(
        transcript.split_whitespace().collect::<Vec<_>>().join(" "),
    ))
}

fn ffmpeg_command() -> Command {
    let ffmpeg = crate::reaction_trim::ffmpeg_executable().unwrap_or_else(|| "ffmpeg".into());
    let mut command = Command::new(ffmpeg);
    command
        .args(["-nostdin", "-hide_banner"])
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    crate::hidden_process::hide_window(&mut command);
    command
}

/// FFmpeg reading a member's media: local files only, so a playlist or a data
/// reference inside the media can never make FFmpeg fetch a URL.
fn member_media_command(input_path: &Path) -> Command {
    let mut command = ffmpeg_command();
    command
        .args(["-protocol_whitelist", "file", "-i"])
        .arg(input_path);
    command
}

struct TranscriptFile(PathBuf);

impl TranscriptFile {
    fn new(path: PathBuf) -> Self {
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TranscriptFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn with_transcript_file<T>(path: PathBuf, operation: impl FnOnce(&Path) -> Result<T>) -> Result<T> {
    let transcript = TranscriptFile::new(path);
    let result = operation(transcript.path());
    match fs::remove_file(transcript.path()) {
        Ok(()) => result,
        Err(error) if error.kind() == io::ErrorKind::NotFound => result,
        Err(error) => Err(error).context("failed to remove temporary transcript"),
    }
}

/// Waits for FFmpeg until `deadline`, then kills it.
fn wait_until(child: &mut std::process::Child, deadline: Instant) -> Result<()> {
    loop {
        if child
            .try_wait()
            .context("failed to monitor local FFmpeg")?
            .is_some()
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("local FFmpeg timed out");
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// "Duration: 00:01:02.50" from FFmpeg's description of the input, in seconds.
fn parse_duration(description: &str) -> Option<f64> {
    let value = description.split("Duration: ").nth(1)?.split(',').next()?;
    let mut parts = value.trim().split(':');
    let hours = parts.next()?.parse::<f64>().ok()?;
    let minutes = parts.next()?.parse::<f64>().ok()?;
    let seconds = parts.next()?.parse::<f64>().ok()?;
    Some(hours * 3600.0 + minutes * 60.0 + seconds).filter(|total| *total > 0.0)
}

fn extract_frames(input: Vec<u8>) -> Result<Vec<Vec<u8>>> {
    let deadline = Instant::now() + FRAMES_TIMEOUT;
    let temporary = TemporaryDirectory::create()?;
    let input_path = temporary.path().join("input");
    fs::write(&input_path, input).context("failed to stage local media")?;

    // Without an output, FFmpeg only describes the input (and exits with an error).
    let description_path = temporary.path().join("description.txt");
    let mut probe = member_media_command(&input_path);
    // A file, not a pipe: a long description would fill the pipe and stall FFmpeg.
    probe.stderr(fs::File::create(&description_path)?);
    let mut child = probe.spawn().context("failed to start local FFmpeg")?;
    wait_until(&mut child, deadline)?;
    let description = fs::read(&description_path).unwrap_or_default();
    let step = parse_duration(&String::from_utf8_lossy(&description))
        .map_or(1.0, |duration| duration / CHECKED_FRAMES as f64);

    let sample = |filter: &str, fps_mode: &str, name: &str| {
        sample_frames(
            &input_path,
            temporary.path(),
            (filter, fps_mode, name),
            deadline,
        )
    };
    let spread = sample(
        &format!(r"select='isnan(prev_selected_t)+gte(t-prev_selected_t\,{step:.3})',{SCALE}"),
        "vfr",
        "spread",
    )?;
    if spread.len() == CHECKED_FRAMES {
        return Ok(spread);
    }
    // Fewer frames than the sample stand for the media only when it has no
    // other frame (a still GIF, a two-frame animation).
    let first = sample(SCALE, "passthrough", "first")?;
    if first.len() < CHECKED_FRAMES {
        return Ok(first);
    }
    // Uneven frame times or an unknown duration: the frames at one and two
    // thirds of the count stand in for the time spread. Anything short of
    // three is a partial check: nothing is returned and the media waits for review.
    let picked = match count_frames(&input_path, deadline).and_then(spread_indexes) {
        Some([start, third, two_thirds]) => sample(
            &format!(r"select='eq(n\,{start})+eq(n\,{third})+eq(n\,{two_thirds})',{SCALE}"),
            "vfr",
            "index",
        )?,
        None => Vec::new(),
    };
    Ok(if picked.len() == CHECKED_FRAMES {
        picked
    } else {
        Vec::new()
    })
}

/// Frame numbers at the start, one third and two thirds of `count` frames.
fn spread_indexes(count: usize) -> Option<[usize; CHECKED_FRAMES]> {
    (count >= CHECKED_FRAMES).then(|| [0, count / 3, count * 2 / 3])
}

/// Video frames of the media, counted without decoding them.
fn count_frames(input_path: &Path, deadline: Instant) -> Option<usize> {
    let mut command = member_media_command(input_path);
    command
        .args([
            "-loglevel",
            "error",
            "-map",
            "0:v:0",
            "-c",
            "copy",
            "-f",
            "null",
            "-",
            "-progress",
            "pipe:1",
            "-nostats",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn().ok()?;
    wait_until(&mut child, deadline).ok()?;
    let output = child.wait_with_output().ok()?;
    output
        .status
        .success()
        .then(|| parse_frame_count(&String::from_utf8_lossy(&output.stdout)))
        .flatten()
}

/// The last `frame=` line of FFmpeg's `-progress` report.
fn parse_frame_count(progress: &str) -> Option<usize> {
    progress
        .lines()
        .filter_map(|line| line.strip_prefix("frame="))
        .next_back()?
        .trim()
        .parse()
        .ok()
}

/// Frames wider than this are scaled down before the check.
const SCALE: &str = r"scale=w='min(1280\,iw)':h=-1";

/// Up to `CHECKED_FRAMES` frames through `filter`, as PNG files named `name1.png`…
fn sample_frames(
    input_path: &Path,
    directory: &Path,
    (filter, fps_mode, name): (&str, &str, &str),
    deadline: Instant,
) -> Result<Vec<Vec<u8>>> {
    let mut command = member_media_command(input_path);
    command
        .args([
            "-loglevel",
            "error",
            "-y",
            "-map",
            "0:v:0",
            "-an",
            "-sn",
            "-dn",
            "-vf",
            filter,
            "-fps_mode",
            fps_mode,
        ])
        .arg("-frames:v")
        .arg(CHECKED_FRAMES.to_string())
        .arg(directory.join(format!("{name}%d.png")))
        .stderr(Stdio::null());
    let mut child = command.spawn().context("failed to start local FFmpeg")?;
    wait_until(&mut child, deadline)?;
    if !child.wait()?.success() {
        bail!("local FFmpeg could not read the media");
    }
    let mut frames = Vec::new();
    for index in 1..=CHECKED_FRAMES {
        match fs::read(directory.join(format!("{name}{index}.png"))) {
            Ok(frame) => frames.push(frame),
            Err(_) => break,
        }
    }
    Ok(frames)
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

/// The only media `make_webview_compatible` may transcode.
pub(crate) fn is_mp4_candidate(filename: &str, content_type: &str) -> bool {
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

    let mut command = member_media_command(&input_path);
    command
        .args([
            "-loglevel",
            "error",
            "-y",
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
        .stderr(Stdio::null());
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
