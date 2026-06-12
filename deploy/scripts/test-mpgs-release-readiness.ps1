param(
    [string] $TagName = "",
    [string] $ArtifactsDir = "",
    [switch] $SkipOpenApiRegeneration
)

$ErrorActionPreference = "Stop"

function Read-JsonFile {
    param([string] $Path)

    return Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
}

function Read-CargoPackageVersion {
    param([string] $Path)

    $content = Get-Content -Raw -LiteralPath $Path
    if ($content -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
        throw "Could not read package version from $Path."
    }
    return $Matches[1]
}

function Assert-Contains {
    param(
        [string] $Content,
        [string] $Pattern,
        [string] $Message
    )

    if ($Content -notmatch $Pattern) {
        throw $Message
    }
}

function Assert-FileMatching {
    param(
        [string] $Directory,
        [string] $Pattern,
        [string] $Message
    )

    $match = Get-ChildItem -LiteralPath $Directory -Recurse -File |
        Where-Object { $_.Name -match $Pattern } |
        Select-Object -First 1
    if (-not $match) {
        throw $Message
    }
}

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")

$packageJson = Read-JsonFile (Join-Path $root "package.json")
$tauriConfig = Read-JsonFile (Join-Path $root "src-tauri\tauri.conf.json")
$clientCargoVersion = Read-CargoPackageVersion (Join-Path $root "src-tauri\Cargo.toml")
$serverCargoVersion = Read-CargoPackageVersion (Join-Path $root "crates\mpgs-server\Cargo.toml")
$coreCargoVersion = Read-CargoPackageVersion (Join-Path $root "crates\mpgs-core\Cargo.toml")

$versions = @(
    $packageJson.version,
    $tauriConfig.version,
    $clientCargoVersion,
    $serverCargoVersion,
    $coreCargoVersion
) | Select-Object -Unique
if ($versions.Count -ne 1) {
    throw "Release versions must match across package.json, Tauri config, src-tauri Cargo.toml, mpgs-server, and mpgs-core. Found: $($versions -join ', ')."
}

$version = [string] $packageJson.version
if ([string]::IsNullOrWhiteSpace($TagName)) {
    $TagName = "v$version"
}
if ($TagName -notmatch '^v\d+\.\d+\.\d+') {
    throw "TagName must look like a semantic version tag such as v0.1.0."
}

$releaseWorkflowPath = Join-Path $root ".github\workflows\release.yml"
$releaseWorkflow = Get-Content -Raw -LiteralPath $releaseWorkflowPath
Assert-Contains $releaseWorkflow 'Release Split Architecture' "release.yml must define the split-architecture release workflow."
Assert-Contains $releaseWorkflow 'tags:\s*\r?\n\s+- "v\*"' "release.yml must trigger on v* tags."
Assert-Contains $releaseWorkflow 'windows-latest' "release.yml must build the Windows client installer."
Assert-Contains $releaseWorkflow 'tauri-apps/tauri-action' "release.yml must use the Tauri action for the Windows installer."
Assert-Contains $releaseWorkflow 'linux/amd64' "release.yml must build a linux/amd64 server image archive."
Assert-Contains $releaseWorkflow 'linux/arm64' "release.yml must build a linux/arm64 server image archive."
Assert-Contains $releaseWorkflow 'docker buildx build' "release.yml must build the server image with Docker Buildx."
Assert-Contains $releaseWorkflow 'docker save' "release.yml must save the server image archive."
Assert-Contains $releaseWorkflow 'releaseDraft:\s*true' "Windows release job must create a draft release."
Assert-Contains $releaseWorkflow 'draft:\s*true' "Server release upload must target the draft release."
Assert-Contains $releaseWorkflow 'prerelease:\s*true' "Release workflow must mark the split-architecture release as a prerelease."
Assert-Contains $releaseWorkflow 'deploy/scripts/test-mpgs-public-catalog-live\.ps1' "Release deployment package must include live public catalog validation."
Assert-Contains $releaseWorkflow 'deploy/scripts/test-mpgs-production-readiness\.ps1' "Release deployment package must include production readiness validation."
Assert-Contains $releaseWorkflow 'deploy/scripts/test-mpgs-release-readiness\.ps1' "Release deployment package must include release readiness validation."

$releaseDoc = Get-Content -Raw -LiteralPath (Join-Path $root "docs\release-signing.md")
Assert-Contains $releaseDoc 'Windows client builds are generated without code signing' "release-signing.md must document unsigned Windows builds."
Assert-Contains $releaseDoc 'draft prerelease' "release-signing.md must document draft prerelease review."
Assert-Contains $releaseDoc 'test-mpgs-release-readiness\.ps1' "release-signing.md must document the release readiness script."

$openApiPath = Join-Path $root "docs\openapi\mpgs-server.openapi.json"
$openApi = Get-Content -Raw -LiteralPath $openApiPath
Assert-Contains $openApi '"/api/v1/service-info"' "OpenAPI must include service-info."
Assert-Contains $openApi '"/api/v1/games/{appid}/analysis"' "OpenAPI must include public game analysis."
Assert-Contains $openApi '"/api/v1/admin/connection-share"' "OpenAPI must include admin connection-share."

if (-not $SkipOpenApiRegeneration) {
    $openApiOutput = & cargo run -p mpgs-server -- --export-openapi
    if ($LASTEXITCODE -ne 0) {
        throw "OpenAPI regeneration failed."
    }
    $currentOpenApi = (Get-Content -Raw -LiteralPath $openApiPath).Replace("`r`n", "`n").Trim()
    $regeneratedOpenApi = (($openApiOutput -join "`n").Replace("`r`n", "`n")).Trim()
    if ($currentOpenApi -ne $regeneratedOpenApi) {
        throw "docs/openapi/mpgs-server.openapi.json is not in sync with cargo run -p mpgs-server -- --export-openapi."
    }
}

if (-not [string]::IsNullOrWhiteSpace($ArtifactsDir)) {
    $resolvedArtifactsDir = Resolve-Path -LiteralPath $ArtifactsDir
    Assert-FileMatching $resolvedArtifactsDir '\.(msi|exe)$' "Artifacts directory must contain a Windows installer."
    Assert-FileMatching $resolvedArtifactsDir "mpgs-server-$([regex]::Escape($TagName))-linux-amd64\.tar$" "Artifacts directory must contain the linux-amd64 server image tar."
    Assert-FileMatching $resolvedArtifactsDir "mpgs-server-$([regex]::Escape($TagName))-linux-arm64\.tar$" "Artifacts directory must contain the linux-arm64 server image tar."
    Assert-FileMatching $resolvedArtifactsDir "mpgs-server-deploy-$([regex]::Escape($TagName))\.tar\.gz$" "Artifacts directory must contain the deployment assets tarball."
    Assert-FileMatching $resolvedArtifactsDir 'mpgs-server\.openapi\.json$' "Artifacts directory must contain the OpenAPI JSON."
}

Write-Output "Release readiness validation passed for $TagName."
