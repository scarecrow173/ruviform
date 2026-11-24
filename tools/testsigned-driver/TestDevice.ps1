# tools/testsigned-driver/TestDevice.ps1
# Issue #961 — devcon 非依存の root-enumerated devnode 作成・driver 結合・
# 削除ラッパ。GitHub-hosted runner に devcon が absent（evidence/rf952/
# probe-env-*.txt 実測）のため、`devcon install <inf> ROOT\<HwId>` 相当を
# setupapi/newdev の P/Invoke で実装する。
#
#   -Action create:
#     SetupDiCreateDeviceInfoList → SetupDiCreateDeviceInfo(DICD_GENERATE_ID)
#     → SetupDiSetDeviceRegistryProperty(SPDRP_HARDWAREID)
#     → SetupDiCallClassInstaller(DIF_REGISTERDEVICE) で devnode を作成し、
#     -InfPath 指定時は newdev!UpdateDriverForPlugAndPlayDevices(INSTALLFLAG_
#     FORCE) で driver を結合する。出力: `instance_id=...` / `bind_ok=...` /
#     `reboot_required=...` 行（workflow が解析して stage 記録する）。
#   -Action remove:
#     SetupDiGetClassDevs(DIGCF_ALLCLASSES, Enumerator="ROOT") を走査して
#     -InstanceId に一致する devinfo を特定し、newdev!DiUninstallDevice で
#     devnode を削除する。見つからない場合は既に削除済みとして正常終了。
#
# 署名・証明書 store 変更・driver install 自体はこの script の責務外
# （workflow の runner job 内でのみ実行。worker ホストでは構文確認まで）。

[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('create', 'remove')][string]$Action,
    [string]$HardwareId = 'ROOT\RvfTestSigned',
    [string]$ClassName = 'System',
    [string]$ClassGuid = '4d36e97d-e325-11ce-bfc1-08002be10318',
    [string]$DeviceDescription = 'ruviform test-signed null device',
    [string]$InfPath,
    [string]$InstanceId
)

$ErrorActionPreference = 'Stop'

$src = @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

public static class RvfTestDevice
{
    const uint DICD_GENERATE_ID = 0x00000001;
    const uint DIF_REGISTERDEVICE = 0x00000019;
    const uint DIGCF_ALLCLASSES = 0x00000004;
    const uint SPDRP_HARDWAREID = 0x00000001;
    const uint INSTALLFLAG_FORCE = 0x00000001;
    const int ERROR_NO_MORE_ITEMS = 259;
    const int LINE_LEN = 256;

    [StructLayout(LayoutKind.Sequential)]
    public struct SP_DEVINFO_DATA
    {
        public int cbSize;
        public Guid ClassGuid;
        public IntPtr DevInst;
        public IntPtr Reserved;
    }

    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern IntPtr SetupDiCreateDeviceInfoList(ref Guid ClassGuid, IntPtr hwndParent);

    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool SetupDiCreateDeviceInfoW(
        IntPtr DeviceInfoSet, string ClassName, ref Guid ClassGuid,
        string DeviceDescription, IntPtr hwndParent, uint CreationFlags,
        ref SP_DEVINFO_DATA DeviceInfoData);

    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool SetupDiSetDeviceRegistryPropertyW(
        IntPtr DeviceInfoSet, ref SP_DEVINFO_DATA DeviceInfoData,
        uint Property, byte[] PropertyBuffer, uint PropertyBufferSize);

    [DllImport("setupapi.dll", SetLastError = true)]
    static extern bool SetupDiCallClassInstaller(
        uint InstallFunction, IntPtr DeviceInfoSet, ref SP_DEVINFO_DATA DeviceInfoData);

    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool SetupDiGetDeviceInstanceIdW(
        IntPtr DeviceInfoSet, ref SP_DEVINFO_DATA DeviceInfoData,
        StringBuilder DeviceInstanceId, uint DeviceInstanceIdSize,
        out uint RequiredSize);

    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern IntPtr SetupDiGetClassDevsW(
        IntPtr ClassGuid, string Enumerator, IntPtr hwndParent, uint Flags);

    [DllImport("setupapi.dll", SetLastError = true)]
    static extern bool SetupDiEnumDeviceInfo(
        IntPtr DeviceInfoSet, uint MemberIndex, ref SP_DEVINFO_DATA DeviceInfoData);

    [DllImport("setupapi.dll", SetLastError = true)]
    static extern bool SetupDiDestroyDeviceInfoList(IntPtr DeviceInfoSet);

