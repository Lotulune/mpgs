# AI Analysis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder single-game AI summary flow with a cached evidence-based detail report that auto-generates on first open, degrades to rule-only output when the model fails, and keeps the existing `aiSummary` / `aiScore` list behavior working.

**Architecture:** Add a dedicated Rust game-analysis pipeline that builds a rule report from the existing `GameCard` metadata, optionally enhances it with constrained LLM narrative output, and persists the latest full report JSON on the `games` table. On the frontend, keep the global AI assistant flow intact, but move detail-page analysis loading into a dedicated hook and panel that reads cached reports, auto-generates when missing, and renders the approved summary-plus-expandable evidence layout.

**Tech Stack:** Tauri 2, Rust, rusqlite, reqwest, React 19, TypeScript, Vitest, Testing Library, `@tauri-apps/api`

---

## Scope Check

The approved spec is a single subsystem: per-game AI analysis for the detail page plus the shared persistence and API surface required to support it. It explicitly excludes AI recommendation chat, analysis history, batch pre-generation, and independent analysis pages, so this can be implemented in one focused plan without splitting into sub-projects.

## File Structure

**Backend**

- Modify: `src-tauri/src/models.rs`
  - Add persisted full-report types while preserving the existing `AiAssessment` compatibility shape.
- Modify: `src-tauri/src/db.rs`
  - Add SQLite columns and helper functions for cached report JSON and generated-at timestamps.
- Create: `src-tauri/src/game_analysis.rs`
  - Own the rule report builder, hybrid orchestration, and `GameAnalysisReport -> AiAssessment` adapter.
- Modify: `src-tauri/src/llm.rs`
  - Add constrained narrative generation/parsing for the hybrid path without changing the rest of the OpenAI-compatible runtime configuration code.
- Modify: `src-tauri/src/commands.rs`
  - Expose `get_game_analysis` and `generate_game_analysis`, and route the legacy `assess_game_with_ai` command through the new report pipeline.
- Modify: `src-tauri/src/lib.rs`
  - Register the new module and Tauri commands.
- Modify: `src-tauri/tests/game_metadata_tests.rs`
  - Cover report persistence round-trips.
- Create: `src-tauri/tests/game_analysis_tests.rs`
  - Cover rule-report construction, hybrid patch merging, and the legacy assessment adapter.

**Frontend**

- Modify: `src/types.ts`
  - Mirror the full-report types returned by Rust.
- Modify: `src/api/client.ts`
  - Add `getGameAnalysis()` / `generateGameAnalysis()` plus browser-mode in-memory caching helpers.
- Create: `src/api/client.test.ts`
  - Lock down browser-mode cache behavior.
- Create: `src/pages/detail/useGameAnalysis.ts`
  - Own per-`appid` cached load, auto-generation, refresh, and error state for the detail page.
- Create: `src/pages/detail/GameAnalysisPanel.tsx`
  - Render summary, badges, dimensions, strengths, risks, evidence, and review quotes.
- Modify: `src/pages/detail/DetailPage.tsx`
  - Replace the placeholder AI tab with the new panel and hook.
- Modify: `src/pages/detail/DetailPage.test.tsx`
  - Cover cached render, auto-generate, refresh, and fallback messaging.
- Modify: `src/App.tsx`
  - Stop treating detail-page analysis as a global `assessment` concern and keep the AI assistant path compatible.
- Modify: `src/App.test.tsx`
  - Add a regression that opening a dashboard card into detail view still works once the detail page owns its own analysis loading.
- Modify: `src/App.css`
  - Add the approved report layout styles.

## Recommended Delivery Order

1. Persist the report schema first so every later layer has a stable storage contract.
2. Build the Rust rule/hybrid generator next so commands and clients can target real behavior.
3. Expose the new commands and browser-mode client wrappers before touching the detail UI.
4. Upgrade the detail page once the data surface is stable.
5. Finish with App-shell regressions and full verification so the existing dashboard and AI assistant flows stay intact.

### Task 1: Persist Cached Game Analysis Reports in SQLite

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/tests/game_metadata_tests.rs`

- [ ] **Step 1: Write the failing persistence test**

Add this test to `src-tauri/tests/game_metadata_tests.rs` after the existing metadata round-trip coverage:

```rust
#[test]
fn sqlite_round_trips_cached_game_analysis_report() {
    let conn = empty_db();
    let card = GameCard {
        appid: 4_220_001,
        name: "Evidence Factory".to_string(),
        section: "new".to_string(),
        short_description: Some("A co-op fixture for cached analysis coverage.".to_string()),
        release_date: Some("2026-04-20".to_string()),
        release_date_text: "Apr 20, 2026".to_string(),
        release_state: StoreReleaseState::Released,
        demo_status: DemoStatus::Released,
        supported_languages: vec!["english".to_string(), "schinese".to_string()],
        is_adult_content: false,
        price_text: Some("$14.99".to_string()),
        discount_percent: None,
        positive_review_pct: Some(93.0),
        total_reviews: Some(540),
        current_players: Some(321),
        recommendation_score: 84.0,
        ai_score: Some(84.0),
        ai_summary: "Legacy summary placeholder.".to_string(),
        capsule_url: "https://cdn.example.test/evidence-factory.jpg".to_string(),
        store_screenshot_urls: vec![],
        tags: vec!["Co-op".to_string(), "Puzzle".to_string()],
        multiplayer_modes: vec!["Online Co-op".to_string()],
        review_snippets: vec![ReviewSnippet {
            voted_up: true,
            review: "Great callouts and low friction teamwork.".to_string(),
            playtime_hours: Some(9.0),
        }],
        user_state: UserGameState::default(),
    };
    db::upsert_game(&conn, &card).expect("upsert game");

    let report = GameAnalysisReport {
        appid: card.appid,
        generated_at: "2026-04-30T08:00:00Z".to_string(),
        source: AnalysisSource::Rule,
        confidence: AnalysisConfidence::Medium,
        overall_score: 84.0,
        overview: "Rule-only fixture overview.".to_string(),
        dimension_scores: vec![AnalysisDimensionScore {
            key: "multiplayer_fun".to_string(),
            label: "多人乐趣".to_string(),
            score: 88.0,
            reason: "Strong co-op signal.".to_string(),
        }],
        strengths: vec![AnalysisPoint {
            title: "沟通负担低".to_string(),
            reason: "评论集中在合作流畅度。".to_string(),
        }],
        risks: vec![AnalysisPoint {
            title: "后期内容未知".to_string(),
            reason: "评论样本仍偏少。".to_string(),
        }],
        evidence: vec![AnalysisEvidenceItem {
            kind: AnalysisEvidenceKind::PositiveReviewPct,
            label: "好评率".to_string(),
            value: "93%".to_string(),
            interpretation: "口碑基础健康。".to_string(),
        }],
        review_evidence: vec![AnalysisReviewEvidenceItem {
            stance: AnalysisReviewStance::Strength,
            quote: "Great callouts and low friction teamwork.".to_string(),
            playtime_text: "9 小时游玩".to_string(),
            interpretation: "支撑多人协作体验稳定。".to_string(),
        }],
    };

    db::save_game_analysis(&conn, card.appid, &report).expect("save report");

    let loaded = db::load_game_analysis(&conn, card.appid)
        .expect("load report")
        .expect("report exists");

    assert_eq!(loaded.source, AnalysisSource::Rule);
    assert_eq!(loaded.confidence, AnalysisConfidence::Medium);
    assert_eq!(loaded.overview, "Rule-only fixture overview.");
    assert_eq!(loaded.evidence.len(), 1);
    assert_eq!(loaded.review_evidence.len(), 1);
}
```

- [ ] **Step 2: Run the targeted Rust test to confirm the failure**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test game_metadata_tests sqlite_round_trips_cached_game_analysis_report -- --exact
```

