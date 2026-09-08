//! Temporary, local audio selections used to prepare reaction sounds.
//!
//! A long source is never copied into the reaction library until the user
//! confirms a bounded selection.  The control panel receives an opaque token
//! for the selected source; all subsequent operations resolve that token
//! through [`ReactionTrimState`] and never accept a client supplied path.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};

pub use crate::reactions::MAX_REACTION_SECONDS;
pub const MAX_SOURCE_BYTES: u64 = 250 * 1024 * 1024;
pub const MAX_RENDERED_BYTES: u64 = 15 * 1024 * 1024;

const RENDER_TIMEOUT: Duration = Duration::from_secs(120);
const PENDING_TTL: Duration = Duration::from_secs(15 * 60);
const RANGE_TOLERANCE: f64 = 1e-9;
const MAX_FILENAME_CHARS: usize = 240;
const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "opus", "m4a", "aac"];

#[derive(Clone, Debug)]
pub struct AudioSource {
    path: PathBuf,
    filename: String,
    duration_seconds: f64,
}

impl AudioSource {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn duration_seconds(&self) -> f64 {
        self.duration_seconds
    }
}

#[derive(Clone, Debug)]
pub struct PendingReactionTrim {
    token: String,
    source: AudioSource,
    expires_at: Instant,
}

impl PendingReactionTrim {
    pub fn source(&self) -> &AudioSource {
        &self.source
    }
}

#[derive(Default)]
pub struct ReactionTrimState {
    pending: Option<PendingReactionTrim>,
}

impl ReactionTrimState {
    /// Replace the one pending selection and return its opaque token.
    pub fn replace(&mut self, source: AudioSource) -> String {
        let token = crate::reactions::new_id();
        self.pending = Some(PendingReactionTrim {
            token: token.clone(),
            source,
            expires_at: Instant::now() + PENDING_TTL,
        });
        token
    }

    /// Resolve a token while preserving the selection for preview retries.
    pub fn resolve(&mut self, token: &str) -> Result<PendingReactionTrim> {
        self.purge_expired();
        if !crate::reactions::valid_id(token) {
            bail!("Trim selection expired or unavailable.");
        }
        self.pending
            .as_ref()
            .filter(|pending| pending.token == token)
            .cloned()
            .context("Trim selection expired or unavailable.")
    }

    /// Consume a selection after a successful render.  This is the
    /// linearization point that makes finish and cancel races deterministic.
    pub fn take(&mut self, token: &str) -> Result<PendingReactionTrim> {
        self.purge_expired();
        if !crate::reactions::valid_id(token) {
            bail!("Trim selection expired or unavailable.");
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.token == token)
        {
            return self
                .pending
                .take()
                .context("Trim selection expired or unavailable.");
        }
        bail!("Trim selection expired or unavailable.");
    }

    /// Cancel the current selection.  The selected source is untouched.
    pub fn cancel(&mut self, token: &str) -> Result<()> {
        self.take(token).map(|_| ())
    }

    /// Put a selection back after a persistence failure, unless another
    /// selection replaced it while the file was being rendered/imported.
    pub fn restore_if_empty(&mut self, pending: PendingReactionTrim) {
        self.purge_expired();
        if self.pending.is_none() {
            self.pending = Some(pending);
        }
    }

    /// Discard the current selection when a short sound was imported directly
    /// or when the caller is replacing an abandoned dialog.
    pub fn clear(&mut self) {
        self.pending = None;
    }

    fn purge_expired(&mut self) {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| Instant::now() >= pending.expires_at)
        {
            self.pending = None;
        }
    }
}

