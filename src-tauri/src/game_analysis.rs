use crate::llm::{self, LlmRuntimeConfig};
use crate::models::{GameAnalysisReport, GameCard};
use anyhow::Result;
pub use mpgs_core::analysis::{
    apply_narrative_patch, build_rule_report, summarize_report_as_assessment, AnalysisNarrative,
    ANALYSIS_DIMENSION_KEYS,
};
use reqwest::Client;

pub async fn generate_game_analysis(
    client: &Client,
    config: &LlmRuntimeConfig,
    game: &GameCard,
    generated_at: String,
) -> Result<GameAnalysisReport> {
    let rule_report = build_rule_report(game, generated_at)?;
    if config.api_key.is_none() {
        return Ok(rule_report);
    }

    match llm::generate_analysis_narrative(client, config, game, &rule_report).await {
        Ok(narrative) => Ok(apply_narrative_patch(rule_report, narrative)),
        Err(_) => Ok(rule_report),
    }
}
