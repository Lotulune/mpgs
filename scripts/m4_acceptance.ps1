#Requires -Version 5.1
<#
.SYNOPSIS
  M4 API-level acceptance smoke for PRD core flows and offline cache signals.

.DESCRIPTION
  Starts a temporary mpgs-server with demo seed (or uses an existing base URL), then:
  - health + meta
  - anonymous session + preferences (first-run / 7.1)
  - four recommendation feeds with reasons
  - feedback loop
  - calendar / new-games path (7.3)
  - search + game detail
  - ETag revalidation (304) for offline/cache readiness

  This script gates API semantics only. Installed GUI, layout, real disconnect,
  and native bundle smoke remain separate M4 evidence.

.PARAMETER BaseUrl
  Existing API base. When omitted, a local server is started on a free port.

.PARAMETER KeepServer
  Leave the temporary server running after the script ends.

.PARAMETER AllowExistingServerWrites
  Required with BaseUrl. The acceptance flow creates a session and writes
  preferences and feedback, so an existing server is never mutated implicitly.

.EXAMPLE
  .\scripts\m4_acceptance.ps1
  .\scripts\m4_acceptance.ps1 -BaseUrl http://127.0.0.1:8080 -AllowExistingServerWrites
#>
param(
    [string]$BaseUrl = '',
    [switch]$KeepServer,
    [switch]$AllowExistingServerWrites
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName System.Net.Http

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$results = [System.Collections.Generic.List[object]]::new()
$serverProc = $null
$startedServer = $false
$deviceId = [guid]::NewGuid().ToString('N')
$dbDir = $null
$fatalError = $null
$usingExistingServer = -not [string]::IsNullOrWhiteSpace($BaseUrl)
$originalEnvironment = @{
    MPGS_BIND_ADDR     = [Environment]::GetEnvironmentVariable('MPGS_BIND_ADDR', 'Process')
    MPGS_DATABASE_PATH = [Environment]::GetEnvironmentVariable('MPGS_DATABASE_PATH', 'Process')
    MPGS_SEED_DEMO     = [Environment]::GetEnvironmentVariable('MPGS_SEED_DEMO', 'Process')
}
$gitSha = (& git -C $repoRoot rev-parse HEAD 2>$null | Select-Object -First 1)
$gitDirty = [bool](& git -C $repoRoot status --porcelain 2>$null | Select-Object -First 1)
$scriptSha256 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
$serverSha256 = 'not-built'
$serviceVersion = 'unknown'
$apiVersion = 'unknown'
$algorithmVersion = 'unknown'

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

function Test-Property([object]$Value, [string]$Name) {
    return $null -ne $Value -and $Value.PSObject.Properties.Name -contains $Name
}

function Restore-ProcessEnvironment {
    foreach ($key in $originalEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($key, $originalEnvironment[$key], 'Process')
    }
}

function Invoke-Api {
    param(
        [string]$Method,
        [string]$Path,
        [hashtable]$Headers = @{},
        [object]$Body = $null,
        [int[]]$ExpectStatus = @(200)
    )
    # Use HttpClient so PS 5.1 and PS 7 both handle non-2xx without throwing.
    $uri = "$BaseUrl$Path"
    $handler = New-Object System.Net.Http.HttpClientHandler
    $client = New-Object System.Net.Http.HttpClient($handler)
    $client.Timeout = [TimeSpan]::FromSeconds(30)
    try {
        $request = New-Object System.Net.Http.HttpRequestMessage
        $request.Method = [System.Net.Http.HttpMethod]::new($Method.ToUpperInvariant())
        $request.RequestUri = [Uri]$uri
        $request.Headers.TryAddWithoutValidation('x-device-id', $deviceId) | Out-Null
        $request.Headers.TryAddWithoutValidation('Accept', 'application/json') | Out-Null
        foreach ($key in $Headers.Keys) {
            $request.Headers.TryAddWithoutValidation([string]$key, [string]$Headers[$key]) | Out-Null
        }
        if ($null -ne $Body) {
            $jsonBody = $Body | ConvertTo-Json -Depth 8 -Compress
            $request.Content = New-Object System.Net.Http.StringContent(
                $jsonBody,
                [System.Text.Encoding]::UTF8,
                'application/json'
            )
        }
        $resp = $client.SendAsync($request).GetAwaiter().GetResult()
        $status = [int]$resp.StatusCode
        $raw = $resp.Content.ReadAsStringAsync().GetAwaiter().GetResult()
        $headerMap = @{}
        foreach ($h in $resp.Headers) {
            $headerMap[$h.Key] = ($h.Value -join ',')
        }
        if ($resp.Content -and $resp.Content.Headers) {
            foreach ($h in $resp.Content.Headers) {
                $headerMap[$h.Key] = ($h.Value -join ',')
            }
        }
        if ($ExpectStatus -notcontains $status) {
            throw "HTTP $status for $Method $Path (expected $($ExpectStatus -join ',')): $raw"
        }
        $json = $null
        if ($raw -and $raw.Trim().Length -gt 0 -and $status -ne 304) {
            $json = $raw | ConvertFrom-Json
        }
        return [pscustomobject]@{
            Status  = $status
            Headers = $headerMap
            Json    = $json
            Raw     = $raw
        }
    } finally {
        $client.Dispose()
    }
}

function Find-FreePort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
    $listener.Stop()
    return $port
}

