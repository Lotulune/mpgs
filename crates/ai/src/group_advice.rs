//! Aggregate group advice without member privacy fields (PRD AI-010).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AiError;

pub const GROUP_ADVICE_PROMPT_VERSION: &str = "group-advice-v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupAdviceRequest {
    /// Aggregated party size the group can field.
    pub party_size: Option<u8>,
    pub platforms: Vec<String>,
    pub modes_preferred: Vec<String>,
    pub modes_excluded: Vec<String>,
    /// Candidate AppIDs only — no member ids or raw feedback.
    pub candidate_app_ids: Vec<u32>,
    /// Optional vote tallies keyed by app_id (public counts only).
    pub vote_counts: Vec<AppVoteCount>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppVoteCount {
    pub app_id: u32,
    pub votes: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupAdviceResult {
    pub primary_app_id: u32,
    pub alternatives: Vec<u32>,
    pub compromise_reason: String,
    pub conflicts: Vec<String>,
    pub evidence_ids: Vec<String>,
}

pub fn parse_group_advice(
    value: &Value,
    allowed_app_ids: &[u32],
    allowed_evidence: &std::collections::HashSet<String>,
) -> Result<GroupAdviceResult, AiError> {
    let result: GroupAdviceResult = serde_json::from_value(value.clone())
        .map_err(|error| AiError::InvalidOutput(format!("group advice schema: {error}")))?;

    if !allowed_app_ids.contains(&result.primary_app_id) {
        return Err(AiError::InvalidOutput(format!(
            "primary_app_id {} outside candidates",
            result.primary_app_id
        )));
    }
    for app_id in &result.alternatives {
        if !allowed_app_ids.contains(app_id) {
            return Err(AiError::InvalidOutput(format!(
                "alternative app_id {app_id} outside candidates"
            )));
        }
    }
    if result.alternatives.len() > 4 {
        return Err(AiError::InvalidOutput(
            "alternatives exceeds 4 entries".into(),
        ));
    }
    if result.compromise_reason.chars().count() > 500 {
        return Err(AiError::InvalidOutput(
            "compromise_reason exceeds 500 chars".into(),
        ));
    }
    if result.conflicts.len() > 8 {
        return Err(AiError::InvalidOutput(
            "conflicts exceeds 8 entries".into(),
        ));
    }
    for id in &result.evidence_ids {
        if !allowed_evidence.contains(id) {
            return Err(AiError::InvalidOutput(format!(
                "group advice evidence_id '{id}' is not allowed"
            )));
        }
    }
    if (!result.compromise_reason.is_empty() || !result.conflicts.is_empty())
        && result.evidence_ids.is_empty()
    {
        return Err(AiError::InvalidOutput(
            "group advice claims require evidence_ids".into(),
        ));
    }
    Ok(result)
}

/// Deterministic compromise: highest votes, then stable app_id order.
pub fn deterministic_group_advice(
    candidates: &[u32],
    votes: &[AppVoteCount],
) -> Option<GroupAdviceResult> {
    if candidates.is_empty() {
        return None;
    }
    let mut scored: Vec<(u32, u32)> = candidates
        .iter()
        .map(|app_id| {
            let v = votes
                .iter()
                .find(|row| row.app_id == *app_id)
                .map(|row| row.votes)
                .unwrap_or(0);
            (*app_id, v)
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let primary = scored[0].0;
    let alternatives: Vec<u32> = scored.iter().skip(1).take(2).map(|(id, _)| *id).collect();
    Some(GroupAdviceResult {
        primary_app_id: primary,
        alternatives,
        compromise_reason: "按聚合想玩票数与稳定排序给出确定性折中。".into(),
        conflicts: vec![],
        evidence_ids: vec![format!("vote:aggregate:{primary}")],
    })
}

pub fn group_advice_system_prompt() -> &'static str {
    "You are MPGS group advisor. Use only aggregate preferences, vote counts, and \
     candidate facts. Never mention individual members, raw feedback, or private data. \
     Output primary, alternatives, compromise_reason, conflicts, and evidence_ids. JSON only."
}

pub fn group_advice_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["primary_app_id", "alternatives", "compromise_reason", "conflicts", "evidence_ids"],
        "properties": {
            "primary_app_id": { "type": "integer", "minimum": 1 },
            "alternatives": { "type": "array", "items": { "type": "integer", "minimum": 1 } },
            "compromise_reason": { "type": "string" },
            "conflicts": { "type": "array", "items": { "type": "string" } },
            "evidence_ids": { "type": "array", "items": { "type": "string" } }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    #[test]
    fn rejects_out_of_candidate_primary() {
        let value = json!({
            "primary_app_id": 9,
            "alternatives": [],
            "compromise_reason": "x",
            "conflicts": [],
            "evidence_ids": ["e1"]
        });
        let mut evidence = HashSet::new();
        evidence.insert("e1".into());
        assert!(parse_group_advice(&value, &[1, 2], &evidence).is_err());
    }

    #[test]
    fn deterministic_picks_highest_votes() {
        let advice = deterministic_group_advice(
            &[10, 20, 30],
            &[
                AppVoteCount {
                    app_id: 20,
                    votes: 5,
                },
                AppVoteCount {
                    app_id: 10,
                    votes: 2,
                },
            ],
        )
        .unwrap();
        assert_eq!(advice.primary_app_id, 20);
        assert_eq!(advice.alternatives, vec![10, 30]);
    }
}
