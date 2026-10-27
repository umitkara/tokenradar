<#
.SYNOPSIS
Packs the portable zip from an existing release build (Step 12).

.DESCRIPTION
Run `pnpm tauri build` first: it builds the app, the size-optimized CLI and the third-party
notices. The zip holds:

  tokenradar-app.exe
  bin\tokenradar.exe          the CLI, under bin\ as in the installer (ADR-017)
  portable                    empty marker: data stays in .\data next to the app
  LICENSE.txt
  THIRD-PARTY-NOTICES.txt

The CLI under bin\ finds the marker one level up (paths.rs), so both binaries share .\data.
Scoop installs this zip too (bucket/tokenradar.json persists .\data).

.PARAMETER OutDir
Where the zip goes. Default target\release\bundle\portable.

.EXAMPLE
.\scripts\package-portable.ps1
#>
param(
    [string]$OutDir
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
if (-not $OutDir) { $OutDir = Join-Path $root 'target\release\bundle\portable' }

$metadata = cargo metadata --format-version 1 --no-deps --locked --manifest-path (Join-Path $root 'Cargo.toml') | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -eq 'tokenradar-app').version

$files = [ordered]@{
    'tokenradar-app.exe'      = 'target\release\tokenradar-app.exe'
    'bin\tokenradar.exe'      = 'target\release-cli\tokenradar.exe'
    'LICENSE.txt'             = 'LICENSE'
    'THIRD-PARTY-NOTICES.txt' = 'target\third-party\THIRD-PARTY-NOTICES.txt'
}
foreach ($source in $files.Values) {
    if (-not (Test-Path (Join-Path $root $source) -PathType Leaf)) {
        throw "$source is missing; run 'pnpm tauri build' first"
    }
}

$stage = Join-Path ([IO.Path]::GetTempPath()) "tokenradar-portable-$PID"
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force (Join-Path $stage 'bin') | Out-Null
try {
    foreach ($entry in $files.GetEnumerator()) {
        Copy-Item (Join-Path $root $entry.Value) (Join-Path $stage $entry.Key)
    }
    New-Item -ItemType File (Join-Path $stage 'portable') | Out-Null

    New-Item -ItemType Directory -Force $OutDir | Out-Null
    $zip = Join-Path $OutDir "TokenRadar_${version}_x64-portable.zip"
    if (Test-Path $zip) { Remove-Item -Force $zip }
    # ZipFile writes '/' separators; Windows PowerShell's Compress-Archive may write '\'.
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [IO.Compression.ZipFile]::CreateFromDirectory($stage, $zip, [IO.Compression.CompressionLevel]::Optimal, $false)
} finally {
    Remove-Item -Recurse -Force $stage
}

$hash = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLowerInvariant()
$size = (Get-Item $zip).Length
'{0}  ({1:N2} MB)' -f $zip, ($size / 1MB)
"SHA256 $hash"
