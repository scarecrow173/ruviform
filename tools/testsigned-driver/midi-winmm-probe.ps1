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
    [DllImport("winmm.dll")] public static extern uint midiInReset(IntPtr hmi);
    [DllImport("winmm.dll")] public static extern uint midiInMessage(IntPtr hmi, uint uMsg, IntPtr dw1, IntPtr dw2);
    [DllImport("winmm.dll")] public static extern uint midiInPrepareHeader(IntPtr hmi, IntPtr lpMidiInHdr, uint cbMidiInHdr);
    [DllImport("winmm.dll")] public static extern uint midiInUnprepareHeader(IntPtr hmi, IntPtr lpMidiInHdr, uint cbMidiInHdr);
    [DllImport("winmm.dll")] public static extern uint midiInAddBuffer(IntPtr hmi, IntPtr lpMidiInHdr, uint cbMidiInHdr);

    // tests/loopback.rs と同じ callback 経路を踏むための CALLBACK_FUNCTION
    // delegate — test は CALLBACK_FUNCTION|MIDI_IO_STATUS で開き SysEx
    // buffer を prepare+add して midiInStart する。
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    public delegate void MidiInProc(IntPtr hmi, uint wMsg, UIntPtr dwInstance, UIntPtr dwParam1, UIntPtr dwParam2);
    public static MidiInProc InCallback = new MidiInProc(StaticInCallback);
    public static volatile int InMsgCount = 0;
    public static volatile uint InLastMsg = 0;
    public static volatile UIntPtr InLastParam1 = UIntPtr.Zero;
    public static volatile UIntPtr InLastParam2 = UIntPtr.Zero;
    static void StaticInCallback(IntPtr hmi, uint wMsg, UIntPtr dwInstance, UIntPtr dwParam1, UIntPtr dwParam2) {
        InMsgCount++;
        InLastMsg = wMsg;
        InLastParam1 = dwParam1;
        InLastParam2 = dwParam2;
    }

    // windows-sys Multimedia/mod.rs: DRV_MCI_FIRST+12/+13。
    // 1006/1007 は誤値で attempt-2/3 の probe は別 message を叩いていた。
    public const uint DRV_QUERYDEVICEINTERFACESIZE = 2061;
    public const uint DRV_QUERYDEVICEINTERFACE     = 2060;
    public const uint CALLBACK_FUNCTION            = 0x00030000;
    public const uint CALLBACK_NULL                = 0x00000000;
    public const uint MIDI_IO_STATUS               = 0x00000020;
    public const uint MIDIHDR_SIZE                 = 120;  // x64 sizeof(MIDIHDR)
    public const uint MHDR_DONE                    = 0x00000001;
    public const uint MM_MIM_OPEN                  = 0x3C1;
    public const uint MM_MIM_CLOSE                 = 0x3C2;
    public const uint MM_MIM_DATA                  = 0x3C3;
    public const uint MM_MIM_LONGDATA              = 0x3C4;
    public const uint MM_MIM_ERROR                 = 0x3C5;
    public const uint MM_MIM_MOREDATA              = 0x3CC;
    public const uint MM_MIM_LONGERROR             = 0x3CD;
}
'@