/// Validate a picker-selected local file and probe its audio duration without
/// loading its contents into memory.  The path is canonicalized before it is
/// stored in pending state so later rendering never follows a caller-provided
/// path from the IPC request.
pub fn inspect_source(path: &Path) -> Result<AudioSource> {
    let canonical = fs::canonicalize(path).context("The selected audio file is unavailable.")?;
    let metadata = fs::metadata(&canonical).context("The selected audio file is unavailable.")?;
    if !metadata.is_file() || metadata.len() == 0 {
        bail!("The selected audio file must be a regular, non-empty file.");
    }
    if metadata.len() > MAX_SOURCE_BYTES {
        bail!("The selected audio file must be under 250 MB.");
    }
    if !has_supported_extension(&canonical) {
        bail!("Unsupported reaction audio format.");
    }
    let filename = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .context("The selected audio filename is invalid.")?
        .to_owned();
    if filename.is_empty()
        || filename.chars().count() > MAX_FILENAME_CHARS
        || filename.chars().any(char::is_control)
    {
        bail!("The selected audio filename is invalid.");
    }
    let duration_seconds = probe_duration(&canonical)?;
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        bail!("The selected audio file has no usable duration.");
    }
    Ok(AudioSource {
        path: canonical,
        filename,
        duration_seconds,
    })
}

pub fn render_pcm_wav(
    source: &AudioSource,
    start_seconds: f64,
    end_seconds: f64,
) -> Result<Vec<u8>> {
    validate_bounds(start_seconds, end_seconds, source.duration_seconds)?;
    validate_source_for_render(source.path(), end_seconds)?;

    let temporary = TemporaryDirectory::create("relay-reaction-trim")?;
    let output_path = temporary.path().join("selection.wav");
    let selection_seconds = (end_seconds - start_seconds).min(MAX_REACTION_SECONDS);
    let executable = ffmpeg_executable().context("Audio trimming requires FFmpeg.")?;
    let mut command = Command::new(executable);
    command
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-protocol_whitelist",
            "file",
            "-ss",
        ])
        .arg(format!("{start_seconds:.6}"))
        .args(["-i"])
        .arg(source.path())
        .args([
            "-map",
            "0:a:0",
            "-vn",
            "-map_metadata",
            "-1",
            "-map_chapters",
            "-1",
            "-sn",
            "-dn",
            "-af",
            "asetpts=PTS-STARTPTS",
            "-t",
        ])
        .arg(format!("{selection_seconds:.6}"))
        .args([
            "-ac",
            "2",
            "-ar",
            "44100",
            "-c:a",
            "pcm_s16le",
            "-f",
            "wav",
            "-fs",
        ])
        .arg(MAX_RENDERED_BYTES.to_string())
        .arg(&output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_hidden_window(&mut command);
    let child = command.spawn().context("Audio trimming requires FFmpeg.")?;
    let status =
        wait_for_process(child, RENDER_TIMEOUT).context("Unable to complete the audio trim.")?;
    if !status.success() {
        bail!("FFmpeg rejected the selected audio range.");
    }
    let bytes = read_bounded(&output_path, MAX_RENDERED_BYTES)
        .context("FFmpeg did not create a valid audio preview.")?;
    validate_pcm_wav(&bytes)?;
    Ok(bytes)
}

/// Persist a rendered WAV through the existing reaction import validator.
/// The intermediate file is private and removed with its temporary directory.
pub fn import_rendered_sound(directory: &Path, bytes: &[u8]) -> Result<String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_RENDERED_BYTES {
        bail!("The trimmed reaction sound is too large.");
    }
    validate_pcm_wav(bytes)?;
    let temporary = TemporaryDirectory::create("relay-reaction-import")?;
    let source_path = temporary.path().join("selection.wav");
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&source_path)
        .context("Unable to stage the trimmed reaction sound.")?;
    file.write_all(bytes)
        .context("Unable to stage the trimmed reaction sound.")?;
    file.sync_all()
        .context("Unable to stage the trimmed reaction sound.")?;
    crate::reactions::import_sound(directory, &source_path)
}

pub fn validate_bounds(start_seconds: f64, end_seconds: f64, source_duration: f64) -> Result<()> {
    if !start_seconds.is_finite()
        || !end_seconds.is_finite()
        || !source_duration.is_finite()
        || source_duration <= 0.0
        || start_seconds < 0.0
        || end_seconds <= start_seconds
        || end_seconds > source_duration + RANGE_TOLERANCE
        || end_seconds - start_seconds > MAX_REACTION_SECONDS + RANGE_TOLERANCE
    {
        bail!("Trim range must be finite, within the source, and no longer than 30 seconds.");
    }
    Ok(())
}

