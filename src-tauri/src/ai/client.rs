//! The two provider adapters behind one request and one reply shape.
//!
//! Only `build_body`, `headers` and `parse_reply` differ per provider. Provider
//! error messages are never kept or shown: some echo part of the API key.

use std::{sync::LazyLock, time::Duration};

use reqwest::{
    Client, StatusCode,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue, RETRY_AFTER},
};
use serde::Serialize;
use serde_json::{Value, json};
use tokio::time::Instant;

use super::{AiModelTier, AiProvider, model_id};

const OPENAI_BASE_URL: &str = "https://api.openai.com";
const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// A rate limit asking to wait longer than this is not retried.
const MAX_RETRY_WAIT: Duration = Duration::from_secs(3);
const DEFAULT_RETRY_WAIT: Duration = Duration::from_millis(500);

/// Why no answer could be used. Carries no provider text on purpose.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AiError {
    NoProvider,
    NoKey,
    HourlyLimit,
    DailyLimit,
    Paused,
    KeyRefused,
    SpendLimit,
    RateLimited,
    Unavailable,
    Timeout,
    Refused,
    InvalidOutput,
    BadRequest,
    /// Help: one question every five minutes.
    HelpCooldown,
    /// Help: ten questions a day.
    HelpDailyLimit,
}

impl AiError {
    /// Worth one retry, and counted by the circuit breaker.
    pub fn transient(self) -> bool {
        matches!(self, Self::RateLimited | Self::Unavailable | Self::Timeout)
    }
}

/// One request, the same for both providers. `system` is fixed Relay text;
/// `user_text` may hold untrusted data and is sent as the user turn only.
pub struct AiRequest {
    pub system: &'static str,
    pub user_text: String,
    /// JSON schema the reply must follow (objects with `additionalProperties: false`).
    pub schema: Value,
    pub max_output_tokens: u32,
    /// Sent as bytes, never as a link: the provider never fetches Discord itself.
    pub image: Option<AiImage>,
}

/// An image sent with the request, in a format both providers accept.
#[derive(Clone, Debug, PartialEq)]
pub struct AiImage {
    pub mime: &'static str,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct AiReply {
    pub value: Value,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// `allow_retry` counts the second attempt against the request caps; an error
/// there means no retry.
pub async fn complete(
    provider: AiProvider,
    tier: AiModelTier,
    api_key: &str,
    request: &AiRequest,
    deadline: Duration,
    allow_retry: &mut (dyn FnMut() -> Result<(), AiError> + Send),
) -> Result<AiReply, AiError> {
    let base_url = match provider {
        AiProvider::OpenAi => OPENAI_BASE_URL,
        AiProvider::Anthropic => ANTHROPIC_BASE_URL,
    };
    complete_at(
        base_url,
        provider,
        tier,
        api_key,
        request,
        deadline,
        allow_retry,
    )
    .await
}

/// At most two attempts, both within `deadline`, both to the same provider.
pub(super) async fn complete_at(
    base_url: &str,
    provider: AiProvider,
    tier: AiModelTier,
    api_key: &str,
    request: &AiRequest,
    deadline: Duration,
    allow_retry: &mut (dyn FnMut() -> Result<(), AiError> + Send),
) -> Result<AiReply, AiError> {
    let started = Instant::now();
    let url = format!(
        "{base_url}{}",
        match provider {
            AiProvider::OpenAi => "/v1/responses",
            AiProvider::Anthropic => "/v1/messages",
        }
    );
    let headers = headers(provider, api_key)?;
    let body = serde_json::to_vec(&build_body(provider, tier, request))
        .map_err(|_| AiError::BadRequest)?;
    let mut retried = false;
    loop {
        let remaining = deadline
            .checked_sub(started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .ok_or(AiError::Timeout)?;
        match send_once(&url, &headers, &body, remaining).await {
            Ok(response) => return parse_reply(provider, &response),
            Err((error, retry_after)) => {
                let wait = retry_after.unwrap_or(DEFAULT_RETRY_WAIT);
                if retried
                    || !error.transient()
                    || wait > MAX_RETRY_WAIT
                    || wait >= remaining
                    || allow_retry().is_err()
                {
                    return Err(error);
                }
                retried = true;
                tokio::time::sleep(wait).await;
            }
        }
    }
}

fn http_client() -> &'static Client {
    static CLIENT: LazyLock<Client> = LazyLock::new(|| {
        Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            // A redirect could carry the key to another host.
            .redirect(reqwest::redirect::Policy::none())
            .https_only(!cfg!(test))
            .build()
            .unwrap_or_default()
    });
    &CLIENT
}

fn headers(provider: AiProvider, api_key: &str) -> Result<HeaderMap, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let (name, value) = match provider {
        AiProvider::OpenAi => ("authorization", format!("Bearer {api_key}")),
        AiProvider::Anthropic => ("x-api-key", api_key.to_owned()),
    };
    let mut value = HeaderValue::from_str(&value).map_err(|_| AiError::KeyRefused)?;
    value.set_sensitive(true);
    headers.insert(name, value);
    if provider == AiProvider::Anthropic {
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
    }
    Ok(headers)
}

