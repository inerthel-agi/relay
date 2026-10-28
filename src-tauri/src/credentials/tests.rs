// These tests never touch Windows Credential Manager.
use super::*;

#[test]
fn each_ai_provider_has_its_own_credential_entry() {
    let openai = ai_api_key_account(AiProvider::OpenAi);
    let anthropic = ai_api_key_account(AiProvider::Anthropic);
    assert_ne!(openai, anthropic);
    for account in [
        DISCORD_ACCOUNT,
        YOUTUBE_API_KEY_ACCOUNT,
        RELAY_SECRET_ACCOUNT,
        OBS_WEBSOCKET_ACCOUNT,
    ] {
        assert_ne!(openai, account);
        assert_ne!(anthropic, account);
    }
}

#[test]
fn ai_api_keys_are_checked_for_shape_only() {
    validate_ai_api_key(&format!("sk-{}", "a".repeat(40))).unwrap();
    validate_ai_api_key(&format!("sk-ant-api03-{}", "B_c-9".repeat(20))).unwrap();
    for invalid in [
        "short".to_owned(),
        "x".repeat(513),
        format!("sk-{} trailing", "a".repeat(30)),
        format!("sk-{}\n", "a".repeat(30)),
    ] {
        let error = validate_ai_api_key(&invalid).unwrap_err().to_string();
        assert!(
            !error.contains(&invalid),
            "the key must never appear in errors"
        );
    }
}

#[test]
fn credential_status_never_carries_a_key() {
    let status = CredentialStatus {
        configured: false,
        source: None,
        client_id: None,
        youtube_configured: false,
        obs_password_configured: false,
        openai_configured: true,
        anthropic_configured: false,
    };
    let serialized = serde_json::to_value(&status).unwrap();
    assert_eq!(serialized["openaiConfigured"], true);
    assert_eq!(serialized["anthropicConfigured"], false);
    assert_eq!(serialized.as_object().unwrap().len(), 7);
}

#[test]
fn whether_an_ai_key_is_saved_is_read_once_then_remembered() {
    let cache = Mutex::new([None, None]);
    let mut reads = 0;
    // A failed read is not remembered: the next status tries again.
    assert!(!remembered(&cache, 0, || {
        reads += 1;
        None
    }));
    assert!(remembered(&cache, 0, || {
        reads += 1;
        Some(true)
    }));
    assert!(remembered(&cache, 0, || {
        reads += 1;
        Some(false)
    }));
    assert_eq!(reads, 2);
    // Each provider has its own slot.
    assert!(!remembered(&cache, 1, || Some(false)));
}