function Step([string]$name) { Write-Output "step: $name" }

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
    # test と同じ flags: CALLBACK_FUNCTION | MIDI_IO_STATUS + SysEx
    # buffer prepare/add — callback が MIM_DATA を受け取れば driver の
    # render→capture loopback 経路が user-mode 観測として機能している。
    Step "midiInOpen[$i] (CALLBACK_FUNCTION|MIDI_IO_STATUS)"
    $hmi = [IntPtr]::Zero
    $cbPtr = [System.Runtime.InteropServices.Marshal]::GetFunctionPointerForDelegate([WinMM]::InCallback)
    $rc = [WinMM]::midiInOpen([ref]$hmi, $i, $cbPtr, [IntPtr]::Zero, ([WinMM]::CALLBACK_FUNCTION -bor [WinMM]::MIDI_IO_STATUS))
    Write-Output "  midiInOpen rc=$rc hmi=0x$($hmi.ToString('x'))"
    if ($rc -eq 0) {
        # open handle での DRV_QUERYDEVICEINTERFACE* — device-index 版と
        # 対比し open-handle 経路が正しい workaround かを検証する。
        Step "midiInMessage DRV_QUERYDEVICEINTERFACE (open handle)"
        $sizeBuf = [System.Runtime.InteropServices.Marshal]::AllocHGlobal(8)
        [System.Runtime.InteropServices.Marshal]::WriteInt64($sizeBuf, 0)
        $rc = [WinMM]::midiInMessage($hmi, [WinMM]::DRV_QUERYDEVICEINTERFACESIZE, $sizeBuf, [IntPtr]::Zero)
        $size = [System.Runtime.InteropServices.Marshal]::ReadInt64($sizeBuf)
        [System.Runtime.InteropServices.Marshal]::FreeHGlobal($sizeBuf)
        Write-Output "  open-handle ifsize rc=$rc size=$size"
        if ($rc -eq 0 -and $size -gt 0) {
            $buf = [System.Runtime.InteropServices.Marshal]::AllocHGlobal($size)
            $rc = [WinMM]::midiInMessage($hmi, [WinMM]::DRV_QUERYDEVICEINTERFACE, $buf, [IntPtr]$size)
            $iface = if ($rc -eq 0) { [System.Runtime.InteropServices.Marshal]::PtrToStringUni($buf) } else { $null }
            Write-Output "  open-handle if rc=$rc if='$iface'"
            [System.Runtime.InteropServices.Marshal]::FreeHGlobal($buf)
        }

        # SysEx buffers x2 (test は 4 だが crash bisect には十分)
        $hdrs = @()
        for ($b = 0; $b -lt 2; $b++) {
            Step "midiInPrepareHeader+AddBuffer[$b]"
            $hdrMem = [System.Runtime.InteropServices.Marshal]::AllocHGlobal([int][WinMM]::MIDIHDR_SIZE)
            $dataMem = [System.Runtime.InteropServices.Marshal]::AllocHGlobal(1024)
            $zero = New-Object byte[] ([int][WinMM]::MIDIHDR_SIZE)
            [System.Runtime.InteropServices.Marshal]::Copy($zero, 0, $hdrMem, [int][WinMM]::MIDIHDR_SIZE)
            [System.Runtime.InteropServices.Marshal]::WriteInt64($hdrMem, 0, $dataMem.ToInt64())  # lpData
            [System.Runtime.InteropServices.Marshal]::WriteInt32($hdrMem, 8, 1024)               # dwBufferLength
            $rc = [WinMM]::midiInPrepareHeader($hmi, $hdrMem, [WinMM]::MIDIHDR_SIZE)
            Write-Output "  prepare rc=$rc"
            $rc = [WinMM]::midiInAddBuffer($hmi, $hdrMem, [WinMM]::MIDIHDR_SIZE)
            Write-Output "  addbuf rc=$rc"
            $hdrs += ,@($hdrMem, $dataMem)
        }
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
                Start-Sleep -Milliseconds 300
                [void][WinMM]::midiOutShortMsg($hmo, 0x00003C80)   # note-off
                Start-Sleep -Milliseconds 200
                [void][WinMM]::midiOutClose($hmo)
                Write-Output "  loopback send done"
            } else {
                Write-Output "  midiOutOpen for send rc=$rc"
            }
        }
        "  callback count=$([WinMM]::InMsgCount) lastMsg=0x$([WinMM]::InLastMsg.ToString('x')) lastP1=0x$([WinMM]::InLastParam1.ToUInt64().ToString('x')) lastP2=0x$([WinMM]::InLastParam2.ToUInt64().ToString('x'))" | Write-Output
        Step "midiInStop[$i]"
        [void][WinMM]::midiInStop($hmi)
        Step "midiInReset[$i]"
        [void][WinMM]::midiInReset($hmi)
        foreach ($pair in $hdrs) {
            Step "midiInUnprepareHeader"
            [void][WinMM]::midiInUnprepareHeader($hmi, $pair[0], [WinMM]::MIDIHDR_SIZE)
            [System.Runtime.InteropServices.Marshal]::FreeHGlobal($pair[0])
            [System.Runtime.InteropServices.Marshal]::FreeHGlobal($pair[1])
        }
        Step "midiInClose[$i]"
        $rc = [WinMM]::midiInClose($hmi)
        Write-Output "  midiInClose rc=$rc"
        "  callback count(final)=$([WinMM]::InMsgCount) lastMsg=0x$([WinMM]::InLastMsg.ToString('x'))" | Write-Output
    }
}

# ---- device-index-as-handle DRV_QUERYDEVICEINTERFACE — expected-crash repro ----
# cdb 解析で確定: tests/loopback.rs の interface_id() が
# midiInMessage(index-as-handle, DRV_QUERYDEVICEINTERFACESIZE) を叩くと
# winmmbase!mregHandleInternalMessages → StringCchCopyW(→0x90) で
# AV する（Server 2025 winmmbase の latent NULL-deref — real WDM device
# のみ発火、GS のような softsynth は interface 不在で早期 return）。
# この section が最後 — crash 時はこの step 行が log の末尾になる。
if ($loopbackIn.Count -gt 0) {
    $i = $loopbackIn[0]
    Step "midiInMessage DRV_QUERYDEVICEINTERFACE[device-index-as-handle $i] (expected crash)"
    $sizeBuf = [System.Runtime.InteropServices.Marshal]::AllocHGlobal(8)
    [System.Runtime.InteropServices.Marshal]::WriteInt64($sizeBuf, 0)
    $rc = [WinMM]::midiInMessage([IntPtr]$i, [WinMM]::DRV_QUERYDEVICEINTERFACESIZE, $sizeBuf, [IntPtr]::Zero)
    $size = [System.Runtime.InteropServices.Marshal]::ReadInt64($sizeBuf)
    Write-Output "  dev-index ifsize rc=$rc size=$size"
    if ($rc -eq 0 -and $size -gt 0) {
        $buf = [System.Runtime.InteropServices.Marshal]::AllocHGlobal($size)
        $rc = [WinMM]::midiInMessage([IntPtr]$i, [WinMM]::DRV_QUERYDEVICEINTERFACE, $buf, [IntPtr]$size)
        $iface = if ($rc -eq 0) { [System.Runtime.InteropServices.Marshal]::PtrToStringUni($buf) } else { $null }
        Write-Output "  dev-index if rc=$rc if='$iface'"
        [System.Runtime.InteropServices.Marshal]::FreeHGlobal($buf)
    }
    [System.Runtime.InteropServices.Marshal]::FreeHGlobal($sizeBuf)
}

Write-Output "probe: done"
exit 0
