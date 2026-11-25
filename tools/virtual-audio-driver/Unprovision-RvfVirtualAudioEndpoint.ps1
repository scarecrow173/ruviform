# Unprovision-RvfVirtualAudioEndpoint.ps1
# sv#115 Slice A - Ruviform virtual audio endpoint unprovision/cleanup primitive.
# machine-readable contract is rvf-virtual-audio-provisioning/v1 state JSON.

[CmdletBinding()]
param(
    [string]$StatePath,
    [string]$WorkspaceRoot,
    [string]$EvidenceDir,
    [string]$CleanupMode = 'Full',
    [int]$TimeoutSeconds = 900
)

$ErrorActionPreference = 'Continue'

try {
    Import-Module (Join-Path $PSScriptRoot 'RvfVirtualAudioProvisioning.psm1') -Force -ErrorAction Stop
} catch {
    Write-Error "provisioning module import failed: $($_.Exception.Message)"
    exit 2
}

$missing = @()
foreach ($pair in @(
    @{ name = 'StatePath'; value = $StatePath },
    @{ name = 'EvidenceDir'; value = $EvidenceDir }
)) {
    if ([string]::IsNullOrWhiteSpace($pair.value)) { $missing += $pair.name }
}
if ($missing.Count) {
    Write-Error "mandatory input missing: $($missing -join ', ')"
    exit 2
}
if ($CleanupMode -notin @('Full', 'ReportOnly')) {
    Write-Error "unsupported CleanupMode '$CleanupMode' (supported: Full, ReportOnly)"
    exit 2
}
if ($TimeoutSeconds -lt 1) {
    Write-Error 'TimeoutSeconds must be positive'
    exit 2
}

try {
    $invokeArgs = @{
        StatePath = $StatePath
        EvidenceDir = $EvidenceDir
        CleanupMode = $CleanupMode
        TimeoutSeconds = $TimeoutSeconds
    }
    if (-not [string]::IsNullOrWhiteSpace($WorkspaceRoot)) { $invokeArgs.WorkspaceRoot = $WorkspaceRoot }
    $result = @(Invoke-RvfUnprovision @invokeArgs)
    $code = @($result | Where-Object { $_ -is [int] } | Select-Object -Last 1)
    if (-not $code.Count) {
        Write-Error 'unprovision primitive did not return an exit code'
        exit 2
    }
    exit [int]$code[0]
} catch {
    Write-Error "unprovision primitive contract failure: $($_.Exception.Message)"
    exit 2
}
