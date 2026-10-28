// No test here reaches a provider, Discord or Windows Credential Manager.
use super::*;

fn settings() -> AiSettings {
    AiSettings {
        image_guard: true,
        personal_data_relay: true,
        ..AiSettings::default()
    }
}

fn done(verdict: AiVerdict, categories: Vec<AiCategory>) -> AiAdvice {
    AiAdvice::Done {
        verdict,
        categories,
        reason: String::new(),
    }
}

#[test]
fn the_relay_scope_follows_the_type_of_image() {
    let settings = settings();
    // Photos, videos and unfinished scans by default; screenshots not.
    assert_eq!(wanted(&settings, false, true, false), (true, true));
    assert_eq!(wanted(&settings, false, false, false), (true, false));
    assert_eq!(wanted(&settings, false, false, true), (true, true));
    // GIFs and videos are checked by the image check only when asked.
    assert_eq!(wanted(&settings, true, false, false), (false, true));
    let animated = AiSettings {
        image_guard_animated: true,
        personal_data_relay: false,
        ..settings.clone()
    };
    assert_eq!(wanted(&animated, true, false, false), (true, false));
    let no_category = AiSettings {
        image_guard_categories: Vec::new(),
        ..settings
    };
    assert!(!wanted(&no_category, false, true, false).0);
}

#[test]
fn only_categories_turned_on_block() {
    let mut settings = settings();
    assert_eq!(
        free_hit(&settings, &["violence"]),
        Some(ImageGuardCategory::Violence)
    );
    assert_eq!(free_hit(&settings, &["harassment"]), None);
    assert_eq!(
        paid_hit(&settings, &[AiCategory::Hate]),
        Some(ImageGuardCategory::Hate)
    );
    assert_eq!(
        paid_hit(&settings, &[AiCategory::Threat]),
        Some(ImageGuardCategory::Harassment)
    );
    assert_eq!(
        paid_hit(&settings, &[AiCategory::PersonalData, AiCategory::Other]),
        None
    );
    settings.image_guard_categories = vec![ImageGuardCategory::Spam];
    assert_eq!(
        free_hit(&settings, &["sexual", "violence", "self_harm"]),
        None
    );
    assert_eq!(paid_hit(&settings, &[AiCategory::Hate]), None);
    assert_eq!(
        paid_hit(&settings, &[AiCategory::Spam]),
        Some(ImageGuardCategory::Spam)
    );
}

#[test]
fn a_clear_problem_or_a_doubt_blocks_with_its_category_a_failure_holds() {
    let settings = settings();
    let hate = vec![AiCategory::Hate];
    assert_eq!(
        from_advice(
            &settings,
            true,
            false,
            &done(AiVerdict::ClearProblem, hate.clone())
        ),
        Some(Outcome::Block("ai_image_blocked_hate"))
    );
    assert_eq!(
        from_advice(
            &settings,
            true,
            false,
            &done(AiVerdict::Ambiguous, hate.clone())
        ),
        Some(Outcome::Block("ai_image_blocked_hate"))
    );
    assert_eq!(
        from_advice(
            &settings,
            true,
            false,
            &done(AiVerdict::NoIssueSeen, hate.clone())
        ),
        None
    );
    // The relay alone never blocks: hate found there is not its job.
    assert_eq!(
        from_advice(&settings, false, true, &done(AiVerdict::ClearProblem, hate)),
        None
    );
    let address = vec![AiCategory::PersonalData];
    assert_eq!(
        from_advice(
            &settings,
            false,
            true,
            &done(AiVerdict::Ambiguous, address.clone())
        ),
        Some(Outcome::Hold(HOLD_PERSONAL_DATA))
    );
    assert_eq!(
        from_advice(
            &settings,
            true,
            false,
            &done(AiVerdict::ClearProblem, address)
        ),
        None
    );
    for failure in [
        AiAdvice::Failed {
            cause: AiError::DailyLimit,
        },
        AiAdvice::Failed {
            cause: AiError::NoProvider,
        },
        AiAdvice::NotSent {
            cause: NotSentCause::Unverified,
        },
    ] {
        assert_eq!(
            from_advice(&settings, true, true, &failure),
            Some(Outcome::Hold(HOLD_UNCHECKED))
        );
    }
}

#[test]
fn an_image_with_personal_data_is_held_and_never_sent() {
    // Found on this PC, before any request: said apart from what the AI saw.
    assert_eq!(
        not_sent(NotSentCause::PersonalData),
        Outcome::Hold(HOLD_NOT_SENT)
    );
    assert_eq!(
        not_sent(NotSentCause::Location),
        Outcome::Hold(HOLD_NOT_SENT)
    );
    assert_eq!(
        not_sent(NotSentCause::TooLarge),
        Outcome::Hold(HOLD_UNCHECKED)
    );
}