Expected: FAIL with compile errors for missing analysis report types and `save_game_analysis` / `load_game_analysis` helpers.

- [ ] **Step 3: Add the persisted Rust report model types**

Extend `src-tauri/src/models.rs` with the full-report types directly after `AiAssessment`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisSource {
    Hybrid,
    Rule,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisEvidenceKind {
    PositiveReviewPct,
    TotalReviews,
    CurrentPlayers,
    Tags,
    MultiplayerModes,
    ShortDescription,
    ReviewSnippet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisReviewStance {
    Strength,
    Risk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisDimensionScore {
    pub key: String,
    pub label: String,
    pub score: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisPoint {
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisEvidenceItem {
    pub kind: AnalysisEvidenceKind,
    pub label: String,
    pub value: String,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisReviewEvidenceItem {
    pub stance: AnalysisReviewStance,
    pub quote: String,
    pub playtime_text: String,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameAnalysisReport {
    pub appid: u32,
    pub generated_at: String,
    pub source: AnalysisSource,
    pub confidence: AnalysisConfidence,
    pub overall_score: f64,
    pub overview: String,
    pub dimension_scores: Vec<AnalysisDimensionScore>,
    pub strengths: Vec<AnalysisPoint>,
    pub risks: Vec<AnalysisPoint>,
    pub evidence: Vec<AnalysisEvidenceItem>,
    pub review_evidence: Vec<AnalysisReviewEvidenceItem>,
}
```

- [ ] **Step 4: Add the SQLite columns and helpers**

Update `src-tauri/src/db.rs` in three places:

1. Add the columns to the `games` table definition:

```rust
ai_summary TEXT NOT NULL,
ai_analysis_report_json TEXT,
ai_analysis_generated_at TEXT,
capsule_url TEXT NOT NULL,
```

2. Add migration guards inside `ensure_games_metadata_columns(conn)`:

```rust
add_games_column_if_missing(
    conn,
    "ai_analysis_report_json",
    "ALTER TABLE games ADD COLUMN ai_analysis_report_json TEXT",
)?;
add_games_column_if_missing(
    conn,
    "ai_analysis_generated_at",
    "ALTER TABLE games ADD COLUMN ai_analysis_generated_at TEXT",
)?;
```

3. Add dedicated helpers near the other `load_*` / `set_*` database functions:

```rust
pub fn load_game_analysis(conn: &Connection, appid: u32) -> Result<Option<GameAnalysisReport>> {
    let payload = conn
        .query_row(
            "SELECT ai_analysis_report_json FROM games WHERE appid = ?1",
            params![appid],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();

    match payload {
        Some(payload) => Ok(Some(serde_json::from_str(&payload)?)),
        None => Ok(None),
    }
}

pub fn save_game_analysis(
    conn: &Connection,
    appid: u32,
    report: &GameAnalysisReport,
) -> Result<()> {
    conn.execute(
        r#"
        UPDATE games
        SET ai_analysis_report_json = ?2,
            ai_analysis_generated_at = ?3
        WHERE appid = ?1
        "#,
        params![appid, serde_json::to_string(report)?, report.generated_at],
    )?;
    Ok(())
}
```

- [ ] **Step 5: Re-run the metadata persistence test**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test game_metadata_tests sqlite_round_trips_cached_game_analysis_report -- --exact
```

Expected: PASS with the new report types serializing cleanly through SQLite.

- [ ] **Step 6: Commit the persistence layer**

```bash
git add src-tauri/src/models.rs src-tauri/src/db.rs src-tauri/tests/game_metadata_tests.rs
git commit -m "feat: persist cached game analysis reports"
```

### Task 2: Build the Rule/Hybrid Analysis Engine and Legacy Adapter

**Files:**
- Create: `src-tauri/src/game_analysis.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/llm.rs`
- Create: `src-tauri/tests/game_analysis_tests.rs`

- [ ] **Step 1: Write the failing Rust analysis-engine tests**

Create `src-tauri/tests/game_analysis_tests.rs` with these first assertions:

```rust
use tauri_app_lib::game_analysis::{
    apply_narrative_patch, build_rule_report, summarize_report_as_assessment,
};
use tauri_app_lib::llm::AnalysisNarrative;
use tauri_app_lib::models::{AnalysisConfidence, AnalysisSource, GameCard, ReviewSnippet, UserGameState};
use tauri_app_lib::recommendation::DemoStatus;

fn fixture_game() -> GameCard {
    GameCard {
        appid: 55_000,
        name: "Signal Kitchen".to_string(),
        short_description: Some("A communication-heavy co-op kitchen run.".to_string()),
        section: "new".to_string(),
        release_date: Some("2026-04-22".to_string()),
        release_date_text: "Apr 22, 2026".to_string(),
        release_state: tauri_app_lib::models::StoreReleaseState::Released,
        demo_status: DemoStatus::ReleasedWithDemo,
        supported_languages: vec!["english".to_string(), "schinese".to_string()],
        is_adult_content: false,
        price_text: Some("$12.99".to_string()),
        discount_percent: None,
        positive_review_pct: Some(94.0),
        total_reviews: Some(620),
        current_players: Some(1450),
        recommendation_score: 86.0,
        ai_score: Some(86.0),
        ai_summary: "Legacy summary.".to_string(),
        capsule_url: "https://cdn.example.test/signal-kitchen.jpg".to_string(),
        store_screenshot_urls: vec![],
        tags: vec!["Co-op".to_string(), "Cooking".to_string()],
        multiplayer_modes: vec!["Online Co-op".to_string(), "Co-op".to_string()],
        review_snippets: vec![
            ReviewSnippet {
                voted_up: true,
                review: "Fast callouts, great rhythm, low downtime.".to_string(),
                playtime_hours: Some(14.0),
            },
            ReviewSnippet {
                voted_up: false,
                review: "Needs a regular squad or the chaos spikes hard.".to_string(),
                playtime_hours: Some(4.5),
            },
        ],
        user_state: UserGameState::default(),
    }
}

#[test]
fn rule_report_builds_scores_evidence_and_confidence() {
    let report = build_rule_report(&fixture_game(), "2026-04-30T08:15:00Z".to_string())
        .expect("build rule report");

    assert_eq!(report.source, AnalysisSource::Rule);
    assert_eq!(report.confidence, AnalysisConfidence::High);
    assert_eq!(report.dimension_scores.len(), 5);
    assert!(!report.strengths.is_empty());
    assert!(!report.risks.is_empty());
    assert!(report.evidence.iter().any(|item| item.label == "好评率"));
    assert!(report.review_evidence.len() >= 2);
}

#[test]
fn narrative_patch_updates_copy_without_losing_rule_structure() {
    let rule_report = build_rule_report(&fixture_game(), "2026-04-30T08:15:00Z".to_string())
        .expect("rule report");
    let patched = apply_narrative_patch(
        rule_report.clone(),
        AnalysisNarrative {
            overview: "Hybrid narrative summary.".to_string(),
            strengths: vec![tauri_app_lib::models::AnalysisPoint {
                title: "强社交节奏".to_string(),
                reason: "模型只改文案，不改结构。".to_string(),
            }],
            risks: vec![tauri_app_lib::models::AnalysisPoint {
                title: "固定队收益更高".to_string(),
                reason: "陌生人局更容易丢节奏。".to_string(),
            }],
            dimension_reasons: vec![
                ("multiplayer_fun".to_string(), "多人反馈集中在高频互动。".to_string()),
            ],
        },
    );

    assert_eq!(patched.overview, "Hybrid narrative summary.");
    assert_eq!(patched.dimension_scores.len(), rule_report.dimension_scores.len());
    assert_eq!(patched.evidence, rule_report.evidence);
}

#[test]
fn assessment_adapter_reuses_report_summary_and_risks() {
    let report = build_rule_report(&fixture_game(), "2026-04-30T08:15:00Z".to_string())
        .expect("rule report");
    let assessment = summarize_report_as_assessment(&report);

    assert_eq!(assessment.appid, report.appid);
    assert_eq!(assessment.summary, report.overview);
    assert_eq!(assessment.score, report.overall_score);
    assert!(!assessment.risks.is_empty());
}
```

- [ ] **Step 2: Run the new Rust engine tests to confirm the failure**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test game_analysis_tests
```

Expected: FAIL because the new `game_analysis` module and `AnalysisNarrative` type do not exist yet.

- [ ] **Step 3: Implement the rule-report builder and assessment adapter**

Create `src-tauri/src/game_analysis.rs` with the public API below:

```rust
use crate::llm::{self, AnalysisNarrative, LlmRuntimeConfig};
use crate::models::{
    AnalysisConfidence, AnalysisDimensionScore, AnalysisEvidenceItem, AnalysisEvidenceKind,
    AnalysisPoint, AnalysisReviewEvidenceItem, AnalysisReviewStance, AnalysisSource,
    GameAnalysisReport, GameCard, AiAssessment,
};
use anyhow::{bail, Result};
use reqwest::Client;

pub fn build_rule_report(game: &GameCard, generated_at: String) -> Result<GameAnalysisReport> {
    if game.tags.is_empty()
        && game.multiplayer_modes.is_empty()
        && game.positive_review_pct.is_none()
        && game.total_reviews.is_none()
        && game.review_snippets.is_empty()
    {
        bail!("数据不足，暂时无法分析");
    }

    let dimension_scores = vec![
        score_approachability(game),
        score_multiplayer_fun(game),
        score_content_depth(game),
        score_reputation_stability(game),
        score_activity_health(game),
    ];
    let overall_score = dimension_scores.iter().map(|item| item.score).sum::<f64>() / 5.0;

    Ok(GameAnalysisReport {
        appid: game.appid,
        generated_at,
        source: AnalysisSource::Rule,
        confidence: derive_confidence(game),
        overall_score,
        overview: build_rule_overview(game),
        dimension_scores,
        strengths: build_strengths(game),
        risks: build_risks(game),
        evidence: build_evidence(game),
        review_evidence: build_review_evidence(game),
    })
}

pub fn apply_narrative_patch(
    mut report: GameAnalysisReport,
    narrative: AnalysisNarrative,
) -> GameAnalysisReport {
    report.source = AnalysisSource::Hybrid;
    report.overview = narrative.overview;
    report.strengths = narrative.strengths;
    report.risks = narrative.risks;

    for (key, reason) in narrative.dimension_reasons {
        if let Some(score) = report.dimension_scores.iter_mut().find(|item| item.key == key) {
            score.reason = reason;
        }
    }

    report
}

pub fn summarize_report_as_assessment(report: &GameAnalysisReport) -> AiAssessment {
    AiAssessment {
        appid: report.appid,
        score: report.overall_score,
        summary: report.overview.clone(),
        best_for: report.strengths.iter().map(|item| item.title.clone()).take(3).collect(),
        risks: report.risks.iter().map(|item| item.title.clone()).take(3).collect(),
    }
}
```

- [ ] **Step 4: Extend `llm.rs` with the constrained narrative parser**

Add a narrative-only parser and request path in `src-tauri/src/llm.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisNarrative {
    pub overview: String,
    pub strengths: Vec<AnalysisPoint>,
    pub risks: Vec<AnalysisPoint>,
    pub dimension_reasons: Vec<(String, String)>,
}

pub async fn generate_analysis_narrative(
    client: &Client,
    config: &LlmRuntimeConfig,
    game: &GameCard,
    rule_report: &GameAnalysisReport,
) -> Result<AnalysisNarrative> {
    let api_key = config
        .api_key
        .clone()
        .context("missing llm api key for hybrid narrative generation")?;

    let response = client
        .post(format!("{}/v1/chat/completions", config.base_url.trim_end_matches('/')))
        .bearer_auth(api_key)
        .json(&ChatRequest {
            model: config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You rewrite structured Steam analysis into strict JSON. Never invent facts.".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: build_analysis_prompt(game, rule_report),
                },
            ],
            temperature: 0.2,
        })
        .send()
        .await?
        .error_for_status()?
        .json::<ChatResponse>()
        .await?;

    let content = response
        .choices
        .first()
        .map(|choice| choice.message.content.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("{}");

    Ok(serde_json::from_str(content)?)
}
```

- [ ] **Step 5: Add the hybrid orchestration entry point and export the module**

Finish `src-tauri/src/game_analysis.rs` with the high-level async entry point, then export it from `src-tauri/src/lib.rs`:

```rust
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
```

And in `src-tauri/src/lib.rs`:

```rust
pub mod game_analysis;
```

- [ ] **Step 6: Re-run the Rust engine tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test game_analysis_tests
```

Expected: PASS with rule output, narrative patching, and legacy assessment adaptation covered.

- [ ] **Step 7: Commit the analysis engine**

```bash
git add src-tauri/src/game_analysis.rs src-tauri/src/llm.rs src-tauri/src/lib.rs src-tauri/tests/game_analysis_tests.rs
git commit -m "feat: add hybrid game analysis engine"
```

### Task 3: Expose Commands and Frontend Client APIs for Cached Reports

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/types.ts`
- Modify: `src/api/client.ts`
- Create: `src/api/client.test.ts`

- [ ] **Step 1: Write the failing browser-mode client cache tests**

Create `src/api/client.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { generateGameAnalysis, getGameAnalysis } from "./client";

describe("game analysis client", () => {
  it("creates and caches a browser-mode report", async () => {
    const first = await generateGameAnalysis(3744430);
    const cached = await getGameAnalysis(3744430);

    expect(first.overview.length).toBeGreaterThan(0);
    expect(cached?.overview).toBe(first.overview);
    expect(cached?.appid).toBe(3744430);
  });

  it("replaces the cached browser-mode report when forced", async () => {
    const first = await generateGameAnalysis(3087930);
    const second = await generateGameAnalysis(3087930, true);

    expect(second.generatedAt).not.toBe(first.generatedAt);
    expect(second.overview).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run the client test to confirm the failure**

Run:

```bash
npm run test -- src/api/client.test.ts
```

Expected: FAIL because `generateGameAnalysis` and `getGameAnalysis` do not exist yet.

- [ ] **Step 3: Mirror the shared report types in `src/types.ts`**

Add the TypeScript equivalents near `AiAssessment`:

```ts
export type AnalysisSource = "hybrid" | "rule";
export type AnalysisConfidence = "high" | "medium" | "low";
export type AnalysisEvidenceKind =
  | "positive_review_pct"
  | "total_reviews"
  | "current_players"
  | "tags"
  | "multiplayer_modes"
  | "short_description"
  | "review_snippet";
export type AnalysisReviewStance = "strength" | "risk";

export interface AnalysisDimensionScore {
  key:
    | "approachability"
    | "multiplayer_fun"
    | "content_depth"
    | "reputation_stability"
    | "activity_health";
  label: string;
  score: number;
  reason: string;
}

export interface AnalysisPoint {
  title: string;
  reason: string;
}

export interface AnalysisEvidenceItem {
  kind: AnalysisEvidenceKind;
  label: string;
  value: string;
  interpretation: string;
}

export interface AnalysisReviewEvidenceItem {
  stance: AnalysisReviewStance;
  quote: string;
  playtimeText: string;
  interpretation: string;
}

export interface GameAnalysisReport {
  appid: number;
  generatedAt: string;
  source: AnalysisSource;
  confidence: AnalysisConfidence;
  overallScore: number;
  overview: string;
  dimensionScores: AnalysisDimensionScore[];
  strengths: AnalysisPoint[];
  risks: AnalysisPoint[];
  evidence: AnalysisEvidenceItem[];
  reviewEvidence: AnalysisReviewEvidenceItem[];
}
```

- [ ] **Step 4: Add client wrappers and browser-mode cache helpers**

Extend `src/api/client.ts` with an in-memory mock cache and the new methods:

```ts
const mockGameAnalysisCache = new Map<number, GameAnalysisReport>();

function buildMockGameAnalysis(game: GameCard): GameAnalysisReport {
  const generatedAt = new Date().toISOString();
  return {
    appid: game.appid,
    generatedAt,
    source: "hybrid",
    confidence: game.reviewSnippets.length > 0 ? "high" : "medium",
    overallScore: game.aiScore ?? game.recommendationScore ?? 80,
    overview: game.aiSummary,
    dimensionScores: [
      {
        key: "approachability",
        label: "上手门槛",
        score: Math.min(96, (game.aiScore ?? game.recommendationScore ?? 80) + 4),
        reason: "浏览器预览模式：使用本地 mock 元数据估算。",
      },
      {
        key: "multiplayer_fun",
        label: "多人乐趣",
        score: Math.min(98, (game.aiScore ?? game.recommendationScore ?? 80) + 6),
        reason: "浏览器预览模式：使用多人模式和标签估算。",
      },
      {
        key: "content_depth",
        label: "内容耐玩度",
        score: Math.max(70, (game.aiScore ?? game.recommendationScore ?? 80) - 2),
        reason: "浏览器预览模式：使用评论量与简介估算。",
      },
      {
        key: "reputation_stability",
        label: "口碑稳定性",
        score: game.positiveReviewPct ?? 75,
        reason: "浏览器预览模式：直接映射好评率。",
      },
      {
        key: "activity_health",
        label: "活跃度健康度",
        score: Math.min(95, Math.max(60, Math.round(((game.currentPlayers ?? 0) / 50) + 60))),
        reason: "浏览器预览模式：直接映射当前在线规模。",
      },
    ],
    strengths: [{ title: "预览环境摘要", reason: game.aiSummary }],
    risks: [{ title: "未调用真实模型", reason: "当前结果来自浏览器预览 mock 逻辑。" }],
    evidence: [
      {
        kind: "positive_review_pct",
        label: "好评率",
        value: typeof game.positiveReviewPct === "number" ? `${Math.round(game.positiveReviewPct)}%` : "—",
        interpretation: "浏览器预览模式：直接使用本地卡片指标。",
      },
    ],
    reviewEvidence: game.reviewSnippets.slice(0, 1).map((snippet) => ({
      stance: snippet.votedUp ? "strength" : "risk",
      quote: snippet.review,
      playtimeText:
        typeof snippet.playtimeHours === "number" ? `${snippet.playtimeHours} 小时游玩` : "游玩时长未知",
      interpretation: "浏览器预览模式：直接引用本地评论摘录。",
    })),
  };
}

export async function getGameAnalysis(appid: number): Promise<GameAnalysisReport | null> {
  if (!isTauriRuntime()) {
    return mockGameAnalysisCache.get(appid) ?? null;
  }

  return invoke<GameAnalysisReport | null>("get_game_analysis", { appid });
}

export async function generateGameAnalysis(
  appid: number,
  forceRefresh = false,
): Promise<GameAnalysisReport> {
  if (!isTauriRuntime()) {
    if (!forceRefresh) {
      const cached = mockGameAnalysisCache.get(appid);
      if (cached) return cached;
    }

    const game = allMockGames().find((item) => item.appid === appid);
    if (!game) throw new Error(`未找到 Steam App ${appid}`);
    const report = buildMockGameAnalysis(game);
    mockGameAnalysisCache.set(appid, report);
    return report;
  }

  return invoke<GameAnalysisReport>("generate_game_analysis", {
    appid,
    forceRefresh,
  });
}
```

- [ ] **Step 5: Add the Rust commands and route the legacy assessment command through the report pipeline**

Update `src-tauri/src/commands.rs` and the `invoke_handler` list in `src-tauri/src/lib.rs`:

```rust
async fn load_or_generate_analysis(
    state: &AppState,
    appid: u32,
    force_refresh: bool,
) -> Result<crate::models::GameAnalysisReport, String> {
    let (game, config, cached_report) = {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        let game = db::load_game(&conn, appid)
            .map_err(to_command_error)?
            .ok_or_else(|| format!("未找到 Steam App {appid}"))?;
        let cached_report = if force_refresh {
            None
        } else {
            db::load_game_analysis(&conn, appid).map_err(to_command_error)?
        };
        let config = LlmRuntimeConfig {
            api_key: db::get_secret(&conn, "llm_api_key").map_err(to_command_error)?,
            base_url: db::get_config(&conn, "llm_base_url")
                .map_err(to_command_error)?
                .unwrap_or_else(|| "https://api.openai.com".to_string()),
            model: db::get_config(&conn, "llm_model")
                .map_err(to_command_error)?
                .unwrap_or_else(|| "gpt-4.1-mini".to_string()),
        };
        (game, config, cached_report)
    };

    if let Some(report) = cached_report {
        return Ok(report);
    }

    let generated_at = now_rfc3339().map_err(to_command_error)?;
    let report = crate::game_analysis::generate_game_analysis(
        &state.http,
        &config,
        &game,
        generated_at,
    )
    .await
    .map_err(to_command_error)?;

    {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        db::save_game_analysis(&conn, appid, &report).map_err(to_command_error)?;
        if let Some(mut existing) = db::load_game(&conn, appid).map_err(to_command_error)? {
            existing.ai_score = Some(report.overall_score);
            existing.ai_summary = report.overview.clone();
            existing.recommendation_score = db::score_card(&existing);
            db::upsert_game(&conn, &existing).map_err(to_command_error)?;
        }
    }

    Ok(report)
}

#[tauri::command]
pub fn get_game_analysis(
    state: State<'_, AppState>,
    appid: u32,
) -> Result<Option<crate::models::GameAnalysisReport>, String> {
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    db::load_game_analysis(&conn, appid).map_err(to_command_error)
}

#[tauri::command]
pub async fn generate_game_analysis(
    state: State<'_, AppState>,
    appid: u32,
    force_refresh: Option<bool>,
) -> Result<crate::models::GameAnalysisReport, String> {
    load_or_generate_analysis(&state, appid, force_refresh.unwrap_or(false)).await
}

#[tauri::command]
pub async fn assess_game_with_ai(
    state: State<'_, AppState>,
    appid: u32,
) -> Result<AiAssessment, String> {
    let report = load_or_generate_analysis(&state, appid, true).await?;
    Ok(crate::game_analysis::summarize_report_as_assessment(&report))
}
```

And add these registrations:

```rust
commands::get_game_analysis,
commands::generate_game_analysis,
```

- [ ] **Step 6: Re-run the client and Rust command-adjacent checks**

Run:

```bash
npm run test -- src/api/client.test.ts
```

Expected: PASS with browser-mode analysis caching covered.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test game_analysis_tests --test game_metadata_tests
```

Expected: PASS with the report types, persistence, and orchestration compiling together.

- [ ] **Step 7: Commit the command/client surface**

```bash
git add src/types.ts src/api/client.ts src/api/client.test.ts src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: expose cached game analysis APIs"
```

### Task 4: Upgrade the Detail Page to Load, Generate, and Refresh Reports

**Files:**
- Create: `src/pages/detail/useGameAnalysis.ts`
- Create: `src/pages/detail/GameAnalysisPanel.tsx`
- Modify: `src/pages/detail/DetailPage.tsx`
- Modify: `src/pages/detail/DetailPage.test.tsx`
- Modify: `src/App.css`

- [ ] **Step 1: Add failing detail-page tests for cached load, auto-generation, and refresh**

Update `src/pages/detail/DetailPage.test.tsx` to mock the new client methods and add these cases:

```tsx
const getGameAnalysisMock = vi.fn();
const generateGameAnalysisMock = vi.fn();

vi.mock("../../api/client", async () => {
  const actual = await vi.importActual<typeof import("../../api/client")>("../../api/client");
  return {
    ...actual,
    getGameAnalysis: (...args: unknown[]) => getGameAnalysisMock(...args),
    generateGameAnalysis: (...args: unknown[]) => generateGameAnalysisMock(...args),
  };
});

function buildReport(appid: number) {
  return {
    appid,
    generatedAt: "2026-04-30T08:30:00Z",
    source: "hybrid" as const,
    confidence: "high" as const,
    overallScore: 91,
    overview: "这是一份缓存命中的完整证据型分析。",
    dimensionScores: [
      { key: "multiplayer_fun", label: "多人乐趣", score: 93, reason: "队伍反馈集中在配合节奏。" },
      { key: "approachability", label: "上手门槛", score: 87, reason: "教程和反馈都比较明确。" },
      { key: "content_depth", label: "内容耐玩度", score: 86, reason: "评论样本支持反复开黑。" },
      { key: "reputation_stability", label: "口碑稳定性", score: 94, reason: "好评率和评论量都健康。" },
      { key: "activity_health", label: "活跃度健康度", score: 90, reason: "当前在线规模足够支撑匹配。" },
    ],
    strengths: [{ title: "合作节奏稳", reason: "玩家评价集中在沟通顺畅。" }],
    risks: [{ title: "固定队收益更高", reason: "路人局容错略低。" }],
    evidence: [{ kind: "positive_review_pct", label: "好评率", value: "97%", interpretation: "口碑基础扎实。" }],
    reviewEvidence: [{ stance: "strength", quote: "联机节奏非常顺。", playtimeText: "12 小时游玩", interpretation: "支撑多人乐趣判断。" }],
  };
}

it("renders a cached report without auto-generating again", async () => {
  const game = buildGame();
  getGameAnalysisMock.mockResolvedValue(buildReport(game.appid));
  generateGameAnalysisMock.mockResolvedValue(buildReport(game.appid));

  render(
    <DetailPage
      game={game}
      relatedGames={buildRelatedGames()}
      isBusy={false}
      onBack={vi.fn()}
      onToggleState={vi.fn()}
    />,
  );

  expect(await screen.findByText("这是一份缓存命中的完整证据型分析。")).toBeInTheDocument();
  expect(generateGameAnalysisMock).not.toHaveBeenCalled();
});

it("auto-generates the first report when no cache exists", async () => {
  const game = buildGame();
  getGameAnalysisMock.mockResolvedValue(null);
  generateGameAnalysisMock.mockResolvedValue(buildReport(game.appid));

  render(
    <DetailPage
      game={game}
      relatedGames={buildRelatedGames()}
      isBusy={false}
      onBack={vi.fn()}
      onToggleState={vi.fn()}
    />,
  );

  expect(await screen.findByText("这是一份缓存命中的完整证据型分析。")).toBeInTheDocument();
  expect(generateGameAnalysisMock).toHaveBeenCalledWith(game.appid, false);
});

it("forces a refresh when clicking 重新 AI 评估", async () => {
  const game = buildGame();
  getGameAnalysisMock.mockResolvedValue(buildReport(game.appid));
  generateGameAnalysisMock.mockResolvedValue(buildReport(game.appid));

  render(
    <DetailPage
      game={game}
      relatedGames={buildRelatedGames()}
      isBusy={false}
      onBack={vi.fn()}
      onToggleState={vi.fn()}
    />,
  );

  await screen.findByText("这是一份缓存命中的完整证据型分析。");
  fireEvent.click(screen.getByRole("button", { name: "重新 AI 评估" }));

  expect(generateGameAnalysisMock).toHaveBeenLastCalledWith(game.appid, true);
});
```

- [ ] **Step 2: Run the detail-page test file to confirm the failure**

Run:

```bash
npm run test -- src/pages/detail/DetailPage.test.tsx
```

Expected: FAIL because `DetailPage` still requires `onAiAssess`, does not load cached reports, and does not render the new report structure.

- [ ] **Step 3: Implement the detail-page analysis hook**

Create `src/pages/detail/useGameAnalysis.ts`:

```ts
import { useEffect, useState } from "react";
import { generateGameAnalysis, getGameAnalysis } from "../../api/client";
import type { GameAnalysisReport, GameCard } from "../../types";

export interface DetailAnalysisState {
  report: GameAnalysisReport | null;
  loading: boolean;
  error: string | null;
  expanded: boolean;
}

const initialState: DetailAnalysisState = {
  report: null,
  loading: true,
  error: null,
  expanded: false,
};

export function useGameAnalysis(game: GameCard) {
  const [state, setState] = useState<DetailAnalysisState>(initialState);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setState(initialState);
      try {
        const cached = await getGameAnalysis(game.appid);
        if (cancelled) return;
        if (cached) {
          setState({ report: cached, loading: false, error: null, expanded: false });
          return;
        }

        const generated = await generateGameAnalysis(game.appid, false);
        if (cancelled) return;
        setState({ report: generated, loading: false, error: null, expanded: false });
      } catch (error) {
        if (cancelled) return;
        setState({
          report: null,
          loading: false,
          error: error instanceof Error ? error.message : String(error),
          expanded: false,
        });
      }
    }

    void load();

    return () => {
      cancelled = true;
    };
  }, [game.appid]);

  async function refresh() {
    setState((current) => ({ ...current, loading: true, error: null }));
    try {
      const report = await generateGameAnalysis(game.appid, true);
      setState((current) => ({ ...current, report, loading: false, error: null }));
    } catch (error) {
      setState((current) => ({
        ...current,
        loading: false,
        error: error instanceof Error ? error.message : String(error),
      }));
    }
  }

  function toggleExpanded() {
    setState((current) => ({ ...current, expanded: !current.expanded }));
  }

  return { state, refresh, toggleExpanded };
}
```

- [ ] **Step 4: Implement the report renderer and wire it into `DetailPage.tsx`**

Create `src/pages/detail/GameAnalysisPanel.tsx` and replace the current placeholder AI tab content in `DetailPage.tsx`:

```tsx
import type { GameAnalysisReport } from "../../types";

export function GameAnalysisPanel({
  report,
  loading,
  error,
  expanded,
  onToggleExpanded,
  onRefresh,
}: {
  report: GameAnalysisReport | null;
  loading: boolean;
  error: string | null;
  expanded: boolean;
  onToggleExpanded: () => void;
  onRefresh: () => void;
}) {
  if (loading && !report) {
    return (
      <div className="ai-eval-panel ai-analysis-loading">
        <h3>AI 评估</h3>
        <p>正在生成首份证据型分析报告……</p>
      </div>
    );
  }

  if (error && !report) {
    return (
      <div className="ai-eval-panel ai-analysis-error">
        <h3>AI 评估</h3>
        <p>{error}</p>
        <button className="gold-button" type="button" onClick={onRefresh}>
          重新 AI 评估
        </button>
      </div>
    );
  }

  if (!report) return null;

  return (
    <div className="ai-eval-panel ai-analysis-report">
      <div className="analysis-summary-card">
        <div className="analysis-summary-head">
          <div>
            <h3>AI 评估</h3>
            <p>{report.overview}</p>
          </div>
          <div className="analysis-summary-badges">
            <span>{report.source === "hybrid" ? "混合分析" : "基础分析"}</span>
            <span>{report.confidence === "high" ? "高置信度" : report.confidence === "medium" ? "中置信度" : "低置信度"}</span>
          </div>
        </div>

        <div className="analysis-score-grid">
          {report.dimensionScores.map((item) => (
            <article key={item.key}>
              <strong>{item.label}</strong>
              <span>{Math.round(item.score)}</span>
              <p>{item.reason}</p>
            </article>
          ))}
        </div>

        <div className="analysis-actions">
          <button className="muted-button" type="button" onClick={onToggleExpanded}>
            {expanded ? "收起完整分析" : "查看完整分析"}
          </button>
          <button className="gold-button" disabled={loading} type="button" onClick={onRefresh}>
            {loading ? "AI 评估中…" : "重新 AI 评估"}
          </button>
        </div>
      </div>

      {expanded && (
        <div className="analysis-expanded-grid">
          <section>
            <h4>亮点</h4>
            {report.strengths.map((item) => (
              <article key={item.title}>
                <strong>{item.title}</strong>
                <p>{item.reason}</p>
              </article>
            ))}
          </section>
          <section>
            <h4>风险</h4>
            {report.risks.map((item) => (
              <article key={item.title}>
                <strong>{item.title}</strong>
                <p>{item.reason}</p>
              </article>
            ))}
          </section>
          <section>
            <h4>结构化证据</h4>
            {report.evidence.map((item) => (
              <article key={`${item.kind}-${item.label}`}>
                <strong>{item.label}</strong>
                <em>{item.value}</em>
                <p>{item.interpretation}</p>
              </article>
            ))}
          </section>
          <section>
            <h4>评论摘录证据</h4>
            {report.reviewEvidence.map((item, index) => (
              <article key={`${item.stance}-${index}`}>
                <strong>{item.stance === "strength" ? "亮点支撑" : "风险支撑"}</strong>
                <em>{item.playtimeText}</em>
                <blockquote>{item.quote}</blockquote>
                <p>{item.interpretation}</p>
              </article>
            ))}
          </section>
        </div>
      )}
    </div>
  );
}
```

Then in `src/pages/detail/DetailPage.tsx`, remove the `onAiAssess` prop, call `useGameAnalysis(game)`, and render the new panel inside the `AI 评估` tab instead of the old static bar list.

- [ ] **Step 5: Add the report styles and re-run the detail tests**

Add the corresponding styles to `src/App.css`:

```css
.ai-analysis-report,
.analysis-summary-card,
.analysis-expanded-grid section {
  background: #fff8ea;
  border: 1px solid var(--line);
  border-radius: 10px;
}

.analysis-summary-card {
  display: grid;
  gap: 16px;
  padding: 18px;
}

.analysis-summary-head {
  display: flex;
  gap: 16px;
  justify-content: space-between;
}

.analysis-summary-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.analysis-summary-badges span {
  background: #fff2c7;
  border-radius: 999px;
  color: #8a6200;
  font-size: 12px;
  padding: 4px 10px;
}

.analysis-score-grid,
.analysis-expanded-grid {
  display: grid;
  gap: 12px;
}

.analysis-score-grid {
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
}

.analysis-expanded-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  margin-top: 16px;
}

.analysis-actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}
```

Run:

```bash
npm run test -- src/pages/detail/DetailPage.test.tsx
```

Expected: PASS with cached rendering, first-load generation, and forced refresh behavior covered.

- [ ] **Step 6: Commit the detail-page upgrade**

```bash
git add src/pages/detail/useGameAnalysis.ts src/pages/detail/GameAnalysisPanel.tsx src/pages/detail/DetailPage.tsx src/pages/detail/DetailPage.test.tsx src/App.css
git commit -m "feat: add cached ai analysis detail panel"
```

### Task 5: Reconcile the App Shell and Run Regression Verification

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.test.tsx`

- [ ] **Step 1: Add a failing App regression for opening detail view with cached analysis**

Update `src/App.test.tsx` to mock the new client calls and add a regression:

```tsx
const getGameAnalysisMock = vi.fn();
const generateGameAnalysisMock = vi.fn();
const assessGameWithAiMock = vi.fn();

vi.mock("./api/client", async () => {
  const actual = await vi.importActual<typeof import("./api/client")>("./api/client");

  return {
    ...actual,
    assessGameWithAi: (...args: unknown[]) => assessGameWithAiMock(...args),
    getDashboard: () => getDashboardMock(),
    getGameAnalysis: (...args: unknown[]) => getGameAnalysisMock(...args),
    generateGameAnalysis: (...args: unknown[]) => generateGameAnalysisMock(...args),
    previewSteamAppList: vi.fn(),
    saveConfig: vi.fn(),
    setGameUserState: vi.fn(),
    syncSeedGames: (...args: unknown[]) => syncSeedGamesMock(...args),
  };
});

function buildAnalysisReport(appid: number) {
  return {
    appid,
    generatedAt: "2026-04-30T08:45:00Z",
    source: "hybrid" as const,
    confidence: "high" as const,
    overallScore: 92,
    overview: "打开详情页后应直接显示缓存分析。",
    dimensionScores: [
      { key: "approachability", label: "上手门槛", score: 88, reason: "回归测试用例。" },
      { key: "multiplayer_fun", label: "多人乐趣", score: 94, reason: "回归测试用例。" },
      { key: "content_depth", label: "内容耐玩度", score: 86, reason: "回归测试用例。" },
      { key: "reputation_stability", label: "口碑稳定性", score: 95, reason: "回归测试用例。" },
      { key: "activity_health", label: "活跃度健康度", score: 90, reason: "回归测试用例。" },
    ],
    strengths: [{ title: "缓存可见", reason: "详情页直接显示摘要。" }],
    risks: [{ title: "无", reason: "纯回归夹具。" }],
    evidence: [{ kind: "positive_review_pct", label: "好评率", value: "97%", interpretation: "回归测试夹具。" }],
    reviewEvidence: [],
  };
}

it("opens a dashboard game card into detail view and shows cached analysis", async () => {
  const dashboard = buildDashboard();
  getDashboardMock.mockResolvedValue(dashboard);
  getGameAnalysisMock.mockResolvedValue(buildAnalysisReport(dashboard.newGames[0].appid));
  generateGameAnalysisMock.mockResolvedValue(buildAnalysisReport(dashboard.newGames[0].appid));

  render(<App />);
  await screen.findByRole("heading", { name: "新游区" });

  fireEvent.click(screen.getByRole("button", { name: /Together Moon Escape/i }));

  expect(await screen.findByText("打开详情页后应直接显示缓存分析。")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the App regression to confirm the failure**

Run:

```bash
npm run test -- src/App.test.tsx
```

Expected: FAIL because `App` still passes the old `onAiAssess` prop into `DetailPage`, and the test mocks do not yet match the updated detail-page data flow.

- [ ] **Step 3: Update the App shell to match the new detail-page contract**

In `src/App.tsx`, remove the detail-page `onAiAssess` prop and keep the legacy AI assistant path routed through `handleAiAssess(game)`:

```tsx
{activeView === "detail" && selectedGame && (
  <DetailPage
    game={selectedGame}
    isBusy={isBusy}
    onBack={() => setActiveView("home")}
    onToggleState={(patch, message) =>
      handleUserState(selectedGame.appid, patch, message)
    }
    relatedGames={allGames.filter((game) => game.appid !== selectedGame.appid)}
  />
)}

{activeView === "ai" && (
  <AiAssistantPage
    assessment={assessment}
    games={[...visibleNewGames, ...visibleClassics].slice(0, 4)}
    isBusy={isBusy}
    onAssess={(game) => {
      setSelectedGame(game);
      void handleAiAssess(game);
    }}
    selectedGame={selectedGame}
  />
)}
```

The important part is that detail-page analysis becomes local to `DetailPage`, while the existing AI assistant page keeps using the old `assessment` state until the second-phase recommendation work.

- [ ] **Step 4: Run the focused regression suite plus build verification**

Run:

```bash
npm run test -- src/api/client.test.ts src/pages/detail/DetailPage.test.tsx src/App.test.tsx
```

Expected: PASS with browser-mode caching, detail-page report behavior, and App-level dashboard-to-detail navigation covered.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test game_metadata_tests --test game_analysis_tests
```

Expected: PASS with persistence and engine coverage intact.

Run:

```bash
npm run build
```

Expected: PASS with the updated TypeScript API, detail-page hook, and renderer compiling together.

- [ ] **Step 5: Commit the integration sweep**

```bash
git add src/App.tsx src/App.test.tsx
git commit -m "feat: integrate cached ai analysis into detail flow"
```

## Spec Coverage Check

- Cached latest full report: covered by Task 1 persistence and Task 3 command/client wiring.
- Rule + hybrid generation with LLM fallback: covered by Task 2.
- Separate cache read vs generate commands: covered by Task 3.
- Detail-page summary + expandable full report: covered by Task 4.
- First-open auto generation + manual refresh: covered by Task 4 tests and hook.
- Preserve legacy lightweight `aiSummary` / `aiScore` and `AiAssessment`: covered by Task 2 adapter and Task 3 command update.
- Keep the App shell and existing AI assistant page working: covered by Task 5.

## Placeholder Scan

- No `TODO`, `TBD`, “implement later”, or “similar to above” placeholders remain.
- Every task includes exact file paths, commands, and code snippets.
- Test commands are scoped to the exact files added or modified in each task.

## Type Consistency Check

- Rust uses `snake_case` enums and `camelCase` serde payloads for `GameAnalysisReport`, matching the TypeScript interfaces in `src/types.ts`.
- The public API names stay consistent across layers:
  - Rust commands: `get_game_analysis`, `generate_game_analysis`
  - TS client wrappers: `getGameAnalysis`, `generateGameAnalysis`
  - UI state: `DetailAnalysisState`
- The legacy summary path consistently adapts through `summarize_report_as_assessment(report)` instead of inventing a second report-to-summary mapping.
