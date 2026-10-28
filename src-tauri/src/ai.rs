//! Optional AI assistance (OpenAI or Anthropic, chosen by the user).
//!
//! Content only ever goes to the provider chosen in the settings: there is no
//! fallback. The one exception is OpenAI's free moderation filter on images
//! (`image_guard`), which uses the saved OpenAI key whatever the provider. API keys stay in Windows Credential Manager
//! (`credentials.rs`), never in the config. Request caps and a circuit breaker
//! keep a failing or costly provider from being called again and again.

pub mod advice;
pub mod client;
pub mod image_guard;
pub mod settings;
pub mod troubleshoot;
pub mod watch;

use std::{
    collections::VecDeque,
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::clock::now_ms;
pub use client::{AiError, AiRequest};

pub const MAX_CALLS_PER_HOUR: u16 = 600;
pub const MAX_CALLS_PER_DAY: u16 = 10_000;
const HOUR_MS: u64 = 60 * 60 * 1_000;
const DAY_MS: u64 = 24 * HOUR_MS;
/// Consecutive network or server failures that pause the AI.
const BREAKER_FAILURES: u8 = 3;
const BREAKER_PAUSE_MS: u64 = 5 * 60 * 1_000;
const TEST_DEADLINE: Duration = Duration::from_secs(20);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AiProvider {
    #[serde(rename = "openai")]
    OpenAi,
    Anthropic,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AiModelTier {
    #[default]
    Fast,
    Precise,
}

/// Model behind each tier. The panel shows the same table (`AI_MODELS`).
pub fn model_id(provider: AiProvider, tier: AiModelTier) -> &'static str {
    match (provider, tier) {
        (AiProvider::OpenAi, AiModelTier::Fast) => "gpt-6-luna",
        (AiProvider::OpenAi, AiModelTier::Precise) => "gpt-6.1-sol",
        (AiProvider::Anthropic, AiModelTier::Fast) => "claude-haiku-5-5",
        (AiProvider::Anthropic, AiModelTier::Precise) => "claude-sonnet-5-5",
    }
}

/// What the image check looks for. The first three come from OpenAI's free
/// moderation filter, the others from the chosen provider's paid model.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageGuardCategory {
    Sexual,
    Violence,
    SelfHarm,
    Hate,
    Harassment,
    Spam,
}

impl ImageGuardCategory {
    pub const ALL: [Self; 6] = [
        Self::Sexual,
        Self::Violence,
        Self::SelfHarm,
        Self::Hate,
        Self::Harassment,
        Self::Spam,
    ];

    pub fn free(self) -> bool {
        matches!(self, Self::Sexual | Self::Violence | Self::SelfHarm)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AiSettings {
    /// `None` until the user picks a provider: nothing can be sent then.
    pub provider: Option<AiProvider>,
    pub model_tier: AiModelTier,
    /// Hard caps on requests, on top of the spend limit set at the provider.
    pub max_calls_per_hour: u16,
    pub max_calls_per_day: u16,
    /// Ask for an advisory opinion on text messages held for review.
    pub message_advice: bool,
    /// Ask before publishing each text notification of a member; the AI can
    /// only hold it for review.
    pub watch_messages: bool,
    /// Check each image of a member before it is shown; a harmful one is
    /// blocked, a doubtful one or a failed check is held for review.
    pub image_guard: bool,
    pub image_guard_categories: Vec<ImageGuardCategory>,
    /// Also delete the Discord message of a blocked image.
    pub image_guard_delete_message: bool,
    /// Also check GIFs and videos, from three frames of each.
    pub image_guard_animated: bool,
    /// Ask the paid model whether an image can locate or identify someone when
    /// the local scan found nothing or could not finish.
    pub personal_data_relay: bool,
    /// JPEG photos.
    pub personal_data_photos: bool,
    /// Other still images (PNG, WebP), mostly screenshots.
    pub personal_data_screenshots: bool,
    /// GIFs and videos, from three frames of each.
    pub personal_data_videos: bool,
    /// Images whose local scan could not finish.
    pub personal_data_incomplete: bool,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: None,
            model_tier: AiModelTier::Fast,
            max_calls_per_hour: 120,
            max_calls_per_day: 1_000,
            message_advice: false,
            watch_messages: false,
            image_guard: false,
            image_guard_categories: ImageGuardCategory::ALL.to_vec(),
            image_guard_delete_message: true,
            image_guard_animated: false,
            personal_data_relay: false,
            personal_data_photos: true,
            personal_data_screenshots: false,
            personal_data_videos: true,
            personal_data_incomplete: true,
        }
    }
}

impl AiSettings {
    pub fn validate(&self) -> Result<()> {
        if !(1..=MAX_CALLS_PER_HOUR).contains(&self.max_calls_per_hour) {
            bail!("The hourly AI request limit must be between 1 and {MAX_CALLS_PER_HOUR}.");
        }
        if !(1..=MAX_CALLS_PER_DAY).contains(&self.max_calls_per_day) {
            bail!("The daily AI request limit must be between 1 and {MAX_CALLS_PER_DAY}.");
        }
        if self.max_calls_per_hour > self.max_calls_per_day {
            bail!("The hourly AI request limit cannot exceed the daily limit.");
        }
        Ok(())
    }
}

