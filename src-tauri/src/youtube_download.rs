use std::{
    env,
    ffi::{OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

const YTDLP_ASSET: &str = "yt-dlp.exe";
const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
const YTDLP_SUMS_URL: &str =
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS";
const NODE_ASSET: &str = "node.exe";
const NODE_URL: &str = "https://nodejs.org/download/release/v24.21.0/win-x64/node.exe";
const NODE_SUMS_URL: &str = "https://nodejs.org/download/release/v24.21.0/SHASUMS256.txt";
const NODE_SUMS_NAME: &str = "win-x64/node.exe";
const FFMPEG_ASSET: &str = "ffmpeg-release-essentials.zip";
const FFMPEG_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
const FFMPEG_SUMS_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip.sha256";
const CHECKSUM_LIMIT: usize = 64 * 1024;
const YTDLP_LIMIT: usize = 32 * 1024 * 1024;
const NODE_LIMIT: usize = 128 * 1024 * 1024;
const FFMPEG_LIMIT: usize = 180 * 1024 * 1024;
const MAX_VIDEO_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const HELPER_TIMEOUT: Duration = Duration::from_secs(5 * 60);

static INSTALL_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Clone, Debug)]
struct JsRuntime {
    kind: &'static str,
    path: PathBuf,
}

/// Download one YouTube video to a user-selected path.
///
/// The output is staged outside the selected path and atomically renamed only
/// after yt-dlp exits successfully.  The command receives a locally-built URL
/// containing only a validated eleven-character YouTube video ID.
pub async fn download(
    video_id: &str,
    title: &str,
    format: &str,
    tool_dir: &Path,
) -> Result<bool, String> {
    let video_url = youtube_url(video_id)?;
    let format = download_format(format)?;
    let filename = PathBuf::from(safe_title(title)).with_extension(format);
    let Some(file) = rfd::AsyncFileDialog::new()
        .add_filter(format.to_ascii_uppercase(), &[format])
        .set_file_name(filename.to_string_lossy())
        .save_file()
        .await
    else {
        return Ok(false);
    };

    let destination = file.path().to_path_buf();
    let tool_path = resolve_ytdlp(tool_dir).await?;
    let runtime = resolve_runtime(&tool_path, tool_dir).await?;
    let ffmpeg = resolve_ffmpeg(tool_dir).await?;
    let temporary = TemporaryDirectory::create("relay-youtube")?;
    let output_template = temporary.path().join("download.%(ext)s");
    let args = ytdlp_args(
        &video_url,
        &output_template,
        runtime.as_ref(),
        ffmpeg.as_deref(),
        format,
    );
    run_ytdlp(tool_path, args).await?;

    let source = find_downloaded_file(temporary.path(), format)?;
    let source_for_copy = source.clone();
    tokio::task::spawn_blocking(move || persist_output(&source_for_copy, &destination))
        .await
        .map_err(|_| "The downloaded video could not be saved.".to_string())??;
    Ok(true)
}

/// The panel chooses the format; only the two supported containers pass.
fn download_format(format: &str) -> Result<&'static str, String> {
    match format {
        "mp3" => Ok("mp3"),
        "mp4" => Ok("mp4"),
        _ => Err("Choose MP3 or MP4.".into()),
    }
}

fn youtube_url(video_id: &str) -> Result<String, String> {
    validate_video_id(video_id)?;
    Ok(format!("https://www.youtube.com/watch?v={video_id}"))
}

fn validate_video_id(video_id: &str) -> Result<(), String> {
    if video_id.len() != 11
        || !video_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("Invalid YouTube video ID.".into());
    }
    Ok(())
}

fn safe_title(title: &str) -> String {
    let mut result = title
        .chars()
        .filter(|character| !character.is_control())
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => character,
        })
        .take(120)
        .collect::<String>();
    while result.ends_with([' ', '.']) {
        result.pop();
    }
    if result.is_empty() || result == "." || result == ".." {
        result = "youtube-video".into();
    }
    if !result.to_ascii_lowercase().ends_with(".mp4") {
        result.push_str(".mp4");
    }
    result
}

