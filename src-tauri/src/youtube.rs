use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};
use reqwest::Client;
use serde::Deserialize;

const SEARCH_ENDPOINT: &str = "https://www.googleapis.com/youtube/v3/search";
const VIDEOS_ENDPOINT: &str = "https://www.googleapis.com/youtube/v3/videos";
/// Jukebox-friendly upper bound; full singles/MVs usually sit under this.
const MAX_TRACK_DURATION_SECONDS: u64 = 300;
/// Drop Shorts-length edits; most spam clips land under a minute.
const MIN_TRACK_DURATION_SECONDS: u64 = 60;
const RELEVANCE_CANDIDATE_LIMIT: usize = 40;
const RECENT_CANDIDATE_LIMIT: usize = 15;
const RESULT_LIMIT: usize = 15;
const MAX_QUERY_CHARS: usize = 200;
/// Keep the Discord dropdown aligned with youtube.com relevance for the first picks.
const RELEVANCE_PRIMARY: usize = 10;
const MUSIC_CATEGORY_ID: &str = "10";
/// Titles with this many `#` tags are treated as hashtag spam.
const MAX_HASHTAGS_BEFORE_SPAM: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YouTubeTrack {
    pub video_id: String,
    pub title: String,
    pub channel_title: String,
    pub thumbnail: String,
    pub duration_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    items: Vec<SearchItem>,
}

#[derive(Debug, Deserialize)]
struct SearchItem {
    id: SearchId,
    snippet: VideoSnippet,
}

#[derive(Debug, Deserialize)]
struct SearchId {
    #[serde(rename = "videoId")]
    video_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VideoResponse {
    #[serde(default)]
    items: Vec<VideoItem>,
}

#[derive(Debug, Deserialize)]
struct VideoItem {
    id: String,
    #[serde(rename = "contentDetails")]
    content_details: VideoContentDetails,
    #[serde(default)]
    snippet: Option<VideoDetailsSnippet>,
}

#[derive(Debug, Deserialize)]
struct VideoContentDetails {
    duration: String,
}

#[derive(Debug, Deserialize)]
struct VideoDetailsSnippet {
    #[serde(rename = "categoryId")]
    category_id: Option<String>,
}

#[derive(Clone, Debug)]
struct VideoDetails {
    duration_seconds: u64,
    category_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct VideoSnippet {
    title: String,
    #[serde(rename = "channelTitle")]
    channel_title: String,
    thumbnails: ThumbnailSet,
}

#[derive(Clone, Debug, Deserialize)]
struct ThumbnailSet {
    default: Option<Thumbnail>,
    medium: Option<Thumbnail>,
    high: Option<Thumbnail>,
}

#[derive(Clone, Debug, Deserialize)]
struct Thumbnail {
    url: String,
}

pub async fn search(query: &str, api_key: &str) -> Result<Vec<YouTubeTrack>> {
    let query = normalize_query(query)?;
    if api_key.trim().is_empty() {
        bail!("YouTube is not configured.");
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|_| anyhow::anyhow!("Unable to initialize the YouTube client."))?;

    // Relevance matches the website ranking. Date is a best-effort filler only —
    // a quota/API hiccup on one call must not blank the whole Discord search.
    let (relevance_result, recent_result) = tokio::join!(
        search_page(
            &client,
            api_key,
            &query,
            "relevance",
            RELEVANCE_CANDIDATE_LIMIT
        ),
        search_page(&client, api_key, &query, "date", RECENT_CANDIDATE_LIMIT),
    );
    let (relevance, recent) = match (relevance_result, recent_result) {
        (Ok(relevance), Ok(recent)) => (relevance, recent),
        (Ok(relevance), Err(_)) => (relevance, Vec::new()),
        (Err(_), Ok(recent)) => (Vec::new(), recent),
        (Err(relevance_error), Err(_)) => return Err(relevance_error),
    };

    let search_items = merge_search_candidates(relevance, recent);
    if search_items.is_empty() {
        return Ok(Vec::new());
    }

    let details = fetch_video_details(&client, api_key, &search_items).await?;
    Ok(map_search_results(search_items, details))
}

async fn search_page(
    client: &Client,
    api_key: &str,
    query: &str,
    order: &str,
    max_results: usize,
) -> Result<Vec<(String, VideoSnippet)>> {
    let max_results = max_results.clamp(1, 50).to_string();
    let search_response = client
        .get(SEARCH_ENDPOINT)
        .query(&[
            ("part", "snippet"),
            ("type", "video"),
            ("videoEmbeddable", "true"),
            ("maxResults", max_results.as_str()),
            ("order", order),
            ("q", query),
            ("key", api_key),
        ])
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("YouTube search failed."))?;
    let status = search_response.status();
    let body = search_response
        .bytes()
        .await
        .map_err(|_| anyhow::anyhow!("YouTube search returned an invalid response."))?;
    if !status.is_success() {
        bail!(
            "{}",
            youtube_error_message(&body, status.as_u16(), "search")
        );
    }
    let search_response = serde_json::from_slice::<SearchResponse>(&body)
        .map_err(|_| anyhow::anyhow!("YouTube search returned an invalid response."))?;

