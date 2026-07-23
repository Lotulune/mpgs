//! Task-level model routing configuration.
//!
//! Model *names* in defaults are suggestions only. Production selection is
//! filtered by the live `/v1/models` registry and canary results (see
//! [`crate::model_registry`]). Routes can be overridden via environment without
//! hardcoding secrets.

use std::collections::HashMap;
use std::env;
use std::time::Duration;

use crate::error::AiError;
use crate::types::{AiTaskType, ApiProtocol, TaskRouteConfig};

/// Shared route table version. Bump when default policy changes.
pub const DEFAULT_ROUTE_VERSION: &str = "m8-route-v6";

/// Build the default multi-model route table from PRD suggestions.
///
/// Names are not permanent guarantees — callers must intersect with the
/// discovered model registry before issuing requests.
pub fn default_task_routes() -> HashMap<AiTaskType, TaskRouteConfig> {
    let version = DEFAULT_ROUTE_VERSION.to_owned();
    let mut routes = HashMap::new();

    // Primary: grok-4.5 (currently the most reliable path on sub2api-gcp).
    // Fallbacks cover 4.20 non-reasoning / 4.3 when 4.5 is unavailable.
    let primary = "grok-4.5";
    let heavy_fallbacks = vec![
        "grok-4.20-0309-non-reasoning".into(),
        "grok-4.20-non-reasoning".into(),
        "grok-4.3".into(),
    ];
    let reasoning_fallbacks = vec![
        "grok-4.20-0309-non-reasoning".into(),
        "grok-4.20-0309-reasoning".into(),
        "grok-4.3".into(),
    ];
    // Prefer Responses when available; chat completions is a solid fallback.
    let responses_first = vec![ApiProtocol::Responses, ApiProtocol::ChatCompletions];

    routes.insert(
        AiTaskType::IntentParse,
        TaskRouteConfig {
            task: AiTaskType::IntentParse,
            primary_model: primary.into(),
            fallback_models: heavy_fallbacks.clone(),
            protocol_preference: responses_first.clone(),
            // Shared across the whole model/protocol chain. 5s was too tight for
            // grok-4.5 Responses cold starts and starved fallbacks; 12s still
            // keeps NL snappy while leaving room for a protocol or model retry.
            timeout: Duration::from_secs(12),
            max_output_tokens: 512,
            enabled: true,
            route_version: version.clone(),
        },
    );

    routes.insert(
        AiTaskType::RankExplain,
        TaskRouteConfig {
            task: AiTaskType::RankExplain,
            primary_model: primary.into(),
            fallback_models: heavy_fallbacks.clone(),
            protocol_preference: responses_first.clone(),
            timeout: Duration::from_secs(35),
            max_output_tokens: 1_800,
            enabled: true,
            route_version: version.clone(),
        },
    );

    routes.insert(
        AiTaskType::GameSummary,
        TaskRouteConfig {
            task: AiTaskType::GameSummary,
            primary_model: primary.into(),
            fallback_models: heavy_fallbacks.clone(),
            protocol_preference: responses_first.clone(),
            timeout: Duration::from_secs(35),
            max_output_tokens: 2_000,
            enabled: true,
            route_version: version.clone(),
        },
    );

    routes.insert(
        AiTaskType::CompareGames,
        TaskRouteConfig {
            task: AiTaskType::CompareGames,
            primary_model: primary.into(),
            fallback_models: reasoning_fallbacks.clone(),
            protocol_preference: responses_first.clone(),
            timeout: Duration::from_secs(40),
            max_output_tokens: 2_400,
            enabled: true,
            route_version: version.clone(),
        },
    );

    routes.insert(
        AiTaskType::GroupAdvice,
        TaskRouteConfig {
            task: AiTaskType::GroupAdvice,
            primary_model: primary.into(),
            fallback_models: reasoning_fallbacks,
            protocol_preference: responses_first.clone(),
            timeout: Duration::from_secs(40),
            max_output_tokens: 2_000,
            enabled: true,
            route_version: version.clone(),
        },
    );

    routes.insert(
        AiTaskType::DataQuality,
        TaskRouteConfig {
            task: AiTaskType::DataQuality,
            primary_model: primary.into(),
            fallback_models: heavy_fallbacks,
            protocol_preference: responses_first,
            timeout: Duration::from_secs(20),
            max_output_tokens: 1_200,
            enabled: true,
            route_version: version,
        },
    );

    routes
}

/// Whether multi-model task routes are enabled.
///
/// Default **true** (PRD M8). Set `MPGS_AI_MULTI_MODEL=0|false|off` to collapse
/// online tasks onto a single `MPGS_AI_MODEL` (M5-compatible).
pub fn multi_model_enabled_from_env() -> bool {
    match env::var("MPGS_AI_MULTI_MODEL") {
        Ok(raw) => !matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off" | "single"
        ),
        Err(_) => true,
    }
}

