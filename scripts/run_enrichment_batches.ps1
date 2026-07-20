param(
    [string]$DbPath = "data/m7-preview-real-v1.db",
    [int]$BatchSize = 50,
    [int]$BatchCooldownSeconds = 60,
    [int]$ErrorCooldownSeconds = 300,
    [int]$MaxRounds = 100,
    [int]$WaitForPid = 0,
    [string]$LogPath = "data/enrich-stats-batches.log",
    [string]$ToolPath = "",
    [switch]$SkipReviews,
    [switch]$SkipCcu,
    [switch]$SkipStore,
    [switch]$StoreOnly,
    [int]$InterRequestMs = 0
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$tool = if ([string]::IsNullOrWhiteSpace($ToolPath)) {
    Join-Path $root "target/debug/mpgs-dbtool.exe"
} elseif ([System.IO.Path]::IsPathRooted($ToolPath)) {
    $ToolPath
} else {
    Join-Path $root $ToolPath
}
$database = if ([System.IO.Path]::IsPathRooted($DbPath)) {
    $DbPath
} else {
    Join-Path $root $DbPath
}
$log = if ([System.IO.Path]::IsPathRooted($LogPath)) {
    $LogPath
} else {
    Join-Path $root $LogPath
}

if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) {
    throw "mpgs-dbtool is missing: $tool"
}
if (-not (Test-Path -LiteralPath $database -PathType Leaf)) {
    throw "database is missing: $database"
}
if ($BatchSize -lt 1 -or $BatchSize -gt 5000) {
    throw "BatchSize must be between 1 and 5000"
}
if ($StoreOnly) {
    # Pure store/price mode: skip reviews, popular excerpts, and CCU.
    $env:MPGS_ENRICH_STORE_ONLY = "true"
}
if ($SkipReviews) {
    $env:MPGS_ENRICH_SKIP_REVIEWS = "true"
}
if ($SkipCcu) {
    $env:MPGS_ENRICH_SKIP_CCU = "true"
}
if ($SkipStore) {
    $env:MPGS_ENRICH_SKIP_STORE = "true"
}
if ($InterRequestMs -gt 0) {
    $env:MPGS_ENRICH_INTER_REQUEST_MS = [string]$InterRequestMs
}

if ($WaitForPid -gt 0) {
    $existing = Get-Process -Id $WaitForPid -ErrorAction SilentlyContinue
    if ($existing) {
        "$(Get-Date -Format o) waiting_for_pid=$WaitForPid" |
            Out-File -LiteralPath $log -Append -Encoding utf8
        Wait-Process -Id $WaitForPid
    }
}

for ($round = 1; $round -le $MaxRounds; $round += 1) {
    "$(Get-Date -Format o) round=$round batch_size=$BatchSize start" |
        Out-File -LiteralPath $log -Append -Encoding utf8
    $output = @(
        & $tool enrich-steam-candidates $database $BatchSize 2>&1 |
            ForEach-Object {
                $line = $_.ToString()
                $line | Out-File -LiteralPath $log -Append -Encoding utf8
                $line
            }
    )
    $exitCode = $LASTEXITCODE
    "$(Get-Date -Format o) round=$round exit_code=$exitCode end" |
        Out-File -LiteralPath $log -Append -Encoding utf8

    if ($output -match "steam_candidate_enrichment=already_satisfied") {
        "$(Get-Date -Format o) enrichment_complete=true" |
            Out-File -LiteralPath $log -Append -Encoding utf8
        exit 0
    }

    $cooldown = if ($exitCode -eq 0) {
        $BatchCooldownSeconds
    } else {
        $ErrorCooldownSeconds
    }
    Start-Sleep -Seconds ([Math]::Max($cooldown, 0))
}

"$(Get-Date -Format o) enrichment_complete=false reason=max_rounds" |
    Out-File -LiteralPath $log -Append -Encoding utf8
exit 2
