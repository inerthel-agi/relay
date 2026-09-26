use super::log::{LogAction, LogEntry, ModerationLog};
use super::*;
use crate::privacy::{self, PrivacyAction};

const USER: u64 = 1_200_000_000_000_000_000;

fn input<'a>(lane: Lane, text: &'a str, now_ms: u64) -> GateInput<'a> {
    GateInput {
        lane,
        user_id: USER,
        role_ids: &[],
        member_joined_ms: None,
        mention_count: 0,
        text,
        media_keys: Vec::new(),
        now_ms,
    }
}

fn config_with(settings: ModerationSettings) -> AppConfig {
    AppConfig {
        moderation: settings,
        ..AppConfig::default()
    }
}

#[test]
fn word_lists_block_variants_and_respect_exceptions() {
    let mut config = config_with(ModerationSettings {
        word_packs: vec![WordPack::Scams],
        ..ModerationSettings::default()
    });
    assert!(privacy::privacy_rules_enabled(&config));
    let report = privacy::classify_text(Some("FREE N1TRO for everyone"), &config);
    assert_eq!(privacy::action_for(&report, &config), PrivacyAction::Block);
    assert_eq!(report.primary_reason(), Some("forbidden_concept"));

    config.moderation.pack_exclusions = vec!["free nitro".into()];
    let report = privacy::classify_text(Some("FREE N1TRO for everyone"), &config);
    assert_eq!(report.classification, privacy::PrivacyClassification::Safe);
    // Lists are off by default: nothing changes for existing users.
    assert!(!content_rules_enabled(&AppConfig::default()));
}

#[test]
fn every_built_in_word_is_long_enough_to_match() {
    for pack in WordPack::ALL {
        for word in pack.words() {
            let compact: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
            assert!(compact.chars().count() >= 3, "{word}");
        }
    }
}

#[test]
fn link_rules_catch_invites_shorteners_and_look_alike_domains() {
    let settings = ModerationSettings {
        block_invites: true,
        block_shorteners: true,
        block_scam_domains: true,
        ..ModerationSettings::default()
    };
    assert_eq!(
        link_violation("join discord.gg/abc", &settings),
        Some("discord_invite")
    );
    assert_eq!(
        link_violation("https://discord.com/invite/abc", &settings),
        Some("discord_invite")
    );
    assert_eq!(
        link_violation("(https://bit.ly/x)", &settings),
        Some("link_shortener")
    );
    assert_eq!(
        link_violation("claim at https://dlscord-gift.com/nitro", &settings),
        Some("scam_domain")
    );
    // Official domains and ordinary links pass.
    assert_eq!(
        link_violation("https://discord.com/channels/1/2", &settings),
        None
    );
    assert_eq!(
        link_violation("https://steamcommunity.com/id/me", &settings),
        None
    );
    assert_eq!(
        link_violation("see youtube.com/watch?v=1 ok.", &settings),
        None
    );
    assert_eq!(
        link_violation("discord.gg/abc", &ModerationSettings::default()),
        None
    );
}

#[test]
fn link_rules_normalize_url_authorities() {
    let settings = ModerationSettings {
        block_invites: true,
        block_shorteners: true,
        block_scam_domains: true,
        ..ModerationSettings::default()
    };
    for link in [
        "https://discord.gg:443/fictional",
        "https://discord.com:443/invite/fictional",
        "https://discord.gg./fictional",
        "https://user@discord.gg/fictional",
        "discord.gg:443/fictional",
    ] {
        assert_eq!(
            link_violation(link, &settings),
            Some("discord_invite"),
            "{link}"
        );
    }
    assert_eq!(
        link_violation("https://bit.ly:443/x", &settings),
        Some("link_shortener")
    );
    assert_eq!(
        link_violation("https://dlscord-gift.com:443/x", &settings),
        Some("scam_domain")
    );
    assert_eq!(
        link_violation("https://discord.gg@example.org/x", &settings),
        None
    );
    assert_eq!(link_violation("contact person@discord.gg", &settings), None);
    let config = config_with(settings);
    let report = privacy::classify_text(Some("https://discord.gg:443/fictional"), &config);
    assert_eq!(privacy::action_for(&report, &config), PrivacyAction::Block);
}

#[test]
fn link_rules_block_messages_through_the_privacy_filter() {
    let config = config_with(ModerationSettings {
        block_invites: true,
        ..ModerationSettings::default()
    });
    let report = privacy::classify_text(Some("rejoins discord.gg/raid"), &config);
    assert_eq!(report.primary_reason(), Some("discord_invite"));
    assert_eq!(privacy::action_for(&report, &config), PrivacyAction::Block);
    // Exempt roles skip every text rule, link rules included.
    let exempt = AppConfig {
        privacy_filter_exempt_role_ids: vec!["123456789012345678".into()],
        ..config
    };
    let scoped = privacy::scoped_config_for_roles(&exempt, &["123456789012345678".into()]);
    assert!(!scoped.moderation.links_filtered());
}

