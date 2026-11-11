//! Image check before the stream: OpenAI's free moderation filter (sexual,
//! violence, self-harm), then one request to the chosen provider's paid model
//! (hate, harassment, spam, and personal data for the relay) per image.
//! Without an OpenAI key, the paid model checks the free categories too.
//!
//! A clearly harmful image is blocked, with the category that matched in its
//! reason; a doubt (paid model unsure, free filter below its block score and
//! not settled by the paid model), a failed check or an image that cannot be
//! sent is held for review. Nothing is
//! sent before the local personal-data scan of the exact bytes
//! (`advice::image_payload`).

use std::time::Duration;

use super::{
    AiError, AiSettings, ImageGuardCategory,
    advice::{self, AiAdvice, AiCategory, AiVerdict, NotSentCause},
    client::{self, AiImage},
};
use crate::{
    config::AppConfig,
    model::MediaKind,
    privacy::{self, PrivacyReport},
};

const FREE_DEADLINE: Duration = Duration::from_secs(10);

/// Hold reasons and block reason, as shown in the review queue and the log.
/// The image could not be checked (no key, limit, provider down, unreadable file).
pub const HOLD_UNCHECKED: &str = "ai_image_unchecked";
pub const HOLD_PERSONAL_DATA: &str = "ai_personal_data";
/// The AI had a doubt about harmful content: it waits for approval.
pub const HOLD_DOUBT: &str = "ai_image";
/// The scan on this PC found personal data or a location: nothing was sent.
pub const HOLD_NOT_SENT: &str = "ai_image_not_sent";
/// Local scan results the relay may settle (`personal_data_incomplete`).
const INCOMPLETE: [&str; 2] = ["scan_incomplete", "image_scan_unavailable"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// Not concerned: Relay's usual checks decide alone.
    Skip,
    /// Checked, nothing found.
    Clear,
    /// Checked, nothing found, and the relay settled a scan that could not finish.
    Release,
    Hold(&'static str),
    /// Blocked; the reason names the category that matched.
    Block(&'static str),
}

/// Block reason for one category, as shown in the log.
fn blocked(category: ImageGuardCategory) -> &'static str {
    match category {
        ImageGuardCategory::Sexual => "ai_image_blocked_sexual",
        ImageGuardCategory::Violence => "ai_image_blocked_violence",
        ImageGuardCategory::SelfHarm => "ai_image_blocked_self_harm",
        ImageGuardCategory::Hate => "ai_image_blocked_hate",
        ImageGuardCategory::Harassment => "ai_image_blocked_harassment",
        ImageGuardCategory::Spam => "ai_image_blocked_spam",
    }
}

/// One media of a message, before it is submitted.
pub struct Candidate<'a> {
    pub kind: MediaKind,
    pub url: &'a str,
    pub proxy_url: &'a str,
    /// A sticker already downloaded.
    pub bytes: Option<&'a [u8]>,
    /// The local scan of this media.
    pub report: &'a PrivacyReport,
    /// Discord ID of the member who posted it: paid requests use their share.
    pub member: &'a str,
}

/// The local scan stopped short and found nothing else.
pub fn scan_incomplete_only(report: &PrivacyReport) -> bool {
    !report.reasons.is_empty()
        && report
            .reasons
            .iter()
            .all(|reason| INCOMPLETE.contains(reason))
}

/// Which checks apply: (image check, personal-data relay).
fn wanted(settings: &AiSettings, animated: bool, jpeg: bool, incomplete: bool) -> (bool, bool) {
    let guard = settings.image_guard
        && !settings.image_guard_categories.is_empty()
        && (!animated || settings.image_guard_animated);
    let relay = settings.personal_data_relay
        && ((incomplete && settings.personal_data_incomplete)
            || (animated && settings.personal_data_videos)
            || (!animated && jpeg && settings.personal_data_photos)
            || (!animated && !jpeg && settings.personal_data_screenshots));
    (guard, relay)
}

fn enabled(settings: &AiSettings, category: ImageGuardCategory) -> bool {
    settings.image_guard_categories.contains(&category)
}

/// What the free filter flagged, among the categories turned on: a block
/// when it is sure (`client::MODERATION_BLOCK_SCORE`), a hold otherwise.
fn free_hit(settings: &AiSettings, groups: &[(&str, bool)]) -> Option<Outcome> {
    let mut doubt = None;
    for (group, sure) in groups {
        let category = match *group {
            "sexual" => ImageGuardCategory::Sexual,
            "violence" => ImageGuardCategory::Violence,
            "self_harm" => ImageGuardCategory::SelfHarm,
            _ => continue,
        };
        if !enabled(settings, category) {
            continue;
        }
        if *sure {
            return Some(Outcome::Block(blocked(category)));
        }
        doubt = Some(Outcome::Hold(HOLD_DOUBT));
    }
    doubt
}

