# M1 数据可行性报告

状态：完成（夹具驱动 Spike + 黄金集）。  
范围：`DAT-001`～`DAT-005`。默认测试不访问实时 Steam。

## 1. 结论摘要

| 能力 | 可行性 | 稳定性 | 备注 |
| --- | --- | --- | --- |
| App 目录增量 | 可行 | 官方稳定 | `IStoreService/GetAppList`，需 Web API Key；支持 `last_appid` 分页与 `if_modified_since` |
| 评论摘要 | 可行 | 官方稳定（Store Reviews 文档） | `query_summary` 提供正/负/总数；参数哈希进入快照键 |
| CCU | 可行 | 官方稳定 | 仅 Steam 在线连接玩家；缺失用 `result != 1` 表达，不可写成 0 成功 |
| 发售日历 | 有条件可行 | 易变商店适配器 | `store.steampowered.com/api/appdetails` 可解析 `coming_soon`/`date`，非稳定合约 |
| Demo/Playtest 关系 | 有条件可行 | 易变商店适配器 | `demos[]` / `fullgame` 可建立关系提案；Playtest 需额外人工或规则校验 |
| 熟人联机质量 | 部分可行 | 人工为主 | 商店类别只能作提示；自建服/私人房以黄金集与后续人工校正为准 |

**产品范围决策（M1 退出条件）：**

- 不因日历/Demo 无法 100% 官方覆盖而缩减四分区产品目标。
- 将商店 `appdetails` 定为 **经批准的易变适配器**：隔离在 `mpgs-steam-source::store`，解析失败保留旧值并进入审核队列。
- 多人关键特征以 **人工校正 + 证据** 为权威，标签/类别不得单独证明自建服。

## 2. 字段来源分类

### 2.1 官方稳定接口

| 字段 | 来源 | 适配器版本 |
| --- | --- | --- |
| `app_id`, `name`, `last_modified`, `price_change_number` | `IStoreService/GetAppList` | `app-list-0.1.0` |
| `total_positive`, `total_negative`, `total_reviews`, score 描述 | Store Reviews `query_summary` | `reviews-0.1.0` |
| `player_count`（可空） | `ISteamUserStats/GetNumberOfCurrentPlayers` | `ccu-0.1.0` |

### 2.2 经批准的易变适配器

| 字段 | 来源 | 适配器版本 | 失败回退 |
| --- | --- | --- | --- |
| `coming_soon`, 原始发售日字符串 | Store `appdetails` | `store-appdetails-0.1.0` | 保留 `apps` 旧值 + `release_events` 不写入 |
| Demo 关系 | `demos[]` / `fullgame` | 同上 | 人工关系表 / 黄金集 |
| 类别/类型多人提示 | categories/genres | 同上 | 仅作 `feature_evidence` 低置信提示 |

### 2.3 人工维护

| 字段 | 说明 |
| --- | --- |
| `private_session`, `self_host_or_dedicated`, 主导体验 | 黄金集 `golden-0.1.0` 与后续 `curation_overrides` |
| 停服风险、人数不匹配案例 | 黄金集 `case_tags`：`shutdown_risk`, `party_size` |
| 高影响字段双人复核 | `dual_reviewed=true` 子集（≥10） |

## 3. 适配器行为（已实现）

代码：`crates/steam-source`（crate 名 `mpgs-steam-source`）。

统一阶段：

```text
request -> RawResponse 校验（状态/大小/UTF-8）
        -> 来源 DTO 反序列化
        -> 规范化 Proposal
        -> 错误分类（可重试 / 永久 / 结构变化）
```

### 3.1 GetAppList（DAT-001）

- 请求构造：`last_appid`, `if_modified_since`, `max_results`, 类型过滤标志。
- 分页：`have_more_results` + `last_appid` 写入 `AppListCursor`。
- 断点续传：进程重启后用 cursor 的 `last_appid` 继续；整轮结束后 `complete_pass` 抬高 `if_modified_since`。
- 结构异常：`have_more_results=true` 且空列表 → `InvalidStructure`（禁止当成功空数据）。

夹具：`fixtures/app_list_page1.json`, `app_list_page2.json`, `app_list_incremental.json`。

### 3.2 评论摘要（DAT-002）

