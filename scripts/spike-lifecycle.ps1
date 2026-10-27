<#
.SYNOPSIS
Runs the TOKENRADAR_SPIKE=lifecycle scenario several times and summarizes memory and latency.

.DESCRIPTION
For each run the script starts the app with the scenario enabled, samples its process tree with
measure-memory.ps1 until the app exits, and joins the scenario's timestamped markers with the
samples. Memory at a marker is the median of the samples in the 2 s before it. The summary table
holds the median of all runs.

Build the app with the scenario compiled in first, for example:
  pnpm tauri build --no-bundle --features spike     (release, representative)
  cargo build -p tokenradar-app                     (debug; needs `pnpm dev` for the frontend)

.PARAMETER Exe
Path to tokenradar-app.exe. Default: target/release/tokenradar-app.exe.

.PARAMETER Runs
Number of runs. Default 3.

.PARAMETER IntervalMs
Sampling interval passed to measure-memory.ps1. Default 250.

.PARAMETER OutDir
Directory for the raw CSV/log files and the summary. Default docs/perf/raw.

.PARAMETER Tag
File name prefix. Default "<yyyy-MM-dd>-lifecycle".

.PARAMETER BrowserArgs
Extra WebView2 browser arguments for every spike window (TOKENRADAR_SPIKE_BROWSER_ARGS).

.EXAMPLE
.\scripts\spike-lifecycle.ps1 -Runs 3

.EXAMPLE
.\scripts\spike-lifecycle.ps1 -Tag 2026-10-27-lifecycle-rpl1 -BrowserArgs '--renderer-process-limit=1'
#>
[CmdletBinding()]
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\release\tokenradar-app.exe'),
    [ValidateRange(1, 20)]
    [int]$Runs = 3,
    [ValidateRange(50, 5000)]
    [int]$IntervalMs = 250,
    [string]$OutDir = (Join-Path $PSScriptRoot '..\docs\perf\raw'),
    [string]$Tag = ((Get-Date -Format 'yyyy-MM-dd') + '-lifecycle'),
    [string]$BrowserArgs
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
$inv = [System.Globalization.CultureInfo]::InvariantCulture

$marks = 'baseline', 'panel_open', 'panel_hidden', 'after_destroy1', 'one_window', 'two_windows',
    'after_destroy2', 'hidden_window', 'hidden_controller', 'hidden_low', 'end'

function Get-Median([double[]]$values) {
    $sorted = @($values | Where-Object { -not [double]::IsNaN($_) } | Sort-Object)
    if ($sorted.Count -eq 0) { return [double]::NaN }
    $mid = [int][math]::Floor($sorted.Count / 2)
    if ($sorted.Count % 2) { $sorted[$mid] } else { ($sorted[$mid - 1] + $sorted[$mid]) / 2 }
}

function Read-Samples($csv) {
    Import-Csv $csv | ForEach-Object {
        [pscustomobject]@{
            unix_ms    = [long]$_.unix_ms
            processes  = [int]$_.processes
            webview    = [int]$_.webview_processes
            private_ws = [double]::Parse($_.private_ws_mb, $inv)
            private_b  = [double]::Parse($_.private_bytes_mb, $inv)
        }
    }
}

function Read-Log($log) {
    $markTimes = @{}
    $metrics = @{}
    foreach ($line in Get-Content $log) {
        $parts = $line.Split(' ', 3)
        if ($parts.Count -lt 3) { continue }
        switch ($parts[1]) {
            'mark' { $markTimes[$parts[2]] = [long]$parts[0] }
            'metric' {
                $name, $value = $parts[2].Split('=')
                $metrics[$name] = [double]::Parse($value, $inv)
            }
            'error' { Write-Warning "scenario error: $($parts[2])" }
        }
    }
    @{ Marks = $markTimes; Metrics = $metrics }
}