/// What the paid model found, among the categories turned on.
fn paid_hit(settings: &AiSettings, categories: &[AiCategory]) -> Option<ImageGuardCategory> {
    categories.iter().find_map(|category| {
        let wanted: &[ImageGuardCategory] = match category {
            AiCategory::Hate => &[ImageGuardCategory::Hate],
            AiCategory::Harassment => &[ImageGuardCategory::Harassment],
            AiCategory::Threat => &[ImageGuardCategory::Harassment, ImageGuardCategory::Violence],
            AiCategory::Spam => &[ImageGuardCategory::Spam],
            AiCategory::Sexual => &[ImageGuardCategory::Sexual],
            AiCategory::SelfHarm => &[ImageGuardCategory::SelfHarm],
            AiCategory::Violence => &[ImageGuardCategory::Violence],
            _ => &[],
        };
        wanted
            .iter()
            .copied()
            .find(|category| enabled(settings, *category))
    })
}

/// Decision from the paid model's opinion on one image; `None` lets it pass.
fn from_advice(
    settings: &AiSettings,
    guard: bool,
    relay: bool,
    advice: &AiAdvice,
) -> Option<Outcome> {
    let AiAdvice::Done {
        verdict,
        categories,
        ..
    } = advice
    else {
        return Some(Outcome::Hold(HOLD_UNCHECKED));
    };
    let harmful = guard.then(|| paid_hit(settings, categories)).flatten();
    let personal = relay
        && categories.iter().any(|category| {
            matches!(
                category,
                AiCategory::PersonalData | AiCategory::PrivateConversation
            )
        });
    // A doubt about harmful content waits for approval, never shown nor
    // deleted (user's choice, 2026-11-10). Personal data the model listed
    // holds the image whatever its verdict.
    match (verdict, harmful) {
        (AiVerdict::ClearProblem, Some(category)) => Some(Outcome::Block(blocked(category))),
        (AiVerdict::Ambiguous, Some(_)) => Some(Outcome::Hold(HOLD_DOUBT)),
        _ => personal.then_some(Outcome::Hold(HOLD_PERSONAL_DATA)),
    }
}

fn not_sent(cause: NotSentCause) -> Outcome {
    match cause {
        NotSentCause::PersonalData | NotSentCause::Location => Outcome::Hold(HOLD_NOT_SENT),
        _ => Outcome::Hold(HOLD_UNCHECKED),
    }
}

async fn download(candidate: &Candidate<'_>) -> Option<Vec<u8>> {
    if let Some(bytes) = candidate.bytes {
        return Some(bytes.to_vec());
    }
    match privacy::download_image_bounded(candidate.url).await {
        Ok(bytes) => Some(bytes),
        Err(_) => privacy::download_image_bounded(candidate.proxy_url)
            .await
            .ok(),
    }
}

/// Frames to check and which checks apply, or the outcome when there is nothing to check.
async fn frames(
    settings: &AiSettings,
    candidate: &Candidate<'_>,
    incomplete: bool,
) -> Result<(Vec<Vec<u8>>, bool, bool), Outcome> {
    let animated_frames = |bytes: Option<Vec<u8>>| async move {
        match bytes {
            Some(bytes) => crate::media_compat::frames_from_bytes(bytes).await,
            None => crate::media_compat::frames_from_url(candidate.url, candidate.proxy_url).await,
        }
    };
    match candidate.kind {
        MediaKind::Audio => Err(Outcome::Skip),
        MediaKind::Gif | MediaKind::Video => {
            let (guard, relay) = wanted(settings, true, false, incomplete);
            if !guard && !relay {
                return Err(Outcome::Skip);
            }
            let frames = animated_frames(candidate.bytes.map(<[u8]>::to_vec)).await;
            Ok((frames.ok_or(Outcome::Hold(HOLD_UNCHECKED))?, guard, relay))
        }
        MediaKind::Image => {
            let bytes = download(candidate)
                .await
                .ok_or(Outcome::Hold(HOLD_UNCHECKED))?;
            let Some(mime) = advice::image_mime(&bytes) else {
                // A format the providers do not read, or a Lottie sticker.
                return Err(if candidate.bytes.is_some() {
                    Outcome::Skip
                } else {
                    Outcome::Hold(HOLD_UNCHECKED)
                });
            };
            let animated = advice::is_animated(&bytes, mime);
            let (guard, relay) = wanted(settings, animated, mime == "image/jpeg", incomplete);
            if !guard && !relay {
                return Err(Outcome::Skip);
            }
            if !animated {
                return Ok((vec![bytes], guard, relay));
            }
            let frames = animated_frames(Some(bytes)).await;
            Ok((frames.ok_or(Outcome::Hold(HOLD_UNCHECKED))?, guard, relay))
        }
    }
}

/// Whether an image check can run. Turning the assistant off (no provider)
/// stops every check, the free filter included.
pub fn active(settings: &AiSettings) -> bool {
    settings.provider.is_some() && (settings.image_guard || settings.personal_data_relay)
}

