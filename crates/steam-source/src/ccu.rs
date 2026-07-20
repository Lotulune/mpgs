//! Steam `ISteamUserStats/GetNumberOfCurrentPlayers` CCU spike.

use serde::Deserialize;

use crate::error::SourceError;
use crate::proposal::{CcuProposal, SourceStability};
use crate::raw::RawResponse;

pub const ADAPTER_VERSION: &str = "ccu-0.1.0";
pub const SOURCE_NAME: &str = "steam_userstats_current_players";

/// Steam result code for success in GetNumberOfCurrentPlayers.
pub const RESULT_OK: i32 = 1;
/// Synthetic local result used when the official endpoint returns HTTP 404.
pub const RESULT_HTTP_NOT_FOUND: i32 = 404;

pub fn http_not_found_proposal(app_id: u32) -> CcuProposal {
    CcuProposal {
        app_id,
        player_count: None,
        result_code: RESULT_HTTP_NOT_FOUND,
        content_hash: format!("http-404:{app_id}"),
        source: SOURCE_NAME,
        stability: SourceStability::OfficialStable,
        adapter_version: ADAPTER_VERSION,
        offline_players_excluded: true,
        missing_reason: Some("endpoint_http_not_found"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CcuRequest {
    pub app_id: u32,
}

impl CcuRequest {
    pub fn new(app_id: u32) -> Self {
        Self { app_id }
    }

    pub fn path_and_query(&self) -> String {
        format!(
            "/ISteamUserStats/GetNumberOfCurrentPlayers/v1/?appid={}",
            self.app_id
        )
    }
}

#[derive(Debug, Deserialize)]
struct CcuEnvelope {
    response: CcuResponseBody,
}

#[derive(Debug, Deserialize)]
struct CcuResponseBody {
    #[serde(default)]
    player_count: Option<u32>,
    #[serde(default)]
    result: i32,
}

pub fn parse_ccu(request: &CcuRequest, raw: &RawResponse) -> Result<CcuProposal, SourceError> {
    let envelope: CcuEnvelope = raw.parse_json()?;
    let body = envelope.response;

    if body.result != RESULT_OK {
        return Ok(CcuProposal {
            app_id: request.app_id,
            player_count: None,
            result_code: body.result,
            content_hash: raw.content_hash.clone(),
            source: SOURCE_NAME,
            stability: SourceStability::OfficialStable,
            adapter_version: ADAPTER_VERSION,
            offline_players_excluded: true,
            missing_reason: Some(missing_reason_for(body.result)),
        });
    }

    let player_count = body
        .player_count
        .ok_or_else(|| SourceError::invalid_structure("result=1 but player_count is missing"))?;

    Ok(CcuProposal {
        app_id: request.app_id,
        player_count: Some(player_count),
        result_code: body.result,
        content_hash: raw.content_hash.clone(),
        source: SOURCE_NAME,
        stability: SourceStability::OfficialStable,
        adapter_version: ADAPTER_VERSION,
        offline_players_excluded: true,
        missing_reason: None,
    })
}

fn missing_reason_for(result: i32) -> &'static str {
    match result {
        42 => "app_not_found_or_no_stats",
        0 => "result_failure",
        _ => "unknown_non_success_result",
    }
}

/// Sampling note for long-tail vs focus candidates (documentation for scheduler).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CcuSampleTier {
    /// Focus candidates: sample about every 30 minutes.
    Focus,
    /// Long-tail candidates: sample every 6–24 hours.
    LongTail,
}

impl CcuSampleTier {
    pub const fn suggested_interval_secs(self) -> u64 {
        match self {
            Self::Focus => 30 * 60,
            Self::LongTail => 6 * 60 * 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::RawResponse;

    fn fixture(name: &str) -> RawResponse {
        let body = match name {
            "ok" => include_bytes!("../fixtures/ccu_ok.json").to_vec(),
            "missing" => include_bytes!("../fixtures/ccu_missing.json").to_vec(),
            other => panic!("unknown fixture {other}"),
        };
        RawResponse::validate(200, body, Some("application/json".into()), 1024 * 1024).unwrap()
    }

    #[test]
    fn parses_player_count_and_documents_offline_exclusion() {
        let request = CcuRequest::new(730);
        let proposal = parse_ccu(&request, &fixture("ok")).unwrap();
        assert_eq!(proposal.player_count, Some(512_345));
        assert_eq!(proposal.result_code, RESULT_OK);
        assert!(proposal.offline_players_excluded);
        assert!(proposal.missing_reason.is_none());
        assert_eq!(proposal.stability, SourceStability::OfficialStable);
    }

    #[test]
    fn missing_app_yields_none_count_not_zero_success() {
        let request = CcuRequest::new(1);
        let proposal = parse_ccu(&request, &fixture("missing")).unwrap();
        assert_eq!(proposal.player_count, None);
        assert_ne!(proposal.result_code, RESULT_OK);
        assert_eq!(proposal.missing_reason, Some("app_not_found_or_no_stats"));
    }

    #[test]
    fn sample_tiers_document_focus_vs_long_tail() {
        assert_eq!(CcuSampleTier::Focus.suggested_interval_secs(), 1800);
        assert_eq!(CcuSampleTier::LongTail.suggested_interval_secs(), 21600);
    }

    #[test]
    fn http_not_found_is_a_missing_snapshot_not_zero_players() {
        let proposal = http_not_found_proposal(42);
        assert_eq!(proposal.player_count, None);
        assert_eq!(proposal.result_code, RESULT_HTTP_NOT_FOUND);
        assert_eq!(proposal.missing_reason, Some("endpoint_http_not_found"));
    }
}
