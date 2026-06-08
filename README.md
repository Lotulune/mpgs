# MPGS / Co-Play

MPGS（界面品牌名为 Co-Play）正在从本地一体 Tauri 应用迁移为“轻量使用者客户端 + 自托管公共发现服务”的多人游戏发现系统。

当前主线边界：

- **使用者客户端**：用户安装的 Tauri 桌面壳。保存服务地址、服务身份信息、个人收藏/愿望单/关注/浏览记录，并通过 HTTPS REST 读取公共游戏库。
- **公共发现服务**：Rust + Axum + Postgres 服务端。负责公共游戏库、Steam/LLM 配置、发现任务、AI 分析、管理 API 和管理界面。
- **管理界面**：维护者使用的 WebUI，由 `mpgs-server` 同源托管在 `/admin`。

第一版没有默认官方服务地址。普通用户需要维护者提供服务地址或无密钥连接文件。

## 普通用户怎么开始

1. 安装 Windows 客户端。
2. 输入维护者提供的公共发现服务地址，或导入维护者提供的连接文件。
3. 客户端会读取 `/api/v1/service-info` 并探测一个匿名公共读取接口，确认 API `v1`、公共只读能力、HTTPS 或本机/局域网规则后才保存。
4. 浏览公共游戏库，并在本机保存个人收藏、愿望单、关注和浏览记录。

普通客户端不会要求你填写 Steam Key、LLM Key，也不会执行 Steam 同步、发现任务或 AI 批处理。连接公共发现服务后，这些旧本地命令路径会被冻结；公共游戏数据由服务端维护。

## 客户端本地数据

客户端只保存壳能力相关数据：

- 当前公共发现服务连接
- 服务身份验证结果
- 按服务实例 ID 隔离的个人游戏状态
- 后续可加入的公共库只读缓存

个人状态不会写入公共发现服务。服务地址变化但服务实例 ID 相同，客户端可以沿用该实例下的个人状态；实例 ID 不同则视为另一个服务。

## 维护者自托管服务端

服务端部署基线在 [docs/deployment/mpgs-server-compose.md](docs/deployment/mpgs-server-compose.md)。

关键约束：

- 服务端是 Rust + Axum + Postgres + SQLx。
- API 是 REST `/api/v1`，OpenAPI 由 Rust 类型生成。
- 官方自托管目标是 Docker Compose。
- 镜像必须在本地开发机或 CI 构建，再上传到服务器。
- VPS 只允许执行 `docker load`、`docker compose up -d`、探针和反代检查，严禁在 VPS 上编译 Rust、跑 `npm run build` 或 `docker build`。
- 默认 Compose 只把 `mpgs-server` 暴露到主机 `127.0.0.1:4310`；公网访问应通过 Caddy profile 或外部 HTTPS 反向代理。

本地构建镜像：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/build-mpgs-server-image.ps1 `
  -ImageTag mpgs-server:local `
  -OutputTar mpgs-server-local.tar
```

Arm VPS（例如 `ora_vps`）需要在本地或 CI 构建 `linux/arm64` 镜像：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/build-mpgs-server-image.ps1 `
  -ImageTag mpgs-server:local `
  -OutputTar mpgs-server-linux-arm64.tar `
  -UseBuildx `
  -Platform linux/arm64
```

远程部署脚本只上传镜像 tar 和 Compose 资产，不覆盖远端真实 `.env`、active secrets 或 active service config：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/deploy-mpgs-server-remote.ps1 `
  -RemoteHost ora_vps `
  -RemotePath ~/mpgs-server `
  -ImageTar mpgs-server-linux-arm64.tar `
  -UseCaddy `
  -PublicBaseUrl https://mpgs.example.com
```

## 服务配置与密钥

Compose `.env` 只放容器级值，例如 Postgres 容器密码、镜像名和 Caddy 域名。服务配置放在 TOML 文件中：