fn ytdlp_args(
    video_url: &str,
    output_template: &Path,
    runtime: Option<&JsRuntime>,
    ffmpeg: Option<&Path>,
    format: &str,
) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("--ignore-config"),
        OsString::from("--no-playlist"),
        OsString::from("--no-cache-dir"),
        OsString::from("--no-warnings"),
        OsString::from("--quiet"),
        OsString::from("--no-progress"),
        OsString::from("--no-part"),
        OsString::from("--no-overwrites"),
        OsString::from("--max-filesize"),
        OsString::from("4G"),
        OsString::from("-f"),
    ];
    if format == "mp3" {
        args.extend(
            [
                "ba/b",
                "--extract-audio",
                "--audio-format",
                "mp3",
                "--audio-quality",
                "0",
            ]
            .map(OsString::from),
        );
    } else if ffmpeg.is_some() {
        args.push(OsString::from("bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]"));
    } else {
        args.push(OsString::from("b[ext=mp4]"));
    }
    if format == "mp4" {
        args.extend([
            OsString::from("--merge-output-format"),
            OsString::from("mp4"),
        ]);
    }
    if let Some(runtime) = runtime {
        args.extend([
            OsString::from("--js-runtimes"),
            OsString::from(format!("{}:{}", runtime.kind, runtime.path.display())),
        ]);
    }
    args.extend([
        OsString::from("--remote-components"),
        OsString::from("ejs:github"),
        OsString::from("--socket-timeout"),
        OsString::from("15"),
        OsString::from("--retries"),
        OsString::from("2"),
        OsString::from("--extractor-retries"),
        OsString::from("2"),
    ]);
    if let Some(ffmpeg) = ffmpeg
        && let Some(directory) = ffmpeg.parent()
    {
        args.extend([
            OsString::from("--ffmpeg-location"),
            directory.as_os_str().to_owned(),
        ]);
    }
    args.extend([
        OsString::from("-o"),
        output_template.as_os_str().to_owned(),
        OsString::from(video_url),
    ]);
    args
}

async fn resolve_ytdlp(tool_dir: &Path) -> Result<PathBuf, String> {
    if let Some(path) = cached_executable(tool_dir, ytdlp_name()) {
        return Ok(path);
    }
    if let Some(path) = find_local_executable(ytdlp_name()) {
        return Ok(path);
    }

    let _guard = install_lock().lock().await;
    if let Some(path) = cached_executable(tool_dir, ytdlp_name()) {
        return Ok(path);
    }
    let bytes =
        download_verified_asset(YTDLP_URL, YTDLP_SUMS_URL, YTDLP_ASSET, YTDLP_LIMIT).await?;
    install_asset(tool_dir, ytdlp_name(), bytes).await
}

async fn resolve_runtime(tool_path: &Path, tool_dir: &Path) -> Result<Option<JsRuntime>, String> {
    for kind in ["deno", "node"] {
        if let Some(path) = find_runtime(kind, tool_path)
            && runtime_supported(kind, &path).await
        {
            return Ok(Some(JsRuntime { kind, path }));
        }
    }

    if !cfg!(windows) {
        return Ok(None);
    }

    if let Some(path) = cached_executable(tool_dir, NODE_ASSET) {
        return Ok(Some(JsRuntime { kind: "node", path }));
    }

    let _guard = install_lock().lock().await;
    if let Some(path) = cached_executable(tool_dir, NODE_ASSET) {
        return Ok(Some(JsRuntime { kind: "node", path }));
    }
    let bytes =
        download_verified_asset(NODE_URL, NODE_SUMS_URL, NODE_SUMS_NAME, NODE_LIMIT).await?;
    let path = install_asset(tool_dir, NODE_ASSET, bytes).await?;
    Ok(Some(JsRuntime { kind: "node", path }))
}

