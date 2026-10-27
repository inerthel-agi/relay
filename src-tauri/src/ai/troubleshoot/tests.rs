// No test here reaches a provider or Windows Credential Manager.
use super::*;
use crate::ai::advice::NotSentCause;
use crate::{
    model::{ChannelSummary, OutputConnectionStatus},
    moderation::{Lane, log::LogAction},
};

const NOW: u64 = 1_800_000_000_000;

fn log(minutes_ago: u64, action: LogAction, reason: &str, author: &str) -> LogEntry {
    LogEntry {
        at: NOW - minutes_ago * 60_000,
        lane: Some(Lane::Media),
        action,
        reason: reason.into(),
        author_id: Some(author.into()),
    }
}

#[test]
fn the_report_holds_states_counts_and_codes_but_nothing_private() {
    let mut config = AppConfig {
        watched_channel_id: "123456789012345678".into(),
        music_channel_id: "123456789012345679".into(),
        privacy_allowlist: vec!["public@example.com".into()],
        moderation_enabled: true,
        ..AppConfig::default()
    };
    config.moderation.trusted_user_ids = vec!["223456789012345678".into()];
    let bot = BotStatus {
        connected: false,
        username: Some("StreamBot#0001".into()),
        display_avatar_url: Some("https://cdn.discordapp.com/avatars/1/secretavatar.png".into()),
        error: Some(
            r"Login failed: MTIzNDU2Nzg5MDEyMzQ1Njc4.GabcdE.secrettokenpartthatislong123 in C:\Users\alice\AppData"
                .into(),
        ),
    };
    let server = ServerStatus {
        connected: true,
        overlay_clients: 1,
        error: Some("bind failed on http://127.0.0.1:4590/overlay?secret=hunter2".into()),
        outputs: crate::model::OutputStatuses {
            visual: OutputConnectionStatus {
                obs_clients: 1,
                last_connected_at: Some(NOW - 5 * 60_000),
                ..OutputConnectionStatus::default()
            },
            ..crate::model::OutputStatuses::default()
        },
    };
    let channels = [ChannelSummary {
        id: "123456789012345678".into(),
        name: "secret-channel-name".into(),
        guild_name: "Alice's private server".into(),
    }];
    let entries = [
        log(3, LogAction::Held, "scan_incomplete", "323456789012345678"),
        log(9, LogAction::Ignored, "cooldown", "423456789012345678"),
        // A reason Relay does not recognise as a plain code is not sent as such.
        log(
            12,
            LogAction::Blocked,
            "custom: alice@example.com",
            "523456789012345678",
        ),
    ];
    let held = [(MediaKind::Gif, "scan_incomplete".to_string())];

    let facts = facts(&Snapshot {
        config: &config,
        bot: &bot,
        server: &server,
        channels: &channels,
        paused: false,
        queued: 2,
        held_media: &held,
        held_texts: 1,
        log: &entries,
        now: NOW,
    });

    let report = facts
        .iter()
        .map(|fact| format!("[{}] {}", fact.id, fact.text))
        .collect::<Vec<_>>()
        .join("\n");
    for private in [
        "123456789012345678",
        "123456789012345679",
        "223456789012345678",
        "323456789012345678",
        "423456789012345678",
        "523456789012345678",
        "public@example.com",
        "alice",
        "StreamBot",
        "secretavatar",
        "secrettoken",
        "MTIzNDU2",
        "hunter2",
        "127.0.0.1",
        "secret-channel-name",
        "private server",
    ] {
        assert!(
            !report.contains(private),
            "{private} is in the report:\n{report}"
        );
    }
    for expected in [
        "[bot] Discord bot connected: no. Bot error reported: yes.",
        "[relay_channel] Relay channel set: yes. Visible to the bot: yes (1 channels visible).",
        "[server] Local server online: yes. Server error reported: yes.",
        "[moderation] Manual review: on.",
        "[ai_watch] AI watch before publishing messages: off.",
        "[held_1] Waiting for review: gif, reason scan_incomplete.",
        "[log_1] 3 min ago: held in media, reason scan_incomplete.",
        "[log_2] 9 min ago: ignored in media, reason cooldown.",
        "[log_3] 12 min ago: blocked in media, reason other.",
        "[queue] Media queued for the stream: 2. Waiting for review: 1 media and 1 messages.",
    ] {
        assert!(
            report.contains(expected),
            "missing {expected} in:\n{report}"
        );
    }
    assert!(report.contains("[output_visual] Visual output (images, GIFs, videos): OBS sources 1, widgets 0, previews 0, last connection 5 min ago."));
}

fn two_facts() -> Vec<Fact> {
    vec![
        Fact {
            id: "bot".into(),
            text: "Discord bot connected: yes. Bot error reported: no.".into(),
        },
        Fact {
            id: "log_1".into(),
            text: "3 min ago: held in media, reason scan_incomplete.".into(),
        },
    ]
}