fn validate_source_for_render(path: &Path, end_seconds: f64) -> Result<()> {
    let metadata = fs::metadata(path).context("The selected audio file is unavailable.")?;
    if !metadata.is_file() || metadata.len() == 0 {
        bail!("The selected audio file must be a regular, non-empty file.");
    }
    if metadata.len() > MAX_SOURCE_BYTES {
        bail!("The selected audio file must be under 250 MB.");
    }
    if !has_supported_extension(path) {
        bail!("Unsupported reaction audio format.");
    }
    // The path may have been replaced while the dialog was open.  Probe it
    // again before handing it to FFmpeg and reject a replacement whose
    // duration no longer covers the selected range.
    let current_duration = probe_duration(path)?;
    if !current_duration.is_finite()
        || current_duration <= 0.0
        || current_duration + 0.001 < end_seconds
    {
        bail!("The selected audio file changed or is no longer usable.");
    }
    Ok(())
}

fn has_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            SUPPORTED_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        })
}

fn probe_duration(path: &Path) -> Result<f64> {
    use lofty::prelude::AudioFile;

    let tagged = lofty::probe::Probe::open(path)
        .context("Unable to inspect the selected audio.")?
        .read()
        .context("Unable to inspect the selected audio.")?;
    let duration = tagged.properties().duration().as_secs_f64();
    if duration.is_finite() && duration > 0.0 {
        Ok(duration)
    } else {
        bail!("The selected audio has no usable duration.");
    }
}

fn ffmpeg_executable() -> Option<PathBuf> {
    let search_path = std::env::var_os("PATH");
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    find_ffmpeg(search_path.as_deref(), local_app_data.as_deref())
}

