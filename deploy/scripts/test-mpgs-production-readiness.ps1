param(
    [Parameter(Mandatory = $true)]
    [string] $BaseUrl,

    [string] $AdminToken = $env:MPGS_ADMIN_TOKEN,
    [switch] $AllowHttp,
    [switch] $RequirePublicCors,
    [switch] $RequireAdminDiagnostics,
    [switch] $RequireSteamConfigured
)

$ErrorActionPreference = "Stop"

function Normalize-BaseUrl {
    param([string] $Url)

    return $Url.Trim().TrimEnd("/")
}

function Invoke-MpgsRest {
    param(
        [string] $Method,
        [string] $Uri,
        [object] $Body = $null,
        [Microsoft.PowerShell.Commands.WebRequestSession] $WebSession = $null
    )

    $parameters = @{
        Method = $Method
        Uri = $Uri
        ErrorAction = "Stop"
    }
    if ($null -ne $Body) {
        $parameters.ContentType = "application/json"
        $parameters.Body = ($Body | ConvertTo-Json -Depth 8)
    }
    if ($WebSession) {
        $parameters.WebSession = $WebSession
    }

    try {
        return Invoke-RestMethod @parameters
    } catch {
        throw "$Method $Uri failed: $($_.Exception.Message)"
    }
}

function Invoke-MpgsWeb {
    param(
        [string] $Method,
        [string] $Uri,
        [hashtable] $Headers = @{}
    )

    try {
        return Invoke-WebRequest -UseBasicParsing -Method $Method -Uri $Uri -Headers $Headers -ErrorAction Stop
    } catch {
        throw "$Method $Uri failed: $($_.Exception.Message)"
    }
}

function Get-HeaderValue {
    param(
        [object] $Headers,
        [string] $Name
    )

    foreach ($key in $Headers.Keys) {
        if ($key -ieq $Name) {
            return [string] $Headers[$key]
        }
    }
    return $null
}

$base = Normalize-BaseUrl $BaseUrl
$parsed = [Uri] $base
if ($parsed.Scheme -ne "https" -and -not $AllowHttp) {
    throw "Production readiness validation requires HTTPS. Pass -AllowHttp only for localhost/LAN development checks."
}

$health = Invoke-MpgsWeb "GET" "$base/healthz"
if ($health.StatusCode -ne 200) {
    throw "Expected /healthz HTTP 200, got $($health.StatusCode)."
}

$serviceInfo = Invoke-MpgsRest "GET" "$base/api/v1/service-info"
if ($serviceInfo.apiVersion -ne "v1") {
    throw "Expected service API v1, got '$($serviceInfo.apiVersion)'."
}
if (@($serviceInfo.capabilities) -notcontains "public_catalog_read") {
    throw "service-info must advertise public_catalog_read."
}

$corsProbe = Invoke-MpgsWeb "GET" "$base/api/v1/discovery-home" @{
    Origin = "https://mpgs-validation.invalid"
}
$allowOrigin = Get-HeaderValue $corsProbe.Headers "Access-Control-Allow-Origin"
if ($RequirePublicCors -and [string]::IsNullOrWhiteSpace($allowOrigin)) {
    throw "Public read CORS is required but Access-Control-Allow-Origin was not returned."
}

$adminPage = Invoke-MpgsWeb "GET" "$base/admin"
if ($adminPage.StatusCode -ne 200 -or $adminPage.Content -notmatch 'admin-root') {
    throw "/admin must serve the same-origin management UI entry."
}

if ($RequireAdminDiagnostics -and [string]::IsNullOrWhiteSpace($AdminToken)) {
    throw "Admin diagnostics were required, but no AdminToken or MPGS_ADMIN_TOKEN was provided."
}

if (-not [string]::IsNullOrWhiteSpace($AdminToken)) {
    $session = New-Object Microsoft.PowerShell.Commands.WebRequestSession
    Invoke-MpgsRest "POST" "$base/api/v1/admin/session" @{ token = $AdminToken } $session | Out-Null

    $diagnostics = Invoke-MpgsRest "GET" "$base/api/v1/admin/diagnostics" $null $session
    if ($diagnostics.activeConfig -ne "ok") {
        throw "Admin diagnostics activeConfig must be ok, got '$($diagnostics.activeConfig)'."
    }
    if ($diagnostics.publicBaseUrlStatus -ne "configured") {
        throw "Admin diagnostics publicBaseUrlStatus must be configured, got '$($diagnostics.publicBaseUrlStatus)'."
    }
    if ($diagnostics.httpsStatus -ne "ok" -and -not $AllowHttp) {
        throw "Admin diagnostics httpsStatus must be ok for production, got '$($diagnostics.httpsStatus)'."
    }
    if ($diagnostics.restartPolicy -ne "compose:unless-stopped") {
        throw "Admin diagnostics restartPolicy must be compose:unless-stopped, got '$($diagnostics.restartPolicy)'."
    }
    if ($RequirePublicCors -and $diagnostics.publicCors -ne "allow_any_origin") {
        throw "Admin diagnostics publicCors must be allow_any_origin, got '$($diagnostics.publicCors)'."
    }
    if ($RequireSteamConfigured -and $diagnostics.steam -ne "configured") {
        throw "Admin diagnostics steam must be configured for real catalog generation, got '$($diagnostics.steam)'."
    }

    $share = Invoke-MpgsRest "GET" "$base/api/v1/admin/connection-share" $null $session
    if ($share.baseUrl -ne $diagnostics.publicBaseUrl) {
        throw "Connection-share baseUrl must match diagnostics publicBaseUrl."
    }
    if (@($share.capabilities) -notcontains "public_catalog_read") {
        throw "Connection-share file must include public_catalog_read."
    }
    if (($share | ConvertTo-Json -Depth 8) -match 'token|secret|api[_-]?key|password') {
        throw "Connection-share response must not include secret-looking fields."
    }
}

Write-Output "Production readiness validation passed for $base."