/// Request caps (rolling hour and day) and circuit breaker, in memory only.
#[derive(Debug, Default)]
pub struct AiGate {
    calls: VecDeque<u64>,
    failures: u8,
    paused_until: u64,
    /// A refused key or a reached spend limit stops that provider until the
    /// user saves a key again or a manual test succeeds.
    stopped: Option<(AiProvider, AiError)>,
}

impl AiGate {
    /// Counts one request, or says why none may be sent now. A manual test
    /// ignores the pause and the stop (it is how the user checks a fix), not the caps.
    pub fn acquire(
        &mut self,
        provider: AiProvider,
        settings: &AiSettings,
        now: u64,
        manual: bool,
    ) -> Result<(), AiError> {
        while self
            .calls
            .front()
            .is_some_and(|at| now.saturating_sub(*at) >= DAY_MS)
        {
            self.calls.pop_front();
        }
        if !manual {
            if let Some((stopped_provider, error)) = self.stopped
                && stopped_provider == provider
            {
                return Err(error);
            }
            if now < self.paused_until {
                return Err(AiError::Paused);
            }
        }
        let last_hour = self
            .calls
            .iter()
            .filter(|at| now.saturating_sub(**at) < HOUR_MS)
            .count();
        if last_hour >= usize::from(settings.max_calls_per_hour) {
            return Err(AiError::HourlyLimit);
        }
        if self.calls.len() >= usize::from(settings.max_calls_per_day) {
            return Err(AiError::DailyLimit);
        }
        self.calls.push_back(now);
        Ok(())
    }

    pub fn record<T>(&mut self, provider: AiProvider, result: &Result<T, AiError>, now: u64) {
        match result {
            Ok(_) => {
                self.failures = 0;
                self.paused_until = 0;
                self.stopped = None;
            }
            Err(error @ (AiError::KeyRefused | AiError::SpendLimit)) => {
                self.stopped = Some((provider, *error));
            }
            Err(error) if error.transient() => {
                self.failures += 1;
                if self.failures >= BREAKER_FAILURES {
                    self.failures = 0;
                    self.paused_until = now + BREAKER_PAUSE_MS;
                }
            }
            Err(_) => {}
        }
    }

    /// A new key was saved: give the provider another chance. Caps are kept.
    pub fn reset(&mut self) {
        self.failures = 0;
        self.paused_until = 0;
        self.stopped = None;
    }
}

static GATE: LazyLock<Mutex<AiGate>> = LazyLock::new(Mutex::default);

fn gate() -> std::sync::MutexGuard<'static, AiGate> {
    GATE.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn reset_gate() {
    gate().reset();
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTestResult {
    /// `None` when the provider answered as expected.
    pub error: Option<AiError>,
    pub model: Option<&'static str>,
    pub latency_ms: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Sends one fixed message (no Discord content) to the chosen provider to check
/// the key, the model and the spend limit.
pub async fn test_connection(settings: &AiSettings) -> AiTestResult {
    let failed = |error, model| AiTestResult {
        error: Some(error),
        model,
        latency_ms: 0,
        input_tokens: 0,
        output_tokens: 0,
    };
    let Some(provider) = settings.provider else {
        return failed(AiError::NoProvider, None);
    };
    let model = model_id(provider, settings.model_tier);
    let failed = |error| failed(error, Some(model));
    let Ok(Some(api_key)) = crate::credentials::load_ai_api_key(provider) else {
        return failed(AiError::NoKey);
    };
    if let Err(error) = gate().acquire(provider, settings, now_ms(), true) {
        return failed(error);
    }
    let request = AiRequest {
        system: "You check that Relay can reach this model. Reply only with the requested JSON.",
        user_text: "Reply with ok set to true.".into(),
        schema: serde_json::json!({
            "type": "object",
            "properties": { "ok": { "type": "boolean" } },
            "required": ["ok"],
            "additionalProperties": false
        }),
        max_output_tokens: 200,
        image: None,
    };
    let started = Instant::now();
    let result = client::complete(
        provider,
        settings.model_tier,
        &api_key,
        &request,
        TEST_DEADLINE,
        &mut || gate().acquire(provider, settings, now_ms(), true),
    )
    .await
    .and_then(|reply| {
        if reply.value["ok"] == true {
            Ok(reply)
        } else {
            Err(AiError::InvalidOutput)
        }
    });
    gate().record(provider, &result, now_ms());
    let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    match result {
        Ok(reply) => AiTestResult {
            error: None,
            model: Some(model),
            latency_ms,
            input_tokens: reply.input_tokens,
            output_tokens: reply.output_tokens,
        },
        Err(error) => AiTestResult {
            latency_ms,
            ..failed(error)
        },
    }
}

#[cfg(test)]
mod tests;