async fn resolve_ffmpeg(tool_dir: &Path) -> Result<Option<PathBuf>, String> {
    if let Some(path) = cached_executable(tool_dir, "ffmpeg.exe").filter(|path| has_ffprobe(path)) {
        return Ok(Some(path));
    }
    if let Some(path) = crate::reaction_trim::ffmpeg_executable().filter(|path| has_ffprobe(path)) {
        return Ok(Some(path));
    }
    if !cfg!(windows) {
        return Ok(None);
    }

    let _guard = install_lock().lock().await;
    if let Some(path) = cached_executable(tool_dir, "ffmpeg.exe").filter(|path| has_ffprobe(path)) {
        return Ok(Some(path));
    }
    let archive =
        download_verified_asset(FFMPEG_URL, FFMPEG_SUMS_URL, FFMPEG_ASSET, FFMPEG_LIMIT).await?;
    let archive_directory = TemporaryDirectory::create("relay-ffmpeg-archive")?;
    let archive_path = archive_directory.path().join(FFMPEG_ASSET);
    tokio::task::spawn_blocking({
        let archive_path = archive_path.clone();
        move || fs::write(archive_path, archive)
    })
    .await
    .map_err(|_| "FFmpeg could not be staged.".to_string())?
    .map_err(|_| "FFmpeg could not be staged.".to_string())?;

    let extraction_directory = TemporaryDirectory::create("relay-ffmpeg-extract")?;
    extract_archive(&archive_path, extraction_directory.path()).await?;
    let source = find_file_named(extraction_directory.path(), "ffmpeg.exe")
        .ok_or_else(|| "FFmpeg did not contain an executable.".to_string())?;
    let path = install_file(tool_dir, "ffmpeg.exe", &source).await?;
    let probe = find_file_named(extraction_directory.path(), "ffprobe.exe")
        .ok_or_else(|| "FFmpeg did not contain FFprobe.".to_string())?;
    install_file(tool_dir, "ffprobe.exe", &probe).await?;
    Ok(Some(path))
}

fn has_ffprobe(ffmpeg: &Path) -> bool {
    ffmpeg
        .with_file_name(if cfg!(windows) {
            "ffprobe.exe"
        } else {
            "ffprobe"
        })
        .is_file()
}

async fn extract_archive(archive: &Path, destination: &Path) -> Result<(), String> {
    let archive = archive.to_path_buf();
    let destination = destination.to_path_buf();
    tokio::task::spawn_blocking(move || expand_archive(&archive, &destination))
        .await
        .map_err(|_| "FFmpeg could not be extracted.".to_string())?
}

fn expand_archive(archive: &Path, destination: &Path) -> Result<(), String> {
    let script = format!(
        "Expand-Archive -LiteralPath {} -DestinationPath {} -Force",
        powershell_literal(archive),
        powershell_literal(destination)
    );
    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command"])
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_hidden_window(&mut command);
    let child = command
        .spawn()
        .map_err(|_| "FFmpeg could not be extracted.".to_string())?;
    let mut child = KillOnDrop(Some(child));
    let deadline = Instant::now() + HELPER_TIMEOUT;
    loop {
        let process = child
            .0
            .as_mut()
            .ok_or_else(|| "FFmpeg extraction could not be monitored.".to_string())?;
        if let Some(status) = process
            .try_wait()
            .map_err(|_| "FFmpeg extraction could not be monitored.".to_string())?
        {
            let _ = child.0.take();
            return status
                .success()
                .then_some(())
                .ok_or_else(|| "FFmpeg could not be extracted.".to_string());
        }
        if Instant::now() >= deadline {
            return Err("FFmpeg extraction timed out.".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn powershell_literal(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "''"))
}

fn find_file_named(directory: &Path, name: &str) -> Option<PathBuf> {
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(current).ok()?.flatten() {
            let path = entry.path();
            if entry.file_type().ok()?.is_file()
                && path.file_name().is_some_and(|file_name| file_name == name)
            {
                return Some(path);
            }
            if entry.file_type().ok()?.is_dir() {
                pending.push(path);
            }
        }
    }
    None
}

fn ytdlp_name() -> &'static str {
    if cfg!(windows) { YTDLP_ASSET } else { "yt-dlp" }
}

