<#
.SYNOPSIS
Samples the memory and CPU time of a process together with its whole child-process tree.

.DESCRIPTION
Finds the root process (by name or PID) and, on every sample, walks its descendants through the
parent process IDs. This is what makes WebView2 visible: msedgewebview2.exe runs as a child of the
app, and its renderer/GPU/utility processes are children of that browser process. Matching by
name alone would also count WebView2 processes of unrelated apps.

Each sample records, summed over the tree:
  - private_ws_mb     private working set (Task Manager "Memory" column). Budgets use this value.
  - private_bytes_mb  private bytes / commit (Process.PrivateMemorySize64).
  - working_set_mb    working set including shared pages (Process.WorkingSet64).
  - cpu_s             total CPU time (user + kernel) of the processes alive at that moment.
  - processes, webview_processes

Numbers are written with invariant culture, so the CSV parses the same on every locale.

.PARAMETER ProcessName
Root process name without ".exe", for example "tokenradar-app".

.PARAMETER RootPid
Root process ID. Use instead of -ProcessName when several instances could exist.

.PARAMETER IntervalMs
Sampling interval in milliseconds. One sample costs about 50-150 ms of WMI time.

.PARAMETER DurationSec
How long to sample. 0 (default) samples until the root process exits.

.PARAMETER Out
Optional CSV path. The directory is created if needed.

.PARAMETER WaitSec
How long to wait for the root process to appear. Default 30.

.PARAMETER Quiet
Do not print the summary table.

.EXAMPLE
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -IntervalMs 500 -DurationSec 180

.EXAMPLE
.\scripts\measure-memory.ps1 -RootPid 1234 -Out docs/perf/raw/2026-10-27-lifecycle.csv

