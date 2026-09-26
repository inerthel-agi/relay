use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::Cursor,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::{Arc, OnceLock},
    time::Duration,
};

use anyhow::{Context, Result};
use exif::{In, Reader, Tag};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

pub const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;
pub const MAX_IMAGE_DIMENSION: u32 = 16_384;
pub const MAX_IMAGE_PIXELS: u64 = 40_000_000;
pub const MAX_IMAGE_FRAMES: u32 = 256;
pub const MAX_ANIMATED_PIXELS: u64 = 500_000_000;
pub const OCR_TEXT_LIMIT: usize = 8_192;
pub const PRIVACY_TEXT_LIMIT: usize = 4_096;
const IMAGE_SCAN_TIMEOUT: Duration = Duration::from_secs(12);
const IMAGE_SCAN_CONCURRENCY: usize = 2;
pub const MAX_CONFIGURED_REGEXES: usize = 100;
pub const MAX_PRIVACY_LIST_ENTRIES: usize = 100;
pub const MAX_PRIVACY_LIST_VALUE_CHARS: usize = 256;
const MAX_REGEXES_PER_CONCEPT: usize = 25;
const MAX_REGEX_PATTERN_BYTES: usize = 512;
const REGEX_SIZE_LIMIT: usize = 256 * 1024;
const REGEX_DFA_SIZE_LIMIT: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProtectionLevel {
    #[default]
    Balanced,
    Strict,
    Paranoid,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PrivacyCategory {
    Email,
    Phone,
    IpAddress,
    GpsLocation,
    PostalAddress,
    Financial,
    LicensePlate,
    SensitiveUrl,
    CustomPattern,
    ContentFilter,
    ImageMetadata,
    Ocr,
    Document,
    MediaSafety,
}

impl PrivacyCategory {
    pub const USER_CONFIGURABLE: [Self; 12] = [
        Self::Email,
        Self::Phone,
        Self::IpAddress,
        Self::GpsLocation,
        Self::PostalAddress,
        Self::Financial,
        Self::LicensePlate,
        Self::SensitiveUrl,
        Self::CustomPattern,
        Self::ImageMetadata,
        Self::Ocr,
        Self::Document,
    ];

    pub fn log_code(self) -> &'static str {
        match self {
            Self::Email => "EMAIL",
            Self::Phone => "PHONE",
            Self::IpAddress => "IP_ADDRESS",
            Self::GpsLocation => "GPS_LOCATION",
            Self::PostalAddress => "POSTAL_ADDRESS",
            Self::Financial => "FINANCIAL",
            Self::LicensePlate => "LICENSE_PLATE",
            Self::SensitiveUrl => "SENSITIVE_URL",
            Self::CustomPattern => "CUSTOM_PATTERN",
            Self::ContentFilter => "CONTENT_FILTER",
            Self::ImageMetadata => "IMAGE_METADATA",
            Self::Ocr => "OCR",
            Self::Document => "DOCUMENT",
            Self::MediaSafety => "MEDIA_SAFETY",
        }
    }
}

