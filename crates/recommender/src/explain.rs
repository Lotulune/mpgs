use mpgs_domain::RankingSignals;
use serde::{Deserialize, Serialize};

use crate::ScoreBreakdown;
use crate::unit;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Explanation {
    pub reasons: Vec<String>,
    pub cautions: Vec<String>,
    pub evidence_ids: Vec<String>,
}

pub fn explain(
    app_id: u32,
    signals: &RankingSignals,
    score: &ScoreBreakdown,
    dominant_mode: Option<&str>,
) -> Explanation {
    let mut reasons = Vec::new();
    let mut cautions = Vec::new();
    let mut evidence_ids = Vec::new();
    let mp = &signals.multiplayer;

    if unit(mp.private_session) >= 0.6 {
        reasons.push("支持私人房间联机".into());
        evidence_ids.push(format!("feature:private_session:{app_id}"));
    }
    if unit(mp.self_host_or_dedicated) >= 0.6 {
        reasons.push("可自建服或专用服务器".into());
        evidence_ids.push(format!("feature:self_hosted_server:{app_id}"));
    }
    if unit(mp.online_coop) >= 0.6 {
        reasons.push("具备在线合作体验".into());
        evidence_ids.push(format!("feature:online_coop:{app_id}"));
    }
    if unit(mp.group_size_fit) >= 0.7 {
        reasons.push("人数匹配当前小组".into());
    }
    if unit(signals.quality) >= 0.75 {
        reasons.push("累计口碑稳定".into());
        evidence_ids.push(format!("review:{app_id}:summary"));
    }

    if unit(mp.matchmaking_core) >= 0.6 {
        cautions.push("核心体验偏公共匹配".into());
    }
    if unit(mp.public_world_dependency) >= 0.6 {
        cautions.push("依赖公共世界玩家生态".into());
    }
    if unit(mp.service_shutdown_risk) >= 0.5 {
        cautions.push("服务停运风险需关注".into());
    }
    if unit(mp.group_size_mismatch) >= 0.4 {
        cautions.push("推荐人数与当前小组可能不匹配".into());
    }
    if unit(signals.data_confidence) < 0.45 {
        cautions.push("早期数据，置信度偏低".into());
        reasons.push("早期数据".into());
    }
    if let Some(mode) = dominant_mode
        && mode.eq_ignore_ascii_case("mmo")
    {
        cautions.push("MMO/公共世界主导".into());
    }

    if reasons.is_empty() && score.friend_fit >= 0.5 {
        reasons.push("熟人联机适配度尚可".into());
    }
    if reasons.is_empty() {
        reasons.push("进入候选池".into());
    }

    Explanation {
        reasons,
        cautions,
        evidence_ids,
    }
}