    [DllImport("newdev.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool UpdateDriverForPlugAndPlayDevicesW(
        IntPtr hwndParent, string HardwareId, string FullInfPath,
        uint InstallFlags, out bool bRebootRequired);

    [DllImport("newdev.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool DiUninstallDevice(
        IntPtr hwndParent, IntPtr DeviceInfoSet,
        ref SP_DEVINFO_DATA DeviceInfoData, uint Flags, out bool NeedReboot);

    static readonly IntPtr InvalidHandle = new IntPtr(-1);

    static void Check(bool ok, string api)
    {
        if (!ok) {
            throw new Win32Exception(Marshal.GetLastWin32Error(), api + " failed");
        }
    }

    static IntPtr CreateInfoList(Guid classGuid)
    {
        IntPtr set = SetupDiCreateDeviceInfoList(ref classGuid, IntPtr.Zero);
        if (set == InvalidHandle || set == IntPtr.Zero) {
            throw new Win32Exception(Marshal.GetLastWin32Error(),
                "SetupDiCreateDeviceInfoList failed");
        }
        return set;
    }

    static byte[] MultiSz(string s)
    {
        return Encoding.Unicode.GetBytes(s + "\0\0");
    }

    static string InstanceIdOf(IntPtr set, ref SP_DEVINFO_DATA data)
    {
        StringBuilder sb = new StringBuilder(LINE_LEN);
        uint required;
        Check(SetupDiGetDeviceInstanceIdW(set, ref data, sb, (uint)sb.Capacity, out required),
            "SetupDiGetDeviceInstanceId");
        return sb.ToString();
    }

    public static string Create(string className, Guid classGuid,
        string hardwareId, string deviceDescription)
    {
        IntPtr set = CreateInfoList(classGuid);
        try {
            SP_DEVINFO_DATA data = new SP_DEVINFO_DATA();
            data.cbSize = Marshal.SizeOf(typeof(SP_DEVINFO_DATA));
            Check(SetupDiCreateDeviceInfoW(set, className, ref classGuid,
                    deviceDescription, IntPtr.Zero, DICD_GENERATE_ID, ref data),
                "SetupDiCreateDeviceInfo");
            byte[] hwid = MultiSz(hardwareId);
            Check(SetupDiSetDeviceRegistryPropertyW(set, ref data,
                    SPDRP_HARDWAREID, hwid, (uint)hwid.Length),
                "SetupDiSetDeviceRegistryProperty");
            Check(SetupDiCallClassInstaller(DIF_REGISTERDEVICE, set, ref data),
                "SetupDiCallClassInstaller(DIF_REGISTERDEVICE)");
            return InstanceIdOf(set, ref data);
        } finally {
            SetupDiDestroyDeviceInfoList(set);
        }
    }

    public static bool Bind(string hardwareId, string infPath, out bool rebootRequired)
    {
        bool ok = UpdateDriverForPlugAndPlayDevicesW(IntPtr.Zero, hardwareId,
            infPath, INSTALLFLAG_FORCE, out rebootRequired);
        if (!ok) {
            throw new Win32Exception(Marshal.GetLastWin32Error(),
                "UpdateDriverForPlugAndPlayDevices failed");
        }
        return ok;
    }

    public static bool Remove(string instanceId)
    {
        IntPtr set = SetupDiGetClassDevsW(IntPtr.Zero, "ROOT", IntPtr.Zero, DIGCF_ALLCLASSES);
        if (set == InvalidHandle || set == IntPtr.Zero) {
            throw new Win32Exception(Marshal.GetLastWin32Error(),
                "SetupDiGetClassDevs failed");
        }
        try {
            for (uint i = 0; ; i++) {
                SP_DEVINFO_DATA data = new SP_DEVINFO_DATA();
                data.cbSize = Marshal.SizeOf(typeof(SP_DEVINFO_DATA));
                if (!SetupDiEnumDeviceInfo(set, i, ref data)) {
                    int err = Marshal.GetLastWin32Error();
                    if (err == ERROR_NO_MORE_ITEMS) {
                        return false;
                    }
                    throw new Win32Exception(err, "SetupDiEnumDeviceInfo failed");
                }
                string id;
                try {
                    id = InstanceIdOf(set, ref data);
                } catch {
                    continue;
                }
                if (!string.Equals(id, instanceId, StringComparison.OrdinalIgnoreCase)) {
                    continue;
                }
                bool needReboot;
                Check(DiUninstallDevice(IntPtr.Zero, set, ref data, 0, out needReboot),
                    "DiUninstallDevice");
                return true;
            }
        } finally {
            SetupDiDestroyDeviceInfoList(set);
        }
    }
}
'@

Add-Type -TypeDefinition $src -ErrorAction Stop

$guid = [Guid]$ClassGuid

if ($Action -eq 'create') {
    $id = [RvfTestDevice]::Create($ClassName, $guid, $HardwareId, $DeviceDescription)
    "instance_id=$id"
    if ($InfPath) {
        $resolved = (Resolve-Path $InfPath).ProviderPath
        $reboot = $false
        $ok = [RvfTestDevice]::Bind($HardwareId, $resolved, [ref]$reboot)
        "bind_ok=$ok"
        "reboot_required=$reboot"
        "bound_inf=$resolved"
    }
} else {
    if (-not $InstanceId) {
        throw 'remove requires -InstanceId'
    }
    $removed = [RvfTestDevice]::Remove($InstanceId)
    "removed=$removed"
}
