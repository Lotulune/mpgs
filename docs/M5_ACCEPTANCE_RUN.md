# M5 acceptance run

- When: 2026-07-16 21:15:38 +08:00
- Result: PASS
- Git commit: `63aa54b8d0e42d6abf6c61837c57cccdebfa6804`
- Git worktree dirty: `True`
- Acceptance script SHA-256: `61e53d6024d8381c227c569480995517b663c7e322e8b5e89b8cae9dc105ed59`
- Live AI check requested: `False`
- Passed: 10 / 10

| ID | OK | Detail |
| --- | --- | --- |
| unit.ai_storage | yes | mpgs-ai + mpgs-storage tests passed |
| build.tools | yes | mpgs-server + mpgs-dbtool built |
| server.start | yes | temporary server on http://127.0.0.1:18673 |
| meta.ai_disabled | yes | ai_available=False |
| feed.without_ai | yes | items=5 |
| nl.fallback | yes | ai_status=fallback items=5 |
| retrieval.sync | yes | sync-retrieval completed |
| offline.features | yes | extract-offline-features completed |
| embed.batch | yes | embed-documents (hash) completed |
| live.ai.not_requested | yes | pass -LiveAi with MPGS_AI_API_KEY for live provider check |

This run proves offline M5 exit conditions (disabled AI safety, retrieval/embed/offline features, NL fallback).
Live provider success requires an API key and is optional.
