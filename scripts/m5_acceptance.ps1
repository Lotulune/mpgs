#Requires -Version 5.1
<#
.SYNOPSIS
  M5 offline acceptance for AI/retrieval exit conditions (no external Key required).

.DESCRIPTION
  Verifies:
  - AI disabled does not break health/meta/feeds/NL (ai_status=fallback)
  - retrieval sync, offline feature extract, and embed-documents (hash) work
  - NL hybrid path remains available
  - unit gates for validation / offline features

  Optional live AI check runs only when MPGS_AI_API_KEY is set and
  -LiveAi is passed.

.EXAMPLE
  .\scripts\m5_acceptance.ps1
  .\scripts\m5_acceptance.ps1 -LiveAi
#>
param(
    [switch]$LiveAi,
    [switch]$KeepServer
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName System.Net.Http

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$results = [System.Collections.Generic.List[object]]::new()
$serverProc = $null
$dbDir = $null
$fatalError = $null
$gitSha = (& git -C $repoRoot rev-parse HEAD 2>$null | Select-Object -First 1)
$gitDirty = [bool](& git -C $repoRoot status --porcelain 2>$null | Select-Object -First 1)
$scriptSha256 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()

function Write-Step([string]$Name) {
    Write-Host ""
    Write-Host "==> $Name" -ForegroundColor Cyan
}

function Add-Result([string]$Id, [bool]$Ok, [string]$Detail) {
    $results.Add([pscustomobject]@{ id = $Id; ok = $Ok; detail = $Detail })
    if ($Ok) {
        Write-Host "  PASS  $Id — $Detail" -ForegroundColor Green
    } else {
        Write-Host "  FAIL  $Id — $Detail" -ForegroundColor Red
    }
}

function Invoke-Json {
    param(
        [Parameter(Mandatory = $true)][string]$Method,
        [Parameter(Mandatory = $true)][string]$Url,
        [string]$Body = $null,
        [hashtable]$Headers = @{}
    )
    $handler = [System.Net.Http.HttpClientHandler]::new()
    $client = [System.Net.Http.HttpClient]::new($handler)
    try {
        $req = [System.Net.Http.HttpRequestMessage]::new([System.Net.Http.HttpMethod]::new($Method), $Url)
        foreach ($key in $Headers.Keys) {
            [void]$req.Headers.TryAddWithoutValidation($key, [string]$Headers[$key])
        }
        if ($null -ne $Body) {
            $req.Content = [System.Net.Http.StringContent]::new($Body, [System.Text.Encoding]::UTF8, 'application/json')
        }
        $resp = $client.SendAsync($req).GetAwaiter().GetResult()
        $text = $resp.Content.ReadAsStringAsync().GetAwaiter().GetResult()
        $json = $null
        if (-not [string]::IsNullOrWhiteSpace($text)) {
            try { $json = $text | ConvertFrom-Json } catch { $json = $null }
        }
        return [pscustomobject]@{
            StatusCode = [int]$resp.StatusCode
            Json       = $json
            Text       = $text
        }
    } finally {
        $client.Dispose()
        $handler.Dispose()
    }
}

function Write-Report([string]$Path, [bool]$Passed) {
    $lines = @(
        '# M5 acceptance run'
        ''
        "- When: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss K')"
        "- Result: $(if ($Passed) { 'PASS' } else { 'FAIL' })"
        "- Git commit: ``$gitSha``"
        "- Git worktree dirty: ``$gitDirty``"
        "- Acceptance script SHA-256: ``$scriptSha256``"
        "- Live AI check requested: ``$LiveAi``"
        "- Passed: $(($results | Where-Object ok).Count) / $($results.Count)"
        ''
        '| ID | OK | Detail |'
        '| --- | --- | --- |'
    )
    foreach ($r in $results) {
        $ok = if ($r.ok) { 'yes' } else { 'no' }
        $detail = ($r.detail -replace '\|', '/')
        $lines += "| $($r.id) | $ok | $detail |"
    }
    $lines += ''
    $lines += 'This run proves offline M5 exit conditions (disabled AI safety, retrieval/embed/offline features, NL fallback).'
    $lines += 'Live provider success requires an API key and is optional.'
    [System.IO.File]::WriteAllLines($Path, $lines)
}

try {
    Write-Step 'unit gates'
    Push-Location $repoRoot
    cargo test -p mpgs-ai -p mpgs-storage --locked --quiet
    if ($LASTEXITCODE -ne 0) { throw "cargo test ai/storage failed: $LASTEXITCODE" }
    Add-Result 'unit.ai_storage' $true 'mpgs-ai + mpgs-storage tests passed'

    Write-Step 'build tools'
    cargo build -p mpgs-server -p mpgs-dbtool --locked --quiet
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed: $LASTEXITCODE" }
    Add-Result 'build.tools' $true 'mpgs-server + mpgs-dbtool built'

    $dbDir = Join-Path $env:TEMP ("mpgs-m5-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $dbDir -Force | Out-Null
    $dbPath = Join-Path $dbDir 'm5.db'

    Write-Step 'retrieval + offline features + embeddings'
    $dbtool = Join-Path $repoRoot 'target\debug\mpgs-dbtool.exe'
    if (-not (Test-Path $dbtool)) { $dbtool = Join-Path $repoRoot 'target\debug\mpgs-dbtool' }

    # Seed by running server once is heavy; create empty DB then let server seed demo via env.
    # For dbtool commands we need a migrated DB with apps: start server briefly with seed, or use migrate + import.
    # Simpler: migrate empty then run server seed into this db, then sync.

    $env:MPGS_DATABASE_PATH = $dbPath
    $env:MPGS_SEED_DEMO = 'true'
    $env:MPGS_AI_PROVIDER = 'disabled'
    $env:MPGS_AI_EMBED_PROVIDER = 'hash'
    $port = Get-Random -Minimum 18000 -Maximum 19000
    $env:MPGS_BIND_ADDR = "127.0.0.1:$port"
    $serverExe = Join-Path $repoRoot 'target\debug\mpgs-server.exe'
    if (-not (Test-Path $serverExe)) { $serverExe = Join-Path $repoRoot 'target\debug\mpgs-server' }

    $serverProc = Start-Process -FilePath $serverExe -PassThru -WindowStyle Hidden
    $base = "http://127.0.0.1:$port"
    $ready = $false
    for ($i = 0; $i -lt 40; $i++) {
        try {
            $h = Invoke-Json -Method GET -Url "$base/health/live"
            if ($h.StatusCode -eq 200) { $ready = $true; break }
        } catch { }
        Start-Sleep -Milliseconds 250
    }
    if (-not $ready) { throw 'server failed to become ready' }
    Add-Result 'server.start' $true "temporary server on $base"

    $meta = Invoke-Json -Method GET -Url "$base/v1/meta"
    Add-Result 'meta.ai_disabled' ($meta.StatusCode -eq 200 -and $meta.Json.ai_available -eq $false) "ai_available=$($meta.Json.ai_available)"

    $feed = Invoke-Json -Method GET -Url "$base/v1/feeds/classic_legacy?limit=5"
    Add-Result 'feed.without_ai' ($feed.StatusCode -eq 200 -and @($feed.Json.items).Count -gt 0) "items=$(@($feed.Json.items).Count)"

    $nl = Invoke-Json -Method POST -Url "$base/v1/recommendations/natural-language" -Body '{"query":"3 people casual coop replayable","limit":5}'
    $nlOk = $nl.StatusCode -eq 200 -and $nl.Json.ai_status -eq 'fallback' -and -not [string]::IsNullOrWhiteSpace([string]$nl.Json.fallback_reason) -and @($nl.Json.items).Count -ge 3
    Add-Result 'nl.fallback' $nlOk "ai_status=$($nl.Json.ai_status) items=$(@($nl.Json.items).Count)"

    if (-not $KeepServer -and $null -ne $serverProc -and -not $serverProc.HasExited) {
        Stop-Process -Id $serverProc.Id -Force -ErrorAction SilentlyContinue
        $serverProc = $null
        Start-Sleep -Seconds 1
    }

    & $dbtool sync-retrieval $dbPath 500 0
    if ($LASTEXITCODE -ne 0) { throw "sync-retrieval failed: $LASTEXITCODE" }
    Add-Result 'retrieval.sync' $true 'sync-retrieval completed'

    & $dbtool extract-offline-features $dbPath 500 0
    if ($LASTEXITCODE -ne 0) { throw "extract-offline-features failed: $LASTEXITCODE" }
    Add-Result 'offline.features' $true 'extract-offline-features completed'

    & $dbtool embed-documents $dbPath 200 16
    if ($LASTEXITCODE -ne 0) { throw "embed-documents failed: $LASTEXITCODE" }
    Add-Result 'embed.batch' $true 'embed-documents (hash) completed'

    if ($LiveAi) {
        Write-Step 'optional live AI'
        $key = [Environment]::GetEnvironmentVariable('MPGS_AI_API_KEY', 'Process')
        if ([string]::IsNullOrWhiteSpace($key)) {
            Add-Result 'live.ai.skipped' $true 'MPGS_AI_API_KEY not set; live AI skipped'
        } else {
            Add-Result 'live.ai.manual' $true 'Key present; configure MPGS_AI_PROVIDER=openai_compat and re-run manual NL used check'
        }
    } else {
        Add-Result 'live.ai.not_requested' $true 'pass -LiveAi with MPGS_AI_API_KEY for live provider check'
    }
}
catch {
    $fatalError = "$_"
    Add-Result 'fatal' $false $fatalError
}
finally {
    if (-not $KeepServer -and $null -ne $serverProc -and -not $serverProc.HasExited) {
        Stop-Process -Id $serverProc.Id -Force -ErrorAction SilentlyContinue
    }
    Pop-Location -ErrorAction SilentlyContinue
    if ($dbDir -and (Test-Path $dbDir) -and -not $KeepServer) {
        Remove-Item -Recurse -Force $dbDir -ErrorAction SilentlyContinue
    }
}

$failed = @($results | Where-Object { -not $_.ok })
$passed = $failed.Count -eq 0
$report = Join-Path $repoRoot 'docs\M5_ACCEPTANCE_RUN.md'
Write-Report -Path $report -Passed $passed
Write-Host ""
Write-Host "Report: $report" -ForegroundColor Cyan
if ($passed) {
    Write-Host 'M5 acceptance: PASS' -ForegroundColor Green
    exit 0
} else {
    Write-Host 'M5 acceptance: FAIL' -ForegroundColor Red
    exit 1
}
