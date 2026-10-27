// The provider is a test function here: no network, no API key read.
use std::sync::{Arc, Mutex};

use super::*;
use crate::ai::{AiError, AiProvider, advice::AiVerdict};

fn watching() -> AppConfig {
    let mut config = AppConfig::default();
    config.ai.provider = Some(AiProvider::Anthropic);
    config.ai.watch_messages = true;
    config
}

fn done(verdict: AiVerdict) -> AiAdvice {
    AiAdvice::Done {
        verdict,
        categories: Vec::new(),
        reason: "Raison.".into(),
    }
}

/// Answers `advice` and remembers every text it was sent.
fn provider(
    advice: AiAdvice,
    sent: &Arc<Mutex<Vec<String>>>,
) -> impl FnOnce(AiProvider, AiRequest) -> std::future::Ready<AiAdvice> {
    let sent = sent.clone();
    move |_, request| {
        sent.lock().unwrap().push(request.user_text);
        std::future::ready(advice)
    }
}

const MEMBER: Sender = Sender::Member;

#[tokio::test]
async fn only_a_clear_problem_from_a_member_holds_the_message() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let config = watching();
    let hold = |advice| {
        hold_advice(
            &config,
            "t'es nul <@123456789012345678>",
            MEMBER,
            false,
            "fr",
            DEADLINE,
            provider(advice, &sent),
        )
    };

    assert_eq!(
        hold(done(AiVerdict::ClearProblem)).await,
        Some(done(AiVerdict::ClearProblem))
    );
    assert_eq!(hold(done(AiVerdict::Ambiguous)).await, None);
    assert_eq!(hold(done(AiVerdict::NoIssueSeen)).await, None);
    let sent = sent.lock().unwrap();
    assert_eq!(sent.len(), 3);
    // The same cleaning as held messages: no Discord identifier leaves the PC.
    assert!(sent[0].contains("t'es nul @member"));
    assert!(!sent[0].contains("123456789012345678"));
}

#[tokio::test]
async fn nothing_is_sent_when_the_watch_does_not_apply() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let clear = || provider(done(AiVerdict::ClearProblem), &sent);
    let text = "t'es nul";

    let mut off = watching();
    off.ai.watch_messages = false;
    assert_eq!(
        hold_advice(&off, text, MEMBER, false, "fr", DEADLINE, clear()).await,
        None
    );
    let mut no_provider = watching();
    no_provider.ai.provider = None;
    assert_eq!(
        hold_advice(&no_provider, text, MEMBER, false, "fr", DEADLINE, clear()).await,
        None
    );
    let config = watching();
    for sender in [Sender::Trusted, Sender::Unknown] {
        assert_eq!(
            hold_advice(&config, text, sender, false, "fr", DEADLINE, clear()).await,
            None
        );
    }
    // Paused by the Panic button.
    assert_eq!(
        hold_advice(&config, text, MEMBER, true, "fr", DEADLINE, clear()).await,
        None
    );
    // Personal data is never sent, even with the privacy scan off.
    assert_eq!(
        hold_advice(
            &config,
            "appelle le 06 12 34 56 78",
            MEMBER,
            false,
            "fr",
            DEADLINE,
            clear()
        )
        .await,
        None
    );
    assert!(sent.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_slow_or_failing_provider_lets_the_message_through() {
    let config = watching();
    let text = "t'es nul";
    let slow = |_, _| async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        done(AiVerdict::ClearProblem)
    };
    assert_eq!(
        hold_advice(
            &config,
            text,
            MEMBER,
            false,
            "fr",
            Duration::from_millis(30),
            slow
        )
        .await,
        None
    );
    let sent = Arc::new(Mutex::new(Vec::new()));
    for answer in [
        AiAdvice::Failed {
            cause: AiError::Timeout,
        },
        AiAdvice::Failed {
            cause: AiError::DailyLimit,
        },
        AiAdvice::Failed {
            cause: AiError::InvalidOutput,
        },
        AiAdvice::Pending,
    ] {
        assert_eq!(
            hold_advice(
                &config,
                text,
                MEMBER,
                false,
                "fr",
                DEADLINE,
                provider(answer, &sent)
            )
            .await,
            None
        );
    }
}

#[tokio::test]
async fn a_single_short_word_is_shown_without_asking_the_ai() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let config = watching();
    for text in ["test", " gg ", "salut", "😂", "ok...", "lol!"] {
        let clear = provider(done(AiVerdict::ClearProblem), &sent);
        assert_eq!(
            hold_advice(&config, text, MEMBER, false, "fr", DEADLINE, clear).await,
            None,
            "{text}"
        );
    }
    assert!(sent.lock().unwrap().is_empty());
    // Two words, a long word, a link or a mention still go to the AI.
    for text in [
        "t'es nul",
        "anticonstitutionnellement",
        "x.co/abc",
        "<@123456789>",
        "www.arnaque.fr",
        "evil.co",
        "nitro.gift",
        "[x](evil.co)",
    ] {
        let clear = provider(done(AiVerdict::ClearProblem), &sent);
        assert!(
            hold_advice(&config, text, MEMBER, false, "fr", DEADLINE, clear)
                .await
                .is_some(),
            "{text}"
        );
    }
    assert_eq!(sent.lock().unwrap().len(), 8);
}
