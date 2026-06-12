use crate::worker::SteamSnapshotSource;
use anyhow::{anyhow, Context, Result};
use mpgs_core::models::{ReviewSnippet, StoreReleaseState};
use mpgs_core::recommendation::DemoStatus;
use mpgs_core::steam_mapping::SteamGameSnapshot;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use time::{format_description::FormatItem, macros::format_description, Date};

const SHORT_STEAM_DATE: &[FormatItem<'_>] =
    format_description!("[month repr:short] [day padding:none], [year]");
const LONG_STEAM_DATE: &[FormatItem<'_>] =
    format_description!("[month repr:long] [day padding:none], [year]");
const ISO_DATE: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");
const APP_DETAILS_FILTERS: &str =
    "basic,price_overview,release_date,categories,genres,demos,content_descriptors,screenshots";
const CURRENT_PLAYERS_BASE_URL: &str = "https://api.steampowered.com";
const APP_DETAILS_MAX_ATTEMPTS: u8 = 3;
const APP_DETAILS_RETRY_BASE_DELAY_MS: u64 = 350;

#[derive(Debug, Clone)]
pub struct SteamHttpSnapshotSource {
    client: Client,
    country: String,
    language: String,
}

impl SteamHttpSnapshotSource {
    pub fn new(client: Client, country: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            client,
            country: country.into(),
            language: language.into(),
        }
    }
}

impl SteamSnapshotSource for SteamHttpSnapshotSource {
    fn fetch_snapshot<'a>(
        &'a self,
        appid: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Option<SteamGameSnapshot>>> + Send + 'a>> {
        Box::pin(async move {
            fetch_game_snapshot(&self.client, appid, &self.country, &self.language).await
        })
    }
}

pub async fn fetch_game_snapshot(
    client: &Client,
    appid: u32,
    country: &str,
    language: &str,
) -> Result<Option<SteamGameSnapshot>> {
    let Some(details) = fetch_app_details(client, appid, country, language)
        .await
        .with_context(|| format!("fetch appdetails for appid {appid}"))?
    else {
        return Ok(None);
    };
    let (positive_review_pct, total_reviews) = if details.has_multiplayer_modes() {
        fetch_reviews(client, appid)
            .await
            .map(|reviews| review_metrics_from_summary(Some(&reviews)))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };
    let current_players = fetch_current_players(client, appid).await.ok();
    let release_date = details
        .release_date
        .as_ref()
        .and_then(|release| parse_steam_date(release.date.as_deref()));
    let release_date_text = details
        .release_date
        .as_ref()
        .and_then(|release| release.date.clone());
    let has_demo = details.has_demo();
    let is_demo = details
        .type_field
        .as_deref()
        .is_some_and(|kind| kind.eq_ignore_ascii_case("demo"));

    Ok(Some(SteamGameSnapshot {
        name: details.name.clone(),
        short_description: details
            .short_description
            .clone()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty()),
        release_date: release_date.clone(),
        release_date_text: release_date_text.clone(),
        release_state: Some(infer_store_release_state(
            details.release_date.as_ref(),
            release_date.as_deref(),
            release_date_text.as_deref(),
        )),
        demo_status: DemoStatus::from_parts(is_demo, has_demo),
        supported_languages: details
            .supported_languages
            .as_deref()
            .map(parse_supported_languages)
            .filter(|languages| !languages.is_empty()),
        is_adult_content: Some(details.is_adult_content()),
        is_free: details.is_free,
        price_text: details.price_text().filter(|text| !text.trim().is_empty()),
        discount_percent: details
            .price_overview
            .as_ref()
            .and_then(|price| price.discount_percent),
        positive_review_pct,
        total_reviews,
        current_players,
        capsule_url: details.header_image.clone(),
        store_screenshot_urls: details.store_screenshot_urls(),
        tags: details.tags(),
        multiplayer_modes: details.multiplayer_modes(),
        review_snippets: Vec::<ReviewSnippet>::new(),
    }))
}

