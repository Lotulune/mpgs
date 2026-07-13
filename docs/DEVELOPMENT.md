# 本地开发指南

## 1. 当前基线

仓库当前处于 M3（确定性推荐与公开 API 完成）：

- `mpgs-domain`：分区、偏好、反馈类型与推荐信号。
- `mpgs-recommender`：评分、个性化、硬过滤、MMR、解释与 `rank_feed`。
- `mpgs-steam-source`：Steam 源规范化 Spike 与黄金集。
- `mpgs-storage`：SQLite 迁移（含用户/偏好/反馈）、Repository、种子目录、查询与备份。
- `mpgs-server`：公开 API（会话/偏好/四分区/日历/搜索/详情/证据/反馈）、`x-request-id`、ETag；管理/内部 jobs。
- `mpgs-dbtool`：migrate / integrity / backup / restore。
- 尚未接入 AI Provider 或 Tauri 客户端。

下一项开发工作是 [M4 Tauri 桌面客户端](MVP_PLAN.md#m4tauri-桌面客户端)。

### Git

本机已使用 Git for Windows。新终端若找不到 `git`，将 `C:\Program Files\Git\cmd` 加入 PATH，或在当前会话执行：

```powershell
$env:Path = "C:\Program Files\Git\cmd;" + $env:Path
```

仓库本地配置（不写全局）：`user.name` / `user.email` 仅限本仓库；`core.autocrlf=true`。

## 2. 前置环境

- Rust stable，最低 `1.97`，包含 `rustfmt` 与 `clippy`。
- Git，用于正常版本管理；当前代码不依赖 Git 才能编译。
- Node.js LTS 与 pnpm，仅在建立 Tauri/React 客户端后需要。
- Windows 开发 Tauri 时需要 Microsoft C++ Build Tools 与 WebView2。
- Linux 开发 Tauri 时需要对应发行版的 WebKitGTK 4.1 开发包。
- macOS 打包需要 macOS/Xcode 环境，不能把正式签名产物建立在非 macOS 主机上。

## 3. 常用命令

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p mpgs-storage --locked
cargo run -p mpgs-server
```

`mpgs-steam-source` / `mpgs-storage` 默认测试只使用夹具与临时库，不调用实时 Steam，也不需要 Web API Key。

带本地 SQLite 启动服务端：

```powershell
New-Item -ItemType Directory -Force data | Out-Null
$env:MPGS_DATABASE_PATH = '.\data\mpgs.db'
$env:MPGS_ADMIN_TOKEN = 'dev-only-token'
cargo run -p mpgs-server
```

运行后验证：

```powershell
Invoke-RestMethod 'http://127.0.0.1:8080/health/live'
Invoke-RestMethod 'http://127.0.0.1:8080/health/ready'
Invoke-RestMethod 'http://127.0.0.1:8080/v1/meta'
```

数据库工具：

```powershell
cargo run -p mpgs-dbtool -- migrate .\data\mpgs.db
cargo run -p mpgs-dbtool -- integrity .\data\mpgs.db
cargo run -p mpgs-dbtool -- backup .\data\mpgs.db .\backups\mpgs.db
cargo run -p mpgs-dbtool -- restore .\backups\mpgs.db .\data-restored\mpgs.db
# 或
.\scripts\backup_db.ps1 -DbPath .\data\mpgs.db -OutPath .\backups\mpgs.db
.\scripts\restore_db.ps1 -From .\backups\mpgs.db -To .\data-restored\mpgs.db
```

服务默认绑定 `127.0.0.1:8080`。仅在本地端口冲突时临时设置进程变量：

```powershell
$env:MPGS_BIND_ADDR = '127.0.0.1:8081'
cargo run -p mpgs-server
```

不要把本地地址、Key 或个人路径提交为默认配置。

## 4. Workspace 依赖方向

允许：

```text
server -> storage/steam-source/recommender/domain
dbtool -> storage
recommender -> domain
storage -> domain + steam-source（仅 proposal 类型）
steam-source -> domain
desktop Rust -> api-contract/domain（必要时）
```

禁止：

```text
domain -> Axum/SQLite/AI SDK/Tauri
recommender -> HTTP/数据库/具体 AI Provider
source adapter -> UI
客户端 -> storage 或服务端密钥模块
```

领域逻辑应能使用纯结构体和夹具测试，不需要网络和数据库。

## 5. Rust 代码约定

- 新 crate 默认 `#![forbid(unsafe_code)]`；确需 unsafe 的底层依赖封装必须单独评审。
- 公共边界使用强类型、枚举和范围校验，不用自由字符串穿透领域层。
- 外部错误在适配器边界转换为稳定业务错误，不根据供应商错误文本驱动逻辑。
- 网络请求不得持有 SQLite 写事务。
- 时间由可注入 Clock 提供，测试不依赖真实当前时间。
- 随机探索由可注入种子提供，推荐测试必须可重现。
- 日志使用结构化字段，禁止日志拼接 Key、令牌、Cookie、完整 Prompt 或私人文本。
- `unwrap/expect` 只用于测试或编译期不变量；服务请求路径返回可分类错误。

## 6. 推荐器开发

- 所有输入在边界归一化为 `[0,1]`；NaN、无穷和越界值必须显式处理。
- 改变公式、阈值或缺失值策略时更新算法配置版本。
- 新规则至少添加一个正例、一个反例和一个边界测试。
- 黄金游戏的真实标签不应直接硬编码进公式；测试使用它们代表的特征信号。
- AI 分数不得在推荐器之外私自混合，统一经过受限融合函数。

## 7. 外部数据适配器

每个来源独立 crate/module，并实现统一阶段：

```text
request -> raw response validation -> source DTO -> normalized proposal
```

要求：

- 明确 User-Agent、限流、超时、重试和最大响应大小。
- DTO 与领域模型分离，外部字段变化不直接污染数据库 Schema。
- 保存脱敏夹具用于测试；默认测试不调用实时 Steam。
- 解析失败不得返回“成功空数据”。
- 适配器版本进入 `source_runs` 和原始文档元数据。
- 新增来源前更新 [数据来源表](DATA_STORAGE.md#2-数据源) 与 [外部资料](SOURCES.md)。

## 8. SQLite 开发

- 迁移放在 `migrations/NNNN_description.sql`，发布后不得修改已有迁移。
- 集成测试为每个用例创建独立临时数据库。
- 每次获取连接后验证必要 PRAGMA，不假设连接池自动继承。
- 生产文件只能由同机服务访问；测试也不要使用网络共享目录。
- 写事务保持短小，批量写使用有界批次。
- FTS、Embedding 和推荐快照必须能从权威表重建。

新增迁移的最低测试：

1. 空库升级。
2. 上一版本含数据数据库升级。
3. 外键、唯一键和 CHECK 约束。
4. 重复执行迁移入口不会静默破坏数据。
5. 备份恢复后可继续迁移。

## 9. API 开发

- 先更新 [API 契约](API.md)，再实现 DTO 与 handler。
- handler 只做协议、鉴权和输入校验；业务逻辑进入 service/domain。
- 列表端点使用不透明游标，不使用易漂移的公开 offset 分页。
- 写端点考虑幂等、乐观版本和审计。
- AI 回退属于成功响应的明确状态，不用 500 表示正常降级。
- 新端点必须有成功、输入错误、权限错误和依赖故障测试。

## 10. AI 开发

- 测试默认使用 Fake 或 Disabled Provider，不消耗真实额度。
- 模型输出先经过 JSON Schema 和语义校验，再进入推荐器。
- 不提供通用 SQL、任意 URL 或文件工具。
- 所有模型可见外部文本均标记为数据，并进行大小/字符清洗。
- 真实 Provider 集成测试通过显式开关运行，不能成为普通测试前置条件。
- 不在仓库、命令历史示例或测试夹具中放真实 Key。

## 11. 客户端开发

- Tauri capabilities 使用最小权限，不开放任意 shell 或文件系统访问。
- UI 必须覆盖加载、空、错误、过期、离线和 AI 回退状态。
- 缓存响应与用户待同步反馈分开，清缓存不能丢失未提交反馈。
- 外部描述按纯文本/清洗内容渲染；只允许 HTTPS 和受控 Steam 链接。
- 桌面主工作流优先支持键盘，文本在所有目标窗口尺寸下不得溢出。

## 12. 配置和密钥

计划中的服务端密钥名称：

```text
MPGS_STEAM_WEB_API_KEY
MPGS_AI_API_KEY
MPGS_AI_API_BASE_URL
MPGS_AI_MODEL
MPGS_EMBEDDING_MODEL
```

这些变量当前尚未被代码读取。实现 Provider 时提供 `.env.example` 但不自动加载生产 `.env`，并确保日志只显示“已配置/未配置”。供应商 URL 是否允许自定义需要单独安全评审。

## 13. 提交前检查

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

同时确认：

- 文档和 OpenAPI 与行为一致。
- 没有真实 Key、令牌、数据库、日志或原始个人数据进入变更。
- 没有绕过证据、候选白名单或 SQLite 单主边界。
- 新外部事实附官方来源与核验日期。

CI/CD 尚未创建；建立自动化工作流需要项目负责人明确确认。

