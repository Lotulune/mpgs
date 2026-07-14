use std::cmp::Ordering;

use mpgs_domain::SteamAppId;

use crate::pipeline::RankedCandidate;
use crate::unit;

const MMR_WINDOW_SIZE: usize = 200;

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
    let mut sorted = items;
    sorted.sort_by(|a, b| {
        b.score
            .final_score
            .partial_cmp(&a.score.final_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.app_id.cmp(&b.app_id))
    });
    let tail = if sorted.len() > MMR_WINDOW_SIZE {
        sorted.split_off(MMR_WINDOW_SIZE)
    } else {
        Vec::new()
    };
    let mut remaining: Vec<_> = sorted.into_iter().map(|item| (item, 0.0_f64)).collect();

    let mut selected: Vec<RankedCandidate> = Vec::with_capacity(remaining.len() + tail.len());
    while !remaining.is_empty() {
        let mut best_idx = 0usize;
        let mut best_val = f64::NEG_INFINITY;
        for (idx, (cand, diversity_pen)) in remaining.iter().enumerate() {
            let relevance = cand.score.final_score;
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
        let (chosen, _) = remaining.remove(best_idx);
        for (candidate, diversity_pen) in &mut remaining {
            *diversity_pen = diversity_pen.max(similarity(&chosen, candidate));
        }
        selected.push(chosen);
    }
    selected.extend(tail);
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

#[cfg(test)]
mod tests {
    use super::{MMR_WINDOW_SIZE, mmr_rerank};
    use crate::{Explanation, RankedCandidate, ScoreBreakdown};

    #[test]
    fn large_tail_is_preserved_in_relevance_order() {
        let items: Vec<_> = (0..(MMR_WINDOW_SIZE + 25))
            .map(|index| RankedCandidate {
                app_id: index as u32,
                name: format!("game-{index}"),
                dominant_mode: Some("coop".into()),
                recommended_min: Some(1),
                recommended_max: Some(4),
                score: ScoreBreakdown {
                    friend_fit: 0.8,
                    section_score: 1.0 - index as f64 / 1_000.0,
                    personalized_score: 1.0 - index as f64 / 1_000.0,
                    final_score: 1.0 - index as f64 / 1_000.0,
                },
                explanation: Explanation {
                    reasons: vec!["coop".into()],
                    cautions: Vec::new(),
                    evidence_ids: Vec::new(),
                },
                algorithm_version: "test".into(),
            })
            .collect();
        let reranked = mmr_rerank(items, 0.75, 2);
        assert_eq!(reranked.len(), MMR_WINDOW_SIZE + 25);
        let tail_ids: Vec<_> = reranked[MMR_WINDOW_SIZE..]
            .iter()
            .map(|item| item.app_id)
            .collect();
        assert_eq!(
            tail_ids,
            (MMR_WINDOW_SIZE as u32..(MMR_WINDOW_SIZE + 25) as u32).collect::<Vec<_>>()
        );
    }
}
