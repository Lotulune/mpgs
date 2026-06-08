use crate::models::{GameCard, ReviewSnippet, StoreReleaseState, UserGameState};
use crate::recommendation::{
    bucket_game, compute_recommendation_score, DemoStatus, GameFacts, ReleaseBucket,
};
use serde::{Deserialize, Serialize};

pub const DISCOVERY_TASK_TARGET_ADDED_GAMES_DEFAULT: u32 = 200;
pub const DISCOVERY_TASK_TARGET_ADDED_GAMES_MAX: u32 = 200;
pub const STORE_SEARCH_DISCOVERY_MAX_PAGES_PER_RUN: u32 = 2;

const NEW_DISCOVERY_MIN_TOTAL_REVIEWS: u32 = 50;
const NEW_DISCOVERY_MIN_POSITIVE_REVIEW_PCT: f64 = 40.0;
const NEW_DISCOVERY_BLOCKED_TITLE_KEYWORDS: [&str; 1] = ["传奇"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamAppListPreview {
    pub apps: Vec<SteamAppListItem>,
    #[serde(alias = "last_appid")]
    pub last_appid: Option<u32>,
    #[serde(alias = "have_more_results")]
    pub have_more_results: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteamAppListItem {
    pub appid: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct SteamGameSnapshot {
    pub name: Option<String>,
    pub short_description: Option<String>,
    pub release_date: Option<String>,
    pub release_date_text: Option<String>,
    pub release_state: Option<StoreReleaseState>,
    pub demo_status: DemoStatus,
    pub supported_languages: Option<Vec<String>>,
    pub is_adult_content: Option<bool>,
    pub is_free: Option<bool>,
    pub price_text: Option<String>,
    pub discount_percent: Option<u32>,
    pub positive_review_pct: Option<f64>,
    pub total_reviews: Option<u32>,
    pub current_players: Option<u32>,
    pub capsule_url: Option<String>,
    pub store_screenshot_urls: Vec<String>,
    pub tags: Vec<String>,
    pub multiplayer_modes: Vec<String>,
    pub review_snippets: Vec<ReviewSnippet>,
}

pub fn build_discovered_game_card(
    app: &SteamAppListItem,
    snapshot: SteamGameSnapshot,
    today_iso: &str,
) -> Option<GameCard> {
    if snapshot.multiplayer_modes.is_empty() {
        return None;
    }

    let name = snapshot
        .name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| app.name.clone());
    if NEW_DISCOVERY_BLOCKED_TITLE_KEYWORDS
        .iter()
        .any(|keyword| name.contains(keyword))
    {
        return None;
    }
    let release_date = snapshot.release_date;
    let release_date_text = snapshot
        .release_date_text
        .filter(|date| !date.trim().is_empty())
        .unwrap_or_else(|| "日期未知".to_string());
    let release_state = snapshot.release_state.unwrap_or_default();
    let demo_status = snapshot.demo_status;
    let supported_languages = snapshot.supported_languages.unwrap_or_default();
    let is_adult_content = snapshot.is_adult_content.unwrap_or(false);
    let price_text = snapshot.price_text.filter(|text| !text.trim().is_empty());
    let discount_percent = snapshot.discount_percent;
    let positive_review_pct = snapshot.positive_review_pct;
    let total_reviews = snapshot.total_reviews;
    let current_players = snapshot.current_players;
    let capsule_url = snapshot
        .capsule_url
        .filter(|url| !url.trim().is_empty())
        .unwrap_or_else(|| steam_header_url(app.appid));
    let store_screenshot_urls = snapshot.store_screenshot_urls;
    let tags = snapshot.tags;
    let multiplayer_modes = snapshot.multiplayer_modes;
    let review_snippets = snapshot.review_snippets;

    let facts = GameFacts {
        appid: app.appid,
        name: name.clone(),
        release_date: release_date.clone(),
        positive_review_pct,
        total_reviews,
        current_players,
        multiplayer_modes: multiplayer_modes.clone(),
        demo_status: demo_status.clone(),
        ai_score: None,
    };

    let section = match bucket_game(&facts, today_iso) {
        ReleaseBucket::New => "new",
        ReleaseBucket::Classic => "classic",
        ReleaseBucket::ClassicHidden => "classic_hidden",
    }
    .to_string();
    if section == "new"
        && !passes_new_game_quality_gate(demo_status.clone(), total_reviews, positive_review_pct)
    {
        return None;
    }
    let recommendation_score = compute_recommendation_score(&facts, today_iso);

    Some(GameCard {
        appid: app.appid,
        name,
        short_description: snapshot.short_description,
        section,
        release_date,
        release_date_text,
        release_state,
        demo_status,
        supported_languages,
        is_adult_content,
        is_free: snapshot.is_free.unwrap_or(false),
        price_text,
        discount_percent,
        positive_review_pct,
        total_reviews,
        current_players,
        recommendation_score,
        ai_score: None,
        ai_summary: "由 Steam 自动发现，等待 AI 评估后生成更精确的推荐短评。".to_string(),
        capsule_url,
        store_screenshot_urls,
        tags,
        multiplayer_modes,
        review_snippets,
        user_state: UserGameState::default(),
    })
}

fn passes_new_game_quality_gate(
    demo_status: DemoStatus,
    total_reviews: Option<u32>,
    positive_review_pct: Option<f64>,
) -> bool {
    if matches!(
        demo_status,
        DemoStatus::DemoOnly | DemoStatus::ReleasedWithDemo
    ) {
        return true;
    }

    total_reviews.unwrap_or_default() >= NEW_DISCOVERY_MIN_TOTAL_REVIEWS
        && positive_review_pct.unwrap_or_default() >= NEW_DISCOVERY_MIN_POSITIVE_REVIEW_PCT
}

pub fn next_discovery_cursor(preview: &SteamAppListPreview) -> Option<u32> {
    preview
        .last_appid
        .or_else(|| preview.apps.last().map(|app| app.appid))
}

pub fn store_search_start_for_page(pages_processed: u32, page_size: u32) -> u32 {
    pages_processed.saturating_mul(page_size)
}

pub fn store_search_reached_page_budget(pages_processed: u32) -> bool {
    pages_processed >= STORE_SEARCH_DISCOVERY_MAX_PAGES_PER_RUN
}

pub fn parse_saved_cursor(value: Option<String>) -> Option<u32> {
    value.and_then(|value| value.parse::<u32>().ok())
}

pub fn clamp_discovery_pages(value: Option<u32>) -> u32 {
    value.unwrap_or(2).clamp(1, 5)
}

pub fn clamp_discovery_page_size(value: Option<u32>) -> u32 {
    // Steam Store Search normalizes larger page windows and breaks offset paging.
    value.unwrap_or(100).clamp(1, 100)
}

pub fn clamp_discovery_target_added_games(value: Option<u32>) -> u32 {
    value
        .unwrap_or(DISCOVERY_TASK_TARGET_ADDED_GAMES_DEFAULT)
        .clamp(1, DISCOVERY_TASK_TARGET_ADDED_GAMES_MAX)
}

fn steam_header_url(appid: u32) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/header.jpg")
}