- `deploy/config/active/service.toml`：服务身份、公开连接地址、CORS、部署元信息等非敏感配置
- `deploy/config/active/secrets.toml`：数据库 URL、admin token hash、session secret、Steam/LLM/R2 等服务端密钥
- `deploy/config/setup.toml`：首次配置或安全修复用 setup token hash
- `deploy/config/pending/`：管理界面写入的待重启配置

不要把 Steam Key、LLM Key、R2 凭据、setup token 明文或 admin token 明文写进 Postgres、`.env`、文档或日志。

Key 轮换通过管理界面写入 pending 配置并标记 `restartRequired=true`。确认后调用 `/api/v1/admin/restart`，服务校验 pending 配置并优雅退出，由 Docker Compose `restart: unless-stopped` 拉起。服务不会挂 Docker socket，不使用 restart-helper，也不会执行宿主机命令。

## HTTPS 和反代

生产服务地址必须使用 HTTPS。客户端默认拒绝公网 HTTP，只允许 localhost 或显式局域网例外。

默认 Compose 不直接暴露公网端口：

```bash
curl http://127.0.0.1:4310/healthz
curl http://127.0.0.1:4310/api/v1/service-info
```

使用 Caddy profile 时，`deploy/Caddyfile` 会把 HTTPS 域名反代到 `mpgs-server:4310`：

```bash
docker compose --env-file deploy/.env \
  -f deploy/compose.yml \
  -f deploy/compose.caddy.yml \
  --profile caddy \
  up -d
```

公共客户端只需要访问匿名只读 API；管理、setup 和 restart 接口保持同源管理界面访问。

## 备份与恢复

第一版备份以本地运维命令为准，不包含云备份。

建议同时备份：

- Postgres 数据库：`docker compose --env-file deploy/.env -f deploy/compose.yml exec -T postgres pg_dump -U mpgs -d mpgs > mpgs.sql`
- 服务配置目录：`deploy/config/active/`、`deploy/config/setup.toml`，以及需要保留的 pending 配置
- 部署 `.env`：只在安全介质中保存，不提交到 Git

恢复时先恢复 `deploy/.env` 与配置文件，再恢复 Postgres dump，最后执行 `docker compose up -d` 并探测 `/healthz` 与 `/api/v1/service-info`。

## OpenAPI 和类型生成

服务端 OpenAPI 从 Rust 类型导出：

```powershell
cargo run -p mpgs-server -- --export-openapi > docs/openapi/mpgs-server.openapi.json
```

TypeScript API 类型由 OpenAPI JSON 生成：

```powershell
npm run generate:api-types
```

提交 API 变更时，应同时提交 OpenAPI JSON 与生成的 `src/api/generated/mpgsServerApi.ts`，并运行相关契约测试。

## 开发命令

安装依赖：

```bash
npm install
```

前端开发：

```bash
npm run dev
```

桌面客户端开发：

```bash
npm run tauri dev
```

服务端测试：

```bash
cargo test -p mpgs-server
```

前端测试：

```bash
npm test
```

部署契约检查：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File docs/deployment/deployment_contract_tests.ps1
```

## 项目结构

```text
src/                  React 使用者客户端、管理入口和共享前端代码
src/api/              REST/API 适配和旧 Tauri 命令过渡封装
src/domain/           客户端服务连接、个人状态和推荐规则模型
src-tauri/            Tauri 本地壳与旧本地能力代码，后续逐步冻结/移除
crates/mpgs-core/     纯 Rust 领域模型、评分、推荐和规则逻辑
crates/mpgs-server/   Axum REST API、Postgres、OpenAPI、管理 API
deploy/               Docker Compose、Caddy、配置样例和部署脚本
docs/                 ADR、路线、OpenAPI 和部署文档
```

## 参考入口

- 服务端部署：[docs/deployment/mpgs-server-compose.md](docs/deployment/mpgs-server-compose.md)
- 架构决策：[docs/2026-06-08-public-service-architecture-decisions.md](docs/2026-06-08-public-service-architecture-decisions.md)
- 迁移路线：[docs/2026-06-07-public-discovery-service-migration-roadmap.md](docs/2026-06-07-public-discovery-service-migration-roadmap.md)
