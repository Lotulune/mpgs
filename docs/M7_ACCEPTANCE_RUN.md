# M7 本地验收运行记录

- 时间：2026-07-17
- Git commit：`4faa878d89b6af0b69068bc224fb844e6d62817c`
- 工作树：`dirty=true`（本次 M7 实现及用户已有未提交修改均保留在工作树中）
- 历史运行 SQLite schema：`9`；当前源码最新 schema：`14`
- 服务端构建：隔离目标目录 `target/m7-smoke-build-final`

## 通过的检查

| 检查 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS：AI 20、dbtool 4、domain 6、recommender 10、服务端 40（另有 1 个既有手动性能测试 IGNORE）、Steam 36、存储 44 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` | PASS：1 |
| `pnpm --dir web typecheck` | PASS |
| `pnpm --dir web test` | PASS：59 |
| `pnpm --dir web build` | PASS |
| 持久化 API 冒烟 | PASS：6 个独立账号、`Votes=6`、`PreviewCount=5`、`OmittedCount=1`；schema 9 记录为历史证据 |
| 头像审核 API 冒烟 | PASS：屏蔽后 `image/svg+xml`，解除后 `image/webp` |
| 数据/CORS 冒烟 | PASS：5 个调度任务、`OPTIONS /v1/me/avatar = 204` |
| 浏览器交互 | PASS：同源代理下验证演示标识、社区筛选、桌面 `5 + 1`、窄屏 `3 + 3`、账户菜单和 AI 设置 DOM |

`git diff --check` 未报告空白错误；Git 仅输出工作树既有的 LF/CRLF 提示。

## 真实数据烟测（不计入发布通过）

- `IStoreService/GetAppList`：成功请求两页、每页 25 条，并在重构后的二进制中额外续传一条；目录游标依次为 `1250`、`2420`、`2450`，说明游标已持久化。
- SQLite：`schema_version=9`、`integrity_check=["ok"]`、`ready=ok`；数据库二进制扫描未发现 Web API Key 明文。
- 多人候选：`collect-steam-candidates` 初始得到 `2,071` 个规范化候选；最终 `m3-audit` 为 `2,091`，`m3_catalog_gate=ok`。
- 人工画像和动态富化：导入 50 条黄金画像（其中 14 条可信熟人联机画像）；最终真实快照为平台 `10`、价格 `9`、评测 `10`、CCU `10`。失败目标保留待重试状态，未清空既有快照。
- 调度与 worker：独立临时服务成功入队目录、候选和富化作业；同机 worker 领取目录作业并写入 1,000 条应用。随后 `/admin/v1/data-status` 返回 `catalog_sync` 的 `last_success_at_ms`、`next_run_at_ms`、持久化游标和覆盖率，临时服务已停止。
- DATA-206 可执行审计：`m7-data-audit data/m7-live-smoke.db` 按预期返回非零退出。候选为 `2091/2000`、封面 `2075/2091`（`99.2%`），但可信熟人联机画像仅 `14/300`、日期 `10/2091`（`0.5%`）、四分区均为 `0`，且连续 7 天评价/CCU 均为 `0`；因此没有将这次烟测误报为可发布。

## 未关闭的发布门禁

- DATA-203 至 DATA-206 的持续 worker 部署、完整真实数据库覆盖率、连续 7 天趋势和发布批准。一次真实烟测不能关闭这些门禁；`m7-data-audit` 当前明确拒绝该真实库。
- 真实 Provider 凭据下的 AI 连通性与成本负责人确认。
- 真实数据库备份恢复、跨平台发布包安装、代码签名和人工隐私/头像内容政策签字。
- 浏览器自动化后端的截图接口超时；本运行使用 DOM 状态验证响应式规则，未将截图列为 PASS 证据。
