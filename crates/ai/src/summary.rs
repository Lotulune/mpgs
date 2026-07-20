//! Offline game summary schema and validation (PRD AI-008 / M8.3).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AiError;

pub const SUMMARY_PROMPT_VERSION: &str = "summary-v1";

/// Six-section game summary used by detail pages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GameAiSummary {
    pub who_it_fits: SectionBlock,
    pub how_to_play: SectionBlock,
    pub multiplayer_dependency: SectionBlock,
    pub review_strengths: SectionBlock,
    pub common_issues: SectionBlock,
    pub unknowns: SectionBlock,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SectionBlock {
    pub text: String,
    pub evidence_ids: Vec<String>,
    pub confidence: f64,
}

impl GameAiSummary {
    pub fn validate_evidence(
        &self,
        allowed: &std::collections::HashSet<String>,
    ) -> Result<(), AiError> {
        for (name, block) in self.sections() {
            if !(0.0..=1.0).contains(&block.confidence) {
                return Err(AiError::InvalidOutput(format!(
                    "summary.{name}.confidence out of range"
                )));
            }
            if block.text.chars().count() > 800 {
                return Err(AiError::InvalidOutput(format!(
                    "summary.{name}.text exceeds 800 chars"
                )));
            }
            // Concrete claims require evidence, except the explicit unknowns list.
            if name != "unknowns" && !block.text.trim().is_empty() && block.evidence_ids.is_empty()
            {
                return Err(AiError::InvalidOutput(format!(
                    "summary.{name} requires evidence_ids"
                )));
            }
            for id in &block.evidence_ids {
                if !allowed.contains(id) {
                    return Err(AiError::InvalidOutput(format!(
                        "summary evidence_id '{id}' is not allowed"
                    )));
                }
            }
        }
        Ok(())
    }

    fn sections(&self) -> [(&'static str, &SectionBlock); 6] {
        [
            ("who_it_fits", &self.who_it_fits),
            ("how_to_play", &self.how_to_play),
            ("multiplayer_dependency", &self.multiplayer_dependency),
            ("review_strengths", &self.review_strengths),
            ("common_issues", &self.common_issues),
            ("unknowns", &self.unknowns),
        ]
    }
}

pub fn parse_game_summary(value: &Value) -> Result<GameAiSummary, AiError> {
    serde_json::from_value(value.clone())
        .map_err(|error| AiError::InvalidOutput(format!("summary schema: {error}")))
}

/// Deterministic rule summary when the offline model is unavailable.
pub fn rule_game_summary(
    name: &str,
    party_min: Option<i64>,
    party_max: Option<i64>,
    private_session: Option<bool>,
    self_hosted: Option<bool>,
    evidence_ids: &[String],
) -> GameAiSummary {
    let party = match (party_min, party_max) {
        (Some(a), Some(b)) if a == b => format!("{a} 人"),
        (Some(a), Some(b)) => format!("{a}–{b} 人"),
        (Some(a), None) => format!("约 {a}+ 人"),
        (None, Some(b)) => format!("最多 {b} 人"),
        _ => "人数未知".into(),
    };
    let lobby = match private_session {
        Some(true) => "支持私密局/好友局",
        Some(false) => "私密局支持不明确或偏匹配",
        None => "私密局信息未知",
    };
    let host = match self_hosted {
        Some(true) => "可能支持自建服",
        Some(false) => "更依赖官方服务",
        None => "服务器依赖未知",
    };
    let ev = |ids: &[String]| ids.iter().take(3).cloned().collect::<Vec<_>>();
    GameAiSummary {
        who_it_fits: SectionBlock {
            text: format!("{name} 适合想找 {party} 联机体验的小队。"),
            evidence_ids: ev(evidence_ids),
            confidence: 0.45,
        },
        how_to_play: SectionBlock {
            text: "请以商店描述与联机画像为准，典型局时长可能不完整。".into(),
            evidence_ids: ev(evidence_ids),
            confidence: 0.3,
        },
        multiplayer_dependency: SectionBlock {
            text: format!("{lobby}；{host}。"),
            evidence_ids: ev(evidence_ids),
            confidence: 0.4,
        },
        review_strengths: SectionBlock {
            text: String::new(),
            evidence_ids: vec![],
            confidence: 0.0,
        },
        common_issues: SectionBlock {
            text: String::new(),
            evidence_ids: vec![],
            confidence: 0.0,
        },
        unknowns: SectionBlock {
            text: "评价主题与服务停运状态可能尚未完成富化。".into(),
            evidence_ids: vec![],
            confidence: 0.2,
        },
    }
}

pub fn game_summary_system_prompt() -> &'static str {
    "You are MPGS offline game summarizer. Only use provided evidence. \
     Produce six sections: who_it_fits, how_to_play, multiplayer_dependency, \
     review_strengths, common_issues, unknowns. Every non-empty text needs evidence_ids. \
     Never invent AppIDs, URLs, or uncited service claims. Output JSON only."
}

pub fn game_summary_schema() -> Value {
    let section = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["text", "evidence_ids", "confidence"],
        "properties": {
            "text": { "type": "string" },
            "evidence_ids": { "type": "array", "items": { "type": "string" } },
            "confidence": { "type": "number", "minimum": 0, "maximum": 1 }
        }
    });
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "who_it_fits", "how_to_play", "multiplayer_dependency",
            "review_strengths", "common_issues", "unknowns"
        ],
        "properties": {
            "who_it_fits": section,
            "how_to_play": section,
            "multiplayer_dependency": section,
            "review_strengths": section,
            "common_issues": section,
            "unknowns": section
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn non_empty_section_requires_evidence() {
        let summary = GameAiSummary {
            who_it_fits: SectionBlock {
                text: "great for 4".into(),
                evidence_ids: vec![],
                confidence: 0.5,
            },
            ..GameAiSummary::default()
        };
        let err = summary.validate_evidence(&HashSet::new()).unwrap_err();
        assert!(matches!(err, AiError::InvalidOutput(_)));
    }

    #[test]
    fn rule_summary_emits_party_and_dependency() {
        let s = rule_game_summary(
            "Deep Rock",
            Some(1),
            Some(4),
            Some(true),
            Some(true),
            &["e1".into()],
        );
        assert!(s.who_it_fits.text.contains("1–4"));
        assert!(!s.multiplayer_dependency.evidence_ids.is_empty());
    }
}