.NOTES
Used by scripts/spike-lifecycle.ps1 and by every later step that measures a memory budget
(docs/performance-budget.md).
#>
[CmdletBinding(DefaultParameterSetName = 'ByName')]
param(
    [Parameter(Mandatory, ParameterSetName = 'ByName')]
    [string]$ProcessName,

    [Parameter(Mandatory, ParameterSetName = 'ByPid')]
    [int]$RootPid,

    [ValidateRange(50, 60000)]
    [int]$IntervalMs = 500,

    [ValidateRange(0, 86400)]
    [int]$DurationSec = 0,

    [string]$Out,

    [ValidateRange(0, 3600)]
    [int]$WaitSec = 30,

    [switch]$Quiet
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
$inv = [System.Globalization.CultureInfo]::InvariantCulture

function Get-ProcessTable {
    # One WMI round trip returns parent links and every counter we need for all processes.
    Get-CimInstance -ClassName Win32_PerfRawData_PerfProc_Process `
        -Property Name, IDProcess, CreatingProcessID, PrivateBytes, WorkingSet, WorkingSetPrivate, PercentProcessorTime |
        Where-Object { $_.IDProcess -ne 0 -and $_.Name -ne '_Total' }
}

function Find-Roots($table) {
    if ($PSCmdlet.ParameterSetName -eq 'ByPid') {
        return @($table | Where-Object { $_.IDProcess -eq $RootPid } | ForEach-Object { $_.IDProcess })
    }
    # Perf instance names look like "name" or "name#2" when several processes share a name.
    $matching = @($table | Where-Object { $_.Name -eq $ProcessName -or $_.Name -like "$ProcessName#*" })
    $ids = @($matching | ForEach-Object { $_.IDProcess })
    # A root is a matching process whose parent is not itself a match.
    @($matching | Where-Object { $ids -notcontains $_.CreatingProcessID } | ForEach-Object { $_.IDProcess })
}

function Get-Tree($table, [int[]]$roots) {
    $children = @{}
    foreach ($p in $table) {
        $parent = [int]$p.CreatingProcessID
        if (-not $children.ContainsKey($parent)) { $children[$parent] = [System.Collections.Generic.List[object]]::new() }
        $children[$parent].Add($p)
    }
    $byId = @{}
    foreach ($p in $table) { $byId[[int]$p.IDProcess] = $p }

    $tree = [System.Collections.Generic.List[object]]::new()
    $queue = [System.Collections.Generic.Queue[int]]::new()
    $seen = [System.Collections.Generic.HashSet[int]]::new()
    foreach ($r in $roots) {
        if ($byId.ContainsKey($r) -and $seen.Add($r)) { $tree.Add($byId[$r]); $queue.Enqueue($r) }
    }
    while ($queue.Count -gt 0) {
        $id = $queue.Dequeue()
        if (-not $children.ContainsKey($id)) { continue }
        foreach ($c in $children[$id]) {
            if ($seen.Add([int]$c.IDProcess)) { $tree.Add($c); $queue.Enqueue([int]$c.IDProcess) }
        }
    }
    $tree
}

# --- Wait for the root -------------------------------------------------------------------------

$deadline = [DateTime]::UtcNow.AddSeconds($WaitSec)
do {
    $roots = @(Find-Roots (Get-ProcessTable))
    if ($roots.Count -gt 0) { break }
    Start-Sleep -Milliseconds 250
} while ([DateTime]::UtcNow -lt $deadline)
if ($roots.Count -eq 0) {
    $what = if ($PSCmdlet.ParameterSetName -eq 'ByPid') { "PID $RootPid" } else { "'$ProcessName'" }
    throw "No process $what found within $WaitSec s."
}

# --- Sample ------------------------------------------------------------------------------------

$writer = $null
if ($Out) {
    $dir = Split-Path -Parent $Out
    if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force $dir | Out-Null }
    $writer = [System.IO.StreamWriter]::new($Out, $false, [System.Text.UTF8Encoding]::new($false))
    $writer.WriteLine('unix_ms,elapsed_s,processes,webview_processes,private_ws_mb,private_bytes_mb,working_set_mb,cpu_s')
}

$samples = [System.Collections.Generic.List[object]]::new()
$start = [DateTime]::UtcNow
$mb = 1MB
try {
    while ($true) {
        $tickStart = [DateTime]::UtcNow
        $tree = @(Get-Tree (Get-ProcessTable) $roots)
        if ($tree.Count -eq 0 -or ($DurationSec -eq 0 -and -not ($tree | Where-Object { $roots -contains $_.IDProcess }))) {
            break  # The root exited.
        }
        $s = [pscustomobject]@{
            unix_ms           = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
            elapsed_s         = ($tickStart - $start).TotalSeconds
            processes         = $tree.Count
            webview_processes = @($tree | Where-Object { $_.Name -like 'msedgewebview2*' }).Count
            private_ws_mb     = ($tree | Measure-Object -Property WorkingSetPrivate -Sum).Sum / $mb
            private_bytes_mb  = ($tree | Measure-Object -Property PrivateBytes -Sum).Sum / $mb
            working_set_mb    = ($tree | Measure-Object -Property WorkingSet -Sum).Sum / $mb
            # PercentProcessorTime raw values are 100 ns ticks of accumulated CPU time.
            cpu_s             = ($tree | Measure-Object -Property PercentProcessorTime -Sum).Sum / 1e7
        }
        $samples.Add($s)
        if ($writer) {
            $writer.WriteLine(([string]::Join(',', @(
                $s.unix_ms.ToString($inv),
                $s.elapsed_s.ToString('F3', $inv),
                $s.processes.ToString($inv),
                $s.webview_processes.ToString($inv),
                $s.private_ws_mb.ToString('F2', $inv),
                $s.private_bytes_mb.ToString('F2', $inv),
                $s.working_set_mb.ToString('F2', $inv),
                $s.cpu_s.ToString('F3', $inv)))))
            $writer.Flush()
        }
        if ($DurationSec -gt 0 -and ([DateTime]::UtcNow - $start).TotalSeconds -ge $DurationSec) { break }
        $spent = ([DateTime]::UtcNow - $tickStart).TotalMilliseconds
        if ($spent -lt $IntervalMs) { Start-Sleep -Milliseconds ([int]($IntervalMs - $spent)) }
    }
} finally {
    if ($writer) { $writer.Dispose() }
}

# --- Summary -----------------------------------------------------------------------------------

if ($samples.Count -eq 0) { throw 'No samples were taken.' }
if (-not $Quiet) {
    $rows = foreach ($metric in 'private_ws_mb', 'private_bytes_mb', 'working_set_mb', 'processes', 'webview_processes') {
        $values = $samples | ForEach-Object { $_.$metric }
        $stats = $values | Measure-Object -Minimum -Maximum
        [pscustomobject]@{
            Metric = $metric
            Min    = [math]::Round($stats.Minimum, 1)
            Max    = [math]::Round($stats.Maximum, 1)
            Last   = [math]::Round($values[-1], 1)
        }
    }
    $last = $samples[$samples.Count - 1]
    $cpu = $last.cpu_s - $samples[0].cpu_s
    Write-Host ("{0} samples over {1:N1} s; CPU time of the tree: {2:N2} s" -f $samples.Count, $last.elapsed_s, $cpu)
    $rows | Format-Table -AutoSize | Out-String | Write-Host
}
