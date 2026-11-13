# Keeps `target/debug` from growing without bound.
#
# Cargo never garbage-collects stale artifacts in the target directory: every change in
# features, RUSTFLAGS, or crate sources leaves the old `deps/*` files and `incremental/*`
# sessions behind. This script measures `target/debug` and runs `cargo clean --profile dev`
# when it is larger than the limit. The next build is a full rebuild.
#
# Usage:
#   pwsh scripts/trim-target.ps1            # clean only above 20 GB
#   pwsh scripts/trim-target.ps1 -LimitGB 10
#   pwsh scripts/trim-target.ps1 -Force     # clean regardless of size

param(
    [double]$LimitGB = 20,
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$debugDir = Join-Path $root 'target/debug'

if (-not (Test-Path $debugDir)) {
    Write-Host 'target/debug does not exist; nothing to trim.'
    exit 0
}

$bytes = (Get-ChildItem $debugDir -Recurse -File -Force -ErrorAction SilentlyContinue |
    Measure-Object Length -Sum).Sum
$sizeGB = [math]::Round($bytes / 1GB, 1)

if (-not $Force -and $sizeGB -le $LimitGB) {
    Write-Host "target/debug is $sizeGB GB (limit $LimitGB GB); keeping it."
    exit 0
}

Write-Host "target/debug is $sizeGB GB (limit $LimitGB GB); cleaning the dev profile."
Push-Location $root
try {
    cargo clean --profile dev
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}
