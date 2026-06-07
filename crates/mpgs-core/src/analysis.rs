use crate::models::{
    AiAssessment, AnalysisDimensionScore, AnalysisEvidenceItem, AnalysisEvidenceKind,
    AnalysisPoint, AnalysisReviewEvidenceItem, AnalysisReviewStance, AnalysisSource,
    GameAnalysisReport, GameCard,
};
use crate::scoring::score_game_v2;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const ANALYSIS_DIMENSION_KEYS: [&str; 6] = [
    "review_quality",
    "multiplayer_fit",
    "activity_health",
    "content_depth",
    "accessibility",
    "discovery_value",
];

const MAX_NARRATIVE_POINTS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisNarrative {
    pub overview: String,
    pub strengths: Vec<AnalysisPoint>,
    pub risks: Vec<AnalysisPoint>,
    pub dimension_reasons: Vec<(String, String)>,
}

type DimensionReasonPatch = (String, String);

struct SanitizedNarrativePatch {
    overview: Option<String>,
    strengths: Vec<AnalysisPoint>,
    risks: Vec<AnalysisPoint>,
    dimension_reasons: Vec<DimensionReasonPatch>,
}

pub fn build_rule_report(game: &GameCard, generated_at: String) -> Result<GameAnalysisReport> {
    let has_core_signal = !game.tags.is_empty()
        || !game.multiplayer_modes.is_empty()
        || game.positive_review_pct.is_some()
        || game.total_reviews.is_some()
        || !game.review_snippets.is_empty();
    if !has_core_signal {
        return Err(anyhow!("数据不足，暂时无法分析"));
    }

    let scoring = score_game_v2(game);
    let evidence = build_evidence(game);
    let review_evidence = build_review_evidence(game);
    let strengths = build_strengths(game, &scoring.dimension_scores);
    let risks = build_risks(game, &scoring.dimension_scores, &scoring.risk_flags);

    Ok(GameAnalysisReport {
        appid: game.appid,
        generated_at,
        source: AnalysisSource::Rule,
        confidence: scoring.confidence,
        score_version: "v2".to_string(),
        quality_score: scoring.quality_score,
        recommendation_score: scoring.recommendation_score,
        confidence_score: scoring.confidence_score,
        pool_type: scoring.pool_type,
        risk_flags: scoring.risk_flags,
        overall_score: scoring.recommendation_score,
        overview: build_overview(
            game,
            scoring.recommendation_score,
            scoring.quality_score,
            scoring
                .dimension_scores
                .iter()
                .find(|dimension| dimension.key == "multiplayer_fit")
                .map(|dimension| dimension.score)
                .unwrap_or_default(),
        ),
        dimension_scores: scoring.dimension_scores,
        strengths,
        risks,
        evidence,
        review_evidence,
    })
}

pub fn apply_narrative_patch(
    report: GameAnalysisReport,
    narrative: AnalysisNarrative,
) -> GameAnalysisReport {
    let sanitized = sanitize_narrative_patch(narrative);
    if !sanitized.has_useful_content() {
        return report;
    }

    let mut patched = report.clone();
    let mut changed = false;

    if let Some(overview) = sanitized
        .overview
        .filter(|overview| *overview != patched.overview)
    {
        patched.overview = overview;
        changed = true;
    }
    if !sanitized.strengths.is_empty()
        && !point_lists_equal(&sanitized.strengths, &patched.strengths)
    {
        patched.strengths = sanitized.strengths;
        changed = true;
    }
    if !sanitized.risks.is_empty() && !point_lists_equal(&sanitized.risks, &patched.risks) {
        patched.risks = sanitized.risks;
        changed = true;
    }
    for (key, reason) in sanitized.dimension_reasons {
        if let Some(dimension) = patched
            .dimension_scores
            .iter_mut()
            .find(|item| item.key == key && item.reason != reason)
        {
            dimension.reason = reason;
            changed = true;
        }
    }

    if changed {
        patched.source = AnalysisSource::Hybrid;
        patched
    } else {
        report
    }
}