fn base64(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// The image comes first, as the Anthropic vision guide recommends.
fn anthropic_content(request: &AiRequest) -> Value {
    match &request.image {
        None => json!(request.user_text),
        Some(image) => json!([
            { "type": "image", "source": { "type": "base64", "media_type": image.mime, "data": base64(&image.data) } },
            { "type": "text", "text": request.user_text },
        ]),
    }
}

fn openai_input(request: &AiRequest) -> Value {
    match &request.image {
        None => json!(request.user_text),
        Some(image) => json!([{ "role": "user", "content": [
            { "type": "input_image", "image_url": format!("data:{};base64,{}", image.mime, base64(&image.data)) },
            { "type": "input_text", "text": request.user_text },
        ] }]),
    }
}

/// Low reasoning on both: these are short classification tasks.
pub(super) fn build_body(provider: AiProvider, tier: AiModelTier, request: &AiRequest) -> Value {
    let model = model_id(provider, tier);
    match provider {
        AiProvider::Anthropic => json!({
            "model": model,
            "max_tokens": request.max_output_tokens,
            "system": request.system,
            "messages": [{ "role": "user", "content": anthropic_content(request) }],
            // Haiku 5.5 accepts "disabled"; Sonnet 5.5 uses "between_tools" instead.
            "thinking": { "type": match tier {
                AiModelTier::Fast => "disabled",
                AiModelTier::Precise => "between_tools",
            } },
            "output_config": {
                "effort": "low",
                "format": { "type": "json_schema", "schema": request.schema },
            },
        }),
        AiProvider::OpenAi => json!({
            "model": model,
            "instructions": request.system,
            "input": openai_input(request),
            // Responses are kept 30 days by default; Relay never needs them again.
            "store": false,
            "max_output_tokens": request.max_output_tokens,
            "reasoning": { "effort": match tier {
                AiModelTier::Fast => "none",
                AiModelTier::Precise => "low",
            } },
            "text": { "format": {
                "type": "json_schema",
                "name": "relay_result",
                "schema": request.schema,
                "strict": true,
            } },
        }),
    }
}

async fn send_once(
    url: &str,
    headers: &HeaderMap,
    body: &[u8],
    timeout: Duration,
) -> Result<Value, (AiError, Option<Duration>)> {
    let mut response = http_client()
        .post(url)
        .headers(headers.clone())
        .body(body.to_vec())
        .timeout(timeout)
        .send()
        .await
        .map_err(|error| (network_error(&error), None))?;
    let status = response.status();
    let retry_after = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs);
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| (network_error(&error), None))?
    {
        if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err((AiError::InvalidOutput, None));
        }
        bytes.extend_from_slice(&chunk);
    }
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if status.is_success() {
        Ok(value)
    } else {
        Err((classify_error(status, &value), retry_after))
    }
}

fn network_error(error: &reqwest::Error) -> AiError {
    if error.is_timeout() {
        AiError::Timeout
    } else if error.is_redirect() || error.is_builder() {
        AiError::BadRequest
    } else {
        AiError::Unavailable
    }
}

/// Spend and billing codes of both providers, checked before rate limits.
const SPEND_LIMIT_CODES: [&str; 6] = [
    "enforced_spend_limit_reached",
    "insufficient_quota",
    "credit_balance_exhausted",
    "organization_spend_limit_exceeded",
    "project_spend_limit_exceeded",
    "organization_usage_limit_exceeded",
];

