# MPGS / Co-Play

MPGS（界面品牌名为 Co-Play）是一款面向 Steam 多人游戏发现的 Windows 桌面工具。

它的核心用途不是“完整替代 Steam 商店”，而是帮你更快找到适合和朋友一起玩的游戏，再把同步到的 Steam 数据、本地收藏状态和 AI 分析组织成一个更容易浏览和筛选的本地游戏库。

## 这套系统能做什么

- 浏览多人游戏推荐分区，例如新游区、精品老游区、最近发现
- 同步 Steam 游戏基础信息、评价、在线人数等数据
- 扫描 Steam AppList，持续发现新的多人游戏并导入本地库
- 为单个游戏生成 AI 评估摘要和更详细的分析结果
- 管理收藏、愿望单、关注、浏览记录
- 在本地保存配置和数据，不依赖云端账号体系

## 普通用户怎么开始

### 第 1 步：下载 Windows 安装包

优先去项目的 [GitHub Releases](https://github.com/RedAiyo/mpgs/releases) 页面下载 Windows 安装包。

建议按下面的顺序选择：

1. 优先下载名称里带 `windows-x64` 的 Windows 安装包。
2. 如果同时看到 `.msi` 和 `-setup.exe`，一般优先选 `.msi`。
3. 如果 Releases 页面还没有正式安装包，你需要联系维护者索取构建好的安装文件，或者按本文末尾的开发者方式自行构建。

注意：

- 本项目的发布流程已经配置了 Windows x64 桌面打包。
- 当前仓库的发布说明里明确写了 Windows 包还没有代码签名，所以首次运行时出现 SmartScreen 警告是预期现象，不代表一定有问题。

### 第 2 步：安装并启动

下载完成后：

1. 双击安装包完成安装。
2. 第一次打开应用时，如果 Windows 弹出 SmartScreen，可先点“更多信息”，再选择继续打开。
3. 应用启动后，就算你还没有配置任何 API Key，也可以先看到本地初始化的基础内容。

## 使用前要准备什么

如果你想把这套系统真正用起来，通常需要准备两类 Key：

### 必备 1：Steam Web API Key

用途：

- 用来同步 Steam 数据
- 用来扫描和发现新的多人游戏

没有这个 Key 时：

- 你仍然可以打开应用
- 但同步 Steam 数据、扩充本地库这些核心能力会受限

### 可选 2：DeepSeek API Key

用途：

- 用来生成 AI 分析摘要
- 用来做更完整的 AI 游戏评估

没有这个 Key 时：

- 你仍然可以使用同步、浏览、收藏等功能
- 但 AI 分析能力不可用或会退化

## 如何申请 DeepSeek API Key

以下官方入口已于 `2026-05-02` 核对，后续如 DeepSeek 平台改版，请以官方页面最新展示为准。

### 官方入口

- DeepSeek API 文档：[api-docs.deepseek.com](https://api-docs.deepseek.com/)
- DeepSeek 平台：[platform.deepseek.com](https://platform.deepseek.com/)
- API Keys 页面：[platform.deepseek.com/api_keys](https://platform.deepseek.com/api_keys)
- Models & Pricing：[api-docs.deepseek.com/quick_start/pricing](https://api-docs.deepseek.com/quick_start/pricing)

### 操作步骤

1. 打开 [DeepSeek 平台](https://platform.deepseek.com/) 并登录账号。
2. 登录后进入 [API Keys](https://platform.deepseek.com/api_keys) 页面。
3. 创建一个新的 API Key。
4. 把生成后的 Key 立即保存好。

补充说明：

- DeepSeek 官方文档确认它提供 OpenAI 兼容格式接口，因此本项目里可以直接使用 `https://api.deepseek.com` 作为 `LLM Base URL`。
- 本项目当前默认模型配置是 `deepseek-v4-flash`，README 也优先推荐这个模型名。
- 如果后续 AI 调用失败，并提示余额或计费问题，请再去 DeepSeek 平台检查余额、套餐或充值状态。

## 如何申请 Steam Web API Key

以下官方入口已于 `2026-05-02` 核对，后续如 Steam 页面改版，请以官方页面最新展示为准。

### 官方入口

- Steam Web API 文档：[steamcommunity.com/dev](https://steamcommunity.com/dev)
- Steam Web API Key 注册页：[steamcommunity.com/dev/apikey](https://steamcommunity.com/dev/apikey)
- Steamworks 认证说明：[partner.steamgames.com/doc/webapi_overview/auth](https://partner.steamgames.com/doc/webapi_overview/auth)

### 操作步骤

1. 用你的 Steam 账号登录 [Steam Web API Key 注册页](https://steamcommunity.com/dev/apikey)。
2. 阅读并同意 Steam Web API Terms of Use。
3. 按页面要求填写将与该 Key 关联的域名。
4. 提交后保存生成的 Steam Web API Key。

补充说明：

- Valve 官方文档明确说明：标准用户 Key 对所有 Steam 账号开放，但需要一个 Steam 账号和一个将与该 Key 关联的域名。
- Valve 官方文档没有专门给“纯本地桌面应用”提供单独示例，所以如果你在域名这一步拿不准，请以注册页面当时的最新提示为准。

## 在 MPGS 里怎么配置

启动应用后，进入“设置”页，按下面步骤填写。

### 1. 配置 API 密钥

打开：

- `设置` -> `API 密钥`

把下面两个字段填好：

- `Steam Web API Key`
- `LLM API Key`

说明：

- `Steam Web API Key` 对应你刚刚在 Steam 申请到的 Key
- `LLM API Key` 如果你准备用 DeepSeek，就填 DeepSeek 的 API Key

### 2. 配置 LLM 参数

打开：

- `设置` -> `LLM 配置`

推荐按下面填写：

- `LLM Base URL`：`https://api.deepseek.com`
- `模型`：`deepseek-v4-flash`
- `地区`：默认可用 `US`
- `语言`：简体中文推荐 `schinese`

填完后点击：

- `保存设置`

### 3. 你需要知道的保存方式

这几个配置项会保存在本地 SQLite 数据库里，不会主动上传到项目自带服务器。

应用启动时会在系统应用数据目录下创建本地数据库文件：

- `mpgs.sqlite3`

## 配好以后怎么用

### 第 1 步：先做一次完整同步

进入：

- `设置` -> `数据同步`

点击：

- `完整同步`

这一步会尽量把当前库内游戏的商店信息、评价、在线人数和样本数据补齐。

### 第 2 步：开始发现新的多人游戏

进入：

- `设置` -> `发现任务`

然后点击：

- `开始新任务`

你可以在这里看到：

- 当前任务状态
- 进度
- 历史记录
- 失败项

如果中途不想继续，还可以：

- `暂停任务`
- `继续任务`
- `取消任务`

### 第 3 步：回到首页浏览结果

同步和发现完成后，你可以回到首页看这些区域：

- 新游区
- 精品老游区
- 最近发现

你也可以继续打开详情页，查看：

- 标签
- 联机模式
- 好评率
- 当前在线人数
- 发售时间
- Demo 状态

### 第 4 步：用 AI 看更细的分析

如果你已经配置好 DeepSeek API Key，可以：

1. 进入某个游戏详情页。
2. 触发 AI 分析。
3. 查看更完整的分析结果，例如摘要、维度评分、优点、风险和证据。

如果你希望把整个库都重新跑一遍 AI 评分，可以去：

- `设置` -> `AI 批量重算`

## 常见使用场景

### 我只想先试试，不想申请任何 Key

可以。

你可以先安装并打开应用，先看界面和基础内容，但真正的数据同步、发现任务和 AI 分析能力不会完整启用。

### 我只想同步 Steam，不需要 AI

可以。

这种情况下你只需要申请并配置：

- `Steam Web API Key`

### 我已经能同步数据了，还想看 AI 建议

这时再补配：

- `LLM API Key`
- `LLM Base URL`
- `模型`

## 常见问题

### Windows 提示“无法验证发布者”怎么办

当前项目的 Windows 打包流程还没有代码签名，所以 SmartScreen 警告是预期现象。只要安装包来源可信，可以按系统提示继续。

### API Key 保存在哪里

会保存在本机 SQLite 中，不会因为关闭应用就丢失。

### 我改了 Key 以后要不要重装

不用。

直接回到 `设置` 页重新填写并保存即可。

### 为什么我已经填了 DeepSeek Key，但 AI 还是调用失败

常见原因有：

- Key 填错
- `LLM Base URL` 填错
- 模型名不对
- DeepSeek 账户余额、计费或权限状态有问题

推荐先核对：

- Base URL 是否是 `https://api.deepseek.com`
- 模型是否是当前可用模型
- DeepSeek 平台账户是否可正常调用 API

## 给开发者的补充

如果你不是普通用户，而是要从源码启动项目，可以用：

```bash
npm install
npm run tauri dev
```

前端单独预览：

```bash
npm run dev
```

测试：

```bash
npm test
```

## 公共发现服务部署

服务端化后的 `mpgs-server` 使用 Rust + Axum + Postgres，部署基线在 [docs/deployment/mpgs-server-compose.md](D:/AI%20Coding/mpgs/docs/deployment/mpgs-server-compose.md)。

关键约束：

- 镜像必须在本地开发机或 CI 构建，再上传到服务器。
- VPS 只执行 `docker load` 和 `docker compose up`，严禁在 VPS 上编译 Rust 或构建镜像。
- 默认 Compose 只把服务端绑定到 `127.0.0.1:4310`，公网 HTTPS 通过可选 Caddy profile 或外部反代提供。
- 可用 `deploy/scripts/build-mpgs-server-image.ps1` 本地生成镜像 tar，再用 `deploy/scripts/deploy-mpgs-server-remote.ps1` 上传、启动并验证 `/healthz` 与 `/api/v1/service-info`。

## 项目结构

```text
src/                  React 前端页面与组件
src/api/              前端调用后端命令的封装
src/features/         发现任务、库状态等功能模块
src/pages/            首页、详情、设置、收藏、AI 页面
src-tauri/src/        Rust 后端、Steam、数据库、评分、AI 逻辑
src-tauri/tests/      Rust 侧测试
docs/                 规格说明、实现计划、路线文档
```

## 参考链接

- DeepSeek API Docs: <https://api-docs.deepseek.com/>
- DeepSeek Platform: <https://platform.deepseek.com/>
- Steam Web API Docs: <https://steamcommunity.com/dev>
- Steam Web API Auth Docs: <https://partner.steamgames.com/doc/webapi_overview/auth>
