# MPGS 公共发现服务架构决策总览

- 文档日期：2026-06-08
- 目的：汇总 grill-with-docs 阶段已经确认的服务端化架构边界，作为后续实现入口

## 1. 目标架构

MPGS 从本地前后端一体的 Tauri 应用，迁移为：

- **使用者客户端**：Tauri 桌面客户端，用户手动输入或导入服务连接文件后连接一个公共发现服务。
- **公共发现服务**：Rust + Axum 服务端，负责公共游戏库、Steam/LLM 集成、发现任务、分析任务、管理界面和公开 REST API。
- **管理界面**：由服务端同源托管的 WebUI，面向维护者，不是普通用户配置入口。

第一版不提供默认官方服务地址。朋友用户仍需要输入一次服务地址或导入服务连接文件，但不需要配置 Steam/LLM Key，也不需要导库。

## 2. 客户端边界

客户端只保留壳能力：

- 保存服务地址和服务身份信息
- 验证服务地址可用性
- 保存个人游戏状态
- 保存公共库只读缓存
- 打开外部链接
- 导入服务连接文件
- 导出/导入同服务实例的个人状态

客户端不再承担：

- Steam 同步
- 游戏发现
- LLM 调用
- AI 批处理
- 公共库写入
- 管理配置
- 客户端候选提交

客户端 React 直接通过 HTTPS REST 调用匿名只读公共 API；Tauri Rust 侧只处理本地壳能力。第一版只连接一个当前服务，可保存最近服务历史，但不做多服务聚合。

## 3. 服务身份与连接

服务端提供公开的服务身份信息：

- 服务实例 ID
- 服务实例名
- 服务版本
- API 版本
- 公共库状态
- 能力 flags

客户端第一版只支持 API `v1`。保存服务地址前必须：

- 获取服务身份信息
- 成功调用至少一个匿名只读公共 API
- 检查 HTTPS 或局域网例外规则
- 检查 API 版本兼容
- 确认当前客户端环境可跨源读取公共 API

服务实例 ID 是首次配置生成的稳定公开 UUID。服务地址变化但实例 ID 相同，客户端视为同一服务迁移地址并沿用个人状态；实例名相同但实例 ID 不同，视为不同服务。

## 4. API 契约

服务端使用 REST + `/api/v1` + OpenAPI。Rust 类型作为 API schema 源头，生成 OpenAPI，再生成 TypeScript 类型。

公共 API：

- `GET /api/v1/service-info`
- `GET /api/v1/discovery-home`
- `GET /api/v1/games`
- `GET /api/v1/games/{appid}`
- `GET /api/v1/games/{appid}/analysis`

管理 API：

- `POST /api/v1/admin/session`
- `GET /api/v1/admin/overview`
- `GET /api/v1/admin/diagnostics`
- 管理配置、任务、审核、连接分享等接口

错误响应使用统一服务错误信封：

```json
{
  "error": {
    "code": "service_config_missing",
    "message": "Steam Key 尚未配置。",
    "requestId": "01HX...",
    "details": {}
  }
}
```

`code` 使用英文稳定枚举，`message` 第一版使用中文。

## 5. 公共库模型

新服务从空 Postgres 公共库启动，不迁移旧本机 SQLite 数据。公共库由首次配置后的服务端发现能力生成。

公共库只保留当前态和刷新记录，不做完整历史快照。普通客户端只读取：

- `review_status = accepted`
- `visibility = public`

待审核、拒绝、隐藏和归档只在管理界面可见。

发现任务按置信度分流：

- 高置信多人游戏：自动 accepted + public
- 中置信候选：needs_review + hidden
- 低置信或明显不匹配：rejected

公共游戏不默认物理删除，使用可见性和审核状态治理。

## 6. 数据库与任务

服务端使用 Postgres，一步到位，不沿用 SQLite 作为服务端目标数据库。

同一个 Postgres 数据库按 schema 区分：

- `public_catalog`：公共库数据、游戏、公开分析、公开查询模型
- `ops`：任务、运行记录、失败项、配置状态、审计日志

数据库访问层使用 SQLx，不使用 ORM。任务队列使用 Postgres-backed jobs，不引入 Redis。

服务端 schema 重新设计，不平移 SQLite 表。旧本机数据不要求导入。

## 7. 配置、密钥与重启

管理 WebUI 提供首次配置引导。首次配置需要部署级引导令牌进入。

首次配置必填：

- admin token
- Steam Key

可跳过：

- LLM 配置
- R2 图片缓存配置
- 调度/限额高级配置

密钥不写数据库。WebUI 写挂载配置目录中的 TOML 文件：

- `service.toml`：非敏感配置
- `secrets.toml`：Steam/LLM/R2/admin token hash 等敏感配置

Compose `.env` 只用于定位 config/data 目录或选择部署模式，WebUI 不直接修改 `.env`。

配置使用 active/pending 双槽。WebUI 写 pending；服务重启前校验 pending；启动成功后 promote active。pending 密钥使用 patch/继承语义，避免未回显密钥被误清空。

