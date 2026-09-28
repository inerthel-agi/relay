use super::*;

const ONE_PIXEL_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60, 0x60, 0x60, 0x00,
    0x00, 0x00, 0x04, 0x00, 0x01, 0x27, 0x34, 0x13, 0xa6, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
    0x44, 0xae, 0x42, 0x60, 0x82,
];

fn jpeg_with_gps_metadata() -> Vec<u8> {
    let mut tiff = vec![0x49, 0x49, 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00];
    tiff.extend_from_slice(&[0x01, 0x00, 0x25, 0x88, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00]);
    tiff.extend_from_slice(&[0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    tiff.extend_from_slice(&[0x02, 0x00]);
    tiff.extend_from_slice(&[
        0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, b'N', 0x00, 0x00, 0x00,
    ]);
    tiff.extend_from_slice(&[
        0x02, 0x00, 0x05, 0x00, 0x03, 0x00, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00,
    ]);
    tiff.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    tiff.extend_from_slice(&[
        1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0,
    ]);
    let payload_len = 6 + tiff.len();
    let segment_len = payload_len + 2;
    let mut jpeg = vec![
        0xff,
        0xd8,
        0xff,
        0xe1,
        (segment_len as u16 >> 8) as u8,
        segment_len as u8,
    ];
    jpeg.extend_from_slice(b"Exif\0\0");
    jpeg.extend_from_slice(&tiff);
    jpeg.extend_from_slice(&[0xff, 0xd9]);
    jpeg
}

fn jpeg_with_model_metadata() -> Vec<u8> {
    let mut tiff = vec![0x49, 0x49, 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00];
    tiff.extend_from_slice(&[0x01, 0x00]);
    tiff.extend_from_slice(&[
        0x10, 0x01, 0x02, 0x00, 0x06, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x00,
    ]);
    tiff.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    tiff.extend_from_slice(b"Phone\0");
    let payload_len = 6 + tiff.len();
    let segment_len = payload_len + 2;
    let mut jpeg = vec![
        0xff,
        0xd8,
        0xff,
        0xe1,
        (segment_len as u16 >> 8) as u8,
        segment_len as u8,
    ];
    jpeg.extend_from_slice(b"Exif\0\0");
    jpeg.extend_from_slice(&tiff);
    jpeg.extend_from_slice(&[0xff, 0xd9]);
    jpeg
}

fn config() -> AppConfig {
    AppConfig {
        privacy_scan_enabled: true,
        privacy_concepts: vec![ForbiddenConcept {
            canonical: "hitler".into(),
            aliases: vec!["austrian painter".into()],
            regexes: Vec::new(),
        }],
        ..AppConfig::default()
    }
}

#[test]
fn safe_text_does_not_overclassify() {
    let config = config();
    for text in [
        "a landscape",
        "generic building",
        "public monument",
        "meme 123 456",
    ] {
        assert_eq!(
            classify_text(Some(text), &config).classification,
            PrivacyClassification::Safe
        );
    }
}

#[test]
fn address_and_coordinates_are_sensitive() {
    let config = config();
    assert_eq!(
        classify_text(Some("12 Main Street 75001 Paris"), &config).classification,
        PrivacyClassification::Sensitive
    );
    assert_eq!(
        classify_text(Some("48.8566, 2.3522"), &config).classification,
        PrivacyClassification::Sensitive
    );
}

#[test]
fn probable_addresses_are_sensitive_across_common_formats() {
    let config = config();
    for text in [
        "1 rue canot massy",
        "1, rue Canot - Massy",
        "1 r.u.e Canot Massy",
        "adresse : 1 Canot Massy",
        "6\nrue\ncanot\nmassy\n91300",
    ] {
        assert_eq!(
            classify_text(Some(text), &config).classification,
            PrivacyClassification::High,
            "{text}"
        );
    }
}

#[test]
fn partial_addresses_stay_low_without_overclassifying_ordinary_numbers() {
    let config = config();
    for text in ["5 rue", "5 avenue", "5 r.u.e"] {
        let report = classify_text(Some(text), &config);
        assert_eq!(report.classification, PrivacyClassification::Low, "{text}");
        assert!(report.reasons.contains(&"partial_address"), "{text}");
    }
    for text in [
        "5 martin",
        "5 m-a-r-t-i-n",
        "5 minutes",
        "5 euros",
        "5 kilometres",
        "version 1.2.3.4",
        "meme 123 456",
    ] {
        assert_eq!(
            classify_text(Some(text), &config).classification,
            PrivacyClassification::Safe,
            "{text}"
        );
    }
}

#[test]
fn doxxing_text_formats_classify_contextually() {
    let config = config();
    for text in ["IP: 203.0.113.42", "[2001:db8::1]"] {
        assert_eq!(
            classify_text(Some(text), &config).classification,
            PrivacyClassification::Medium,
            "{text}"
        );
    }
    assert_eq!(
        classify_text(Some("12 Main Street 75001 Paris"), &config).classification,
        PrivacyClassification::High
    );
    for text in [
        "version 1.2.3",
        "Paris, France",
        "public monument in a city",
    ] {
        assert_eq!(
            classify_text(Some(text), &config).classification,
            PrivacyClassification::Safe,
            "{text}"
        );
    }
}

#[test]
fn game_map_is_never_sensitive() {
    let config = config();
    assert_ne!(
        classify_text(Some("game map"), &config).classification,
        PrivacyClassification::Sensitive
    );
}

#[test]
fn concepts_match_explicit_variants_only() {
    let config = config();
    for text in [
        "hitler",
        "h1tler",
        "hitier",
        "h1tl3r",
        "h3tler",
        "hiiitler",
        "hi tler",
        "hit ler",
        "austrian painter",
        "austrian-painter",
        "aus trian painter",
        "a u s t r i a n painter",
        "h i t l e r",
    ] {
        let report = classify_text(Some(text), &config);
        assert!(
            report.classification.rank() >= PrivacyClassification::High.rank(),
            "{text}"
        );
        assert_eq!(action_for(&report, &config), PrivacyAction::Block, "{text}");
    }
    assert_eq!(
        classify_text(Some("hit"), &config).classification,
        PrivacyClassification::Safe
    );
    assert_eq!(
        classify_text(Some("unrelated"), &config).classification,
        PrivacyClassification::Safe
    );
    for text in ["hitter", "hither"] {
        assert_eq!(
            classify_text(Some(text), &config).classification,
            PrivacyClassification::Safe,
            "{text}"
        );
    }
    let mut long_text = "a".repeat(200);
    long_text.push_str(" h1tler");
    assert_eq!(
        classify_text(Some(&long_text), &config).classification,
        PrivacyClassification::Critical
    );
    let truncated = "a".repeat(PRIVACY_TEXT_LIMIT + 1);
    let truncated_report = classify_text(Some(&truncated), &config);
    assert_eq!(truncated_report.classification, PrivacyClassification::Low);
    assert!(truncated_report.reasons.contains(&"scan_incomplete"));
    let concept_before_cap = format!("h1tler {}", "a".repeat(PRIVACY_TEXT_LIMIT + 1));
    let concept_report = classify_text(Some(&concept_before_cap), &config);
    assert_eq!(
        concept_report.classification,
        PrivacyClassification::Critical
    );
    assert!(concept_report.reasons.contains(&"scan_incomplete"));
    let punctuation_report = classify_text(
        Some(&format!("{}hitler", ".".repeat(PRIVACY_TEXT_LIMIT + 1))),
        &config,
    );
    assert_eq!(
        punctuation_report.classification,
        PrivacyClassification::Low
    );
    assert!(punctuation_report.reasons.contains(&"scan_incomplete"));
    assert!(
        ForbiddenConcept {
            canonical: "1234".into(),
            aliases: Vec::new(),
            regexes: Vec::new(),
        }
        .validate()
        .is_err()
    );
    assert!(
        ForbiddenConcept {
            canonical: "okay".into(),
            aliases: vec!["5678".into()],
            regexes: Vec::new(),
        }
        .validate()
        .is_err()
    );
}

#[test]
fn regex_filters_block_directly_and_similarity_boosts_score() {
    let mut config = config();
    config.privacy_scan_enabled = false;
    config.privacy_concepts[0].regexes = vec![r"\bsecret[-_ ]phrase\b".into()];
    let regex_report = classify_text(Some("SECRET_phrase"), &config);
    assert_eq!(regex_report.classification, PrivacyClassification::Critical);
    assert!(regex_report.reasons.contains(&"forbidden_regex"));
    config.privacy_concepts[0].regexes = vec![r"^---$".into()];
    assert_eq!(
        classify_text(Some("---"), &config).classification,
        PrivacyClassification::Critical
    );

    config.privacy_concepts[0].regexes.clear();
    config.privacy_similarity_boost = 2;
    let weak_similarity = classify_text(Some("hitier"), &config);
    assert_eq!(
        weak_similarity.classification,
        PrivacyClassification::Suspicious
    );
    assert!(weak_similarity.reasons.contains(&"forbidden_similarity"));

    config.privacy_similarity_boost = 4;
    let strong_similarity = classify_text(Some("hitier"), &config);
    assert_eq!(
        strong_similarity.classification,
        PrivacyClassification::Sensitive
    );
    assert!(strong_similarity.reasons.contains(&"similarity_score"));

    for regexes in [vec!["(".into()], vec![".*".into()]] {
        assert!(
            ForbiddenConcept {
                canonical: "filter".into(),
                aliases: Vec::new(),
                regexes,
            }
            .validate()
            .is_err()
        );
    }
}

#[test]
fn three_character_filter_words_match_exact_separators_only() {
    let filter_config = AppConfig {
        privacy_scan_enabled: false,
        privacy_concepts: vec![ForbiddenConcept {
            canonical: "fdp".into(),
            aliases: Vec::new(),
            regexes: Vec::new(),
        }],
        ..AppConfig::default()
    };
    for text in ["fdp", "f.d.p", "f-d-p", "f d p"] {
        assert_eq!(
            classify_text(Some(text), &filter_config).classification,
            PrivacyClassification::Critical,
            "{text}"
        );
    }
    for text in ["fd", "ffdp", "fdpp", "unrelated"] {
        assert_eq!(
            classify_text(Some(text), &filter_config).classification,
            PrivacyClassification::Safe,
            "{text}"
        );
    }
    assert!(
        ForbiddenConcept {
            canonical: "fdp".into(),
            aliases: vec!["f.d.p".into()],
            regexes: Vec::new(),
        }
        .validate()
        .is_ok()
    );
    assert!(
        ForbiddenConcept {
            canonical: "fd".into(),
            aliases: Vec::new(),
            regexes: Vec::new(),
        }
        .validate()
        .is_err()
    );
    assert!(
        ForbiddenConcept {
            canonical: "filter".into(),
            aliases: vec!["fd".into()],
            regexes: Vec::new(),
        }
        .validate()
        .is_err()
    );
    assert_eq!(
        classify_text(Some("h1tl3r"), &config()).classification,
        PrivacyClassification::Critical
    );
}

#[test]
fn disabled_scan_bypasses_all_rules() {
    let mut config = config();
    config.privacy_scan_enabled = false;
    config.privacy_concepts.clear();
    assert_eq!(
        classify_text(Some("12 Main Street 75001 Paris hitler"), &config).classification,
        PrivacyClassification::Safe
    );
}

#[test]
fn filter_words_run_without_local_image_scan() {
    let mut config = config();
    config.privacy_scan_enabled = false;
    assert!(privacy_rules_enabled(&config));
    assert_eq!(
        classify_text(Some("hitler"), &config).classification,
        PrivacyClassification::Critical
    );

    config.privacy_concepts[0].canonical = "private".into();
    config.privacy_concepts[0].aliases.clear();
    config.privacy_concepts[0].regexes = vec![r"\bsecret\b".into()];
    let regex_report = classify_text(Some("a SECRET message"), &config);
    assert_eq!(regex_report.classification, PrivacyClassification::Critical);
    assert!(regex_report.reasons.contains(&"forbidden_regex"));
}

#[test]
fn exempt_roles_clear_only_filter_concepts() {
    let mut config = config();
    config.privacy_scan_enabled = true;
    config.privacy_filter_exempt_role_ids = vec!["123456789012345678".into()];
    let roles = vec!["123456789012345678".into()];
    assert!(has_exempt_role(&config, &roles));
    let scoped = scoped_config_for_roles(&config, &roles);
    assert!(scoped.privacy_concepts.is_empty());
    assert!(scoped.privacy_scan_enabled);
    assert_eq!(
        classify_text(Some("hitler"), &scoped).classification,
        PrivacyClassification::Safe
    );
    assert_eq!(
        classify_text(Some("gps 48.8566, 2.3522"), &scoped).classification,
        PrivacyClassification::Sensitive
    );
    assert_eq!(
        classify_text(Some("203.0.113.42"), &scoped).classification,
        PrivacyClassification::Medium
    );
    assert!(!has_exempt_role(&config, &Vec::new()));
    assert!(!has_exempt_role(&config, &["223456789012345678".into()]));
    let signature = config_signature(&config);
    let mut changed = config.clone();
    changed.privacy_filter_exempt_role_ids = vec!["223456789012345678".into()];
    assert_ne!(signature, config_signature(&changed));
}

#[test]
fn filter_signals_can_be_removed_without_dropping_gps() {
    let mut report = PrivacyReport::sensitive("forbidden_concept");
    report.merge(PrivacyReport::sensitive("gps"));
    let scoped = report.without_filter_signals();
    assert_eq!(scoped.classification, PrivacyClassification::Sensitive);
    assert_eq!(scoped.reasons, vec!["gps"]);

    let report = PrivacyReport::sensitive("forbidden_regex");
    let scoped = report.without_filter_signals();
    assert_eq!(scoped.classification, PrivacyClassification::Safe);
    assert!(scoped.reasons.is_empty());

    let custom = PrivacyReport::sensitive("custom_pattern");
    let scoped = custom.clone().without_filter_signals();
    assert_eq!(scoped, custom);
}

#[test]
fn common_visual_text_stays_safe_or_weak() {
    let config = config();
    for text in [
        "Street Fighter 6 meme",
        "public monument city",
        "landscape city landmark",
        "meme 12.5 3.2",
    ] {
        assert_eq!(
            classify_text(Some(text), &config).classification,
            PrivacyClassification::Safe,
            "{text}"
        );
    }
    assert_ne!(
        classify_text(Some("meme, 12.5 3.2"), &config).classification,
        PrivacyClassification::Sensitive
    );
    assert_ne!(
        classify_text(Some("flat meme 12.5 3.2"), &config).classification,
        PrivacyClassification::Sensitive
    );
    assert_eq!(
        classify_text(Some("gps 48.8566, 2.3522"), &config).classification,
        PrivacyClassification::Sensitive
    );
    assert_eq!(
        classify_text(Some("passport landmark company city"), &config).classification,
        PrivacyClassification::Safe
    );
}

#[test]
fn contact_signals_are_scored_without_treating_ordinary_numbers_as_phone_numbers() {
    let config = config();
    for email in ["person@example.com", "person [at] example [dot] com"] {
        let report = classify_text(Some(email), &config);
        assert_eq!(report.classification, PrivacyClassification::Low, "{email}");
        assert!(report.categories.contains(&PrivacyCategory::Email));
    }
    for phone in ["06 12 34 56 78", "+33 (0)6 12 34 56 78"] {
        let report = classify_text(Some(phone), &config);
        assert_eq!(
            report.classification,
            PrivacyClassification::Medium,
            "{phone}"
        );
        assert!(report.categories.contains(&PrivacyCategory::Phone));
    }
    for ordinary in [
        "order 123456",
        "version 1.2.3.4",
        "Discord 123456789012345678",
    ] {
        assert_eq!(
            classify_text(Some(ordinary), &config).classification,
            PrivacyClassification::Safe,
            "{ordinary}"
        );
    }
}

#[test]
fn financial_identifiers_use_checksum_validation() {
    let config = config();
    let iban = classify_text(Some("GB82 WEST 1234 5698 7654 32"), &config);
    assert_eq!(iban.classification, PrivacyClassification::High);
    assert!(iban.reasons.contains(&"iban"));

    let card = classify_text(Some("4111 1111 1111 1111"), &config);
    assert_eq!(card.classification, PrivacyClassification::Critical);
    assert!(card.reasons.contains(&"payment_card"));

    for invalid in ["GB82 WEST 1234 5698 7654 31", "4111 1111 1111 1112"] {
        assert_eq!(
            classify_text(Some(invalid), &config).classification,
            PrivacyClassification::Safe,
            "{invalid}"
        );
    }
}

#[test]
fn context_combinations_raise_risk_and_plate_detection_stays_bounded() {
    let config = config();
    let combined = classify_text(
        Some("name: Jean Dupont, 12 rue Victor Hugo 75001 Paris, 06 12 34 56 78"),
        &config,
    );
    assert_eq!(combined.classification, PrivacyClassification::Critical);
    assert_eq!(action_for(&combined, &config), PrivacyAction::Block);

    let plate = classify_text(Some("plaque AB-123-CD"), &config);
    assert_eq!(plate.classification, PrivacyClassification::Medium);
    assert!(plate.categories.contains(&PrivacyCategory::LicensePlate));
    assert_eq!(
        classify_text(Some("build AB123CD"), &config).classification,
        PrivacyClassification::Safe
    );
}

#[test]
fn sensitive_urls_custom_patterns_allowlist_and_category_toggles_work() {
    let mut config = config();
    let url = classify_text(Some("https://example.com/reset?token=abc123"), &config);
    assert_eq!(url.classification, PrivacyClassification::Medium);
    assert!(url.categories.contains(&PrivacyCategory::SensitiveUrl));

    config.privacy_custom_patterns = vec!["oldnickname".into(), "private street".into()];
    for text in [
        "old\u{200b}nickname",
        "private-street",
        "oldnicknam\u{0435}",
    ] {
        let report = classify_text(Some(text), &config);
        assert_eq!(report.classification, PrivacyClassification::High, "{text}");
        assert!(report.categories.contains(&PrivacyCategory::CustomPattern));
    }

    config.privacy_allowlist = vec!["public@example.com".into()];
    assert_eq!(
        classify_text(Some("public@example.com"), &config).classification,
        PrivacyClassification::Safe
    );
    config.privacy_enabled_categories = vec![PrivacyCategory::Phone];
    assert_eq!(
        classify_text(Some("private@example.com"), &config).classification,
        PrivacyClassification::Safe
    );
}

#[test]
fn protection_profiles_and_actions_follow_the_five_level_policy() {
    assert_eq!(
        risk_for_score(25, ProtectionLevel::Balanced),
        PrivacyClassification::Low
    );
    assert_eq!(
        risk_for_score(25, ProtectionLevel::Strict),
        PrivacyClassification::Medium
    );
    assert_eq!(
        risk_for_score(25, ProtectionLevel::Paranoid),
        PrivacyClassification::Medium
    );
    let mut config = config();
    let medium = PrivacyReport::suspicious("phone");
    assert_eq!(action_for(&medium, &config), PrivacyAction::Review);
    config.privacy_review_intermediate = false;
    assert_eq!(action_for(&medium, &config), PrivacyAction::Allow);
    config.privacy_block_threshold = PrivacyClassification::Critical;
    assert_eq!(
        action_for(&PrivacyReport::sensitive("gps"), &config),
        PrivacyAction::Review
    );
    assert_eq!(
        action_for(
            &PrivacyReport {
                classification: PrivacyClassification::Critical,
                score: 100,
                categories: vec![PrivacyCategory::Financial],
                reasons: vec!["payment_card"],
                config_signature: None,
            },
            &config,
        ),
        PrivacyAction::Block
    );

    let mut combined = PrivacyReport::suspicious("phone");
    combined.merge(PrivacyReport::suspicious("postal_address"));
    assert_eq!(combined.classification, PrivacyClassification::Medium);
    combined.apply_score_policy(&config);
    assert_eq!(combined.classification, PrivacyClassification::High);
}

#[test]
fn image_metadata_mime_and_resource_limits_are_classified_locally() {
    let config = config();
    let metadata = analyze_image_bytes(&jpeg_with_model_metadata(), None, &config);
    assert!(
        metadata
            .categories
            .contains(&PrivacyCategory::ImageMetadata)
    );
    assert!(metadata.reasons.contains(&"exif_metadata"));

    let mismatch = analyze_image_bytes(b"not an image", None, &config);
    assert_eq!(mismatch.classification, PrivacyClassification::High);
    assert!(mismatch.reasons.contains(&"mime_mismatch"));

    let too_large = image_limit_report(None, &config);
    assert_eq!(too_large.classification, PrivacyClassification::High);
    assert_eq!(action_for(&too_large, &config), PrivacyAction::Block);
}

#[test]
fn ordinary_image_fixture_is_safe() {
    let mut config = config();
    config.privacy_scan_enabled = false;
    let report = analyze_image_bytes(ONE_PIXEL_PNG, None, &config);
    assert_eq!(report.classification, PrivacyClassification::Safe);
}

#[test]
fn gps_fixture_is_sensitive_before_publication() {
    let report = analyze_image_bytes(&jpeg_with_gps_metadata(), None, &config());
    assert_eq!(report.classification, PrivacyClassification::Sensitive);
    assert!(report.reasons.contains(&"gps"));
}

#[test]
fn malformed_exif_is_reviewable_incomplete() {
    let malformed = b"\xff\xd8\xff\xe1\0\x08Exif\0\0\0";
    let report = analyze_image_bytes(malformed, None, &config());
    assert_eq!(report.classification, PrivacyClassification::Low);
    assert!(report.reasons.contains(&"scan_incomplete"));
    assert_eq!(action_for(&report, &config()), PrivacyAction::Review);
    let mut no_intermediate_review = config();
    no_intermediate_review.privacy_review_intermediate = false;
    assert_eq!(
        action_for(&report, &no_intermediate_review),
        PrivacyAction::Review
    );
    assert_eq!(
        action_for(
            &PrivacyReport::suspicious("image_fetch_unavailable"),
            &no_intermediate_review
        ),
        PrivacyAction::Review
    );
}

#[test]
fn gif_without_exif_does_not_claim_an_incomplete_scan() {
    let gif = include_bytes!("../../../outputs/samples/motion.gif");
    assert!(!parse_exif(gif).incomplete);
    let mut config = config();
    config
        .privacy_enabled_categories
        .retain(|category| *category != PrivacyCategory::Ocr);
    let report = analyze_image_bytes(gif, None, &config);
    assert!(!report.reasons.contains(&"scan_incomplete"));
    assert_eq!(action_for(&report, &config), PrivacyAction::Allow);
}

#[cfg(target_os = "windows")]
#[test]
fn animated_image_requires_review_even_when_first_frame_ocr_succeeds() {
    let gif = include_bytes!("../../../outputs/samples/motion.gif");
    let signals = inspect_image_windows(gif, true).expect("valid animated GIF fixture");
    assert!(signals.frame_count > 1);
    // Missing optional Windows OCR language packs must also keep the image in review.
    let mut config = config();
    config.privacy_review_intermediate = false;
    let report = analyze_image_bytes(gif, None, &config);
    assert!(report.reasons.contains(&"scan_incomplete"));
    assert_eq!(action_for(&report, &config), PrivacyAction::Review);

    let blocked = analyze_image_bytes(gif, Some("hitler"), &config);
    assert_eq!(action_for(&blocked, &config), PrivacyAction::Block);
}

#[cfg(target_os = "windows")]
#[test]
fn library_gifs_only_need_their_first_frame_scanned() {
    let gif = include_bytes!("../../../outputs/samples/motion.gif");
    let signals = inspect_image_windows(gif, true).expect("valid animated GIF fixture");
    let mut config = config();
    config.privacy_review_intermediate = false;
    let report = analyze_image(gif, None, &config, true);
    if signals.ocr_available {
        assert!(!report.reasons.contains(&"scan_incomplete"));
        assert_eq!(action_for(&report, &config), PrivacyAction::Allow);
    } else {
        // Without Windows OCR even the first frame is unread: review stays.
        assert_eq!(action_for(&report, &config), PrivacyAction::Review);
    }
    // Every other check still applies.
    let blocked = analyze_image(gif, Some("hitler"), &config, true);
    assert_eq!(action_for(&blocked, &config), PrivacyAction::Block);
}