async fn fetch_app_details(
    client: &Client,
    appid: u32,
    country: &str,
    language: &str,
) -> Result<Option<AppDetails>> {
    let query = [
        ("appids", appid.to_string()),
        ("cc", country.to_string()),
        ("l", language.to_string()),
        ("filters", APP_DETAILS_FILTERS.to_string()),
    ];

    for attempt in 1..=APP_DETAILS_MAX_ATTEMPTS {
        let response = client
            .get("https://store.steampowered.com/api/appdetails")
            .query(&query)
            .send()
            .await;

        match response {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    if attempt < APP_DETAILS_MAX_ATTEMPTS && is_retryable_steam_status(status) {
                        tokio::time::sleep(app_details_retry_delay(attempt)).await;
                        continue;
                    }
                    return Err(anyhow!(
                        "Steam appdetails for appid {appid}: Steam returned HTTP {status}"
                    ));
                }

                let response = response
                    .json::<HashMap<String, AppDetailsEnvelope>>()
                    .await
                    .map_err(|error| {
                        anyhow!(
                            "Steam appdetails for appid {appid}: parse response failed ({})",
                            describe_reqwest_error(&error)
                        )
                    })?;

                return Ok(response.get(&appid.to_string()).and_then(|entry| {
                    if entry.success {
                        entry.data.clone()
                    } else {
                        None
                    }
                }));
            }
            Err(error) => {
                if attempt < APP_DETAILS_MAX_ATTEMPTS && is_retryable_steam_transport_error(&error)
                {
                    tokio::time::sleep(app_details_retry_delay(attempt)).await;
                    continue;
                }
                return Err(anyhow!(
                    "Steam appdetails for appid {appid}: request failed ({})",
                    describe_reqwest_error(&error)
                ));
            }
        }
    }

    Err(anyhow!(
        "Steam appdetails for appid {appid}: exhausted retries unexpectedly"
    ))
}

fn is_retryable_steam_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable_steam_transport_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request()
}

fn app_details_retry_delay(attempt: u8) -> Duration {
    let multiplier = 1u64 << u32::from(attempt.saturating_sub(1));
    Duration::from_millis(APP_DETAILS_RETRY_BASE_DELAY_MS.saturating_mul(multiplier))
}

