# M7 本地验收记录

日期：2026-07-17
范围：`docs/PRD_0.2.md` 的 M7 账号、社区、头像、AI 设置和数据运行时要求。

这是一份当前工作树的本地工程验收记录，不是生产发布批准。已完成一次真实 Steam 数据和租约 worker 烟测，但持续 worker 部署、完整覆盖率、跨平台发布包和人工合规签字仍必须单独完成后才能关闭 M7 发布门禁。

## 已交付的本地实现

- `0008_m7_accounts_community_ai.sql` 至 `0014_device_local_ai_mode.sql` 将账号、会话、头像元数据/审核、社区投票、AI 模式/用量和数据运行状态纳入 SQLite 迁移；当前源码最新 schema 为 `14`。
- 账号采用 Argon2id、短期访问令牌和可轮换刷新令牌；刷新令牌复用会撤销该账号全部会话。匿名偏好、反馈和投票在注册/登录时事务化合并。
- 账户写操作（偏好、反馈、投票、头像、AI 配置）要求已登录账号；公开社区响应只给显示名称和不透明头像 URL。
- Tauri 将会话令牌迁移至操作系统安全凭据存储，客户端 SQLite 不再保存会话令牌。
- 头像仅接受 JPEG/PNG/WebP，限制 2 MiB，服务端实际解码、居中裁剪、重编码为 `128x128` WebP；替换和删除均清理对应存储对象。管理员可按内容哈希屏蔽头像，保留理由和审计事件；解除屏蔽或替换头像会刷新版本。
- “大家想玩”独立页面支持趋势/总票排序、游标、ETag、单账号单票和稳定的头像优先级；支持发售状态、Demo、平台和人数筛选。桌面最多显示 5 个头像、窄屏最多 3 个并显示 `+N`。
- AI 设置支持内置、自定义 OpenAI-compatible 与关闭模式。自定义 endpoint 仅允许 HTTPS 公网地址并禁止重定向；自定义密钥仅保存在设备凭据库/浏览器标签页会话，服务端拒绝持久化。额度按账号持久化计数，并叠加每账号并发限制和全局网关预算。
- 非 release 开发环境可使用空内存库；演示数据只能通过显式 `MPGS_SEED_DEMO=true` 加载，`/v1/meta` 和 UI 显示演示状态。release 构建未配置 `MPGS_DATABASE_PATH` 会拒绝启动。
- 调度器按独立频率入队官方目录、候选发现和富化任务；每类任务最多保留一个活动作业，避免目录同步长期挤占候选或富化。`mpgs-dbtool run-steam-worker-once` 同机通过租约领取任务、写入来源运行记录，并保留上一次成功的运行状态、游标和快照。
- `mpgs-dbtool m7-data-audit` 将 DATA-206 变成可执行门禁：它以当前算法配置和公开 feed 相同的分区资格规则检查候选、可信熟人联机画像、日期、封面、四分区和连续 7 天评价/CCU 覆盖；新游不足的例外必须显式带原因。

## 本地验收证据

- 持久化 SQLite 冒烟：旧记录已验证 schema `9`；当前源码新增迁移至 `14`，显式演示种子可用，创建并登录 6 个独立账号后投票；社区响应结果为 `Votes=6`、`PreviewCount=5`、`OmittedCount=1`。
- HTTP 验证：`/v1/meta` 返回 `storage_enabled=true` 和 `demo_mode=true`；筛选后的社区接口成功返回数据；带 `Origin: http://127.0.0.1:5175` 的 CORS 预检返回 `204`，包含 `authorization`、`x-device-id` 等允许请求头。
- 前端静态检查和单元测试覆盖账号会话、账户写权限、社区投票、头像组、筛选和 AI 设置。浏览器自动化通过同源开发代理加载本地服务，验证了演示标识、桌面 `5 + 1`、窄屏 `3 + 3`、登录后的账户菜单和 AI 设置 DOM；截图命令在该自动化后端超时，未将截图计为证据。
- 真实 Steam 烟测：官方 `IStoreService/GetAppList` 成功拉取两页并从持久化游标续传；同一被忽略 SQLite 数据库通过 schema `9` 完整性检查，且密钥明文扫描为空。候选采集初始得到 `2,071` 条，最终审计为 `2,091` 个规范化候选（M3 目录门槛通过）；导入 50 条人工校准画像后，当前真实快照覆盖平台 `10`、价格 `9`、评测 `10`、CCU `10` 条。临时服务入队后，worker 成功完成一页 1,000 条目录作业，管理状态接口返回成功时间、下次运行、游标和覆盖率。新增 M7 审计的真实结果为可信熟人画像 `14/300`、日期 `10/2091`、封面 `2075/2091`、四分区均 `0`、连续 7 天评价/CCU 均 `0`，因此明确拒绝 DATA-206 发布。

## 门禁状态

| 范围 | 状态 | 说明 |
| --- | --- | --- |
| AUTH-001 至 AUTH-011 | 本地通过 | 存储回归、服务端 API 与客户端账户流程已覆盖。 |
| PROF-001 至 PROF-007 | 本地通过 | 包含格式校验、WebP 重编码、公开字段边界、更新/删除清理及可审计的内容哈希屏蔽。 |
| COM-001 至 COM-010 | 本地通过 | 持久化 6 账号冒烟验证票数、头像预览、溢出数和筛选；游标、ETag 和缓存键绑定筛选条件。 |
| AICFG-001 至 AICFG-010 | 本地通过 | 代码和单元测试覆盖模式隔离、加密、SSRF 防护、额度、删除和缓存隔离。真实 Provider 连通性需在部署密钥环境复验。 |
| DATA-201、DATA-202 | 本地通过 | 已验证持久化/显式演示策略及可观测元数据。 |
| DATA-203 至 DATA-206 | 未关闭 | 调度与同机 worker 已实现，但真实库仍缺可信画像和连续 7 天趋势；VPS 持续 worker 部署完成后仍需重新运行审计。 |
| 发布包、跨浏览器/跨进程、备份恢复、人工签字 | 未关闭 | 需按 PRD 10、11 节在目标环境执行。 |

## 复现命令

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
pnpm --dir web typecheck
pnpm --dir web test
pnpm --dir web build
mpgs-dbtool m7-data-audit <real-db>
```

本次实际执行的 SHA、工作树状态和结果见 [M7_ACCEPTANCE_RUN](M7_ACCEPTANCE_RUN.md)。

本地演示服务必须显式声明状态：

```powershell
$env:MPGS_DATABASE_PATH = '.\data\mpgs-m7-demo.db'
$env:MPGS_SEED_DEMO = 'true'
$env:MPGS_ADMIN_TOKEN = 'dev-only-token'
cargo run -p mpgs-server
```

生产部署不得设置 `MPGS_SEED_DEMO=true`，并应在真实数据覆盖率、持续 worker、备份恢复和发布包验证完成后更新本记录。
