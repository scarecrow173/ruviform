# tools/testsigned-driver/midi-winmm-probe.ps1
# Issue #978 — WinMM MIDI port の bisection probe。wdmaud2.drv は test/probe
# process の in-process コンポーネントなので、driver-side の contract 違反が
# user-mode access violation として現れる。この script は child process
# として起動され、各 WinMM call の直前に step 名を出力する — crash 時は
# 最後に出力された step が faulting call を特定する。
#
# usage: pwsh -NoProfile -NonInteractive -File midi-winmm-probe.ps1
# exit 0: probe 完走。非 0: crash or call 失敗で中断（中断 step が last line）。

$ErrorActionPreference = 'Continue'

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class WinMM {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct MIDIOUTCAPSW {
        public ushort wMid;
        public ushort wPid;
        public uint vDriverVersion;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string szPname;
        public ushort wTechnology;
        public ushort wVoices;
        public ushort wNotes;
        public ushort wChannelMask;
        public uint dwSupport;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct MIDIINCAPSW {
        public ushort wMid;
        public ushort wPid;
        public uint vDriverVersion;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string szPname;
        public uint dwSupport;
    }

    [DllImport("winmm.dll")] public static extern uint midiOutGetNumDevs();
    [DllImport("winmm.dll", CharSet = CharSet.Unicode)] public static extern uint midiOutGetDevCapsW(UIntPtr uDeviceID, out MIDIOUTCAPSW lpCaps, uint cbMidiOutCaps);
    [DllImport("winmm.dll")] public static extern uint midiOutOpen(out IntPtr lphmo, uint uDeviceID, IntPtr dwCallback, IntPtr dwInstance, uint dwFlags);
    [DllImport("winmm.dll")] public static extern uint midiOutClose(IntPtr hmo);
    [DllImport("winmm.dll")] public static extern uint midiOutShortMsg(IntPtr hmo, uint dwMsg);
    [DllImport("winmm.dll")] public static extern uint midiOutMessage(IntPtr hmo, uint uMsg, IntPtr dw1, IntPtr dw2);
    [DllImport("winmm.dll")] public static extern uint midiOutReset(IntPtr hmo);

    [DllImport("winmm.dll")] public static extern uint midiInGetNumDevs();
    [DllImport("winmm.dll", CharSet = CharSet.Unicode)] public static extern uint midiInGetDevCapsW(UIntPtr uDeviceID, out MIDIINCAPSW lpCaps, uint cbMidiInCaps);
    [DllImport("winmm.dll")] public static extern uint midiInOpen(out IntPtr lphmi, uint uDeviceID, IntPtr dwCallback, IntPtr dwInstance, uint dwFlags);
    [DllImport("winmm.dll")] public static extern uint midiInClose(IntPtr hmi);
    [DllImport("winmm.dll")] public static extern uint midiInStart(IntPtr hmi);
    [DllImport("winmm.dll")] public static extern uint midiInStop(IntPtr hmi);
    [DllImport("winmm.dll")] public static extern uint midiInMessage(IntPtr hmi, uint uMsg, IntPtr dw1, IntPtr dw2);

    public const uint DRV_QUERYDEVICEINTERFACESIZE = 1006;
    public const uint DRV_QUERYDEVICEINTERFACE     = 1007;
}
'@

function Step([string]$name) { Write-Output "step: $name" }

$inIfaces  = @{}
$outIfaces = @{}
$loopbackOut = @()
$loopbackIn  = @()

# ---- midiOut enumeration ----
Step 'midiOutGetNumDevs'
$outNum = [WinMM]::midiOutGetNumDevs()
Write-Output "  midiOutGetNumDevs=$outNum"
for ($i = 0; $i -lt $outNum; $i++) {
    Step "midiOutGetDevCapsW[$i]"
    $caps = New-Object WinMM+MIDIOUTCAPSW
    $rc = [WinMM]::midiOutGetDevCapsW([UIntPtr]$i, [ref]$caps, [uint32][System.Runtime.InteropServices.Marshal]::SizeOf($caps))
    Write-Output "  midiOut[$i] rc=$rc name='$($caps.szPname)' tech=0x$($caps.wTechnology.ToString('x4')) support=0x$($caps.dwSupport.ToString('x8'))"

    Step "midiOutMessage DRV_QUERYDEVICEINTERFACE[$i]"
    $sizeBuf = [System.Runtime.InteropServices.Marshal]::AllocHGlobal(8)
    [System.Runtime.InteropServices.Marshal]::WriteInt64($sizeBuf, 0)
    $rc = [WinMM]::midiOutMessage([IntPtr]$i, [WinMM]::DRV_QUERYDEVICEINTERFACESIZE, $sizeBuf, [IntPtr]::Zero)
    $size = [System.Runtime.InteropServices.Marshal]::ReadInt64($sizeBuf)
    [System.Runtime.InteropServices.Marshal]::FreeHGlobal($sizeBuf)
    Write-Output "  ifsize rc=$rc size=$size"
    if ($rc -eq 0 -and $size -gt 0) {
        $buf = New-Object byte[] ($size)
        $h = [System.Runtime.InteropServices.GCHandle]::Alloc($buf, 'Pinned')
        try {
            $rc = [WinMM]::midiOutMessage([IntPtr]$i, [WinMM]::DRV_QUERYDEVICEINTERFACE, $h.AddrOfPinnedObject(), [IntPtr]$size)
            if ($rc -eq 0) {
                $iface = [System.Text.Encoding]::Unicode.GetString($buf).TrimEnd([char]0)
                $outIfaces[$i] = $iface
                Write-Output "  if='$iface'"
            } else {
                Write-Output "  if fetch rc=$rc"
            }
        } finally { $h.Free() }
    }
    if ($caps.szPname -match 'Ruviform MIDI Loopback') { $loopbackOut += $i }
}

# ---- midiIn enumeration ----
Step 'midiInGetNumDevs'
$inNum = [WinMM]::midiInGetNumDevs()
Write-Output "  midiInGetNumDevs=$inNum"
for ($i = 0; $i -lt $inNum; $i++) {
    Step "midiInGetDevCapsW[$i]"
    $caps = New-Object WinMM+MIDIINCAPSW
    $rc = [WinMM]::midiInGetDevCapsW([UIntPtr]$i, [ref]$caps, [uint32][System.Runtime.InteropServices.Marshal]::SizeOf($caps))
    Write-Output "  midiIn[$i] rc=$rc name='$($caps.szPname)' support=0x$($caps.dwSupport.ToString('x8'))"

    Step "midiInMessage DRV_QUERYDEVICEINTERFACE[$i]"
    $sizeBuf = [System.Runtime.InteropServices.Marshal]::AllocHGlobal(8)
    [System.Runtime.InteropServices.Marshal]::WriteInt64($sizeBuf, 0)
    $rc = [WinMM]::midiInMessage([IntPtr]$i, [WinMM]::DRV_QUERYDEVICEINTERFACESIZE, $sizeBuf, [IntPtr]::Zero)
    $size = [System.Runtime.InteropServices.Marshal]::ReadInt64($sizeBuf)
    [System.Runtime.InteropServices.Marshal]::FreeHGlobal($sizeBuf)
    Write-Output "  ifsize rc=$rc size=$size"
    if ($rc -eq 0 -and $size -gt 0) {
        $buf = New-Object byte[] ($size)
        $h = [System.Runtime.InteropServices.GCHandle]::Alloc($buf, 'Pinned')
        try {
            $rc = [WinMM]::midiInMessage([IntPtr]$i, [WinMM]::DRV_QUERYDEVICEINTERFACE, $h.AddrOfPinnedObject(), [IntPtr]$size)
            if ($rc -eq 0) {
                $iface = [System.Text.Encoding]::Unicode.GetString($buf).TrimEnd([char]0)
                $inIfaces[$i] = $iface
                Write-Output "  if='$iface'"
            } else {
                Write-Output "  if fetch rc=$rc"
            }
        } finally { $h.Free() }
    }
    if ($caps.szPname -match 'Ruviform MIDI Loopback') { $loopbackIn += $i }
}

Write-Output "  loopbackOut=[$($loopbackOut -join ',')] loopbackIn=[$($loopbackIn -join ',')]"

# ---- open/send/close on the loopback pair ----
if ($loopbackOut.Count -gt 0) {
    $i = $loopbackOut[0]
    Step "midiOutOpen[$i]"
    $hmo = [IntPtr]::Zero
    $rc = [WinMM]::midiOutOpen([ref]$hmo, $i, [IntPtr]::Zero, [IntPtr]::Zero, 0)
    Write-Output "  midiOutOpen rc=$rc hmo=0x$($hmo.ToString('x'))"
    if ($rc -eq 0) {
        Step "midiOutShortMsg[$i] note-on"
        $rc = [WinMM]::midiOutShortMsg($hmo, 0x007F3C90)   # note-on ch1 C4 vel 0x7f
        Write-Output "  midiOutShortMsg rc=$rc"
        Step "midiOutReset[$i]"
        [void][WinMM]::midiOutReset($hmo)
        Step "midiOutClose[$i]"
        $rc = [WinMM]::midiOutClose($hmo)
        Write-Output "  midiOutClose rc=$rc"
    }
}
if ($loopbackIn.Count -gt 0) {
    $i = $loopbackIn[0]
    Step "midiInOpen[$i] (CALLBACK_NULL)"
    $hmi = [IntPtr]::Zero
    $rc = [WinMM]::midiInOpen([ref]$hmi, $i, [IntPtr]::Zero, [IntPtr]::Zero, 0)
    Write-Output "  midiInOpen rc=$rc hmi=0x$($hmi.ToString('x'))"
    if ($rc -eq 0) {
        Step "midiInStart[$i]"
        $rc = [WinMM]::midiInStart($hmi)
        Write-Output "  midiInStart rc=$rc"
        if ($loopbackOut.Count -gt 0) {
            $oi = $loopbackOut[0]
            Step "loopback send note-on via midiOut[$oi]"
            $hmo = [IntPtr]::Zero
            $rc = [WinMM]::midiOutOpen([ref]$hmo, $oi, [IntPtr]::Zero, [IntPtr]::Zero, 0)
            if ($rc -eq 0) {
                [void][WinMM]::midiOutShortMsg($hmo, 0x007F3C90)
                Start-Sleep -Milliseconds 200
                [void][WinMM]::midiOutShortMsg($hmo, 0x00003C80)   # note-off
                Start-Sleep -Milliseconds 50
                [void][WinMM]::midiOutClose($hmo)
                Write-Output "  loopback send done"
            } else {
                Write-Output "  midiOutOpen for send rc=$rc"
            }
        }
        Step "midiInStop[$i]"
        [void][WinMM]::midiInStop($hmi)
        Step "midiInClose[$i]"
        $rc = [WinMM]::midiInClose($hmi)
        Write-Output "  midiInClose rc=$rc"
    }
}

Write-Output "probe: done"
exit 0
