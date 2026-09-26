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