pub fn default_privacy_categories() -> Vec<PrivacyCategory> {
    PrivacyCategory::USER_CONFIGURABLE.to_vec()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForbiddenConcept {
    pub canonical: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub regexes: Vec<String>,
}

impl ForbiddenConcept {
    pub fn validate(&self) -> Result<()> {
        if !self.canonical.chars().any(char::is_alphabetic) {
            anyhow::bail!("Forbidden concepts must contain alphabetic characters.");
        }
        let canonical = normalize_compact(&self.canonical);
        if !(3..=64).contains(&canonical.chars().count()) {
            anyhow::bail!("Forbidden concept canonical values must contain 3 to 64 letters.");
        }
        if self.aliases.len() > 50 {
            anyhow::bail!("Each forbidden concept may contain at most 50 aliases.");
        }
        for alias in &self.aliases {
            if !alias.chars().any(char::is_alphabetic) {
                anyhow::bail!("Forbidden concept aliases must contain alphabetic characters.");
            }
            let normalized = normalize_compact(alias);
            if !(3..=64).contains(&normalized.chars().count()) {
                anyhow::bail!("Forbidden concept aliases must contain 3 to 64 letters.");
            }
        }
        if self.regexes.len() > MAX_REGEXES_PER_CONCEPT {
            anyhow::bail!(
                "Each forbidden concept may contain at most {MAX_REGEXES_PER_CONCEPT} regular expressions."
            );
        }
        for pattern in &self.regexes {
            if pattern.is_empty() || pattern.len() > MAX_REGEX_PATTERN_BYTES {
                anyhow::bail!(
                    "Forbidden concept regular expressions must contain 1 to {MAX_REGEX_PATTERN_BYTES} bytes."
                );
            }
            let regex = compile_filter_regex(pattern).map_err(|_| {
                anyhow::anyhow!("Forbidden concept regular expressions are invalid.")
            })?;
            if regex.is_match("") {
                anyhow::bail!(
                    "Forbidden concept regular expressions must not match an empty value."
                );
            }
        }
        Ok(())
    }
}

fn compile_filter_regex(pattern: &str) -> Result<Regex> {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(REGEX_DFA_SIZE_LIMIT)
        .build()
        .context("Invalid filter regular expression")
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PrivacyClassification {
    #[default]
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl PrivacyClassification {
    #[allow(non_upper_case_globals)]
    pub const Suspicious: Self = Self::Medium;
    #[allow(non_upper_case_globals)]
    pub const Sensitive: Self = Self::High;

    fn rank(self) -> u8 {
        match self {
            Self::Safe => 0,
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }

    fn log_code(self) -> &'static str {
        match self {
            Self::Safe => "SAFE",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PrivacyReport {
    pub classification: PrivacyClassification,
    pub score: u8,
    pub categories: Vec<PrivacyCategory>,
    /// Stable, minimized reason codes. Never put OCR/EXIF values here.
    pub reasons: Vec<&'static str>,
    /// Signature of the privacy configuration snapshot used for this report.
    /// It is process-local metadata and is never serialized to the UI.
    pub config_signature: Option<u64>,
}

impl PrivacyReport {
    pub fn safe() -> Self {
        Self::default()
    }

    pub fn suspicious(reason: &'static str) -> Self {
        Self {
            classification: PrivacyClassification::Medium,
            score: 35,
            categories: reason_category(reason).into_iter().collect(),
            reasons: vec![reason],
            config_signature: None,
        }
    }

    pub fn low(reason: &'static str) -> Self {
        Self {
            classification: PrivacyClassification::Low,
            score: 10,
            categories: reason_category(reason).into_iter().collect(),
            reasons: vec![reason],
            config_signature: None,
        }
    }

    #[cfg(test)]
    pub fn sensitive(reason: &'static str) -> Self {
        Self {
            classification: PrivacyClassification::High,
            score: 70,
            categories: reason_category(reason).into_iter().collect(),
            reasons: vec![reason],
            config_signature: None,
        }
    }

    pub fn merge(&mut self, other: Self) {
        let other_signature = other.config_signature;
        if other.classification.rank() > self.classification.rank() {
            self.classification = other.classification;
        }
        self.score = self.score.saturating_add(other.score).min(100);
        for category in other.categories {
            if !self.categories.contains(&category) {
                self.categories.push(category);
            }
        }
        for reason in other.reasons {
            if !self.reasons.contains(&reason) {
                self.reasons.push(reason);
            }
        }
        if self.config_signature.is_none() {
            self.config_signature = other_signature;
        }
    }

    pub fn without_filter_signals(mut self) -> Self {
        let has_filter_signal = self.categories.contains(&PrivacyCategory::ContentFilter)
            || self.reasons.iter().any(|reason| {
                matches!(
                    *reason,
                    "forbidden_concept"
                        | "forbidden_regex"
                        | "forbidden_similarity"
                        | "similarity_score"
                        | "discord_invite"
                        | "link_shortener"
                        | "scam_domain"
                )
            });
        if !has_filter_signal {
            return self;
        }
        self.reasons.retain(|reason| {
            !matches!(
                *reason,
                "forbidden_concept"
                    | "forbidden_regex"
                    | "forbidden_similarity"
                    | "similarity_score"
                    | "discord_invite"
                    | "link_shortener"
                    | "scam_domain"
            )
        });
        self.classification = if self.reasons.is_empty() {
            PrivacyClassification::Safe
        } else if self.reasons.iter().any(|reason| {
            matches!(
                *reason,
                "gps"
                    | "coordinates"
                    | "ip_address"
                    | "partial_address"
                    | "location_profile"
                    | "address_or_visual_context"
                    | "image_limits"
            )
        }) {
            PrivacyClassification::High
        } else {
            PrivacyClassification::Medium
        };
        self.categories
            .retain(|category| *category != PrivacyCategory::ContentFilter);
        self.score = match self.classification {
            PrivacyClassification::Safe => 0,
            PrivacyClassification::Low => 15,
            PrivacyClassification::Medium => 35,
            PrivacyClassification::High => 70,
            PrivacyClassification::Critical => 100,
        };
        self
    }

    pub fn primary_reason(&self) -> Option<&'static str> {
        self.reasons.first().copied()
    }

    pub fn apply_score_policy(&mut self, config: &AppConfig) {
        let scored = risk_for_score(self.score, config.privacy_protection_level);
        if scored.rank() > self.classification.rank() {
            self.classification = scored;
        }
    }

    fn add_signal(
        &mut self,
        category: PrivacyCategory,
        points: u8,
        minimum: PrivacyClassification,
        reason: &'static str,
        config: &AppConfig,
    ) {
        self.score = self.score.saturating_add(points).min(100);
        if !self.categories.contains(&category) {
            self.categories.push(category);
        }
        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
        }
        let scored = risk_for_score(self.score, config.privacy_protection_level);
        self.classification = if scored.rank() > minimum.rank() {
            scored
        } else if minimum.rank() > self.classification.rank() {
            minimum
        } else {
            self.classification
        };
    }
}

fn reason_category(reason: &str) -> Option<PrivacyCategory> {
    Some(match reason {
        "email" => PrivacyCategory::Email,
        "phone" => PrivacyCategory::Phone,
        "ip_address" => PrivacyCategory::IpAddress,
        "gps" | "coordinates" | "location_profile" => PrivacyCategory::GpsLocation,
        "postal_address" | "partial_address" | "address_or_visual_context" => {
            PrivacyCategory::PostalAddress
        }
        "iban" | "payment_card" => PrivacyCategory::Financial,
        "license_plate" => PrivacyCategory::LicensePlate,
        "sensitive_url" => PrivacyCategory::SensitiveUrl,
        "custom_pattern" => PrivacyCategory::CustomPattern,
        "forbidden_concept"
        | "forbidden_regex"
        | "forbidden_similarity"
        | "similarity_score"
        | "discord_invite"
        | "link_shortener"
        | "scam_domain" => PrivacyCategory::ContentFilter,
        "exif_metadata" => PrivacyCategory::ImageMetadata,
        "document" => PrivacyCategory::Document,
        "image_limits" | "mime_mismatch" => PrivacyCategory::MediaSafety,
        "ocr_text" => PrivacyCategory::Ocr,
        _ => return None,
    })
}

fn risk_for_score(score: u8, level: ProtectionLevel) -> PrivacyClassification {
    let (low, medium, high, critical) = match level {
        ProtectionLevel::Balanced => (15, 30, 60, 90),
        ProtectionLevel::Strict => (10, 25, 50, 80),
        ProtectionLevel::Paranoid => (5, 20, 40, 70),
    };
    if score >= critical {
        PrivacyClassification::Critical
    } else if score >= high {
        PrivacyClassification::High
    } else if score >= medium {
        PrivacyClassification::Medium
    } else if score >= low {
        PrivacyClassification::Low
    } else {
        PrivacyClassification::Safe
    }
}

#[derive(Clone, Debug, Default)]
struct ImageSignals {
    gps: bool,
    exif_incomplete: bool,
    ocr_text: String,
    ocr_truncated: bool,
    dimensions_valid: bool,
    decoded_pixels: u64,
    frame_count: u32,
    ocr_available: bool,
}

/// Applies all text-only rules. The returned report contains only coarse reason
/// codes, so OCR and Discord text never leave this function.
pub fn classify_text(text: Option<&str>, config: &AppConfig) -> PrivacyReport {
    let config_signature = Some(config_signature(config));
    if !privacy_rules_enabled(config) {
        return PrivacyReport {
            config_signature,
            ..PrivacyReport::safe()
        };
    }
    let Some(text) = text.filter(|value| !value.trim().is_empty()) else {
        return PrivacyReport {
            config_signature,
            ..PrivacyReport::safe()
        };
    };
    let (text, text_truncated) = cap_text(text);
    let cleaned = strip_invisible_characters(text);
    let masked = mask_allowlisted_values(&cleaned, &config.privacy_allowlist);
    let normalized = normalize_words(&masked);
    let mut report = PrivacyReport {
        config_signature,
        ..PrivacyReport::safe()
    };
    let concepts = crate::moderation::effective_concepts(config);
    let concept_matches = forbidden_concept_match(&masked, &normalized, &concepts);
    if let Some(reason) = crate::moderation::link_violation(&cleaned, &config.moderation) {
        report.add_signal(
            PrivacyCategory::ContentFilter,
            100,
            PrivacyClassification::Critical,
            reason,
            config,
        );
    }
    if concept_matches.regex {
        report.add_signal(
            PrivacyCategory::ContentFilter,
            100,
            PrivacyClassification::Critical,
            "forbidden_regex",
            config,
        );
    }

    if normalized.is_empty() {
        if text_truncated {
            report.merge(PrivacyReport::low("scan_incomplete"));
        }
        return report;
    }
    if concept_matches.exact {
        report.add_signal(
            PrivacyCategory::ContentFilter,
            100,
            PrivacyClassification::Critical,
            "forbidden_concept",
            config,
        );
    }
    if concept_matches.similarities > 0 {
        report.add_signal(
            PrivacyCategory::ContentFilter,
            config
                .privacy_similarity_boost
                .saturating_mul(concept_matches.similarities)
                .saturating_mul(20),
            PrivacyClassification::Medium,
            if config.privacy_similarity_boost >= 4 {
                "similarity_score"
            } else {
                "forbidden_similarity"
            },
            config,
        );
    }

    if !config.privacy_scan_enabled {
        if text_truncated {
            report.merge(PrivacyReport::suspicious("scan_incomplete"));
        }
        return report;
    }

    let rule_words = normalize_rule_words(&masked);
    let has_email = category_enabled(config, PrivacyCategory::Email) && contains_email(&masked);
    if has_email {
        report.add_signal(
            PrivacyCategory::Email,
            15,
            PrivacyClassification::Low,
            "email",
            config,
        );
    }
    let has_phone = category_enabled(config, PrivacyCategory::Phone) && contains_phone(&masked);
    if has_phone {
        report.add_signal(
            PrivacyCategory::Phone,
            30,
            PrivacyClassification::Medium,
            "phone",
            config,
        );
    }
    if category_enabled(config, PrivacyCategory::IpAddress) && contains_ip_address(&masked) {
        report.add_signal(
            PrivacyCategory::IpAddress,
            30,
            PrivacyClassification::Medium,
            "ip_address",
            config,
        );
    }
    if category_enabled(config, PrivacyCategory::GpsLocation)
        && coordinate_signal(&masked).is_some()
    {
        report.add_signal(
            PrivacyCategory::GpsLocation,
            70,
            PrivacyClassification::High,
            "coordinates",
            config,
        );
    }
    let address = if category_enabled(config, PrivacyCategory::PostalAddress) {
        postal_address_signal(&masked, &rule_words)
    } else {
        AddressSignal::None
    };
    match address {
        AddressSignal::Partial => report.add_signal(
            PrivacyCategory::PostalAddress,
            15,
            PrivacyClassification::Low,
            "partial_address",
            config,
        ),
        AddressSignal::Probable => report.add_signal(
            PrivacyCategory::PostalAddress,
            60,
            PrivacyClassification::High,
            "postal_address",
            config,
        ),
        AddressSignal::Full => report.add_signal(
            PrivacyCategory::PostalAddress,
            65,
            PrivacyClassification::High,
            "postal_address",
            config,
        ),
        AddressSignal::None => {}
    }
    if category_enabled(config, PrivacyCategory::Financial) && contains_valid_iban(&masked) {
        report.add_signal(
            PrivacyCategory::Financial,
            75,
            PrivacyClassification::High,
            "iban",
            config,
        );
    }
    if category_enabled(config, PrivacyCategory::Financial) && contains_valid_payment_card(&masked)
    {
        report.add_signal(
            PrivacyCategory::Financial,
            100,
            PrivacyClassification::Critical,
            "payment_card",
            config,
        );
    }
    if category_enabled(config, PrivacyCategory::LicensePlate) && contains_license_plate(&masked) {
        report.add_signal(
            PrivacyCategory::LicensePlate,
            35,
            PrivacyClassification::Medium,
            "license_plate",
            config,
        );
    }
    if category_enabled(config, PrivacyCategory::SensitiveUrl) && contains_sensitive_url(&masked) {
        report.add_signal(
            PrivacyCategory::SensitiveUrl,
            45,
            PrivacyClassification::Medium,
            "sensitive_url",
            config,
        );
    }
    if category_enabled(config, PrivacyCategory::Document) && contains_document_signal(&masked) {
        report.add_signal(
            PrivacyCategory::Document,
            50,
            PrivacyClassification::Medium,
            "document",
            config,
        );
    }
    if category_enabled(config, PrivacyCategory::CustomPattern)
        && custom_pattern_match(&normalized, &config.privacy_custom_patterns)
    {
        report.add_signal(
            PrivacyCategory::CustomPattern,
            75,
            PrivacyClassification::High,
            "custom_pattern",
            config,
        );
    }

    let has_name = contains_person_name_label(&rule_words);
    if has_name && matches!(address, AddressSignal::Probable | AddressSignal::Full) {
        report.add_signal(
            PrivacyCategory::PostalAddress,
            30,
            PrivacyClassification::High,
            "person_address_combination",
            config,
        );
    }
    if has_name && has_phone {
        report.add_signal(
            PrivacyCategory::Phone,
            35,
            PrivacyClassification::High,
            "person_phone_combination",
            config,
        );
    }
    if has_email && has_phone && !matches!(address, AddressSignal::None) {
        report.classification = PrivacyClassification::Critical;
        report.score = 100;
    }
    if text_truncated {
        report.add_signal(
            PrivacyCategory::MediaSafety,
            5,
            PrivacyClassification::Low,
            "scan_incomplete",
            config,
        );
    }
    report
}

/// Downloads and inspects an image from the Discord CDN. Failures are kept
/// deliberately coarse and become a reviewable signal rather than leaking
/// parser/network details to the UI or logs.
pub async fn analyze_remote_image(
    url: &str,
    proxy_url: &str,
    text: Option<&str>,
    config: &AppConfig,
) -> PrivacyReport {
    let mut report = classify_text(text, config);
    if !config.privacy_scan_enabled {
        return report;
    }

    let bytes = match download_image_bounded(url).await {
        Ok(bytes) => bytes,
        Err(_) if proxy_url != url => match download_image_bounded(proxy_url).await {
            Ok(bytes) => bytes,
            Err(_) => {
                report.merge(PrivacyReport::suspicious("image_fetch_unavailable"));
                return report;
            }
        },
        Err(_) => {
            report.merge(PrivacyReport::suspicious("image_fetch_unavailable"));
            return report;
        }
    };

    let scanned = analyze_image_bytes_async(&bytes, text, config).await;
    report.merge(scanned);
    report.apply_score_policy(config);
    report
}

pub fn image_limit_report(text: Option<&str>, config: &AppConfig) -> PrivacyReport {
    let mut report = classify_text(text, config);
    report.add_signal(
        PrivacyCategory::MediaSafety,
        80,
        PrivacyClassification::High,
        "image_limits",
        config,
    );
    report
}

/// Deterministic in-memory image analysis entry point used by the bot and
/// fixtures. It never returns image bytes, EXIF values, or OCR text.
pub fn analyze_image_bytes(bytes: &[u8], text: Option<&str>, config: &AppConfig) -> PrivacyReport {
    let mut report = classify_text(text, config);
    if !config.privacy_scan_enabled {
        return report;
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        report.add_signal(
            PrivacyCategory::MediaSafety,
            80,
            PrivacyClassification::High,
            "image_limits",
            config,
        );
        return report;
    }
    if !supported_image_signature(bytes) {
        report.add_signal(
            PrivacyCategory::MediaSafety,
            70,
            PrivacyClassification::High,
            "mime_mismatch",
            config,
        );
        return report;
    }
    // Parse GPS independently of the platform decoder so a malformed or
    // unsupported image cannot turn a location-bearing asset into a safe one.
    // EXIF parsing remains best-effort and never exposes field values.
    let exif = parse_exif(bytes);
    if exif.gps && category_enabled(config, PrivacyCategory::GpsLocation) {
        report.add_signal(
            PrivacyCategory::GpsLocation,
            75,
            PrivacyClassification::High,
            "gps",
            config,
        );
    }
    if exif.sensitive_metadata && category_enabled(config, PrivacyCategory::ImageMetadata) {
        report.add_signal(
            PrivacyCategory::ImageMetadata,
            10,
            PrivacyClassification::Low,
            "exif_metadata",
            config,
        );
    }
    let bytes = bytes.to_vec();
    let ocr_requested = category_enabled(config, PrivacyCategory::Ocr);
    let signals = match inspect_image(bytes, ocr_requested) {
        Ok(signals) => signals,
        Err(_) => {
            if exif.incomplete {
                report.merge(PrivacyReport::low("scan_incomplete"));
            }
            report.merge(PrivacyReport::low("image_scan_unavailable"));
            return report;
        }
    };
    if signals.gps && category_enabled(config, PrivacyCategory::GpsLocation) {
        report.add_signal(
            PrivacyCategory::GpsLocation,
            75,
            PrivacyClassification::High,
            "gps",
            config,
        );
    }
    // OCR inspects only frame zero; later animation frames still need review.
    if exif.incomplete
        || signals.exif_incomplete
        || signals.ocr_truncated
        || (ocr_requested && signals.frame_count > 1)
    {
        report.merge(PrivacyReport::low("scan_incomplete"));
    }
    if !signals.dimensions_valid
        || signals.frame_count == 0
        || signals.frame_count > MAX_IMAGE_FRAMES
        || signals
            .decoded_pixels
            .saturating_mul(u64::from(signals.frame_count))
            > MAX_ANIMATED_PIXELS
    {
        report.add_signal(
            PrivacyCategory::MediaSafety,
            80,
            PrivacyClassification::High,
            "image_limits",
            config,
        );
        return report;
    }
    if ocr_requested && signals.ocr_available {
        let ocr_report = classify_text(Some(&signals.ocr_text), config);
        if ocr_report.classification != PrivacyClassification::Safe
            && !report.categories.contains(&PrivacyCategory::Ocr)
        {
            report.categories.push(PrivacyCategory::Ocr);
        }
        report.merge(ocr_report);
    } else if ocr_requested {
        report.merge(PrivacyReport::low("scan_incomplete"));
    }
    report.apply_score_policy(config);
    report
}

/// Runs the platform decoder behind a bounded worker pool. The semaphore
/// permit is moved into the blocking closure, so a timed-out WinRT operation
/// cannot cause unbounded concurrent decoder jobs.
pub async fn analyze_image_bytes_async(
    bytes: &[u8],
    text: Option<&str>,
    config: &AppConfig,
) -> PrivacyReport {
    if !config.privacy_scan_enabled {
        return classify_text(text, config);
    }
    let semaphore = scan_semaphore();
    let Ok(Ok(permit)) =
        tokio::time::timeout(IMAGE_SCAN_TIMEOUT, semaphore.clone().acquire_owned()).await
    else {
        return PrivacyReport::low("scan_incomplete");
    };
    let bytes = bytes.to_vec();
    let text = text.map(str::to_owned);
    let config = config.clone();
    match tokio::time::timeout(
        IMAGE_SCAN_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            analyze_image_bytes(&bytes, text.as_deref(), &config)
        }),
    )
    .await
    {
        Ok(Ok(report)) => report,
        _ => PrivacyReport::low("scan_incomplete"),
    }
}

async fn download_image_bounded(url: &str) -> Result<Vec<u8>> {
    let parsed = reqwest::Url::parse(url).context("invalid image URL")?;
    if !discord_cdn_url(&parsed) {
        anyhow::bail!("image URL host is not an approved Discord CDN");
    }
    let response = image_client()
        .get(parsed)
        .send()
        .await?
        .error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_IMAGE_BYTES as u64)
    {
        anyhow::bail!("image exceeds the in-memory limit");
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len() + chunk.len() > MAX_IMAGE_BYTES {
            anyhow::bail!("image exceeds the in-memory limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn scan_semaphore() -> &'static Arc<tokio::sync::Semaphore> {
    static SEMAPHORE: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    SEMAPHORE.get_or_init(|| Arc::new(tokio::sync::Semaphore::new(IMAGE_SCAN_CONCURRENCY)))
}

fn image_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(12))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 4 {
                    attempt.error("too many redirects")
                } else if discord_cdn_url(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.error("redirect outside the Discord CDN")
                }
            }))
            .build()
            .expect("valid privacy image client")
    })
}

