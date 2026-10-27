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