pub fn summarize_report_as_assessment(report: &GameAnalysisReport) -> AiAssessment {
    let best_for = report
        .strengths
        .iter()
        .map(|item| item.title.clone())
        .filter(|item| !item.trim().is_empty())
        .take(3)
        .collect::<Vec<_>>();
    let risks = report
        .risks
        .iter()
        .map(|item| item.title.clone())
        .filter(|item| !item.trim().is_empty())
        .take(3)
        .collect::<Vec<_>>();

    AiAssessment {
        appid: report.appid,
        score: if report.recommendation_score > 0.0 {
            report.recommendation_score
        } else {
            report.overall_score
        },
        summary: report.overview.clone(),
        best_for,
        risks,
    }
}

fn build_overview(
    game: &GameCard,
    recommendation_score: f64,
    quality_score: f64,
    multiplayer_fit: f64,
) -> String {
    let review_phrase = match game.positive_review_pct.unwrap_or(0.0) {
        pct if pct >= 90.0 => "口碑基础扎实",
        pct if pct >= 80.0 => "口碑总体健康",
        pct if pct > 0.0 => "口碑存在分歧",
        _ => "口碑样本有限",
    };
    let multiplayer_phrase = if game.multiplayer_modes.is_empty() {
        "多人模式标签缺失，联机结论只能低置信度保守参考"
    } else if multiplayer_fit >= 65.0 {
        "多人组局信号较强"
    } else if multiplayer_fit >= 50.0 {
        "具备一定联机属性，更适合固定好友局"
    } else {
        "联机标签存在，但多人内容更像辅助体验"
    };
    format!(
        "{}，{}。综合推荐 {:.1} 分，游戏质量 {:.1} 分。",
        review_phrase, multiplayer_phrase, recommendation_score, quality_score
    )
}

fn sanitize_narrative_patch(narrative: AnalysisNarrative) -> SanitizedNarrativePatch {
    SanitizedNarrativePatch {
        overview: sanitize_overview(&narrative.overview),
        strengths: sanitize_points(narrative.strengths),
        risks: sanitize_points(narrative.risks),
        dimension_reasons: sanitize_dimension_reasons(narrative.dimension_reasons),
    }
}

fn sanitize_overview(overview: &str) -> Option<String> {
    let trimmed = overview.trim();
    if trimmed.chars().count() < 8 {
        return None;
    }
    Some(trimmed.to_string())
}

fn sanitize_points(points: Vec<AnalysisPoint>) -> Vec<AnalysisPoint> {
    points
        .into_iter()
        .filter_map(|point| {
            let title = point.title.trim();
            let reason = point.reason.trim();
            if title.is_empty() || reason.is_empty() {
                return None;
            }
            Some(AnalysisPoint {
                title: title.to_string(),
                reason: reason.to_string(),
            })
        })
        .take(MAX_NARRATIVE_POINTS)
        .collect()
}

fn sanitize_dimension_reasons(reasons: Vec<(String, String)>) -> Vec<DimensionReasonPatch> {
    let valid_keys = ANALYSIS_DIMENSION_KEYS
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();

    reasons
        .into_iter()
        .filter_map(|(key, reason)| {
            let trimmed_key = key.trim();
            let trimmed_reason = reason.trim();
            if trimmed_reason.is_empty()
                || !valid_keys.contains(trimmed_key)
                || !seen.insert(trimmed_key.to_string())
            {
                return None;
            }
            Some((trimmed_key.to_string(), trimmed_reason.to_string()))
        })
        .take(ANALYSIS_DIMENSION_KEYS.len())
        .collect()
}

fn point_lists_equal(left: &[AnalysisPoint], right: &[AnalysisPoint]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.title == b.title && a.reason == b.reason)
}

