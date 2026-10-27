use super::*;

#[test]
fn defaults_send_nothing_and_stay_within_limits() {
    let settings = AiSettings::default();
    assert_eq!(settings.provider, None);
    assert_eq!(settings.model_tier, AiModelTier::Fast);
    settings.validate().unwrap();
}

#[test]
fn providers_use_stable_names_in_the_config() {
    assert_eq!(
        serde_json::to_value(AiProvider::OpenAi).unwrap(),
        serde_json::json!("openai")
    );
    assert_eq!(
        serde_json::to_value(AiProvider::Anthropic).unwrap(),
        serde_json::json!("anthropic")
    );
    assert!(serde_json::from_value::<AiProvider>(serde_json::json!("gemini")).is_err());
    let settings: AiSettings =
        serde_json::from_value(serde_json::json!({ "provider": "anthropic" })).unwrap();
    assert_eq!(settings.provider, Some(AiProvider::Anthropic));
    assert_eq!(settings.max_calls_per_day, 1_000);
}

#[test]
fn request_limits_are_bounded() {
    let valid = AiSettings {
        max_calls_per_hour: MAX_CALLS_PER_HOUR,
        max_calls_per_day: MAX_CALLS_PER_DAY,
        ..AiSettings::default()
    };
    valid.validate().unwrap();
    for (hour, day) in [
        (0, 10),
        (10, 0),
        (MAX_CALLS_PER_HOUR + 1, MAX_CALLS_PER_DAY),
        (10, MAX_CALLS_PER_DAY + 1),
        (50, 20),
    ] {
        let settings = AiSettings {
            max_calls_per_hour: hour,
            max_calls_per_day: day,
            ..AiSettings::default()
        };
        assert!(settings.validate().is_err(), "{hour}/{day}");
    }
}

#[test]
fn every_provider_and_tier_names_a_model() {
    let mut models = Vec::new();
    for provider in [AiProvider::OpenAi, AiProvider::Anthropic] {
        for tier in [AiModelTier::Fast, AiModelTier::Precise] {
            models.push(model_id(provider, tier));
        }
    }
    models.sort_unstable();
    models.dedup();
    assert_eq!(models.len(), 4);
    // The panel shows the same table.
    let panel = include_str!("../../../gui/moderation-ui.mjs");
    for model in models {
        assert!(panel.contains(&format!("\"{model}\"")), "{model}");
    }
}

fn settings(hour: u16, day: u16) -> AiSettings {
    AiSettings {
        provider: Some(AiProvider::OpenAi),
        max_calls_per_hour: hour,
        max_calls_per_day: day,
        ..AiSettings::default()
    }
}

#[test]
fn caps_count_requests_over_a_rolling_hour_and_day() {
    let mut gate = AiGate::default();
    let limits = settings(2, 3);
    let provider = AiProvider::OpenAi;
    gate.acquire(provider, &limits, 0, false).unwrap();
    gate.acquire(provider, &limits, 1, false).unwrap();
    assert_eq!(
        gate.acquire(provider, &limits, 2, false),
        Err(AiError::HourlyLimit)
    );
    // A manual test respects the caps too.
    assert_eq!(
        gate.acquire(provider, &limits, 2, true),
        Err(AiError::HourlyLimit)
    );
    // The first request left the last hour, not the last day.
    gate.acquire(provider, &limits, HOUR_MS, false).unwrap();
    assert_eq!(
        gate.acquire(provider, &limits, HOUR_MS + 1, false),
        Err(AiError::DailyLimit)
    );
    gate.acquire(provider, &limits, DAY_MS + 1, false).unwrap();
}

#[test]
fn repeated_outages_pause_the_ai_for_five_minutes() {
    let mut gate = AiGate::default();
    let limits = settings(100, 100);
    let provider = AiProvider::Anthropic;
    for error in [AiError::Unavailable, AiError::Timeout] {
        gate.record::<()>(provider, &Err(error), 0);
    }
    gate.acquire(provider, &limits, 1, false).unwrap();
    gate.record::<()>(provider, &Err(AiError::RateLimited), 10);
    assert_eq!(
        gate.acquire(provider, &limits, 11, false),
        Err(AiError::Paused)
    );
    assert_eq!(
        gate.acquire(provider, &limits, 10 + BREAKER_PAUSE_MS - 1, false),
        Err(AiError::Paused)
    );
    gate.acquire(provider, &limits, 10 + BREAKER_PAUSE_MS, false)
        .unwrap();
    // A refusal or a bad answer is not an outage.
    for _ in 0..5 {
        gate.record::<()>(provider, &Err(AiError::InvalidOutput), 20);
    }
    gate.acquire(provider, &limits, 10 + BREAKER_PAUSE_MS + 1, false)
        .unwrap();
}

#[test]
fn a_refused_key_or_spend_limit_stops_only_that_provider() {
    let mut gate = AiGate::default();
    let limits = settings(100, 100);
    gate.record::<()>(AiProvider::OpenAi, &Err(AiError::SpendLimit), 0);
    assert_eq!(
        gate.acquire(AiProvider::OpenAi, &limits, 1, false),
        Err(AiError::SpendLimit)
    );
    gate.acquire(AiProvider::Anthropic, &limits, 1, false)
        .unwrap();
    // The user checks the fix with a manual test; success clears the stop.
    gate.acquire(AiProvider::OpenAi, &limits, 2, true).unwrap();
    gate.record(AiProvider::OpenAi, &Ok(()), 3);
    gate.acquire(AiProvider::OpenAi, &limits, 4, false).unwrap();

    gate.record::<()>(AiProvider::OpenAi, &Err(AiError::KeyRefused), 5);
    assert_eq!(
        gate.acquire(AiProvider::OpenAi, &limits, 6, false),
        Err(AiError::KeyRefused)
    );
    gate.reset();
    gate.acquire(AiProvider::OpenAi, &limits, 7, false).unwrap();
}