async fn fetch_reviews(client: &Client, appid: u32) -> Result<ReviewFetch> {
    let url = format!("https://store.steampowered.com/appreviews/{appid}");
    let response = client
        .get(url)
        .query(&[
            ("json", "1"),
            ("filter", "all"),
            ("language", "all"),
            ("review_type", "all"),
            ("purchase_type", "all"),
            ("num_per_page", "1"),
            ("cursor", "*"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<AppReviewsResponse>()
        .await?;

    Ok(ReviewFetch {
        total_reviews: response.query_summary.total_reviews,
        total_positive: response.query_summary.total_positive,
        total_negative: response.query_summary.total_negative,
    })
}

fn review_metrics_from_summary(summary: Option<&ReviewFetch>) -> (Option<f64>, Option<u32>) {
    let Some(summary) = summary else {
        return (None, None);
    };

    let derived_total = match (summary.total_positive, summary.total_negative) {
        (Some(positive), Some(negative)) => positive.checked_add(negative),
        _ => None,
    };
    let total_reviews = summary.total_reviews.or(derived_total);
    let positive_review_pct = match (summary.total_positive, total_reviews) {
        (Some(positive), Some(total)) if total > 0 => {
            Some((positive as f64 / total as f64) * 100.0)
        }
        _ => None,
    };

    (positive_review_pct, total_reviews)
}

async fn fetch_current_players(client: &Client, appid: u32) -> Result<u32> {
    fetch_current_players_from_base_url(client, CURRENT_PLAYERS_BASE_URL, appid).await
}

async fn fetch_current_players_from_base_url(
    client: &Client,
    base_url: &str,
    appid: u32,
) -> Result<u32> {
    let url = format!(
        "{}/ISteamUserStats/GetNumberOfCurrentPlayers/v1/",
        base_url.trim_end_matches('/')
    );
    let response = client
        .get(url)
        .query(&[("appid", appid.to_string())])
        .send()
        .await
        .map_err(|error| {
            anyhow!(
                "Steam current players for appid {appid}: request failed ({})",
                describe_reqwest_error(&error)
            )
        })?
        .error_for_status()
        .map_err(|error| {
            anyhow!(
                "Steam current players for appid {appid}: Steam returned {}",
                error
                    .status()
                    .map(|status| format!("HTTP {status}"))
                    .unwrap_or_else(|| describe_reqwest_error(&error))
            )
        })?
        .json::<CurrentPlayersEnvelope>()
        .await
        .map_err(|error| {
            anyhow!(
                "Steam current players for appid {appid}: parse response failed ({})",
                describe_reqwest_error(&error)
            )
        })?;

    Ok(response.response.player_count)
}

fn describe_reqwest_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return "request timed out".to_string();
    }
    if error.is_connect() {
        return "network connection failed".to_string();
    }
    if error.is_decode() {
        return "response JSON parse failed".to_string();
    }

    let message = error.to_string();
    if message.trim().is_empty() {
        "unknown request error".to_string()
    } else {
        message
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AppDetailsEnvelope {
    success: bool,
    data: Option<AppDetails>,
}

#[derive(Debug, Clone, Deserialize)]
struct AppDetails {
    #[serde(rename = "type")]
    type_field: Option<String>,
    name: Option<String>,
    short_description: Option<String>,
    header_image: Option<String>,
    screenshots: Option<Vec<StoreScreenshot>>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    required_age: Option<String>,
    is_free: Option<bool>,
    supported_languages: Option<String>,
    price_overview: Option<PriceOverview>,
    release_date: Option<ReleaseDate>,
    categories: Option<Vec<StoreDescriptor>>,
    genres: Option<Vec<StoreDescriptor>>,
    demos: Option<Vec<DemoEntry>>,
    content_descriptors: Option<ContentDescriptors>,
}

impl AppDetails {
    fn has_demo(&self) -> bool {
        self.demos.as_ref().is_some_and(|demos| !demos.is_empty())
    }

    fn tags(&self) -> Vec<String> {
        self.genres
            .clone()
            .unwrap_or_default()
            .into_iter()
            .chain(self.categories.clone().unwrap_or_default())
            .filter_map(|descriptor| descriptor.description)
            .take(12)
            .collect()
    }

    fn multiplayer_modes(&self) -> Vec<String> {
        self.categories
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|descriptor| descriptor.multiplayer_label())
            .collect()
    }

    fn has_multiplayer_modes(&self) -> bool {
        !self.multiplayer_modes().is_empty()
    }

    fn is_adult_content(&self) -> bool {
        self.required_age
            .as_deref()
            .and_then(|value| value.parse::<u32>().ok())
            .is_some_and(|age| age >= 18)
            || self
                .content_descriptors
                .as_ref()
                .is_some_and(ContentDescriptors::contains_adult_marker)
    }

    fn price_text(&self) -> Option<String> {
        if self.is_free.unwrap_or(false) {
            return Some("Free To Play".to_string());
        }

        self.price_overview
            .as_ref()
            .and_then(|price| price.final_formatted.clone())
    }

    fn store_screenshot_urls(&self) -> Vec<String> {
        self.screenshots
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|screenshot| {
                let thumbnail = screenshot
                    .path_thumbnail
                    .filter(|url| !url.trim().is_empty());
                let full = screenshot.path_full.filter(|url| !url.trim().is_empty());
                thumbnail.or(full)
            })
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty())
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ReleaseDate {
    date: Option<String>,
    coming_soon: Option<bool>,
}

fn infer_store_release_state(
    release: Option<&ReleaseDate>,
    release_date: Option<&str>,
    release_date_text: Option<&str>,
) -> StoreReleaseState {
    if let Some(release) = release {
        if release.coming_soon.unwrap_or(false) {
            return StoreReleaseState::Upcoming;
        }
        if release_date.is_some() {
            return StoreReleaseState::Released;
        }
    }
    let text = release_date_text
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if text.is_empty() {
        return StoreReleaseState::Unknown;
    }
    if text.contains("coming soon") || text.contains("to be announced") || text == "tba" {
        return StoreReleaseState::Tba;
    }

    StoreReleaseState::Released
}

fn parse_steam_date(date: Option<&str>) -> Option<String> {
    let date = date?.trim();
    let lower = date.to_ascii_lowercase();
    if date.is_empty()
        || lower.contains("coming soon")
        || lower.contains("to be announced")
        || lower == "tba"
    {
        return None;
    }

    Date::parse(date, SHORT_STEAM_DATE)
        .or_else(|_| Date::parse(date, LONG_STEAM_DATE))
        .or_else(|_| parse_chinese_steam_date(date))
        .ok()
        .and_then(|date| date.format(ISO_DATE).ok())
}

fn parse_chinese_steam_date(date: &str) -> Result<Date, time::error::Parse> {
    let separators = ['年', '月', '日'];
    let mut normalized = date.trim().to_string();
    for separator in separators {
        normalized = normalized.replace(separator, "-");
    }
    let normalized = normalized
        .split('-')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if normalized.len() != 3 {
        return Err(time::error::Parse::TryFromParsed(
            time::error::TryFromParsed::InsufficientInformation,
        ));
    }
    let normalized = format!(
        "{:0>4}-{:0>2}-{:0>2}",
        normalized[0], normalized[1], normalized[2]
    );
    Date::parse(&normalized, ISO_DATE)
}

fn parse_supported_languages(raw: &str) -> Vec<String> {
    let without_notes = raw
        .replace("<br>", ",")
        .replace("<br/>", ",")
        .replace("<br />", ",");
    without_notes
        .split(',')
        .filter_map(|language| {
            let cleaned = strip_html_tags(language)
                .replace("*", "")
                .trim()
                .to_string();
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        })
        .collect()
}

fn strip_html_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| match value {
        serde_json::Value::String(value) => Some(value),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }))
}