fn build_evidence(game: &GameCard) -> Vec<AnalysisEvidenceItem> {
    let mut items = Vec::new();
    if let Some(pct) = game.positive_review_pct {
        items.push(AnalysisEvidenceItem {
            kind: AnalysisEvidenceKind::PositiveReviewPct,
            label: "好评率".to_string(),
            value: format!("{pct:.1}%"),
            interpretation: if pct >= 90.0 {
                "正向口碑明显，玩家满意度基础较稳。".to_string()
            } else if pct >= 80.0 {
                "整体评价偏正面，但仍需结合差评主题判断。".to_string()
            } else {
                "评价分歧较明显，需要重点核对风险点。".to_string()
            },
        });
    }
    if let Some(total_reviews) = game.total_reviews {
        items.push(AnalysisEvidenceItem {
            kind: AnalysisEvidenceKind::TotalReviews,
            label: "评测总量".to_string(),
            value: total_reviews.to_string(),
            interpretation: if total_reviews >= 1000 {
                "样本量充足，口碑波动更有参考价值。".to_string()
            } else {
                "样本量一般，结论需要保留一定弹性。".to_string()
            },
        });
    }
    if let Some(current_players) = game.current_players {
        items.push(AnalysisEvidenceItem {
            kind: AnalysisEvidenceKind::CurrentPlayers,
            label: "当前在线".to_string(),
            value: current_players.to_string(),
            interpretation: if current_players >= 1000 {
                "活跃度不错，组队成功率通常更稳定。".to_string()
            } else if current_players >= 100 {
                "有一定活跃样本，适合固定好友局。".to_string()
            } else {
                "在线样本偏小，临时匹配体验需要谨慎看待。".to_string()
            },
        });
    }
    if !game.tags.is_empty() {
        items.push(AnalysisEvidenceItem {
            kind: AnalysisEvidenceKind::Tags,
            label: "标签".to_string(),
            value: game
                .tags
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .join(" / "),
            interpretation: "标签能帮助判断题材、节奏与目标人群。".to_string(),
        });
    }
    if !game.multiplayer_modes.is_empty() {
        items.push(AnalysisEvidenceItem {
            kind: AnalysisEvidenceKind::MultiplayerModes,
            label: "联机模式".to_string(),
            value: game.multiplayer_modes.join(" / "),
            interpretation: "联机模式直接影响开黑方式与组队门槛。".to_string(),
        });
    }
    if let Some(short_description) = game
        .short_description
        .as_ref()
        .filter(|text| !text.is_empty())
    {
        items.push(AnalysisEvidenceItem {
            kind: AnalysisEvidenceKind::ShortDescription,
            label: "简介".to_string(),
            value: short_description.clone(),
            interpretation: "商店简介用于补足玩法目标与节奏信息。".to_string(),
        });
    }
    items
}

fn build_review_evidence(game: &GameCard) -> Vec<AnalysisReviewEvidenceItem> {
    let mut items = game
        .review_snippets
        .iter()
        .take(4)
        .map(review_snippet_to_evidence)
        .collect::<Vec<_>>();

    let has_positive_anywhere = game.review_snippets.iter().any(|snippet| snippet.voted_up);
    let has_negative_anywhere = game.review_snippets.iter().any(|snippet| !snippet.voted_up);
    let has_strength_selected = items
        .iter()
        .any(|item| item.stance == AnalysisReviewStance::Strength);
    let has_risk_selected = items
        .iter()
        .any(|item| item.stance == AnalysisReviewStance::Risk);

    if has_positive_anywhere && has_negative_anywhere {
        if !has_strength_selected {
            if let Some(snippet) = game.review_snippets.iter().find(|snippet| snippet.voted_up) {
                items.push(review_snippet_to_evidence(snippet));
            }
        }
        if !has_risk_selected {
            if let Some(snippet) = game
                .review_snippets
                .iter()
                .find(|snippet| !snippet.voted_up)
            {
                items.push(review_snippet_to_evidence(snippet));
            }
        }
    }

    items
}

fn review_snippet_to_evidence(
    snippet: &crate::models::ReviewSnippet,
) -> AnalysisReviewEvidenceItem {
    AnalysisReviewEvidenceItem {
        stance: if snippet.voted_up {
            AnalysisReviewStance::Strength
        } else {
            AnalysisReviewStance::Risk
        },
        quote: snippet.review.clone(),
        playtime_text: snippet
            .playtime_hours
            .map(|hours| format!("{hours:.1}h"))
            .unwrap_or_else(|| "未知时长".to_string()),
        interpretation: if snippet.voted_up {
            "正向评测通常反映了合作体验、反馈手感或上手门槛的优势。".to_string()
        } else {
            "负向评测可用于识别内容深度、平衡或留存方面的风险。".to_string()
        },
    }
}

