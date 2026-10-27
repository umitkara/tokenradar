<#
.SYNOPSIS
Runs the TOKENRADAR_PANEL_SCENARIO=lifecycle scenario several times and summarizes the panel's
open latency and memory against the budgets.

.DESCRIPTION
For each run the script starts the app with the scenario and a fake panel scene (so the core and
real data are never touched), samples its process tree with measure-memory.ps1 until the app
exits, and joins the scenario's markers with the samples. Memory at a marker is the median of the
private working set samples in the 2 s before it. Latency is the time from the open request to
the second animation frame after the panel was shown (`painted cold|warm <ms>`). The summary
table holds the median of all runs.

Build the app with the scenario compiled in first:
  pnpm tauri build --no-bundle --features spike

.PARAMETER Exe
Path to tokenradar-app.exe. Default: target/release/tokenradar-app.exe.

.PARAMETER Runs
Number of runs. Default 3.

.PARAMETER Scene
Fake panel scene (TOKENRADAR_FAKE_LIMITS=panel:<scene>). Default "ok".

.PARAMETER BrowserArgs
Extra WebView2 browser arguments (WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS), e.g. --disable-gpu.

.PARAMETER OutDir
Directory for the raw CSV/log files and the summary. Default docs/perf/raw.

.PARAMETER Tag
File name prefix. Default "<yyyy-MM-dd>-panel".

.EXAMPLE
.\scripts\measure-panel.ps1 -Runs 3

.EXAMPLE
.\scripts\measure-panel.ps1 -Tag 2026-10-27-panel-nogpu -BrowserArgs '--disable-gpu'
#>
[CmdletBinding()]
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\release\tokenradar-app.exe'),
    [ValidateRange(1, 20)]
    [int]$Runs = 3,
    [string]$Scene = 'ok',
    [string]$BrowserArgs,
    [ValidateRange(50, 5000)]
    [int]$IntervalMs = 250,
    [string]$OutDir = (Join-Path $PSScriptRoot '..\docs\perf\raw'),
    [string]$Tag = ((Get-Date -Format 'yyyy-MM-dd') + '-panel')
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
$inv = [System.Globalization.CultureInfo]::InvariantCulture
$rounds = 1..3

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
        }
    }
}

function Read-Log($log) {
    $markTimes = @{}
    $painted = @{ cold = [System.Collections.Generic.List[double]]::new(); warm = [System.Collections.Generic.List[double]]::new() }
    foreach ($line in Get-Content $log) {
        $parts = $line.Split(' ', 3)
        if ($parts.Count -lt 3) { continue }
        switch ($parts[1]) {
            'mark' { $markTimes[$parts[2]] = [long]$parts[0] }
            'painted' {
                $kind, $ms = $parts[2].Split(' ')
                $painted[$kind].Add([double]::Parse($ms, $inv))
            }
        }
    }
    @{ Marks = $markTimes; Painted = $painted }
}

function Get-RunSummary($samples, $log) {
    $row = [ordered]@{}
    $at = { param($name)
        $t = $log.Marks[$name]
        $window = @($samples | Where-Object { $t -and $_.unix_ms -gt $t - 2000 -and $_.unix_ms -le $t })
        if ($window) { Get-Median ($window | ForEach-Object private_ws) } else { [double]::NaN }
    }
    $row['baseline_ws'] = & $at 'baseline'
    foreach ($r in $rounds) {
        $row["cold$r`_ms"] = if ($log.Painted.cold.Count -ge $r) { $log.Painted.cold[$r - 1] } else { [double]::NaN }
        $row["warm$r`_ms"] = if ($log.Painted.warm.Count -ge $r) { $log.Painted.warm[$r - 1] } else { [double]::NaN }
        $row["open$r`_ws"] = & $at "open$r"
        $row["hidden$r`_ws"] = & $at "hidden$r"
        $row["ttl$r`_ws"] = & $at "ttl$r"
    }
    $row['peak_ws'] = ($samples | Measure-Object private_ws -Maximum).Maximum
    [pscustomobject]$row
}

