use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: HealthStatus,
}

impl HealthResponse {
    pub fn new(status: HealthStatus) -> Self {
        Self { status }
    }
}
