use mpgs_core::models::PublicCatalogStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicGameListItem {
    pub appid: u32,
    pub name: String,
    pub recommendation_score: Option<f64>,
    pub updated_at: String,
}

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