fn find_runtime(kind: &'static str, tool_path: &Path) -> Option<PathBuf> {
    let executable = if cfg!(windows) {
        format!("{kind}.exe")
    } else {
        kind.to_owned()
    };
    find_in_path(&executable, env::var_os("PATH").as_deref())
        .or_else(|| {
            tool_path
                .parent()
                .map(|parent| parent.join(&executable))
                .filter(|path| path.is_file())
        })
        .or_else(|| {
            local_bin()
                .map(|directory| directory.join(executable))
                .filter(|path| path.is_file())
        })
}

async fn runtime_supported(kind: &'static str, path: &Path) -> bool {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || runtime_supported_blocking(kind, &path))
        .await
        .unwrap_or(false)
}

fn runtime_supported_blocking(kind: &str, path: &Path) -> bool {
    let mut command = Command::new(path);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped());
    configure_hidden_window(&mut command);
    let Ok(output) = command.output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let output = String::from_utf8_lossy(&output.stdout).into_owned();
    let Some(version) = output.split_whitespace().find(|part| {
        part.trim_start_matches('v')
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
    }) else {
        return false;
    };
    let mut numbers = version.trim_start_matches('v').split('.');
    let Some(major) = numbers.next().and_then(|value| value.parse::<u32>().ok()) else {
        return false;
    };
    let minor = numbers
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    match kind {
        "deno" => major > 2 || (major == 2 && minor >= 3),
        "node" => major >= 22,
        _ => false,
    }
}

fn find_local_executable(name: &str) -> Option<PathBuf> {
    find_in_path(name, env::var_os("PATH").as_deref()).or_else(|| {
        local_bin()
            .map(|directory| directory.join(name))
            .filter(|path| path.is_file())
    })
}

fn find_in_path(name: &str, search_path: Option<&OsStr>) -> Option<PathBuf> {
    let search_path = search_path?;
    std::env::split_paths(search_path)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

fn local_bin() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .map(|home| home.join(".local").join("bin"))
}

fn cached_executable(tool_dir: &Path, name: &str) -> Option<PathBuf> {
    let path = tool_dir.join(name);
    path.is_file().then_some(path)
}

fn install_lock() -> &'static Mutex<()> {
    INSTALL_LOCK.get_or_init(|| Mutex::new(()))
}

async fn download_verified_asset(
    asset_url: &str,
    sums_url: &str,
    asset_name: &str,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(HELPER_TIMEOUT)
        .user_agent("Relay YouTube downloader")
        .build()
        .map_err(|_| "Unable to initialize the YouTube helper download.".to_string())?;
    let sums_response = client
        .get(sums_url)
        .send()
        .await
        .map_err(|_| "Unable to download the helper checksum.".to_string())?
        .error_for_status()
        .map_err(|_| "The helper checksum was rejected.".to_string())?;
    let sums = read_response_bounded(sums_response, CHECKSUM_LIMIT).await?;
    let expected = checksum_for_asset(&sums, asset_name)
        .ok_or_else(|| "The helper checksum is unavailable.".to_string())?;
    let asset_response = client
        .get(asset_url)
        .send()
        .await
        .map_err(|_| "Unable to download the YouTube helper.".to_string())?
        .error_for_status()
        .map_err(|_| "The YouTube helper download was rejected.".to_string())?;
    let bytes = read_response_bounded(asset_response, limit).await?;
    let actual = Sha256::digest(&bytes);
    if actual.as_slice() != expected.as_slice() {
        return Err("The downloaded helper failed its checksum.".into());
    }
    Ok(bytes)
}

async fn read_response_bounded(
    response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("The downloaded helper is too large.".into());
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "The helper download was interrupted.".to_string())?;
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Err("The downloaded helper is too large.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err("The downloaded helper is empty.".into());
    }
    Ok(bytes)
}

fn checksum_for_asset(bytes: &[u8], asset_name: &str) -> Option<[u8; 32]> {
    std::str::from_utf8(bytes).ok()?.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let hash = fields.next()?;
        let name = fields.next().map(|name| name.trim_start_matches('*'));
        (name.is_none() || name == Some(asset_name))
            .then(|| decode_hex(hash))
            .flatten()
    })
}

