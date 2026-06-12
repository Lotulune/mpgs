use mpgs_core::models::PublicCatalogStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicGameListItem {
    pub appid: u32,
    pub name: String,
    pub short_description: Option<String>,
    pub section: String,
    pub release_date: Option<String>,
    pub release_date_text: String,
    pub release_state: String,
    pub demo_status: String,
    pub supported_languages: Vec<String>,
    pub is_adult_content: bool,
    pub is_free: bool,
    pub price_text: Option<String>,
    pub discount_percent: Option<u32>,
    pub positive_review_pct: Option<f64>,
    pub total_reviews: Option<u32>,
    pub current_players: Option<u32>,
    pub recommendation_score: Option<f64>,
    pub capsule_url: String,
    pub store_screenshot_urls: Vec<String>,
    pub tags: Vec<String>,
    pub multiplayer_modes: Vec<String>,
    pub review_snippets: Vec<PublicReviewSnippet>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicReviewSnippet {
    pub voted_up: bool,
    pub review: String,
    pub playtime_hours: Option<f64>,
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminReviewCandidate {
    pub appid: u32,
    pub name: String,
    pub review_status: String,
    pub visibility: String,
    pub recommendation_score: Option<f64>,
    pub updated_at: String,
    pub review_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminReviewQueueResponse {
    pub items: Vec<AdminReviewCandidate>,
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminReviewActionResponse {
    pub game: AdminReviewCandidate,
}

#[doc(hidden)]
pub type AdminReviewFixture = AdminReviewCandidate;

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryHomeResponse {
    pub status: PublicCatalogStatus,
    pub total_games: i64,
    pub sections: DiscoveryHomeSections,
}

#[derive(Debug, Clone, Default, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryHomeSections {
    pub newly_published: Vec<PublicGameListItem>,
    pub high_confidence: Vec<PublicGameListItem>,
    pub recently_added: Vec<PublicGameListItem>,
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicGamesPage {
    pub items: Vec<PublicGameListItem>,
    pub page: PageMeta,
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PageMeta {
    pub limit: u32,
    pub offset: u32,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicGameDetail {
    pub game: PublicGameListItem,
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicGameAnalysis {
    pub appid: u32,
    pub report: serde_json::Value,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct ServiceErrorEnvelope {
    pub error: ServiceErrorBody,
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceErrorBody {
    pub code: String,
    pub message: String,
    pub request_id: String,
    pub details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct GamesQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl GamesQuery {
    pub fn normalized(&self) -> (u32, u32) {
        let limit = self.limit.unwrap_or(24).clamp(1, 100);
        let offset = self.offset.unwrap_or(0);
        (limit, offset)
    }
}

impl DiscoveryHomeResponse {
    pub fn empty() -> Self {
        Self {
            status: PublicCatalogStatus::Empty,
            total_games: 0,
            sections: DiscoveryHomeSections::default(),
        }
    }
}

impl PublicGamesPage {
    pub fn empty(limit: u32, offset: u32) -> Self {
        Self {
            items: Vec::new(),
            page: PageMeta {
                limit,
                offset,
                total: 0,
            },
        }
    }
}

impl ServiceErrorEnvelope {
    pub fn new(code: &'static str, message: &'static str) -> Self {
        Self {
            error: ServiceErrorBody {
                code: code.to_string(),
                message: message.to_string(),
                request_id: uuid::Uuid::now_v7().to_string(),
                details: BTreeMap::new(),
            },
        }
    }
}
