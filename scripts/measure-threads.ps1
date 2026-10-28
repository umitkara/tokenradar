<#
.SYNOPSIS
Attributes a process's idle CPU and wakeups to its threads.

.DESCRIPTION
Takes two snapshots of every thread of the process, DurationSec apart, and prints per thread:
  - switches  context switches in the window, i.e. how often the thread woke up
  - cycles_m  CPU cycles in millions (QueryThreadCycleTime: exact, not sampled at the 15.6 ms
              clock tick like Process.TotalProcessorTime)
  - cpu_ms    kernel + user time in milliseconds (tick-sampled, for comparison with
              measure-memory.ps1)
Thread names come from GetThreadDescription; Rust's std, tokio and Tauri name their threads.
Threads that start or exit inside the window are listed with "new" or "exited".

Only the root process is covered, not WebView2 children: the idle budget applies with no window
open, when the app has no children.

.PARAMETER ProcessName
Process name without ".exe". Default "tokenradar-app".

.PARAMETER DurationSec
Length of the window. Default 600 (the idle CPU budget is a 10-minute average).

.PARAMETER Out
Optional CSV path for the per-thread rows.

.EXAMPLE
.\scripts\measure-threads.ps1 -DurationSec 600 -Out "$env:TEMP\tr-threads.csv"
#>
param(
    [string] $ProcessName = 'tokenradar-app',
    [int] $DurationSec = 600,
    [string] $Out
)

$ErrorActionPreference = 'Stop'

Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class ThreadProbe {
    public struct Row { public uint Tid; public ulong Switches; public ulong Cycles; public long CpuTicks; public string Name; }

    [DllImport("ntdll.dll")] static extern int NtQuerySystemInformation(int cls, IntPtr buf, int len, out int ret);
    [DllImport("kernel32.dll")] static extern IntPtr OpenThread(uint access, bool inherit, uint tid);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
    [DllImport("kernel32.dll")] static extern bool QueryThreadCycleTime(IntPtr h, out ulong cycles);
    [DllImport("kernel32.dll")] static extern int GetThreadDescription(IntPtr h, out IntPtr desc);
    [DllImport("kernel32.dll")] static extern IntPtr LocalFree(IntPtr p);

    const int SystemProcessInformation = 5;
    const uint THREAD_QUERY_LIMITED_INFORMATION = 0x0800;
    // x64 layouts: SYSTEM_PROCESS_INFORMATION is 256 bytes, SYSTEM_THREAD_INFORMATION 80.
    const int ProcThreadCount = 4, ProcPid = 80, ProcSize = 256;
    const int ThrKernel = 0, ThrUser = 8, ThrTid = 48, ThrSwitches = 64, ThrSize = 80;

    public static List<Row> Snapshot(int pid) {
        int len = 1 << 20;
        IntPtr buf = IntPtr.Zero;
        try {
            while (true) {
                buf = Marshal.AllocHGlobal(len);
                int ret;
                int status = NtQuerySystemInformation(SystemProcessInformation, buf, len, out ret);
                if (status == 0) break;
                Marshal.FreeHGlobal(buf); buf = IntPtr.Zero;
                if (status != unchecked((int)0xC0000004)) throw new Exception("NtQuerySystemInformation 0x" + status.ToString("X8"));
                len = Math.Max(len * 2, ret + 65536);
            }
            var rows = new List<Row>();
            long offset = 0;
            while (true) {
                IntPtr p = new IntPtr(buf.ToInt64() + offset);
                if (Marshal.ReadIntPtr(p, ProcPid).ToInt64() == pid) {
                    int n = Marshal.ReadInt32(p, ProcThreadCount);
                    for (int i = 0; i < n; i++) {
                        IntPtr t = new IntPtr(p.ToInt64() + ProcSize + (long)i * ThrSize);
                        var row = new Row {
                            Tid = (uint)Marshal.ReadIntPtr(t, ThrTid).ToInt64(),
                            Switches = (uint)Marshal.ReadInt32(t, ThrSwitches),
                            CpuTicks = Marshal.ReadInt64(t, ThrKernel) + Marshal.ReadInt64(t, ThrUser),
                            Name = ""
                        };
                        IntPtr h = OpenThread(THREAD_QUERY_LIMITED_INFORMATION, false, row.Tid);
                        if (h != IntPtr.Zero) {
                            ulong c; if (QueryThreadCycleTime(h, out c)) row.Cycles = c;
                            IntPtr d; if (GetThreadDescription(h, out d) >= 0 && d != IntPtr.Zero) { row.Name = Marshal.PtrToStringUni(d) ?? ""; LocalFree(d); }
                            CloseHandle(h);
                        }
                        rows.Add(row);
                    }
                    return rows;
                }
                int next = Marshal.ReadInt32(p);
                if (next == 0) throw new Exception("process " + pid + " not found");
                offset += next;
            }
        } finally {
            if (buf != IntPtr.Zero) Marshal.FreeHGlobal(buf);
        }
    }
}
'@