pub(super) fn classify_error(status: StatusCode, body: &Value) -> AiError {
    let error = &body["error"];
    let codes = [
        error["code"].as_str(),
        error["type"].as_str(),
        error["details"]["error_code"].as_str(),
    ];
    if codes
        .iter()
        .flatten()
        .any(|code| SPEND_LIMIT_CODES.contains(code))
        || status == StatusCode::PAYMENT_REQUIRED
    {
        return AiError::SpendLimit;
    }
    match status.as_u16() {
        401 | 403 => AiError::KeyRefused,
        // Anthropic answers 400 when a spend limit set by the user is reached.
        400 if error["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("You have reached your specified")) =>
        {
            AiError::SpendLimit
        }
        429 => AiError::RateLimited,
        408 | 504 => AiError::Timeout,
        500..=599 => AiError::Unavailable,
        _ => AiError::BadRequest,
    }
}

/// OpenAI's free moderation model; it reads images for these three groups only.
const MODERATION_MODEL: &str = "omni-moderation-latest";
/// Flag names of the moderation answer, grouped as the image check shows them.
const MODERATION_GROUPS: [(&str, &[&str]); 3] = [
    ("sexual", &["sexual", "sexual/minors"]),
    ("violence", &["violence", "violence/graphic"]),
    (
        "self_harm",
        &["self-harm", "self-harm/intent", "self-harm/instructions"],
    ),
];

/// One image through OpenAI's free moderation filter: the groups it flags.
/// Nothing is stored or retried; any failure is an error the caller holds on.
pub async fn moderate_image(
    api_key: &str,
    image: &AiImage,
    deadline: Duration,
) -> Result<Vec<&'static str>, AiError> {
    moderate_image_at(OPENAI_BASE_URL, api_key, image, deadline).await
}

pub(super) async fn moderate_image_at(
    base_url: &str,
    api_key: &str,
    image: &AiImage,
    deadline: Duration,
) -> Result<Vec<&'static str>, AiError> {
    let headers = headers(AiProvider::OpenAi, api_key)?;
    let body = serde_json::to_vec(&json!({
        "model": MODERATION_MODEL,
        "input": [{ "type": "image_url", "image_url": {
            "url": format!("data:{};base64,{}", image.mime, base64(&image.data)),
        } }],
    }))
    .map_err(|_| AiError::BadRequest)?;
    let response = send_once(
        &format!("{base_url}/v1/moderations"),
        &headers,
        &body,
        deadline,
    )
    .await
    .map_err(|(error, _)| error)?;
    parse_moderation(&response)
}

pub(super) fn parse_moderation(response: &Value) -> Result<Vec<&'static str>, AiError> {
    let categories = response["results"][0]["categories"]
        .as_object()
        .ok_or(AiError::InvalidOutput)?;
    Ok(MODERATION_GROUPS
        .iter()
        .filter(|(_, flags)| {
            flags
                .iter()
                .any(|flag| categories.get(*flag).and_then(Value::as_bool) == Some(true))
        })
        .map(|(group, _)| *group)
        .collect())
}

pub(super) fn parse_reply(provider: AiProvider, response: &Value) -> Result<AiReply, AiError> {
    let usage = &response["usage"];
    let text = match provider {
        AiProvider::Anthropic => {
            match response["stop_reason"].as_str() {
                Some("refusal") => return Err(AiError::Refused),
                Some("max_tokens") => return Err(AiError::InvalidOutput),
                _ => {}
            }
            response["content"]
                .as_array()
                .and_then(|blocks| blocks.iter().find(|block| block["type"] == "text"))
                .and_then(|block| block["text"].as_str())
        }
        AiProvider::OpenAi => {
            if response["status"] == "incomplete" {
                return Err(
                    if response["incomplete_details"]["reason"] == "content_filter" {
                        AiError::Refused
                    } else {
                        AiError::InvalidOutput
                    },
                );
            }
            let content = response["output"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|item| item["type"] == "message")
                .filter_map(|item| item["content"].as_array())
                .flatten();
            let mut text = None;
            for part in content {
                match part["type"].as_str() {
                    Some("refusal") => return Err(AiError::Refused),
                    Some("output_text") if text.is_none() => text = part["text"].as_str(),
                    _ => {}
                }
            }
            text
        }
    };
    let value = text
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .filter(Value::is_object)
        .ok_or(AiError::InvalidOutput)?;
    Ok(AiReply {
        value,
        input_tokens: usage["input_tokens"].as_u64().unwrap_or(0),
        output_tokens: usage["output_tokens"].as_u64().unwrap_or(0),
    })
}

#[cfg(test)]
mod tests;
