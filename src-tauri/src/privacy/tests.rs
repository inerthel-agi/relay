use super::*;

pub(crate) const ONE_PIXEL_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60, 0x60, 0x60, 0x00,
    0x00, 0x00, 0x04, 0x00, 0x01, 0x27, 0x34, 0x13, 0xa6, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
    0x44, 0xae, 0x42, 0x60, 0x82,
];

pub(crate) fn jpeg_with_gps_metadata() -> Vec<u8> {
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

pub(crate) fn jpeg_with_model_metadata() -> Vec<u8> {
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
fn look_alike_digits_and_invisible_selectors_do_not_hide_numbers() {
    let config = config();
    for text in [
        // Fullwidth digits, as Discord shows them.
        "Mon numéro : ０６ １２ ３４ ５６ ７８",
        // A variation selector between two digits.
        "Mon numéro : 06\u{fe0f} 12 34 56 78",
        // Tag characters and a Mongolian selector inside the number.
        "Mon numéro : 0\u{e0031}6 12 3\u{180b}4 56 78",
        // Mathematical and Arabic-Indic digits.
        "Mon numéro : \u{1d7ce}\u{1d7d4} 12 34 56 78",
        "Mon numéro : ٠٦ ١٢ ٣٤ ٥٦ ٧٨",
    ] {
        assert!(
            classify_text(Some(text), &config)
                .categories
                .contains(&PrivacyCategory::Phone),
            "{text}"
        );
    }
    // A test card number written in fullwidth digits is still a card.
    let card = classify_text(Some("carte ４１１１ １１１１ １１１１ １１１１"), &config);
    assert_eq!(card.classification, PrivacyClassification::Critical);
}

#[test]
fn look_alike_letters_and_fillers_do_not_hide_filtered_words() {
    let config = config();
    for text in [
        "ｈｉｔｌｅｒ",
        "\u{1d421}\u{1d422}\u{1d42d}\u{1d425}\u{1d41e}\u{1d42b}",
        "hit\u{3164}ler",
        "h\u{3164}i\u{3164}t\u{3164}l\u{3164}e\u{3164}r",
        "hi\u{ffa0}tler",
    ] {
        assert_eq!(
            classify_text(Some(text), &config).classification,
            PrivacyClassification::Critical,
            "{text}"
        );
    }
    assert_eq!(
        classify_text(Some("bonne soirée à tous"), &config).classification,
        PrivacyClassification::Safe
    );
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
        "h1tl3r",
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
    // Look-alikes wait for review instead (user's choice, 2026-11-11).
    for text in ["hitier", "h3tler", "hiiitler"] {
        let report = classify_text(Some(text), &config);
        assert_eq!(
            report.classification,
            PrivacyClassification::Medium,
            "{text}"
        );
        assert_eq!(
            action_for(&report, &config),
            PrivacyAction::Review,
            "{text}"
        );
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
        PrivacyClassification::Suspicious
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
                ..PrivacyReport::safe()
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

    // Limits and unknown formats wait for review, they are not blocked
    // (user's choice, 2026-11-11), even with intermediate review off.
    let mut no_intermediate_review = config.clone();
    no_intermediate_review.privacy_review_intermediate = false;
    let mismatch = analyze_image_bytes(b"not an image", None, &config);
    assert!(mismatch.reasons.contains(&"mime_mismatch"));
    assert_eq!(
        action_for(&mismatch, &no_intermediate_review),
        PrivacyAction::Review
    );

    let too_large = image_limit_report(None, &config);
    assert!(too_large.reasons.contains(&"image_limits"));
    assert_eq!(
        action_for(&too_large, &no_intermediate_review),
        PrivacyAction::Review
    );
    // Personal data in the caption still blocks.
    let with_gps = image_limit_report(Some("gps 48.8566, 2.3522"), &config);
    assert_eq!(action_for(&with_gps, &config), PrivacyAction::Block);
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

#[tokio::test]
async fn sampled_frames_lift_only_the_unread_frames_review() {
    let config = config();
    // No frames (no FFmpeg, timeout, partial sample): the review stays.
    for frames in [None, Some(Vec::new())] {
        let held = scan_sampled_frames(frames, &config).await;
        assert!(held.reasons.contains(&"scan_incomplete"));
        assert_eq!(action_for(&held, &config), PrivacyAction::Review);
    }
    // Any finding in any sampled frame keeps its normal effect.
    let found = scan_sampled_frames(
        Some(vec![ONE_PIXEL_PNG.to_vec(), jpeg_with_gps_metadata()]),
        &config,
    )
    .await;
    assert!(found.reasons.contains(&"gps"));
    assert_eq!(found.classification, PrivacyClassification::Sensitive);
    // A clean frame adds nothing, as long as it could be read in full.
    let (still, unread) = scan_first_frame(ONE_PIXEL_PNG, None, &config);
    assert!(!unread);
    let clean = scan_sampled_frames(Some(vec![ONE_PIXEL_PNG.to_vec()]), &config).await;
    assert_eq!(clean.reasons, still.reasons);
}

/// Real FFmpeg and Windows OCR: skipped, like the `media_compat` tests, when absent.
#[cfg(target_os = "windows")]
#[tokio::test]
async fn a_clean_animation_is_checked_on_sampled_frames_instead_of_held() {
    if crate::reaction_trim::ffmpeg_executable().is_none() {
        eprintln!("FFmpeg not found; skipped");
        return;
    }
    let gif = include_bytes!("../../../outputs/samples/motion.gif");
    let signals = inspect_image_windows(gif, true).expect("valid animated GIF fixture");
    assert!(signals.frame_count > 1);
    if !signals.ocr_available {
        eprintln!("Windows OCR not available; skipped");
        return;
    }
    let mut config = config();
    config.privacy_review_intermediate = false;
    let report = analyze_image_bytes_async(gif, None, &config).await;
    assert!(!report.reasons.contains(&"scan_incomplete"), "{report:?}");
    assert_eq!(action_for(&report, &config), PrivacyAction::Allow);
    // Every other check still applies.
    let blocked = analyze_image_bytes_async(gif, Some("hitler"), &config).await;
    assert_eq!(action_for(&blocked, &config), PrivacyAction::Block);
}

/// The level the user runs: Medium from 25 points, High from 50.
fn strict_config() -> AppConfig {
    AppConfig {
        privacy_protection_level: ProtectionLevel::Strict,
        ..config()
    }
}

fn has_category(text: &str, category: PrivacyCategory) -> bool {
    classify_text(Some(text), &strict_config())
        .categories
        .contains(&category)
}

#[test]
fn everyday_counts_are_not_addresses() {
    let config = strict_config();
    for text in [
        "Il y a 3 personnes en live",
        "Il reste 1 place pour ce soir",
        "Je joue avec 3 potes chez moi",
        "J'ai lu 3 bd hier soir",
        "the 1 way to win",
        "I live 2 hours away",
        "chez moi 3 potes",
        "j'habite 2 rues plus loin",
    ] {
        assert!(
            !has_category(text, PrivacyCategory::PostalAddress),
            "{text}"
        );
    }
    for text in [
        "chez moi au 12 Canot Massy",
        "j'habite 4 Canot Massy",
        "j'habite 12 Victor Hugo",
        "j'habite 12 rue Victor Hugo",
        "j'habite 12 rue victor hugo",
        "12 place Bellecour",
        "12 place de la Concorde",
        "5 Penny Lane",
        "1 place bellecour 69002 Lyon",
    ] {
        let report = classify_text(Some(text), &config);
        assert!(
            report.reasons.contains(&"postal_address"),
            "{text}: {report:?}"
        );
        assert_eq!(action_for(&report, &config), PrivacyAction::Block, "{text}");
    }
}

#[test]
fn dates_times_and_amounts_are_not_phone_numbers() {
    for text in [
        "Rendez-vous le 11/11/2026 14h",
        "Rendez-vous le 01/11/2026 20h",
        "le 01-11-2026 20h",
        "le 01.11.2026 9h",
        "2026-11-11 14",
        "1 000 000 000 de vues",
        "12 15 20 25 30",
    ] {
        assert!(!has_category(text, PrivacyCategory::Phone), "{text}");
    }
    for text in [
        "06 12 34 56 78",
        "0612345678",
        "06.12.34.56.78",
        "06-12-34-56-78",
        "+33 6 12 34 56 78",
        "0033 6 12 34 56 78",
        "0033612345678",
    ] {
        assert!(has_category(text, PrivacyCategory::Phone), "{text}");
    }
    let config = strict_config();
    assert_eq!(
        classify_text(Some("Rendez-vous le 01/11/2026 20h"), &config).classification,
        PrivacyClassification::Safe
    );
}

#[test]
fn number_pairs_are_coordinates_only_when_shaped_like_them() {
    let config = strict_config();
    for text in ["Ping 12.5, 13.75", "Prix 1.5, 2.25"] {
        assert_eq!(
            classify_text(Some(text), &config).classification,
            PrivacyClassification::Safe,
            "{text}"
        );
    }
    // Three decimals could be a place: reviewed, not blocked.
    for text in ["10.000 membres et 2.500 en ligne", "Tours 12.345 13.021"] {
        let report = classify_text(Some(text), &config);
        assert_eq!(
            report.classification,
            PrivacyClassification::Medium,
            "{text}"
        );
        assert_eq!(
            action_for(&report, &config),
            PrivacyAction::Review,
            "{text}"
        );
    }
    for text in [
        "48.8566, 2.3522",
        "gps 48.8566, 2.3522",
        "lat 12.5 lon 13.75",
    ] {
        let report = classify_text(Some(text), &config);
        assert_eq!(report.classification, PrivacyClassification::High, "{text}");
        assert_eq!(action_for(&report, &config), PrivacyAction::Block, "{text}");
    }
}

#[test]
fn the_same_text_scanned_twice_counts_once() {
    let config = strict_config();
    let text = Some("appelle-moi au 06 12 34 56 78");
    let once = classify_text(text, &config);
    assert_eq!(once.classification, PrivacyClassification::Medium);
    // Caption, then image scan, then the final gate: still one phone number.
    let mut thrice = once.clone();
    thrice.merge(classify_text(text, &config));
    thrice.merge(classify_text(text, &config));
    thrice.apply_score_policy(&config);
    assert_eq!(thrice.score, once.score);
    assert_eq!(thrice.classification, PrivacyClassification::Medium);
    assert_eq!(action_for(&thrice, &config), PrivacyAction::Review);
    // Distinct findings still add up.
    let mut both = once.clone();
    both.merge(classify_text(Some("ecris a moi@example.com"), &config));
    assert_eq!(both.score, once.score + 15);
    assert!(both.reasons.contains(&"phone") && both.reasons.contains(&"email"));
    // So does a finding the image adds to the caption.
    let mut image = once.clone();
    image.merge(analyze_image_bytes(
        &jpeg_with_gps_metadata(),
        text,
        &config,
    ));
    assert!(image.reasons.contains(&"gps"));
    assert_eq!(action_for(&image, &config), PrivacyAction::Block);
}

#[test]
fn everyday_words_numbers_and_links_are_not_identity_data() {
    let config = strict_config();
    for text in [
        // Documents: everyday words, a Discord link.
        "date limite pour la carte cadeau 12345678",
        "carte du jour, date et social https://discord.com/channels/123456789012345678/987654321098765432",
        // Plates: lowercase words around a number.
        "ping en 100 ms",
        // IPv4: version numbers.
        "Build 1.5.2.1",
        "Relay 1.5.2.1 est sorti",
        "Build 2.10.1.0",
        // Links: hash and ID tokens with long digit runs.
        "https://cdn.discordapp.com/attachments/1234567890123456789/1234567890123456789/cat.png?ex=6734b2c1&is=67336141&hm=0612345678ab0c0d0e0f&",
        "https://media.giphy.com/media/abc/giphy.gif?cid=00123456789abcdef&rid=giphy.gif&ct=g",
        "https://tenor.com/view/cat-dance-gif-25012345",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s",
        "https://www.twitch.tv/videos/1234567890?t=1h2m3s",
    ] {
        let report = classify_text(Some(text), &config);
        assert_eq!(
            report.classification,
            PrivacyClassification::Safe,
            "{text}: {report:?}"
        );
    }
    for (text, category) in [
        (
            "passport number 123456789 date of birth",
            PrivacyCategory::Document,
        ),
        ("AB-123-CD", PrivacyCategory::LicensePlate),
        ("plaque ab-123-cd", PrivacyCategory::LicensePlate),
        ("IP: 203.0.113.42", PrivacyCategory::IpAddress),
        ("Build 2.10.1.0 from 10.0.0.12", PrivacyCategory::IpAddress),
        (
            "https://example.com/reset?token=abc123",
            PrivacyCategory::SensitiveUrl,
        ),
        (
            "https://example.com/call?n=0612345678",
            PrivacyCategory::SensitiveUrl,
        ),
    ] {
        assert!(has_category(text, category), "{text}");
    }
}

#[test]
fn a_look_alike_word_waits_for_review_at_every_level() {
    for level in [
        ProtectionLevel::Balanced,
        ProtectionLevel::Strict,
        ProtectionLevel::Paranoid,
    ] {
        let config = AppConfig {
            privacy_protection_level: level,
            privacy_similarity_boost: 100,
            ..config()
        };
        let look_alike = classify_text(Some("hitier"), &config);
        assert_eq!(look_alike.classification, PrivacyClassification::Medium);
        assert_eq!(action_for(&look_alike, &config), PrivacyAction::Review);
        let exact = classify_text(Some("hitler"), &config);
        assert_eq!(action_for(&exact, &config), PrivacyAction::Block);
    }
}