$process = @(Get-Process $ProcessName)
if ($process.Count -ne 1) { throw "expected one $ProcessName process, found $($process.Count)" }
$id = $process[0].Id

$before = @{}
foreach ($r in [ThreadProbe]::Snapshot($id)) { $before[$r.Tid] = $r }
Write-Host "pid $id, $($before.Count) threads; sampling $DurationSec s ..."
Start-Sleep -Seconds $DurationSec
$after = [ThreadProbe]::Snapshot($id)

$rows = foreach ($a in $after) {
    $b = $before[$a.Tid]
    $state = if ($b) { '' } else { 'new' }
    if (-not $b) { $b = [ThreadProbe+Row]::new() }
    [pscustomobject]@{
        tid      = $a.Tid
        name     = $a.Name
        switches = [int64]$a.Switches - [int64]$b.Switches
        cycles_m = [math]::Round(([double]$a.Cycles - [double]$b.Cycles) / 1e6, 2)
        cpu_ms   = [math]::Round(($a.CpuTicks - $b.CpuTicks) / 1e4, 1)
        state    = $state
    }
}
$alive = @{}; foreach ($a in $after) { $alive[$a.Tid] = $true }
$rows = @($rows) + @($before.Values | Where-Object { -not $alive[$_.Tid] } | ForEach-Object {
        [pscustomobject]@{ tid = $_.Tid; name = $_.Name; switches = $null; cycles_m = $null; cpu_ms = $null; state = 'exited' }
    })

$rows = $rows | Sort-Object -Property @{ Expression = 'switches'; Descending = $true }, cycles_m -Descending
$rows | Format-Table -AutoSize | Out-String -Width 200 | Write-Host
$sum = ($rows | Measure-Object switches -Sum).Sum
$cyc = ($rows | Measure-Object cycles_m -Sum).Sum
$cpu = ($rows | Measure-Object cpu_ms -Sum).Sum
$inv = [Globalization.CultureInfo]::InvariantCulture
# Cycles at the nominal clock; turbo makes the real share a little smaller.
$mhz = (Get-CimInstance Win32_Processor | Select-Object -First 1).MaxClockSpeed
Write-Host ("total: {0} switches ({1}/min), {2} M cycles (~{3} % of one {4} MHz core), {5} ms tick-sampled CPU" -f $sum,
    ($sum / ($DurationSec / 60)).ToString('0.0', $inv), $cyc.ToString('0.0', $inv),
    ($cyc / $mhz / $DurationSec * 100).ToString('0.000', $inv), $mhz, $cpu.ToString('0', $inv))
if ($Out) {
    # Invariant culture, so the CSV parses the same on every locale (like measure-memory.ps1).
    $rows | ForEach-Object {
        [pscustomobject]@{
            tid = $_.tid; name = $_.name; switches = $_.switches
            cycles_m = if ($null -ne $_.cycles_m) { ([double]$_.cycles_m).ToString($inv) }
            cpu_ms = if ($null -ne $_.cpu_ms) { ([double]$_.cpu_ms).ToString($inv) }
            state = $_.state
        }
    } | Export-Csv -NoTypeInformation -Encoding utf8 $Out
    Write-Host "rows written to $Out"
}