fn discord_cdn_url(url: &reqwest::Url) -> bool {
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    ["discordapp.com", "discordapp.net", "discord.com"]
        .into_iter()
        .any(|suffix| host == suffix || host.ends_with(&format!(".{suffix}")))
}

fn supported_image_signature(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || bytes.starts_with(&[0xff, 0xd8])
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP".as_slice()))
        || bytes.starts_with(b"BM")
}

fn inspect_image(bytes: Vec<u8>, ocr_requested: bool) -> Result<ImageSignals> {
    let exif = parse_exif(&bytes);
    let gps = exif.gps;
    #[cfg(target_os = "windows")]
    {
        let mut signals = inspect_image_windows(&bytes, ocr_requested)?;
        signals.gps = gps;
        signals.exif_incomplete = exif.incomplete;
        Ok(signals)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let (width, height, frame_count) = basic_image_limits(&bytes);
        Ok(ImageSignals {
            gps,
            exif_incomplete: exif.incomplete,
            ocr_text: String::new(),
            ocr_truncated: false,
            dimensions_valid: width > 0
                && height > 0
                && width <= MAX_IMAGE_DIMENSION
                && height <= MAX_IMAGE_DIMENSION
                && u64::from(width) * u64::from(height) <= MAX_IMAGE_PIXELS,
            decoded_pixels: u64::from(width) * u64::from(height),
            frame_count,
            ocr_available: false,
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ExifSignals {
    gps: bool,
    sensitive_metadata: bool,
    incomplete: bool,
}

fn parse_exif(bytes: &[u8]) -> ExifSignals {
    // This EXIF reader does not support GIF containers; their format error is not a failed scan.
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return ExifSignals::default();
    }
    let mut cursor = Cursor::new(bytes);
    let exif = match Reader::new().read_from_container(&mut cursor) {
        Ok(exif) => exif,
        Err(exif::Error::NotFound(_)) => return ExifSignals::default(),
        Err(_) => {
            return ExifSignals {
                incomplete: true,
                ..ExifSignals::default()
            };
        }
    };
    let has_lat = exif.get_field(Tag::GPSLatitude, In::PRIMARY).is_some();
    let has_lon = exif.get_field(Tag::GPSLongitude, In::PRIMARY).is_some();
    let has_ref = exif
        .get_field(Tag::GPSLatitudeRef, In::PRIMARY)
        .or_else(|| exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY))
        .is_some();
    let coordinate_pair = has_lat && has_lon;
    let coordinate_with_ref = has_ref && (has_lat || has_lon);
    let sensitive_metadata = [
        Tag::Make,
        Tag::Model,
        Tag::DateTime,
        Tag::DateTimeOriginal,
        Tag::BodySerialNumber,
        Tag::LensSerialNumber,
        Tag::UserComment,
    ]
    .into_iter()
    .any(|tag| exif.get_field(tag, In::PRIMARY).is_some());
    ExifSignals {
        gps: coordinate_pair || coordinate_with_ref,
        sensitive_metadata,
        incomplete: false,
    }
}

#[cfg(not(target_os = "windows"))]
fn basic_image_limits(bytes: &[u8]) -> (u32, u32, u32) {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.len() >= 24 {
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap_or_default());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap_or_default());
        return (width, height, 1);
    }
    if bytes.starts_with(b"GIF8") && bytes.len() >= 10 {
        let width = u16::from_le_bytes(bytes[6..8].try_into().unwrap_or_default()) as u32;
        let height = u16::from_le_bytes(bytes[8..10].try_into().unwrap_or_default()) as u32;
        let frames = bytes
            .windows(8)
            .filter(|window| *window == b"\x00\x21\xF9\x04")
            .count();
        return (width, height, frames.max(1) as u32);
    }
    if bytes.starts_with(&[0xff, 0xd8]) {
        let mut index = 2;
        while index + 9 < bytes.len() {
            if bytes[index] != 0xff {
                index += 1;
                continue;
            }
            let marker = bytes[index + 1];
            index += 2;
            if marker == 0xd8 || marker == 0xd9 || (0xd0..=0xd7).contains(&marker) {
                continue;
            }
            if index + 2 > bytes.len() {
                break;
            }
            let length = u16::from_be_bytes([bytes[index], bytes[index + 1]]) as usize;
            if length < 2 || index + length > bytes.len() {
                break;
            }
            if (0xc0..=0xc3).contains(&marker)
                || (0xc5..=0xc7).contains(&marker)
                || (0xc9..=0xcb).contains(&marker)
                || (0xcd..=0xcf).contains(&marker)
            {
                if length >= 7 {
                    let height = u16::from_be_bytes([bytes[index + 3], bytes[index + 4]]) as u32;
                    let width = u16::from_be_bytes([bytes[index + 5], bytes[index + 6]]) as u32;
                    return (width, height, 1);
                }
            }
            index += length;
        }
    }
    (0, 0, 0)
}