function Get-RunSummary($samples, $log) {
    $row = [ordered]@{}
    foreach ($name in $marks) {
        $at = $log.Marks[$name]
        $window = @($samples | Where-Object { $at -and $_.unix_ms -gt $at - 2000 -and $_.unix_ms -le $at })
        $row["${name}_ws"] = if ($window) { Get-Median ($window | ForEach-Object private_ws) } else { [double]::NaN }
        $row["${name}_pb"] = if ($window) { Get-Median ($window | ForEach-Object private_b) } else { [double]::NaN }
        $row["${name}_procs"] = if ($window) { Get-Median ($window | ForEach-Object processes) } else { [double]::NaN }
    }
    # How long after each destroy until no msedgewebview2 process is left.
    foreach ($d in 'destroy1', 'destroy2', 'destroy3') {
        $at = $log.Marks[$d]
        $gone = $samples | Where-Object { $at -and $_.unix_ms -ge $at -and $_.webview -eq 0 } | Select-Object -First 1
        $row["${d}_webview_exit_ms"] = if ($gone) { [double]($gone.unix_ms - $at) } else { [double]::NaN }
    }
    $row['peak_ws'] = ($samples | Measure-Object private_ws -Maximum).Maximum
    foreach ($k in $log.Metrics.Keys) { $row[$k] = $log.Metrics[$k] }
    [pscustomobject]$row
}

# --- Run ---------------------------------------------------------------------------------------

if (-not (Test-Path $Exe)) { throw "Not found: $Exe" }
if (Get-Process tokenradar-app -ErrorAction SilentlyContinue) {
    throw 'tokenradar-app is already running; single-instance would swallow the spike run. Quit it first.'
}
New-Item -ItemType Directory -Force $OutDir | Out-Null

$summaries = foreach ($i in 1..$Runs) {
    $csv = Join-Path $OutDir "$Tag-run$i.csv"
    $log = Join-Path $OutDir "$Tag-run$i.log"
    Write-Host "run $i/$Runs ..."
    $env:TOKENRADAR_SPIKE = 'lifecycle'
    $env:TOKENRADAR_SPIKE_LOG = $log
    if ($BrowserArgs) { $env:TOKENRADAR_SPIKE_BROWSER_ARGS = $BrowserArgs }
    try {
        $process = Start-Process -FilePath $Exe -PassThru
    } finally {
        Remove-Item Env:TOKENRADAR_SPIKE, Env:TOKENRADAR_SPIKE_LOG -ErrorAction SilentlyContinue
        Remove-Item Env:TOKENRADAR_SPIKE_BROWSER_ARGS -ErrorAction SilentlyContinue
    }
    & (Join-Path $PSScriptRoot 'measure-memory.ps1') -RootPid $process.Id -IntervalMs $IntervalMs -Out $csv -Quiet
    $process.WaitForExit()
    Get-RunSummary (Read-Samples $csv) (Read-Log $log)
}

# --- Summarize ---------------------------------------------------------------------------------

$names = $summaries[0].PSObject.Properties.Name
$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add("# Lifecycle spike: $Tag")
$lines.Add('')
$lines.Add("Runs: $Runs; sampling every $IntervalMs ms; exe: $(Split-Path -Leaf $Exe)$(if ($BrowserArgs) { "; browser args: $BrowserArgs" })")
$lines.Add('')
$lines.Add('| Value | Median | ' + ((1..$Runs | ForEach-Object { "Run $_" }) -join ' | ') + ' |')
$lines.Add('|---|---|' + ('---|' * $Runs))
foreach ($n in $names) {
    $values = @($summaries | ForEach-Object { [double]$_.$n })
    $fmt = { param($v) if ([double]::IsNaN($v)) { '–' } else { $v.ToString('0.0', $inv) } }
    $lines.Add("| $n | $(& $fmt (Get-Median $values)) | " + (($values | ForEach-Object { & $fmt $_ }) -join ' | ') + ' |')
}
$summary = Join-Path $OutDir "$Tag-summary.md"
[System.IO.File]::WriteAllLines($summary, $lines, [System.Text.UTF8Encoding]::new($false))
$lines | Write-Host
Write-Host "`nSummary written to $summary"