#[test]
fn only_an_unfinished_scan_can_be_settled_by_the_relay() {
    assert!(!scan_incomplete_only(&PrivacyReport::safe()));
    assert!(scan_incomplete_only(&PrivacyReport::suspicious(
        "scan_incomplete"
    )));
    let mut found = PrivacyReport::suspicious("scan_incomplete");
    found.merge(PrivacyReport::suspicious("email"));
    assert!(!scan_incomplete_only(&found));
}

#[tokio::test]
async fn nothing_is_checked_when_off_and_a_missing_image_is_held() {
    let report = PrivacyReport::safe();
    let candidate = |kind| Candidate {
        kind,
        url: "https://relay.invalid/image.png",
        proxy_url: "https://relay.invalid/image.png",
        bytes: None,
        report: &report,
    };
    let off = AppConfig::default();
    assert_eq!(
        check(&off, candidate(MediaKind::Image), "en").await,
        Outcome::Skip
    );
    let on = AppConfig {
        ai: AiSettings {
            provider: Some(crate::ai::AiProvider::OpenAi),
            ..settings()
        },
        ..AppConfig::default()
    };
    assert_eq!(
        check(&on, candidate(MediaKind::Audio), "en").await,
        Outcome::Skip
    );
    // Not a Discord CDN link: nothing is downloaded, so it cannot be shown unchecked.
    assert_eq!(
        check(&on, candidate(MediaKind::Image), "en").await,
        Outcome::Hold(HOLD_UNCHECKED)
    );
    // A Lottie sticker is not an image the providers read.
    let lottie = br#"{"v":"5.7.4","layers":[]}"#;
    let sticker = Candidate {
        bytes: Some(lottie),
        ..candidate(MediaKind::Image)
    };
    assert_eq!(check(&on, sticker, "en").await, Outcome::Skip);
}

#[test]
fn every_category_has_its_own_block_reason() {
    let reasons: std::collections::HashSet<_> =
        ImageGuardCategory::ALL.into_iter().map(blocked).collect();
    assert_eq!(reasons.len(), 6);
    assert!(
        reasons
            .iter()
            .all(|reason| reason.starts_with("ai_image_blocked_"))
    );
}

#[test]
fn personal_data_the_model_lists_holds_even_with_no_issue_seen() {
    let settings = settings();
    let address = vec![AiCategory::PersonalData];
    assert_eq!(
        from_advice(
            &settings,
            true,
            true,
            &done(AiVerdict::NoIssueSeen, address.clone())
        ),
        Some(Outcome::Hold(HOLD_PERSONAL_DATA))
    );
    // Without the relay, personal data is not the image check's job.
    assert_eq!(
        from_advice(
            &settings,
            true,
            false,
            &done(AiVerdict::NoIssueSeen, address)
        ),
        None
    );
    // Harmful and personal at once: the block wins.
    assert_eq!(
        from_advice(
            &settings,
            true,
            true,
            &done(
                AiVerdict::Ambiguous,
                vec![AiCategory::PersonalData, AiCategory::Hate]
            )
        ),
        Some(Outcome::Block("ai_image_blocked_hate"))
    );
}

#[test]
fn the_strongest_result_over_the_frames_wins() {
    let personal = Some(Outcome::Hold(HOLD_NOT_SENT));
    let hate = Some(Outcome::Block("ai_image_blocked_hate"));
    let unchecked = Some(Outcome::Hold(HOLD_UNCHECKED));
    // A personal frame first, a harmful frame next: blocked.
    let mut held = None;
    assert_eq!(fold_frame(&mut held, personal), None);
    assert_eq!(fold_frame(&mut held, None), None);
    assert_eq!(fold_frame(&mut held, hate), hate);
    // A failure stops the check and keeps the earlier hold.
    let mut held = None;
    assert_eq!(fold_frame(&mut held, personal), None);
    assert_eq!(fold_frame(&mut held, unchecked), personal);
    let mut held = None;
    assert_eq!(fold_frame(&mut held, unchecked), unchecked);
    // Nothing found on any frame: no result, the caller clears the media.
    let mut held = None;
    assert_eq!(fold_frame(&mut held, None), None);
    assert_eq!(held, None);
}

#[test]
fn no_image_check_runs_with_the_assistant_off() {
    // The image options stay on when the user picks Off: nothing may be sent.
    let mut settings = settings();
    assert!(!active(&settings));
    settings.provider = Some(crate::ai::AiProvider::OpenAi);
    assert!(active(&settings));
    settings.image_guard = false;
    settings.personal_data_relay = false;
    assert!(!active(&settings));
}
