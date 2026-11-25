# Provision-RvfVirtualAudioEndpoint.ps1
# sv#115 Slice A - Ruviform virtual audio endpoint provisioning primitive.
# machine-readable contract is rvf-virtual-audio-provisioning/v1 state JSON.

[CmdletBinding()]
param(
    [string]$SourceRoot,
    [string]$ExpectedSourceRev,
    [string]$WorkspaceRoot,
    [string]$StatePath,
    [string]$EvidenceDir,
    [string]$Arch = 'x64',
    [string]$Configuration = 'Release',
    [switch]$AllowWdkInstall,
    [switch]$AllowMicConsentProvisioning,
    [int]$TimeoutSeconds = 3600,
    [string]$CleanupOnFailure = 'Full'
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
    @{ name = 'SourceRoot'; value = $SourceRoot },
    @{ name = 'WorkspaceRoot'; value = $WorkspaceRoot },
    @{ name = 'StatePath'; value = $StatePath },
    @{ name = 'EvidenceDir'; value = $EvidenceDir }
)) {
    if ([string]::IsNullOrWhiteSpace($pair.value)) { $missing += $pair.name }
}
if ($missing.Count) {
    Write-Error "mandatory input missing: $($missing -join ', ')"
    exit 2
}
if ($Configuration -notin @('Release', 'Debug')) {
    Write-Error "unsupported Configuration '$Configuration' (supported: Release, Debug)"
    exit 2
}
if ($CleanupOnFailure -notin @('Full', 'Preserve')) {
    Write-Error "unsupported CleanupOnFailure '$CleanupOnFailure' (supported: Full, Preserve)"
    exit 2
}
if ($TimeoutSeconds -lt 1) {
    Write-Error 'TimeoutSeconds must be positive'
    exit 2
}

try {
    $result = @(Invoke-RvfProvision `
        -SourceRoot $SourceRoot `
        -ExpectedSourceRev $ExpectedSourceRev `
        -WorkspaceRoot $WorkspaceRoot `
        -StatePath $StatePath `
        -EvidenceDir $EvidenceDir `
        -Arch $Arch `
        -Configuration $Configuration `
        -AllowWdkInstall:$AllowWdkInstall `
        -AllowMicConsentProvisioning:$AllowMicConsentProvisioning `
        -TimeoutSeconds $TimeoutSeconds `
        -CleanupOnFailure $CleanupOnFailure)
    $code = @($result | Where-Object { $_ -is [int] } | Select-Object -Last 1)
    if (-not $code.Count) {
        Write-Error 'provision primitive did not return an exit code'
        exit 2
    }
    exit [int]$code[0]
} catch {
    Write-Error "provision primitive contract failure: $($_.Exception.Message)"
    exit 2
}
