param(
    [string] $ImageTag = "mpgs-server:local",
    [string] $OutputTar = "mpgs-server-local.tar",
    [string] $Platform = "",
    [string] $RustBaseImage = "",
    [string] $NodeBaseImage = "",
    [string] $DebianBaseImage = "",
    [switch] $UseBuildx
)

$ErrorActionPreference = "Stop"

function Require-Command {
    param([string] $Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name is required on the local build machine."
    }
}

function Add-BuildArg {
    param(
        [string[]] $BuildArgs,
        [string] $Name,
        [string] $Value
    )

    if (-not $Value) {
        return $BuildArgs
    }

    return $BuildArgs[0..($BuildArgs.Length - 2)] + @("--build-arg", "$Name=$Value") + $BuildArgs[($BuildArgs.Length - 1)]
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
    $buildArgs = Add-BuildArg $buildArgs "RUST_BASE_IMAGE" $RustBaseImage
    $buildArgs = Add-BuildArg $buildArgs "NODE_BASE_IMAGE" $NodeBaseImage
    $buildArgs = Add-BuildArg $buildArgs "DEBIAN_BASE_IMAGE" $DebianBaseImage

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
