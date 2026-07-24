# Steam 游戏媒体画廊实现规格（交给 Grok Build）

状态：已实现（2026-07-24）  
编写日期：2026-07-24  
目标版本：下一次功能迭代  
优先级：截图画廊 P0；预告片播放 P1

## 1. 任务目标

游戏详情页当前只展示一张 Steam 商店封面。改为展示同一游戏的 Steam 商店媒体：

1. 保留现有封面及其回退能力。
2. 展示 Steam 商店截图缩略图和大图。
3. 在运行环境支持时展示 Steam 商店预告片；播放失败时必须平稳回退。
4. 媒体来自已有的 Steam `appdetails` 富化任务，不允许浏览器临时请求 Steam API，也不允许根据 AppID 猜测截图或视频 URL。
5. 不扩大 Feed、搜索、日历和社区列表响应；完整媒体只随游戏详情返回。

完成后的核心体验：

- 进入详情页即可看到封面、截图和视频海报组成的横向缩略图列表。
- 点击缩略图切换主画面；点击图片可查看大图。
- 视频不自动播放，只有用户明确点击后才加载/播放。
- 无截图、无视频、单个资源失效或 Steam 暂时不可用时，页面仍正常显示现有封面和游戏资料。

## 2. 当前代码事实

实现前必须基于这些现有位置继续开发，不要另建一条平行数据链路：

| 环节 | 当前实现 | 本任务缺口 |
| --- | --- | --- |
| Steam 解析 | `crates/steam-source/src/store.rs` 的 `AppDetailsData` 和 `parse_store_details` | 只解析 `header_image`，未解析 `screenshots` / `movies` |
| 规范化提案 | `crates/steam-source/src/proposal.rs` 的 `StoreDetailsProposal` | 只有 `header_image_url` |
| 持久化 | `migrations/0008_m7_accounts_community_ai.sql` 的 `app_media` | 每个 App 只有一行 `capsule_url`，不支持一对多媒体 |
| 入库 | `crates/storage/src/ingest.rs` 的 `ingest_store_details` | 只更新 `app_media.capsule_url` |
| 查询 | `crates/storage/src/query.rs` 的 `get_game_detail` / `GameCandidateRow` | 只返回封面字段 |
| 服务 API | `apps/server/src/api.rs` 的游戏详情处理 | 只返回 `cover_url` / `cover_updated_at_ms` |
| Web 类型 | `web/src/api/types.ts` 的 `GameDetail` | 没有媒体列表 |
| 详情 UI | `web/src/screens/GameDetailScreen.tsx` | Hero 只渲染 `GameMedia` |
| 单图组件 | `web/src/components/GameMedia.tsx` | 只处理封面候选和图片失败回退 |
| 页面样式 | `web/src/styles/screens/game-detail.css` | `.detail-cover` 只适配单图 |
| CSP | `web/index.html`、`apps/desktop/src-tauri/tauri.conf.json` | 当前没有允许远程视频的 `media-src` |

注意：`GameMedia` 仍被游戏卡片等列表使用。不要把它直接改造成重量级画廊；新建详情页专用组件，并复用现有封面回退函数。

## 3. 已核验的 Steam 响应

2026-07-24 对 Valheim（AppID `892970`）的实际响应核验显示：

- `screenshots[]` 含 `id`、`path_thumbnail`、`path_full`。
- `movies[]` 含 `id`、`name`、`thumbnail`、`highlight`，当前响应提供 `hls_h264`、`dash_h264`、`dash_av1`。
- 同一响应仍含 `header_image`、`capsule_image`、`background` 等字段。

核验地址：

