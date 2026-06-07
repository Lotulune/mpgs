pub mod config;
pub mod db;
pub mod health;
pub mod public_catalog;

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
pub use config::{ConfigError, ConfigHealth, ServerConfig};
use health::{HealthResponse, HealthStatus};
use mpgs_core::models::{PublicCatalogStatus, ServiceCapability, ServiceInfo};
use public_catalog::{
    DiscoveryHomeResponse, GamesQuery, PublicGameAnalysis, PublicGameDetail, PublicGamesPage,
    ServiceErrorEnvelope,
};
use sqlx_postgres::PgPool;
use utoipa::OpenApi;

#[derive(Debug, Clone)]
pub struct ServiceInfoConfig {
    pub service_instance_id: String,
    pub service_name: String,
    pub service_version: String,
}

impl ServiceInfoConfig {
    pub fn from_env() -> Self {
        Self {
            service_instance_id: std::env::var("MPGS_SERVICE_INSTANCE_ID")
                .unwrap_or_else(|_| "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string()),
            service_name: std::env::var("MPGS_SERVICE_NAME")
                .unwrap_or_else(|_| "MPGS Public Discovery Service".to_string()),
            service_version: std::env::var("MPGS_SERVICE_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
        }
    }

    pub fn service_info(&self) -> ServiceInfo {
        self.service_info_with_catalog_status(PublicCatalogStatus::Empty)
    }

    pub fn service_info_with_catalog_status(
        &self,
        public_catalog_status: PublicCatalogStatus,
    ) -> ServiceInfo {
        ServiceInfo {
            service_instance_id: self.service_instance_id.clone(),
            service_name: self.service_name.clone(),
            service_version: self.service_version.clone(),
            api_version: "v1".to_string(),
            public_catalog_status,
            capabilities: vec![ServiceCapability::PublicCatalogRead],
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    service_info: ServiceInfo,
    database_health: DatabaseHealth,
    config_health: ConfigHealth,
}

impl AppState {
    pub fn new(service_info: ServiceInfo, database_health: DatabaseHealth) -> Self {
        Self {
            service_info,
            database_health,
            config_health: ConfigHealth::HealthyForTest,
        }
    }

    pub fn new_with_config_health(
        service_info: ServiceInfo,
        database_health: DatabaseHealth,
        config_health: ConfigHealth,
    ) -> Self {
        Self {
            service_info,
            database_health,
            config_health,
        }
    }
}

#[derive(Clone)]
pub enum DatabaseHealth {
    Pool(PgPool),
    #[doc(hidden)]
    HealthyForTest,
    Unavailable,
}

impl DatabaseHealth {
    async fn is_healthy(&self) -> bool {
        match self {
            Self::Pool(pool) => db::migration_health_check(pool).await.unwrap_or(false),
            Self::HealthyForTest => true,
            Self::Unavailable => false,
        }
    }
}

pub fn build_router(config: ServiceInfoConfig) -> Router {
    build_router_with_state(AppState::new(
        config.service_info(),
        DatabaseHealth::HealthyForTest,
    ))
}

pub fn build_router_with_state(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/service-info", get(service_info))
        .route("/api/v1/discovery-home", get(discovery_home))
        .route("/api/v1/games", get(games))
        .route("/api/v1/games/{appid}", get(game_detail))
        .route("/api/v1/games/{appid}/analysis", get(game_analysis))
        .route("/openapi.json", get(openapi_json))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/api/v1/service-info",
    tag = "public",
    responses(
        (status = 200, description = "Public MPGS service identity information", body = ServiceInfo)
    )
)]
async fn service_info(State(state): State<AppState>) -> Json<ServiceInfo> {
    Json(state.service_info)
}

#[utoipa::path(
    get,
    path = "/api/v1/discovery-home",
    tag = "public",
    responses(
        (status = 200, description = "Public catalog discovery home summary", body = DiscoveryHomeResponse),
        (status = 503, description = "Public catalog unavailable", body = ServiceErrorEnvelope)
    )
)]
async fn discovery_home(State(state): State<AppState>) -> Response {
    match state.database_health.discovery_home().await {
        Ok(payload) => Json(payload).into_response(),
        Err(error) => service_error_response(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "public_catalog_unavailable",
            public_catalog_error_message(error),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/games",
    tag = "public",
    params(GamesQuery),
    responses(
        (status = 200, description = "Paginated public games", body = PublicGamesPage),
        (status = 503, description = "Public catalog unavailable", body = ServiceErrorEnvelope)
    )
)]
async fn games(State(state): State<AppState>, Query(query): Query<GamesQuery>) -> Response {
    let (limit, offset) = query.normalized();
    match state.database_health.public_games_page(limit, offset).await {
        Ok(payload) => Json(payload).into_response(),
        Err(error) => service_error_response(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "public_catalog_unavailable",
            public_catalog_error_message(error),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/games/{appid}",
    tag = "public",
    params(("appid" = u32, Path, description = "Steam AppID")),
    responses(
        (status = 200, description = "Public game detail", body = PublicGameDetail),
        (status = 404, description = "Public game not found", body = ServiceErrorEnvelope),
        (status = 503, description = "Public catalog unavailable", body = ServiceErrorEnvelope)
    )
)]
async fn game_detail(State(state): State<AppState>, Path(appid): Path<u32>) -> Response {
    match state.database_health.public_game_detail(appid).await {
        Ok(Some(payload)) => Json(payload).into_response(),
        Ok(None) => service_error_response(
            axum::http::StatusCode::NOT_FOUND,
            "public_game_not_found",
            "公开游戏不存在。",
        ),
        Err(error) => service_error_response(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "public_catalog_unavailable",
            public_catalog_error_message(error),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/games/{appid}/analysis",
    tag = "public",
    params(("appid" = u32, Path, description = "Steam AppID")),
    responses(
        (status = 200, description = "Public game analysis", body = PublicGameAnalysis),
        (status = 404, description = "Public game analysis not found", body = ServiceErrorEnvelope),
        (status = 503, description = "Public catalog unavailable", body = ServiceErrorEnvelope)
    )
)]
async fn game_analysis(State(state): State<AppState>, Path(appid): Path<u32>) -> Response {
    match state.database_health.public_game_analysis(appid).await {
        Ok(Some(payload)) => Json(payload).into_response(),
        Ok(None) => service_error_response(
            axum::http::StatusCode::NOT_FOUND,
            "public_game_analysis_not_found",
            "公开游戏分析不存在。",
        ),
        Err(error) => service_error_response(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "public_catalog_unavailable",
            public_catalog_error_message(error),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/healthz",
    tag = "system",
    responses(
        (status = 200, description = "Public service health check", body = HealthResponse),
        (status = 503, description = "Service dependency unavailable", body = HealthResponse)
    )
)]
async fn healthz(State(state): State<AppState>) -> (axum::http::StatusCode, Json<HealthResponse>) {
    if state.database_health.is_healthy().await && state.config_health.is_healthy() {
        (
            axum::http::StatusCode::OK,
            Json(HealthResponse::new(HealthStatus::Ok)),
        )
    } else {
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse::new(HealthStatus::Unavailable)),
        )
    }
}

impl DatabaseHealth {
    async fn discovery_home(&self) -> Result<DiscoveryHomeResponse, sqlx_core::error::Error> {
        match self {
            Self::Pool(pool) => db::discovery_home(pool).await,
            Self::HealthyForTest => Ok(DiscoveryHomeResponse::empty()),
            Self::Unavailable => Err(sqlx_core::error::Error::PoolClosed),
        }
    }

    async fn public_games_page(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<PublicGamesPage, sqlx_core::error::Error> {
        match self {
            Self::Pool(pool) => db::public_games_page(pool, limit, offset).await,
            Self::HealthyForTest => Ok(PublicGamesPage::empty(limit, offset)),
            Self::Unavailable => Err(sqlx_core::error::Error::PoolClosed),
        }
    }

    async fn public_game_detail(
        &self,
        appid: u32,
    ) -> Result<Option<PublicGameDetail>, sqlx_core::error::Error> {
        match self {
            Self::Pool(pool) => db::public_game_detail(pool, appid).await,
            Self::HealthyForTest => Ok(None),
            Self::Unavailable => Err(sqlx_core::error::Error::PoolClosed),
        }
    }

    async fn public_game_analysis(
        &self,
        appid: u32,
    ) -> Result<Option<PublicGameAnalysis>, sqlx_core::error::Error> {
        match self {
            Self::Pool(pool) => db::public_game_analysis(pool, appid).await,
            Self::HealthyForTest => Ok(None),
            Self::Unavailable => Err(sqlx_core::error::Error::PoolClosed),
        }
    }
}

fn service_error_response(
    status: axum::http::StatusCode,
    code: &'static str,
    message: &'static str,
) -> Response {
    (status, Json(ServiceErrorEnvelope::new(code, message))).into_response()
}

fn public_catalog_error_message(_error: sqlx_core::error::Error) -> &'static str {
    "公共游戏库暂不可用。"
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(build_openapi())
}

#[derive(OpenApi)]
#[openapi(
    paths(
        service_info,
        discovery_home,
        games,
        game_detail,
        game_analysis,
        healthz
    ),
    components(schemas(
        health::HealthResponse,
        health::HealthStatus,
        public_catalog::DiscoveryHomeResponse,
        public_catalog::DiscoveryHomeSections,
        public_catalog::PageMeta,
        public_catalog::PublicGameAnalysis,
        public_catalog::PublicGameDetail,
        public_catalog::PublicGameListItem,
        public_catalog::PublicGamesPage,
        public_catalog::ServiceErrorBody,
        public_catalog::ServiceErrorEnvelope,
        mpgs_core::models::ServiceInfo,
        mpgs_core::models::PublicCatalogStatus,
        mpgs_core::models::ServiceCapability
    ))
)]
struct ApiDoc;

pub fn build_openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
