// No test here reaches a provider or Windows Credential Manager.
use super::*;
use crate::ai::advice::NotSentCause;
use crate::privacy::ForbiddenConcept;

fn sent(request: &AiRequest) -> String {
    format!(
        "{}\n{}\n{}",
        request.system, request.user_text, request.schema
    )
}

#[test]
fn only_the_preset_settings_and_the_wish_leave_the_pc() {
    let mut config = AppConfig {
        watched_channel_id: "123456789012345678".into(),
        music_channel_id: "123456789012345679".into(),
        privacy_filter_exempt_role_ids: vec!["423456789012345678".into()],
        privacy_allowlist: vec!["public@example.com".into()],
        privacy_custom_patterns: vec!["secretpattern".into()],
        privacy_concepts: vec![ForbiddenConcept {
            canonical: "ownword".into(),
            aliases: vec!["ownalias".into()],
            regexes: Vec::new(),
        }],
        ..AppConfig::default()
    };
    config.moderation.trusted_role_ids = vec!["223456789012345678".into()];
    config.moderation.blocked_user_ids = vec!["323456789012345678".into()];
    config.moderation.pack_exclusions = vec!["exceptionword".into()];
    config.moderation.user_cooldown_seconds = 42;

    let request = request(&config, "valider les vidéos", "fr");

    let sent = sent(&request);
    for private in [
        "123456789012345678",
        "123456789012345679",
        "223456789012345678",
        "323456789012345678",
        "423456789012345678",
        "exceptionword",
        "public@example.com",
        "secretpattern",
        "ownword",
        "ownalias",
    ] {
        assert!(!sent.contains(private), "{private} was sent");
    }
    assert!(request.user_text.contains("\"userCooldownSeconds\":42"));
    assert!(request.user_text.contains("<<<\nvalider les vidéos\n>>>"));
    assert!(request.user_text.contains("French"));
    assert!(request.image.is_none());
}

/// Relay's name of a setting, and the plain one the AI sees instead.
const PLAIN_NAMES: [(&str, &str); 5] = [
    ("moderationEnabled", "manualReviewEnabled"),
    ("allowImages", "reviewImagesAndGifs"),
    ("allowVideos", "reviewVideos"),
    ("allowAudio", "reviewAudio"),
    ("reviewOnlySelected", "uncheckedTypesShownDirectly"),
];

#[test]
fn the_ai_sees_names_that_say_what_each_setting_does() {
    let request = request(&AppConfig::default(), "valider les vidéos", "fr");
    let sent = sent(&request);
    // `allowImages` reads as "images may be shown" when it means "images wait for review".
    for (misleading, plain) in PLAIN_NAMES {
        assert!(!sent.contains(misleading), "{misleading} was sent");
        assert!(
            request.user_text.contains(&format!("\"{plain}\":")),
            "{plain}"
        );
        assert!(
            request.schema["properties"]["settings"]["properties"][plain].is_object(),
            "{plain}"
        );
    }
}

/// An answer, with the names the AI sees, that keeps the current values, then applies `edit`.
fn answer(config: &AppConfig, edit: impl FnOnce(&mut Value)) -> Value {
    let mut settings = serde_json::to_value(presets::capture(config)).unwrap();
    let fields = settings.as_object_mut().unwrap();
    for (relay, plain) in PLAIN_NAMES {
        let value = fields.remove(relay).unwrap();
        fields.insert(plain.into(), value);
    }
    edit(&mut settings);
    json!({ "settings": settings, "note": "  Images now pass;\n\tvideos wait.  " })
}