impl SanitizedNarrativePatch {
    fn has_useful_content(&self) -> bool {
        self.overview.is_some()
            || !self.strengths.is_empty()
            || !self.risks.is_empty()
            || !self.dimension_reasons.is_empty()
    }
}

fn build_strengths(game: &GameCard, dimensions: &[AnalysisDimensionScore]) -> Vec<AnalysisPoint> {
    let mut items = Vec::new();
    if let Some(dimension) = dimensions
        .iter()
        .find(|item| item.key == "accessibility" && item.score >= 70.0)
    {
        items.push(AnalysisPoint {
            title: "上手负担较低".to_string(),
            reason: dimension.reason.clone(),
        });
    }
    if let Some(dimension) = dimensions
        .iter()
        .find(|item| item.key == "multiplayer_fit" && item.score >= 75.0)
    {
        items.push(AnalysisPoint {
            title: "适合朋友开黑".to_string(),
            reason: dimension.reason.clone(),
        });
    }
    if let Some(pct) = game.positive_review_pct.filter(|pct| *pct >= 90.0) {
        items.push(AnalysisPoint {
            title: "口碑表现稳定".to_string(),
            reason: format!("好评率达到 {pct:.1}%，且在样本量校正后仍保持较强稳定性。"),
        });
    }
    if let Some(dimension) = dimensions
        .iter()
        .find(|item| item.key == "discovery_value" && item.score >= 65.0)
    {
        items.push(AnalysisPoint {
            title: "具备发现价值".to_string(),
            reason: dimension.reason.clone(),
        });
    }
    if items.is_empty() {
        items.push(AnalysisPoint {
            title: if game.multiplayer_modes.is_empty() {
                "口碑信号仍可参考".to_string()
            } else {
                "具备基础尝试价值".to_string()
            },
            reason: if game.multiplayer_modes.is_empty() {
                "虽然多人模式标签缺失，但现有口碑和元数据仍能提供部分参考。".to_string()
            } else {
                "现有元数据仍显示出一定的多人尝试空间。".to_string()
            },
        });
    }
    items.into_iter().take(MAX_NARRATIVE_POINTS).collect()
}

fn build_risks(
    game: &GameCard,
    dimensions: &[AnalysisDimensionScore],
    risk_flags: &[crate::models::AnalysisRiskFlag],
) -> Vec<AnalysisPoint> {
    let mut items = Vec::new();
    if game.multiplayer_modes.is_empty() {
        items.push(AnalysisPoint {
            title: "多人标签缺失".to_string(),
            reason: "当前缺少多人模式这个核心信号，联机结论只能低置信度保守解读。".to_string(),
        });
    }
    if let Some(dimension) = dimensions
        .iter()
        .find(|item| item.key == "multiplayer_fit" && item.score < 45.0)
    {
        items.push(AnalysisPoint {
            title: "联机适配偏弱".to_string(),
            reason: dimension.reason.clone(),
        });
    }
    if let Some(dimension) = dimensions
        .iter()
        .find(|item| item.key == "content_depth" && item.score <= 60.0)
    {
        items.push(AnalysisPoint {
            title: "长期内容待观察".to_string(),
            reason: dimension.reason.clone(),
        });
    }
    for flag in risk_flags.iter().take(2) {
        items.push(AnalysisPoint {
            title: flag.label.clone(),
            reason: flag.reason.clone(),
        });
    }
    if game.current_players.unwrap_or(0) < 100 {
        items.push(AnalysisPoint {
            title: "在线样本偏小".to_string(),
            reason: "活跃人数不足时，匹配与留存结论会更不稳定。".to_string(),
        });
    }
    if items.is_empty() {
        items.push(AnalysisPoint {
            title: "后续更新仍要继续观察".to_string(),
            reason: "即使基础面不错，版本节奏和内容扩展仍可能影响长期体验。".to_string(),
        });
    }
    items.into_iter().take(MAX_NARRATIVE_POINTS).collect()
}
