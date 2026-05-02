use crate::models::{
    AnalysisConfidence, AnalysisDimensionScore, AnalysisRiskFlag, RecommendationPool,
};
use crate::scoring::accessibility::score_accessibility;
use crate::scoring::activity_health::score_activity_health;
use crate::scoring::confidence::score_confidence;
use crate::scoring::content_depth::score_content_depth;
use crate::scoring::discovery_value::{freshness_score, score_discovery_value};
use crate::scoring::multiplayer_fit::score_multiplayer_fit;
use crate::scoring::normalize::normalize_game_signals;
use crate::scoring::review_quality::score_review_quality;
use crate::scoring::risk::{score_risks, RiskScore};

#[derive(Debug, Clone)]
pub struct GameScoreV2 {
    pub quality_score: f64,
    pub recommendation_score: f64,
    pub confidence_score: f64,
    pub confidence: AnalysisConfidence,
    pub pool_type: RecommendationPool,
    pub risk_flags: Vec<AnalysisRiskFlag>,
    pub dimension_scores: Vec<AnalysisDimensionScore>,
}

pub fn score_game_v2(game: &crate::models::GameCard) -> GameScoreV2 {
    let signals = normalize_game_signals(game);
    let review_quality = score_review_quality(&signals.review_stats);
    let multiplayer_fit = score_multiplayer_fit(&signals);
    let activity_health = score_activity_health(&signals.activity);
    let content_depth = score_content_depth(&signals);
    let accessibility = score_accessibility(&signals);
    let discovery_value = score_discovery_value(&signals, review_quality);
    let risk = score_risks(&signals);
    let confidence_score = score_confidence(&signals);
    let uncertainty_penalty = (1.0 - confidence_score) * 10.0;
    let shortboard_penalty = shortboard_penalty(review_quality, multiplayer_fit, activity_health);

    let mut quality_score = (0.28 * review_quality
        + 0.24 * multiplayer_fit
        + 0.16 * activity_health
        + 0.14 * content_depth
        + 0.10 * accessibility
        + 0.08 * discovery_value
        - risk.total_penalty
        - uncertainty_penalty
        - shortboard_penalty)
        .clamp(0.0, 100.0);
    quality_score = apply_quality_caps(
        quality_score,
        review_quality,
        multiplayer_fit,
        confidence_score,
        &signals,
        &risk,
    );

    let pool_type = classify_pool(
        &signals,
        quality_score,
        discovery_value,
        confidence_score,
        risk.total_penalty,
        review_quality,
    );
    let recommendation_score = score_recommendation(
        quality_score,
        review_quality,
        multiplayer_fit,
        activity_health,
        discovery_value,
        freshness_score(&signals),
    );
    let confidence = if confidence_score >= 0.72 {
        AnalysisConfidence::High
    } else if confidence_score >= 0.45 {
        AnalysisConfidence::Medium
    } else {
        AnalysisConfidence::Low
    };

    GameScoreV2 {
        quality_score: round_score(quality_score),
        recommendation_score: round_score(recommendation_score),
        confidence_score,
        confidence,
        pool_type,
        risk_flags: risk.flags,
        dimension_scores: vec![
            AnalysisDimensionScore {
                key: "review_quality".to_string(),
                label: "口碑质量".to_string(),
                score: round_score(review_quality),
                reason: review_quality_reason(review_quality, signals.review_stats.total_reviews),
            },
            AnalysisDimensionScore {
                key: "multiplayer_fit".to_string(),
                label: "联机适配度".to_string(),
                score: round_score(multiplayer_fit),
                reason: multiplayer_reason(&signals, multiplayer_fit),
            },
            AnalysisDimensionScore {
                key: "activity_health".to_string(),
                label: "活跃健康度".to_string(),
                score: round_score(activity_health),
                reason: activity_reason(signals.activity.current_players),
            },
            AnalysisDimensionScore {
                key: "content_depth".to_string(),
                label: "内容深度".to_string(),
                score: round_score(content_depth),
                reason: content_depth_reason(&signals, content_depth),
            },
            AnalysisDimensionScore {
                key: "accessibility".to_string(),
                label: "上手与本地化".to_string(),
                score: round_score(accessibility),
                reason: accessibility_reason(&signals, accessibility),
            },
            AnalysisDimensionScore {
                key: "discovery_value".to_string(),
                label: "发现价值".to_string(),
                score: round_score(discovery_value),
                reason: discovery_reason(&signals, discovery_value),
            },
        ],
    }
}

