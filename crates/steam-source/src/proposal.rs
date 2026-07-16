use mpgs_domain::SteamAppId;
use serde::{Deserialize, Serialize};

/// Source confidence labels for normalized proposals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStability {
    /// Documented official Web API / Store Reviews contract.
    OfficialStable,
    /// Works in practice but is not a stable public contract.
    ApprovedVolatile,
    /// Human-maintained or curated only.
    HumanMaintained,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppTypeProposal {
    Game,
    Demo,
    Playtest,
    Dlc,
    Tool,
    Application,
    Music,
    Video,
    Series,
    Comic,
    Advertising,
    Mod,
    Hardware,
    Unknown,
}

impl AppTypeProposal {
    pub fn from_steam_type(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "game" => Self::Game,
            "demo" => Self::Demo,
            "playtest" => Self::Playtest,
            "dlc" => Self::Dlc,
            "tool" | "server" | "dedicated_server" => Self::Tool,
            "application" | "software" => Self::Application,
            "music" => Self::Music,
            "video" => Self::Video,
            "series" => Self::Series,
            "comic" => Self::Comic,
            "advertising" => Self::Advertising,
            "mod" => Self::Mod,
            "hardware" => Self::Hardware,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStateProposal {
    Released,
    Upcoming,
    ComingSoon,
    Retired,
    Unknown,
}

/// Normalized catalog entry proposed by a source adapter.
///
/// Proposals are not authoritative until storage resolution applies source
/// priority and human overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppCatalogProposal {
    pub app_id: SteamAppId,
    pub name: String,
    pub app_type: AppTypeProposal,
    pub last_modified: Option<u32>,
    pub price_change_number: Option<u32>,
    pub source: &'static str,
    pub stability: SourceStability,
    pub adapter_version: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewSummaryProposal {
    pub app_id: SteamAppId,
    pub total_positive: u32,
    pub total_negative: u32,
    pub total_reviews: u32,
    pub review_score: Option<u32>,
    pub review_score_desc: Option<String>,
    pub language_scope: String,
    pub purchase_type: String,
    pub filter_offtopic_activity: bool,
    pub parameter_hash: String,
    pub content_hash: String,
    pub source: &'static str,
    pub stability: SourceStability,
    pub adapter_version: &'static str,
    /// CCU and review counts never include offline / non-Steam-connected players.
    pub offline_players_excluded: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CcuProposal {
    pub app_id: SteamAppId,
    pub player_count: Option<u32>,
    pub result_code: i32,
    pub content_hash: String,
    pub source: &'static str,
    pub stability: SourceStability,
    pub adapter_version: &'static str,
    /// Documented Steam limitation: count is Steam-connected only.
    pub offline_players_excluded: bool,
    pub missing_reason: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationTypeProposal {
    DemoOf,
    PlaytestOf,
    DedicatedServerFor,
    EditionOf,
    Replaces,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppRelationProposal {
    pub source_app_id: SteamAppId,
    pub target_app_id: SteamAppId,
    pub relation_type: RelationTypeProposal,
    pub confidence: f64,
    pub stability: SourceStability,
    pub adapter_version: &'static str,
}

/// Normalized store price snapshot proposed by appdetails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorePriceProposal {
    pub country_code: String,
    pub currency: String,
    pub initial_price_minor: Option<i64>,
    pub final_price_minor: Option<i64>,
    pub discount_percent: Option<i32>,
    pub is_purchasable: Option<bool>,
    pub package_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoreDetailsProposal {
    pub app_id: SteamAppId,
    pub name: Option<String>,
    pub app_type: AppTypeProposal,
    pub release_state: ReleaseStateProposal,
    pub release_date_raw: Option<String>,
    pub release_date: Option<String>,
    pub release_date_precision: Option<String>,
    pub release_date_observed: bool,
    pub is_free: Option<bool>,
    pub platforms: Option<Vec<String>>,
    pub supported_languages: Option<Vec<String>>,
    pub price: Option<StorePriceProposal>,
    pub coming_soon: Option<bool>,
    pub categories: Vec<String>,
    pub genres: Vec<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub short_description: Option<String>,
    pub demo_app_ids: Vec<SteamAppId>,
    pub fullgame_app_id: Option<SteamAppId>,
    pub multiplayer_category_hints: Vec<String>,
    pub content_hash: String,
    pub source: &'static str,
    pub stability: SourceStability,
    pub adapter_version: &'static str,
}
