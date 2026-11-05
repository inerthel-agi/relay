//! Voice check before the stream: Discord voice messages, audio files and the
//! sound of videos are transcribed on this PC (FFmpeg's whisper filter, see
//! `media_compat::transcribe`), then the text goes through the privacy scan
//! and the word filter like a message. Nothing is sent online; a sound that
//! cannot be checked is held for review.

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::State;
use tokio::sync::Mutex;

use crate::{config::AppConfig, media_compat, model::MediaKind, privacy, state::AppCore};

/// whisper.cpp's multilingual base model, quantized: measured on this PC about
/// twice as fast as the small one (3.3 s instead of 7.3 s for 28 s of sound),
/// still right on clear French numbers and addresses, weaker on accents and noise.
pub const MODEL_FILE: &str = "ggml-base-q5_1.bin";
const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin";
const MODEL_BYTES: u64 = 59_707_625;
const MODEL_SHA256: &str = "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
static DOWNLOAD: Mutex<()> = Mutex::const_new(());

/// Hold reason: the sound could not be checked (FFmpeg without Whisper, no
/// model, a sound too long or unreadable).
pub const HOLD_UNCHECKED: &str = "voice_unchecked";

pub fn model_directory(data_directory: &Path) -> PathBuf {
    data_directory.join("whisper")
}

fn model_ready(directory: &Path) -> bool {
    std::fs::metadata(directory.join(MODEL_FILE)).is_ok_and(|file| file.len() == MODEL_BYTES)
}

/// Only sounds go through the check, and only when it can find something.
pub fn applies(config: &AppConfig, kind: MediaKind) -> bool {
    config.voice_scan_enabled
        && matches!(kind, MediaKind::Audio | MediaKind::Video)
        && privacy::privacy_rules_enabled(config)
}

/// What the check decided for one media.
#[derive(Debug, PartialEq)]
pub enum VoiceCheck {
    /// No sound track: nothing to hear.
    Silent,
    /// The transcript stays in memory for the AI's second reading, never in a log.
    Checked {
        report: privacy::PrivacyReport,
        transcript: String,
    },
    Unchecked,
}

/// Transcribes one media and checks the text. `bytes` are the audio file when
/// Relay already downloaded it; otherwise the media is fetched from Discord.
pub async fn check(
    directory: PathBuf,
    (url, proxy_url, bytes): (&str, &str, Option<Vec<u8>>),
    config: &AppConfig,
) -> VoiceCheck {
    if !model_ready(&directory) {
        return VoiceCheck::Unchecked;
    }
    let transcript = match bytes {
        Some(bytes) => media_compat::transcribe(bytes, directory, MODEL_FILE).await,
        None => media_compat::transcribe_url(url, proxy_url, directory, MODEL_FILE).await,
    };
    match transcript {
        Ok(Some(text)) => judge(&text, config),
        Ok(None) => VoiceCheck::Silent,
        Err(_) => VoiceCheck::Unchecked,
    }
}

/// The text is checked whole: a transcript beyond the scan limit (several
/// minutes of speech) is held rather than checked in part.
fn judge(text: &str, config: &AppConfig) -> VoiceCheck {
    if text.chars().count() > privacy::PRIVACY_TEXT_LIMIT {
        return VoiceCheck::Unchecked;
    }
    let text = join_spoken_digits(text);
    let mut report = privacy::classify_text(Some(&text), config);
    report.apply_score_policy(config);
    VoiceCheck::Checked {
        report,
        transcript: text,
    }
}

/// Whisper writes the pauses of a dictated number as commas ("06, 12, 34"):
/// they become spaces so the scan still sees one phone number. Typed messages
/// keep their commas, where "10, 20, 30, 40, 50" is a list.
fn join_spoken_digits(text: &str) -> String {
    let characters = text.chars().collect::<Vec<_>>();
    characters
        .iter()
        .enumerate()
        .map(|(index, &character)| {
            let spoken_pause = character == ','
                && next_is_digit(characters[..index].iter().rev())
                && next_is_digit(characters[index + 1..].iter());
            if spoken_pause { ' ' } else { character }
        })
        .collect()
}

fn next_is_digit<'a>(mut characters: impl Iterator<Item = &'a char>) -> bool {
    characters
        .find(|character| !character.is_whitespace())
        .is_some_and(char::is_ascii_digit)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceScanStatus {
    ffmpeg_ready: bool,
    model_ready: bool,
}

async fn status(core: &AppCore) -> VoiceScanStatus {
    let directory = model_directory(&core.data_directory);
    let ffmpeg_ready = tauri::async_runtime::spawn_blocking(media_compat::ffmpeg_has_whisper)
        .await
        .unwrap_or(false);
    VoiceScanStatus {
        ffmpeg_ready,
        model_ready: model_ready(&directory),
    }
}

#[tauri::command]
pub async fn voice_scan_status(core: State<'_, Arc<AppCore>>) -> Result<VoiceScanStatus, String> {
    Ok(status(&core).await)
}

#[tauri::command]
pub async fn download_voice_model(
    core: State<'_, Arc<AppCore>>,
) -> Result<VoiceScanStatus, String> {
    let Ok(_download) = DOWNLOAD.try_lock() else {
        return Err("The voice model is already downloading.".into());
    };
    let directory = model_directory(&core.data_directory);
    if !model_ready(&directory) {
        download_model(&directory)
            .await
            .map_err(|_| "The voice model download failed. Check the connection and try again.")?;
    }
    Ok(status(&core).await)
}

/// Streams the model to disk, then keeps it only if its size and SHA-256 match.
async fn download_model(directory: &Path) -> Result<()> {
    std::fs::create_dir_all(directory)?;
    let partial = directory.join(format!("{MODEL_FILE}.part"));
    let result = async {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(DOWNLOAD_TIMEOUT)
            .user_agent("Relay voice check")
            .build()?;
        let response = client.get(MODEL_URL).send().await?.error_for_status()?;
        if response
            .content_length()
            .is_some_and(|length| length != MODEL_BYTES)
        {
            bail!("unexpected voice model size");
        }
        // ponytail: plain file writes between chunks; a one-off download, tokio's fs feature is off.
        let mut file = std::fs::File::create(&partial)?;
        let mut hasher = Sha256::new();
        let mut written = 0_u64;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            written += chunk.len() as u64;
            if written > MODEL_BYTES {
                bail!("the voice model is larger than expected");
            }
            hasher.update(&chunk);
            file.write_all(&chunk)?;
        }
        file.flush()?;
        drop(file);
        verify(written, &format!("{:x}", hasher.finalize()))?;
        std::fs::rename(&partial, directory.join(MODEL_FILE))
            .context("failed to install the voice model")
    }
    .await;
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

fn verify(size: u64, sha256: &str) -> Result<()> {
    if size != MODEL_BYTES || sha256 != MODEL_SHA256 {
        bail!("the voice model failed its checksum");
    }
    Ok(())
}

#[cfg(test)]
mod tests;
