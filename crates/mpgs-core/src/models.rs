use crate::recommendation::DemoStatus;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PublicCatalogStatus {
    Empty,
    Ready,
    Updating,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceCapability {
    PublicCatalogRead,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub service_instance_id: String,
    pub service_name: String,
    pub service_version: String,
    pub api_version: String,
    pub public_catalog_status: PublicCatalogStatus,
    pub capabilities: Vec<ServiceCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameCard {
    pub appid: u32,
    pub name: String,
    pub short_description: Option<String>,
    pub section: String,
    pub release_date: Option<String>,
    pub release_date_text: String,
    pub release_state: StoreReleaseState,
    pub demo_status: DemoStatus,
    pub supported_languages: Vec<String>,
    pub is_adult_content: bool,
    pub is_free: bool,
    pub price_text: Option<String>,
    pub discount_percent: Option<u32>,
    pub positive_review_pct: Option<f64>,
    pub total_reviews: Option<u32>,
    pub current_players: Option<u32>,
    pub recommendation_score: f64,
    pub ai_score: Option<f64>,
    pub ai_summary: String,
    pub capsule_url: String,
    pub store_screenshot_urls: Vec<String>,
    pub tags: Vec<String>,
    pub multiplayer_modes: Vec<String>,
    pub review_snippets: Vec<ReviewSnippet>,
    pub user_state: UserGameState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StoreReleaseState {
    Upcoming,
    Released,
    Tba,
    Unknown,
}

impl Default for StoreReleaseState {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserGameState {
    pub favorite: bool,
    pub wishlist: bool,
    pub followed: bool,
    pub viewed: bool,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSnippet {
    pub voted_up: bool,
    pub review: String,
    pub playtime_hours: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiAssessment {
    pub appid: u32,
    pub score: f64,
    pub summary: String,
    pub best_for: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRecommendationMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRecommendationRequest {
    pub prompt: String,
    #[serde(default)]
    pub context_messages: Vec<AiRecommendationMessage>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRecommendedGame {
    pub game: GameCard,
    pub match_score: f64,
    pub reason: String,
    pub matched_traits: Vec<String>,
    pub missing_traits: Vec<String>,
    pub caveats: Vec<String>,
    pub exact_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRecommendationResponse {
    pub reply: String,
    pub follow_up_question: Option<String>,
    pub exact_match_count: usize,
    pub source: AnalysisSource,
    pub llm_used: bool,
    pub diagnostic: Option<String>,
    pub items: Vec<AiRecommendedGame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisSource {
    Hybrid,
    Rule,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisEvidenceKind {
    PositiveReviewPct,
    TotalReviews,
    CurrentPlayers,
    Tags,
    MultiplayerModes,
    ShortDescription,
    ReviewSnippet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisReviewStance {
    Strength,
    Risk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationPool {
    NewRelease,
    Evergreen,
    HiddenGem,
    FriendsParty,
    DemoPotential,
}

impl Default for RecommendationPool {
    fn default() -> Self {
        Self::Evergreen
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisDimensionScore {
    pub key: String,
    pub label: String,
    pub score: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisPoint {
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisEvidenceItem {
    pub kind: AnalysisEvidenceKind,
    pub label: String,
    pub value: String,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisReviewEvidenceItem {
    pub stance: AnalysisReviewStance,
    pub quote: String,
    pub playtime_text: String,
    pub interpretation: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRiskFlag {
    pub key: String,
    pub label: String,
    pub severity: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameAnalysisReport {
    pub appid: u32,
    pub generated_at: String,
    pub source: AnalysisSource,
    pub confidence: AnalysisConfidence,
    #[serde(default = "default_score_version")]
    pub score_version: String,
    #[serde(default)]
    pub quality_score: f64,
    #[serde(default)]
    pub recommendation_score: f64,
    #[serde(default)]
    pub confidence_score: f64,
    #[serde(default)]
    pub pool_type: RecommendationPool,
    #[serde(default)]
    pub risk_flags: Vec<AnalysisRiskFlag>,
    pub overall_score: f64,
    pub overview: String,
    pub dimension_scores: Vec<AnalysisDimensionScore>,
    pub strengths: Vec<AnalysisPoint>,
    pub risks: Vec<AnalysisPoint>,
    pub evidence: Vec<AnalysisEvidenceItem>,
    pub review_evidence: Vec<AnalysisReviewEvidenceItem>,
}

fn default_score_version() -> String {
    "v1_compat".to_string()
}