#[cfg(target_os = "windows")]
fn inspect_image_windows(bytes: &[u8], ocr_requested: bool) -> Result<ImageSignals> {
    use windows::{
        Globalization::Language,
        Graphics::Imaging::BitmapDecoder,
        Media::Ocr::OcrEngine,
        Storage::Streams::{DataWriter, InMemoryRandomAccessStream},
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        core::HSTRING,
    };

    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.context("WinRT initialization failed")?;
    struct RoGuard;
    impl Drop for RoGuard {
        fn drop(&mut self) {
            unsafe { RoUninitialize() };
        }
    }
    let _ro_guard = RoGuard;
    (|| {
        let stream = InMemoryRandomAccessStream::new()?;
        let output = stream.GetOutputStreamAt(0)?;
        let writer = DataWriter::CreateDataWriter(&output)?;
        writer.WriteBytes(bytes)?;
        writer.StoreAsync()?.get()?;
        writer.FlushAsync()?.get()?;
        writer.DetachStream()?;
        stream.Seek(0)?;
        let decoder = BitmapDecoder::CreateAsync(&stream)?.get()?;
        let frame_count = decoder.FrameCount()?;
        let frame = decoder.GetFrameAsync(0)?.get()?;
        let width = frame.PixelWidth()?;
        let height = frame.PixelHeight()?;
        let dimensions_valid = width > 0
            && height > 0
            && width <= MAX_IMAGE_DIMENSION
            && height <= MAX_IMAGE_DIMENSION
            && u64::from(width) * u64::from(height) <= MAX_IMAGE_PIXELS;
        if !dimensions_valid || frame_count == 0 || frame_count > MAX_IMAGE_FRAMES {
            return Ok(ImageSignals {
                gps: false,
                exif_incomplete: false,
                ocr_text: String::new(),
                ocr_truncated: false,
                dimensions_valid,
                decoded_pixels: u64::from(width) * u64::from(height),
                frame_count,
                ocr_available: false,
            });
        }
        let bitmap = if ocr_requested {
            Some(frame.GetSoftwareBitmapAsync()?.get()?)
        } else {
            None
        };
        let mut ocr_text = String::new();
        let mut ocr_truncated = false;
        let mut ocr_available = false;
        for tag in ["fr-FR", "en-US"] {
            let Some(bitmap) = bitmap.as_ref() else {
                break;
            };
            let Ok(language) = Language::CreateLanguage(&HSTRING::from(tag)) else {
                continue;
            };
            let Ok(engine) = OcrEngine::TryCreateFromLanguage(&language) else {
                continue;
            };
            let Ok(result) = engine
                .RecognizeAsync(bitmap)
                .and_then(|operation| operation.get())
            else {
                continue;
            };
            let Ok(text) = result.Text() else {
                continue;
            };
            let text = text.to_string();
            ocr_truncated = text.chars().count() > OCR_TEXT_LIMIT;
            ocr_text = text.chars().take(OCR_TEXT_LIMIT).collect();
            ocr_available = true;
            if !ocr_text.trim().is_empty() {
                break;
            }
        }
        Ok(ImageSignals {
            gps: false,
            exif_incomplete: false,
            ocr_text,
            ocr_truncated,
            dimensions_valid,
            decoded_pixels: u64::from(width) * u64::from(height),
            frame_count,
            ocr_available,
        })
    })()
}

