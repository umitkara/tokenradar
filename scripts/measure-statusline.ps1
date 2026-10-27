<#
.SYNOPSIS
Measures the latency of `tokenradar statusline` from process start to exit, as Claude Code sees it.

.DESCRIPTION
Runs the release CLI many times with a synthetic payload on stdin, in two modes:
  - on   the core's IPC server runs (examples/ipc_serve.rs): push + status.get over the pipe
  - off  nothing listens: the reading goes to the spool

The CLI runs from a temporary copy marked portable, so its spool and config live in the temporary
directory. Nothing touches the user's TokenRadar data or Claude Code settings. The payload is
synthetic: fresh reset times and a transcript path inside a temporary config directory.

If the TokenRadar app itself serves the pipe, "on" cannot start its own server and "off" would
talk to the app; the spool line counts printed at the end show which path ran.

.PARAMETER Runs
Measured calls per mode. Default 200.

.PARAMETER Warmup
Unmeasured calls before each mode. Default 10.

.PARAMETER Mode
both (default), on or off.

.PARAMETER NoBuild
Use the existing release binaries.

.EXAMPLE
.\scripts\measure-statusline.ps1

.EXAMPLE
.\scripts\measure-statusline.ps1 -Runs 500 -Mode on -NoBuild
#>
param(
    [int]$Runs = 200,
    [int]$Warmup = 10,
    [ValidateSet('both', 'on', 'off')][string]$Mode = 'both',
    [switch]$NoBuild
)

$ErrorActionPreference = 'Stop'
$inv = [Globalization.CultureInfo]::InvariantCulture
$root = Split-Path -Parent $PSScriptRoot

if (-not $NoBuild) {
    cargo build --release -q -p tokenradar-cli
    if ($LASTEXITCODE) { throw 'cargo build -p tokenradar-cli failed' }
    cargo build --release -q -p tokenradar-core --example ipc_serve
    if ($LASTEXITCODE) { throw 'cargo build --example ipc_serve failed' }
}
$cli = Join-Path $root 'target\release\tokenradar.exe'
$serveExe = Join-Path $root 'target\release\examples\ipc_serve.exe'

$tmp = Join-Path ([IO.Path]::GetTempPath()) ('tokenradar-measure-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$bin = Join-Path $tmp 'bin'
$cfg = Join-Path $tmp 'claude'
$project = Join-Path $cfg 'projects\C--Work-Alpha'
New-Item -ItemType Directory -Force $bin, $project | Out-Null
Copy-Item $cli $bin
New-Item -ItemType File (Join-Path $bin 'portable') | Out-Null
$exe = Join-Path $bin 'tokenradar.exe'
$spool = Join-Path $bin 'data\spool'

function New-Payload([double]$percent) {
    $now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    @{
        session_id      = '00000000-0000-4000-8000-000000000001'
        transcript_path = (Join-Path $project '00000000-0000-4000-8000-000000000001.jsonl')
        cwd             = 'C:\Work\Alpha'
        rate_limits     = @{
            five_hour = @{ used_percentage = $percent; resets_at = $now + 7200 }
            seven_day = @{ used_percentage = 41.2; resets_at = $now + 3 * 86400 }
        }
    } | ConvertTo-Json -Compress -Depth 5
}

function Invoke-Statusline([string]$payload) {
    $psi = [Diagnostics.ProcessStartInfo]::new($exe, 'statusline')
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.Environment['NO_COLOR'] = '1'
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $p = [Diagnostics.Process]::Start($psi)
    $p.StandardInput.Write($payload)
    $p.StandardInput.Close()
    $out = $p.StandardOutput.ReadToEnd()
    $p.WaitForExit()
    $sw.Stop()
    [pscustomobject]@{ Ms = $sw.Elapsed.TotalMilliseconds; Out = $out.Trim() }
}

function Get-SpoolLines {
    if (-not (Test-Path $spool)) { return 0 }
    (Get-ChildItem $spool -File | Get-Content | Measure-Object -Line).Lines
}

function Measure-Mode([string]$name) {
    for ($i = 0; $i -lt $Warmup; $i++) { $null = Invoke-Statusline (New-Payload 10) }
    $before = Get-SpoolLines
    $samples = for ($i = 0; $i -lt $Runs; $i++) { Invoke-Statusline (New-Payload (10 + $i % 80)) }
    $ms = @($samples.Ms | Sort-Object)
    $q = { param($p) $ms[[math]::Max(0, [math]::Ceiling($p * $ms.Count) - 1)] }
    [pscustomobject]@{
        Mode    = $name
        Runs    = $Runs
        P50     = (& $q 0.50).ToString('0.0', $inv)
        P95     = (& $q 0.95).ToString('0.0', $inv)
        Max     = $ms[-1].ToString('0.0', $inv)
        Spooled = (Get-SpoolLines) - $before
        Line    = $samples[-1].Out
    }
}

$results = @()
$server = $null
try {
    if ($Mode -in 'both', 'off') {
        $results += Measure-Mode 'off'
    }
    if ($Mode -in 'both', 'on') {
        $psi = [Diagnostics.ProcessStartInfo]::new($serveExe)
        $psi.ArgumentList.Add($cfg)
        $psi.ArgumentList.Add((Join-Path $tmp 'db.sqlite'))
        $psi.UseShellExecute = $false
        $psi.RedirectStandardInput = $true
        $psi.RedirectStandardOutput = $true
        $server = [Diagnostics.Process]::Start($psi)
        $ready = $server.StandardOutput.ReadLine()
        if ($ready -ne 'READY') { throw "ipc_serve did not start (got '$ready')" }
        $results += Measure-Mode 'on'
    }
}
finally {
    if ($server) {
        $server.StandardInput.Close()
        if (-not $server.WaitForExit(10000)) { $server.Kill() }
    }
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

$results | Format-Table -AutoSize Mode, Runs, P50, P95, Max, Spooled
$results | ForEach-Object { "{0}: {1}" -f $_.Mode, $_.Line }
"Budget: p95 <= 30 ms (on), <= 40 ms (off). 'Spooled' should equal Runs for off and 0 for on."
