# Real-OS close driver for manual GUI verification (Issue #431).
#
# Starts an example executable, then posts WM_CLOSE (the same message the
# window close button produces) to the visible titled top-level window of
# that process whose title contains each step substring, in order. Reports
# whether the process is still alive and whether the targeted window is still
# open after each step, then the app's exit code and its stdout/stderr.
#
# Example (demo_multi: closing the panel must keep the loop alive, closing
# the last window must exit 0):
#   powershell -NoProfile -ExecutionPolicy Bypass -File wm-close.ps1 `
#     -Exe target\debug\examples\demo_multi.exe -Steps "panel,main"
#
# Windows only. Not a CI lane. ASCII-only so Windows PowerShell 5.1 reads it
# without a BOM.

param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [string]$ArgList = '',
    [string]$Steps = '',
    [int]$StartWaitMs = 1500,
    [int]$StepWaitMs = 2500,
    [int]$ReadyTimeoutSec = 30
)

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class WmClose {
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr lParam);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowTextLengthW(IntPtr h);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool PostMessageW(IntPtr h, uint msg, IntPtr w, IntPtr l);
    public static List<KeyValuePair<IntPtr, string>> VisibleTitled(uint pid) {
        var list = new List<KeyValuePair<IntPtr, string>>();
        EnumWindows((h, l) => {
            uint p;
            GetWindowThreadProcessId(h, out p);
            if (p != pid || !IsWindowVisible(h)) return true;
            int n = GetWindowTextLengthW(h);
            if (n <= 0) return true;
            var sb = new StringBuilder(n + 1);
            GetWindowTextW(h, sb, sb.Capacity);
            list.Add(new KeyValuePair<IntPtr, string>(h, sb.ToString()));
            return true;
        }, IntPtr.Zero);
        return list;
    }
}
"@

$WM_CLOSE = 0x0010
$runId = [guid]::NewGuid().ToString('N')
$outLog = Join-Path $env:TEMP "wm-close-$runId.out.log"
$errLog = Join-Path $env:TEMP "wm-close-$runId.err.log"

# Output goes to files via cmd so the exit code is the app's and no pipe is
# left open for WebView2 helper processes that inherit the handles.
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = 'cmd.exe'
$psi.Arguments = "/d /c `"`"$Exe`" $ArgList > `"$outLog`" 2> `"$errLog`"`""
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true
$shell = [System.Diagnostics.Process]::Start($psi)

# The app is the child of cmd; its PID is what owns the windows.
$exeName = [IO.Path]::GetFileName($Exe)
$app = $null
$deadline = (Get-Date).AddSeconds($ReadyTimeoutSec)
while (-not $app -and (Get-Date) -lt $deadline) {
    $app = Get-CimInstance Win32_Process -Filter "ParentProcessId=$($shell.Id)" |
        Where-Object { $_.Name -eq $exeName } | Select-Object -First 1
    if (-not $app) { Start-Sleep -Milliseconds 200 }
}
if (-not $app) {
    Write-Output "[wm-close] app process not found under cmd pid=$($shell.Id)"
    exit 1
}
$appPid = [uint32]$app.ProcessId
function Test-Alive { [bool](Get-Process -Id $appPid -ErrorAction SilentlyContinue) }
Start-Sleep -Milliseconds $StartWaitMs
Write-Output "[wm-close] started app pid=$appPid exe=$Exe"

foreach ($step in ($Steps -split ',' | Where-Object { $_ })) {
    $deadline = (Get-Date).AddSeconds($ReadyTimeoutSec)
    $target = $null
    while (-not $target -and (Get-Date) -lt $deadline -and (Test-Alive)) {
        $target = [WmClose]::VisibleTitled($appPid) |
            Where-Object { $_.Value -like "*$step*" } | Select-Object -First 1
        if (-not $target) { Start-Sleep -Milliseconds 300 }
    }
    if (-not $target) {
        Write-Output "[wm-close] no visible window matching '$step' (alive=$(Test-Alive))"
        break
    }
    if ([WmClose]::PostMessageW($target.Key, $WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)) {
        Write-Output "[wm-close] WM_CLOSE -> '$($target.Value)'"
    } else {
        Write-Output "[wm-close] PostMessage failed for '$($target.Value)'"
    }
    Start-Sleep -Milliseconds $StepWaitMs
    $stillOpen = @([WmClose]::VisibleTitled($appPid) | Where-Object { $_.Value -like "*$step*" }).Count -gt 0
    Write-Output "[wm-close] after '$step': process alive=$(Test-Alive) window still open=$stillOpen"
}

if ($shell.WaitForExit(10000)) {
    Write-Output "[wm-close] exited code=$($shell.ExitCode)"
} else {
    Stop-Process -Id $appPid -Force -ErrorAction SilentlyContinue
    Write-Output "[wm-close] did not exit within 10s; force-stopped"
}
foreach ($log in @(@('stderr', $errLog), @('stdout', $outLog))) {
    Write-Output "[wm-close] --- child $($log[0]) ($($log[1])) ---"
    if (Test-Path $log[1]) { Get-Content $log[1] | Where-Object { $_ } | ForEach-Object { Write-Output "  | $_" } }
}