/// Checks one media of a member. Moderators and trusted members are never
/// checked: the caller does not call this for them.
pub async fn check(config: &AppConfig, candidate: Candidate<'_>, language: &str) -> Outcome {
    let settings = &config.ai;
    if !active(settings) {
        return Outcome::Skip;
    }
    let incomplete = scan_incomplete_only(candidate.report);
    let (frames, guard, relay) = match frames(settings, &candidate, incomplete).await {
        Ok(found) => found,
        Err(outcome) => return outcome,
    };
    // The frames of a GIF or video are checked at the same time: one wait
    // instead of three before it can be shown.
    let member = candidate.member;
    let outcomes = futures_util::future::join_all(frames.into_iter().map(|frame| async move {
        match advice::image_payload(frame, config).await {
            Ok(image) => check_image(settings, (guard, relay), image, language, member).await,
            Err(cause) => Some(not_sent(cause)),
        }
    }))
    .await;
    let mut held = None;
    for outcome in outcomes {
        if let Some(outcome) = fold_frame(&mut held, outcome) {
            return outcome;
        }
    }
    if let Some(outcome) = held {
        return outcome;
    }
    if relay && incomplete && settings.personal_data_incomplete {
        Outcome::Release
    } else {
        Outcome::Clear
    }
}

/// Keeps the strongest result over the frames of a GIF or video: a block ends
/// the check, a hold is kept while later frames are still checked for a block,
/// and a check that cannot run stops (the next frames would fail the same way).
fn fold_frame(held: &mut Option<Outcome>, outcome: Option<Outcome>) -> Option<Outcome> {
    match outcome? {
        Outcome::Block(reason) => Some(Outcome::Block(reason)),
        Outcome::Hold(HOLD_UNCHECKED) => Some(held.unwrap_or(Outcome::Hold(HOLD_UNCHECKED))),
        outcome => {
            held.get_or_insert(outcome);
            None
        }
    }
}

/// Which requests one image needs: (free filter, paid model). Without an
/// OpenAI key, the paid model checks the free filter's categories too.
fn requests(settings: &AiSettings, guard: bool, relay: bool, openai_key: bool) -> (bool, bool) {
    let any = |free: bool| {
        guard
            && settings
                .image_guard_categories
                .iter()
                .any(|category| category.free() == free)
    };
    let free = any(true) && openai_key;
    (free, relay || any(false) || (any(true) && !free))
}

/// The free filter first (a block there saves the paid request), then at
/// most one paid request. `None` when nothing was found.
async fn check_image(
    settings: &AiSettings,
    (guard, relay): (bool, bool),
    image: AiImage,
    language: &str,
    member: &str,
) -> Option<Outcome> {
    let key = requests(settings, guard, relay, true)
        .0
        .then(|| crate::credentials::load_ai_api_key(super::AiProvider::OpenAi).ok())
        .flatten()
        .flatten();
    // Loaded only when the free filter is wanted, so a key means it runs.
    let (_, paid) = requests(settings, guard, relay, key.is_some());
    // A doubt of the free filter is kept unless the paid model settles it.
    let mut doubt = None;
    if let Some(key) = key {
        match client::moderate_image(&key, &image, FREE_DEADLINE).await {
            Ok(groups) => match free_hit(settings, &groups) {
                Some(Outcome::Block(reason)) => return Some(Outcome::Block(reason)),
                hit => doubt = hit,
            },
            Err(_) => return Some(Outcome::Hold(HOLD_UNCHECKED)),
        }
    }
    if !paid {
        return doubt;
    }
    let advice = match settings.provider {
        Some(provider) => {
            advice::ask(
                settings,
                provider,
                &advice::image_request(image, language),
                Some(member),
            )
            .await
        }
        None => AiAdvice::Failed {
            cause: AiError::NoProvider,
        },
    };
    strongest(
        doubt,
        from_advice(settings, guard, relay, &advice),
        settles_doubt(settings, &advice),
    )
}

/// The paid model saw the image (its prompt names the free filter's
/// categories) and found nothing harmful among the categories turned on:
/// no issue at all, or only categories that are off. An answer without a
/// category or with `other` settles nothing unless it saw no issue.
fn settles_doubt(settings: &AiSettings, advice: &AiAdvice) -> bool {
    let AiAdvice::Done {
        verdict,
        categories,
        ..
    } = advice
    else {
        return false;
    };
    paid_hit(settings, categories).is_none()
        && (*verdict == AiVerdict::NoIssueSeen
            || (!categories.is_empty() && !categories.contains(&AiCategory::Other)))
}

/// The paid model's block wins over a doubt of the free filter; a doubt it
/// settled is dropped (a personal-data hold stays); otherwise the doubt's
/// hold is kept.
fn strongest(doubt: Option<Outcome>, paid: Option<Outcome>, settled: bool) -> Option<Outcome> {
    match paid {
        Some(Outcome::Block(reason)) => Some(Outcome::Block(reason)),
        paid if settled => paid,
        paid => doubt.or(paid),
    }
}

#[cfg(test)]
mod tests;
