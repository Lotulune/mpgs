# AI Evaluation V2 Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current AI recommendation evaluation flow with the approved V2 scoring model while preserving compatibility for existing dashboard and detail experiences.

**Architecture:** Introduce a dedicated Rust `scoring` module that normalizes Steam signals and computes V2 quality, recommendation, confidence, risk, and pool outputs. Keep legacy fields for compatibility, persist V2 summary fields onto `GameCard`, and make the detail UI show the new semantics while list cards gracefully fall back for legacy data.

**Tech Stack:** Rust, Tauri, SQLite, TypeScript, React, Vitest

---

### Task 1: Backend V2 Scoring Core

**Files:**
- Create: `src-tauri/src/scoring/mod.rs`
- Create: `src-tauri/src/scoring/signals.rs`
- Create: `src-tauri/src/scoring/normalize.rs`
- Create: `src-tauri/src/scoring/review_quality.rs`
- Create: `src-tauri/src/scoring/multiplayer_fit.rs`
- Create: `src-tauri/src/scoring/activity_health.rs`
- Create: `src-tauri/src/scoring/content_depth.rs`
- Create: `src-tauri/src/scoring/accessibility.rs`
- Create: `src-tauri/src/scoring/discovery_value.rs`
- Create: `src-tauri/src/scoring/risk.rs`
- Create: `src-tauri/src/scoring/confidence.rs`
- Create: `src-tauri/src/scoring/aggregate.rs`
- Test: `src-tauri/tests/game_analysis_tests.rs`

- [ ] Add failing tests that expect six V2 dimensions, numeric confidence, recommendation/quality split, and multilingual normalization behavior.
- [ ] Run `cargo test game_analysis --manifest-path src-tauri/Cargo.toml` and confirm the new assertions fail for the expected missing V2 behavior.
- [ ] Implement the scoring module and wire `build_rule_report()` to use V2 scoring output instead of the old five-dimension averaging logic.
- [ ] Re-run `cargo test game_analysis --manifest-path src-tauri/Cargo.toml` and confirm the targeted tests pass.

### Task 2: Persistence And Command Integration

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/recommendation_tests.rs`
- Test: `src-tauri/src/commands.rs`

- [ ] Add failing tests for the lightweight proxy recommendation score, removed `ai_score ?? 72` fallback, persisted V2 summary fields, and command writeback behavior.
- [ ] Run `cargo test recommendation --manifest-path src-tauri/Cargo.toml` and `cargo test commands --manifest-path src-tauri/Cargo.toml` to verify the expected failures.
- [ ] Add V2 fields to the Rust models and SQLite schema, update card load/save code, write V2 summary values after analysis generation, and update proxy recommendation scoring.
- [ ] Re-run the Rust recommendation and command tests and confirm they pass.

### Task 3: Frontend Types, Proxy Scoring, And Display Semantics

**Files:**
- Modify: `src/types.ts`
- Modify: `src/domain/recommendation.ts`
- Modify: `src/domain/recommendation.test.ts`
- Modify: `src/features/library/gameScoreDisplay.ts`
- Modify: `src/features/library/gameDashboardState.ts`
- Modify: `src/features/library/gameDashboardState.test.ts`
- Modify: `src/pages/detail/GameAnalysisPanel.tsx`
- Modify: `src/pages/detail/DetailPage.tsx`
- Modify: `src/pages/detail/DetailPage.test.tsx`
- Modify: `src/App.tsx`
- Modify: `src/api/client.ts`

- [ ] Add failing Vitest assertions for the new proxy recommendation behavior, V2 card labels, dashboard snapshot propagation, and detail-panel V2 score presentation.
- [ ] Run `npm test -- --runInBand recommendation gameDashboardState DetailPage` and verify the failures are caused by missing V2 fields or old labels.
- [ ] Update TypeScript models, client mocks, dashboard snapshot handling, score display logic, and detail UI copy to reflect V2 recommendation vs. quality semantics.
- [ ] Re-run the targeted Vitest suite and confirm it passes.

### Task 4: Full Verification

**Files:**
- Verify only

- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `npm test`.
- [ ] Run `npm run build`.
- [ ] Review changed files for compatibility risks and summarize any remaining limitations before reporting completion.
