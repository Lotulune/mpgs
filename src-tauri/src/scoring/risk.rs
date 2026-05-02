use crate::models::AnalysisRiskFlag;
use crate::scoring::signals::{CanonicalGameSignals, LanguageCode};

#[derive(Debug, Clone, Default)]
pub struct RiskScore {
    pub total_penalty: f64,
    pub flags: Vec<AnalysisRiskFlag>,
}

pub fn score_risks(signals: &CanonicalGameSignals) -> RiskScore {
    let server_risk =
        severity(signals.review_topics.server.negative + signals.review_topics.disconnect.negative);
    let bug_risk = severity(signals.review_topics.bug.negative);
    let content_risk = severity(
        signals.review_topics.content_depth.negative + signals.review_topics.repetition.negative,
    );
    let balance_risk = severity(signals.review_topics.balance.negative);
    let monetization_risk = severity(signals.review_topics.monetization.negative);
    let abandonment_risk = severity(signals.review_topics.abandonment.negative).max(
        if signals.activity.current_players.unwrap_or(100) < 25
            && signals.release.release_age_days.unwrap_or(0) > 365
        {
            0.35
        } else {
            0.0
        },
    );
    let localization_risk = if signals
        .language_codes
        .iter()
        .any(|code| matches!(code, LanguageCode::ZhCn | LanguageCode::ZhTw))
    {
        severity(signals.review_topics.localization.negative)
    } else {
        0.65
    };

    let total_penalty = (server_risk * 10.0
        + bug_risk * 8.0
        + content_risk * 8.0
        + balance_risk * 6.0
        + monetization_risk * 5.0
        + abandonment_risk * 8.0
        + localization_risk * 5.0)
        .min(28.0);

    let mut flags = Vec::new();
    push_flag(
        &mut flags,
        "server_risk",
        "联机稳定性风险",
        server_risk,
        "评论中出现服务器、匹配或掉线问题。",
    );
    push_flag(
        &mut flags,
        "bug_risk",
        "技术状态风险",
        bug_risk,
        "评论中出现崩溃、卡顿或 Bug 投诉。",
    );
    push_flag(
        &mut flags,
        "content_risk",
        "内容厚度风险",
        content_risk,
        "评论中出现内容偏少、后期重复或留存不足的反馈。",
    );
    push_flag(
        &mut flags,
        "balance_risk",
        "平衡性风险",
        balance_risk,
        "评论中出现对战平衡或数值争议。",
    );
    push_flag(
        &mut flags,
        "monetization_risk",
        "付费争议风险",
        monetization_risk,
        "评论中出现 DLC、价格或付费策略争议。",
    );
    push_flag(
        &mut flags,
        "abandonment_risk",
        "长线维护风险",
        abandonment_risk,
        "更新节奏或活跃度信号提示长期维护需要继续观察。",
    );
    push_flag(
        &mut flags,
        "localization_risk",
        "本地化风险",
        localization_risk,
        "中文支持或翻译质量存在不确定性。",
    );

    RiskScore {
        total_penalty,
        flags,
    }
}

fn severity(count: u32) -> f64 {
    match count {
        0 => 0.0,
        1 => 0.45,
        2 => 0.7,
        _ => 1.0,
    }
}

fn push_flag(
    flags: &mut Vec<AnalysisRiskFlag>,
    key: &str,
    label: &str,
    severity: f64,
    reason: &str,
) {
    if severity <= 0.0 {
        return;
    }

    flags.push(AnalysisRiskFlag {
        key: key.to_string(),
        label: label.to_string(),
        severity,
        reason: reason.to_string(),
    });
}
