# MPGS 前端重构：给 AI 的执行提示词

面向编码 Agent（Grok / Claude / Cursor 等）。**不要生图**；按阶段改 `web/` 代码。

| 字段 | 内容 |
| --- | --- |
| 范围 | 仅前端 `web/`（必要时读 `docs/API.md`）；默认不改 `apps/server`、`crates/*` |
| 栈 | React 19 + TypeScript + Vite；Tauri 2 加载 `web/dist` |
| 校验 | `pnpm --filter lobbytally-web typecheck` · `test` · `build` |

---

## 怎么用

1. 先贴 **§0 总约束**（每次会话必贴）。
2. 再贴 **一个**阶段提示词（§1～§N），不要一次做完全部。
3. 阶段完成后再开下一阶段；每阶段结束跑 typecheck / test。
4. 若你要换视觉风格，在 §0 后加一句，例如：`视觉：极简白` 或 `视觉：深蓝商店风`。

---

## §0 总约束（每次必贴）

```text
你在 MPGS 仓库做前端重构。产品：熟人联机 Steam 游戏发现工具（私人房/合作/自建服优先，不是热门榜克隆）。

## 范围
- 只改 web/（React + TS + Vite）。默认不改后端、migrations、crates。
- 可只读 docs/API.md、docs/PRD.md、docs/PRD_0.2.md 与 web/src/api/types.ts 对齐契约。

## 必须保留的产品行为
- 匿名可浏览推荐/搜索/日历/详情；写操作（反馈、想玩、跨设备偏好、AI 自定义）需登录。
- 四分区推荐：recent_release / upcoming / popular_legacy / classic_legacy。
- 辅助页：大家想玩、描述推荐、搜索、日历、设置、资料、AI 设置、登录注册、游戏详情。
- 推荐卡：封面、标题、适配分、模式/人数/发售/评价/在线、想玩、理由、风险、喜欢/玩过/不感兴趣。
- 未知数据显示「未知 / 日期未定 / 人数未定」，不伪造确定性。
- 离线可看 ETag 快照；演示数据/离线快照/早期数据/AI 回退要可见标注。
- AI 关闭或失败时确定性推荐仍可用。
- 主题 token 架构保留：组件只用 CSS 变量，皮肤在 themes.css / theme/*；可简化 FX，不可拆掉 data-theme。
- ApiClient、feedbackQueue、playIntent、session 刷新、账户门闩逻辑可重构文件结构，但语义与 API 路径不能 silently break。

## 工程约束
- 不新增重型 UI 框架，除非我明确要求（默认继续手写 CSS + 现有 token）。
- 不引入任意 HTTP 代理或把服务端密钥放进前端。
- 中文 UI 文案；保持现有 data-testid（如 nav-feed-*、nav-*）若 e2e 依赖。
- 小步提交式改动；不要无关重构、不要删测试除非同步更新。
- 每阶段结束：pnpm --filter lobbytally-web typecheck && pnpm --filter lobbytally-web test

## 当前结构（熟悉后再动）
web/src/
  api/        client、types、feedbackQueue、playIntentStore、storage
  app/        runtime、auth、preferences、format、hooks、Theme/Toast
  screens/    Shell + 各页面
  theme/ + fx/ + styles/base.css + styles/themes.css

先阅读相关文件，再改；改完说明改了哪些文件与行为是否不变。
```

---

## §1 阶段：信息架构与路由壳（Shell）

```text
【阶段 1 · Shell / 导航】
在遵守 §0 的前提下重构 web/src/screens/Shell.tsx 及相关样式。

目标：
1. 顶栏信息层级清晰：品牌 | 四分区 | 辅助入口 | 状态 chip | 主题/特效 | 账户。
2. 导航状态单一来源；game 详情返回上一列表视图（已有 lastListView 语义保留）。
3. 键盘：1–4 切分区，/ 打开搜索；输入框内不抢键。
4. 状态 chip：离线、演示数据(meta.demo_mode)、待同步反馈数。
5. 账户菜单入口保留：登录 / 资料 / AI 设置 / 退出。

允许：
- 拆分子组件（Topbar、NavTabs、StatusChips 等）到 screens/ 或 components/。
- 调整 CSS 布局（例如 sticky topbar、窄屏折叠辅助入口），但不删页面能力。

不要：
- 改 API 客户端协议。
- 引入 react-router（除非收益极大且你先说明）；当前 view state 模式可保留。

交付：typecheck + test 通过；简述导航信息架构。
```

