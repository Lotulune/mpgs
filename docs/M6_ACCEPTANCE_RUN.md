# M6 acceptance run

- When: 2026-07-17 07:43:11 +08:00
- Result: FAIL
- Git commit: `4faa878d89b6af0b69068bc224fb844e6d62817c`
- Git worktree dirty: `True`
- Acceptance script SHA-256: `8e37c7caf14806ef617223b1ddcc85eaea52c9f0c72f3cd151c0fc105fbd8d09`
- Package built: `True`
- Package path: `C:\Users\Administrator\AppData\Local\Temp\mpgs-m6-c9315a25872e498b9fb5476b925c9555\dist\mpgs-server-windows-x64-0.1.0+4faa878`
- Passed: 12 / 14

| ID | OK | Detail |
| --- | --- | --- |
| source.clean | no | git_worktree_dirty=True |
| layout.files | yes | all 17 required paths present |
| licenses.generated | yes | bytes=11998 regenerated_diff= |
| unit.storage_upgrade_backup | yes | upgrade path + backup/restore tests passed |
| unit.server_m6 | yes | meta provenance + soak + fault tests passed |
| performance.feed_p95 | yes | 2,000-game uncached + ETag P95 gate passed |
| build.tools | yes | sha=4faa878d89b6af0b69068bc224fb844e6d62817c target=x86_64-pc-windows-msvc schema=7 algorithm=rules-0.2.0 |
| runtime.ready | yes | url=http://127.0.0.1:19098/health/ready live+ready=200 |
| runtime.meta_provenance | yes | service=0.1.0 algo=rules-0.2.0 schema=7 git=4faa878d89b6af0b69068bc224fb844e6d62817c data_ms=1784245384675 |
| runtime.feed | yes | status=200 items=3 |
| runtime.nl_fallback | yes | status=200 ai_status=fallback body={"ai_status":"fallback","ai_summary":null,"ai_summary_evidence_ids":[],"algorithm_version":"rules-0.2.0","data_updated_a |
| runtime.process_soak | yes | duration_seconds=1 requests=16 process_alive=True |
| ops.backup_restore | yes | backup+restore+integrity ok |
| package.provenance | no | path=C:\Users\Administrator\AppData\Local\Temp\mpgs-m6-c9315a25872e498b9fb5476b925c9555\dist\mpgs-server-windows-x64-0.1.0+4faa878 git=4faa878d89b6af0b69068bc224fb844e6d62817c target=x86_64-pc-windows-msvc source_dirty=True checksums=True |

This run does not close M6; inspect the failed checks above.
Code signing, notarization, and production compliance signatures remain human gates (see SIGNING_AND_UPDATES.md / PRIVACY.md).