#[test]
fn the_request_cites_report_lines_by_id_and_keeps_the_question_as_data() {
    let request = request(&two_facts(), "pourquoi mon GIF ne s'affiche pas ?", "fr");
    assert!(
        request
            .user_text
            .contains("[log_1] 3 min ago: held in media, reason scan_incomplete.")
    );
    assert!(
        request
            .user_text
            .contains("<<<\npourquoi mon GIF ne s'affiche pas ?\n>>>")
    );
    assert!(request.user_text.contains("French"));
    // The AI can only cite lines that exist.
    assert_eq!(
        request.schema["properties"]["findings"]["items"]["properties"]["evidence"]["items"]["enum"],
        json!(["bot", "log_1"])
    );
    assert!(request.image.is_none());
}

#[test]
fn only_findings_backed_by_existing_report_lines_read_as_from_the_report() {
    let facts = two_facts();
    let reply = parse_reply(
        &json!({
            "findings": [
                { "text": "  Ton GIF attend une validation :\n scan incomplet. ", "evidence": ["log_1"] },
                { "text": "OBS a peut-être été relancé.", "evidence": [] },
                { "text": "Le bot est banni.", "evidence": ["made_up"] },
                { "text": "Mélange.", "evidence": ["log_1", "made_up"] },
                { "text": "Cinquième.", "evidence": ["bot"] },
            ],
            "next_step": "Ouvre la page Modération et valide-le."
        }),
        &facts,
    )
    .unwrap();
    assert_eq!(
        reply.findings,
        vec![
            Finding {
                text: "Ton GIF attend une validation : scan incomplet.".into(),
                from_report: true
            },
            Finding {
                text: "OBS a peut-être été relancé.".into(),
                from_report: false
            },
            Finding {
                text: "Le bot est banni.".into(),
                from_report: false
            },
            Finding {
                text: "Mélange.".into(),
                from_report: false
            },
        ]
    );
    assert_eq!(reply.next_step, "Ouvre la page Modération et valide-le.");
    assert_eq!(
        reply.report,
        facts
            .iter()
            .map(|fact| fact.text.clone())
            .collect::<Vec<_>>()
    );

    let long = "x".repeat(1_000);
    let bounded = parse_reply(
        &json!({ "findings": [{ "text": long, "evidence": ["bot"] }], "next_step": long }),
        &facts,
    )
    .unwrap();
    assert_eq!(bounded.findings[0].text.chars().count(), 300);
    assert_eq!(bounded.next_step.chars().count(), 300);

    assert_eq!(
        parse_reply(&json!({ "next_step": "rien" }), &facts).unwrap_err(),
        AiError::InvalidOutput
    );
}

#[tokio::test]
async fn nothing_is_sent_without_a_provider_or_with_personal_data() {
    let mut config = AppConfig::default();
    let facts = two_facts();
    assert_eq!(
        answer(&config, &facts, "pourquoi rien ne s'affiche ?", "fr").await,
        TroubleshootAnswer::Failed {
            cause: AiError::NoProvider
        }
    );
    config.ai.provider = Some(crate::ai::AiProvider::OpenAi);
    // Both stop before the API key is even read.
    assert_eq!(
        answer(&config, &facts, "  ", "fr").await,
        TroubleshootAnswer::NotSent {
            cause: NotSentCause::Empty
        }
    );
    assert_eq!(
        answer(
            &config,
            &facts,
            "mon numéro 06 12 34 56 78 ne marche pas",
            "fr"
        )
        .await,
        TroubleshootAnswer::NotSent {
            cause: NotSentCause::PersonalData
        }
    );
}

#[test]
fn the_help_pace_allows_one_question_every_five_minutes_and_ten_a_day() {
    let mut pace = HelpPace::default();
    assert_eq!(pace.reserve(NOW), Ok(()));
    assert_eq!(
        pace.reserve(NOW + HELP_COOLDOWN_MS - 1),
        Err(AiError::HelpCooldown)
    );
    for step in 1..HELP_DAILY_LIMIT as u64 {
        assert_eq!(pace.reserve(NOW + step * HELP_COOLDOWN_MS), Ok(()));
    }
    let eleventh = NOW + HELP_DAILY_LIMIT as u64 * HELP_COOLDOWN_MS;
    assert_eq!(pace.reserve(eleventh), Err(AiError::HelpDailyLimit));
    assert_eq!(pace.recent(eleventh).len(), HELP_DAILY_LIMIT);
    // The first question leaves the count 24 hours after it was asked.
    assert_eq!(pace.reserve(NOW + DAY_MS), Ok(()));
}

#[test]
fn a_question_that_never_left_gives_its_turn_back() {
    let mut pace = HelpPace::default();
    pace.reserve(NOW).unwrap();
    pace.release(NOW);
    assert!(pace.recent(NOW).is_empty());
    assert_eq!(pace.reserve(NOW + 1), Ok(()));
    assert!(never_sent(&TroubleshootAnswer::Failed {
        cause: AiError::NoKey
    }));
    assert!(!never_sent(&TroubleshootAnswer::Failed {
        cause: AiError::Timeout
    }));
}