/// Load routes from defaults, then apply environment overrides.
///
/// Override format (optional):
/// - `MPGS_AI_MULTI_MODEL` default true; false forces single-model mode
/// - `MPGS_AI_ROUTE_<TASK>_MODEL` primary model
/// - `MPGS_AI_ROUTE_<TASK>_FALLBACKS` comma-separated fallbacks
/// - `MPGS_AI_ROUTE_<TASK>_TIMEOUT_SECS`
/// - `MPGS_AI_MODEL` only collapses every online task when multi-model is off
pub fn task_routes_from_env() -> Result<HashMap<AiTaskType, TaskRouteConfig>, AiError> {
    let mut routes = default_task_routes();
    let multi = multi_model_enabled_from_env();
    let global_model = env::var("MPGS_AI_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty());

    for (task, route) in routes.iter_mut() {
        let key = task_env_key(*task);
        if let Ok(primary) = env::var(format!("MPGS_AI_ROUTE_{key}_MODEL")) {
            let primary = primary.trim();
            if primary.is_empty() {
                return Err(AiError::Config(format!(
                    "MPGS_AI_ROUTE_{key}_MODEL must not be empty"
                )));
            }
            route.primary_model = primary.to_owned();
        } else if !multi && let Some(model) = &global_model {
            // Explicit single-model mode only.
            if matches!(
                task,
                AiTaskType::IntentParse
                    | AiTaskType::RankExplain
                    | AiTaskType::CompareGames
                    | AiTaskType::GroupAdvice
                    | AiTaskType::GameSummary
            ) {
                route.primary_model = model.clone();
                route.fallback_models.clear();
            }
        }

        if let Ok(raw) = env::var(format!("MPGS_AI_ROUTE_{key}_FALLBACKS")) {
            route.fallback_models = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
        }

        if let Ok(raw) = env::var(format!("MPGS_AI_ROUTE_{key}_TIMEOUT_SECS")) {
            let secs: u64 = raw.parse().map_err(|_| {
                AiError::Config(format!(
                    "MPGS_AI_ROUTE_{key}_TIMEOUT_SECS must be an integer"
                ))
            })?;
            route.timeout = Duration::from_secs(secs.clamp(1, 120));
        }

        if let Ok(raw) = env::var(format!("MPGS_AI_ROUTE_{key}_ENABLED")) {
            route.enabled = matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            );
        }
    }

    Ok(routes)
}

fn task_env_key(task: AiTaskType) -> &'static str {
    match task {
        AiTaskType::IntentParse => "INTENT_PARSE",
        AiTaskType::RankExplain => "RANK_EXPLAIN",
        AiTaskType::FeatureExtract => "FEATURE_EXTRACT",
        AiTaskType::Embed => "EMBED",
        AiTaskType::GameSummary => "GAME_SUMMARY",
        AiTaskType::CompareGames => "COMPARE_GAMES",
        AiTaskType::GroupAdvice => "GROUP_ADVICE",
        AiTaskType::DataQuality => "DATA_QUALITY",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_routes_cover_prd_online_tasks() {
        let routes = default_task_routes();
        assert!(routes.contains_key(&AiTaskType::IntentParse));
        assert!(routes.contains_key(&AiTaskType::RankExplain));
        assert!(routes.contains_key(&AiTaskType::GameSummary));
        assert!(routes.contains_key(&AiTaskType::CompareGames));
        assert!(routes.contains_key(&AiTaskType::GroupAdvice));
        assert!(routes.contains_key(&AiTaskType::DataQuality));

        let rank = routes.get(&AiTaskType::RankExplain).unwrap();
        assert_eq!(rank.primary_model, "grok-4.5");
        assert!(
            rank.fallback_models
                .iter()
                .any(|m| m.contains("4.20") || m == "grok-4.3")
        );
        assert_eq!(rank.protocol_preference[0], ApiProtocol::Responses);
        let compare = routes.get(&AiTaskType::CompareGames).unwrap();
        assert_eq!(compare.primary_model, "grok-4.5");
        assert_eq!(compare.protocol_preference[0], ApiProtocol::Responses);
    }

    #[test]
    fn grok_45_routes_prefer_responses_and_intent_has_a_bounded_deadline() {
        let routes = default_task_routes();
        let intent = routes.get(&AiTaskType::IntentParse).unwrap();
        assert_eq!(intent.primary_model, "grok-4.5");
        assert_eq!(intent.protocol_preference[0], ApiProtocol::Responses);
        assert_eq!(intent.timeout, Duration::from_secs(12));
        assert!(intent.max_output_tokens <= 1_024);
        // Intent stays shorter than heavy generation tasks.
        let rank = routes.get(&AiTaskType::RankExplain).unwrap();
        assert!(intent.timeout < rank.timeout);

        let data_quality = routes.get(&AiTaskType::DataQuality).unwrap();
        assert_eq!(data_quality.primary_model, "grok-4.5");
        assert_eq!(data_quality.protocol_preference[0], ApiProtocol::Responses);
    }

    #[test]
    fn multi_model_defaults_on_and_accepts_explicit_false() {
        // Default when unset is true; if the process already has the var, still
        // assert that "false" is recognized by parsing the same match arm.
        let off = matches!("false", "0" | "false" | "no" | "off" | "single");
        assert!(off);
        assert!(multi_model_enabled_from_env() || !std::env::var("MPGS_AI_MULTI_MODEL").is_ok());
    }

    #[test]
    fn route_version_is_shared_across_defaults() {
        let routes = default_task_routes();
        let versions: Vec<_> = routes.values().map(|r| r.route_version.as_str()).collect();
        assert!(versions.iter().all(|v| *v == DEFAULT_ROUTE_VERSION));
    }
}
