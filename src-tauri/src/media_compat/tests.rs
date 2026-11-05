use super::*;

fn minimal_mp4(codec: &[u8; 4]) -> Vec<u8> {
    let mut bytes = b"\0\0\0\x18ftypisom\0\0\0\0isom".to_vec();
    bytes.extend_from_slice(codec);
    bytes
}

#[test]
fn identifies_hevc_sample_entries_in_mp4_probes() {
    assert!(is_hevc_mp4(&minimal_mp4(b"hvc1")));
    assert!(is_hevc_mp4(&minimal_mp4(b"hev1")));
    assert!(!is_hevc_mp4(&minimal_mp4(b"avc1")));
    assert!(!is_hevc_mp4(b"not-an-mp4-hvc1"));
}

#[test]
fn accepts_mp4_mime_types_and_common_extensions() {
    assert!(is_mp4_candidate("clip.bin", "video/mp4; charset=binary"));
    assert!(is_mp4_candidate("clip.MOV", "application/octet-stream"));
    assert!(!is_mp4_candidate("clip.webm", "video/webm"));
}

#[tokio::test]
async fn byte_helper_leaves_h264_bytes_unchanged() {
    let result =
        make_webview_compatible_from_bytes(minimal_mp4(b"avc1"), "clip.mp4", "video/mp4").await;
    assert!(matches!(result, VideoCompatibility::Unchanged));
}

#[tokio::test]
async fn byte_helper_keeps_original_when_synthetic_hevc_cannot_be_transcoded() {
    let result =
        make_webview_compatible_from_bytes(minimal_mp4(b"hvc1"), "clip.mp4", "video/mp4").await;
    assert!(matches!(result, VideoCompatibility::HevcFallback));
}

#[test]
fn reads_the_duration_ffmpeg_prints() {
    let description =
        "Input #0, gif, from 'input':\n  Duration: 00:01:02.50, start: 0.000000, bitrate: 12 kb/s";
    assert_eq!(parse_duration(description), Some(62.5));
    assert_eq!(parse_duration("  Duration: N/A, bitrate: N/A"), None);
    assert_eq!(parse_duration("no description"), None);
}

#[test]
fn removes_partial_transcript_when_transcription_exits_early() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(TRANSCRIPT_FILE);
    let result: Result<()> = with_transcript_file(path.clone(), |path| {
        fs::write(path, "partial private transcript")?;
        bail!("simulated transcription timeout")
    });

    assert!(result.is_err());
    assert!(!path.exists());
}

#[tokio::test]
async fn a_dropped_transcription_keeps_its_slot_until_the_work_ends() {
    static SLOT: Semaphore = Semaphore::const_new(1);
    let (started_tx, started) = std::sync::mpsc::channel();
    let (finish, finish_rx) = std::sync::mpsc::channel::<()>();
    let waiting = tokio::spawn(in_slot(&SLOT, move || {
        started_tx.send(()).unwrap();
        finish_rx.recv().unwrap();
    }));
    tokio::task::spawn_blocking(move || started.recv().unwrap())
        .await
        .unwrap();
    // The bot restarts: the task waiting for the transcription is dropped.
    waiting.abort();
    let _ = waiting.await;
    // FFmpeg is still running: a second transcription must wait.
    assert!(SLOT.try_acquire().is_err());
    finish.send(()).unwrap();
    let permit = tokio::time::timeout(Duration::from_secs(5), SLOT.acquire())
        .await
        .expect("the slot is released once the work ends");
    drop(permit);
}

/// Runs only where FFmpeg is installed; nothing leaves the PC.
#[tokio::test]
async fn takes_three_frames_spread_over_a_video() {
    if crate::reaction_trim::ffmpeg_executable().is_none() {
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let video = directory.path().join("clip.webm");
    let mut command = ffmpeg_command();
    command
        .args([
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=3:size=320x240:rate=10",
        ])
        .arg(&video)
        .stderr(Stdio::null());
    assert!(command.status().unwrap().success());
    let frames = frames_from_bytes(fs::read(&video).unwrap()).await.unwrap();
    assert_eq!(frames.len(), CHECKED_FRAMES);
    for frame in &frames {
        assert!(frame.starts_with(&[0x89, b'P', b'N', b'G']));
    }
    assert_ne!(frames[0], frames[2]);
    assert!(frames_from_bytes(b"not a video".to_vec()).await.is_none());
}

/// A partial sample cannot stand in for the three frames Relay promises to check.
#[tokio::test]
async fn refuses_video_when_ffmpeg_returns_fewer_than_three_sample_frames() {
    if crate::reaction_trim::ffmpeg_executable().is_none() {
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let video = directory.path().join("sparse.webm");
    let mut command = ffmpeg_command();
    command
        .args([
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=320x240:rate=2",
        ])
        .arg(&video)
        .stderr(Stdio::null());
    assert!(command.status().unwrap().success());

    assert!(frames_from_bytes(fs::read(&video).unwrap()).await.is_none());
}

/// A still GIF or a two-frame animation is checked whole, not held.
#[tokio::test]
async fn checks_every_frame_of_a_gif_with_fewer_than_three() {
    if crate::reaction_trim::ffmpeg_executable().is_none() {
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    for (name, source, expected) in [
        ("still.gif", "color=c=red:size=64x64:duration=1:rate=1", 1),
        ("blink.gif", "testsrc=size=64x64:duration=1:rate=2", 2),
    ] {
        let gif = directory.path().join(name);
        let mut command = ffmpeg_command();
        command
            .args(["-loglevel", "error", "-f", "lavfi", "-i", source])
            .arg(&gif)
            .stderr(Stdio::null());
        assert!(command.status().unwrap().success());
        let frames = frames_from_bytes(fs::read(&gif).unwrap()).await;
        assert_eq!(frames.map(|frames| frames.len()), Some(expected), "{name}");
    }
}
