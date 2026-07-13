//! Steam Store Reviews summary spike (`appreviews` query_summary).

use serde::Deserialize;

use crate::error::SourceError;
use crate::hash::parameter_hash;
use crate::proposal::{ReviewSummaryProposal, SourceStability};
use crate::raw::RawResponse;

pub const ADAPTER_VERSION: &str = "reviews-0.1.0";
pub const SOURCE_NAME: &str = "steam_store_appreviews";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSummaryRequest {
    pub app_id: u32,
    pub language: String,
    pub purchase_type: String,
    pub filter_offtopic_activity: bool,
    pub num_per_page: u32,
}

impl ReviewSummaryRequest {
    pub fn summary_only(app_id: u32) -> Self {
        Self {
            app_id,
            language: "all".into(),
            purchase_type: "all".into(),
            filter_offtopic_activity: true,
            num_per_page: 0,
        }
    }

    pub fn parameter_pairs(&self) -> Vec<(&str, String)> {
        vec![
            ("json", "1".into()),
            ("language", self.language.clone()),
            ("purchase_type", self.purchase_type.clone()),
            (
                "filter_offtopic_activity",
                if self.filter_offtopic_activity {
                    "1".into()
                } else {
                    "0".into()
                },
            ),
            ("num_per_page", self.num_per_page.to_string()),
        ]
    }

    pub fn parameter_hash(&self) -> String {
        let owned = self.parameter_pairs();
        let refs: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
        parameter_hash(&refs)
    }

    pub fn path_and_query(&self) -> String {
        let pairs = self.parameter_pairs();
        let query = pairs
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        format!("/appreviews/{}?{query}", self.app_id)
    }
}

#[derive(Debug, Deserialize)]
struct ReviewsEnvelope {
    success: i32,
    #[serde(default)]
    query_summary: Option<QuerySummaryDto>,
}

#[derive(Debug, Deserialize)]
struct QuerySummaryDto {
    #[serde(default)]
    total_positive: u32,
    #[serde(default)]
    total_negative: u32,
    #[serde(default)]
    total_reviews: u32,
    #[serde(default)]
    review_score: Option<u32>,
    #[serde(default)]
    review_score_desc: Option<String>,
}

pub fn parse_review_summary(
    request: &ReviewSummaryRequest,
    raw: &RawResponse,
) -> Result<ReviewSummaryProposal, SourceError> {
    let envelope: ReviewsEnvelope = raw.parse_json()?;
    if envelope.success != 1 {
        return Err(SourceError::Permanent {
            message: format!(
                "appreviews success={} for app {}",
                envelope.success, request.app_id
            ),
        });
    }

    let summary = envelope.query_summary.ok_or_else(|| {
        SourceError::invalid_structure("missing query_summary on successful reviews response")
    })?;

    let positive = summary.total_positive;
    let negative = summary.total_negative;
    // Prefer explicit total_reviews; if Steam omits it but provides parts, derive a floor.
    let total = if summary.total_reviews == 0 && (positive > 0 || negative > 0) {
        positive.saturating_add(negative)
    } else {
        summary.total_reviews
    };

    // Allow small Steam-side inconsistencies, but reject clearly impossible aggregates.
    let parts = positive.saturating_add(negative);
    if parts > total.saturating_add(total / 10 + 1) {
        return Err(SourceError::invalid_structure(format!(
            "review counts inconsistent: +{positive} -{negative} total {total}"
        )));
    }

    Ok(ReviewSummaryProposal {
        app_id: request.app_id,
        total_positive: positive,
        total_negative: negative,
        total_reviews: total,
        review_score: summary.review_score,
        review_score_desc: summary.review_score_desc,
        language_scope: request.language.clone(),
        purchase_type: request.purchase_type.clone(),
        filter_offtopic_activity: request.filter_offtopic_activity,
        parameter_hash: request.parameter_hash(),
        content_hash: raw.content_hash.clone(),
        source: SOURCE_NAME,
        stability: SourceStability::OfficialStable,
        adapter_version: ADAPTER_VERSION,
        offline_players_excluded: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::RawResponse;

    fn fixture(name: &str) -> RawResponse {
        let body = match name {
            "summary" => include_bytes!("../fixtures/reviews_summary.json").to_vec(),
            "fail" => include_bytes!("../fixtures/reviews_fail.json").to_vec(),
            other => panic!("unknown fixture {other}"),
        };
        RawResponse::validate(200, body, Some("application/json".into()), 1024 * 1024).unwrap()
    }

    #[test]
    fn normalizes_positive_negative_total_and_parameter_hash() {
        let request = ReviewSummaryRequest::summary_only(892970);
        let proposal = parse_review_summary(&request, &fixture("summary")).unwrap();
        assert_eq!(proposal.app_id, 892970);
        assert_eq!(proposal.total_positive, 180_000);
        assert_eq!(proposal.total_negative, 12_000);
        assert_eq!(proposal.total_reviews, 192_000);
        assert_eq!(proposal.review_score_desc.as_deref(), Some("Very Positive"));
        assert!(!proposal.parameter_hash.is_empty());
        assert_eq!(
            proposal.parameter_hash,
            ReviewSummaryRequest::summary_only(1).parameter_hash()
        );
        assert!(proposal.offline_players_excluded);
        assert_eq!(proposal.stability, SourceStability::OfficialStable);
    }

    #[test]
    fn parameter_hash_changes_with_language_scope() {
        let mut a = ReviewSummaryRequest::summary_only(10);
        let mut b = ReviewSummaryRequest::summary_only(10);
        b.language = "schinese".into();
        assert_ne!(a.parameter_hash(), b.parameter_hash());
        a.filter_offtopic_activity = false;
        assert_ne!(
            a.parameter_hash(),
            ReviewSummaryRequest::summary_only(10).parameter_hash()
        );
    }

    #[test]
    fn non_success_is_permanent_failure() {
        let request = ReviewSummaryRequest::summary_only(1);
        let err = parse_review_summary(&request, &fixture("fail")).unwrap_err();
        assert!(matches!(err, SourceError::Permanent { .. }));
    }
}
