# 外部资料与核验记录

最后核验日期：2026-07-13。

本文件只记录影响设计的外部事实。算法权重、MVP 范围和架构取舍属于项目决策，不声称来自这些来源。

## Steam

- [IStoreService/GetAppList](https://partner.steamgames.com/doc/webapi/IStoreService)：需要 Web API Key；支持 `last_appid` 分页、`if_modified_since` 增量、`last_modified` 与类型过滤。
- [ISteamUserStats/GetNumberOfCurrentPlayers](https://partner.steamgames.com/doc/webapi/isteamuserstats)：返回指定 App 当前连接 Steam 的玩家数，不包含未连接 Steam 的玩家。
- [Steam Store Reviews API](https://partner.steamgames.com/doc/store/getreviews)：返回 `query_summary`、正/负/总评价和分页评论。
- [Steam Demos](https://partner.steamgames.com/doc/store/application/demos)：Demo 是独立 AppID，可在本体发售前发布。
- [Steam Web API Terms](https://steamcommunity.com/dev/apiterms)：页面标注 2010 年 7 月更新；包含每日 100,000 次调用、Key 保密、用户数据按需获取和隐私政策要求。条款可能变更，上线前必须再次审查。

设计结论：官方接口能覆盖目录、单游戏 CCU 和评论，但不能以一个稳定公开接口完整覆盖发售日历、Demo 关系和熟人联机质量。商店适配器必须在 M1 单独验证。

M1 核验结果（2026-07-14，夹具 Spike）：见 [M1_FEASIBILITY.md](M1_FEASIBILITY.md)。`store.steampowered.com/api/appdetails` 可用作经批准的易变适配器，解析 `coming_soon`、原始发售日与 Demo/`fullgame` 关系；解析失败不得清空权威字段。熟人联机质量以黄金集与人工校正为准。

## SQLite

- [SQLite Over a Network](https://sqlite.org/useovernet.html)：SQLite 文件不应由多台机器通过网络文件系统并发访问；推荐将 SQLite 与应用服务放在同机，由应用 API 代理远程请求。
- [Appropriate Uses For SQLite](https://sqlite.org/whentouse.html)：SQLite 可作为应用服务器后端，但高并发写入和多服务器场景更适合客户端/服务器数据库。

设计结论：MVP 使用本机单主 SQLite；客户端和远程 Worker 通过 HTTP API 访问。active-active 需求触发 PostgreSQL 迁移评估。

## Tauri、Flutter 与 Rust

- [Tauri 2 Prerequisites](https://v2.tauri.app/start/prerequisites/)：桌面开发覆盖 Linux、macOS、Windows，并列出 Android/iOS 工具链；Linux 依赖 WebKitGTK，Windows 使用 WebView2。
- [Tauri Distribution](https://v2.tauri.app/distribute/)：提供桌面与 Android/iOS 的构建、签名和分发入口。
- [Tauri GitHub Pipeline](https://v2.tauri.app/distribute/pipelines/github/)：官方示例包含 Windows x64、Linux x64/ARM64、macOS x64/ARM64；Linux ARM AppImage 另有工具链注意事项。
- [Flutter Supported Platforms](https://docs.flutter.dev/reference/supported-platforms)：核验时文档标注 Flutter 3.44.0、2026-05-20 更新，列出 Windows/macOS/Debian/Ubuntu 的 x64/ARM64 部署支持。
- [Rust Platform Support](https://doc.rust-lang.org/rustc/platform-support.html)：核验时 `aarch64/x86_64` 的 Windows MSVC、Linux GNU 与 macOS 目标属于 Tier 1 host tools 范围。

设计结论：Rust 服务端目标矩阵可行；桌面先选 Tauri 2 以保持 Rust 为主。Windows ARM 客户端打包不作为 MVP 阻塞项，需单独冒烟验证。

## 关键核验命令

```powershell
smart-search search "Official Steam Web API current players app list user reviews upcoming releases demos Store API documentation 2026" --validation balanced --extra-sources 2 --timeout 180 --format json
smart-search fetch "https://partner.steamgames.com/doc/webapi/IStoreService" --format markdown
smart-search fetch "https://partner.steamgames.com/doc/webapi/isteamuserstats" --format markdown
smart-search fetch "https://partner.steamgames.com/doc/store/getreviews" --format markdown
smart-search fetch "https://partner.steamgames.com/doc/store/application/demos" --format markdown
smart-search fetch "https://steamcommunity.com/dev/apiterms" --format markdown
smart-search fetch "https://sqlite.org/useovernet.html" --format markdown
smart-search fetch "https://v2.tauri.app/start/prerequisites/" --format markdown
smart-search fetch "https://docs.flutter.dev/reference/supported-platforms" --format markdown
smart-search fetch "https://doc.rust-lang.org/rustc/platform-support.html" --format markdown
```

