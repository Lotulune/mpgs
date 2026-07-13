//! Steam Store `appdetails` volatile adapter spike (calendar / Demo relations).
//!
//! This endpoint is **not** a documented stable Web API contract. It is isolated
//! here so M2 can swap or disable it without touching official adapters.

use serde::Deserialize;
use serde_json::Value;

use crate::error::SourceError;
use crate::proposal::{
    AppRelationProposal, AppTypeProposal, RelationTypeProposal, ReleaseStateProposal,
    SourceStability, StoreDetailsProposal,
};
use crate::raw::RawResponse;

pub const ADAPTER_VERSION: &str = "store-appdetails-0.1.0";
pub const SOURCE_NAME: &str = "steam_store_appdetails";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreDetailsRequest {
    pub app_id: u32,
}

impl StoreDetailsRequest {
    pub fn new(app_id: u32) -> Self {
        Self { app_id }
    }

    pub fn path_and_query(&self) -> String {
        format!("/api/appdetails?appids={}", self.app_id)
    }
}

#[derive(Debug, Deserialize)]
struct AppDetailsNode {
    success: bool,
    #[serde(default)]
    data: Option<AppDetailsData>,
}

#[derive(Debug, Deserialize)]
struct AppDetailsData {
    #[serde(default)]
    #[serde(rename = "type")]
    app_type: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    steam_appid: Option<u32>,
    #[serde(default)]
    is_free: Option<bool>,
    #[serde(default)]
    short_description: Option<String>,
    #[serde(default)]
    developers: Option<Vec<String>>,
    #[serde(default)]
    publishers: Option<Vec<String>>,
    #[serde(default)]
    categories: Option<Vec<CategoryDto>>,
    #[serde(default)]
    genres: Option<Vec<GenreDto>>,
    #[serde(default)]
    release_date: Option<ReleaseDateDto>,
    #[serde(default)]
    demos: Option<Vec<DemoDto>>,
    #[serde(default)]
    fullgame: Option<FullGameDto>,
}

