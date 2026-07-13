use std::cmp::Ordering;

use mpgs_domain::SteamAppId;

use crate::pipeline::RankedCandidate;
use crate::unit;

/// Maximal Marginal Relevance style re-rank for diversity.
///
/// `lambda` closer to 1 prefers pure relevance; closer to 0 prefers novelty.
pub fn mmr_rerank(
    items: Vec<RankedCandidate>,
    lambda: f64,
    explore_slots: usize,
) -> Vec<RankedCandidate> {
    if items.len() <= 1 {
        return items;
    }
    let lambda = unit(lambda);
    let mut remaining = items;
    remaining.sort_by(|a, b| {
        b.score
            .final_score
            .partial_cmp(&a.score.final_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.app_id.cmp(&b.app_id))
    });

    let mut selected: Vec<RankedCandidate> = Vec::with_capacity(remaining.len());
    while !remaining.is_empty() {
        let mut best_idx = 0usize;
        let mut best_val = f64::NEG_INFINITY;
        for (idx, cand) in remaining.iter().enumerate() {
            let relevance = cand.score.final_score;
            let diversity_pen = if selected.is_empty() {
                0.0
            } else {
                selected
                    .iter()
                    .map(|s| similarity(s, cand))
                    .fold(0.0_f64, f64::max)
            };
            let mut mmr = lambda * relevance - (1.0 - lambda) * diversity_pen;
            // Soft exploration: slightly boost lower-confidence items early.
            if selected.len() < explore_slots
                && cand.score.friend_fit > 0.55
                && cand.components_evidence_low()
            {
                mmr += 0.02;
            }
            if mmr > best_val {
                best_val = mmr;
                best_idx = idx;
            }
        }
        selected.push(remaining.remove(best_idx));
    }
    selected
}

fn similarity(a: &RankedCandidate, b: &RankedCandidate) -> f64 {
    if a.app_id == b.app_id {
        return 1.0;
    }
    let mode_same = match (&a.dominant_mode, &b.dominant_mode) {
        (Some(x), Some(y)) if x == y => 0.45,
        _ => 0.0,
    };
    let fit_close = 1.0 - (a.score.friend_fit - b.score.friend_fit).abs();
    unit(mode_same + 0.35 * fit_close + 0.2 * tag_overlap(a.app_id, b.app_id))
}

fn tag_overlap(a: SteamAppId, b: SteamAppId) -> f64 {
    let _ = (a, b);
    0.0
}

impl RankedCandidate {
    fn components_evidence_low(&self) -> bool {
        self.explanation.reasons.iter().any(|r| r.contains("早期"))
            || self
                .explanation
                .cautions
                .iter()
                .any(|c| c.contains("置信度"))
    }
}
