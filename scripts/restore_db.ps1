# Restore a backup into a new directory/path and verify integrity.
# Usage: ./scripts/restore_db.ps1 -From .\backups\mpgs.db -To .\data-restored\mpgs.db

param(
    [Parameter(Mandatory = $true)]
    [string]$From,
    [Parameter(Mandatory = $true)]
    [string]$To
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

cargo run -p mpgs-dbtool --quiet -- restore $From $To
exit $LASTEXITCODE