#[derive(Debug, Deserialize)]
struct CategoryDto {
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GenreDto {
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReleaseDateDto {
    #[serde(default)]
    coming_soon: Option<bool>,
    #[serde(default)]
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DemoDto {
    #[serde(default)]
    appid: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct FullGameDto {
    #[serde(default)]
    appid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreDetailsParseResult {
    pub details: StoreDetailsProposal,
    pub relations: Vec<AppRelationProposal>,
}

pub fn parse_store_details(
    request: &StoreDetailsRequest,
    raw: &RawResponse,
) -> Result<StoreDetailsParseResult, SourceError> {
    let root: Value = raw.parse_json()?;
    let key = request.app_id.to_string();
    let node_value = root
        .get(&key)
        .ok_or_else(|| SourceError::invalid_structure(format!("missing top-level key {key}")))?;

    let node: AppDetailsNode =
        serde_json::from_value(node_value.clone()).map_err(SourceError::json_parse)?;

    if !node.success {
        return Err(SourceError::NotFound { entity_key: key });
    }

    let data = node
        .data
        .ok_or_else(|| SourceError::invalid_structure("success=true but data object is missing"))?;

    let app_id = data.steam_appid.unwrap_or(request.app_id);
    let categories: Vec<String> = data
        .categories
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| c.description)
        .collect();
    let genres: Vec<String> = data
        .genres
        .unwrap_or_default()
        .into_iter()
        .filter_map(|g| g.description)
        .collect();

    let multiplayer_category_hints = categories
        .iter()
        .filter(|c| is_multiplayer_hint(c))
        .cloned()
        .collect();

    let coming_soon = data.release_date.as_ref().and_then(|r| r.coming_soon);
    let release_date_raw = data
        .release_date
        .as_ref()
        .and_then(|r| r.date.clone())
        .filter(|d| !d.trim().is_empty());

    let release_state = match coming_soon {
        Some(true) => ReleaseStateProposal::ComingSoon,
        Some(false) => ReleaseStateProposal::Released,
        None => ReleaseStateProposal::Unknown,
    };

    let demo_app_ids: Vec<u32> = data
        .demos
        .unwrap_or_default()
        .into_iter()
        .filter_map(|d| d.appid)
        .filter(|id| *id != 0)
        .collect();

    let fullgame_app_id = data
        .fullgame
        .and_then(|f| f.appid)
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|id| *id != 0);

    let app_type = data
        .app_type
        .as_deref()
        .map(AppTypeProposal::from_steam_type)
        .unwrap_or(AppTypeProposal::Unknown);

    let details = StoreDetailsProposal {
        app_id,
        name: data.name,
        app_type,
        release_state,
        release_date_raw,
        is_free: data.is_free,
        coming_soon,
        categories,
        genres,
        developers: data.developers.unwrap_or_default(),
        publishers: data.publishers.unwrap_or_default(),
        short_description: data.short_description,
        demo_app_ids: demo_app_ids.clone(),
        fullgame_app_id,
        multiplayer_category_hints,
        content_hash: raw.content_hash.clone(),
        source: SOURCE_NAME,
        stability: SourceStability::ApprovedVolatile,
        adapter_version: ADAPTER_VERSION,
    };

    let mut relations = Vec::new();
    for demo_id in demo_app_ids {
        relations.push(AppRelationProposal {
            source_app_id: demo_id,
            target_app_id: app_id,
            relation_type: RelationTypeProposal::DemoOf,
            confidence: 0.7,
            stability: SourceStability::ApprovedVolatile,
            adapter_version: ADAPTER_VERSION,
        });
    }
    if let Some(full_id) = fullgame_app_id {
        let relation_type = match details.app_type {
            AppTypeProposal::Playtest => RelationTypeProposal::PlaytestOf,
            _ => RelationTypeProposal::DemoOf,
        };
        relations.push(AppRelationProposal {
            source_app_id: app_id,
            target_app_id: full_id,
            relation_type,
            confidence: 0.75,
            stability: SourceStability::ApprovedVolatile,
            adapter_version: ADAPTER_VERSION,
        });
    }

    Ok(StoreDetailsParseResult { details, relations })
}

fn is_multiplayer_hint(label: &str) -> bool {
    let lower = label.to_ascii_lowercase();
    lower.contains("multi")
        || lower.contains("co-op")
        || lower.contains("coop")
        || lower.contains("pvp")
        || lower.contains("online")
        || lower.contains("mmo")
        || lower.contains("lan")
}

/// Static feasibility summary used by docs and runtime diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreAdapterFeasibility {
    pub endpoint: &'static str,
    pub stability: SourceStability,
    pub supports_release_calendar: bool,
    pub supports_demo_relation: bool,
    pub requires_web_api_key: bool,
    pub recommended_fallback: &'static str,
}

pub const STORE_APPDETAILS_FEASIBILITY: StoreAdapterFeasibility = StoreAdapterFeasibility {
    endpoint: "https://store.steampowered.com/api/appdetails",
    stability: SourceStability::ApprovedVolatile,
    supports_release_calendar: true,
    supports_demo_relation: true,
    requires_web_api_key: false,
    recommended_fallback: "human curation + release_events table when structure changes",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::RawResponse;

    fn fixture(name: &str) -> RawResponse {
        let body = match name {
            "game" => include_bytes!("../fixtures/store_appdetails_game.json").to_vec(),
            "demo" => include_bytes!("../fixtures/store_appdetails_demo.json").to_vec(),
            "coming_soon" => {
                include_bytes!("../fixtures/store_appdetails_coming_soon.json").to_vec()
            }
            "fail" => include_bytes!("../fixtures/store_appdetails_fail.json").to_vec(),
            other => panic!("unknown fixture {other}"),
        };
        RawResponse::validate(200, body, Some("application/json".into()), 1024 * 1024).unwrap()
    }

    #[test]
    fn parses_released_game_with_demo_relation() {
        let request = StoreDetailsRequest::new(892970);
        let result = parse_store_details(&request, &fixture("game")).unwrap();
        assert_eq!(result.details.app_id, 892970);
        assert_eq!(result.details.release_state, ReleaseStateProposal::Released);
        assert_eq!(result.details.stability, SourceStability::ApprovedVolatile);
        assert!(!result.details.multiplayer_category_hints.is_empty());
        assert!(result.relations.iter().any(|r| {
            matches!(r.relation_type, RelationTypeProposal::DemoOf) && r.target_app_id == 892970
        }));
    }

    #[test]
    fn parses_demo_fullgame_link() {
        let request = StoreDetailsRequest::new(1_888_930);
        let result = parse_store_details(&request, &fixture("demo")).unwrap();
        assert_eq!(result.details.app_type, AppTypeProposal::Demo);
        assert_eq!(result.details.fullgame_app_id, Some(892970));
        assert!(result.relations.iter().any(|r| {
            r.source_app_id == 1_888_930
                && r.target_app_id == 892970
                && matches!(r.relation_type, RelationTypeProposal::DemoOf)
        }));
    }

    #[test]
    fn parses_coming_soon_calendar_fields() {
        let request = StoreDetailsRequest::new(2_500_000);
        let result = parse_store_details(&request, &fixture("coming_soon")).unwrap();
        assert_eq!(
            result.details.release_state,
            ReleaseStateProposal::ComingSoon
        );
        assert_eq!(result.details.coming_soon, Some(true));
        assert!(result.details.release_date_raw.is_some());
    }

    #[test]
    fn unsuccessful_appdetails_is_not_found() {
        let request = StoreDetailsRequest::new(1);
        let err = parse_store_details(&request, &fixture("fail")).unwrap_err();
        assert!(matches!(err, SourceError::NotFound { .. }));
    }

    #[test]
    fn feasibility_marks_store_as_volatile() {
        assert_eq!(
            STORE_APPDETAILS_FEASIBILITY.stability,
            SourceStability::ApprovedVolatile
        );
        const {
            assert!(STORE_APPDETAILS_FEASIBILITY.supports_demo_relation);
            assert!(!STORE_APPDETAILS_FEASIBILITY.requires_web_api_key);
        }
    }
}
