//! Structured natural-language intent parsing and hard-constraint safety.
//!
//! Rule parsing remains the safety baseline. AI intent may only add soft
//! preferences or high-confidence explicit fields; it must never weaken or
//! invent hard filters for platforms, party size, budget, or service state.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AiError;

pub const INTENT_PROMPT_VERSION: &str = "intent-v1";

/// Allowed hard-constraint field names that can ever become hard filters.
pub const HARD_FIELD_WHITELIST: &[&str] = &[
    "party_size",
    "platforms",
    "budget",
    "session_minutes",
    "demo_required",
    "self_hosting",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionMinutes {
    pub min: Option<u32>,
    pub max: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BudgetIntent {
    pub currency: Option<String>,
    /// Minor units (e.g. CNY fen).
    pub max_each: Option<i64>,
}

/// AI / merged structured intent for search (PRD AI-001).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StructuredIntent {
    pub party_size: Option<u8>,
    pub platforms: Vec<String>,
    pub modes_preferred: Vec<String>,
    pub modes_excluded: Vec<String>,
    pub session_minutes: Option<SessionMinutes>,
    pub budget: Option<BudgetIntent>,
    pub self_hosting: Option<String>,
    pub demo_required: Option<bool>,
    pub free_text_terms: Vec<String>,
    /// Fields that must be enforced as hard filters.
    pub hard_constraints: Vec<String>,
    pub confidence: f64,
}

/// Deterministic rule baseline used as the safety floor.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuleIntentBaseline {
    pub party_size: Option<u8>,
    pub platforms: Vec<String>,
    pub session_minutes_max: Option<u32>,
    pub demo_required: bool,
    pub self_hosting_required: bool,
    /// Soft coop/competitive preference in [0,1]; None = unknown.
    pub coop_competitive: Option<f64>,
}

impl StructuredIntent {
    pub fn is_hard(&self, field: &str) -> bool {
        self.hard_constraints.iter().any(|f| f == field)
    }
}

/// Parse and validate raw AI JSON into a structured intent.
pub fn parse_structured_intent(value: &Value) -> Result<StructuredIntent, AiError> {
    let intent: StructuredIntent = serde_json::from_value(value.clone())
        .map_err(|error| AiError::InvalidOutput(format!("intent schema: {error}")))?;
    validate_intent_shape(&intent)?;
    Ok(intent)
}

fn validate_intent_shape(intent: &StructuredIntent) -> Result<(), AiError> {
    if !(0.0..=1.0).contains(&intent.confidence) {
        return Err(AiError::InvalidOutput(
            "intent.confidence must be in [0,1]".into(),
        ));
    }
    if let Some(size) = intent.party_size
        && !(1..=64).contains(&size)
    {
        return Err(AiError::InvalidOutput(
            "intent.party_size must be 1..=64".into(),
        ));
    }
    for platform in &intent.platforms {
        match platform.as_str() {
            "windows" | "macos" | "linux" | "steamdeck" => {}
            other => {
                return Err(AiError::InvalidOutput(format!(
                    "unsupported platform '{other}'"
                )));
            }
        }
    }
    for field in &intent.hard_constraints {
        if !HARD_FIELD_WHITELIST.contains(&field.as_str()) {
            return Err(AiError::InvalidOutput(format!(
                "hard_constraints field '{field}' is not allowed"
            )));
        }
    }
    if let Some(self_hosting) = &intent.self_hosting {
        match self_hosting.as_str() {
            "required" | "optional" | "excluded" => {}
            other => {
                return Err(AiError::InvalidOutput(format!(
                    "unsupported self_hosting '{other}'"
                )));
            }
        }
    }
    if intent.free_text_terms.len() > 12 {
        return Err(AiError::InvalidOutput(
            "free_text_terms exceeds 12 entries".into(),
        ));
    }
    for term in &intent.free_text_terms {
        if term.chars().count() > 80 {
            return Err(AiError::InvalidOutput(
                "free_text_terms entry exceeds 80 chars".into(),
            ));
        }
    }
    Ok(())
}