#[derive(Debug, Clone, Deserialize)]
struct StoreDescriptor {
    #[serde(default, deserialize_with = "deserialize_optional_u32")]
    id: Option<u32>,
    description: Option<String>,
}

impl StoreDescriptor {
    fn multiplayer_label(&self) -> Option<String> {
        let description = self
            .description
            .as_deref()
            .map(str::trim)
            .filter(|description| !description.is_empty());

        if self.id.is_some_and(is_multiplayer_category_id) {
            return Some(
                description
                    .map(ToOwned::to_owned)
                    .or_else(|| {
                        self.id
                            .and_then(multiplayer_category_fallback_label)
                            .map(str::to_string)
                    })
                    .expect("known multiplayer category ids should always have fallback labels"),
            );
        }

        description
            .filter(|description| is_multiplayer_category_description(description))
            .map(ToOwned::to_owned)
    }
}

fn deserialize_optional_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| match value {
        serde_json::Value::Number(value) => {
            value.as_u64().and_then(|value| u32::try_from(value).ok())
        }
        serde_json::Value::String(value) => value.parse::<u32>().ok(),
        _ => None,
    }))
}

fn is_multiplayer_category_id(category_id: u32) -> bool {
    matches!(
        category_id,
        1 | 9 | 24 | 27 | 36 | 37 | 38 | 39 | 47 | 48 | 49
    )
}

fn multiplayer_category_fallback_label(category_id: u32) -> Option<&'static str> {
    match category_id {
        1 => Some("Multi-player"),
        9 => Some("Co-op"),
        24 => Some("Shared/Split Screen"),
        27 => Some("Cross-Platform Multiplayer"),
        36 => Some("Online PvP"),
        37 => Some("Shared/Split Screen PvP"),
        38 => Some("Online Co-op"),
        39 => Some("Shared/Split Screen Co-op"),
        47 => Some("LAN PvP"),
        48 => Some("LAN Co-op"),
        49 => Some("PvP"),
        _ => None,
    }
}

