use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppRecord {
    pub app_id: u32,
    pub app_type: String,
    pub canonical_name: String,
    pub release_state: String,
    pub release_date: Option<String>,
    pub release_date_precision: Option<String>,
    pub is_early_access: Option<bool>,
    pub current_data_confidence: Option<f64>,
    pub source_modified_at_ms: Option<i64>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiplayerProfile {
    pub app_id: u32,
    pub dominant_mode: Option<String>,
    pub private_session: Option<bool>,
    pub online_coop: Option<bool>,
    pub self_hosted_server: Option<bool>,
    pub drop_in_out: Option<bool>,
    pub crossplay: Option<bool>,
    pub recommended_min_players: Option<i64>,
    pub recommended_max_players: Option<i64>,
    pub profile_confidence: Option<f64>,
    pub computed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurationOverride {
    pub override_id: i64,
    pub app_id: u32,
    pub feature_name: String,
    pub value_json: String,
    pub reason: String,
    pub external_evidence: Option<String>,
    pub operator: String,
    pub created_at_ms: i64,
    pub revoked_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateOverrideRequest {
    pub feature_name: String,
    pub value_json: serde_json::Value,
    pub reason: String,
    pub external_evidence: Option<String>,
    pub operator: String,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: i64,
    pub source: String,
    pub task_type: String,
    pub entity_key: String,
    pub priority: i64,
    pub attempts: i64,
    pub max_attempts: i64,
    pub due_at_ms: i64,
    pub status: String,
    pub lease_owner: Option<String>,
    pub lease_expires_at_ms: Option<i64>,
    pub idempotency_key: String,
    pub payload_json: Option<String>,
    pub last_error_category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnqueueJob {
    pub source: String,
    pub task_type: String,
    pub entity_key: String,
    pub priority: i64,
    pub due_at_ms: i64,
    pub idempotency_key: String,
    pub payload_json: Option<String>,
    pub max_attempts: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectiveFeatureValue {
    pub app_id: u32,
    pub feature_name: String,
    pub value_json: serde_json::Value,
    pub origin: FeatureOrigin,
    pub override_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureOrigin {
    HumanOverride,
    SourceEvidence,
    Missing,
}
