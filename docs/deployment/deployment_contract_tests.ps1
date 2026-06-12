$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")

$compose = Get-Content -Raw -Path (Join-Path $root "deploy\compose.yml")
$caddyCompose = Get-Content -Raw -Path (Join-Path $root "deploy\compose.caddy.yml")
$caddyfile = Get-Content -Raw -Path (Join-Path $root "deploy\Caddyfile")
$dockerfile = Get-Content -Raw -Path (Join-Path $root "Dockerfile.mpgs-server")
$dockerignore = Get-Content -Raw -Path (Join-Path $root ".dockerignore")
$readme = Get-Content -Raw -Path (Join-Path $root "README.md")
$deploymentDoc = Get-Content -Raw -Path (Join-Path $root "docs\deployment\mpgs-server-compose.md")
$productionValidationDocPath = Join-Path $root "docs\deployment\production-validation.md"
if (-not (Test-Path $productionValidationDocPath)) {
    throw "production validation doc must exist."
}
$productionValidationDoc = Get-Content -Raw -Path $productionValidationDocPath
$serviceConfigExample = Get-Content -Raw -Path (Join-Path $root "deploy\config\active\service.toml")
$secretsExample = Get-Content -Raw -Path (Join-Path $root "deploy\config\active\secrets.toml.example")
$setupExample = Get-Content -Raw -Path (Join-Path $root "deploy\config\setup.toml.example")
$localBuildScript = Get-Content -Raw -Path (Join-Path $root "deploy\scripts\build-mpgs-server-image.ps1")
$remoteDeployScript = Get-Content -Raw -Path (Join-Path $root "deploy\scripts\deploy-mpgs-server-remote.ps1")
$remotePreflightScriptPath = Join-Path $root "deploy\scripts\test-mpgs-server-remote-preflight.ps1"
if (-not (Test-Path $remotePreflightScriptPath)) {
    throw "remote preflight script must exist."
}
$remotePreflightScript = Get-Content -Raw -Path $remotePreflightScriptPath
$publicCatalogLiveScriptPath = Join-Path $root "deploy\scripts\test-mpgs-public-catalog-live.ps1"
$productionReadinessScriptPath = Join-Path $root "deploy\scripts\test-mpgs-production-readiness.ps1"
$releaseReadinessScriptPath = Join-Path $root "deploy\scripts\test-mpgs-release-readiness.ps1"
foreach ($scriptPath in @($publicCatalogLiveScriptPath, $productionReadinessScriptPath, $releaseReadinessScriptPath)) {
    if (-not (Test-Path $scriptPath)) {
        throw "validation script must exist: $scriptPath"
    }
}
$publicCatalogLiveScript = Get-Content -Raw -Path $publicCatalogLiveScriptPath
$productionReadinessScript = Get-Content -Raw -Path $productionReadinessScriptPath
$releaseReadinessScript = Get-Content -Raw -Path $releaseReadinessScriptPath
$releaseWorkflow = Get-Content -Raw -Path (Join-Path $root ".github\workflows\release.yml")