/// Merge AI intent with the rule baseline so hard filters cannot be weakened.
///
/// Rules:
/// 1. Rule-derived hard facts always win when present.
/// 2. AI hard_constraints with confidence < 0.75 are demoted to soft.
/// 3. AI cannot invent hard party_size/platforms/budget when rules are empty
///    unless confidence >= 0.85 and the field is in hard_constraints.
/// 4. Soft modes and free-text terms may be unioned.
pub fn merge_intent_with_rules(
    ai: StructuredIntent,
    rules: &RuleIntentBaseline,
) -> StructuredIntent {
    let mut merged = StructuredIntent {
        party_size: rules.party_size.or(ai.party_size),
        platforms: if rules.platforms.is_empty() {
            ai.platforms.clone()
        } else {
            rules.platforms.clone()
        },
        modes_preferred: ai.modes_preferred,
        modes_excluded: ai.modes_excluded,
        session_minutes: match (rules.session_minutes_max, ai.session_minutes) {
            (Some(max), Some(mut session)) => {
                session.max = Some(max);
                Some(session)
            }
            (Some(max), None) => Some(SessionMinutes {
                min: None,
                max: Some(max),
            }),
            (None, session) => session,
        },
        budget: ai.budget,
        self_hosting: if rules.self_hosting_required {
            Some("required".into())
        } else {
            ai.self_hosting
        },
        demo_required: if rules.demo_required {
            Some(true)
        } else {
            ai.demo_required
        },
        free_text_terms: ai.free_text_terms,
        hard_constraints: Vec::new(),
        confidence: ai.confidence.clamp(0.0, 1.0),
    };

    // Soft modes from coop/competitive rule signal.
    if let Some(coop) = rules.coop_competitive {
        if coop < 0.45 {
            push_unique(&mut merged.modes_preferred, "private_coop".into());
            push_unique(&mut merged.modes_excluded, "matchmaking_competitive".into());
        } else if coop > 0.65 {
            push_unique(&mut merged.modes_preferred, "competitive".into());
        }
    }

    let high_confidence = ai.confidence >= 0.75;
    let very_high = ai.confidence >= 0.85;

    // Rule hard fields.
    if rules.party_size.is_some() {
        push_unique(&mut merged.hard_constraints, "party_size".into());
    }
    if !rules.platforms.is_empty() {
        push_unique(&mut merged.hard_constraints, "platforms".into());
    }
    if rules.session_minutes_max.is_some() {
        push_unique(&mut merged.hard_constraints, "session_minutes".into());
    }
    if rules.demo_required {
        push_unique(&mut merged.hard_constraints, "demo_required".into());
    }
    if rules.self_hosting_required {
        push_unique(&mut merged.hard_constraints, "self_hosting".into());
    }

    // AI hard fields only when confident and not inventing against empty rules
    // for the most sensitive dimensions without very high confidence.
    for field in &ai.hard_constraints {
        if !HARD_FIELD_WHITELIST.contains(&field.as_str()) {
            continue;
        }
        if !high_confidence {
            continue;
        }
        match field.as_str() {
            "party_size" if rules.party_size.is_none() && !very_high => continue,
            "platforms" if rules.platforms.is_empty() && !very_high => continue,
            "budget" if !very_high => continue,
            _ => {}
        }
        // Ensure the value is actually present before promoting to hard.
        let present = match field.as_str() {
            "party_size" => merged.party_size.is_some(),
            "platforms" => !merged.platforms.is_empty(),
            "session_minutes" => merged
                .session_minutes
                .as_ref()
                .is_some_and(|s| s.min.is_some() || s.max.is_some()),
            "budget" => merged.budget.as_ref().is_some_and(|b| b.max_each.is_some()),
            "demo_required" => merged.demo_required.is_some(),
            "self_hosting" => merged.self_hosting.is_some(),
            _ => false,
        };
        if present {
            push_unique(&mut merged.hard_constraints, field.clone());
        }
    }

    // Rule facts always override AI values for hard fields.
    if let Some(size) = rules.party_size {
        merged.party_size = Some(size);
    }
    if !rules.platforms.is_empty() {
        merged.platforms = rules.platforms.clone();
    }
    if rules.self_hosting_required {
        merged.self_hosting = Some("required".into());
    }
    if rules.demo_required {
        merged.demo_required = Some(true);
    }

    merged
}