    Ok(search_response
        .items
        .into_iter()
        .filter_map(|item| item.id.video_id.map(|video_id| (video_id, item.snippet)))
        .collect())
}

/// Relevance leads; recent uploads only fill gaps afterward (never outrank top picks).
fn merge_search_candidates(
    relevance: Vec<(String, VideoSnippet)>,
    recent: Vec<(String, VideoSnippet)>,
) -> Vec<(String, VideoSnippet)> {
    let mut seen = HashSet::new();
    let mut merged = Vec::with_capacity(relevance.len() + recent.len());

    for item in relevance {
        if seen.insert(item.0.clone()) {
            merged.push(item);
        }
    }
    for item in recent {
        if seen.insert(item.0.clone()) {
            merged.push(item);
        }
    }
    merged
}

async fn fetch_video_details(
    client: &Client,
    api_key: &str,
    search_items: &[(String, VideoSnippet)],
) -> Result<HashMap<String, VideoDetails>> {
    let mut details = HashMap::new();
    for chunk in search_items.chunks(50) {
        let video_ids = chunk
            .iter()
            .map(|(video_id, _)| video_id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let videos_response = client
            .get(VIDEOS_ENDPOINT)
            .query(&[
                ("part", "contentDetails,snippet"),
                ("id", video_ids.as_str()),
                ("key", api_key),
            ])
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("YouTube duration lookup failed."))?;
        let status = videos_response.status();
        let body = videos_response.bytes().await.map_err(|_| {
            anyhow::anyhow!("YouTube duration lookup returned an invalid response.")
        })?;
        if !status.is_success() {
            bail!(
                "{}",
                youtube_error_message(&body, status.as_u16(), "duration lookup")
            );
        }
        let videos_response = serde_json::from_slice::<VideoResponse>(&body).map_err(|_| {
            anyhow::anyhow!("YouTube duration lookup returned an invalid response.")
        })?;

        for item in videos_response.items {
            if let Some(duration) = parse_iso8601_duration(&item.content_details.duration) {
                details.insert(
                    item.id,
                    VideoDetails {
                        duration_seconds: duration,
                        category_id: item.snippet.and_then(|snippet| snippet.category_id),
                    },
                );
            }
        }
    }
    Ok(details)
}

fn youtube_error_message(body: &[u8], status: u16, action: &str) -> String {
    #[derive(Deserialize)]
    struct ErrorBody {
        error: Option<ErrorObject>,
    }
    #[derive(Deserialize)]
    struct ErrorObject {
        message: Option<String>,
        #[serde(default)]
        errors: Vec<ErrorItem>,
    }
    #[derive(Deserialize)]
    struct ErrorItem {
        reason: Option<String>,
    }

    if let Ok(parsed) = serde_json::from_slice::<ErrorBody>(body)
        && let Some(error) = parsed.error
    {
        let reason = error
            .errors
            .first()
            .and_then(|item| item.reason.as_deref())
            .unwrap_or("");
        if matches!(reason, "quotaExceeded" | "dailyLimitExceeded") {
            return format!(
                "YouTube {action} quota exceeded. Wait for the daily reset or raise the Google Cloud quota."
            );
        }
        if matches!(
            reason,
            "keyInvalid" | "ipRefererBlocked" | "apiNotActivated"
        ) {
            return format!(
                "YouTube API key rejected ({reason}). Check Music settings and Google Cloud."
            );
        }
        if let Some(message) = error.message {
            let safe = message
                .chars()
                .filter(|character| !character.is_control())
                .take(160)
                .collect::<String>();
            if !safe.is_empty() {
                return format!("YouTube {action} failed ({status}): {safe}");
            }
        }
    }
    format!("YouTube {action} failed with status {status}.")
}

fn normalize_query(query: &str) -> Result<String> {
    let query = query.trim();
    if query.is_empty() || query.chars().any(char::is_control) {
        bail!("The YouTube query is invalid.");
    }
    Ok(query.chars().take(MAX_QUERY_CHARS).collect())
}

fn is_jukebox_duration(duration_seconds: u64) -> bool {
    (MIN_TRACK_DURATION_SECONDS..=MAX_TRACK_DURATION_SECONDS).contains(&duration_seconds)
}

/// Detect Shorts / fyp hashtag spam that relevance alone still returns.
fn is_shorts_spam_title(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    if lower.contains("#shorts")
        || lower.contains("#short")
        || lower.contains("#fyp")
        || lower.contains("#ytshorts")
    {
        return true;
    }
    title.matches('#').count() >= MAX_HASHTAGS_BEFORE_SPAM
}

fn is_music_category(category_id: Option<&str>) -> bool {
    category_id == Some(MUSIC_CATEGORY_ID)
}

fn map_search_results(
    search_items: Vec<(String, VideoSnippet)>,
    details: HashMap<String, VideoDetails>,
) -> Vec<YouTubeTrack> {
    let mut primary = Vec::new();
    let mut music_boost = Vec::new();
    let mut fillers = Vec::new();
    let mut primary_kept = 0_usize;

    for (video_id, snippet) in search_items {
        let Some(video) = details.get(&video_id) else {
            continue;
        };
        if !is_jukebox_duration(video.duration_seconds) {
            continue;
        }
        let title = clean_text(&snippet.title, 200);
        if title.is_empty() || is_shorts_spam_title(&title) {
            continue;
        }
        let channel_title = clean_text(&snippet.channel_title, 100);
        let thumbnail = snippet
            .thumbnails
            .high
            .or(snippet.thumbnails.medium)
            .or(snippet.thumbnails.default)
            .map(|thumbnail| thumbnail.url)
            .filter(|url| url.starts_with("https://"));
        let Some(thumbnail) = thumbnail else {
            continue;
        };
        if channel_title.is_empty() {
            continue;
        }
        let track = YouTubeTrack {
            video_id,
            title,
            channel_title,
            thumbnail,
            duration_seconds: video.duration_seconds,
        };
        // First quality hits keep youtube.com relevance order; later slots may
        // soft-prefer Music category when filling the Discord dropdown.
        if primary_kept < RELEVANCE_PRIMARY {
            primary.push(track);
            primary_kept += 1;
        } else if is_music_category(video.category_id.as_deref()) {
            music_boost.push(track);
        } else {
            fillers.push(track);
        }
    }

    let mut ranked = Vec::with_capacity(RESULT_LIMIT);
    ranked.extend(primary);
    ranked.extend(music_boost);
    ranked.extend(fillers);
    ranked.truncate(RESULT_LIMIT);
    ranked
}

fn clean_text(value: &str, max_chars: usize) -> String {
    decode_basic_html_entities(value)
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

fn decode_basic_html_entities(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        output.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';') else {
            output.push_str(rest);
            return output;
        };
        let entity = &rest[..=end];
        let decoded = match entity {
            "&amp;" => Some("&"),
            "&lt;" => Some("<"),
            "&gt;" => Some(">"),
            "&quot;" => Some("\""),
            "&apos;" | "&#39;" | "&#x27;" | "&#X27;" => Some("'"),
            _ => None,
        };
        if let Some(decoded) = decoded {
            output.push_str(decoded);
        } else if let Some(code) = entity
            .strip_prefix("&#x")
            .or_else(|| entity.strip_prefix("&#X"))
            .and_then(|value| value.strip_suffix(';'))
            .and_then(|value| u32::from_str_radix(value, 16).ok())
            .and_then(char::from_u32)
        {
            output.push(code);
        } else if let Some(code) = entity
            .strip_prefix("&#")
            .and_then(|value| value.strip_suffix(';'))
            .and_then(|value| value.parse::<u32>().ok())
            .and_then(char::from_u32)
        {
            output.push(code);
        } else {
            output.push_str(entity);
        }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    output
}

fn parse_iso8601_duration(value: &str) -> Option<u64> {
    let value = value.strip_prefix("PT")?;
    if value.is_empty() {
        return None;
    }

    let mut number = 0_u64;
    let mut total = 0_u64;
    let mut has_component = false;
    for character in value.chars() {
        if let Some(digit) = character.to_digit(10) {
            number = number.checked_mul(10)?.checked_add(u64::from(digit))?;
            continue;
        }
        let multiplier = match character {
            'H' => 3_600,
            'M' => 60,
            'S' => 1,
            _ => return None,
        };
        total = total.checked_add(number.checked_mul(multiplier)?)?;
        number = 0;
        has_component = true;
    }
    (number == 0 && has_component).then_some(total)
}

#[cfg(test)]
mod tests;