- [Steam appdetails 实际响应](https://store.steampowered.com/api/appdetails?appids=892970&l=schinese&cc=cn)
- [Steam Web API Terms of Use（页面标注 Last updated July 2010）](https://steamcommunity.com/dev/apiterms)

仓库已在 `docs/SOURCES.md` 和 `crates/steam-source/src/store.rs` 中把 `appdetails` 定义为 `ApprovedVolatile`，不是稳定公开契约。实现必须保持：

- 结构变化可观测；
- 解析失败不覆盖已有有效媒体；
- URL 严格白名单；
- 不暗示 LobbyTally 获得 Valve/Steam 官方背书。

## 4. 范围与非目标

### 4.1 本次必须完成

- Steam 截图和视频元数据解析。
- 一对多媒体数据库迁移、写入和读取。
- 游戏详情 API 的向后兼容增量。
- 详情页媒体画廊、键盘操作、加载/错误/空状态。
- Web 与 Tauri CSP 更新。
- 解析、迁移、存储、API、组件测试。
- `docs/API.md`、`docs/DATA_STORAGE.md`、`docs/SOURCES.md` 的同步更新。

### 4.2 本次不要做

- 不把图片或视频二进制下载到 LobbyTally 数据目录或数据库。
- 不由 LobbyTally 服务端代理 Steam 视频流。
- 不抓取第三方素材站、YouTube 或开发商官网作为补充。
- 不把完整媒体塞进 Feed、搜索、日历、社区接口。
- 不自动播放、不静音自动播、不进入页面后预取整段视频。
- 不使用 `dangerouslySetInnerHTML` 注入播放器或 Steam 页面代码。
- 不用 AppID 拼接或枚举截图/视频 URL。
- 不重构无关的详情信息面板、推荐逻辑或主题系统。

## 5. 数据契约

### 5.1 Steam 解析 DTO

在 `crates/steam-source/src/store.rs` 增加容错 DTO。所有字段均使用 `#[serde(default)]`，未知字段继续忽略：

```rust
struct ScreenshotDto {
    id: Option<u64>,
    path_thumbnail: Option<String>,
    path_full: Option<String>,
}

struct MovieDto {
    id: Option<u64>,
    name: Option<String>,
    thumbnail: Option<String>,
    highlight: Option<bool>,
    hls_h264: Option<String>,
    dash_h264: Option<String>,
    dash_av1: Option<String>,
    mp4: Option<MovieMp4Dto>,
    webm: Option<MovieWebmDto>,
}
```

`mp4` / `webm` 是为兼容旧响应形态保留的可选字段；只提取明确存在的 URL，不推导 URL。

### 5.2 规范化提案

在 `crates/steam-source/src/proposal.rs` 新增类型，不要把上游 DTO 直接泄漏到 storage：

```rust
pub struct StoreScreenshotProposal {
    pub source_id: String,
    pub sort_order: u16,
    pub thumbnail_url: String,
    pub full_url: String,
}

pub struct StoreMovieProposal {
    pub source_id: String,
    pub sort_order: u16,
    pub title: Option<String>,
    pub poster_url: String,
    pub highlight: bool,
    pub mp4_url: Option<String>,
    pub hls_h264_url: Option<String>,
    pub dash_h264_url: Option<String>,
}
```

并在 `StoreDetailsProposal` 增加：

```rust
pub screenshots: Option<Vec<StoreScreenshotProposal>>,
pub movies: Option<Vec<StoreMovieProposal>>,
```

这里必须使用 `Option<Vec<_>>`：

- 字段缺失或结构异常：`None`，入库时保留旧快照。
- Steam 明确返回空数组：`Some(vec![])`，允许清空该类型旧数据。
- 正常数组：`Some(items)`，事务替换该类型快照。

解析上限：

- 截图最多保留前 `20` 张。
- 视频最多保留前 `5` 个。
- 保持 Steam 原始顺序；不要按标题或 ID 重排。
- 同一响应内按 `(kind, source_id)` 去重，首次出现优先。

### 5.3 URL 安全规则

把现有 `normalize_header_image` 抽成可复用的 Steam 媒体 URL 校验器，至少满足：

- 只接受绝对 `https://`。
- 禁止 userinfo、显式端口、空 host、路径为空、反斜杠和控制字符。
- 图片 host 只允许 `steamstatic.com` 或其子域。
- 视频播放 host 只允许 `video.akamai.steamstatic.com`，以及实际夹具证明需要的其他 `steamstatic.com` 视频子域；不要开放任意 `https:`。
- 图片 URL 只能进入图片/海报字段，播放 URL 只能进入播放字段。
- 无效的单个媒体项被丢弃并计数，不使整个 App 解析失败。
- 截图必须同时有合法缩略图和大图。
- 视频必须有合法海报，且至少有一个合法播放 URL；否则不进入 `movies`。
- 标题去除首尾空白并限制为 `200` 个 Unicode 字符。

记录结构漂移和被拒绝 URL 的聚合计数，不在普通日志打印完整查询字符串或批量 URL。

## 6. 数据库迁移

保留 `app_media` 作为列表封面表，新增下一序号迁移，例如 `migrations/0016_steam_media_gallery.sql`：

```sql
CREATE TABLE app_media_assets (
    app_id INTEGER NOT NULL REFERENCES apps (app_id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('screenshot', 'movie')),
    source_id TEXT NOT NULL,
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    title TEXT,
    thumbnail_url TEXT NOT NULL,
    full_url TEXT,
    mp4_url TEXT,
    hls_h264_url TEXT,
    dash_h264_url TEXT,
    is_highlight INTEGER NOT NULL DEFAULT 0 CHECK (is_highlight IN (0, 1)),
    source TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (app_id, kind, source_id)
);

CREATE INDEX idx_app_media_assets_order
    ON app_media_assets (app_id, kind, sort_order, source_id);
```

约束语义：

- `screenshot`：`thumbnail_url` 和 `full_url` 必填；播放字段必须为 `NULL`。
- `movie`：`thumbnail_url` 是海报，`full_url` 为 `NULL`，至少一个播放字段非空。
- 应用层写入前验证上述互斥约束；如项目迁移规范允许，可再用 `CHECK` 固化。
- 不把这些字段加成 `app_media` 的 JSON 列，避免一行大 JSON 难以增量验证和排序。
- 旧数据库升级后 `app_media_assets` 为空属于正常状态，现有封面必须继续可用。

同步更新 `crates/storage/src/query.rs::latest_data_update_ms`，把
`app_media_assets.updated_at_ms` 纳入全局数据更新时间。

## 7. 入库语义

在 `crates/storage/src/ingest.rs::ingest_store_details` 所在事务内完成：

1. 继续按现有逻辑更新封面。
2. `screenshots == None`：不动现有截图。
3. `screenshots == Some(items)`：删除该 App 的旧 `screenshot`，再批量插入新集合。
4. `movies == None`：不动现有视频。
5. `movies == Some(items)`：删除该 App 的旧 `movie`，再批量插入新集合。
6. 任何插入失败必须回滚整个本次详情入库，不能留下半套媒体。
7. `appdetails` 请求失败、`success=false`、JSON 结构异常时沿用现有失败策略，绝不能清空旧媒体。

不要为每个媒体发额外 Steam 请求。媒体跟随现有每 App 一次的详情富化响应入库，不增加 Steam 调用量。

## 8. Storage 与 API

### 8.1 Storage 读模型

不要把一对多媒体硬塞进 `GameCandidateRow` 的主查询造成重复行。新增独立读模型和查询：

```rust
pub struct GameMediaAssetRow {
    pub kind: String,
    pub source_id: String,
    pub sort_order: u16,
    pub title: Option<String>,
    pub thumbnail_url: String,
    pub full_url: Option<String>,
    pub mp4_url: Option<String>,
    pub hls_h264_url: Option<String>,
    pub dash_h264_url: Option<String>,
    pub is_highlight: bool,
    pub updated_at_ms: i64,
}
```

在 `Repository` 提供按 `app_id` 一次查询、稳定排序的接口。详情处理允许固定两次查询（游戏主记录 + 媒体集合），禁止按媒体 N+1。

### 8.2 游戏详情 API

保持现有 `cover_url` 和 `cover_updated_at_ms` 不变，在
`GET /v1/games/{app_id}` 增加：

```json
{
  "media": {
    "updated_at_ms": 1784880000000,
    "screenshots": [
      {
        "id": "0",
        "thumbnail_url": "https://shared.akamai.steamstatic.com/...",
        "full_url": "https://shared.akamai.steamstatic.com/..."
      }
    ],
    "videos": [
      {
        "id": "257363622",
        "title": "1.0 Release Date Reveal Trailer",
        "poster_url": "https://shared.akamai.steamstatic.com/...",
        "highlight": true,
        "mp4_url": null,
        "hls_h264_url": "https://video.akamai.steamstatic.com/...",
        "dash_h264_url": "https://video.akamai.steamstatic.com/..."
      }
    ]
  }
}
```

契约要求：

- `media` 在新服务端响应中始终存在。
- 无媒体时返回 `screenshots: []`、`videos: []`、`updated_at_ms: null`，不返回 `null` 数组。
- 只返回已通过服务端白名单校验的 URL。
- `updated_at_ms` 为该 App 媒体行最大更新时间。
- 字段是向后兼容增量，不删除或改名现有封面字段。
- 若项目 API 版本规则要求对新增字段升级版本，按现有规则执行；客户端仍需忽略未知字段。
- 缓存 ETag/离线缓存的响应体自然包含媒体；媒体刷新后 ETag 必须变化。

同步更新：

- `apps/server/src/api.rs` 的 schema/序列化和 API 测试；
- `web/src/api/types.ts::GameDetail`；
- `docs/API.md`；
- 如存在 OpenAPI 快照或生成物，按仓库现有流程更新。

## 9. Web UI

### 9.1 组件划分

新增 `web/src/components/GameMediaGallery.tsx`，不要破坏
`web/src/components/GameMedia.tsx` 的列表单图职责。

建议内部类型：

```ts
type GalleryItem =
  | {
      kind: "cover" | "screenshot";
      id: string;
      thumbnailUrl: string;
      fullUrl: string;
      alt: string;
    }
  | {
      kind: "video";
      id: string;
      posterUrl: string;
      title: string;
      mp4Url: string | null;
      hlsUrl: string | null;
      dashUrl: string | null;
    };
```

组装顺序：

1. 当前封面；
2. `highlight=true` 的视频；
3. 截图；
4. 其余视频。

如封面 URL 与某张截图完全相同则去重。单个资源失败后从可选列表移除或显示可重试海报，不能出现永久破图。

### 9.2 交互

- 主舞台固定约 `16:9`，使用 `aspect-ratio`，防止切换时布局跳动。
- 下方为可横向滚动的缩略图轨道。
- 缩略图必须是 `<button type="button">`，提供可见焦点和 `aria-label`。
- 当前项使用 `aria-current="true"` 或 `aria-selected="true"`。
- 支持 `ArrowLeft` / `ArrowRight` 在缩略图间切换，`Home` / `End` 跳到首尾。
- 图片使用现有的 `loading="lazy"`、`decoding="async"`、`referrerPolicy="no-referrer"`。
- 主图点击后打开现有 `Modal` 风格的灯箱；`Escape` 关闭、焦点返回触发按钮，不能触发详情页返回。
- 视频海报显示明确的播放按钮；只有点击后才创建/激活实际播放器。
- `<video controls playsInline preload="metadata">`，禁止 `autoplay` 和循环。
- 切换离开视频时暂停并释放 HLS 实例；组件卸载时同样清理。
- 尊重 `prefers-reduced-motion`，不要添加自动轮播和大幅切换动画。
- 图片、视频失败不得影响右侧标题、投票、Steam 跳转和详情面板。

### 9.3 视频播放策略

按以下优先级尝试：

1. 有 `mp4_url`：原生 `<video>`。
2. 浏览器原生支持 HLS 且有 `hls_h264_url`：原生 `<video>`。
3. 有 HLS 且支持 Media Source Extensions：使用 `hls.js`，仅在用户点击播放时动态加载/初始化。
4. 上述都不可用或播放报错：保留海报，显示“当前环境无法播放预告片”，提供现有 `steam_url` 的“在 Steam 查看”链接。

DASH URL 本次可保存并透传，但没有现成播放器时不要为了 DASH 再引入第二套运行时。不要直接把 `.m3u8` 当成在所有 Chromium/WebView 中都可原生播放。

如果采用 `hls.js`：

- 用工作区现有 pnpm 方式把依赖加到 `web/package.json`。
- 禁止 CDN 脚本和运行时远程注入。
- 销毁 `Hls` 实例并移除事件监听。
- 网络/媒体错误只影响当前视频，不弹出阻塞式全局错误。

### 9.4 详情页接入

在 `web/src/screens/GameDetailScreen.tsx`：

- 用 `GameMediaGallery` 替换 Hero 左侧当前单一 `GameMedia`。
- 把 `game.cover_url`、`game.media`、`game.steam_url`、名称和 AppID 传入。
- 加载骨架保持 16:9，避免加载完成后高度突变。
- `media` 缺失时要兼容旧服务端：按空媒体处理并只展示现有封面。

在 `web/src/styles/screens/game-detail.css`：

- 保留现有两列 Hero 布局和 `760px` 以下单列逻辑。
- 新画廊不能撑破 `minmax(260px, 400px)` 的左列。
- 缩略图轨道允许横向滚动，不能让整个页面横向溢出。
- 所有颜色、圆角、阴影使用现有主题 token。
- 在 `1024×640`、`1280×800`、`760px` 和 `390px` 宽度验证。

## 10. CSP 与安全

当前 `web/index.html` 和 `apps/desktop/src-tauri/tauri.conf.json` 没有
`media-src`，远程视频会回退到 `default-src 'self'` 而被阻断。

两处 CSP 必须同步调整：

- `media-src` 只允许经过验证的 Steam 视频 CDN 和必要的 `blob:`。
- 若 `hls.js` 通过 XHR/fetch 拉取清单和分片，`connect-src` 必须允许同一视频 CDN。
- `img-src` 可在不破坏现有资源的前提下收紧到实际 Steam 图片 CDN；本任务至少不能把视频域泛化为任意来源。
- 不增加 `unsafe-eval`。
- Web CSP 与 Tauri CSP 必须具有等价能力，不能只修开发服务器。

服务端 URL 白名单是主安全边界，CSP 是第二层防护。不要只依赖前端字符串判断。

## 11. 失败与降级矩阵

| 场景 | 预期 |
| --- | --- |
| 新服务端 + 有截图/视频 | 展示完整画廊 |
| 新服务端 + 只有截图 | 封面 + 截图，无空视频区域 |
| 新服务端 + 只有视频 | 封面 + 视频海报 |
| 新服务端 + 媒体数组为空 | 完全保持当前单封面体验 |
| 新客户端连接旧服务端 | `media` 缺失时仍展示封面 |
| 某缩略图 404 | 仅跳过该项，不影响其他媒体 |
| 主图加载失败 | 切到下一个有效项；全部失败则显示 `GameMedia` 字母占位 |
| 视频不支持/加载失败 | 海报 + 错误提示 + Steam 链接 |
| `appdetails` 暂时失败 | 数据库保留上一次成功媒体 |
| 响应缺少 `screenshots` / `movies` | 保留对应旧集合 |
| 响应明确返回空数组 | 清空对应旧集合，封面不受影响 |
| 离线快照 | 已缓存详情媒体元数据可展示；远程资源不可用时平稳降级 |

## 12. 测试要求

### 12.1 `steam-source`

扩展录制且脱敏的 fixture，使其至少包含：

- 两张有序截图；
- 一个 HLS/DASH 视频；
- 一个旧式 MP4 视频；
- 重复 ID；
- 非 `steamstatic.com` URL；
- `http://` URL；
- 缺海报或缺播放地址的视频；
- 字段缺失与显式空数组。

断言解析顺序、去重、上限、URL 拒绝和 `None` / `Some([])` 语义。

### 12.2 Storage / migration

至少覆盖：

- 从当前最新旧版本带真实数据升级，封面不丢失；
- 首次写入截图/视频；
- 新快照事务替换并维持顺序；
- 字段缺失保留旧行；
- 显式空数组清空对应 kind；
- 非法写入整体回滚；
- 删除 App 后媒体级联删除；
- `latest_data_update_ms` 包含新表。

### 12.3 API

至少覆盖：

- 有媒体详情的完整 JSON；
- 无媒体时空数组和 `updated_at_ms: null`；
- URL、布尔值和 ID 的稳定序列化；
- 刷新媒体后 ETag 改变；
- Feed/搜索/日历响应没有新增大媒体数组；
- 旧的 `cover_url` 契约测试继续通过。

### 12.4 Web

使用现有 Vitest/Testing Library 方式覆盖：

- `media` 缺失、空数组和完整数组；
- 默认项顺序；
- 点击/键盘切换；
- 当前缩略图可访问状态；
- 图片错误后切换；
- 视频在用户点击前没有开始加载播放；
- MP4/HLS 能力分支；
- 视频错误回退 Steam 链接；
- 切换和卸载时播放器清理；
- 灯箱 Escape 与详情页 Escape 不冲突。

### 12.5 手工与桌面验收

用至少三个真实游戏验收：媒体丰富、只有截图、媒体缺失各一个。

- 浏览器开发模式截图。
- Tauri WebView2 实际播放或回退截图。
- 检查控制台无 CSP 拒绝、React key 警告和未处理 Promise。
- 检查慢网、断网和单资源 404。
- 检查 `1024×640`、`1280×800`、`760px`、`390px`。
- 键盘完整操作一次，并用屏幕阅读器检查按钮名称。

## 13. 实施顺序

Grok Build 按以下顺序执行，每一步通过相应测试后再继续：

1. 扩展 Steam fixture、DTO、规范化提案和解析测试。
2. 新增迁移、storage 写入/读取及迁移测试。
3. 扩展服务端游戏详情 DTO、ETag 与 API 文档/测试。
4. 扩展 Web 类型并实现 `GameMediaGallery` 组件测试。
5. 接入 `GameDetailScreen`，完成响应式样式。
6. 加入按需 HLS 播放与清理；无法可靠通过 Tauri 验收时保留海报回退，不阻断截图 P0。
7. 同步 Web/Tauri CSP。
8. 运行 Rust、Web、构建和桌面相关测试，并记录实际结果。

不要创建 worktree、提交、推送、改 CI、删除文件或大范围重构。不要覆盖仓库当前未跟踪的 `design/` 目录。

## 14. 完成定义

只有同时满足以下条件才算完成：

- 详情页能从数据库/API 展示 Steam 截图，而不是用 AppID 猜 URL。
- 当前封面和所有现有详情功能无回归。
- 视频至少具备海报、用户触发、能力检测和失败回退；若 HLS 在 Tauri 实测通过，则可直接播放。
- Steam 解析失败或字段缺失不会清空旧媒体。
- 新旧服务端/客户端组合可降级。
- URL 白名单与 Web/Tauri CSP 都到位。
- 自动测试通过，真实游戏完成浏览器与 Tauri 验收。
- `docs/API.md`、`docs/DATA_STORAGE.md`、`docs/SOURCES.md` 与代码一致。

## 15. Grok Build 最终回报格式

实现完成后只按事实回报：

1. 修改的文件及每个文件的职责。
2. 最终数据库/API 契约。
3. 截图与视频在浏览器、Tauri 中的实际结果。
4. 执行过的测试命令及 PASS/FAIL/SKIP。
5. 仍存在的限制，尤其是 HLS、CSP、Steam 字段漂移和离线媒体。
6. 不要把未执行的测试写成通过。