fn push_unique(list: &mut Vec<String>, value: String) {
    if !list.iter().any(|existing| existing == &value) {
        list.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_unknown_hard_constraint_fields() {
        let value = json!({
            "party_size": 3,
            "platforms": ["windows"],
            "modes_preferred": [],
            "modes_excluded": [],
            "free_text_terms": [],
            "hard_constraints": ["drop_table"],
            "confidence": 0.9
        });
        let err = parse_structured_intent(&value).unwrap_err();
        assert!(matches!(err, AiError::InvalidOutput(_)));
    }

    #[test]
    fn low_confidence_ai_hard_fields_are_demoted() {
        let ai = StructuredIntent {
            party_size: Some(8),
            platforms: vec!["linux".into()],
            hard_constraints: vec!["party_size".into(), "platforms".into()],
            confidence: 0.4,
            ..StructuredIntent::default()
        };
        let rules = RuleIntentBaseline {
            party_size: Some(3),
            platforms: vec!["windows".into()],
            ..RuleIntentBaseline::default()
        };
        let merged = merge_intent_with_rules(ai, &rules);
        // Rule hard facts win.
        assert_eq!(merged.party_size, Some(3));
        assert_eq!(merged.platforms, vec!["windows".to_owned()]);
        assert!(merged.is_hard("party_size"));
        assert!(merged.is_hard("platforms"));
    }

    #[test]
    fn ai_cannot_weaken_rule_hard_party_size() {
        let ai = StructuredIntent {
            party_size: Some(2),
            hard_constraints: vec!["party_size".into()],
            confidence: 0.99,
            ..StructuredIntent::default()
        };
        let rules = RuleIntentBaseline {
            party_size: Some(4),
            ..RuleIntentBaseline::default()
        };
        let merged = merge_intent_with_rules(ai, &rules);
        assert_eq!(merged.party_size, Some(4));
        assert!(merged.is_hard("party_size"));
    }

    #[test]
    fn high_confidence_explicit_budget_can_become_hard() {
        let ai = StructuredIntent {
            budget: Some(BudgetIntent {
                currency: Some("CNY".into()),
                max_each: Some(10_000),
            }),
            hard_constraints: vec!["budget".into()],
            confidence: 0.9,
            free_text_terms: vec!["预算一百".into()],
            ..StructuredIntent::default()
        };
        let merged = merge_intent_with_rules(ai, &RuleIntentBaseline::default());
        assert!(merged.is_hard("budget"));
        assert_eq!(merged.budget.unwrap().max_each, Some(10_000));
    }

    #[test]
    fn inventing_party_size_without_rules_requires_very_high_confidence() {
        let low = StructuredIntent {
            party_size: Some(5),
            hard_constraints: vec!["party_size".into()],
            confidence: 0.8,
            ..StructuredIntent::default()
        };
        let merged_low = merge_intent_with_rules(low, &RuleIntentBaseline::default());
        assert!(!merged_low.is_hard("party_size"));

        let high = StructuredIntent {
            party_size: Some(5),
            hard_constraints: vec!["party_size".into()],
            confidence: 0.9,
            ..StructuredIntent::default()
        };
        let merged_high = merge_intent_with_rules(high, &RuleIntentBaseline::default());
        assert!(merged_high.is_hard("party_size"));
        assert_eq!(merged_high.party_size, Some(5));
    }

    #[test]
    fn accepts_valid_intent_payload() {
        let value = json!({
            "party_size": 3,
            "platforms": ["windows"],
            "modes_preferred": ["private_coop"],
            "modes_excluded": ["matchmaking_competitive"],
            "session_minutes": { "min": 30, "max": 90 },
            "budget": { "currency": "CNY", "max_each": 10000 },
            "self_hosting": "optional",
            "demo_required": false,
            "free_text_terms": ["不要太卷"],
            "hard_constraints": ["party_size", "platforms"],
            "confidence": 0.88
        });
        let intent = parse_structured_intent(&value).unwrap();
        assert_eq!(intent.party_size, Some(3));
        assert!(intent.is_hard("party_size"));
    }
}