# --- Run ---------------------------------------------------------------------------------------

if (-not (Test-Path $Exe)) { throw "Not found: $Exe" }
if (Get-Process tokenradar-app -ErrorAction SilentlyContinue) {
    throw 'tokenradar-app is already running; single-instance would swallow the run. Quit it first.'
}
New-Item -ItemType Directory -Force $OutDir | Out-Null

$saved = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$summaries = foreach ($i in 1..$Runs) {
    $csv = Join-Path $OutDir "$Tag-run$i.csv"
    $log = Join-Path $OutDir "$Tag-run$i.log"
    Remove-Item $log -ErrorAction SilentlyContinue
    Write-Host "run $i/$Runs ..."
    $env:TOKENRADAR_PANEL_SCENARIO = 'lifecycle'
    $env:TOKENRADAR_PANEL_LOG = $log
    $env:TOKENRADAR_FAKE_LIMITS = "panel:$Scene"
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $BrowserArgs
    try {
        $process = Start-Process -FilePath $Exe -PassThru
    } finally {
        Remove-Item Env:TOKENRADAR_PANEL_SCENARIO, Env:TOKENRADAR_PANEL_LOG, Env:TOKENRADAR_FAKE_LIMITS -ErrorAction SilentlyContinue
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $saved
    }
    & (Join-Path $PSScriptRoot 'measure-memory.ps1') -RootPid $process.Id -IntervalMs $IntervalMs -Out $csv -Quiet
    $process.WaitForExit()
    Get-RunSummary (Read-Samples $csv) (Read-Log $log)
}

# --- Summarize ---------------------------------------------------------------------------------

$fmt = { param($v) if ([double]::IsNaN($v)) { '–' } else { $v.ToString('0.0', $inv) } }
$all = { param($pattern) @($summaries | ForEach-Object { $s = $_; $s.PSObject.Properties.Name | Where-Object { $_ -like $pattern } | ForEach-Object { [double]$s.$_ } }) }
$budget = @(
    @('Cold open (median of all rounds)', (Get-Median (& $all 'cold*_ms')), 400, 'ms'),
    @('Warm open (median of all rounds)', (Get-Median (& $all 'warm*_ms')), 50, 'ms'),
    @('Panel open, private WS (max of round medians)', (& $all 'open*_ws' | Measure-Object -Maximum).Maximum, 130, 'MB'),
    @('warm_ttl + 10 s, private WS (max)', (& $all 'ttl*_ws' | Measure-Object -Maximum).Maximum, 30, 'MB')
)

$names = $summaries[0].PSObject.Properties.Name
$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add("# Panel lifecycle: $Tag")
$lines.Add('')
$lines.Add("Runs: $Runs; scene: $Scene; sampling every $IntervalMs ms; exe: $(Split-Path -Leaf $Exe)$(if ($BrowserArgs) { "; browser args: $BrowserArgs" })")
$lines.Add('')
$lines.Add('| Budget | Value | Limit | |')
$lines.Add('|---|---|---|---|')
foreach ($b in $budget) {
    $ok = if ([double]::IsNaN($b[1])) { '?' } elseif ($b[1] -le $b[2]) { 'ok' } else { 'OVER' }
    $lines.Add("| $($b[0]) | $(& $fmt $b[1]) $($b[3]) | $($b[2]) $($b[3]) | $ok |")
}
$lines.Add('')
$lines.Add('| Value | Median | ' + ((1..$Runs | ForEach-Object { "Run $_" }) -join ' | ') + ' |')
$lines.Add('|---|---|' + ('---|' * $Runs))
foreach ($n in $names) {
    $values = @($summaries | ForEach-Object { [double]$_.$n })
    $lines.Add("| $n | $(& $fmt (Get-Median $values)) | " + (($values | ForEach-Object { & $fmt $_ }) -join ' | ') + ' |')
}
$summary = Join-Path $OutDir "$Tag-summary.md"
[System.IO.File]::WriteAllLines($summary, $lines, [System.Text.UTF8Encoding]::new($false))
$lines | Write-Host
Write-Host "`nSummary written to $summary"