---

## §2 阶段：设计 token 与基础组件

```text
【阶段 2 · Design System 底座】
在遵守 §0 的前提下整理样式与可复用组件，为后续换肤/重排铺路。

目标：
1. 审计 styles/base.css 与 themes.css：token 命名一致，组件不写死主题色。
2. 抽出或整理基础组件（可放 web/src/components/）：
   - Button / Chip / Badge / Panel / EmptyState / Pagination / Modal shell
   - GameMedia / VoteButton / Facepile（可从 screens/ 挪出）
3. GameCard 仍消费 FeedItem 全字段语义；视觉可重排，字段不能丢。
4. 空态、加载骨架、错误 state-box 文案与诚实降级原则一致。

不要：
- 为了“好看”删掉 reasons/cautions/想玩/低置信标记。
- 大改 theme FX 引擎；最多降低默认强度或修明显布局 bug。

交付：列出新增组件文件；旧 screens 改为引用；test + typecheck 通过。
```

---

## §3 阶段：推荐流 + 游戏卡

```text
【阶段 3 · Feed + GameCard】
重构 FeedScreen / GameCard，这是产品主路径。

目标：
1. 列表：分页、排序（recommended/ccu/reviews/release_date）与 section 默认排序逻辑保留（upcoming 默认发售日）。
2. 卡片布局重做但信息优先级固定：
   封面 > 标题+适配分 > 模式/人数/日期/评价/在线 > 想玩 > 理由 > 风险 > 反馈按钮
3. 未登录点反馈/想玩 → 触发登录门闩（requestAccountSignIn）。
4. 反馈 toast 可撤销；队列语义不变。
5. 数据新鲜度 / stale / 离线来源提示保留。

实现注意：
- 继续用 useFeed / apiClient；不要在组件里直接 fetch。
- 封面失败占位尺寸稳定（GameMedia）。
- 保持可访问性：卡片可键盘打开详情。

交付：主路径目视结构说明 + test/typecheck。
```

---

## §4 阶段：游戏详情

```text
【阶段 4 · GameDetailScreen】
重构详情页布局，数据字段对齐 GameDetail / evidence / popular reviews。

目标：
1. Hero：封面、标题、发售状态、模式 chip、想玩、Steam 外链。
2. 资料区：联机方式、可用性（平台/语言/时长/价格）、评价与 CCU、证据来源。
3. 未知/低置信/仅分类弱信号等诚实标注保留。
4. Esc / 返回上一列表；离线快照 chip。
5. 热门评价卡可展开，不引入违规外链策略（保持 noreferrer noopener）。

不要编造后端没有的字段。缺数据用「未知/待同步」类文案。
交付：typecheck + test。
```

---

## §5 阶段：描述推荐（NL）

```text
【阶段 5 · NaturalLanguageScreen】
重构自然语言推荐页。

目标：
1. 输入 + 提交 + 示例 query chips。
2. 展示 interpreted 约束（人数/合作竞技/时长等）与 ai_status（used/cached/fallback/disabled）。
3. 结果复用 GameCard（或同一卡片组件）。
4. 自定义 AI：若本地有 custom settings，按现有 apiClient.naturalLanguageRecommendations 传参，不把 key 写入日志或 localStorage 明文新位置。
5. 离线错误文案明确。

交付：typecheck + test；说明 AI 回退时 UI 如何表达。
```

---

## §6 阶段：大家想玩

```text
【阶段 6 · CommunityScreen】
重构社区想玩页。

目标：
1. 排序：trending（正在升温）/ most_voted（最多人想玩）。
2. 筛选：发售状态、demo、平台、人数（与现 API filters 对齐）。
3. 列表项：封面、名、日期精度文案、票数、facepile（最多 5，+N 不可点开用户目录）、VoteButton。
4. 游标加载更多；cursor_stale 时重拉第一页。
5. 投票后与 playIntentStore 同步刷新逻辑保留。

交付：typecheck + test。
```

---

## §7 阶段：日历 + 搜索