#[test]
fn a_proposal_only_changes_preset_settings_and_relay_lists_the_changes() {
    let mut config = AppConfig {
        watched_channel_id: "123456789012345678".into(),
        ..AppConfig::default()
    };
    config.moderation.trusted_role_ids = vec!["223456789012345678".into()];
    config.moderation.safety_delay_seconds = 5;

    let proposal = parse_proposal(
        &answer(&config, |settings| {
            settings["manualReviewEnabled"] = json!(true);
            settings["reviewImagesAndGifs"] = json!(false);
            settings["uncheckedTypesShownDirectly"] = json!(true);
            settings["userCooldownSeconds"] = json!(10);
        }),
        &config,
    )
    .unwrap();

    assert!(proposal.config.moderation_enabled);
    assert!(!proposal.config.moderation_allow_images);
    assert!(proposal.config.moderation.review_only_selected);
    assert_eq!(proposal.config.moderation.user_cooldown_seconds, 10);
    // Everything outside the preset settings is the user's, untouched.
    assert_eq!(proposal.config.watched_channel_id, "123456789012345678");
    assert_eq!(
        proposal.config.moderation.trusted_role_ids,
        vec!["223456789012345678".to_string()]
    );
    assert_eq!(proposal.config.moderation.safety_delay_seconds, 5);
    assert_eq!(
        proposal.changes,
        vec!["moderationEnabled", "reviewedTypes", "cooldown"]
    );
    assert_eq!(proposal.note, "Images now pass; videos wait.");
    assert!(!proposal.weakens_protection);

    // What the panel reads.
    let wire = serde_json::to_value(SettingsAnswer::Done(Box::new(proposal))).unwrap();
    assert_eq!(wire["state"], "done");
    assert_eq!(wire["changes"][1], "reviewedTypes");
    assert_eq!(wire["weakensProtection"], false);
    assert_eq!(wire["config"]["moderation"]["userCooldownSeconds"], 10);
}

#[test]
fn values_outside_the_allowed_ones_make_the_answer_invalid() {
    let config = AppConfig::default();
    let invalid: [(&str, Value); 7] = [
        ("protectionLevel", json!("extreme")),
        // Accepted by the type, refused by Relay's own rules.
        ("blockThreshold", json!("medium")),
        ("userCooldownSeconds", json!(5000)),
        ("raidLimitPerMinute", json!(-1)),
        ("enabledCategories", json!(["email", "contentFilter"])),
        ("wordPacks", json!(["hate", "hate"])),
        ("minAccountAgeDays", json!("seven")),
    ];
    for (key, value) in invalid {
        let reply = answer(&config, |settings| settings[key] = value.clone());
        assert_eq!(
            parse_proposal(&reply, &config).unwrap_err(),
            AiError::InvalidOutput,
            "{key} = {value}"
        );
    }
    let missing = answer(&config, |settings| {
        settings.as_object_mut().unwrap().remove("blockInvites");
    });
    assert_eq!(
        parse_proposal(&missing, &config).unwrap_err(),
        AiError::InvalidOutput
    );
    assert_eq!(
        parse_proposal(&json!({ "note": "nothing" }), &config).unwrap_err(),
        AiError::InvalidOutput
    );
}

#[test]
fn relay_says_when_a_proposal_lowers_the_protection() {
    let config = AppConfig {
        moderation_enabled: true,
        privacy_scan_enabled: true,
        privacy_auto_delete_blocked_messages: true,
        privacy_protection_level: ProtectionLevel::Strict,
        privacy_review_intermediate: true,
        ..AppConfig::default()
    };
    let lowered: [(&str, Value); 7] = [
        ("manualReviewEnabled", json!(false)),
        ("privacyScanEnabled", json!(false)),
        ("autoDeleteBlockedMessages", json!(false)),
        ("enabledCategories", json!(["email", "phone"])),
        // A laxer scan is lower protection too.
        ("protectionLevel", json!("balanced")),
        ("blockThreshold", json!("critical")),
        ("reviewIntermediate", json!(false)),
    ];
    for (key, value) in lowered {
        let reply = answer(&config, |settings| settings[key] = value.clone());
        assert!(
            parse_proposal(&reply, &config).unwrap().weakens_protection,
            "{key} = {value}"
        );
    }
    let tightened = answer(&config, |settings| {
        settings["protectionLevel"] = json!("paranoid");
        settings["userCooldownSeconds"] = json!(30);
    });
    assert!(
        !parse_proposal(&tightened, &config)
            .unwrap()
            .weakens_protection
    );
}

#[tokio::test]
async fn nothing_is_sent_without_a_provider_or_with_personal_data() {
    let mut config = AppConfig::default();
    assert_eq!(
        suggest(&config, "valider les vidéos", "fr").await,
        SettingsAnswer::Failed {
            cause: AiError::NoProvider
        }
    );
    config.ai.provider = Some(super::super::AiProvider::Anthropic);
    // Both stop before the API key is even read.
    assert_eq!(
        suggest(&config, "   ", "fr").await,
        SettingsAnswer::NotSent {
            cause: NotSentCause::Empty
        }
    );
    assert_eq!(
        suggest(&config, "bloque prenom.nom@example.com", "fr").await,
        SettingsAnswer::NotSent {
            cause: NotSentCause::PersonalData
        }
    );
}