#[test]
fn text_spam_detects_mentions_caps_emoji_and_repeats() {
    assert_eq!(text_spam_reason("hi", 6), Some("mention_spam"));
    assert_eq!(
        text_spam_reason("THIS IS A VERY LOUD MESSAGE OK", 0),
        Some("caps_spam")
    );
    assert_eq!(text_spam_reason(&"😀".repeat(13), 0), Some("emoji_spam"));
    assert_eq!(
        text_spam_reason("noooooooooooooooooo", 0),
        Some("repeated_characters")
    );
    assert_eq!(
        text_spam_reason("Salut tout le monde, bon stream !", 1),
        None
    );
}

#[test]
fn gate_applies_blocked_trusted_cooldown_and_duplicates() {
    let settings = ModerationSettings {
        user_cooldown_seconds: 10,
        block_duplicates: true,
        block_text_spam: true,
        ..ModerationSettings::default()
    };
    let mut runtime = ModerationRuntime::default();
    let now = 1_800_000_000_000;
    let mut first = input(Lane::Media, "", now);
    first.media_keys = vec!["a:cat.png:10".into()];
    assert_eq!(
        runtime.evaluate(&first, &settings),
        GateDecision::Pass { trusted: false }
    );
    let mut again = input(Lane::Media, "", now + 2_000);
    again.media_keys = vec!["a:dog.png:10".into()];
    assert_eq!(
        runtime.evaluate(&again, &settings),
        GateDecision::Drop("cooldown")
    );
    let mut duplicate = input(Lane::Media, "", now + 20_000);
    duplicate.media_keys = vec!["a:cat.png:10".into()];
    assert_eq!(
        runtime.evaluate(&duplicate, &settings),
        GateDecision::Drop("duplicate")
    );
    // Notifications have their own cooldown and spam check.
    let loud = input(Lane::Notifications, "THIS IS A VERY LOUD MESSAGE OK", now);
    assert_eq!(
        runtime.evaluate(&loud, &settings),
        GateDecision::Drop("caps_spam")
    );

    let blocked = ModerationSettings {
        blocked_user_ids: vec![USER.to_string()],
        ..settings.clone()
    };
    assert_eq!(
        runtime.evaluate(&input(Lane::Music, "", now), &blocked),
        GateDecision::Drop("blocked_user")
    );
    let trusted = ModerationSettings {
        trusted_role_ids: vec!["123456789012345678".into()],
        ..settings
    };
    let roles = ["123456789012345678".to_string()];
    let mut from_trusted = input(Lane::Media, "", now + 3_000);
    from_trusted.role_ids = &roles;
    assert_eq!(
        runtime.evaluate(&from_trusted, &trusted),
        GateDecision::Pass { trusted: true }
    );
}

#[test]
fn gate_holds_new_accounts_new_members_and_raids() {
    let now = account_created_ms(USER) + 2 * DAY_MS;
    let mut runtime = ModerationRuntime::default();
    let young = ModerationSettings {
        min_account_age_days: 7,
        ..ModerationSettings::default()
    };
    assert_eq!(
        runtime.evaluate(&input(Lane::Media, "", now), &young),
        GateDecision::Hold("new_account")
    );
    let members = ModerationSettings {
        min_member_age_days: 3,
        ..ModerationSettings::default()
    };
    let mut joined = input(Lane::Notifications, "hello", now);
    joined.member_joined_ms = Some(now - DAY_MS);
    assert_eq!(
        runtime.evaluate(&joined, &members),
        GateDecision::Hold("new_member")
    );

    let raid = ModerationSettings {
        raid_limit_per_minute: 2,
        ..ModerationSettings::default()
    };
    let mut runtime = ModerationRuntime::default();
    for offset in 0..2 {
        assert_eq!(
            runtime.evaluate(&input(Lane::Media, "", now + offset), &raid),
            GateDecision::Pass { trusted: false }
        );
    }
    assert_eq!(
        runtime.evaluate(&input(Lane::Media, "", now + 5), &raid),
        GateDecision::Hold("raid")
    );
    assert!(runtime.raid_active(now + 10));
    assert!(!runtime.raid_active(now + 5 + 61_000));
}

