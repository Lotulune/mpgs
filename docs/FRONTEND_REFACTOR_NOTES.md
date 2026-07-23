# 前端重构说明（收尾质检记录）

日期：2026-07-22 ｜ 范围：仅 `web/`（React 19 + TS + Vite）｜ 校验：`typecheck` ✅ · `test` 66/66 ✅ · `build` ✅

## 信息架构

- **Shell 拆分**：`screens/Shell.tsx` 只保留导航状态（`view` 单一数据源）、订阅（在线状态 / 账户 / demo / 待同步反馈数）与屏幕挂载；顶栏拆到 `screens/shell/`：
  - `Topbar.tsx` — 品牌 | 四分区 | 辅助入口 | 状态 chip | 主题/特效 | 账户
  - `NavTabs.tsx` — 分区与辅助页 tab，保留 `data-testid="nav-feed-*" / "nav-*"`（e2e 依赖）
  - `StatusChips.tsx` — 离线 / 演示数据 / 待同步反馈 chip
  - `nav.ts` — `View` 类型与默认视图；`useNavShortcuts.ts` — `1–4` 切分区、`/` 开搜索（输入框内不抢键）
- **游戏详情返回语义不变**：`lastListView` 记录来源列表，Esc/返回回到原列表视图。

## 组件底座

新增 `web/src/components/`（11 个可复用组件，全部由各页面统一引用）：

- 基础：`Button` / `Chip` / `Panel` / `Modal` / `EmptyState` / `Pagination` / `Skeleton` / `ScoreBadge`
- 业务（从 `screens/` 挪出）：`GameMedia`（封面失败占位尺寸稳定）/ `VoteButton` / `Facepile`
- **GameCard 全仓库唯一实现**（`screens/GameCard.tsx`），Feed 与描述推荐共用；FeedItem 全字段语义保留（理由/风险/想玩/低置信标注均未删）。

## 视觉变化

- `styles/base.css` 重构为「token 契约 + 组件基础样式」：组件只消费 CSS 变量，皮肤全部走 `themes.css` / `theme/*` 的 `data-theme` 覆盖。
- 页面级样式拆到 `styles/screens/`：`calendar-search.css` / `community.css` / `game-detail.css` / `nl.css` / `settings.css`。
- 主题 `mc.ts` / `wafu.ts` 适配新 token；FX 引擎未改架构，仅按 token 对齐。
- 改动规模：25 个已跟踪文件 +1603/−1370，另新增 21 个文件（components 11 + shell 5 + screen CSS 5）。

## 未动的 API 行为

- `ApiClient`（匿名会话、401 单飞刷新、ETag 缓存、`fromOfflineCache`）、`feedbackQueue`（离线排队/撤销/通知）、`playIntentStore`、`subscribeAccountGate`/`requestAccountSignIn` 门闩、偏好 `queuePreferencePatch`、自定义 AI key 本地存储路径——语义与 API 路径均未变。
- 匿名浏览/登录写操作门槛、四分区推荐、未知数据诚实标注（未知/日期未定/人数未定）、离线快照与降级标注均保留。
- 测试更新仅限 `format.test.ts` 同步新增断言，无删测试。

## 质检记录（§9）

1. `pnpm --filter mpgs-web typecheck` ✅ `test` 12 文件 66 用例 ✅ `build` ✅
2. 全局搜 FIXME/临时 hack：无。
3. 无重复卡片实现；旧 `screens/Facepile|GameMedia|VoteButton` 引用已全迁移至 `components/`。
4. `data-testid` 导航与 auth 系列完整。

## 发现的环境问题（未修，仅记录）

- `data/mpgs.db` 处于损坏的迁移中间态：缺 `play_intent_state` 表与 `play_intent_revision_after_insert/delete` 触发器（0006 部分应用），但 0008 的触发器已存在 → 服务端启动报 `error in trigger ... no such table: main.play_intent_state`。建议用 dbtool 重建或从备份恢复该库。
- `target/debug/mpgs-server.exe` 内嵌迁移只到 0013，`target/release/` 到 0015；`m7-preview-real-v1.db` 在 schema v14 → debug 二进制无法对其启动（cannot migrate down to 13）。本地预览请用 release 二进制 + `MPGS_DATABASE_PATH=./data/m7-preview-real-v1.db`，或重新 `cargo build`。
