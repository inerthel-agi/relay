//! Message watch: before a text notification is published, the AI may put it
//! in the review queue. It can only hold a message, never publish, block or
//! delete one, and any failure publishes the message as it would be without AI.

use std::{future::Future, time::Duration};

use super::{
    AiProvider, AiRequest,
    advice::{AiAdvice, AiCategory, AiVerdict, prepare_text, request, transcript_request},
};
use crate::config::AppConfig;

/// Longest wait before the message is published anyway.
pub const DEADLINE: Duration = Duration::from_secs(6);
/// Reason code of a message held by the watch.
pub const HOLD_REASON: &str = "ai_flagged";
/// Longest single word published without asking the AI.
const PLAIN_WORD_CHARS: usize = 12;

/// One short word without a link or mention ("test", "gg", "salut"): shown at
/// once. Single-word insults are left to the local word filter. A dot inside
/// the word ("evil.co", "nitro.gift") or a Markdown link counts as a link.
fn plain(text: &str) -> bool {
    let text = text.trim();
    let chars: Vec<char> = text.chars().collect();
    !chars.is_empty()
        && chars.len() <= PLAIN_WORD_CHARS
        && !chars.iter().any(|c| c.is_whitespace())
        && !text.contains(['@', '<', '/'])
        && !text.contains("](")
        && !chars
            .windows(2)
            .any(|pair| pair[0] == '.' && pair[1].is_alphanumeric())
}

/// Who wrote the message, as Relay's own moderation saw it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sender {
    Member,
    /// Server staff and trusted members are never sent.
    Trusted,
    /// No moderation verdict: not sent either.
    Unknown,
}

/// The advice to hold the message with, or `None` to publish it.
pub async fn hold_advice<F, Fut>(
    config: &AppConfig,
    text: &str,
    sender: Sender,
    paused: bool,
    language: &str,
    deadline: Duration,
    ask: F,
) -> Option<AiAdvice>
where
    F: FnOnce(AiProvider, AiRequest) -> Fut,
    Fut: Future<Output = AiAdvice>,
{
    let watched = Watched {
        config,
        text,
        sender,
        paused,
        deadline,
    };
    watched
        .advice(
            |text| request(text, language),
            ask,
            |advice| {
                matches!(
                    advice,
                    AiAdvice::Done {
                        verdict: AiVerdict::ClearProblem,
                        ..
                    }
                )
            },
        )
        .await
}

/// Second reading of a voice transcript, with the same switch, limits and
/// senders as messages. Only the text leaves the PC, never the sound; a doubt
/// about personal data holds it too. `None` to let the sound play.
pub async fn transcript_hold_advice<F, Fut>(
    config: &AppConfig,
    transcript: &str,
    sender: Sender,
    paused: bool,
    language: &str,
    deadline: Duration,
    ask: F,
) -> Option<AiAdvice>
where
    F: FnOnce(AiProvider, AiRequest) -> Fut,
    Fut: Future<Output = AiAdvice>,
{
    let watched = Watched {
        config,
        text: transcript,
        sender,
        paused,
        deadline,
    };
    watched
        .advice(
            |text| transcript_request(text, language),
            ask,
            |advice| {
                matches!(
                    advice,
                    AiAdvice::Done {
                        verdict: AiVerdict::ClearProblem,
                        ..
                    }
                ) || matches!(
                    advice,
                    AiAdvice::Done { verdict: AiVerdict::Ambiguous, categories, .. }
                        if categories.contains(&AiCategory::PersonalData)
                )
            },
        )
        .await
}

/// One text the watch may send: nothing goes out for staff, trusted members,
/// a paused stream, a single plain word, or a text that may hold personal data.
struct Watched<'a> {
    config: &'a AppConfig,
    text: &'a str,
    sender: Sender,
    paused: bool,
    deadline: Duration,
}

impl Watched<'_> {
    async fn advice<F, Fut>(
        self,
        build: impl FnOnce(&str) -> AiRequest,
        ask: F,
        holds: impl FnOnce(&AiAdvice) -> bool,
    ) -> Option<AiAdvice>
    where
        F: FnOnce(AiProvider, AiRequest) -> Fut,
        Fut: Future<Output = AiAdvice>,
    {
        if !self.config.ai.watch_messages
            || self.sender != Sender::Member
            || self.paused
            || plain(self.text)
        {
            return None;
        }
        let provider = self.config.ai.provider?;
        let text = prepare_text(self.text, self.config).ok()?;
        let advice = tokio::time::timeout(self.deadline, ask(provider, build(&text)))
            .await
            .ok()?;
        holds(&advice).then_some(advice)
    }
}

/// The real provider call, with the caps and circuit breaker of every AI
/// request, counted against the share of the `member` who wrote the text.
#[cfg_attr(test, allow(dead_code))]
pub async fn ask(
    settings: &super::AiSettings,
    provider: AiProvider,
    request: AiRequest,
    member: Option<&str>,
) -> AiAdvice {
    super::advice::ask(settings, provider, &request, member).await
}

/// Stands in for the provider in tests: what it answers and what it was sent.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct WatchStub {
    pub answer: Option<AiAdvice>,
    pub sent: Vec<String>,
}

#[cfg(test)]
mod tests;