fn shortboard_penalty(review_quality: f64, multiplayer_fit: f64, activity_health: f64) -> f64 {
    let critical_min = review_quality.min(multiplayer_fit).min(activity_health);
    (50.0 - critical_min).max(0.0) * 0.18
}

fn apply_quality_caps(
    mut quality_score: f64,
    review_quality: f64,
    multiplayer_fit: f64,
    confidence_score: f64,
    signals: &crate::scoring::signals::CanonicalGameSignals,
    risk: &RiskScore,
) -> f64 {
    if multiplayer_fit < 35.0 {
        quality_score = quality_score.min(74.0);
    }
    if review_quality < 55.0 && signals.review_stats.total_reviews >= 50 {
        quality_score = quality_score.min(70.0);
    }
    if risk.total_penalty >= 18.0 {
        quality_score = quality_score.min(72.0);
    }
    if confidence_score < 0.35 {
        quality_score = quality_score.min(78.0);
    }
    quality_score
}

fn classify_pool(
    signals: &crate::scoring::signals::CanonicalGameSignals,
    quality_score: f64,
    discovery_value: f64,
    confidence_score: f64,
    risk_penalty: f64,
    review_quality: f64,
) -> RecommendationPool {
    if signals.demo.is_demo_only
        || (signals.demo.has_demo && signals.review_stats.total_reviews < 50)
    {
        RecommendationPool::DemoPotential
    } else if signals
        .release
        .release_age_days
        .is_some_and(|days| days <= 30)
        || signals.release.early_access_hint
    {
        RecommendationPool::NewRelease
    } else if review_quality >= 75.0
        && signals.review_stats.total_reviews < 1000
        && signals.activity.current_players.unwrap_or(499) < 500
        && risk_penalty < 12.0
    {
        RecommendationPool::HiddenGem
    } else if quality_score >= 70.0 && discovery_value >= 55.0 && confidence_score >= 0.45 {
        RecommendationPool::FriendsParty
    } else {
        RecommendationPool::Evergreen
    }
}

fn score_recommendation(
    quality_score: f64,
    review_quality: f64,
    multiplayer_fit: f64,
    activity_health: f64,
    discovery_value: f64,
    freshness: f64,
) -> f64 {
    let mut score = 0.64 * quality_score
        + 0.08 * review_quality
        + 0.08 * multiplayer_fit
        + 0.07 * discovery_value
        + 0.05 * freshness;

    if review_quality >= 90.0 && multiplayer_fit >= 65.0 && activity_health >= 80.0 {
        score += 10.0;
    } else if review_quality >= 90.0 && multiplayer_fit >= 58.0 && activity_health >= 80.0 {
        score += 7.0;
    } else if review_quality >= 85.0 && multiplayer_fit >= 52.0 && activity_health >= 65.0 {
        score += 3.0;
    }

    if multiplayer_fit < 40.0 {
        score -= 6.0;
    } else if multiplayer_fit < 50.0 {
        score -= 4.0;
    }

    score.clamp(0.0, 100.0)
}

fn review_quality_reason(review_quality: f64, total_reviews: u32) -> String {
    if total_reviews >= 1000 && review_quality >= 80.0 {
        "好评率经过样本量校正后依然稳定，长期口碑更可信。".to_string()
    } else if total_reviews >= 100 {
        "已有可用的评测样本，但仍需要结合评论主题判断具体短板。".to_string()
    } else {
        "评论样本偏少，口碑分数已按低证据场景做保守处理。".to_string()
    }
}

