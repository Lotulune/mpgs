param(
    [Parameter(Mandatory = $true)]
    [string] $BaseUrl,

    [int] $MinGames = 1,
    [int] $ProbeLimit = 10,
    [switch] $AllowHttp,
    [switch] $AllowSampleCatalog
)

$ErrorActionPreference = "Stop"

function Normalize-BaseUrl {
    param([string] $Url)

    return $Url.Trim().TrimEnd("/")
}

function Invoke-MpgsJson {
    param([string] $Uri)

    try {
        return Invoke-RestMethod -Method Get -Uri $Uri -ErrorAction Stop
    } catch {
        throw "GET $Uri failed: $($_.Exception.Message)"
    }
}

function Assert-NonEmptyString {
    param(
        [object] $Value,
        [string] $Name
    )

    if (-not ($Value -is [string]) -or [string]::IsNullOrWhiteSpace($Value)) {
        throw "$Name must be a non-empty string."
    }
}

function Test-SampleOnlyCatalog {
    param([object[]] $Items)

    if ($Items.Count -eq 0) {
        return $false
    }

    $sampleAppids = @(920001, 920002, 920003, 920004)
    foreach ($item in $Items) {
        if ($sampleAppids -notcontains [int] $item.appid) {
            return $false
        }
    }
    return $true
}

$base = Normalize-BaseUrl $BaseUrl
$parsed = [Uri] $base
if ($parsed.Scheme -ne "https" -and -not $AllowHttp) {
    throw "Real public catalog validation requires HTTPS. Pass -AllowHttp only for localhost/LAN development validation."
}
if ($MinGames -lt 1) {
    throw "MinGames must be at least 1 for real catalog validation."
}
if ($ProbeLimit -lt 1 -or $ProbeLimit -gt 100) {
    throw "ProbeLimit must be between 1 and 100."
}

$serviceInfo = Invoke-MpgsJson "$base/api/v1/service-info"
Assert-NonEmptyString $serviceInfo.serviceInstanceId "serviceInfo.serviceInstanceId"
Assert-NonEmptyString $serviceInfo.serviceName "serviceInfo.serviceName"
if ($serviceInfo.apiVersion -ne "v1") {
    throw "Expected API version v1, got '$($serviceInfo.apiVersion)'."
}
if (@($serviceInfo.capabilities) -notcontains "public_catalog_read") {
    throw "service-info must advertise public_catalog_read."
}

$discoveryHome = Invoke-MpgsJson "$base/api/v1/discovery-home"
if ($discoveryHome.status -ne "ready" -and $discoveryHome.status -ne "updating") {
    throw "Public catalog must be ready or updating for real catalog validation, got '$($discoveryHome.status)'."
}
if ([int64] $discoveryHome.totalGames -lt $MinGames) {
    throw "Expected at least $MinGames public games in discovery-home, got $($discoveryHome.totalGames)."
}

$gamesPage = Invoke-MpgsJson "$base/api/v1/games?limit=$ProbeLimit&offset=0"
$items = @($gamesPage.items)
if ([int64] $gamesPage.page.total -lt $MinGames) {
    throw "Expected at least $MinGames public games in /games page metadata, got $($gamesPage.page.total)."
}
if ($items.Count -eq 0) {
    throw "/api/v1/games returned no items."
}
if (-not $AllowSampleCatalog -and [int64] $gamesPage.page.total -le 4 -and (Test-SampleOnlyCatalog $items)) {
    throw "Catalog appears to contain only deterministic sample appids. Use real admin setup/discovery data, or pass -AllowSampleCatalog for local sample checks."
}

$first = $items[0]
Assert-NonEmptyString $first.name "games[0].name"
Assert-NonEmptyString $first.capsuleUrl "games[0].capsuleUrl"
if ([string]::IsNullOrWhiteSpace($first.shortDescription)) {
    throw "games[0].shortDescription must be present for rich public catalog validation."
}
if (@($first.tags).Count -eq 0) {
    throw "games[0].tags must include at least one tag."
}
if (@($first.multiplayerModes).Count -eq 0) {
    throw "games[0].multiplayerModes must include at least one mode."
}

$appid = [int] $first.appid
$detail = Invoke-MpgsJson "$base/api/v1/games/$appid"
if ([int] $detail.game.appid -ne $appid) {
    throw "Game detail appid mismatch. Expected $appid, got $($detail.game.appid)."
}
Assert-NonEmptyString $detail.game.shortDescription "detail.game.shortDescription"
Assert-NonEmptyString $detail.game.capsuleUrl "detail.game.capsuleUrl"
if (@($detail.game.storeScreenshotUrls).Count -eq 0) {
    throw "detail.game.storeScreenshotUrls must include at least one screenshot or validated image URL."
}

$analysis = Invoke-MpgsJson "$base/api/v1/games/$appid/analysis"
if ([int] $analysis.appid -ne $appid) {
    throw "Analysis appid mismatch. Expected $appid, got $($analysis.appid)."
}
Assert-NonEmptyString $analysis.generatedAt "analysis.generatedAt"
Assert-NonEmptyString $analysis.report.overview "analysis.report.overview"
Assert-NonEmptyString $analysis.report.source "analysis.report.source"

Write-Output "Live public catalog validation passed for $base with $($gamesPage.page.total) public games."
