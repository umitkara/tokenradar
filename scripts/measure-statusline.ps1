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

.PARAMETER Profile
Cargo profile of the CLI: release-cli (default, the shipped size-optimized build) or release.

.PARAMETER NoBuild
Use the existing binaries.

.PARAMETER Cli
Measure this tokenradar.exe instead of the profile's build, e.g. an older build for an A/B run.

.EXAMPLE
.\scripts\measure-statusline.ps1

.EXAMPLE
.\scripts\measure-statusline.ps1 -Runs 500 -Mode on -NoBuild

.EXAMPLE
.\scripts\measure-statusline.ps1 -NoBuild -Cli D:\old\tokenradar.exe

.PARAMETER Config
A config.toml for the portable copy, e.g. a [statusline] table with more items.

.PARAMETER Cwd
The payload's cwd; a git repository makes a git_branch item read .git/HEAD. Default C:\Work\Alpha.

.EXAMPLE
.\scripts\measure-statusline.ps1 -NoBuild -Config .\detailed.toml -Cwd D:\src\repo
#>
[CmdletBinding()]
param(
    [int]$Runs = 200,
    [int]$Warmup = 10,
    [ValidateSet('both', 'on', 'off')][string]$Mode = 'both',
    [ValidateSet('release-cli', 'release')][string]$Profile = 'release-cli',
    [switch]$NoBuild,
    [string]$Cli,
    [string]$Config,
    [string]$Cwd = 'C:\Work\Alpha'
)

$ErrorActionPreference = 'Stop'
$inv = [Globalization.CultureInfo]::InvariantCulture
$root = Split-Path -Parent $PSScriptRoot

if (-not $NoBuild) {
    cargo build --profile $Profile -q -p tokenradar-cli
    if ($LASTEXITCODE) { throw 'cargo build -p tokenradar-cli failed' }
    cargo build --release -q -p tokenradar-core --example ipc_serve
    if ($LASTEXITCODE) { throw 'cargo build --example ipc_serve failed' }
}
$cli = if ($Cli) { $Cli } else { Join-Path $root "target\$Profile\tokenradar.exe" }
$serveExe = Join-Path $root 'target\release\examples\ipc_serve.exe'

$tmp = Join-Path ([IO.Path]::GetTempPath()) ('tokenradar-measure-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$bin = Join-Path $tmp 'bin'
$cfg = Join-Path $tmp 'claude'
$project = Join-Path $cfg 'projects\C--Work-Alpha'
New-Item -ItemType Directory -Force $bin, $project | Out-Null
Copy-Item $cli $bin
New-Item -ItemType File (Join-Path $bin 'portable') | Out-Null
# Step 12n: a synthetic transcript with one compaction, so `status.get` finds the session and
# reads its count, and user settings with an auto-compact window for a `compact` item.
$session = '00000000-0000-4000-8000-000000000001'
$boundary = @{
    type = 'system'; subtype = 'compact_boundary'; isSidechain = $false
    timestamp = [DateTimeOffset]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ss.fffZ')
    uuid = '00000000-0000-4000-8000-0000000000c1'; sessionId = $session
    compactMetadata = @{ trigger = 'auto'; preTokens = 150000 }
} | ConvertTo-Json -Compress -Depth 3
[IO.File]::WriteAllText((Join-Path $project "$session.jsonl"), $boundary + "`n")
[IO.File]::WriteAllText((Join-Path $cfg 'settings.json'), '{"autoCompactWindow": 300000}')
$exe = Join-Path $bin 'tokenradar.exe'
if ($Config) {
    New-Item -ItemType Directory -Force (Join-Path $bin 'data') | Out-Null
    Copy-Item $Config (Join-Path $bin 'data\config.toml')
}
$spool = Join-Path $bin 'data\spool'

function New-Payload([double]$percent) {
    $now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    @{
        session_id      = '00000000-0000-4000-8000-000000000001'
        transcript_path = (Join-Path $project '00000000-0000-4000-8000-000000000001.jsonl')
        cwd             = $Cwd
        model           = @{ id = 'claude-opus-5-5'; display_name = 'Opus 5.5' }
        session_name    = 'synthetic sample'
        output_style    = @{ name = 'default' }
        cost            = @{ total_cost_usd = 4.12; total_duration_ms = 1234567 }
        context_window  = @{
            used_percentage = 38; context_window_size = 200000
            # Step 12m: the cache ratio and the tokens left.
            current_usage   = @{ input_tokens = 8500; cache_creation_input_tokens = 5000; cache_read_input_tokens = 62500 }
        }
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