fn is_multiplayer_category_description(description: &str) -> bool {
    let lower = description.to_ascii_lowercase();
    lower.contains("multi-player")
        || lower.contains("multiplayer")
        || lower.contains("co-op")
        || lower.contains("lan")
        || lower.contains("shared/split")
        || lower.contains("pvp")
        || description.contains("多人")
        || description.contains("合作")
        || description.contains("局域网")
        || description.contains("分屏")
        || description.contains("对战")
}

#[derive(Debug, Clone, Deserialize)]
struct PriceOverview {
    final_formatted: Option<String>,
    discount_percent: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct StoreScreenshot {
    path_thumbnail: Option<String>,
    path_full: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ContentDescriptors {
    ids: Option<Vec<u32>>,
    notes: Option<String>,
}

impl ContentDescriptors {
    fn contains_adult_marker(&self) -> bool {
        let notes = self
            .notes
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        notes.contains("adult")
            || notes.contains("sexual")
            || notes.contains("nudity")
            || notes.contains("mature")
            || (self.ids.as_ref().is_some_and(|ids| !ids.is_empty())
                && (notes.contains("violence") || notes.contains("gore")))
    }
}

#[derive(Debug, Clone, Deserialize)]
struct DemoEntry {
    #[allow(dead_code)]
    appid: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct AppReviewsResponse {
    query_summary: QuerySummary,
}

#[derive(Debug, Clone, Deserialize)]
struct QuerySummary {
    total_reviews: Option<u32>,
    total_positive: Option<u32>,
    total_negative: Option<u32>,
}

struct ReviewFetch {
    total_reviews: Option<u32>,
    total_positive: Option<u32>,
    total_negative: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct CurrentPlayersEnvelope {
    response: CurrentPlayersResponse,
}

#[derive(Debug, Deserialize)]
struct CurrentPlayersResponse {
    player_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::Query, routing::get, Json, Router};
    use serde_json::json;
    use std::collections::HashMap;
    use tokio::net::TcpListener;

    #[test]
    fn app_details_extract_multiplayer_modes_from_localized_categories() {
        let json = r#"
        {
            "success": true,
            "data": {
                "type": "game",
                "name": "Chinese Multiplayer Test",
                "categories": [
                    { "id": 1, "description": "多人" },
                    { "id": 38, "description": "在线合作" },
                    { "id": 29, "description": "Steam 集换式卡牌" }
                ]
            }
        }
        "#;

        let envelope: AppDetailsEnvelope =
            serde_json::from_str(json).expect("decode localized appdetails envelope");
        let details = envelope.data.expect("details should be present");

        assert_eq!(details.multiplayer_modes(), vec!["多人", "在线合作"]);
    }

    #[test]
    fn review_metrics_use_global_positive_and_negative_totals() {
        let summary = ReviewFetch {
            total_reviews: Some(125),
            total_positive: Some(100),
            total_negative: Some(25),
        };

        let (positive_pct, total_reviews) = review_metrics_from_summary(Some(&summary));

        assert_eq!(positive_pct, Some(80.0));
        assert_eq!(total_reviews, Some(125));
    }

    #[test]
    fn parse_steam_date_accepts_chinese_store_dates() {
        assert_eq!(
            parse_steam_date(Some("2026 年 5 月 5 日")).as_deref(),
            Some("2026-05-05")
        );
    }

    #[tokio::test]
    async fn fetch_current_players_reads_player_count_response() {
        async fn handler(Query(params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
            assert_eq!(params.get("appid").map(String::as_str), Some("548430"));
            Json(json!({
                "response": {
                    "player_count": 12345
                }
            }))
        }

        let app = Router::new().route(
            "/ISteamUserStats/GetNumberOfCurrentPlayers/v1/",
            get(handler),
        );
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("read test server addr");
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("serve current players fixture");
        });

        let players = fetch_current_players_from_base_url(
            &Client::new(),
            &format!("http://{addr}"),
            548430,
        )
        .await
        .expect("fetch current players");

        assert_eq!(players, 12345);
    }
}
