use super::*;

fn checking() -> AppConfig {
    AppConfig {
        voice_scan_enabled: true,
        privacy_scan_enabled: true,
        ..AppConfig::default()
    }
}

#[test]
fn only_sounds_are_checked_and_only_when_turned_on() {
    let config = checking();
    assert!(applies(&config, MediaKind::Audio));
    assert!(applies(&config, MediaKind::Video));
    assert!(!applies(&config, MediaKind::Image));
    assert!(!applies(&config, MediaKind::Gif));
    assert!(!applies(&AppConfig::default(), MediaKind::Audio));
    // Without any privacy rule, the text could not change anything.
    let nothing_to_find = AppConfig {
        privacy_scan_enabled: false,
        ..checking()
    };
    assert!(!applies(&nothing_to_find, MediaKind::Audio));
}

#[test]
fn a_spoken_phone_number_is_found_in_the_transcript() {
    // The real transcript FFmpeg's whisper filter gave for a French voice message.
    let transcript = "Salut tout le monde ! Mon numéro c'est le 06 12 34 56 78 et j'habite aux 12 rues de la paix à Paris.";
    let VoiceCheck::Checked { report, .. } = judge(transcript, &checking()) else {
        panic!("a short transcript is checked");
    };
    assert_ne!(report.classification, privacy::PrivacyClassification::Safe);
    assert!(report.categories.contains(&privacy::PrivacyCategory::Phone));
    // The base model writes the pauses of a dictated number as commas.
    let transcript = "Salut, si tu veux me joindre, mon numéro c'est le 06, 12, 34, 56, 78 et j'habite au 12 rue de la paix à Lyon.";
    let VoiceCheck::Checked { report, .. } = judge(transcript, &checking()) else {
        panic!("a short transcript is checked");
    };
    assert!(report.categories.contains(&privacy::PrivacyCategory::Phone));
    let VoiceCheck::Checked { report, .. } =
        judge("Salut tout le monde, bon stream !", &checking())
    else {
        panic!("a short transcript is checked");
    };
    assert_eq!(report.classification, privacy::PrivacyClassification::Safe);
    // Several minutes of speech are held, never checked in part.
    let long = "bla ".repeat(privacy::PRIVACY_TEXT_LIMIT);
    assert_eq!(judge(&long, &checking()), VoiceCheck::Unchecked);
}

#[tokio::test]
async fn without_the_model_nothing_runs_and_the_sound_waits() {
    let directory = tempfile::tempdir().unwrap();
    let source = (
        "https://relay.invalid/voice.ogg",
        "",
        Some(b"OggS".to_vec()),
    );
    assert_eq!(
        check(directory.path().to_path_buf(), source, &checking()).await,
        VoiceCheck::Unchecked
    );
    // A truncated or replaced model file is not trusted either.
    std::fs::write(directory.path().join(MODEL_FILE), b"not a model").unwrap();
    assert!(!model_ready(directory.path()));
}

#[test]
fn only_the_published_model_is_kept() {
    assert!(verify(MODEL_BYTES, MODEL_SHA256).is_ok());
    assert!(verify(MODEL_BYTES - 1, MODEL_SHA256).is_err());
    assert!(verify(MODEL_BYTES, &"0".repeat(64)).is_err());
}

/// Runs the real transcription. Needs FFmpeg 8 with Whisper, the model in
/// `RELAY_WHISPER_MODEL_DIR` and a spoken sample in `RELAY_WHISPER_SAMPLE`.
#[tokio::test]
#[ignore = "needs FFmpeg with Whisper, the voice model and a spoken sample"]
async fn transcribes_speech_on_this_pc() {
    let directory = PathBuf::from(std::env::var("RELAY_WHISPER_MODEL_DIR").unwrap());
    let sample = std::fs::read(std::env::var("RELAY_WHISPER_SAMPLE").unwrap()).unwrap();
    assert!(media_compat::ffmpeg_has_whisper());
    assert!(model_ready(&directory));
    let started = std::time::Instant::now();
    let check = check(directory, ("", "", Some(sample)), &checking()).await;
    let VoiceCheck::Checked { report, .. } = check else {
        panic!("the sample was not transcribed: {check:?}");
    };
    assert!(
        report.categories.contains(&privacy::PrivacyCategory::Phone),
        "{report:?}"
    );
    println!("transcribed and checked in {:?}", started.elapsed());
}