function Wait-Ready([string]$Url, [int]$Seconds = 60) {
    $deadline = (Get-Date).AddSeconds($Seconds)
    $lastError = ''
    while ((Get-Date) -lt $deadline) {
        try {
            # -UseBasicParsing works on Windows PowerShell 5.1 without IE.
            $r = Invoke-WebRequest -Uri "$Url/health/ready" -UseBasicParsing -TimeoutSec 2
            if ([int]$r.StatusCode -eq 200) { return }
            $lastError = "status=$($r.StatusCode)"
        } catch {
            $lastError = "$_"
        }
        Start-Sleep -Milliseconds 400
    }
    throw "server not ready at $Url within ${Seconds}s ($lastError)"
}

try {
    Push-Location $repoRoot

    if ($usingExistingServer -and -not $AllowExistingServerWrites) {
        throw 'BaseUrl points to an existing server. Re-run with -AllowExistingServerWrites after confirming that test preferences and feedback may be written.'
    }

    if (-not $usingExistingServer) {
        Write-Step 'Build + start temporary mpgs-server (demo seed, free port)'
        & cargo build -p mpgs-server --locked --quiet
        if ($LASTEXITCODE -ne 0) { throw 'cargo build -p mpgs-server failed' }
        $serverExe = Join-Path $repoRoot 'target\debug\mpgs-server.exe'
        if (-not (Test-Path $serverExe)) {
            throw "missing server binary: $serverExe"
        }
        $serverSha256 = (Get-FileHash -LiteralPath $serverExe -Algorithm SHA256).Hash.ToLowerInvariant()
        $port = Find-FreePort
        $BaseUrl = "http://127.0.0.1:$port"
        $dbDir = Join-Path $env:TEMP ("mpgs-m4-accept-" + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $dbDir | Out-Null
        $dbPath = Join-Path $dbDir 'accept.db'
        $logOut = Join-Path $dbDir 'server.out.log'
        $logErr = Join-Path $dbDir 'server.err.log'
        $env:MPGS_BIND_ADDR = "127.0.0.1:$port"
        $env:MPGS_DATABASE_PATH = $dbPath
        $env:MPGS_SEED_DEMO = 'true'
        $serverProc = Start-Process -FilePath $serverExe `
            -WorkingDirectory $repoRoot `
            -PassThru `
            -NoNewWindow `
            -RedirectStandardOutput $logOut `
            -RedirectStandardError $logErr
        $startedServer = $true
        try {
            Wait-Ready $BaseUrl 90
        } catch {
            $errTail = if (Test-Path $logErr) { Get-Content $logErr -Raw } else { '' }
            $outTail = if (Test-Path $logOut) { Get-Content $logOut -Raw } else { '' }
            throw "server failed to become ready: $_`n--- stderr ---`n$errTail`n--- stdout ---`n$outTail"
        }
        Add-Result 'server.start' $true "temporary local server started pid=$($serverProc.Id)"
    } else {
        $BaseUrl = $BaseUrl.TrimEnd('/')
        $serverSha256 = 'external-server-not-hashed'
        Write-Step "Using existing server $BaseUrl"
        Write-Warning 'This run is authorized to write a session, preferences, feedback, and feedback undo to the existing server.'
        Wait-Ready $BaseUrl 15
        Add-Result 'server.ready' $true $BaseUrl
    }

    Write-Step 'Health + meta'
    $live = Invoke-Api -Method GET -Path '/health/live'
    Add-Result 'health.live' ($live.Status -eq 200) "status=$($live.Status)"
    $ready = Invoke-Api -Method GET -Path '/health/ready'
    Add-Result 'health.ready' ($ready.Status -eq 200) "status=$($ready.Status)"
    $meta = Invoke-Api -Method GET -Path '/v1/meta'
    $apiVersion = [string]$meta.Json.api_version
    $serviceVersion = [string]$meta.Json.service_version
    $algorithmVersion = [string]$meta.Json.algorithm_version
    $sections = @($meta.Json.supported_sections)
    $need = @('recent_release', 'upcoming', 'popular_legacy', 'classic_legacy')
    $missing = @($need | Where-Object { $sections -notcontains $_ })
    $hasAll = $missing.Count -eq 0
    Add-Result 'meta.sections' $hasAll ("sections=" + ($sections -join ','))
    Add-Result 'meta.versions' (
        $apiVersion -eq 'v1' -and $serviceVersion -ne '' -and $algorithmVersion -ne ''
    ) "api=$apiVersion service=$serviceVersion algorithm=$algorithmVersion"
    Add-Result 'meta.ai_provider_state' (Test-Property $meta.Json 'ai_available') "ai_available=$($meta.Json.ai_available)"

    Write-Step 'PRD 7.1 first-run: session + preferences + four feeds + feedback'
    $session = Invoke-Api -Method POST -Path '/v1/session/anonymous' -ExpectStatus @(200, 201)
    $token = [string]$session.Json.access_token
    if (-not $token) { throw 'missing access_token' }
    Add-Result 'session.anonymous' ($token.Length -gt 10) 'access_token issued'

    $auth = @{ Authorization = "Bearer $token" }
    $prefsGet = Invoke-Api -Method GET -Path '/v1/preferences' -Headers $auth
    $prefsBody = [ordered]@{
        version                   = [int]$prefsGet.Json.version
        party_size                = 4
        coop_competitive          = 0.15
        session_minutes_min       = 30
        session_minutes_max       = 180
        budget_currency           = 'CNY'
        budget_max_each_minor     = 15000
        platforms                 = @('windows')
        self_hosting_willingness  = 0.7
        languages                 = @('schinese', 'english')
        excluded_modes            = @('mmo')
    }
    $prefsPut = Invoke-Api -Method PUT -Path '/v1/preferences' -Headers $auth -Body $prefsBody
    Add-Result 'prefs.put' (
        [int]$prefsPut.Json.party_size -eq 4 -and
        [double]$prefsPut.Json.coop_competitive -le 0.2
    ) "version=$($prefsPut.Json.version) party=$($prefsPut.Json.party_size)"

    $feedItems = 0
    $itemsWithoutReasons = [System.Collections.Generic.List[string]]::new()
    $etag = $null
    $firstApp = $null
    $firstFeedPath = $null
    $firstAppBaselineScore = $null
    foreach ($section in $need) {
        $feed = Invoke-Api -Method GET -Path "/v1/feeds/${section}?limit=10" -Headers $auth
        $items = @($feed.Json.items)
        $feedItems += $items.Count
        foreach ($item in $items) {
            $usableReasons = @($item.reasons | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
            if ($usableReasons.Count -eq 0) {
                $itemsWithoutReasons.Add("$section/$($item.app_id)")
            }
            if (-not $firstApp) {
                $firstApp = $item
                $firstFeedPath = "/v1/feeds/${section}?limit=10"
                $firstAppBaselineScore = [double]$item.score
            }
        }
        if (-not $etag -and $feed.Headers['ETag']) {
            $etag = [string]$feed.Headers['ETag']
            $etagPath = "/v1/feeds/${section}?limit=10"
        }
        Add-Result "feed.$section" ($items.Count -ge 1) "items=$($items.Count)"
    }
    Add-Result 'feed.reasons' (
        $feedItems -gt 0 -and $itemsWithoutReasons.Count -eq 0
    ) "total_items=$feedItems items_without_reasons=$($itemsWithoutReasons.Count) $($itemsWithoutReasons -join ',')"

    Write-Step 'PRD 7.2 natural-language recommendation with explicit deterministic fallback'
    $natural = Invoke-Api -Method POST -Path '/v1/recommendations/natural-language' -Headers $auth -Body @{
        query = '3 people, one hour, casual and replayable'
        limit = 6
    }
    $naturalItems = @($natural.Json.items)
    $naturalItemsWithoutReasons = @($naturalItems | Where-Object {
        @($_.reasons | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }).Count -eq 0
    })
    $naturalParsed = (
        [int]$natural.Json.interpreted.party_size -eq 3 -and
        [int]$natural.Json.interpreted.session_minutes_max -eq 60 -and
        [double]$natural.Json.interpreted.coop_competitive -le 0.2
    )
    Add-Result 'natural_language.constraints' $naturalParsed (
        "party=$($natural.Json.interpreted.party_size) session_max=$($natural.Json.interpreted.session_minutes_max) coop_competitive=$($natural.Json.interpreted.coop_competitive)"
    )
    Add-Result 'natural_language.candidates' (
        $naturalItems.Count -ge 3 -and
        $naturalItems.Count -le 10 -and
        $naturalItemsWithoutReasons.Count -eq 0
    ) "items=$($naturalItems.Count) items_without_reasons=$($naturalItemsWithoutReasons.Count)"
    Add-Result 'natural_language.fallback' (
        [string]$natural.Json.ai_status -eq 'fallback' -and
        -not [string]::IsNullOrWhiteSpace([string]$natural.Json.fallback_reason)
    ) "ai_status=$($natural.Json.ai_status) fallback_reason=$($natural.Json.fallback_reason)"

    if (-not $firstApp) { throw 'no feed items for feedback' }
    $idem = [guid]::NewGuid().ToString()
    $fb = Invoke-Api -Method POST -Path '/v1/feedback' -Headers ($auth + @{
            'Idempotency-Key' = $idem
        }) -Body @{
        app_id               = [int]$firstApp.app_id
        type                 = 'like'
        client_created_at_ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    } -ExpectStatus @(200, 201)
    $feedbackId = if (Test-Property $fb.Json 'feedback_id') { [long]$fb.Json.feedback_id } else { 0 }
    Add-Result 'feedback.like' (
        $feedbackId -gt 0 -and [int]$fb.Json.app_id -eq [int]$firstApp.app_id
    ) "app_id=$($firstApp.app_id) feedback_id=$feedbackId status=$($fb.Status)"

    if ($feedbackId -gt 0) {
        $afterLikeFeed = Invoke-Api -Method GET -Path $firstFeedPath -Headers $auth
        $afterLikeItems = @($afterLikeFeed.Json.items)
        $afterLikeTarget = @($afterLikeItems | Where-Object { [int]$_.app_id -eq [int]$firstApp.app_id } | Select-Object -First 1)
        $afterLikeScore = if ($afterLikeTarget.Count -eq 1) { [double]$afterLikeTarget[0].score } else { $null }
        $feedbackApplied = (
            $afterLikeTarget.Count -eq 0 -or
            [Math]::Abs([double]$afterLikeScore - [double]$firstAppBaselineScore) -gt 0.000001
        )
        Add-Result 'feedback.feed_effect' $feedbackApplied (
            "app_id=$($firstApp.app_id) baseline_score=$firstAppBaselineScore active_score=$afterLikeScore present=$($afterLikeTarget.Count -eq 1)"
        )

        $undo = Invoke-Api -Method POST -Path "/v1/feedback/$($fb.Json.feedback_id)/undo" -Headers $auth -ExpectStatus @(200, 201)
        $undoId = if (Test-Property $undo.Json 'feedback_id') { [long]$undo.Json.feedback_id } else { 0 }
        Add-Result 'feedback.undo' (
            $undoId -gt 0 -and [int]$undo.Json.app_id -eq [int]$firstApp.app_id
        ) "feedback_id=$undoId original_feedback_id=$feedbackId status=$($undo.Status)"

        $afterUndoFeed = Invoke-Api -Method GET -Path $firstFeedPath -Headers $auth
        $afterUndoTarget = @($afterUndoFeed.Json.items | Where-Object { [int]$_.app_id -eq [int]$firstApp.app_id } | Select-Object -First 1)
        $afterUndoScore = if ($afterUndoTarget.Count -eq 1) { [double]$afterUndoTarget[0].score } else { $null }
        Add-Result 'feedback.feed_restored' (
            $afterUndoTarget.Count -eq 1 -and
            [Math]::Abs([double]$afterUndoScore - [double]$firstAppBaselineScore) -le 0.000001
        ) "app_id=$($firstApp.app_id) baseline_score=$firstAppBaselineScore restored_score=$afterUndoScore present=$($afterUndoTarget.Count -eq 1)"
    } else {
        Add-Result 'feedback.undo' $false 'cannot undo: feedback response did not contain a positive feedback_id'
        Add-Result 'feedback.feed_effect' $false 'cannot verify feedback effect without feedback_id'
        Add-Result 'feedback.feed_restored' $false 'cannot verify restoration without feedback_id'
    }

    Write-Step 'PRD 7.3 new-games: calendar + early-data honesty'
    $recentCalendar = Invoke-Api -Method GET -Path '/v1/calendar?state=recent'
    $upcomingCalendar = Invoke-Api -Method GET -Path '/v1/calendar?state=upcoming'
    $dated = @($recentCalendar.Json.dated_items) + @($upcomingCalendar.Json.dated_items)
    $undated = @($recentCalendar.Json.undated_items) + @($upcomingCalendar.Json.undated_items)
    $recentItems = @($recentCalendar.Json.dated_items) + @($recentCalendar.Json.undated_items)
    $upcomingItems = @($upcomingCalendar.Json.dated_items) + @($upcomingCalendar.Json.undated_items)
    $calCount = $dated.Count + $undated.Count
    $calendarItems = @($dated) + @($undated)
    $invalidCalendarItems = @($calendarItems | Where-Object {
        -not (Test-Property $_ 'current_data_confidence') -or
        -not (Test-Property $_ 'source_modified_at_ms') -or
        -not (Test-Property $_ 'updated_at_ms') -or
        -not (Test-Property $_ 'release_date_precision') -or
        -not (Test-Property $_ 'review_total') -or
        -not (Test-Property $_ 'early_data') -or
        -not ($_.early_data -is [bool]) -or
        [string]::IsNullOrWhiteSpace([string]$_.canonical_name) -or
        [long]$_.updated_at_ms -le 0
    })
    $invalidDatedItems = @($dated | Where-Object {
        [string]::IsNullOrWhiteSpace([string]$_.release_date) -or
        [string]::IsNullOrWhiteSpace([string]$_.release_date_precision)
    })
    $recentStateMismatch = @($recentItems | Where-Object { [string]$_.release_state -ne 'released' })
    $upcomingStateMismatch = @($upcomingItems | Where-Object {
        [string]$_.release_state -notin @('upcoming', 'coming_soon')
    })
    $calOk = (
        $recentCalendar.Status -eq 200 -and
        $upcomingCalendar.Status -eq 200 -and
        (Test-Property $recentCalendar.Json 'dated_items') -and
        (Test-Property $recentCalendar.Json 'undated_items') -and
        (Test-Property $upcomingCalendar.Json 'dated_items') -and
        (Test-Property $upcomingCalendar.Json 'undated_items') -and
        [long]$recentCalendar.Json.data_updated_at_ms -gt 0 -and
        [long]$upcomingCalendar.Json.data_updated_at_ms -gt 0 -and
        $calCount -gt 0 -and
        $invalidCalendarItems.Count -eq 0 -and
        $invalidDatedItems.Count -eq 0
    )
    Add-Result 'calendar.get' $calOk "recent=$($recentItems.Count) upcoming=$($upcomingItems.Count) dated=$($dated.Count) undated=$($undated.Count)"
    Add-Result 'calendar.state_filters' (
        $recentItems.Count -gt 0 -and
        $upcomingItems.Count -gt 0 -and
        $recentStateMismatch.Count -eq 0 -and
        $upcomingStateMismatch.Count -eq 0
    ) "recent_mismatch=$($recentStateMismatch.Count) upcoming_mismatch=$($upcomingStateMismatch.Count)"
    Add-Result 'calendar.early_data_honesty' (
        $calendarItems.Count -gt 0 -and $invalidCalendarItems.Count -eq 0 -and $invalidDatedItems.Count -eq 0
    ) "invalid_items=$($invalidCalendarItems.Count) invalid_dated_items=$($invalidDatedItems.Count) review_total_and_early_data=present"

    $search = Invoke-Api -Method GET -Path '/v1/search?q=Rock&limit=5'
    $searchItems = @($search.Json.items)
    if ($searchItems.Count -eq 0) { $searchItems = @($search.Json.results) }
    Add-Result 'search.name' (
        $search.Status -eq 200 -and
        $searchItems.Count -gt 0 -and
        @($searchItems | Where-Object { [string]$_.name -match 'Rock' }).Count -gt 0
    ) "status=$($search.Status) hits=$($searchItems.Count)"

    $detail = Invoke-Api -Method GET -Path "/v1/games/$($firstApp.app_id)" -Headers $auth
    Add-Result 'games.detail' (
        $detail.Status -eq 200 -and
        [int]$detail.Json.app_id -eq [int]$firstApp.app_id
    ) "app_id=$($detail.Json.app_id) name=$($detail.Json.name)"

    Write-Step 'Offline/cache readiness: ETag short-circuit'
    if ($etag -and $etagPath) {
        $reval = Invoke-Api -Method GET -Path $etagPath -Headers ($auth + @{
                'If-None-Match' = $etag
            }) -ExpectStatus @(304)
        Add-Result 'etag.revalidate' ($reval.Status -eq 304) "status=$($reval.Status) etag=$etag"
    } else {
        Add-Result 'etag.revalidate' $false 'no ETag on first feed response'
    }

    Write-Step 'Client offline/cache contract suite (vitest)'
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $offlineTests = & pnpm --filter lobbytally-web exec vitest run `
        tests/apiClient.test.ts `
        tests/feedbackQueue.test.ts `
        tests/playIntentStore.test.ts `
        tests/preferences.test.ts 2>&1
    $offlineCode = $LASTEXITCODE
    $ErrorActionPreference = $prevEap
    $offlineText = ($offlineTests | Out-String)
    $offlineOk = $offlineCode -eq 0
    Add-Result 'web.offline_contract' $offlineOk ($(if ($offlineOk) { 'ETag, offline snapshot, and durable pending-write tests passed' } else { $offlineText.Substring(0, [Math]::Min(400, $offlineText.Length)) }))

    Write-Step 'Complete client unit suite (vitest)'
    $ErrorActionPreference = 'Continue'
    $vitest = & pnpm --filter lobbytally-web test 2>&1
    $vitestCode = $LASTEXITCODE
    $ErrorActionPreference = $prevEap
    $vitestText = ($vitest | Out-String)
    $vitestOk = $vitestCode -eq 0
    Add-Result 'web.vitest' $vitestOk ($(if ($vitestOk) { 'pnpm --filter lobbytally-web test exit 0' } else { $vitestText.Substring(0, [Math]::Min(400, $vitestText.Length)) }))

    Write-Step 'Web production build'
    $ErrorActionPreference = 'Continue'
    $build = & pnpm --filter lobbytally-web build 2>&1
    $buildCode = $LASTEXITCODE
    $ErrorActionPreference = $prevEap
    $buildText = ($build | Out-String)
    $buildOk = $buildCode -eq 0
    Add-Result 'web.build' $buildOk ($(if ($buildOk) { 'typecheck+vite build ok' } else { $buildText.Substring(0, [Math]::Min(400, $buildText.Length)) }))

    Write-Step 'Desktop crate check (Tauri shell, no full installer)'
    Push-Location (Join-Path $repoRoot 'apps\desktop\src-tauri')
    try {
        # cargo writes progress to stderr; do not let native stderr become terminating errors.
        $prevEap = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $chk = & cargo check --locked 2>&1
        $chkCode = $LASTEXITCODE
        $ErrorActionPreference = $prevEap
        $chkText = ($chk | Out-String)
        $chkOk = $chkCode -eq 0
        Add-Result 'desktop.cargo_check' $chkOk ($(if ($chkOk) { 'apps/desktop/src-tauri cargo check ok' } else { $chkText.Substring(0, [Math]::Min(400, $chkText.Length)) }))
    } finally {
        Pop-Location
    }
}
catch {
    $fatalError = $_
    Add-Result 'acceptance.fatal' $false ($_.Exception.Message -replace "[\r\n]+", ' ')
}
finally {
    if ($startedServer -and $serverProc -and -not $KeepServer) {
        Write-Step 'Stop temporary server'
        try {
            $childProcesses = @(Get-CimInstance Win32_Process -Filter "ParentProcessId=$($serverProc.Id)" -ErrorAction SilentlyContinue)
            Stop-Process -Id $serverProc.Id -Force -ErrorAction SilentlyContinue
            $childProcesses | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
            $serverProc.WaitForExit(5000) | Out-Null
        } catch {
            # best-effort
        }
    }
    Restore-ProcessEnvironment
    if ($startedServer -and -not $KeepServer -and $dbDir -and (Test-Path -LiteralPath $dbDir)) {
        Remove-Item -LiteralPath $dbDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    Pop-Location -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host '======== M4 acceptance summary ========' -ForegroundColor Yellow
$pass = @($results | Where-Object { $_.ok }).Count
$fail = @($results | Where-Object { -not $_.ok }).Count
$results | Format-Table -AutoSize id, ok, detail
Write-Host "passed=$pass failed=$fail base=$BaseUrl" -ForegroundColor Yellow

$stamp = Get-Date -Format 'yyyy-MM-dd HH:mm:ss K'
$baseEvidence = if ($usingExistingServer) { $BaseUrl } else { 'temporary-loopback-server' }
$lines = @(
    "# M4 acceptance run",
    "",
    "- When: $stamp",
    "- Result: $(if ($fail -eq 0) { 'PASS' } else { 'FAIL' })",
    "- Base: $baseEvidence",
    "- Git commit: $gitSha",
    "- Git worktree dirty: $($gitDirty.ToString().ToLowerInvariant())",
    "- Acceptance script SHA-256: $scriptSha256",
    "- Server binary SHA-256: $serverSha256",
    "- API / service / algorithm: $apiVersion / $serviceVersion / $algorithmVersion",
    "- Passed: $pass / $($results.Count) (failed: $fail)",
    "",
    '| ID | OK | Detail |',
    '| --- | --- | --- |'
)
foreach ($r in $results) {
    $ok = if ($r.ok) { 'yes' } else { 'no' }
    $detail = (($r.detail -replace '\|', '/') -replace "[\r\n]+", ' ').Trim()
    $lines += "| $($r.id) | $ok | $detail |"
}
$outDir = Join-Path $repoRoot 'docs'
$outPath = Join-Path $outDir 'M4_ACCEPTANCE_RUN.md'
$lines -join "`n" | Set-Content -Path $outPath -Encoding UTF8
Write-Host "Wrote $outPath"

if ($fail -gt 0) {
    if ($fatalError) {
        Write-Error -ErrorAction Continue "M4 acceptance aborted: $($fatalError.Exception.Message)"
    }
    exit 1
}
exit 0
