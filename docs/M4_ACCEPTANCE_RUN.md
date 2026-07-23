# M4 acceptance run

- When: 2026-07-16 14:26:11 +08:00
- Result: PASS
- Base: temporary-loopback-server
- Git commit: 3a0a6b192fc6b2326fd14c5b90c84b07cbfaac31
- Git worktree dirty: true
- Acceptance script SHA-256: e58aeb53a5962ec95e5836deeb6152674ad7ca5ea680162ef95fb89582d1e3f9
- Server binary SHA-256: 1a1628e7dbb05d578d0567446c2784b74568f392bc5458f51b6fe972811b3879
- API / service / algorithm: v1 / 0.1.0 / rules-0.2.0
- Passed: 30 / 30 (failed: 0)

| ID | OK | Detail |
| --- | --- | --- |
| server.start | yes | temporary local server started pid=35296 |
| health.live | yes | status=200 |
| health.ready | yes | status=200 |
| meta.sections | yes | sections=recent_release,upcoming,popular_legacy,classic_legacy |
| meta.versions | yes | api=v1 service=0.1.0 algorithm=rules-0.2.0 |
| meta.ai_provider_state | yes | ai_available=False |
| session.anonymous | yes | access_token issued |
| prefs.put | yes | version=2 party=4 |
| feed.recent_release | yes | items=1 |
| feed.upcoming | yes | items=1 |
| feed.popular_legacy | yes | items=7 |
| feed.classic_legacy | yes | items=5 |
| feed.reasons | yes | total_items=14 items_without_reasons=0 |
| natural_language.constraints | yes | party=3 session_max=60 coop_competitive=0.1 |
| natural_language.candidates | yes | items=5 items_without_reasons=0 |
| natural_language.fallback | yes | ai_status=fallback fallback_reason=AI provider is not configured; deterministic recommendations are shown |
| feedback.like | yes | app_id=2500002 feedback_id=1 status=201 |
| feedback.feed_effect | yes | app_id=2500002 baseline_score=0.674171466590796 active_score=0.699171466590796 present=True |
| feedback.undo | yes | feedback_id=2 original_feedback_id=1 status=200 |
| feedback.feed_restored | yes | app_id=2500002 baseline_score=0.674171466590796 restored_score=0.674171466590796 present=True |
| calendar.get | yes | recent=1 upcoming=1 dated=2 undated=0 |
| calendar.state_filters | yes | recent_mismatch=0 upcoming_mismatch=0 |
| calendar.early_data_honesty | yes | invalid_items=0 invalid_dated_items=0 review_total_and_early_data=present |
| search.name | yes | status=200 hits=1 |
| games.detail | yes | app_id=2500002 name=Recent Co-op Sample |
| etag.revalidate | yes | status=304 etag=W/"d71b37dcd5d94a9e" |
| web.offline_contract | yes | ETag, offline snapshot, and durable pending-write tests passed |
| web.vitest | yes | pnpm --filter lobbytally-web test exit 0 |
| web.build | yes | typecheck+vite build ok |
| desktop.cargo_check | yes | apps/desktop/src-tauri cargo check ok |
