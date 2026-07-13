# Controlled SQLite backup for an active MPGS database.
# Usage: ./scripts/backup_db.ps1 -DbPath .\data\mpgs.db -OutPath .\backups\mpgs-YYYYMMDD.db

param(
    [Parameter(Mandatory = $true)]
    [string]$DbPath,
    [Parameter(Mandatory = $true)]
    [string]$OutPath
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

cargo run -p mpgs-dbtool --quiet -- backup $DbPath $OutPath
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

cargo run -p mpgs-dbtool --quiet -- integrity $OutPath
exit $LASTEXITCODE