fn category_enabled(config: &AppConfig, category: PrivacyCategory) -> bool {
    matches!(
        category,
        PrivacyCategory::ContentFilter | PrivacyCategory::MediaSafety
    ) || config.privacy_enabled_categories.contains(&category)
}

fn strip_invisible_characters(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            !matches!(
                *character,
                '\u{00ad}'
                    | '\u{034f}'
                    | '\u{061c}'
                    | '\u{180e}'
                    | '\u{200b}'..='\u{200f}'
                    | '\u{202a}'..='\u{202e}'
                    | '\u{2060}'..='\u{206f}'
                    | '\u{feff}'
            )
        })
        .collect()
}

fn mask_allowlisted_values(text: &str, allowlist: &[String]) -> String {
    let mut masked = text.to_owned();
    for value in allowlist {
        let value = value.trim();
        if value.chars().count() < 3 {
            continue;
        }
        let Ok(regex) = RegexBuilder::new(&regex::escape(value))
            .case_insensitive(true)
            .size_limit(REGEX_SIZE_LIMIT)
            .dfa_size_limit(REGEX_DFA_SIZE_LIMIT)
            .build()
        else {
            continue;
        };
        masked = regex.replace_all(&masked, " ").into_owned();
    }
    masked
}

fn deobfuscate_contact_text(text: &str) -> String {
    static AT: OnceLock<Regex> = OnceLock::new();
    static DOT: OnceLock<Regex> = OnceLock::new();
    let at = AT.get_or_init(|| {
        Regex::new(r"(?iu)\s*(?:\[\s*at\s*\]|\(\s*at\s*\)|\bat\b)\s*")
            .expect("valid email at matcher")
    });
    let dot = DOT.get_or_init(|| {
        Regex::new(r"(?iu)\s*(?:\[\s*dot\s*\]|\(\s*dot\s*\)|\bdot\b)\s*")
            .expect("valid email dot matcher")
    });
    let text = at.replace_all(text, "@");
    dot.replace_all(&text, ".").into_owned()
}

fn contains_email(text: &str) -> bool {
    static EMAIL: OnceLock<Regex> = OnceLock::new();
    let email = EMAIL.get_or_init(|| {
        Regex::new(
            r"(?iu)\b[a-z0-9.!#$%&'*+/=?^_`{|}~-]{1,64}@[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)+\b",
        )
        .expect("valid email matcher")
    });
    email.is_match(&deobfuscate_contact_text(text))
}

fn numeric_candidates(text: &str) -> Vec<(String, String)> {
    let mut candidates = Vec::new();
    let mut raw = String::new();
    let mut digits = String::new();
    let mut started = false;
    let flush = |raw: &mut String, digits: &mut String, candidates: &mut Vec<(String, String)>| {
        if !digits.is_empty() {
            candidates.push((std::mem::take(raw), std::mem::take(digits)));
        } else {
            raw.clear();
        }
    };
    for character in text.chars().chain(std::iter::once('\0')) {
        if character.is_ascii_digit() {
            raw.push(character);
            digits.push(character);
            started = true;
        } else if started && matches!(character, '+' | '-' | '.' | '/' | '(' | ')' | ' ') {
            raw.push(character);
        } else if !started && character == '+' {
            raw.push(character);
            started = true;
        } else {
            flush(&mut raw, &mut digits, &mut candidates);
            started = false;
        }
    }
    candidates
}