fn find_ffmpeg(
    search_path: Option<&std::ffi::OsStr>,
    local_app_data: Option<&Path>,
) -> Option<PathBuf> {
    let executable = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    if let Some(search_path) = search_path {
        for directory in std::env::split_paths(search_path).filter(|path| path.is_absolute()) {
            let candidate = directory.join(executable);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    // Explorer and already-running desktop applications can retain PATH from
    // before a WinGet installation. Resolve the installed package directly.
    let winget = local_app_data?.join("Microsoft").join("WinGet");
    let link = winget.join("Links").join(executable);
    if link.is_file() {
        return Some(link);
    }
    let mut candidates = Vec::new();
    for package in fs::read_dir(winget.join("Packages")).ok()?.flatten() {
        if !package
            .file_name()
            .to_string_lossy()
            .starts_with("Gyan.FFmpeg_")
        {
            continue;
        }
        let Ok(versions) = fs::read_dir(package.path()) else {
            continue;
        };
        for version in versions.flatten() {
            if !version.file_name().to_string_lossy().starts_with("ffmpeg-") {
                continue;
            }
            let candidate = version.path().join("bin").join(executable);
            if candidate.is_file() {
                candidates.push(candidate);
            }
        }
    }
    candidates.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    candidates.pop()
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let file = File::open(path).context("Output file unavailable.")?;
    let metadata = file
        .metadata()
        .context("Output file metadata unavailable.")?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > limit {
        bail!("Generated audio exceeds the permitted size.");
    }
    let capacity = usize::try_from(metadata.len()).context("Generated audio is too large.")?;
    let mut bytes = Vec::with_capacity(capacity.min(limit as usize));
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .context("Unable to read generated audio.")?;
    if bytes.len() as u64 > limit {
        bail!("Generated audio exceeds the permitted size.");
    }
    Ok(bytes)
}

fn validate_pcm_wav(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 44 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        bail!("Generated audio is not a PCM WAV file.");
    }
    Ok(())
}

fn wait_for_process(mut child: Child, timeout: Duration) -> Result<ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().context("Unable to monitor FFmpeg.")? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("FFmpeg timed out while trimming audio.");
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn configure_hidden_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn create(prefix: &str) -> Result<Self> {
        for _ in 0..8 {
            let path = std::env::temp_dir().join(format!(
                "{prefix}-{}-{:016x}",
                std::process::id(),
                rand::random::<u64>()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error).context("Unable to create a temporary audio directory");
                }
            }
        }
        bail!("Unable to create a temporary audio directory.");
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
    fn finds_installed_winget_ffmpeg_without_inherited_path() {
        let directory = new_directory("relay-ffmpeg-discovery-");
        let bin = directory.path().join("Microsoft/WinGet/Packages/Gyan.FFmpeg_Microsoft.Winget.Source_test/ffmpeg-8.1-full_build/bin");
        fs::create_dir_all(&bin).unwrap();
        let executable = bin.join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        fs::write(&executable, b"fixture").unwrap();
        assert_eq!(find_ffmpeg(None, Some(directory.path())), Some(executable));
        assert_eq!(find_ffmpeg(None, None), None);
    }

    #[test]
    fn explicit_path_installation_is_preferred() {
        let directory = new_directory("relay-ffmpeg-path-");
        let executable = directory.path().join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        fs::write(&executable, b"fixture").unwrap();
        let path = std::env::join_paths([directory.path()]).unwrap();
        assert_eq!(find_ffmpeg(Some(&path), None), Some(executable));
    }

    #[test]
    #[ignore = "requires RELAY_TRIM_SMOKE_SOURCE pointing to an authorized local audio sample"]
    fn user_audio_sample_trims_and_imports() {
        let path = PathBuf::from(std::env::var_os("RELAY_TRIM_SMOKE_SOURCE").expect("sample path"));
        let original = fs::read(&path).unwrap();
        let source = inspect_source(&path).unwrap();
        let directory = new_directory("relay-trim-user-sample-");
        let bytes = render_pcm_wav(&source, 0.0, MAX_REACTION_SECONDS).unwrap();
        let id = import_rendered_sound(directory.path(), &bytes).unwrap();
        assert_eq!(
            crate::reactions::read_sound(directory.path(), &id).unwrap(),
            bytes
        );
        assert_eq!(fs::read(path).unwrap(), original);
    }

    fn ffmpeg_available() -> bool {
        Command::new("ffmpeg")
            .args(["-hide_banner", "-version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    fn new_directory(prefix: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .expect("temporary test directory")
    }

    fn wav_with_pulse(seconds: u32) -> Vec<u8> {
        let sample_rate = 8_000u32;
        let channels = 1u16;
        let sample_count = seconds * sample_rate;
        let mut samples = Vec::with_capacity(sample_count as usize * 2);
        for index in 0..sample_count {
            // A silent first half-second and a stable nonzero tone after it
            // make the selected nonzero range observable in the test output.
            let sample = if index < sample_rate / 2 {
                0
            } else if index % 16 < 8 {
                12_000
            } else {
                -12_000
            };
            samples.extend_from_slice(&(sample as i16).to_le_bytes());
        }
        let data_len = samples.len() as u32;
        let mut bytes = Vec::with_capacity(44 + samples.len());
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes());
        bytes.extend_from_slice(&(channels * 2).to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.extend_from_slice(&samples);
        bytes
    }

    fn wav_data(bytes: &[u8]) -> Option<&[u8]> {
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return None;
        }
        let mut offset = 12usize;
        while offset.checked_add(8)? <= bytes.len() {
            let chunk = &bytes[offset..offset + 4];
            let length =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().ok()?) as usize;
            let data_start = offset + 8;
            let data_end = data_start.checked_add(length)?;
            if data_end > bytes.len() {
                return None;
            }
            if chunk == b"data" {
                return Some(&bytes[data_start..data_end]);
            }
            offset = data_end + (length & 1);
        }
        None
    }

    #[test]
    fn source_probe_and_real_ffmpeg_trim_use_a_nonzero_selection() {
        if !ffmpeg_available() {
            return;
        }
        let directory = new_directory("relay-trim-real-");
        let path = directory.path().join("long.wav");
        fs::write(&path, wav_with_pulse(40)).unwrap();

        let source = inspect_source(&path).unwrap();
        assert!(source.duration_seconds() > 39.0);
        let bytes = render_pcm_wav(&source, 2.0, 32.0).unwrap();
        validate_pcm_wav(&bytes).unwrap();
        let data = wav_data(&bytes).expect("PCM data chunk");
        let duration = data.len() as f64 / (44_100.0 * 2.0 * 2.0);
        assert!(duration > 29.0 && duration <= 30.01, "duration={duration}");
        let first_tenth = &data[..data.len().min(44_100 * 2 * 2 / 10)];
        assert!(
            first_tenth
                .chunks_exact(2)
                .any(|sample| i16::from_le_bytes([sample[0], sample[1]]) != 0)
        );
    }

    #[test]
    fn source_size_and_format_are_rejected_before_probe() {
        let directory = new_directory("relay-trim-invalid-");
        let oversized = directory.path().join("large.wav");
        File::create(&oversized)
            .unwrap()
            .set_len(MAX_SOURCE_BYTES + 1)
            .unwrap();
        let error = inspect_source(&oversized).unwrap_err().to_string();
        assert!(error.contains("250 MB"));

        let unsupported = directory.path().join("audio.txt");
        fs::write(&unsupported, b"not audio").unwrap();
        let error = inspect_source(&unsupported).unwrap_err().to_string();
        assert!(error.contains("Unsupported"));
    }

    #[test]
    fn bounds_require_finite_nonempty_selection_inside_source() {
        for (start, end, duration) in [
            (f64::NAN, 1.0, 20.0),
            (0.0, f64::INFINITY, 20.0),
            (-1.0, 1.0, 20.0),
            (5.0, 5.0, 20.0),
            (0.0, 30.001, 40.0),
            (10.0, 40.0, 39.0),
        ] {
            assert!(validate_bounds(start, end, duration).is_err());
        }
        assert!(validate_bounds(2.0, 32.0, 40.0).is_ok());
    }

    #[test]
    fn replacement_and_cancel_invalidate_old_token_without_touching_source() {
        let directory = new_directory("relay-trim-state-");
        let path = directory.path().join("source.wav");
        fs::write(&path, b"source remains untouched").unwrap();
        let source = AudioSource {
            path: path.clone(),
            filename: "source.wav".into(),
            duration_seconds: 20.0,
        };
        let mut state = ReactionTrimState::default();
        let old = state.replace(source.clone());
        let current = state.replace(source);
        assert!(state.resolve(&old).is_err());
        state.cancel(&current).unwrap();
        assert!(state.resolve(&current).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"source remains untouched");
    }

    #[test]
    fn expired_selection_is_removed_before_it_can_be_resolved() {
        let directory = new_directory("relay-trim-expiry-");
        let path = directory.path().join("source.wav");
        fs::write(&path, b"source remains untouched").unwrap();
        let source = AudioSource {
            path,
            filename: "source.wav".into(),
            duration_seconds: 20.0,
        };
        let mut state = ReactionTrimState::default();
        let token = state.replace(source);
        state.pending.as_mut().unwrap().expires_at = Instant::now() - Duration::from_secs(1);
        assert!(state.resolve(&token).is_err());
    }

    #[test]
    fn rendered_audio_is_persisted_through_the_existing_reaction_import() {
        if !ffmpeg_available() {
            return;
        }
        let directory = new_directory("relay-trim-finish-");
        let source_path = directory.path().join("long.wav");
        fs::write(&source_path, wav_with_pulse(40)).unwrap();
        let original = fs::read(&source_path).unwrap();
        let source = inspect_source(&source_path).unwrap();
        let bytes = render_pcm_wav(&source, 2.0, 5.5).unwrap();
        let managed = directory.path().join("reactions");
        let id = import_rendered_sound(&managed, &bytes).unwrap();
        assert!(crate::reactions::valid_id(&id));
        assert_eq!(crate::reactions::read_sound(&managed, &id).unwrap(), bytes);
        let maximum_selection = render_pcm_wav(&source, 2.0, 32.0).unwrap();
        let maximum_id = import_rendered_sound(&managed, &maximum_selection).unwrap();
        assert_eq!(
            crate::reactions::read_sound(&managed, &maximum_id).unwrap(),
            maximum_selection
        );
        assert_eq!(fs::read(&source_path).unwrap(), original);
    }

    #[test]
    fn rendered_import_rejects_invalid_bytes_without_managed_artifact() {
        let directory = new_directory("relay-trim-import-");
        let managed = directory.path().join("reactions");
        assert!(import_rendered_sound(&managed, b"bad").is_err());
        assert!(!managed.exists());
    }
}