fn decode_hex(value: &str) -> Option<[u8; 32]> {
    let value = value.as_bytes();
    if value.len() != 64 || !value.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let mut decoded = [0; 32];
    for (index, byte) in decoded.iter_mut().enumerate() {
        *byte = (hex_digit(value[index * 2])? << 4) | hex_digit(value[index * 2 + 1])?;
    }
    Some(decoded)
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

async fn install_asset(tool_dir: &Path, name: &str, bytes: Vec<u8>) -> Result<PathBuf, String> {
    let directory = tool_dir.to_path_buf();
    let name = name.to_owned();
    tokio::task::spawn_blocking(move || atomic_install(&directory, &name, &bytes))
        .await
        .map_err(|_| "The helper could not be installed.".to_string())?
}

async fn install_file(tool_dir: &Path, name: &str, source: &Path) -> Result<PathBuf, String> {
    let directory = tool_dir.to_path_buf();
    let name = name.to_owned();
    let source = source.to_path_buf();
    tokio::task::spawn_blocking(move || atomic_copy(&directory, &name, &source))
        .await
        .map_err(|_| "The helper could not be installed.".to_string())?
}

fn atomic_install(directory: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|_| "The helper cache directory is unavailable.".to_string())?;
    let final_path = directory.join(name);
    if final_path.is_file() {
        return Ok(final_path);
    }
    let temporary = unique_path(directory, ".relay-helper")?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| "The helper could not be staged.".to_string())?;
        file.write_all(bytes)
            .map_err(|_| "The helper could not be staged.".to_string())?;
        file.sync_all()
            .map_err(|_| "The helper could not be staged.".to_string())?;
        drop(file);
        fs::rename(&temporary, &final_path)
            .map_err(|_| "The helper could not be installed atomically.".to_string())?;
        Ok(final_path.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn atomic_copy(directory: &Path, name: &str, source: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|_| "The helper cache directory is unavailable.".to_string())?;
    let final_path = directory.join(name);
    if final_path.is_file() {
        return Ok(final_path);
    }
    let temporary = unique_path(directory, ".relay-helper")?;
    let result = (|| {
        fs::copy(source, &temporary).map_err(|_| "The helper could not be staged.".to_string())?;
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&temporary)
            .and_then(|file| file.sync_all())
            .map_err(|_| "The helper could not be staged.".to_string())?;
        fs::rename(&temporary, &final_path)
            .map_err(|_| "The helper could not be installed atomically.".to_string())?;
        Ok(final_path.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

async fn run_ytdlp(executable: PathBuf, args: Vec<OsString>) -> Result<(), String> {
    tokio::task::spawn_blocking(move || run_ytdlp_blocking(&executable, &args))
        .await
        .map_err(|_| "The YouTube download could not be started.".to_string())?
}

fn run_ytdlp_blocking(executable: &Path, args: &[OsString]) -> Result<(), String> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_hidden_window(&mut command);
    let child = command
        .spawn()
        .map_err(|_| "yt-dlp could not be started.".to_string())?;
    let mut child = KillOnDrop(Some(child));
    let deadline = Instant::now() + DOWNLOAD_TIMEOUT;
    loop {
        let process = child
            .0
            .as_mut()
            .ok_or_else(|| "The YouTube download could not be monitored.".to_string())?;
        if let Some(status) = process
            .try_wait()
            .map_err(|_| "The YouTube download could not be monitored.".to_string())?
        {
            let _ = child.0.take();
            return status
                .success()
                .then_some(())
                .ok_or_else(|| "yt-dlp could not download this video.".to_string());
        }
        if Instant::now() >= deadline {
            return Err("The YouTube download timed out.".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

struct KillOnDrop(Option<Child>);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn find_downloaded_file(directory: &Path, format: &str) -> Result<PathBuf, String> {
    let file = directory.join(format!("download.{format}"));
    let metadata =
        fs::metadata(&file).map_err(|_| "yt-dlp did not create a video file.".to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_VIDEO_BYTES {
        return Err("The downloaded video is too large or empty.".into());
    }
    Ok(file)
}

fn persist_output(source: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        return Err("The selected save folder is unavailable.".into());
    }
    if destination.exists() {
        return Err("A file with that name already exists.".into());
    }

    let temporary = unique_path(parent, ".relay-youtube")?;
    let result = (|| {
        let input =
            File::open(source).map_err(|_| "The downloaded video is unavailable.".to_string())?;
        let metadata = input
            .metadata()
            .map_err(|_| "The downloaded video is unavailable.".to_string())?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_VIDEO_BYTES {
            return Err("The downloaded video is too large or empty.".into());
        }
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| "The video could not be staged.".to_string())?;
        let copied = io::copy(&mut input.take(MAX_VIDEO_BYTES + 1), &mut output)
            .map_err(|_| "The video could not be staged.".to_string())?;
        if copied == 0 || copied > MAX_VIDEO_BYTES {
            return Err("The downloaded video is too large or empty.".into());
        }
        output
            .flush()
            .and_then(|_| output.sync_all())
            .map_err(|_| "The video could not be staged.".to_string())?;
        drop(output);
        if destination.exists() {
            return Err("A file with that name already exists.".into());
        }
        fs::rename(&temporary, destination)
            .map_err(|_| "The video could not be saved atomically.".to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn unique_path(directory: &Path, prefix: &str) -> Result<PathBuf, String> {
    for attempt in 0..8u32 {
        let entropy = rand::random::<u64>();
        let path = directory.join(format!(
            "{prefix}-{}-{entropy:016x}-{attempt}",
            std::process::id()
        ));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err("Unable to create a private temporary file.".into())
}

fn configure_hidden_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
}

fn temporary_directory(prefix: &str) -> Result<PathBuf, String> {
    for _ in 0..8 {
        let path = env::temp_dir().join(format!(
            "{prefix}-{}-{:016x}",
            std::process::id(),
            rand::random::<u64>()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("Unable to create a private temporary directory.".into()),
        }
    }
    Err("Unable to create a private temporary directory.".into())
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn create(prefix: &str) -> Result<Self, String> {
        temporary_directory(prefix).map(Self)
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
mod tests {
    use super::*;

    #[test]
    fn accepts_only_strict_youtube_ids() {
        assert_eq!(
            youtube_url("dQw4w9WgXcQ").unwrap(),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        );
        assert!(validate_video_id("too-short").is_err());
        assert!(validate_video_id("dQw4w9WgXcQ!").is_err());
        assert!(validate_video_id("dQw4w9WgXc_").is_ok());
    }

    #[test]
    fn arguments_ignore_user_config_and_playlists() {
        let runtime = JsRuntime {
            kind: "node",
            path: PathBuf::from(r"C:\Relay Cache\node.exe"),
        };
        let args = ytdlp_args(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            Path::new(r"C:\Temp\download.mp4"),
            Some(&runtime),
            Some(Path::new(r"C:\Tools\ffmpeg.exe")),
            "mp4",
        );
        let args = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.contains(&"--ignore-config".into()));
        assert!(args.contains(&"--no-playlist".into()));
        assert!(args.contains(&"bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]".into()));
        assert!(args.contains(&"ejs:github".into()));
        assert!(args.contains(&r"node:C:\Relay Cache\node.exe".into()));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
    }

    #[test]
    fn mp3_choice_extracts_audio_and_unknown_formats_are_rejected() {
        assert_eq!(download_format("mp3"), Ok("mp3"));
        assert_eq!(download_format("mp4"), Ok("mp4"));
        assert!(download_format("MP3").is_err());
        assert!(download_format("webm").is_err());
        let args = ytdlp_args(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            Path::new("download.%(ext)s"),
            None,
            Some(Path::new("ffmpeg.exe")),
            "mp3",
        );
        let args = args
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>();
        assert!(args.windows(2).any(|pair| pair == ["-f", "ba/b"]));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--audio-format", "mp3"])
        );
        assert!(args.iter().any(|arg| arg == "--extract-audio"));
        assert!(!args.iter().any(|arg| arg == "--merge-output-format"));
        let directory = tempfile::tempdir().unwrap();
        let ffmpeg = directory.path().join("ffmpeg.exe");
        assert!(!has_ffprobe(&ffmpeg));
        fs::write(
            ffmpeg.with_file_name(if cfg!(windows) {
                "ffprobe.exe"
            } else {
                "ffprobe"
            }),
            b"probe",
        )
        .unwrap();
        assert!(has_ffprobe(&ffmpeg));
        fs::write(directory.path().join("download.mp3"), b"audio").unwrap();
        assert_eq!(
            find_downloaded_file(directory.path(), "mp3").unwrap(),
            directory.path().join("download.mp3")
        );
    }

    #[test]
    fn title_is_safe_for_a_windows_save_dialog() {
        assert_eq!(safe_title("  clip:01..."), "  clip_01.mp4");
        assert_eq!(safe_title("\0"), "youtube-video.mp4");
    }

    #[test]
    fn parses_checksum_lines() {
        let sums = b"deadbeef  other.exe\n0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef *yt-dlp.exe\n";
        assert_eq!(checksum_for_asset(sums, "yt-dlp.exe").unwrap()[0], 0x01);
        assert!(checksum_for_asset(sums, "node.exe").is_none());
        assert_eq!(
            checksum_for_asset(
                b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
                "ffmpeg-release-essentials.zip"
            )
            .unwrap()[0],
            0x01
        );
        assert!(decode_hex(&"é".repeat(32)).is_none());
    }

    #[test]
    fn failed_save_preserves_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("download.mp4");
        let destination = directory.path().join("saved.mp4");
        fs::write(&source, b"new video").unwrap();
        fs::write(&destination, b"existing video").unwrap();

        assert!(persist_output(&source, &destination).is_err());
        assert_eq!(fs::read(destination).unwrap(), b"existing video");
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "opt-in helper provisioning smoke; set RELAY_YOUTUBE_HELPER_SMOKE=1"]
    fn provisions_verified_helpers_in_an_isolated_cache() {
        if env::var_os("RELAY_YOUTUBE_HELPER_SMOKE").is_none() {
            return;
        }
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let directory = tempfile::tempdir().unwrap();
            let ytdlp = install_asset(
                directory.path(),
                YTDLP_ASSET,
                download_verified_asset(YTDLP_URL, YTDLP_SUMS_URL, YTDLP_ASSET, YTDLP_LIMIT)
                    .await
                    .unwrap(),
            )
            .await
            .unwrap();
            assert!(
                Command::new(&ytdlp)
                    .arg("--version")
                    .status()
                    .unwrap()
                    .success()
            );

            let node = install_asset(
                directory.path(),
                NODE_ASSET,
                download_verified_asset(NODE_URL, NODE_SUMS_URL, NODE_SUMS_NAME, NODE_LIMIT)
                    .await
                    .unwrap(),
            )
            .await
            .unwrap();
            assert!(
                Command::new(&node)
                    .arg("--version")
                    .status()
                    .unwrap()
                    .success()
            );

            let archive_directory = TemporaryDirectory::create("relay-ffmpeg-smoke").unwrap();
            let archive_path = archive_directory.path().join(FFMPEG_ASSET);
            fs::write(
                &archive_path,
                download_verified_asset(FFMPEG_URL, FFMPEG_SUMS_URL, FFMPEG_ASSET, FFMPEG_LIMIT)
                    .await
                    .unwrap(),
            )
            .unwrap();
            let extraction_directory =
                TemporaryDirectory::create("relay-ffmpeg-smoke-extract").unwrap();
            extract_archive(&archive_path, extraction_directory.path())
                .await
                .unwrap();
            let source = find_file_named(extraction_directory.path(), "ffmpeg.exe").unwrap();
            let ffmpeg = install_file(directory.path(), "ffmpeg.exe", &source)
                .await
                .unwrap();
            assert!(
                Command::new(ffmpeg)
                    .arg("-version")
                    .status()
                    .unwrap()
                    .success()
            );
        });
    }
}