```text
【阶段 7 · CalendarScreen + SearchScreen】

日历：
- period：upcoming / recent
- 类型过滤：全部 / game / demo / playtest
- 按月分组；undated_items 单独「日期未定」
- early_data、精度、置信度 chip 保留

搜索：
- 防抖 GET /v1/search；无 AI
- 竞态用 RequestGeneration（或等价）丢弃过期响应
- 结果行：名 + 发售状态 + 日期；可进详情
- / 快捷键由 Shell 进入后自动 focus 输入框

交付：typecheck + test。
```

---

## §8 阶段：设置 / 引导 / 账户 / AI

```text
【阶段 8 · Onboarding + Settings + Auth + Profile + AiSettings】

Onboarding：
- 步 0 主题，步 1 偏好；写 preferences 队列/API；markOnboarded

Settings：
- 人数、合作竞技、时长、预算、平台、语言、排除模式、自建服意愿
- 主题 + FX 强度
- 待同步偏好 / 需登录才能云端保存的路径保持正确

AuthDialog：
- 登录/注册；merge_choice_required 时让用户选 anonymous vs account 偏好
- 错误：冲突、限流、通用失败

Profile：
- 显示名、头像 ≤2MiB jpeg/png/webp、改密、注销确认

AiSettings：
- 三模式 builtin / custom / off
- 自定义：base URL、模型、key 本地、test、discover、routing preset
- 服务端不回读明文 key；错误码文案（temporarily_unavailable、ai_connection_failed 等）保留

交付：typecheck + test；列出登录门槛触发点是否仍完整。
```

---

## §9 阶段：收尾质检

```text
【阶段 9 · 质检与收敛】
1. 跑 pnpm --filter lobbytally-web typecheck && test && build
2. 全局搜 FIXME/临时 hack；清掉本轮引入的死代码
3. 确认 screens 无重复卡片实现（统一 GameCard/组件）
4. 确认 data-testid 导航仍在
5. 用简短中文写「前端重构说明」：信息架构、视觉变化、未动的 API 行为
6. 不要改后端；若发现 API 缺陷只记录不顺手修（除非阻塞且我授权）
```

---

## 一次性总包（仅在上下文很长且你明确要求端到端时用）

```text
请按 docs/FRONTEND_REFACTOR_PROMPTS.md 的 §0 约束，从阶段 1 做到阶段 9，
重构 MPGS 前端 web/ 的视觉与组件结构，但保持全部产品行为与 API 契约不变。
每个阶段结束自行 typecheck + test，失败先修再继续。
优先主路径：Shell → token/组件 → Feed/GameCard → Detail，其余页面再铺开。
不要生图，不要写无关 markdown，不要改服务端。
```

---

## 附录 A · 页面 ↔ API 速查

| 页面 | 主要 API / 客户端 |
| --- | --- |
| Feed | `GET /v1/feeds/{section}`、`POST /v1/feedback`、play-intent |
| Detail | `GET /v1/games/{id}`、`/evidence` |
| Search | `GET /v1/search` |
| NL | `POST /v1/ai/search`（以 client 方法为准） |
| Community | community list + play-intent |
| Calendar | `GET /v1/calendar` |
| Prefs | `GET/PUT /v1/preferences` |
| Auth | `/v1/auth/*`、`/v1/session/*` |
| Me | `/v1/me`、avatar、ai-settings |

## 附录 B · 不要破坏的客户端语义

- `ApiClient`：匿名会话、401 单飞刷新、ETag 缓存、`fromOfflineCache`
- `feedbackQueue`：离线排队、撤销、ranking 变更通知
- `subscribeAccountGate` / `requestAccountSignIn`：写操作门闩
- 偏好 `queuePreferencePatch` / version 冲突
- 自定义 AI key 仅本地安全存储路径（见 `localAiSettings`）

## 附录 C · 推荐视觉方向（可选一句）

任选其一追加到 §0 后：

- `视觉：深蓝商店风，信息密度高，卡片横向封面。`
- `视觉：极简白底，细边框，留白多，单强调色。`
- `视觉：深色工具型 Dashboard，清晰分区，少装饰。`

主题换皮仍走 `data-theme` + token；不要为每个主题复制组件树。