- 默认摘要请求：`language=all`, `purchase_type=all`, `num_per_page=0`, `filter_offtopic_activity=1`。
- `parameter_hash`：参数排序后 SHA-256，进入 `ReviewSummaryProposal`。
- 规范化：正/负/总数 + score 描述；`offline_players_excluded=true` 备注（评论本身非 CCU，但与“不可用离线玩家”同一数据诚实原则）。

### 3.3 CCU（DAT-003）

- 成功：`result=1` 且 `player_count` 存在。
- 缺失：非成功 `result` → `player_count=None` + `missing_reason`，**不得**落成 0 在线成功样本。
- 限制：明确 `offline_players_excluded=true`。
- 采样分层：`CcuSampleTier::Focus` 30 分钟；`LongTail` 6 小时（调度建议，非强制配置）。

### 3.4 商店详情 / Demo / 日历（DAT-004）

- 解析 `coming_soon`、原始日期、`demos`、`fullgame`、类别多人提示。
- `SourceStability::ApprovedVolatile`。
- 不成功响应 → `NotFound`，不产生空成功提案。
- 推荐回退：人工 `release_events` / 关系表；结构变化时隔离来源。

## 4. 限流、失败类型与响应体

| 类别 | 处理 |
| --- | --- |
| HTTP 429 | `SourceError::RateLimited`，可重试 |
| 5xx / 408 / 425 | 可重试临时失败 |
| 4xx（非 429） | 按场景永久或 NotFound |
| 体过大 | 默认 8 MiB 上限，`ResponseTooLarge` |
| JSON/结构错误 | `JsonParse` / `InvalidStructure`；**不覆盖**现有权威值（M2 写入时强制） |
| 日预算 | `DailyBudget` 本地软记账；条款宣传值 100_000/日，须留安全余量 |

实测实时配额与平均响应字节数依赖 Key 与当时商店规模，**未在 CI 中直播调用**。上线前应用真实 Key 做一次受控手工抽样，并把结果回填本节。

夹具样本大小（量级）：

| 夹具 | 约字节 |
| --- | --- |
| GetAppList 单页（3～5 app） | < 1 KiB |
| Reviews summary | < 1 KiB |
| CCU | < 200 B |
| appdetails 单 app | 1～3 KiB |

全量目录页在 `max_results` 较大时可达数百 KiB～数 MiB，适配器必须保留大小上限。

## 5. 黄金集（DAT-005）

- 文件：`crates/steam-source/fixtures/golden_set_v0.json`
- 版本：`golden-0.1.0`
- 规模：50 个 AppID
- 覆盖标签：`coop`, `self_host`, `matchmaking_core`, `mmo`, `shutdown_risk`, `party_size`, `public_world`, `private_session`, `competitive`
- 双人复核高影响项：≥10（`dual_reviewed=true`）
- 加载 API：`GoldenSet::load_embedded()`，测试断言 M1 退出门槛

每个条目包含：AppID、发售状态、评价分桶、基本多人特征、证据说明。

## 6. 2,000 多人候选退出条件说明

M1 **不要求**在本仓库内完成 2,000 条真实采集写入（需 Key 与合规调度，属 M2）。

本阶段交付：

1. 可重复的分页/增量/续传算法与夹具证明；
2. 规范化 Proposal 模型，可直接喂给 M2 Repository；
3. 黄金集作为多人候选质量样板。

在具备 Steam Web API Key 后，使用同一 `AppListRequest`/`AppListCursor` 连续拉页即可扩展到 ≥2,000 多人候选；多人过滤将结合 `appdetails` 类别提示 + 人工/规则，并在 M2 入库。

## 7. 对 M2 的输入清单

1. `AppCatalogProposal` / `ReviewSummaryProposal` / `CcuProposal` / `StoreDetailsProposal` 表映射。
2. `AppListCursor` → `source_cursors`。
3. `SourceError` 分类 → `source_runs` 错误类别。
4. 商店适配器独立限流桶，不与 Web API 100k 预算混用。
5. 黄金集导入 `multiplayer_profiles` + `feature_evidence`（`verified_by_human`）。

## 8. 复现命令

```powershell
cargo test -p mpgs-steam-source --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

相关文档：

- [SOURCES.md](SOURCES.md)
- [DATA_STORAGE.md](DATA_STORAGE.md)
- [MVP_PLAN.md](MVP_PLAN.md#m1数据可行性-spike)
