$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")

$compose = Get-Content -Raw -Path (Join-Path $root "deploy\compose.yml")
$caddyCompose = Get-Content -Raw -Path (Join-Path $root "deploy\compose.caddy.yml")
$caddyfile = Get-Content -Raw -Path (Join-Path $root "deploy\Caddyfile")
$dockerfile = Get-Content -Raw -Path (Join-Path $root "Dockerfile.mpgs-server")
$deploymentDoc = Get-Content -Raw -Path (Join-Path $root "docs\deployment\mpgs-server-compose.md")

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
if ($compose -notmatch './config:/var/lib/mpgs/config:ro') {
    throw "compose.yml must mount the server config directory read-only."
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
if ($dockerfile -notmatch 'cargo build --release -p mpgs-server') {
    throw "Dockerfile must build only the mpgs-server package."
}
if ($deploymentDoc -notmatch 'Do not build on the VPS' -or $deploymentDoc -notmatch 'must not compile Rust') {
    throw "deployment docs must forbid VPS builds."
}
if ($deploymentDoc -notmatch 'active/service.toml' -or $deploymentDoc -notmatch 'active/secrets.toml') {
    throw "deployment docs must describe the active TOML config files."
}
if ($deploymentDoc -notmatch 'only locates the config directory') {
    throw "deployment docs must state that .env only locates config, not service secrets."
}

Write-Output "Deployment contract checks passed."