#[test]
fn blocks_escalate_to_a_timeout_and_warnings_are_rate_limited() {
    let settings = ModerationSettings {
        warn_on_block: true,
        escalation_blocks: 3,
        escalation_window_minutes: 10,
        escalation_timeout_minutes: 15,
        ..ModerationSettings::default()
    };
    let mut runtime = ModerationRuntime::default();
    let now = 1_800_000_000_000;
    let first = runtime.record_block(USER, &settings, now);
    assert!(first.warn);
    assert_eq!(first.timeout_minutes, None);
    assert!(!runtime.record_block(USER, &settings, now + 1_000).warn);
    assert_eq!(
        runtime
            .record_block(USER, &settings, now + 2_000)
            .timeout_minutes,
        Some(15)
    );
    // The counter restarts after a timeout.
    assert_eq!(
        runtime
            .record_block(USER, &settings, now + 3_000)
            .timeout_minutes,
        None
    );
}

#[test]
fn author_names_matching_filter_words_become_anonymous() {
    let config = config_with(ModerationSettings {
        word_packs: vec![WordPack::Harassment],
        ..ModerationSettings::default()
    });
    let author = crate::model::AuthorIdentity {
        username: "xX_c0nnard_Xx".into(),
        display_avatar_url: "https://cdn.discordapp.com/avatars/1/a.png".into(),
    };
    assert_eq!(
        anonymous_author(author.clone(), &AppConfig::default()).username,
        author.username
    );
    let hidden = anonymous_author(
        crate::model::AuthorIdentity {
            username: "c0nnard".into(),
            ..author
        },
        &config,
    );
    assert_eq!(hidden.username, "Anonymous");
}

#[test]
fn channel_scopes_turn_text_rules_off_for_one_lane() {
    let config = config_with(ModerationSettings {
        word_packs: vec![WordPack::Hate],
        filter_scopes: FilterScopes {
            music: false,
            ..FilterScopes::default()
        },
        ..ModerationSettings::default()
    });
    assert!(content_rules_enabled(&scope_config(
        config.clone(),
        Lane::Media
    )));
    assert!(!content_rules_enabled(&scope_config(config, Lane::Music)));
}

#[test]
fn presets_are_recognised_and_report_what_they_change() {
    let mut config = AppConfig::default();
    assert_eq!(presets::matching(&config), None);
    let changes = presets::changed_keys(&config, ModerationPreset::Standard);
    assert!(changes.contains(&"moderationEnabled"));
    assert!(changes.contains(&"wordPacks"));
    presets::apply(&mut config, &presets::fields(ModerationPreset::Standard));
    config.validate().unwrap();
    assert_eq!(presets::matching(&config), Some(ModerationPreset::Standard));
    assert!(presets::changed_keys(&config, ModerationPreset::Standard).is_empty());
    // A preset never touches channels or custom words.
    assert!(config.privacy_concepts.is_empty());
    assert!(config.watched_channel_id.is_empty());
    for preset in ModerationPreset::ALL {
        let mut config = AppConfig::default();
        presets::apply(&mut config, &presets::fields(preset));
        config.validate().unwrap();
    }
}

#[test]
fn settings_validation_rejects_bad_ids_and_ranges() {
    let valid = ModerationSettings::default();
    valid.validate().unwrap();
    for broken in [
        ModerationSettings {
            trusted_user_ids: vec!["not-an-id".into()],
            ..valid.clone()
        },
        ModerationSettings {
            word_packs: vec![WordPack::Hate, WordPack::Hate],
            ..valid.clone()
        },
        ModerationSettings {
            safety_delay_seconds: 31,
            ..valid.clone()
        },
        ModerationSettings {
            escalation_timeout_minutes: 0,
            ..valid.clone()
        },
    ] {
        assert!(broken.validate().is_err());
    }
}

#[test]
fn decision_log_keeps_seven_days_and_summarises_the_last_day() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("moderation-log.json");
    let now = 1_800_000_000_000;
    let mut log = ModerationLog::open(path.clone(), now);
    let entry = |at, action| LogEntry {
        at,
        lane: Some(Lane::Media),
        action,
        reason: "forbidden_concept".into(),
        author_id: Some(USER.to_string()),
    };
    log.record(entry(now - 8 * DAY_MS, LogAction::Blocked));
    log.record(entry(now - 2 * DAY_MS, LogAction::Blocked));
    log.record(entry(now - 1_000, LogAction::Blocked));
    log.record(entry(now, LogAction::Approved));
    assert_eq!(log.entries().len(), 3);
    let summary = log.summary(now);
    assert_eq!(summary.blocked, 1);
    assert_eq!(summary.approved, 1);
    assert_eq!(summary.flagged_members, 1);
    // Reopening reads the saved file; the log never contains message text.
    let reopened = ModerationLog::open(path.clone(), now);
    assert_eq!(reopened.entries().len(), 3);
    let saved = std::fs::read_to_string(path).unwrap();
    assert!(!saved.contains("text"));
}
