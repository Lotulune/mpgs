param(
    [string] $ImageTag = "mpgs-server:local",
    [string] $OutputTar = "mpgs-server-local.tar",
    [string] $Platform = "",
    [switch] $UseBuildx
)

$ErrorActionPreference = "Stop"

function Require-Command {
    param([string] $Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name is required on the local build machine."
    }
}

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$outputPath = if ([System.IO.Path]::IsPathRooted($OutputTar)) {
    $OutputTar
} else {
    Join-Path $root $OutputTar
}

Require-Command "docker"

Push-Location $root
try {
    if ($UseBuildx -or $Platform) {
        $buildArgs = @("buildx", "build", "-f", "Dockerfile.mpgs-server", "-t", $ImageTag, "--load")
        if ($Platform) {
            $buildArgs += @("--platform", $Platform)
        }
        $buildArgs += "."
    } else {
        $buildArgs = @("build", "-f", "Dockerfile.mpgs-server", "-t", $ImageTag, ".")
    }

    Write-Host "Building $ImageTag from Dockerfile.mpgs-server on the local machine..."
    & docker @buildArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Local Docker image build failed."
    }

    Write-Host "Saving $ImageTag to $outputPath..."
    & docker save $ImageTag -o $outputPath
    if ($LASTEXITCODE -ne 0) {
        throw "Docker image save failed."
    }

    Write-Host "Created image archive: $outputPath"
}
finally {
    Pop-Location
}