fn contains_phone(text: &str) -> bool {
    numeric_candidates(text).into_iter().any(|(raw, digits)| {
        if !(10..=15).contains(&digits.len()) {
            return false;
        }
        let separators = raw
            .chars()
            .filter(|character| matches!(character, ' ' | '-' | '.' | '/' | '(' | ')'))
            .count();
        let international = raw.trim_start().starts_with('+') || digits.starts_with("00");
        let national = digits.len() == 10 && digits.starts_with('0');
        (international || national || (digits.len() == 10 && separators >= 2))
            && !looks_like_payment_card(&digits)
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AddressSignal {
    None,
    Partial,
    Probable,
    Full,
}

fn postal_address_signal(text: &str, words: &[String]) -> AddressSignal {
    let street_types = [
        "allee",
        "avenue",
        "av",
        "boulevard",
        "bd",
        "chemin",
        "drive",
        "impasse",
        "lane",
        "place",
        "quai",
        "road",
        "route",
        "rue",
        "square",
        "street",
        "voie",
        "way",
    ];
    for (index, word) in words.iter().enumerate() {
        if !(1..=5).contains(&word.len())
            || !word.chars().all(|character| character.is_ascii_digit())
        {
            continue;
        }
        let following = words.iter().skip(index + 1).take(5).collect::<Vec<_>>();
        let Some(type_index) = following
            .iter()
            .position(|candidate| street_types.contains(&candidate.as_str()))
        else {
            if has_address_prefix(text)
                && following.first().is_some_and(|candidate| {
                    candidate.chars().count() >= 3 && candidate.chars().all(char::is_alphabetic)
                })
            {
                return AddressSignal::Probable;
            }
            continue;
        };
        let has_street_name = following
            .iter()
            .skip(type_index + 1)
            .take(3)
            .any(|candidate| {
                candidate.chars().count() >= 2 && candidate.chars().all(char::is_alphabetic)
            })
            || (type_index > 0
                && following[..type_index].iter().any(|candidate| {
                    candidate.chars().count() >= 2 && candidate.chars().all(char::is_alphabetic)
                }));
        if !has_street_name {
            return AddressSignal::Partial;
        }
        let tail = words.iter().skip(index + 1).take(12).collect::<Vec<_>>();
        let has_postcode_city = tail.windows(2).any(|pair| {
            (4..=6).contains(&pair[0].len())
                && pair[0].chars().all(|character| character.is_ascii_digit())
                && pair[1].chars().count() >= 2
                && pair[1].chars().all(char::is_alphabetic)
        });
        return if has_postcode_city {
            AddressSignal::Full
        } else {
            AddressSignal::Probable
        };
    }
    AddressSignal::None
}

fn contains_valid_iban(text: &str) -> bool {
    static IBAN: OnceLock<Regex> = OnceLock::new();
    let regex = IBAN.get_or_init(|| {
        Regex::new(r"(?iu)\b[A-Z]{2}\s*\d{2}(?:[\s-]?[A-Z0-9]){11,30}\b")
            .expect("valid IBAN matcher")
    });
    regex.find_iter(text).any(|candidate| {
        let mut compact = candidate
            .as_str()
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .map(|character| character.to_ascii_uppercase())
            .collect::<String>();
        if let Some(expected) = iban_country_length(&compact[..compact.len().min(2)])
            && compact.len() >= expected
        {
            compact.truncate(expected);
        }
        if !(15..=34).contains(&compact.len())
            || !compact[..2]
                .chars()
                .all(|character| character.is_ascii_alphabetic())
            || !compact[2..4]
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            return false;
        }
        let rearranged = format!("{}{}", &compact[4..], &compact[..4]);
        let mut remainder = 0_u32;
        for character in rearranged.chars() {
            if character.is_ascii_digit() {
                remainder = (remainder * 10 + character.to_digit(10).unwrap_or_default()) % 97;
            } else {
                let value = character as u32 - 'A' as u32 + 10;
                remainder = (remainder * 100 + value) % 97;
            }
        }
        remainder == 1
    })
}

fn iban_country_length(country: &str) -> Option<usize> {
    Some(match country {
        "AT" => 20,
        "BE" => 16,
        "CH" => 21,
        "DE" => 22,
        "DK" | "FI" | "NO" => 18,
        "ES" | "PT" => 24,
        "FR" | "IT" => 27,
        "GB" => 22,
        "IE" => 22,
        "LU" => 20,
        "NL" => 18,
        "PL" => 28,
        "SE" => 24,
        _ => return None,
    })
}

fn contains_valid_payment_card(text: &str) -> bool {
    numeric_candidates(text)
        .into_iter()
        .any(|(_, digits)| looks_like_payment_card(&digits) && luhn_valid(&digits))
}

fn looks_like_payment_card(digits: &str) -> bool {
    let length = digits.len();
    let prefix2 = digits.get(..2).and_then(|value| value.parse::<u16>().ok());
    let prefix4 = digits.get(..4).and_then(|value| value.parse::<u16>().ok());
    (digits.starts_with('4') && matches!(length, 13 | 16 | 19))
        || (length == 16
            && (prefix2.is_some_and(|value| (51..=55).contains(&value))
                || prefix4.is_some_and(|value| (2221..=2720).contains(&value))
                || digits.starts_with("6011")
                || prefix2.is_some_and(|value| (64..=65).contains(&value))))
        || (length == 15 && matches!(prefix2, Some(34 | 37)))
}

fn luhn_valid(digits: &str) -> bool {
    if digits.bytes().all(|digit| digit == digits.as_bytes()[0]) {
        return false;
    }
    let sum = digits
        .bytes()
        .rev()
        .enumerate()
        .map(|(index, digit)| {
            let mut value = u32::from(digit - b'0');
            if index % 2 == 1 {
                value *= 2;
                if value > 9 {
                    value -= 9;
                }
            }
            value
        })
        .sum::<u32>();
    sum % 10 == 0
}

fn contains_license_plate(text: &str) -> bool {
    static PLATE: OnceLock<Regex> = OnceLock::new();
    let regex = PLATE.get_or_init(|| {
        Regex::new(r"(?iu)\b[A-Z]{2}[- ]?\d{3}[- ]?[A-Z]{2}\b")
            .expect("valid license plate matcher")
    });
    let context = normalize_rule_words(text).iter().any(|word| {
        matches!(
            word.as_str(),
            "immat" | "immatriculation" | "license" | "plate" | "plaque" | "registration"
        )
    });
    regex
        .find_iter(text)
        .any(|candidate| context || candidate.as_str().contains(['-', ' ']))
}

fn contains_sensitive_url(text: &str) -> bool {
    static URL: OnceLock<Regex> = OnceLock::new();
    let regex = URL.get_or_init(|| {
        Regex::new(r#"(?iu)https?://[^\s<>\"']{1,2048}"#).expect("valid URL matcher")
    });
    regex.find_iter(text).any(|candidate| {
        let trimmed = candidate
            .as_str()
            .trim_end_matches([',', '.', ')', ']', '}', ';']);
        let Ok(url) = reqwest::Url::parse(trimmed) else {
            return false;
        };
        if !url.username().is_empty() || url.password().is_some() {
            return true;
        }
        url.query_pairs().any(|(key, value)| {
            matches!(
                normalize_compact(&key).as_str(),
                "access_token"
                    | "accesstoken"
                    | "address"
                    | "adresse"
                    | "apikey"
                    | "auth"
                    | "email"
                    | "iban"
                    | "key"
                    | "lat"
                    | "latitude"
                    | "lon"
                    | "longitude"
                    | "phone"
                    | "telephone"
                    | "token"
            ) || contains_email(&value)
                || contains_phone(&value)
                || contains_ip_address(&value)
        })
    })
}

fn contains_document_signal(text: &str) -> bool {
    let normalized = normalize_rule_words(text);
    let keyword_count = normalized
        .iter()
        .filter(|word| {
            matches!(
                word.as_str(),
                "administration"
                    | "birth"
                    | "carte"
                    | "date"
                    | "driver"
                    | "identity"
                    | "identite"
                    | "license"
                    | "naissance"
                    | "nationalite"
                    | "passport"
                    | "passeport"
                    | "permis"
                    | "securite"
                    | "social"
                    | "surname"
            )
        })
        .count();
    let long_number = numeric_candidates(text)
        .iter()
        .any(|(_, digits)| digits.len() >= 8);
    keyword_count >= 3 || (keyword_count >= 2 && long_number)
}

fn custom_pattern_match(words: &[String], patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        let expected = normalize_compact(pattern);
        expected.chars().count() >= 3
            && matches!(
                variant_match(words, &expected),
                Some(ConceptMatchKind::Exact)
            )
    })
}

fn contains_person_name_label(words: &[String]) -> bool {
    words.iter().enumerate().any(|(index, word)| {
        matches!(
            word.as_str(),
            "fullname" | "name" | "nom" | "prenom" | "surname"
        ) && words
            .iter()
            .skip(index + 1)
            .take(3)
            .filter(|candidate| {
                candidate.chars().count() >= 2 && candidate.chars().all(char::is_alphabetic)
            })
            .count()
            >= 2
    })
}

fn coordinate_signal(text: &str) -> Option<PrivacyClassification> {
    let numbers = text
        .split(|character: char| {
            !(character.is_ascii_digit() || matches!(character, '.' | '-' | '+'))
        })
        .filter_map(|part| {
            let number = part.parse::<f64>().ok()?;
            number
                .is_finite()
                .then_some((number, decimal_precision(part)))
        })
        .collect::<Vec<_>>();
    let coordinate_hint = text.to_ascii_lowercase();
    numbers.windows(2).find_map(|pair| {
        let decimal_pair =
            pair[0].0.abs() <= 90.0 && pair[1].0.abs() <= 180.0 && pair[0].1 > 0 && pair[1].1 > 0;
        if !decimal_pair {
            return None;
        }
        let explicit_hint = has_numeric_comma_pair(text)
            || coordinate_hint
                .split(|character: char| !character.is_ascii_alphanumeric())
                .any(|word| {
                    matches!(
                        word,
                        "gps"
                            | "lat"
                            | "latitude"
                            | "lon"
                            | "long"
                            | "longitude"
                            | "coord"
                            | "coordinates"
                    )
                });
        if explicit_hint || (pair[0].1 >= 4 && pair[1].1 >= 4 && pair[0].0.abs() >= 20.0) {
            Some(PrivacyClassification::Sensitive)
        } else if pair[0].1 >= 3 && pair[1].1 >= 3 {
            Some(PrivacyClassification::Suspicious)
        } else {
            None
        }
    })
}

fn has_address_prefix(value: &str) -> bool {
    normalize_rule_words(value)
        .iter()
        .rev()
        .take(4)
        .any(|word| {
            matches!(
                word.as_str(),
                "address"
                    | "adresse"
                    | "domicile"
                    | "home"
                    | "house"
                    | "residence"
                    | "habite"
                    | "jhabite"
                    | "live"
                    | "lives"
                    | "living"
                    | "reside"
                    | "resides"
                    | "chez"
            )
        })
}

fn contains_ip_address(text: &str) -> bool {
    static IPV4: OnceLock<Regex> = OnceLock::new();
    let ipv4 = IPV4.get_or_init(|| {
        Regex::new(
            r"(?:25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])(?:\.(?:25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])){3}",
        )
        .expect("valid IPv4 matcher")
    });
    ipv4.find_iter(text).any(|candidate| {
        let before = text[..candidate.start()].chars().next_back();
        let after = text[candidate.end()..].chars().next();
        if before.is_some_and(|character| character.is_ascii_alphanumeric() || character == '.')
            || after.is_some_and(|character| character.is_ascii_alphanumeric() || character == '.')
        {
            return false;
        }
        !matches!(
            normalize_rule_words(&text[..candidate.start()])
                .last()
                .map(String::as_str),
            Some("version" | "ver" | "v")
        ) && candidate.as_str().parse::<Ipv4Addr>().is_ok()
    }) || text
        .split(|character: char| !character.is_ascii_hexdigit() && character != ':')
        .any(|candidate| {
            candidate.contains(':')
                && candidate.parse::<Ipv6Addr>().is_ok()
                && candidate.parse::<IpAddr>().is_ok()
        })
}

fn has_numeric_comma_pair(text: &str) -> bool {
    let numeric = |value: &str| {
        !value.is_empty()
            && value
                .chars()
                .all(|character| character.is_ascii_digit() || matches!(character, '.' | '-' | '+'))
            && value.parse::<f64>().is_ok()
    };
    for (index, character) in text.char_indices() {
        if character != ',' {
            continue;
        }
        let left = text[..index]
            .trim_end()
            .rsplit(|character: char| {
                !(character.is_ascii_digit() || matches!(character, '.' | '-' | '+'))
            })
            .next()
            .unwrap_or_default();
        let right = text[index + character.len_utf8()..]
            .trim_start()
            .split(|character: char| {
                !(character.is_ascii_digit() || matches!(character, '.' | '-' | '+'))
            })
            .next()
            .unwrap_or_default();
        if numeric(left) && numeric(right) {
            return true;
        }
    }
    false
}

fn decimal_precision(value: &str) -> usize {
    value
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len())
}