第三方 API Key 以服务可用原文保存在受保护配置文件中，不回显、不写数据库、不进日志。admin token 只保存 hash，不保存明文。

一键重启不使用 restart-helper，不控制 Docker，不挂 Docker socket。服务在管理确认后校验配置、记录审计日志、优雅退出自身进程，由 Docker Compose restart policy 拉起。

## 8. 安全与鉴权

普通客户端使用匿名只读访问，不做客户端登录。管理界面必须鉴权。

第一版管理鉴权：

- 单管理员 token 登录
- 换短期 HttpOnly session cookie
- admin token 只存 hash
- 不做多管理员、注册、找回密码

首次配置完成后，引导令牌不再授予正常管理访问，但可作为 safe mode repair 凭据。

生产环境强制 HTTPS。客户端默认拒绝公网 HTTP，只允许 localhost 或显式局域网例外。

CORS 分层：

- 公共只读 API 可配置跨源开放
- 管理/setup/重启接口只允许同源管理界面

服务访问限流覆盖匿名读、管理登录、setup 和一键重启；管理/setup/重启更严格。

## 9. Safe Mode 与健康检查

服务端提供最小安全修复模式。触发条件包括：

- active 配置损坏
- pending 回滚失败
- 必要配置文件缺失
- admin token 配置不可用

safe mode 只开放健康检查和受引导令牌保护的配置修复入口，不开放公共库 API、任务或第三方 API 调用。

公开 health 极简；详细诊断走鉴权管理接口。

容器 healthcheck 检查：

- 进程响应
- Postgres 连接
- migration 状态
- active 配置可读

不检查：

- Steam
- LLM
- R2
- 公共库是否为空

配置回滚由服务启动逻辑处理，不依赖 Docker healthcheck 自动回滚。

## 10. 管理界面

管理界面由 `mpgs-server` 同源托管静态资源。

管理首页叫“管理概览”，通过 `/api/v1/admin/overview` 获取数据，包含：

- 服务状态
- 配置状态
- 公共库规模
- 待审核数量
- 最近任务
- 失败摘要
- 重启需求
- 连接分享入口

管理界面还提供：

- 首次配置引导
- 轻量部署诊断区
- 手动 AppID 添加
- 待审核处理
- 任务控制
- 连接文件下载
- 个人不可见的运维日志

审核动作第一版只有：

- 接受并公开
- 接受但隐藏
- 拒绝
- 归档
- 可选备注

## 11. 客户端界面

普通客户端第一屏叫“发现首页”，对应 API `/api/v1/discovery-home`。发现首页只展示普通用户可见的公共库摘要和分区预览，不展示管理任务、待审核、密钥或成本状态。

空库时，客户端展示空库客户端状态：

- 服务已连接
- 公共库尚未生成
- 重新检查按钮

不显示：

- Steam Key 配置
- LLM Key 配置
- 发现任务入口

## 12. 图片策略

第一版默认返回经过服务端校验的 Steam CDN 图片 URL。

R2 作为可选图片缓存层：

- 不属于首次配置必填
- 凭据不写数据库
- 改配置后重启生效
- 图片缓存异步回填
- 不阻塞发现、入库或公开

客户端展示优先级：

```text
cached_image_url ?? original_image_url
```

## 13. 缓存与离线

客户端支持公共库只读缓存，用于弱离线浏览。缓存不产生公共数据，也不替代服务端。

公共读 API 支持 ETag/If-None-Match：

- 列表 ETag 基于查询参数 + 公共库修订号
- 详情 ETag 基于单游戏和分析更新时间

公共库修订号只在影响匿名只读结果时递增。

个人状态导出只包含个人状态和服务连接元信息，不包含公共库缓存、分析缓存或图片缓存。

## 14. 部署与发布

第一版正式自托管部署只支持 Docker Compose。裸机运行只作为开发或高级自理方式。

默认 Compose 不强制内置反向代理，但提供 Caddy profile 作为官方 HTTPS 示例。

首版发布产物：

- Windows Tauri 客户端安装包
- `mpgs-server` Docker 镜像
- Docker Compose 部署文件
- 可选 Caddy Compose profile
- 示例配置文件
- 生成的 OpenAPI JSON

不发布：

- restart-helper 镜像
- Helm chart
- macOS/Linux 客户端包
- 自动云部署

客户端和服务端共享项目发布版本号，同一 GitHub Release 发布；API 兼容由 API version 控制。

## 15. 第一实施切片

建议第一切片：

1. 创建根 Cargo workspace。
2. 创建 `crates/mpgs-core`。
3. 只抽纯模型、评分、推荐和映射/规则逻辑。
4. 创建 `crates/mpgs-server`。
5. 实现 `GET /api/v1/service-info`。
6. 生成 OpenAPI。
7. 客户端新增服务地址验证模型。

第一切片不做：

- Postgres 大迁移
- 任务系统迁移
- admin-ui 完整实现
- R2 缓存
- Docker 发布流水线

这个切片用于验证新的服务边界，而不是一次性重写整个项目。
