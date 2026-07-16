# M5 acceptance run

- When: 2026-07-17 03:35:56 +08:00
- Result: FAIL
- Git commit: `39008e92ddafdaabc4162c00301da0905bf18657`
- Git worktree dirty: `True`
- Acceptance script SHA-256: `cb70afb4d1320b6b20703eccd1cd478043438ae69288dff47150cfb4c6661013`
- Live AI check requested: `False`
- Passed: 10 / 11

| ID | OK | Detail |
| --- | --- | --- |
| source.clean | no | git_worktree_dirty=True |
| unit.ai_storage_server | yes | mpgs-ai + mpgs-storage + mpgs-server tests passed |
| build.tools | yes | mpgs-server + mpgs-dbtool built |
| server.start | yes | temporary server on http://127.0.0.1:18171 |
| meta.ai_disabled | yes | ai_available=False |
| feed.without_ai | yes | items=5 |
| embed.batch | yes | targets=22 written=22 |
| nl.fallback | yes | ai_status=fallback items=5 |
| retrieval.sync | yes | sync-retrieval completed |
| offline.features | yes | extract-offline-features completed |
| live.ai.not_requested | yes | pass -LiveAi with MPGS_AI_API_KEY for live provider check |

This run does not close M5; inspect the failed checks above.
Live provider success requires an API key and is optional.