fn cap_text(value: &str) -> (&str, bool) {
    value
        .char_indices()
        .nth(PRIVACY_TEXT_LIMIT)
        .map_or((value, false), |(index, _)| (&value[..index], true))
}

fn normalize_words(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(normalize_compact)
        .filter(|word| !word.is_empty())
        .collect()
}

fn normalize_rule_words(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(|segment| {
            segment
                .chars()
                .flat_map(char::to_lowercase)
                .filter_map(|character| {
                    let mapped = match character {
                        'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
                        'ç' => 'c',
                        'è' | 'é' | 'ê' | 'ë' => 'e',
                        'ì' | 'í' | 'î' | 'ï' => 'i',
                        'ñ' => 'n',
                        'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
                        'ù' | 'ú' | 'û' | 'ü' => 'u',
                        'ý' | 'ÿ' => 'y',
                        character if character.is_ascii_alphanumeric() => character,
                        character if character.is_alphanumeric() => character,
                        _ => return None,
                    };
                    Some(mapped)
                })
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

fn normalize_compact(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter_map(|character| {
            let mapped = match character {
                'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
                'ç' => 'c',
                'è' | 'é' | 'ê' | 'ë' => 'e',
                'ì' | 'í' | 'î' | 'ï' => 'i',
                'ñ' => 'n',
                'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
                'ù' | 'ú' | 'û' | 'ü' => 'u',
                'ý' | 'ÿ' => 'y',
                'і' | 'ӏ' | '1' => 'i',
                'е' | 'ё' | '3' => 'e',
                'а' | '4' => 'a',
                'о' | '0' => 'o',
                'ѕ' | '5' => 's',
                'т' | '7' => 't',
                'х' => 'x',
                'р' => 'p',
                'с' => 'c',
                character if character.is_ascii_alphanumeric() => character,
                character if character.is_alphanumeric() => character,
                _ => return None,
            };
            Some(mapped)
        })
        .collect()
}

#[derive(Default)]
struct ConceptMatchSignals {
    exact: bool,
    regex: bool,
    similarities: u8,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ConceptMatchKind {
    Exact,
    Similarity,
}

fn forbidden_concept_match(
    text: &str,
    words: &[String],
    concepts: &[ForbiddenConcept],
) -> ConceptMatchSignals {
    let mut signals = ConceptMatchSignals::default();
    for concept in concepts {
        if concept
            .regexes
            .iter()
            .any(|pattern| compile_filter_regex(pattern).is_ok_and(|regex| regex.is_match(text)))
        {
            signals.regex = true;
            return signals;
        }

        let mut variants = vec![concept.canonical.as_str()];
        variants.extend(concept.aliases.iter().map(String::as_str));
        let mut similar = false;
        for variant in variants {
            let normalized = normalize_compact(variant);
            if normalized.chars().count() < 3 {
                continue;
            }
            let phrase = normalize_words(variant);
            let expected = if phrase.len() > 1 {
                phrase.join("")
            } else {
                normalized
            };
            match variant_match(words, &expected) {
                Some(ConceptMatchKind::Exact) => {
                    signals.exact = true;
                    return signals;
                }
                Some(ConceptMatchKind::Similarity) => similar = true,
                None => {}
            }
        }
        if similar {
            signals.similarities = signals.similarities.saturating_add(1);
        }
    }
    signals
}

fn variant_match(words: &[String], expected: &str) -> Option<ConceptMatchKind> {
    let mut similar = false;
    for word in words {
        match concept_word_match(word, expected) {
            Some(ConceptMatchKind::Exact) => return Some(ConceptMatchKind::Exact),
            Some(ConceptMatchKind::Similarity) => similar = true,
            None => {}
        }
    }
    for matcher in [
        composite_split_match(words, expected),
        separated_letter_match(words, expected),
    ] {
        match matcher {
            Some(ConceptMatchKind::Exact) => return Some(ConceptMatchKind::Exact),
            Some(ConceptMatchKind::Similarity) => similar = true,
            None => {}
        }
    }
    similar.then_some(ConceptMatchKind::Similarity)
}

fn concept_word_match(candidate: &str, expected: &str) -> Option<ConceptMatchKind> {
    if candidate == expected {
        return Some(ConceptMatchKind::Exact);
    }
    let reduced = collapse_repeated_runs(candidate);
    if expected.chars().count() >= 6 && reduced != candidate && reduced == expected {
        return Some(ConceptMatchKind::Similarity);
    }
    (cautious_distance(candidate, expected)
        || (reduced != candidate && cautious_distance(&reduced, expected)))
    .then_some(ConceptMatchKind::Similarity)
}

fn composite_split_match(words: &[String], expected: &str) -> Option<ConceptMatchKind> {
    let expected_len = expected.chars().count();
    if expected_len < 3 {
        return None;
    }
    let mut similar = false;
    for start in 0..words.len() {
        let mut candidate = String::new();
        for (count, word) in words
            .iter()
            .skip(start)
            .take(expected_len.saturating_add(1))
            .enumerate()
        {
            if word.is_empty() {
                break;
            }
            candidate.push_str(word);
            let candidate_len = candidate.chars().count();
            if candidate_len > expected_len + 1 {
                break;
            }
            if count >= 1 {
                match concept_word_match(&candidate, expected) {
                    Some(ConceptMatchKind::Exact) => return Some(ConceptMatchKind::Exact),
                    Some(ConceptMatchKind::Similarity) => similar = true,
                    None => {}
                }
            }
        }
    }
    similar.then_some(ConceptMatchKind::Similarity)
}

fn collapse_repeated_runs(value: &str) -> String {
    let mut reduced = String::with_capacity(value.len());
    for character in value.chars() {
        if !reduced.ends_with(character) {
            reduced.push(character);
        }
    }
    reduced
}

fn cautious_distance(left: &str, right: &str) -> bool {
    let length = left.chars().count().max(right.chars().count());
    if length < 6 || left.chars().count().abs_diff(right.chars().count()) > 1 {
        return false;
    }
    if bounded_edit_distance(left, right) != 1 {
        return false;
    }
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    if left_chars.len() == right_chars.len() {
        let differing = left_chars
            .iter()
            .zip(right_chars.iter())
            .filter(|(left, right)| left != right)
            .collect::<Vec<_>>();
        return differing.len() == 1 && confusable_pair(*differing[0].0, *differing[0].1);
    }
    let (longer, shorter) = if left_chars.len() > right_chars.len() {
        (&left_chars, &right_chars)
    } else {
        (&right_chars, &left_chars)
    };
    let insertion = (0..longer.len()).find(|index| {
        longer[..*index]
            .iter()
            .chain(longer[index + 1..].iter())
            .eq(shorter.iter())
    });
    insertion.is_some_and(|index| {
        let inserted = longer[index];
        (index > 0 && longer[index - 1] == inserted)
            || (index + 1 < longer.len() && longer[index + 1] == inserted)
    })
}

fn confusable_pair(left: char, right: char) -> bool {
    matches!(
        (left, right),
        ('l', 'i')
            | ('i', 'l')
            | ('l', '1')
            | ('1', 'l')
            | ('e', '3')
            | ('3', 'e')
            | ('e', 'i')
            | ('i', 'e')
    )
}

fn separated_letter_match(words: &[String], expected: &str) -> Option<ConceptMatchKind> {
    let expected_len = expected.chars().count();
    if expected_len < 3 {
        return None;
    }
    let mut similar = false;
    for window in words.windows(expected_len) {
        if !window.iter().all(|word| word.chars().count() == 1) {
            continue;
        }
        match concept_word_match(&window.join(""), expected) {
            Some(ConceptMatchKind::Exact) => return Some(ConceptMatchKind::Exact),
            Some(ConceptMatchKind::Similarity) => similar = true,
            None => {}
        }
    }
    similar.then_some(ConceptMatchKind::Similarity)
}

fn bounded_edit_distance(left: &str, right: &str) -> usize {
    let right = right.chars().collect::<Vec<_>>();
    let mut row = (0..=right.len()).collect::<Vec<_>>();
    for (index, left_char) in left.chars().enumerate() {
        let mut next = vec![index + 1; right.len() + 1];
        for (right_index, right_char) in right.iter().enumerate() {
            next[right_index + 1] = (row[right_index + 1] + 1)
                .min(next[right_index] + 1)
                .min(row[right_index] + usize::from(left_char != *right_char));
        }
        row = next;
    }
    row[right.len()]
}

pub fn action_for(report: &PrivacyReport, config: &AppConfig) -> PrivacyAction {
    if report.classification == PrivacyClassification::Critical
        || report.classification.rank() >= config.privacy_block_threshold.rank()
    {
        return PrivacyAction::Block;
    }
    if config.privacy_scan_enabled
        && report.reasons.iter().any(|reason| {
            matches!(
                *reason,
                "scan_incomplete" | "image_scan_unavailable" | "image_fetch_unavailable"
            )
        })
    {
        return PrivacyAction::Review;
    }
    match report.classification {
        PrivacyClassification::High => PrivacyAction::Review,
        PrivacyClassification::Medium if config.privacy_review_intermediate => {
            PrivacyAction::Review
        }
        PrivacyClassification::Safe
        | PrivacyClassification::Low
        | PrivacyClassification::Medium => PrivacyAction::Allow,
        PrivacyClassification::Critical => PrivacyAction::Block,
    }
}

pub fn log_decision(report: &PrivacyReport, action: PrivacyAction) {
    if matches!(action, PrivacyAction::Allow)
        || report.classification == PrivacyClassification::Safe
    {
        return;
    }
    let detected = if report.categories.is_empty() {
        "UNCLASSIFIED".to_owned()
    } else {
        report
            .categories
            .iter()
            .map(|category| category.log_code())
            .collect::<Vec<_>>()
            .join("+")
    };
    let action = match action {
        PrivacyAction::Allow => "ALLOWED",
        PrivacyAction::Review => "REVIEW",
        PrivacyAction::Block => "BLOCKED",
    };
    eprintln!(
        "Privacy Risk: {} Detected: {detected} Action: {action}",
        report.classification.log_code()
    );
}

/// Text filters remain active even when the optional image metadata/OCR scan
/// is disabled. The latter is deliberately kept separate so filter-only
/// configurations do not make ordinary images incomplete by default.
pub fn privacy_rules_enabled(config: &AppConfig) -> bool {
    config.privacy_scan_enabled
        || !config.privacy_concepts.is_empty()
        || crate::moderation::content_rules_enabled(config)
}

/// True when filter words (custom or built-in lists) match, used for names
/// and titles shown on stream where only the word filter applies.
pub fn filter_words_match(text: &str, config: &AppConfig) -> bool {
    let concepts = crate::moderation::effective_concepts(config);
    if concepts.is_empty() || text.trim().is_empty() {
        return false;
    }
    let (text, _) = cap_text(text);
    let cleaned = strip_invisible_characters(text);
    let masked = mask_allowlisted_values(&cleaned, &config.privacy_allowlist);
    let normalized = normalize_words(&masked);
    let matches = forbidden_concept_match(&masked, &normalized, &concepts);
    matches.exact || matches.regex
}

pub fn has_exempt_role(config: &AppConfig, message_role_ids: &[String]) -> bool {
    !message_role_ids.is_empty()
        && config
            .privacy_filter_exempt_role_ids
            .iter()
            .any(|configured| message_role_ids.iter().any(|role| role == configured))
}

pub fn scoped_config_for_roles(config: &AppConfig, message_role_ids: &[String]) -> AppConfig {
    if !has_exempt_role(config, message_role_ids) {
        return config.clone();
    }
    let mut scoped = config.clone();
    crate::moderation::clear_content_rules(&mut scoped);
    scoped
}

pub fn config_signature(config: &AppConfig) -> u64 {
    let mut hasher = DefaultHasher::new();
    config.privacy_scan_enabled.hash(&mut hasher);
    config.privacy_similarity_boost.hash(&mut hasher);
    config.privacy_protection_level.hash(&mut hasher);
    config.privacy_block_threshold.hash(&mut hasher);
    config.privacy_review_intermediate.hash(&mut hasher);
    for category in &config.privacy_enabled_categories {
        category.hash(&mut hasher);
    }
    for value in &config.privacy_allowlist {
        value.hash(&mut hasher);
    }
    for value in &config.privacy_custom_patterns {
        value.hash(&mut hasher);
    }
    for role_id in &config.privacy_filter_exempt_role_ids {
        role_id.hash(&mut hasher);
    }
    let moderation = &config.moderation;
    for pack in &moderation.word_packs {
        pack.hash(&mut hasher);
    }
    moderation.pack_exclusions.hash(&mut hasher);
    moderation.block_invites.hash(&mut hasher);
    moderation.block_shorteners.hash(&mut hasher);
    moderation.block_scam_domains.hash(&mut hasher);
    for concept in &config.privacy_concepts {
        concept.canonical.hash(&mut hasher);
        for alias in &concept.aliases {
            alias.hash(&mut hasher);
        }
        for pattern in &concept.regexes {
            pattern.hash(&mut hasher);
        }
    }
    hasher.finish()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivacyAction {
    Allow,
    Review,
    Block,
}

#[cfg(test)]
mod tests;