fn multiplayer_reason(
    signals: &crate::scoring::signals::CanonicalGameSignals,
    multiplayer_fit: f64,
) -> String {
    if !signals.multiplayer_modes.has_any {
        return "多人模式标签缺失，联机结论只能保守看待。".to_string();
    }
    if signals.multiplayer_modes.online_pvp && signals.multiplayer_modes.local_coop {
        return "同时覆盖对抗模式与本地/分屏场景，朋友局的切换空间更大。".to_string();
    }
    if signals.multiplayer_modes.online_pvp {
        return "对抗模式明确，组局后的博弈张力比纯标签信号更可信。".to_string();
    }
    if signals.multiplayer_modes.local_coop {
        return "本地/分屏多人能明显降低成局门槛，适合线下聚会。".to_string();
    }
    if multiplayer_fit < 50.0 {
        return "虽然带有联机标签，但更像可选陪伴或弱社交补充，不宜按强多人局来估值。".to_string();
    }
    if multiplayer_fit >= 70.0 {
        "在线合作信号明确，组队摩擦相对可控。".to_string()
    } else {
        "联机模式存在，但评论里仍有成局或稳定性方面的不确定因素。".to_string()
    }
}

fn activity_reason(current_players: Option<u32>) -> String {
    match current_players {
        Some(players) if players >= 1000 => {
            "当前在线样本充足，临时匹配与长期留存的把握更高。".to_string()
        }
        Some(players) if players >= 100 => "已有一定活跃基础，更适合固定好友队一起玩。".to_string(),
        Some(_) => "在线样本偏小，匹配体验需要结合好友局场景判断。".to_string(),
        None => "缺少在线人数数据，活跃结论已按中性且低置信度处理。".to_string(),
    }
}

fn content_depth_reason(
    signals: &crate::scoring::signals::CanonicalGameSignals,
    content_depth: f64,
) -> String {
    if signals.review_topics.repetition.negative > 0
        || signals.review_topics.content_depth.negative > 0
    {
        "评论中出现内容偏少、后期重复或留存不足的反馈，需要压低长线预期。".to_string()
    } else if content_depth >= 70.0 {
        "标签与评论都显示它不只靠一次性新鲜感支撑。".to_string()
    } else {
        "现有标签能支撑中短期体验，但长期厚度仍有待继续观察。".to_string()
    }
}

fn accessibility_reason(
    signals: &crate::scoring::signals::CanonicalGameSignals,
    accessibility: f64,
) -> String {
    let has_chinese = signals.language_codes.iter().any(|code| {
        matches!(
            code,
            crate::scoring::signals::LanguageCode::ZhCn
                | crate::scoring::signals::LanguageCode::ZhTw
        )
    });
    if has_chinese && signals.demo.has_demo {
        "有试玩路径且包含中文支持，拉新和快速判断成本更低。".to_string()
    } else if has_chinese {
        "中文支持能明显降低理解门槛，但教程与机制复杂度仍要继续观察。".to_string()
    } else if accessibility >= 60.0 {
        "虽然没有中文优势，但试玩、休闲标签或输入支持降低了上手门槛。".to_string()
    } else {
        "本地化、教程或复杂度信号一般，临时拉人时可能需要更多解释成本。".to_string()
    }
}

fn discovery_reason(
    signals: &crate::scoring::signals::CanonicalGameSignals,
    discovery_value: f64,
) -> String {
    if signals.demo.is_demo_only {
        "Demo 形态带来了较高的新鲜度与尝鲜价值，但成熟度仍要保守看待。".to_string()
    } else if signals.release.recent_release {
        "发布时间较新，更适合放进新游观察与尝鲜候选池。".to_string()
    } else if discovery_value >= 60.0 {
        "口碑与曝光度的组合提示它具备被低估的发现价值。".to_string()
    } else {
        "发现价值一般，更像稳定型候选而不是冷门惊喜。".to_string()
    }
}

fn round_score(score: f64) -> f64 {
    (score * 10.0).round() / 10.0
}