if ($compose -notmatch 'postgres:16-bookworm') {
    throw "compose.yml must define Postgres 16."
}
if ($compose -notmatch '127\.0\.0\.1:4310:4310') {
    throw "compose.yml must bind mpgs-server to localhost by default."
}
if ($compose -notmatch '/healthz') {
    throw "compose.yml must healthcheck /healthz."
}
if ($compose -notmatch 'MPGS_CONFIG_DIR') {
    throw "compose.yml must locate the server config directory with MPGS_CONFIG_DIR."
}
if ($compose -notmatch './config:/var/lib/mpgs/config' -or $compose -match './config:/var/lib/mpgs/config:ro') {
    throw "compose.yml must mount the server config directory writable for setup-managed TOML files."
}
if ($compose -match 'MPGS_DATABASE_URL') {
    throw "compose.yml must not put service settings or database URL in environment variables."
}
if ($caddyCompose -notmatch '--profile caddy' -and $deploymentDoc -notmatch '--profile caddy') {
    throw "deployment docs must describe the caddy profile."
}
if ($caddyfile -notmatch 'reverse_proxy mpgs-server:4310') {
    throw "Caddyfile must reverse proxy to mpgs-server:4310."
}
if ($dockerfile -notmatch 'ARG RUST_BASE_IMAGE=rust:1-bookworm' -or $dockerfile -notmatch 'ARG NODE_BASE_IMAGE=node:22-bookworm' -or $dockerfile -notmatch 'ARG DEBIAN_BASE_IMAGE=debian:bookworm-slim') {
    throw "Dockerfile must expose base image build args for registry mirror or pre-pulled image fallback."
}
if ($dockerfile -notmatch 'cargo build --release -p mpgs-server') {
    throw "Dockerfile must build only the mpgs-server package."
}
if ($dockerfile -notmatch 'FROM \$\{NODE_BASE_IMAGE\} AS frontend-builder' -or $dockerfile -notmatch 'npm run build') {
    throw "Dockerfile must build the management UI locally or in CI before runtime image assembly."
}
if ($dockerfile -notmatch 'COPY --from=frontend-builder /workspace/dist /usr/local/share/mpgs/admin' -or $dockerfile -notmatch 'MPGS_ADMIN_STATIC_DIR=/usr/local/share/mpgs/admin') {
    throw "Dockerfile must copy the built management UI into the runtime image and point MPGS_ADMIN_STATIC_DIR at it."
}
if ($dockerignore -notmatch 'src-tauri/target-test/' -or $dockerignore -notmatch '\*\.tar' -or $dockerignore -notmatch 'node_modules/') {
    throw ".dockerignore must exclude local build caches, image archives, and node_modules from the server image build context."
}
if ($deploymentDoc -notmatch 'Do not build on the VPS' -or $deploymentDoc -notmatch 'must not compile Rust') {
    throw "deployment docs must forbid VPS builds."
}
if ($deploymentDoc -notmatch '/admin' -or $deploymentDoc -notmatch 'MPGS_ADMIN_STATIC_DIR=/usr/local/share/mpgs/admin') {
    throw "deployment docs must describe same-origin admin UI hosting and the image static directory."
}
if ($deploymentDoc -notmatch 'active/service.toml' -or $deploymentDoc -notmatch 'active/secrets.toml') {
    throw "deployment docs must describe the active TOML config files."
}
if ($serviceConfigExample -notmatch '\[service_connection\]' -or $serviceConfigExample -notmatch 'public_base_url') {
    throw "active service config example must include the public service connection URL."
}
if ($serviceConfigExample -notmatch '\[public_cors\]' -or $serviceConfigExample -notmatch 'allow_any_origin') {
    throw "active service config example must include the public read CORS setting."
}
if ($serviceConfigExample -notmatch '\[deployment\]' -or $serviceConfigExample -notmatch 'restart_policy = "compose:unless-stopped"') {
    throw "active service config example must include the Compose restart policy metadata."
}
if ($deploymentDoc -notmatch 'service_connection\.public_base_url' -or $deploymentDoc -notmatch 'connection-share API') {
    throw "deployment docs must describe the public base URL used for keyless connection sharing."
}
if ($deploymentDoc -notmatch 'public_cors' -or $deploymentDoc -notmatch 'Management, setup, and restart routes stay same-origin') {
    throw "deployment docs must describe public-only CORS boundaries."
}
if ($deploymentDoc -notmatch 'only locates the config directory') {
    throw "deployment docs must state that .env only locates config, not service secrets."
}
if ($secretsExample -notmatch '\[admin\]' -or $secretsExample -notmatch 'token_hash' -or $secretsExample -notmatch 'session_secret') {
    throw "secrets.toml.example must include admin token hash and session secret placeholders."
}
if ($setupExample -notmatch '\[setup\]' -or $setupExample -notmatch 'token_hash') {
    throw "setup.toml.example must include a setup token hash placeholder."
}
if ($deploymentDoc -notmatch 'setup.toml' -or $deploymentDoc -notmatch 'setup token') {
    throw "deployment docs must describe setup token configuration."
}
if ($deploymentDoc -notmatch 'deploy/config/pending' -or $deploymentDoc -notmatch 'restartRequired=true') {
    throw "deployment docs must describe pending config and restart-required state."
}
if ($deploymentDoc -notmatch 'validates pending service configuration before promoting it to active') {
    throw "deployment docs must describe startup pending config validation and promotion."
}
if ($deploymentDoc -notmatch '/api/v1/admin/restart' -or $deploymentDoc -notmatch 'restart: unless-stopped') {
    throw "deployment docs must describe the managed restart API and Compose restart policy."
}
if ($deploymentDoc -notmatch '/api/v1/admin/diagnostics' -or $deploymentDoc -notmatch 'public base URL' -or $deploymentDoc -notmatch 'provider configuration presence') {
    throw "deployment docs must describe admin diagnostics for deployment status."
}
if ($deploymentDoc -notmatch 'does not use the Docker socket' -or $deploymentDoc -notmatch 'restart-helper' -or $deploymentDoc -notmatch 'host commands') {
    throw "deployment docs must forbid Docker socket, restart-helper, and host command restart control."
}
if ($deploymentDoc -notmatch 'must not clear Steam, LLM, R2, or admin credentials') {
    throw "deployment docs must document pending secret inheritance boundaries."
}
if ($deploymentDoc -notmatch 'Do not put the raw admin token') {
    throw "deployment docs must forbid storing the raw admin token."
}
if ($deploymentDoc -notmatch 'Do not put the raw setup token') {
    throw "deployment docs must forbid storing the raw setup token."
}
if ($localBuildScript -notmatch 'Dockerfile\.mpgs-server' -or $localBuildScript -notmatch 'docker save' -or $localBuildScript -notmatch 'buildx') {
    throw "local build script must build and save the server image locally, including buildx platform support."
}
if ($localBuildScript -notmatch 'RustBaseImage' -or $localBuildScript -notmatch 'RUST_BASE_IMAGE' -or $localBuildScript -notmatch 'NODE_BASE_IMAGE' -or $localBuildScript -notmatch 'DEBIAN_BASE_IMAGE') {
    throw "local build script must pass through base image overrides for constrained registry environments."
}
if ($remoteDeployScript -notmatch 'load -i' -or $remoteDeployScript -notmatch 'compose --project-name "__PROJECT_NAME__"' -or $remoteDeployScript -notmatch 'up -d' -or $remoteDeployScript -notmatch '/healthz' -or $remoteDeployScript -notmatch '/api/v1/service-info') {
    throw "remote deploy script must load the image, start compose, and probe healthz plus service-info."
}
if ($remoteDeployScript -notmatch 'UseSudoDocker' -or $remoteDeployScript -notmatch 'sudo -n docker') {
    throw "remote deploy script must support sudo Docker for hosts where the deploy user cannot access the Docker socket directly."
}
if ($remoteDeployScript -notmatch 'deploy/config/active/service.toml.example') {
    throw "remote deploy script must upload the service config example without targeting the active service config path."
}
$unsafeServiceConfigUpload = $remoteDeployScript -match 'Join-Path\s+\$root\s+"deploy/config/active/service\.toml"\),\s+\(Format-RemoteTarget\s+\$RemoteHost\s+"\$RemotePath/deploy/config/active/(service\.toml)?"\)'
if ($unsafeServiceConfigUpload) {
    throw "remote deploy script must not scp service.toml directly into deploy/config/active where it can overwrite active service config."
}
if ($remoteDeployScript -notmatch 'cp -n deploy/config/active/service.toml.example deploy/config/active/service.toml') {
    throw "remote deploy script must initialize missing active service config with cp -n from the uploaded example."
}
if ($remoteDeployScript -match 'cargo build|rustc|docker build|docker compose build|npm run|pnpm|yarn') {
    throw "remote deploy script must not compile or build artifacts on the VPS."
}
if ($remotePreflightScript -notmatch 'UseSudoDocker' -or $remotePreflightScript -notmatch 'sudo -n docker') {
    throw "remote preflight script must support sudo Docker checks."
}
if ($remotePreflightScript -notmatch 'docker compose version' -or $remotePreflightScript -notmatch 'CADDY_DOMAIN' -or $remotePreflightScript -notmatch '/healthz') {
    throw "remote preflight script must check compose, Caddy/public URL inputs, and health probe targets."
}
if ($remotePreflightScript -notmatch 'docker_ps_output="\$\(__DOCKER_COMMAND__ ps' -or $remotePreflightScript -match 'if __DOCKER_COMMAND__ ps') {
    throw "remote preflight script must fail on docker ps access errors instead of treating them as available public ports."
}
if ($remotePreflightScript -match 'cargo build|rustc|docker build|docker compose build|npm run|pnpm|yarn|docker load|up -d') {
    throw "remote preflight script must be read-only and must not build, load, or start services."
}
if ($deploymentDoc -notmatch 'deploy/scripts/build-mpgs-server-image.ps1' -or $deploymentDoc -notmatch 'deploy/scripts/deploy-mpgs-server-remote.ps1') {
    throw "deployment docs must describe the checked local build and remote deploy scripts."
}
if ($deploymentDoc -notmatch 'linux/arm64' -or $deploymentDoc -notmatch 'ora_vps') {
    throw "deployment docs must describe local arm64 image creation for ora_vps-style servers."
}
if ($deploymentDoc -notmatch "-RemotePath\s+'~/mpgs-server'" -or $readme -notmatch "-RemotePath\s+'~/mpgs-server'") {
    throw "PowerShell remote deploy examples must quote ~/mpgs-server so it is not expanded as a local Windows path."
}
if ($deploymentDoc -notmatch 'does not overwrite remote `deploy/.env`, active secrets, or active service config') {
    throw "deployment docs must state that remote deployment does not overwrite server secrets or active config."
}
if ($deploymentDoc -notmatch 'Client Connection Handoff' -or $deploymentDoc -notmatch '/api/v1/service-info' -or $deploymentDoc -notmatch 'public_catalog_read') {
    throw "deployment docs must describe client service connection validation."
}
if ($deploymentDoc -notmatch 'Tauri Rust must not proxy public catalog reads' -or $deploymentDoc -notmatch 'Personal game state stays in client local storage') {
    throw "deployment docs must describe client public REST and local personal-state boundaries."
}
if ($deploymentDoc -notmatch 'Key Rotation' -or $deploymentDoc -notmatch 'never the raw token' -or $deploymentDoc -notmatch 'restartRequired=true') {
    throw "deployment docs must describe key rotation and pending restart boundaries."
}
if ($deploymentDoc -notmatch 'Backup and Restore' -or $deploymentDoc -notmatch 'pg_dump' -or $deploymentDoc -notmatch 'database-only backup is not enough') {
    throw "deployment docs must describe Postgres plus TOML backup and restore."
}
if ($deploymentDoc -notmatch 'OpenAPI Generation' -or $deploymentDoc -notmatch '--export-openapi' -or $deploymentDoc -notmatch 'npm run generate:api-types') {
    throw "deployment docs must describe OpenAPI and TypeScript contract generation."
}
if ($deploymentDoc -notmatch 'Production Validation' -or $deploymentDoc -notmatch 'test-mpgs-production-readiness\.ps1' -or $deploymentDoc -notmatch 'test-mpgs-public-catalog-live\.ps1') {
    throw "deployment docs must link production readiness and live public catalog validation scripts."
}
if ($productionValidationDoc -notmatch 'Real Public Catalog Validation' -or $productionValidationDoc -notmatch 'Production Deployment Validation' -or $productionValidationDoc -notmatch 'Release Readiness Validation') {
    throw "production validation doc must cover real catalog, production deployment, and release readiness validation."
}
if ($productionValidationDoc -notmatch 'test-mpgs-public-catalog-live\.ps1' -or $productionValidationDoc -notmatch 'test-mpgs-production-readiness\.ps1' -or $productionValidationDoc -notmatch 'test-mpgs-release-readiness\.ps1') {
    throw "production validation doc must include all three validation script entrypoints."
}
if ($publicCatalogLiveScript -notmatch 'AllowSampleCatalog' -or $publicCatalogLiveScript -notmatch 'public_catalog_read' -or $publicCatalogLiveScript -notmatch '/api/v1/games/\$appid/analysis') {
    throw "live public catalog script must reject sample-only catalogs by default and validate public analysis."
}
if ($publicCatalogLiveScript -notmatch 'shortDescription' -or $publicCatalogLiveScript -notmatch 'storeScreenshotUrls' -or $publicCatalogLiveScript -notmatch 'multiplayerModes') {
    throw "live public catalog script must validate rich catalog display fields."
}
if ($productionReadinessScript -notmatch 'RequireAdminDiagnostics' -or $productionReadinessScript -notmatch 'MPGS_ADMIN_TOKEN' -or $productionReadinessScript -notmatch '/api/v1/admin/diagnostics') {
    throw "production readiness script must support optional admin diagnostics through MPGS_ADMIN_TOKEN."
}
if ($productionReadinessScript -notmatch 'RequirePublicCors' -or $productionReadinessScript -notmatch '/admin' -or $productionReadinessScript -notmatch 'connection-share') {
    throw "production readiness script must validate public CORS, same-origin admin hosting, and connection sharing."
}
if ($releaseReadinessScript -notmatch 'package\.json' -or $releaseReadinessScript -notmatch 'src-tauri\\tauri\.conf\.json' -or $releaseReadinessScript -notmatch '--export-openapi') {
    throw "release readiness script must validate version alignment and OpenAPI regeneration."
}
if ($releaseReadinessScript -notmatch 'ArtifactsDir' -or $releaseReadinessScript -notmatch 'linux-amd64' -or $releaseReadinessScript -notmatch 'linux-arm64') {
    throw "release readiness script must validate downloaded release artifacts when provided."
}
if ($readme -notmatch '轻量使用者客户端 \+ 自托管公共发现服务' -or $readme -notmatch '严禁在 VPS 上编译 Rust') {
    throw "README must describe the split architecture and no-VPS-build deployment boundary."
}
if ($readme -notmatch '普通客户端不会要求你填写 Steam Key' -or $readme -notmatch '个人状态不会写入公共发现服务') {
    throw "README must describe ordinary client credential and personal-state boundaries."
}
if ($readme -notmatch '备份与恢复' -or $readme -notmatch 'OpenAPI 和类型生成') {
    throw "README must include backup and OpenAPI generation sections."
}
if ($releaseWorkflow -notmatch 'Release Split Architecture') {
    throw "release workflow must describe the split-architecture release."
}
if ($releaseWorkflow -notmatch 'windows-latest' -or $releaseWorkflow -notmatch 'tauri-apps/tauri-action') {
    throw "release workflow must publish the Windows Tauri user client."
}
if ($releaseWorkflow -match 'macos-latest' -or $releaseWorkflow -match 'aarch64-apple-darwin' -or $releaseWorkflow -match 'x86_64-apple-darwin') {
    throw "release workflow must not publish macOS desktop client packages in the first split-architecture release."
}
if ($releaseWorkflow -notmatch 'docker/setup-qemu-action' -or $releaseWorkflow -notmatch 'docker/setup-buildx-action') {
    throw "release workflow must set up QEMU and Buildx for multi-architecture server image builds."
}
if ($releaseWorkflow -notmatch 'linux/amd64' -or $releaseWorkflow -notmatch 'linux/arm64') {
    throw "release workflow must publish both amd64 and arm64 mpgs-server image archives."
}
if ($releaseWorkflow -notmatch 'docker buildx build' -or $releaseWorkflow -notmatch 'Dockerfile\.mpgs-server' -or $releaseWorkflow -notmatch 'docker save') {
    throw "release workflow must build and save the mpgs-server Docker image."
}
if ($releaseWorkflow -notmatch 'deploy' -or $releaseWorkflow -notmatch 'compose\.caddy\.yml' -or $releaseWorkflow -notmatch 'mpgs-server-deploy-') {
    throw "release workflow must publish Docker Compose, Caddy, and example config deployment assets."
}
if ($releaseWorkflow -notmatch 'test-mpgs-public-catalog-live\.ps1' -or $releaseWorkflow -notmatch 'test-mpgs-production-readiness\.ps1' -or $releaseWorkflow -notmatch 'test-mpgs-release-readiness\.ps1') {
    throw "release workflow must package validation scripts in deployment assets."
}
if ($releaseWorkflow -notmatch '--export-openapi' -or $releaseWorkflow -notmatch 'docs/openapi/mpgs-server\.openapi\.json') {
    throw "release workflow must verify and publish the generated OpenAPI JSON."
}
if ($releaseWorkflow -notmatch 'softprops/action-gh-release') {
    throw "release workflow must upload server artifacts to the tag release."
}

Write-Output "Deployment contract checks passed."
