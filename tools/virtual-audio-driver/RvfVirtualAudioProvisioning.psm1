# RvfVirtualAudioProvisioning.psm1
# sv#115 Slice A - shared implementation for the virtual audio endpoint provision/unprovision primitive.
# The external contract is fixed to Provision-RvfVirtualAudioEndpoint.ps1 /
# Unprovision-RvfVirtualAudioEndpoint.ps1 plus state JSON (rvf-virtual-audio-provisioning/v1).

$script:RvfLogPath = $null
$script:RvfStageLogPath = $null
$script:RvfEvidenceDir = $null

function ConvertTo-RvfUtc {
    param([Parameter(Mandatory)][DateTime]$Value)
    return $Value.ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
}

function Get-RvfSha256 {
    param([Parameter(Mandatory)][string]$LiteralPath)
    if (Get-Command Get-FileHash -ErrorAction SilentlyContinue) {
        return (Get-FileHash -LiteralPath $LiteralPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    # Fallback for minimal PowerShell environments where Get-FileHash is absent (same SHA-256 lowercase hex).
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $stream = $null
    try {
        $stream = [System.IO.FileStream]::new($LiteralPath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
        $bytes = $sha.ComputeHash($stream)
        return ([System.BitConverter]::ToString($bytes)).Replace('-', '').ToLowerInvariant()
    } finally {
        if ($stream) { $stream.Dispose() }
        if ($sha) { $sha.Dispose() }
    }
}

function Resolve-RvfPath {
    param([Parameter(Mandatory)][string]$Path)
    if ([IO.Path]::IsPathRooted($Path)) { return [IO.Path]::GetFullPath($Path) }
    return [IO.Path]::GetFullPath((Join-Path (Get-Location).Path $Path))
}

function Initialize-RvfLog {
    param(
        [Parameter(Mandatory)][string]$EvidenceDir,
        [Parameter(Mandatory)][string]$Operation
    )
    $script:RvfEvidenceDir = $EvidenceDir
    New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
    $script:RvfLogPath = Join-Path $EvidenceDir "$Operation.log"
    New-Item -ItemType File -Force -Path $script:RvfLogPath | Out-Null
    $script:RvfStageLogPath = $null
}

function Write-RvfLog {
    param([AllowEmptyString()][string]$Message = '')
    Write-Host $Message
    if ($script:RvfLogPath) { Add-Content -Path $script:RvfLogPath -Value $Message -Encoding utf8 }
    if ($script:RvfStageLogPath) { Add-Content -Path $script:RvfStageLogPath -Value $Message -Encoding utf8 }
}

function Start-RvfStageLog {
    param([Parameter(Mandatory)][string]$Stage)
    if ($script:RvfEvidenceDir) {
        $script:RvfStageLogPath = Join-Path $script:RvfEvidenceDir "stage-$Stage.log"
        New-Item -ItemType File -Force -Path $script:RvfStageLogPath | Out-Null
    }
}

function Stop-RvfStageLog {
    $script:RvfStageLogPath = $null
}

function Add-RvfStageRecord {
    param(
        [Parameter(Mandatory)][object]$State,
        [Parameter(Mandatory)][string]$Stage,
        [Parameter(Mandatory)][ValidateSet('ok', 'absent', 'blocked', 'not_run', 'error')][string]$Status,
        [string]$Detail = '',
        [object]$ExitCode = $null
    )
    $record = [pscustomobject][ordered]@{
        stage        = $Stage
        status       = $Status
        detail       = $Detail
        exit_code    = $ExitCode
        recorded_utc = ConvertTo-RvfUtc (Get-Date)
    }
    $State.stages = @($State.stages) + $record
    Write-RvfLog ("stage={0} status={1} detail={2}" -f $Stage, $Status, $Detail)
}

function New-RvfState {
    param(
        [Parameter(Mandatory)][string]$Operation,
        [Parameter(Mandatory)][object]$Inputs
    )
    $now = ConvertTo-RvfUtc (Get-Date)
    return [pscustomobject][ordered]@{
        schema        = 'rvf-virtual-audio-provisioning/v1'
        operation     = $Operation
        status        = 'not_run'
        detail        = 'UNSET'
        exit_code     = 1
        recorded_utc  = $now
        started_utc   = $now
        finished_utc  = $null
        source        = [pscustomobject][ordered]@{
            repo_url      = $null
            checkout_rev  = $null
            expected_rev  = $Inputs.expected_source_rev
            dirty         = $null
        }
        runner        = [pscustomobject][ordered]@{
            os_caption         = $null
            os_version         = $null
            os_build           = $null
            arch               = $null
            github_actions     = $null
            runner_environment = $null
            admin              = $null
            testsigning        = $null
            secure_boot        = $null
            msbuild            = $null
            vs_install         = $null
            km_headers         = 0
            km_libs            = 0
            initial_toolset_count = 0
            initial_km_headers   = 0
            wdk_install_attempted = $false
            wdm_header_count     = 0
            initial_km_libs      = 0
            initial_signtool_hits = 0
            initial_inf2cat_hits = 0
            initial_makecat_hits = 0
            winget_present       = $null
        }
        inputs        = $Inputs
        stages        = @()
        endpoint      = [pscustomobject][ordered]@{
            candidate   = $false
            pair_count  = 0
            render      = @()
            capture     = @()
            pairs       = @()
            probe_result = 'not_run'
        }
        package       = [pscustomobject][ordered]@{
            inf_path         = $null
            cat_path         = $null
            sys_path         = $null
            package_dir      = $null
            inf_sha256       = $null
            cat_sha256       = $null
            sys_sha256       = $null
            publisher        = 'RvfVirtualAudio Probe (rf977)'
            cert_thumbprint  = $null
            cert_not_after   = $null
            signed           = $false
        }
        os_state      = [pscustomobject][ordered]@{
            instance_id        = $null
            inf_published      = $null
            cert_thumbprint    = $null
            consent_before     = @()
            consent_after      = @()
            service_restart    = @()
            pre_devnode        = $null
        }
        artifacts     = @()
        cleanup       = $null
    }
}

function Get-RvfArtifactManifest {
    param([Parameter(Mandatory)][string]$EvidenceDir)
    if (-not (Test-Path -LiteralPath $EvidenceDir)) { return @() }
    $excluded = @('provision-state.json', 'unprovision-state.json', 'artifact-manifest.json')
    $files = @(Get-ChildItem -LiteralPath $EvidenceDir -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $excluded -notcontains $_.Name } |
        Sort-Object FullName)
    $manifest = @()
    foreach ($file in $files) {
        $hash = Get-RvfSha256 -LiteralPath $file.FullName
        $kind = if ($file.Extension) { $file.Extension.TrimStart('.').ToLowerInvariant() } else { 'file' }
        $manifest += [pscustomobject][ordered]@{
            path   = $file.FullName
            kind   = $kind
            sha256 = $hash
        }
    }
    return $manifest
}

function Save-RvfState {
    param(
        [Parameter(Mandatory)][object]$State,
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$EvidenceDir
    )
    $State.recorded_utc = ConvertTo-RvfUtc (Get-Date)
    if (-not $State.finished_utc) { $State.finished_utc = $State.recorded_utc }
    $State.artifacts = @(Get-RvfArtifactManifest -EvidenceDir $EvidenceDir)
    $parent = Split-Path -Parent $Path
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    $json = $State | ConvertTo-Json -Depth 12
    [IO.File]::WriteAllText($Path, $json, [Text.UTF8Encoding]::new($false))
    $manifestJson = ConvertTo-Json -InputObject @($State.artifacts) -Depth 5
    [IO.File]::WriteAllText((Join-Path $EvidenceDir 'artifact-manifest.json'), $manifestJson, [Text.UTF8Encoding]::new($false))
}

function Find-RvfTool {
    param(
        [Parameter(Mandatory)][string]$Name,
        [switch]$AllKitsHits
    )
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($cmd) { return @($cmd.Source) }
    $exe = if ($Name -notmatch '\.') { "$Name.exe" } else { $Name }
    $roots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "${env:ProgramFiles}\Windows Kits\10\bin"
    ) | Where-Object { $_ -and (Test-Path $_) }
    $hits = @()
    foreach ($root in $roots) {
        $hits += @(Get-ChildItem $root -Recurse -Depth 3 -Filter $exe -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty FullName)
    }
    if ($AllKitsHits) { return $hits }
    if ($hits.Count -gt 0) { return @($hits[0]) }
    return @()
}

function Invoke-RvfProcess {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @(),
        [int]$TimeoutSeconds = 300,
        [string]$WorkingDirectory = $null
    )
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $FilePath
    # Windows PowerShell 5.1 lacks ProcessStartInfo.ArgumentList, so use explicitly quoted Arguments.
    $quoted = @()
    foreach ($arg in @($Arguments)) {
        $text = [string]$arg
        if ($text -match '[\s"]') {
            $text = '"' + ($text -replace '"', '\"') + '"'
        }
        $quoted += $text
    }
    $psi.Arguments = $quoted -join ' '
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    if ($WorkingDirectory) { $psi.WorkingDirectory = $WorkingDirectory }

    $process = $null
    try {
        $process = [Diagnostics.Process]::Start($psi)
    } catch {
        return [pscustomobject]@{
            exit_code = 1
            timed_out = $false
            output    = @("process start error: $($_.Exception.Message)")
        }
    }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $timeoutMs = [Math]::Max(1, $TimeoutSeconds) * 1000
    $timedOut = -not $process.WaitForExit($timeoutMs)
    if ($timedOut) {
        try { $process.Kill($true) } catch { try { $process.Kill() } catch {} }
    }
    try { $process.WaitForExit() } catch {}
    $stdout = ''
    $stderr = ''
    try { $stdout = $stdoutTask.GetAwaiter().GetResult() } catch {}
    try { $stderr = $stderrTask.GetAwaiter().GetResult() } catch {}
    $lines = @()
    if ($stdout) { $lines += @($stdout -split "`r?`n" | Where-Object { $_ -ne '' }) }
    if ($stderr) { $lines += @($stderr -split "`r?`n" | Where-Object { $_ -ne '' } | ForEach-Object { "stderr: $_" }) }
    if ($timedOut) { $lines += "process timeout after $TimeoutSeconds seconds" }
    $exitCode = if ($timedOut) { 124 } else { $process.ExitCode }
    try { $process.Dispose() } catch {}
    return [pscustomobject]@{
        exit_code = $exitCode
        timed_out = $timedOut
        output    = $lines
    }
}

function Get-RvfPwshPath {
    $cmd = Get-Command pwsh -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    return (Get-Process -Id $PID).Path
}

function Invoke-RvfPowerShellScript {
    param(
        [Parameter(Mandatory)][string]$ScriptPath,
        [string[]]$Arguments = @(),
        [int]$TimeoutSeconds = 300,
        [string]$WorkingDirectory = $null
    )
    $pwsh = Get-RvfPwshPath
    $allArgs = @('-NoProfile', '-NonInteractive', '-File', $ScriptPath) + $Arguments
    return Invoke-RvfProcess -FilePath $pwsh -Arguments $allArgs -TimeoutSeconds $TimeoutSeconds -WorkingDirectory $WorkingDirectory
}

function Write-RvfProcessOutput {
    param(
        [Parameter(Mandatory)][object]$Result,
        [string]$Prefix = '  '
    )
    foreach ($line in @($Result.output)) { Write-RvfLog "$Prefix$line" }
}

function Get-RvfEnvironmentInfo {
    $os = Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    $bcdOut = @()
    $bcdExit = $null
    $bcd = Invoke-RvfProcess -FilePath 'bcdedit.exe' -Arguments @('/enum') -TimeoutSeconds 20
    $bcdOut = @($bcd.output)
    $bcdExit = $bcd.exit_code
    $testsigning = [bool](@($bcdOut | Where-Object { $_ -match 'testsigning\s+Yes' }).Count)
    $secureBoot = $null
    try { $secureBoot = [bool](Confirm-SecureBootUEFI -ErrorAction Stop) } catch { $secureBoot = "unavailable: $($_.Exception.Message)" }
    return [pscustomobject][ordered]@{
        os_caption         = if ($os) { $os.Caption } else { $null }
        os_version         = if ($os) { $os.Version } else { $null }
        os_build           = if ($os) { $os.BuildNumber } else { $null }
        arch               = $env:PROCESSOR_ARCHITECTURE
        github_actions     = $env:GITHUB_ACTIONS
        runner_environment = $env:RUNNER_ENVIRONMENT
        runner_os          = $env:RUNNER_OS
        runner_arch        = $env:RUNNER_ARCH
        admin              = $isAdmin
        testsigning        = $testsigning
        secure_boot        = $secureBoot
        bcdedit_exit       = $bcdExit
        bcdedit_output     = $bcdOut
    }
}

function Write-RvfEnvironmentSurface {
    param([Parameter(Mandatory)][object]$Info)
    Write-RvfLog "os: $($Info.os_caption) version=$($Info.os_version) build=$($Info.os_build) arch=$($Info.arch)"
    Write-RvfLog "github_actions=$($Info.github_actions) runner_environment=$($Info.runner_environment) runner_os=$($Info.runner_os) runner_arch=$($Info.runner_arch) admin=$($Info.admin)"
    Write-RvfLog "testsigning=$($Info.testsigning) secure_boot=$($Info.secure_boot) bcdedit_exit=$($Info.bcdedit_exit)"
    foreach ($line in @($Info.bcdedit_output)) { Write-RvfLog "  bcdedit: $line" }
    Write-RvfLog '== tools on PATH =='
    foreach ($tool in 'pnputil', 'signtool', 'devcon', 'infverif', 'Inf2Cat', 'makecat', 'certutil', 'vswhere', 'msbuild', 'cl', 'link', 'winget', 'dotnet', 'git') {
        $cmd = Get-Command $tool -ErrorAction SilentlyContinue
        if ($cmd) { Write-RvfLog "  ${tool}: $($cmd.Source)" } else { Write-RvfLog "  ${tool}: not on PATH" }
    }
    Write-RvfLog '== Windows Kits bin hits =='
    foreach ($tool in 'signtool.exe', 'Inf2Cat.exe', 'makecat.exe', 'devcon.exe', 'infverif.exe', 'certutil.exe') {
        $hits = @(Find-RvfTool $tool -AllKitsHits)
        if ($hits.Count) { foreach ($hit in $hits) { Write-RvfLog "  ${tool} -> $hit" } }
        else { Write-RvfLog "  ${tool}: absent" }
    }
}

function Test-RvfEphemeralEnvironment {
    param([Parameter(Mandatory)][object]$Info)
    if ($Info.arch -ne 'AMD64') {
        return [pscustomobject]@{ ok = $false; detail = 'UNSUPPORTED_ARCH'; reason = "arch=$($Info.arch)" }
    }
    if ($Info.github_actions -ne 'true' -or $Info.runner_environment -ne 'github-hosted') {
        return [pscustomobject]@{ ok = $false; detail = 'NON_EPHEMERAL_RUNNER'; reason = "GITHUB_ACTIONS=$($Info.github_actions); RUNNER_ENVIRONMENT=$($Info.runner_environment)" }
    }
    if (-not $Info.admin) {
        return [pscustomobject]@{ ok = $false; detail = 'UNSUPPORTED_ENVIRONMENT'; reason = 'admin=false' }
    }
    if ("$($Info.os_caption)" -notmatch 'Windows Server 2025') {
        return [pscustomobject]@{ ok = $false; detail = 'UNSUPPORTED_ENVIRONMENT'; reason = "os_caption=$($Info.os_caption)" }
    }
    return [pscustomobject]@{ ok = $true; detail = 'OK'; reason = '' }
}

function Get-RvfBuildSurface {
    param([string]$Arch = 'x64')
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    $vsInstall = $null
    $msbuild = $null
    $msbuildCandidates = @()
    if (Test-Path $vswhere) {
        $vsInstall = @(& $vswhere -latest -products * -property installationPath 2>&1 | Select-Object -First 1) -join ''
        $msbuildCandidates = @(& $vswhere -latest -products * -requires Microsoft.Component.MSBuild -find 'MSBuild\**\Bin\MSBuild.exe' 2>&1)
        foreach ($candidate in $msbuildCandidates) {
            $amd64 = $candidate -replace 'Bin\\MSBuild\.exe$', 'Bin\amd64\MSBuild.exe'
            if ($amd64 -ne $candidate -and (Test-Path $amd64)) { $msbuild = $amd64; break }
        }
        if (-not $msbuild) { $msbuild = @($msbuildCandidates | Where-Object { $_ -and (Test-Path $_) }) | Select-Object -First 1 }
    }
    if (-not $msbuild) {
        $cmd = Get-Command msbuild -ErrorAction SilentlyContinue
        if ($cmd) { $msbuild = $cmd.Source }
    }
    $toolset = @()
    $toolsetAllPlatforms = @()
    if ($vsInstall) {
        $toolset = @(Get-ChildItem "$vsInstall\MSBuild\Microsoft\VC\*\Platforms\$Arch\PlatformToolsets\WindowsKernelModeDriver*" -Directory -ErrorAction SilentlyContinue)
        $toolsetAllPlatforms = @(Get-ChildItem "$vsInstall\MSBuild\Microsoft\VC\*\Platforms\*\PlatformToolsets\WindowsKernelModeDriver*" -Directory -ErrorAction SilentlyContinue)
    }
    $kitsRoot = "${env:ProgramFiles(x86)}\Windows Kits\10"
    $includeRoot = Join-Path $kitsRoot 'Include'
    $kmHeaders = 0
    $wdmHeaders = 0
    foreach ($header in 'wdm.h', 'wdf.h', 'portcls.h', 'stdunk.h') {
        if (Test-Path $includeRoot) {
            $hits = @(Get-ChildItem $includeRoot -Recurse -Depth 2 -Filter $header -ErrorAction SilentlyContinue).Count
            $kmHeaders += $hits
            if ($header -eq 'wdm.h') { $wdmHeaders = $hits }
        }
    }
    $libRoot = Join-Path $kitsRoot 'Lib'
    $kmLibs = 0
    if (Test-Path $libRoot) {
        $kmLibs = @(Get-ChildItem $libRoot -Recurse -Depth 2 -Directory -ErrorAction SilentlyContinue | Where-Object { $_.FullName -match '\\km\\' }).Count
    }
    $driverKitTasks = @()
    $buildRoot = Join-Path $kitsRoot 'build'
    if (Test-Path $buildRoot) {
        $driverKitTasks = @(Get-ChildItem $buildRoot -Recurse -Filter 'Microsoft.DriverKit.Build.Tasks.*.dll' -ErrorAction SilentlyContinue)
    }
    return [pscustomobject][ordered]@{
        vswhere          = if (Test-Path $vswhere) { $vswhere } else { $null }
        vs_install       = $vsInstall
        msbuild          = $msbuild
        toolset_count    = $toolset.Count
        toolsets         = @($toolset | ForEach-Object { $_.FullName })
        toolset_all_count = $toolsetAllPlatforms.Count
        toolsets_all     = @($toolsetAllPlatforms | ForEach-Object { $_.FullName })
        km_headers       = $kmHeaders
        wdm_header_count = $wdmHeaders
        km_libs          = $kmLibs
        driverkit_tasks  = @($driverKitTasks | ForEach-Object { $_.FullName })
        signtool_hits    = @(Find-RvfTool 'signtool.exe' -AllKitsHits)
        inf2cat_hits     = @(Find-RvfTool 'Inf2Cat.exe' -AllKitsHits)
        makecat_hits     = @(Find-RvfTool 'makecat.exe' -AllKitsHits)
    }
}

function Copy-RvfDriverKitTasksForMSBuild {
    param(
        [Parameter(Mandatory)][string]$MSBuildPath,
        [Parameter(Mandatory)][int]$TimeoutSeconds
    )
    $versionProbe = Invoke-RvfProcess -FilePath $MSBuildPath -Arguments @('-version', '-nologo') -TimeoutSeconds ([Math]::Min(60, $TimeoutSeconds))
    Write-RvfProcessOutput $versionProbe '  msbuild-version: '
    $versionText = @($versionProbe.output | Select-Object -Last 1) -join ''
    $version = $versionText -as [version]
    $major = if ($version) { $version.Major } else { 0 }
    Write-RvfLog "msbuild major=$major"
    if (-not $major) { return }
    foreach ($binDir in @(Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\build\*\bin" -Directory -ErrorAction SilentlyContinue)) {
        foreach ($src in @(Get-ChildItem $binDir.FullName -Filter 'Microsoft.DriverKit.Build.Tasks*.dll' -ErrorAction SilentlyContinue)) {
            if ($src.Name -notmatch '\.\d+\.0\.dll$' -or $src.Name -match "\.$major\.0\.dll$") { continue }
            $dst = Join-Path $binDir.FullName ($src.Name -replace '\.\d+\.0\.dll$', ".$major.0.dll")
            if (Test-Path $dst) { continue }
            Write-RvfLog "  task dll copy: $($src.Name) -> $(Split-Path $dst -Leaf)"
            try { Copy-Item $src.FullName $dst -ErrorAction Stop } catch { Write-RvfLog "  copy failed: $($_.Exception.Message)" }
        }
    }
}

function Get-RvfFileSha256 {
    param([string]$Path)
    if (-not $Path -or -not (Test-Path -LiteralPath $Path)) { return $null }
    return (Get-RvfSha256 -LiteralPath $Path)
}

function Get-RvfMicConsentTargets {
    $micCap = '{E5323777-F976-4f5b-9B55-B94699C46E44}'
    return @(
        [pscustomobject]@{ hive = 'HKLM:'; key = "SOFTWARE\Microsoft\Windows\CurrentVersion\DeviceAccess\Global\$micCap"; name = 'Value'; value = 'Allow'; tag = 'hklm-devacc' },
        [pscustomobject]@{ hive = 'HKCU:'; key = "SOFTWARE\Microsoft\Windows\CurrentVersion\DeviceAccess\Global\$micCap"; name = 'Value'; value = 'Allow'; tag = 'hkcu-devacc' },
        [pscustomobject]@{ hive = 'HKLM:'; key = 'SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone'; name = 'Value'; value = 'Allow'; tag = 'hklm-cam' },
        [pscustomobject]@{ hive = 'HKCU:'; key = 'SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone'; name = 'Value'; value = 'Allow'; tag = 'hkcu-cam' },
        [pscustomobject]@{ hive = 'HKLM:'; key = 'SOFTWARE\Policies\Microsoft\Windows\AppPrivacy'; name = 'LetAppsAccessMicrophone'; value = 1; tag = 'policy' }
    )
}

function Get-RvfRegistryValueState {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Name
    )
    $keyExists = Test-Path -LiteralPath $Path
    $valueExists = $false
    $value = $null
    if ($keyExists) {
        $item = Get-ItemProperty -LiteralPath $Path -Name $Name -ErrorAction SilentlyContinue
        if ($null -ne $item -and $item.PSObject.Properties[$Name]) {
            $valueExists = $true
            $value = $item.$Name
        }
    }
    return [pscustomobject][ordered]@{
        key_exists   = $keyExists
        value_exists = $valueExists
        value        = $value
    }
}

function Set-RvfMicConsent {
    param([Parameter(Mandatory)][object]$State)
    $beforeRecords = @()
    $afterRecords = @()
    $problems = @()
    foreach ($target in @(Get-RvfMicConsentTargets)) {
        $path = Join-Path $target.hive $target.key
        try {
            $before = Get-RvfRegistryValueState -Path $path -Name $target.name
            if (-not $before.key_exists) { New-Item -Path $path -Force | Out-Null }
            if ($target.value -is [int]) {
                Set-ItemProperty -LiteralPath $path -Name $target.name -Value $target.value -Type DWord -ErrorAction Stop
            } else {
                Set-ItemProperty -LiteralPath $path -Name $target.name -Value $target.value -ErrorAction Stop
            }
            $after = Get-RvfRegistryValueState -Path $path -Name $target.name
            Write-RvfLog "  mic-consent [$($target.tag)] $path\$($target.name) : '$($before.value)' -> '$($after.value)'"
            $beforeRecords += [pscustomobject][ordered]@{
                tag          = $target.tag
                path         = $path
                name         = $target.name
                key_exists   = $before.key_exists
                value_exists = $before.value_exists
                value        = $before.value
            }
            $afterRecords += [pscustomobject][ordered]@{
                tag          = $target.tag
                path         = $path
                name         = $target.name
                key_exists   = $after.key_exists
                value_exists = $after.value_exists
                value        = $after.value
            }
            if ("$($after.value)" -ne "$($target.value)") { $problems += "$($target.tag)=$($after.value)" }
        } catch {
            Write-RvfLog "  mic-consent [$($target.tag)] $path error: $($_.Exception.Message)"
            $problems += "$($target.tag)=error"
            $beforeRecords += [pscustomobject][ordered]@{
                tag          = $target.tag
                path         = $path
                name         = $target.name
                key_exists   = $null
                value_exists = $null
                value        = $null
                error        = $_.Exception.Message
            }
        }
    }
    $State.os_state.consent_before = $beforeRecords
    $State.os_state.consent_after = $afterRecords
    return $problems
}

function Restart-RvfAudioServices {
    param([Parameter(Mandatory)][object]$State)
    $records = @()
    $problems = @()
    foreach ($serviceName in 'AudioEndpointBuilder', 'Audiosrv') {
        $before = $null
        $after = $null
        $errorText = ''
        try {
            $before = (Get-Service $serviceName -ErrorAction SilentlyContinue).Status
            Restart-Service $serviceName -Force -ErrorAction Stop
            $after = (Get-Service $serviceName -ErrorAction Stop).Status
            Write-RvfLog "  svc $serviceName : $before -> $after"
            if ($after -ne 'Running') { $problems += "$serviceName=$after" }
        } catch {
            $errorText = $_.Exception.Message
            Write-RvfLog "  svc $serviceName restart error: $errorText"
            $problems += "$serviceName=error"
        }
        $records += [pscustomobject][ordered]@{
            service = $serviceName
            before  = if ($null -ne $before) { "$before" } else { $null }
            after   = if ($null -ne $after) { "$after" } else { $null }
            error   = $errorText
        }
    }
    $State.os_state.service_restart = $records
    Start-Sleep -Seconds 2
    return $problems
}

function Initialize-RvfMmDeviceProbeType {
    if ('RvfMmDeviceProbe' -as [type]) { return }
    $source = @'
using System;
using System.Runtime.InteropServices;
using System.Collections.Generic;

public static class RvfMmDeviceProbe
{
    public sealed class EndpointInfo
    {
        public int Flow;
        public string FlowName;
        public string Id;
        public string Name;
        public int State;
    }

    [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
    private class MMDeviceEnumeratorCom { }

    [ComImport, Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IMMDeviceEnumerator
    {
        int EnumAudioEndpoints(int dataFlow, int stateMask, out IMMDeviceCollection ppDevices);
        int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice ppDevice);
        int RegisterEndpointNotificationCallback(IntPtr pClient);
        int UnregisterEndpointNotificationCallback(IntPtr pClient);
    }

    [ComImport, Guid("0BD7A1BE-7A1A-44DB-8397-CC5392387B5E"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IMMDeviceCollection
    {
        int GetCount(out uint pcDevices);
        int Item(uint nDevice, out IMMDevice ppDevice);
    }

    [ComImport, Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IMMDevice
    {
        int Activate(ref Guid iid, int dwClsCtx, IntPtr pActivationParams, [MarshalAs(UnmanagedType.IUnknown)] out object ppInterface);
        int OpenPropertyStore(int stgmAccess, out IPropertyStore ppProperties);
        int GetId([MarshalAs(UnmanagedType.LPWStr)] out string ppstrId);
        int GetState(out int pdwState);
    }

    [ComImport, Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IPropertyStore
    {
        int GetCount(out uint cProps);
        int GetAt(uint iProp, out PROPERTYKEY pkey);
        int GetValue(ref PROPERTYKEY key, out PROPVARIANT pv);
        int SetValue(ref PROPERTYKEY key, ref PROPVARIANT pv);
        int Commit();
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct PROPERTYKEY
    {
        public Guid fmtid;
        public uint pid;
    }

    [StructLayout(LayoutKind.Explicit)]
    private struct PROPVARIANT
    {
        [FieldOffset(0)] public ushort vt;
        [FieldOffset(8)] public IntPtr ptr;
    }

    [DllImport("ole32.dll")]
    private static extern int PropVariantClear(ref PROPVARIANT pvar);

    private const int ERender = 0;
    private const int ECapture = 1;
    private const uint DeviceStateActive = 1;
    private const ushort VtLpWstr = 31;

    private static string ReadFriendlyName(IMMDevice device)
    {
        IPropertyStore store;
        int hr = device.OpenPropertyStore(0, out store);
        if (hr != 0 || store == null) return null;
        var key = new PROPERTYKEY();
        key.fmtid = new Guid("a45c254e-df1c-4efd-8020-67d146a850e0");
        key.pid = 14;
        PROPVARIANT pv = new PROPVARIANT();
        try
        {
            hr = store.GetValue(ref key, out pv);
            if (hr != 0) return null;
            if (pv.vt == VtLpWstr && pv.ptr != IntPtr.Zero) return Marshal.PtrToStringUni(pv.ptr);
            return null;
        }
        finally
        {
            PropVariantClear(ref pv);
        }
    }

    private static void EnumFlow(IMMDeviceEnumerator enumerator, int flow, string flowName, List<EndpointInfo> result)
    {
        IMMDeviceCollection collection;
        int hr = enumerator.EnumAudioEndpoints(flow, unchecked((int)DeviceStateActive), out collection);
        if (hr != 0 || collection == null) return;
        uint count;
        hr = collection.GetCount(out count);
        if (hr != 0) return;
        for (uint i = 0; i < count; i++)
        {
            IMMDevice device;
            hr = collection.Item(i, out device);
            if (hr != 0 || device == null) continue;
            string id;
            int state;
            device.GetId(out id);
            device.GetState(out state);
            result.Add(new EndpointInfo {
                Flow = flow,
                FlowName = flowName,
                Id = id,
                Name = ReadFriendlyName(device),
                State = state
            });
        }
    }

    public static EndpointInfo[] EnumerateActive()
    {
        var result = new List<EndpointInfo>();
        var enumerator = (IMMDeviceEnumerator)new MMDeviceEnumeratorCom();
        EnumFlow(enumerator, ERender, "render", result);
        EnumFlow(enumerator, ECapture, "capture", result);
        return result.ToArray();
    }
}
'@
    Add-Type -TypeDefinition $source -ErrorAction Stop
}

function Test-RvfVirtualEndpointName {
    param([AllowNull()][string]$Name)
    if (-not $Name) { return $false }
    $lower = $Name.ToLowerInvariant()
    foreach ($hint in 'vb-cable', 'vb audio', 'vb-audio', 'cable input', 'cable output', 'hi-fi cable', 'hifi-cable', 'virtual audio cable', 'voicemeeter') {
        if ($lower.Contains($hint)) { return $true }
    }
    return $false
}

function Get-RvfVirtualStem {
    param([AllowNull()][string]$Name)
    if (-not (Test-RvfVirtualEndpointName $Name)) { return $null }
    $tokens = @($Name.ToLowerInvariant() -split '[^a-z0-9]+' | Where-Object { $_ })
    $direction = @('input', 'output', 'render', 'capture', 'in', 'out')
    $stemTokens = @($tokens | Where-Object { $direction -notcontains $_ })
    if (-not $stemTokens.Count) { return $null }
    return ($stemTokens -join ' ')
}

function Get-RvfEndpointPairs {
    param([Parameter(Mandatory)][object[]]$Endpoints)
    $free = @()
    foreach ($endpoint in $Endpoints) {
        if ($endpoint.FlowName -ne 'capture') { continue }
        $stem = Get-RvfVirtualStem $endpoint.Name
        if ($stem) { $free += [pscustomobject]@{ endpoint = $endpoint; stem = $stem } }
    }
    $pairs = @()
    foreach ($render in @($Endpoints | Where-Object { $_.FlowName -eq 'render' })) {
        $stem = Get-RvfVirtualStem $render.Name
        if (-not $stem) { continue }
        $index = -1
        for ($i = 0; $i -lt $free.Count; $i++) {
            if ($free[$i].stem -eq $stem) { $index = $i; break }
        }
        if ($index -lt 0) { continue }
        $capture = $free[$index].endpoint
        $free = @($free | Where-Object { $_ -ne $free[$index] })
        $pairs += [pscustomobject][ordered]@{
            render  = $render.Name
            capture = $capture.Name
            stem    = $stem
            render_id = $render.Id
            capture_id = $capture.Id
        }
    }
    return $pairs
}

function Invoke-RvfEndpointProbe {
    param([Parameter(Mandatory)][object]$State)
    Initialize-RvfMmDeviceProbeType
    $endpoints = @([RvfMmDeviceProbe]::EnumerateActive())
    Write-RvfLog '== primitive MMDevice active endpoint enumeration =='
    foreach ($endpoint in $endpoints) {
        Write-RvfLog "  [$($endpoint.FlowName)] $($endpoint.Name) state=$($endpoint.State) id=$($endpoint.Id)"
    }
    $virtual = @($endpoints | Where-Object { Test-RvfVirtualEndpointName $_.Name })
    $pairs = @(Get-RvfEndpointPairs -Endpoints $endpoints)
    Write-RvfLog '== pair candidates (virtual name-stem match, unconfirmed) =='
    if ($pairs.Count) {
        foreach ($pair in $pairs) { Write-RvfLog "  render \"$($pair.render)\" <-> capture \"$($pair.capture)\" stem=\"$($pair.stem)\"" }
    } else {
        Write-RvfLog '  none - NOT_PROVISIONED'
    }
    Write-RvfLog "  endpoints enumerated: $($endpoints.Count)"
    Write-RvfLog "  virtual-endpoint candidates (name heuristic, unconfirmed): $($virtual.Count)"
    Write-RvfLog "  render/capture pair candidates (name-stem match, unconfirmed): $($pairs.Count)"

    $State.endpoint.pair_count = $pairs.Count
    $State.endpoint.candidate = ($pairs.Count -gt 0)
    $State.endpoint.render = @($endpoints | Where-Object { $_.FlowName -eq 'render' } | ForEach-Object { [pscustomobject]@{ name = $_.Name; id = $_.Id; state = $_.State } })
    $State.endpoint.capture = @($endpoints | Where-Object { $_.FlowName -eq 'capture' } | ForEach-Object { [pscustomobject]@{ name = $_.Name; id = $_.Id; state = $_.State } })
    $State.endpoint.pairs = $pairs
    $State.endpoint.probe_result = if ($pairs.Count -gt 0) { 'CANDIDATES_OBSERVED' } elseif ($virtual.Count -gt 0) { 'UNPAIRED_CANDIDATES_OBSERVED' } else { 'NOT_PROVISIONED' }
    return $pairs
}

function Get-RvfDriverStoreEntries {
    $result = Invoke-RvfProcess -FilePath 'pnputil.exe' -Arguments @('/enum-drivers') -TimeoutSeconds 60
    $entries = @()
    $current = $null
    foreach ($line in @($result.output)) {
        if ($line -match '^\s*Published Name:\s*(\S+)') {
            if ($current) { $entries += [pscustomobject]$current }
            $current = [ordered]@{ published_name = $Matches[1]; original_name = ''; provider_name = ''; class_name = ''; signer_name = ''; raw = @() }
        }
        if ($current) {
            $current.raw += $line
            if ($line -match '^\s*Original Name:\s*(\S+)') { $current.original_name = $Matches[1] }
            if ($line -match '^\s*Provider Name:\s*(.+?)\s*$') { $current.provider_name = $Matches[1] }
            if ($line -match '^\s*Class Name:\s*(.+?)\s*$') { $current.class_name = $Matches[1] }
            if ($line -match '^\s*Signer Name:\s*(.+?)\s*$') { $current.signer_name = $Matches[1] }
        }
    }
    if ($current) { $entries += [pscustomobject]$current }
    return [pscustomobject]@{ exit_code = $result.exit_code; output = $result.output; entries = $entries }
}

function Find-RvfRuviformDevnodes {
    param([string]$InstanceId = $null)
    $matchesOut = @()
    $devices = @(Get-PnpDevice -ErrorAction SilentlyContinue | Where-Object { $_.InstanceId -like 'ROOT\*' })
    foreach ($device in $devices) {
        $hardwareIds = @()
        try {
            $prop = Get-PnpDeviceProperty -InstanceId $device.InstanceId -KeyName 'DEVPKEY_Device_HardwareIds' -ErrorAction SilentlyContinue
            if ($prop -and $prop.Data) { $hardwareIds = @($prop.Data) }
        } catch {}
        if (($InstanceId -and $device.InstanceId -eq $InstanceId) -or (@($hardwareIds | Where-Object { "$_" -eq 'ROOT\RvfVirtualAudio' }).Count -gt 0)) {
            $matchesOut += $device
        }
    }
    return $matchesOut
}

function Add-RvfCleanupStage {
    param(
        [Parameter(Mandatory)][object]$Cleanup,
        [Parameter(Mandatory)][string]$Stage,
        [Parameter(Mandatory)][ValidateSet('ok', 'absent', 'blocked', 'not_run', 'error')][string]$Status,
        [string]$Detail = '',
        [object]$ExitCode = $null
    )
    $Cleanup.stages = @($Cleanup.stages) + [pscustomobject][ordered]@{
        stage = $Stage
        status = $Status
        detail = $Detail
        exit_code = $ExitCode
        recorded_utc = ConvertTo-RvfUtc (Get-Date)
    }
    Write-RvfLog "cleanup-stage=$Stage status=$Status detail=$Detail"
}

function Invoke-RvfCleanup {
    param(
        [Parameter(Mandatory)][object]$SourceState,
        [Parameter(Mandatory)][ValidateSet('Full', 'ReportOnly')][string]$Mode,
        [int]$TimeoutSeconds = 900
    )
    $cleanup = [pscustomobject][ordered]@{
        mode      = $Mode
        stages    = @()
        leftovers = @()
        errors    = @()
    }
    $reportOnly = ($Mode -eq 'ReportOnly')
    $instanceId = $null
    if ($SourceState.os_state -and $SourceState.os_state.instance_id) { $instanceId = [string]$SourceState.os_state.instance_id }
    $sourceRoot = $null
    if ($SourceState.inputs -and $SourceState.inputs.source_root) { $sourceRoot = [string]$SourceState.inputs.source_root }
    $testDevice = $null
    if ($sourceRoot) {
        $candidate = Join-Path $sourceRoot 'tools\testsigned-driver\TestDevice.ps1'
        if (Test-Path -LiteralPath $candidate) { $testDevice = $candidate }
    }

    Start-RvfStageLog 'devnode-cleanup'
    $devices = @(Find-RvfRuviformDevnodes -InstanceId $instanceId)
    $ids = @()
    if ($instanceId) { $ids += $instanceId }
    foreach ($device in $devices) { if ($ids -notcontains $device.InstanceId) { $ids += $device.InstanceId } }
    if (-not $ids.Count) {
        Add-RvfCleanupStage -Cleanup $cleanup -Stage 'devnode' -Status ok -Detail 'no matching devnode'
    } else {
        $problems = @()
        foreach ($id in $ids) {
            Write-RvfLog "  --- remove devnode $id ---"
            if ($reportOnly) {
                Write-RvfLog '  report-only: remove not attempted'
                continue
            }
            $removed = $false
            if ($testDevice) {
                $result = Invoke-RvfPowerShellScript -ScriptPath $testDevice -Arguments @('-Action', 'remove', '-InstanceId', $id) -TimeoutSeconds ([Math]::Min(120, $TimeoutSeconds))
                Write-RvfProcessOutput $result '  '
                if ($result.exit_code -eq 0) { $removed = $true }
            }
            if (-not $removed) {
                $result = Invoke-RvfProcess -FilePath 'pnputil.exe' -Arguments @('/remove-device', $id) -TimeoutSeconds ([Math]::Min(120, $TimeoutSeconds))
                Write-RvfProcessOutput $result '  pnputil-remove: '
            }
            Start-Sleep -Milliseconds 500
            $gone = -not (Get-PnpDevice -InstanceId $id -ErrorAction SilentlyContinue)
            Write-RvfLog "  devnode $id removed=$gone"
            if (-not $gone) { $problems += "devnode_residue:$id" }
        }
        Add-RvfCleanupStage -Cleanup $cleanup -Stage 'devnode' -Status $(if ($problems.Count) { 'error' } else { 'ok' }) -Detail ($problems -join ';')
        foreach ($problem in $problems) { $cleanup.errors += $problem }
    }
    Stop-RvfStageLog

    Start-RvfStageLog 'driver-store-cleanup'
    $published = $null
    if ($SourceState.os_state -and $SourceState.os_state.inf_published) { $published = [string]$SourceState.os_state.inf_published }
    $publisher = if ($SourceState.package -and $SourceState.package.publisher) { [string]$SourceState.package.publisher } else { 'RvfVirtualAudio Probe (rf977)' }
    $enumBefore = Get-RvfDriverStoreEntries
    foreach ($line in @($enumBefore.output)) { Write-RvfLog "  enum-before: $line" }
    $deleteNames = @()
    if ($published) { $deleteNames += $published }
    if (-not $deleteNames.Count) {
        foreach ($entry in @($enumBefore.entries)) {
            if ($entry.original_name -ieq 'rvfvirtualaudio.inf' -and (($entry.signer_name -eq $publisher) -or ($entry.provider_name -eq 'Ruviform'))) {
                $deleteNames += $entry.published_name
            }
        }
    }
    $deleteNames = @($deleteNames | Select-Object -Unique)
    $installedNames = @($enumBefore.entries | ForEach-Object { $_.published_name })
    $missingNames = @($deleteNames | Where-Object { $installedNames -notcontains $_ })
    if ($missingNames.Count) { Write-RvfLog "  already absent from driver store: $($missingNames -join ', ')" }
    $deleteNames = @($deleteNames | Where-Object { $installedNames -contains $_ })
    if (-not $deleteNames.Count) {
        $unowned = @($enumBefore.entries | Where-Object { $_.original_name -ieq 'rvfvirtualaudio.inf' })
        if ($unowned.Count) {
            $cleanup.leftovers += 'driver-store:rvfvirtualaudio-inf-unowned-or-unrecorded'
            Add-RvfCleanupStage -Cleanup $cleanup -Stage 'driver-store' -Status error -Detail 'CLEANUP_RESIDUE driver INF residue exists but primitive provenance is not recorded'
        } else {
            Add-RvfCleanupStage -Cleanup $cleanup -Stage 'driver-store' -Status ok -Detail 'no matching driver package'
        }
    } else {
        $problems = @()
        foreach ($name in $deleteNames) {
            Write-RvfLog "  --- pnputil /delete-driver $name /uninstall /force ---"
            if ($reportOnly) {
                Write-RvfLog '  report-only: delete not attempted'
                continue
            }
            $result = Invoke-RvfProcess -FilePath 'pnputil.exe' -Arguments @('/delete-driver', $name, '/uninstall', '/force') -TimeoutSeconds ([Math]::Min(180, $TimeoutSeconds))
            Write-RvfProcessOutput $result '  '
            if ($result.exit_code -ne 0) { $problems += "delete_driver_failed:$name" }
        }
        Add-RvfCleanupStage -Cleanup $cleanup -Stage 'driver-store' -Status $(if ($problems.Count) { 'error' } else { 'ok' }) -Detail ($problems -join ';')
        foreach ($problem in $problems) { $cleanup.errors += $problem }
    }
    Stop-RvfStageLog

    Start-RvfStageLog 'cert-cleanup'
    $thumbprint = $null
    if ($SourceState.os_state -and $SourceState.os_state.cert_thumbprint) { $thumbprint = [string]$SourceState.os_state.cert_thumbprint }
    if (-not $thumbprint -and $SourceState.package) { $thumbprint = [string]$SourceState.package.cert_thumbprint }
    if (-not $thumbprint) {
        Add-RvfCleanupStage -Cleanup $cleanup -Stage 'cert' -Status ok -Detail 'no cert thumbprint recorded'
    } else {
        $problems = @()
        foreach ($store in 'Root', 'TrustedPublisher', 'My') {
            $path = "Cert:\LocalMachine\$store\$thumbprint"
            if (Test-Path $path) {
                Write-RvfLog "  remove $path"
                if (-not $reportOnly) {
                    try { Remove-Item $path -Force -ErrorAction Stop; Write-RvfLog "  removed $path" }
                    catch { Write-RvfLog "  remove $path error: $($_.Exception.Message)"; $problems += "cert_residue:$store" }
                }
            } else {
                Write-RvfLog "  $path : already absent"
            }
        }
        Add-RvfCleanupStage -Cleanup $cleanup -Stage 'cert' -Status $(if ($problems.Count) { 'error' } else { 'ok' }) -Detail ($problems -join ';')
        foreach ($problem in $problems) { $cleanup.errors += $problem }
    }
    Stop-RvfStageLog

    Start-RvfStageLog 'consent-cleanup'
    $consentBefore = @()
    if ($SourceState.os_state -and $SourceState.os_state.consent_before) { $consentBefore = @($SourceState.os_state.consent_before) }
    if (-not $consentBefore.Count) {
        Add-RvfCleanupStage -Cleanup $cleanup -Stage 'consent' -Status ok -Detail 'no consent before-values recorded'
    } else {
        $problems = @()
        foreach ($entry in $consentBefore) {
            $path = [string]$entry.path
            $name = [string]$entry.name
            if (-not $path -or -not $name) { continue }
            if ($null -eq $entry.value_exists) {
                $problems += "consent_before_not_recorded:$path\$name"
                continue
            }
            Write-RvfLog "  restore consent $path\$name"
            if ($reportOnly) { continue }
            try {
                if ([bool]$entry.value_exists) {
                    if (-not (Test-Path -LiteralPath $path)) { New-Item -Path $path -Force | Out-Null }
                    Set-ItemProperty -LiteralPath $path -Name $name -Value $entry.value -ErrorAction Stop
                } else {
                    if (Test-Path -LiteralPath $path) {
                        Remove-ItemProperty -LiteralPath $path -Name $name -ErrorAction SilentlyContinue
                        $remaining = Get-Item -LiteralPath $path -ErrorAction SilentlyContinue
                        if ($remaining -and -not [bool]$entry.key_exists -and $remaining.Property.Count -eq 0 -and @(Get-ChildItem -LiteralPath $path -ErrorAction SilentlyContinue).Count -eq 0) {
                            Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
                        }
                    }
                }
            } catch {
                Write-RvfLog "  restore consent error: $($_.Exception.Message)"
                $problems += "consent_restore_failed:$path\$name"
            }
        }
        Add-RvfCleanupStage -Cleanup $cleanup -Stage 'consent' -Status $(if ($problems.Count) { 'error' } else { 'ok' }) -Detail ($problems -join ';')
        foreach ($problem in $problems) { $cleanup.errors += $problem }
    }
    Stop-RvfStageLog

    Start-RvfStageLog 'service-cleanup'
    $serviceRecords = @()
    if ($SourceState.os_state -and $SourceState.os_state.service_restart) { $serviceRecords = @($SourceState.os_state.service_restart) }
    if (-not $serviceRecords.Count -and -not $consentBefore.Count) {
        Add-RvfCleanupStage -Cleanup $cleanup -Stage 'service' -Status ok -Detail 'service restart not required by state'
    } else {
        $problems = @()
        foreach ($serviceName in 'AudioEndpointBuilder', 'Audiosrv') {
            Write-RvfLog "  restart $serviceName"
            if ($reportOnly) { continue }
            try {
                Restart-Service $serviceName -Force -ErrorAction Stop
                $after = (Get-Service $serviceName -ErrorAction Stop).Status
                Write-RvfLog "  svc $serviceName after=$after"
                if ($after -ne 'Running') { $problems += "$serviceName=$after" }
            } catch {
                Write-RvfLog "  svc $serviceName restart error: $($_.Exception.Message)"
                $problems += "$serviceName=error"
            }
        }
        Add-RvfCleanupStage -Cleanup $cleanup -Stage 'service' -Status $(if ($problems.Count) { 'error' } else { 'ok' }) -Detail ($problems -join ';')
        foreach ($problem in $problems) { $cleanup.errors += "service:$problem" }
    }
    Stop-RvfStageLog

    Start-RvfStageLog 'residue-check'
    $residueDevices = @(Find-RvfRuviformDevnodes -InstanceId $instanceId)
    foreach ($device in $residueDevices) { $cleanup.leftovers += "devnode:$($device.InstanceId)" }
    $enumAfter = Get-RvfDriverStoreEntries
    $driverResidue = @($enumAfter.entries | Where-Object { $_.original_name -ieq 'rvfvirtualaudio.inf' -and (($_.signer_name -eq $publisher) -or ($_.provider_name -eq 'Ruviform') -or ($_.published_name -eq $published)) })
    foreach ($entry in $driverResidue) { $cleanup.leftovers += "driver:$($entry.published_name)" }
    if ($thumbprint) {
        foreach ($store in 'Root', 'TrustedPublisher', 'My') {
            if (Test-Path "Cert:\LocalMachine\$store\$thumbprint") { $cleanup.leftovers += "cert:${store}:$thumbprint" }
        }
    }
    foreach ($entry in $consentBefore) {
        if ($null -eq $entry.value_exists) { $cleanup.leftovers += "consent-before-missing:$($entry.path)\$($entry.name)"; continue }
        $path = [string]$entry.path
        $name = [string]$entry.name
        if (-not $path -or -not $name) { continue }
        $current = Get-RvfRegistryValueState -Path $path -Name $name
        if ([bool]$entry.value_exists) {
            if (-not $current.value_exists -or "$($current.value)" -ne "$($entry.value)") { $cleanup.leftovers += "consent:$path\$name" }
        } elseif ($current.value_exists) {
            $cleanup.leftovers += "consent:$path\$name"
        }
    }
    foreach ($leftover in @($cleanup.leftovers | Select-Object -Unique)) { Write-RvfLog "  leftover: $leftover" }
    $cleanup.leftovers = @($cleanup.leftovers | Select-Object -Unique)
    Add-RvfCleanupStage -Cleanup $cleanup -Stage 'residue-check' -Status $(if ($cleanup.leftovers.Count) { 'error' } else { 'ok' }) -Detail "leftovers=$($cleanup.leftovers.Count)"
    Stop-RvfStageLog

    return $cleanup
}

function Invoke-RvfProvision {
    param(
        [Parameter(Mandatory)][string]$SourceRoot,
        [string]$ExpectedSourceRev,
        [Parameter(Mandatory)][string]$WorkspaceRoot,
        [Parameter(Mandatory)][string]$StatePath,
        [Parameter(Mandatory)][string]$EvidenceDir,
        [string]$Arch = 'x64',
        [string]$Configuration = 'Release',
        [switch]$AllowWdkInstall,
        [switch]$AllowMicConsentProvisioning,
        [int]$TimeoutSeconds = 3600,
        [ValidateSet('Full', 'Preserve')][string]$CleanupOnFailure = 'Full'
    )

    $SourceRoot = Resolve-RvfPath $SourceRoot
    $WorkspaceRoot = Resolve-RvfPath $WorkspaceRoot
    $StatePath = Resolve-RvfPath $StatePath
    $EvidenceDir = Resolve-RvfPath $EvidenceDir
    New-Item -ItemType Directory -Force -Path $WorkspaceRoot | Out-Null
    Initialize-RvfLog -EvidenceDir $EvidenceDir -Operation 'provision'

    $inputs = [pscustomobject][ordered]@{
        source_root                     = $SourceRoot
        expected_source_rev             = $ExpectedSourceRev
        workspace_root                  = $WorkspaceRoot
        state_path                      = $StatePath
        evidence_dir                    = $EvidenceDir
        arch                            = $Arch
        configuration                   = $Configuration
        allow_wdk_install               = [bool]$AllowWdkInstall
        allow_mic_consent_provisioning  = [bool]$AllowMicConsentProvisioning
        timeout_seconds                 = $TimeoutSeconds
        cleanup_on_failure              = $CleanupOnFailure
    }
    $state = New-RvfState -Operation 'provision' -Inputs $inputs
    $started = Get-Date
    $terminalStatus = 'error'
    $terminalDetail = 'UNSET'
    $stageFailed = $false
    $mutatedOsState = $false

    try {
        Start-RvfStageLog 'environment'
        $envInfo = Get-RvfEnvironmentInfo
        Write-RvfEnvironmentSurface $envInfo
        $initialSurface = Get-RvfBuildSurface -Arch $Arch
        Write-RvfLog "initial build surface: msbuild=$(if ($initialSurface.msbuild) {'present'} else {'absent'}) toolset=$($initialSurface.toolset_all_count) kmHeaders=$($initialSurface.km_headers) kmLibs=$($initialSurface.km_libs) signtoolKits=$($initialSurface.signtool_hits.Count) inf2catKits=$($initialSurface.inf2cat_hits.Count) makecatKits=$($initialSurface.makecat_hits.Count) winget=$(if (Get-Command winget -ErrorAction SilentlyContinue) {'present'} else {'absent'})"
        $state.runner.initial_toolset_count = $initialSurface.toolset_all_count
        $state.runner.initial_km_headers = $initialSurface.km_headers
        $state.runner.initial_km_libs = $initialSurface.km_libs
        $state.runner.initial_signtool_hits = $initialSurface.signtool_hits.Count
        $state.runner.initial_inf2cat_hits = $initialSurface.inf2cat_hits.Count
        $state.runner.initial_makecat_hits = $initialSurface.makecat_hits.Count
        $state.runner.winget_present = [bool](Get-Command winget -ErrorAction SilentlyContinue)
        $state.runner.os_caption = $envInfo.os_caption
        $state.runner.os_version = $envInfo.os_version
        $state.runner.os_build = $envInfo.os_build
        $state.runner.arch = $envInfo.arch
        $state.runner.github_actions = ($envInfo.github_actions -eq 'true')
        $state.runner.runner_environment = $envInfo.runner_environment
        $state.runner.admin = $envInfo.admin
        $state.runner.testsigning = $envInfo.testsigning
        $state.runner.secure_boot = $envInfo.secure_boot
        if ($Arch -ne 'x64') {
            $terminalStatus = 'not_run'
            $terminalDetail = 'UNSUPPORTED_ARCH'
            Add-RvfStageRecord -State $state -Stage 'environment' -Status not_run -Detail "UNSUPPORTED_ARCH input_arch=$Arch (supported: x64)"
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        $guard = Test-RvfEphemeralEnvironment $envInfo
        if (-not $guard.ok) {
            $terminalStatus = 'not_run'
            $terminalDetail = $guard.detail
            Add-RvfStageRecord -State $state -Stage 'environment' -Status not_run -Detail "$($guard.detail); $($guard.reason)"
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        $envDetail = "msbuild=$(if ($initialSurface.msbuild) {'present'} else {'absent'}); toolset=$($initialSurface.toolset_all_count); kmHeaders=$($initialSurface.km_headers); kmLibs=$($initialSurface.km_libs); signtoolKits=$($initialSurface.signtool_hits.Count); inf2catKits=$($initialSurface.inf2cat_hits.Count); winget=$(if ($state.runner.winget_present) {'present'} else {'absent'})"
        Add-RvfStageRecord -State $state -Stage 'environment' -Status ok -Detail $envDetail
        Stop-RvfStageLog

        Start-RvfStageLog 'source'
        $required = @(
            'drivers\rvf-virtual-audio',
            'drivers\rvf-virtual-audio\RvfVirtualAudio.vcxproj',
            'drivers\rvf-virtual-audio\RvfVirtualAudio.inf',
            'tools\testsigned-driver\TestDevice.ps1'
        )
        $missing = @($required | Where-Object { -not (Test-Path -LiteralPath (Join-Path $SourceRoot $_)) })
        foreach ($path in $required) { Write-RvfLog "  source-check $path : $(if ($missing -contains $path) {'missing'} else {'present'})" }
        $gitRev = Invoke-RvfProcess -FilePath 'git.exe' -Arguments @('-C', $SourceRoot, 'rev-parse', 'HEAD') -TimeoutSeconds 30
        $gitUrl = Invoke-RvfProcess -FilePath 'git.exe' -Arguments @('-C', $SourceRoot, 'config', '--get', 'remote.origin.url') -TimeoutSeconds 30
        $gitDirty = Invoke-RvfProcess -FilePath 'git.exe' -Arguments @('-C', $SourceRoot, 'status', '--porcelain') -TimeoutSeconds 30
        $checkoutRev = (@($gitRev.output) | Select-Object -First 1)
        $repoUrl = (@($gitUrl.output) | Select-Object -First 1)
        $dirty = @($gitDirty.output).Count -gt 0
        $state.source.repo_url = $repoUrl
        $state.source.checkout_rev = $checkoutRev
        $state.source.dirty = $dirty
        Write-RvfLog "  checkout_rev=$checkoutRev expected=$ExpectedSourceRev dirty=$dirty repo=$repoUrl"
        if ($missing.Count -or $gitRev.exit_code -ne 0 -or -not $checkoutRev) {
            $terminalStatus = 'error'
            $terminalDetail = 'SOURCE_INVALID'
            Add-RvfStageRecord -State $state -Stage 'source' -Status error -Detail "missing=$($missing -join ','); git_exit=$($gitRev.exit_code)" -ExitCode $gitRev.exit_code
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        if ($ExpectedSourceRev -and $checkoutRev -ine $ExpectedSourceRev) {
            $terminalStatus = 'error'
            $terminalDetail = 'SOURCE_REV_MISMATCH'
            Add-RvfStageRecord -State $state -Stage 'source' -Status error -Detail "SOURCE_REV_MISMATCH expected=$ExpectedSourceRev actual=$checkoutRev"
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        Add-RvfStageRecord -State $state -Stage 'source' -Status ok -Detail "rev=$checkoutRev dirty=$dirty"
        Stop-RvfStageLog

        Start-RvfStageLog 'wdk'
        $surface = Get-RvfBuildSurface -Arch $Arch
        $wdkInstallAttempted = $false
        Write-RvfLog "  initial: msbuild=$($surface.msbuild) toolset=$($surface.toolset_count) kmHeaders=$($surface.km_headers) kmLibs=$($surface.km_libs) signtoolKits=$($surface.signtool_hits.Count) inf2catKits=$($surface.inf2cat_hits.Count)"
        if ((-not $surface.msbuild -or -not $surface.toolset_count -or -not $surface.km_headers) -and $AllowWdkInstall) {
            Write-RvfLog '  == job-local WDK install (winget / Microsoft.WindowsWDK.10.0.26100) =='
            $wingetCmd = Get-Command winget -ErrorAction SilentlyContinue
            if ($wingetCmd) {
                $wdkInstallAttempted = $true
                $winget = Invoke-RvfProcess -FilePath $wingetCmd.Source -Arguments @('install', '--id', 'Microsoft.WindowsWDK.10.0.26100', '--exact', '--accept-source-agreements', '--accept-package-agreements', '--disable-interactivity') -TimeoutSeconds ([Math]::Min(1800, $TimeoutSeconds))
                Write-RvfProcessOutput $winget '  winget: '
                Write-RvfLog "  winget exit=$($winget.exit_code) timed_out=$($winget.timed_out)"
            } else {
                Write-RvfLog '  winget: absent'
            }
            $surface = Get-RvfBuildSurface -Arch $Arch
            Write-RvfLog "  post-install: msbuild=$($surface.msbuild) toolset=$($surface.toolset_count) kmHeaders=$($surface.km_headers) kmLibs=$($surface.km_libs) inf2catKits=$($surface.inf2cat_hits.Count)"
        }
        if ($surface.msbuild) { Copy-RvfDriverKitTasksForMSBuild -MSBuildPath $surface.msbuild -TimeoutSeconds $TimeoutSeconds }
        $state.runner.msbuild = $surface.msbuild
        $state.runner.vs_install = $surface.vs_install
        $state.runner.km_headers = $surface.km_headers
        $state.runner.km_libs = $surface.km_libs
        $state.runner.wdm_header_count = $surface.wdm_header_count
        $state.runner.wdk_install_attempted = $wdkInstallAttempted
        if (-not $surface.msbuild -or -not $surface.toolset_count -or -not $surface.km_headers) {
            $terminalStatus = 'absent'
            $terminalDetail = 'WDK_ABSENT'
            Add-RvfStageRecord -State $state -Stage 'wdk' -Status absent -Detail "WDK_ABSENT msbuild=$(if ($surface.msbuild) {'present'} else {'absent'}); toolset=$($surface.toolset_count); kmHeaders=$($surface.km_headers); allowInstall=$([bool]$AllowWdkInstall)"
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        Add-RvfStageRecord -State $state -Stage 'wdk' -Status ok -Detail "msbuild=present; toolset=$($surface.toolset_count); kmHeaders=$($surface.km_headers); inf2catKits=$($surface.inf2cat_hits.Count)"
        Stop-RvfStageLog

        Start-RvfStageLog 'driver-build'
        $project = Join-Path $SourceRoot 'drivers\rvf-virtual-audio\RvfVirtualAudio.vcxproj'
        $build = Invoke-RvfProcess -FilePath $surface.msbuild -Arguments @($project, "/p:Configuration=$Configuration", "/p:Platform=$Arch", '/v:m', '/nologo') -TimeoutSeconds ([Math]::Min(1800, $TimeoutSeconds)) -WorkingDirectory $SourceRoot
        Write-RvfProcessOutput $build '  '
        Write-RvfLog "  msbuild exit=$($build.exit_code) timed_out=$($build.timed_out)"
        $sys = Get-ChildItem (Join-Path $SourceRoot 'drivers\rvf-virtual-audio') -Recurse -Filter 'RvfVirtualAudio.sys' -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($build.exit_code -ne 0 -or -not $sys) {
            $terminalStatus = 'error'
            $terminalDetail = 'DRIVER_BUILD_FAILED'
            Add-RvfStageRecord -State $state -Stage 'driver-build' -Status error -Detail "DRIVER_BUILD_FAILED msbuild exit=$($build.exit_code)" -ExitCode $build.exit_code
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        $sysPath = $sys.FullName
        $state.package.sys_path = $sysPath
        Write-RvfLog "  built: $sysPath"
        Add-RvfStageRecord -State $state -Stage 'driver-build' -Status ok -Detail "arch=$Arch; sys=$sysPath" -ExitCode $build.exit_code
        Stop-RvfStageLog

        Start-RvfStageLog 'catalog'
        $pkgDir = Join-Path $WorkspaceRoot 'vaudio-pkg'
        New-Item -ItemType Directory -Force -Path $pkgDir | Out-Null
        Copy-Item -LiteralPath $sysPath -Destination $pkgDir -Force
        Copy-Item -LiteralPath (Join-Path $SourceRoot 'drivers\rvf-virtual-audio\RvfVirtualAudio.inf') -Destination $pkgDir -Force
        foreach ($file in @(Get-ChildItem $pkgDir -File -ErrorAction SilentlyContinue)) { Write-RvfLog "  pkg: $($file.Name) $($file.Length)B" }
        $catPath = $null
        $inf2catHits = @(Find-RvfTool 'Inf2Cat.exe' -AllKitsHits)
        foreach ($hit in $inf2catHits) { Write-RvfLog "  inf2cat hit: $hit" }
        if ($inf2catHits.Count) {
            $probe = Invoke-RvfProcess -FilePath $inf2catHits[0] -Arguments @() -TimeoutSeconds 30 -WorkingDirectory $pkgDir
            Write-RvfProcessOutput ($probe | Select-Object -Property @{n='exit_code';e={$_.exit_code}},@{n='timed_out';e={$_.timed_out}},@{n='output';e={@($_.output | Select-Object -First 25)}}) '  inf2cat(no-args): '
            foreach ($osFlag in '10_0_X64', '10_X64') {
                Write-RvfLog "  --- Inf2Cat /os:$osFlag ---"
                $ic = Invoke-RvfProcess -FilePath $inf2catHits[0] -Arguments @('/driver:.', "/os:$osFlag", '/verbose') -TimeoutSeconds ([Math]::Min(300, $TimeoutSeconds)) -WorkingDirectory $pkgDir
                Write-RvfProcessOutput $ic '  '
                Write-RvfLog "  Inf2Cat exit=$($ic.exit_code) os=$osFlag"
                if ($ic.exit_code -eq 0) {
                    $cat = Get-ChildItem (Join-Path $pkgDir '*.cat') -ErrorAction SilentlyContinue | Select-Object -First 1
                    if ($cat) { $catPath = $cat.FullName; break }
                }
            }
        } else {
            Write-RvfLog '  Inf2Cat: absent'
        }
        if (-not $catPath) {
            Write-RvfLog '  Inf2Cat failed/absent - makecat fallback'
            $makecatHits = @(Find-RvfTool 'makecat.exe' -AllKitsHits)
            if ($makecatHits.Count) {
                $makecat = @($makecatHits | Where-Object { $_ -match '\\x64\\' } | Select-Object -First 1)[0]
                if (-not $makecat) { $makecat = $makecatHits[0] }
                $cdf = Join-Path $pkgDir 'package.cdf'
                $cdfLines = @(
                    '[CatalogHeader]',
                    'Name=RvfVirtualAudio.cat',
                    "ResultDir=$pkgDir",
                    'PublicVersion=0x00000001',
                    'EncodingType=0x00010001',
                    'CATATTR1=0x10010001:OSAttr:2:6.0',
                    '[CatalogFiles]',
                    '<HASH>RvfVirtualAudio.inf=RvfVirtualAudio.inf',
                    '<HASH>RvfVirtualAudio.sys=RvfVirtualAudio.sys'
                )
                [IO.File]::WriteAllLines($cdf, $cdfLines, [Text.Encoding]::ASCII)
                $mc = Invoke-RvfProcess -FilePath $makecat -Arguments @('-v', 'package.cdf') -TimeoutSeconds ([Math]::Min(300, $TimeoutSeconds)) -WorkingDirectory $pkgDir
                Write-RvfProcessOutput $mc '  makecat: '
                Write-RvfLog "  makecat exit=$($mc.exit_code)"
                $cat = Get-ChildItem (Join-Path $pkgDir '*.cat') -ErrorAction SilentlyContinue | Select-Object -First 1
                if ($cat) { $catPath = $cat.FullName }
            } else {
                Write-RvfLog '  makecat: absent'
            }
        }
        if (-not $catPath) {
            $terminalStatus = 'error'
            $terminalDetail = 'CATALOG_CREATE_FAILED'
            Add-RvfStageRecord -State $state -Stage 'catalog' -Status error -Detail 'CATALOG_CREATE_FAILED Inf2Cat/makecat could not create .cat'
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        $infPath = Join-Path $pkgDir 'RvfVirtualAudio.inf'
        $pkgSysPath = Join-Path $pkgDir 'RvfVirtualAudio.sys'
        $state.package.inf_path = $infPath
        $state.package.cat_path = $catPath
        $state.package.package_dir = $pkgDir
        $state.package.inf_sha256 = Get-RvfFileSha256 $infPath
        $state.package.cat_sha256 = Get-RvfFileSha256 $catPath
        $state.package.sys_sha256 = Get-RvfFileSha256 $pkgSysPath
        Add-RvfStageRecord -State $state -Stage 'catalog' -Status ok -Detail "cat=$catPath"
        Stop-RvfStageLog

        Start-RvfStageLog 'cert-create'
        try {
            $cert = New-SelfSignedCertificate -Type CodeSigningCert `
                -Subject 'CN=RvfVirtualAudio Probe (rf977)' `
                -CertStoreLocation 'Cert:\LocalMachine\My' `
                -KeyLength 2048 -HashAlgorithm sha256 -KeyExportPolicy Exportable `
                -NotAfter (Get-Date).AddDays(2) -ErrorAction Stop
            $cerPath = Join-Path $EvidenceDir 'vaudio-cert.cer'
            Export-Certificate -Cert $cert -FilePath $cerPath -ErrorAction Stop | Out-Null
            Write-RvfLog "  subject: $($cert.Subject)"
            Write-RvfLog "  thumbprint: $($cert.Thumbprint)"
            Write-RvfLog "  cer: $cerPath"
            $state.package.cert_thumbprint = $cert.Thumbprint
            $state.package.cert_not_after = ConvertTo-RvfUtc $cert.NotAfter
            $state.os_state.cert_thumbprint = $cert.Thumbprint
            $mutatedOsState = $true
            Add-RvfStageRecord -State $state -Stage 'cert-create' -Status ok -Detail "thumbprint=$($cert.Thumbprint)"
        } catch {
            $terminalStatus = 'error'
            $terminalDetail = 'CERT_CREATE_FAILED'
            Write-RvfLog "  cert-create error: $($_.Exception.Message)"
            Add-RvfStageRecord -State $state -Stage 'cert-create' -Status error -Detail "CERT_CREATE_FAILED $($_.Exception.Message)"
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        Stop-RvfStageLog

        Start-RvfStageLog 'cert-install'
        $certInstallOk = $true
        foreach ($store in 'Root', 'TrustedPublisher') {
            try {
                Import-Certificate -FilePath $cerPath -CertStoreLocation "Cert:\LocalMachine\$store" -ErrorAction Stop | Out-Null
                $found = Get-Item "Cert:\LocalMachine\$store\$($cert.Thumbprint)" -ErrorAction SilentlyContinue
                Write-RvfLog "  ${store}: registered=$($null -ne $found)"
                if (-not $found) { $certInstallOk = $false }
            } catch {
                Write-RvfLog "  ${store}: error - $($_.Exception.Message)"
                $certInstallOk = $false
            }
        }
        $mutatedOsState = $true
        if (-not $certInstallOk) {
            $terminalStatus = 'error'
            $terminalDetail = 'CERT_INSTALL_FAILED'
            Add-RvfStageRecord -State $state -Stage 'cert-install' -Status error -Detail "CERT_INSTALL_FAILED thumbprint=$($cert.Thumbprint)"
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        Add-RvfStageRecord -State $state -Stage 'cert-install' -Status ok -Detail "thumbprint=$($cert.Thumbprint); stores=Root+TrustedPublisher"
        Stop-RvfStageLog

        Start-RvfStageLog 'sign'
        $signtoolHits = @(Find-RvfTool 'signtool.exe' -AllKitsHits)
        foreach ($hit in $signtoolHits) { Write-RvfLog "  signtool hit: $hit" }
        if (-not $signtoolHits.Count) {
            $terminalStatus = 'absent'
            $terminalDetail = 'SIGN_FAILED'
            Add-RvfStageRecord -State $state -Stage 'sign' -Status absent -Detail 'SIGN_FAILED signtool absent'
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        $signtool = @($signtoolHits | Where-Object { $_ -match '\\x64\\' } | Select-Object -First 1)[0]
        if (-not $signtool) { $signtool = $signtoolHits[0] }
        Write-RvfLog "  signtool selected: $signtool"
        $signTargets = @($sysPath, $catPath)
        $signExitSum = 0
        foreach ($target in $signTargets) {
            $sign = Invoke-RvfProcess -FilePath $signtool -Arguments @('sign', '/fd', 'sha256', '/sha1', $cert.Thumbprint, '/sm', '/v', $target) -TimeoutSeconds ([Math]::Min(300, $TimeoutSeconds))
            Write-RvfProcessOutput $sign '  '
            Write-RvfLog "  sign exit=$($sign.exit_code) file=$(Split-Path $target -Leaf)"
            $signExitSum += $sign.exit_code
        }
        if ($signExitSum -ne 0) {
            $terminalStatus = 'error'
            $terminalDetail = 'SIGN_FAILED'
            Add-RvfStageRecord -State $state -Stage 'sign' -Status error -Detail "SIGN_FAILED targets=$(($signTargets | ForEach-Object { Split-Path $_ -Leaf }) -join ',')" -ExitCode $signExitSum
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        Add-RvfStageRecord -State $state -Stage 'sign' -Status ok -Detail "signtool=$signtool; targets=$(($signTargets | ForEach-Object { Split-Path $_ -Leaf }) -join ',')" -ExitCode $signExitSum
        Stop-RvfStageLog

        Start-RvfStageLog 'sign-verify'
        $verifyExitSum = 0
        foreach ($target in $signTargets) {
            $verify = Invoke-RvfProcess -FilePath $signtool -Arguments @('verify', '/pa', '/v', $target) -TimeoutSeconds ([Math]::Min(300, $TimeoutSeconds))
            Write-RvfProcessOutput $verify '  '
            Write-RvfLog "  verify exit=$($verify.exit_code) file=$(Split-Path $target -Leaf)"
            $verifyExitSum += $verify.exit_code
        }
        if ($verifyExitSum -ne 0) {
            $terminalStatus = 'error'
            $terminalDetail = 'SIGN_VERIFY_FAILED'
            Add-RvfStageRecord -State $state -Stage 'sign-verify' -Status error -Detail "SIGN_VERIFY_FAILED targets=$(($signTargets | ForEach-Object { Split-Path $_ -Leaf }) -join ',')" -ExitCode $verifyExitSum
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        $state.package.signed = $true
        Add-RvfStageRecord -State $state -Stage 'sign-verify' -Status ok -Detail "targets=$(($signTargets | ForEach-Object { Split-Path $_ -Leaf }) -join ',')" -ExitCode $verifyExitSum
        Stop-RvfStageLog

        Start-RvfStageLog 'driver-store'
        $infForInstall = Join-Path $pkgDir 'RvfVirtualAudio.inf'
        $add = Invoke-RvfProcess -FilePath 'pnputil.exe' -Arguments @('/add-driver', $infForInstall, '/install') -TimeoutSeconds ([Math]::Min(600, $TimeoutSeconds))
        Write-RvfProcessOutput $add '  '
        Write-RvfLog "  pnputil exit=$($add.exit_code)"
        $published = $null
        $match = @($add.output | Select-String -Pattern 'Published Name\s*:\s*(\S+)' -AllMatches)
        foreach ($m in $match) { if ($m.Matches.Count -and $m.Matches[0].Groups[1].Value -match 'oem\d+\.inf') { $published = $m.Matches[0].Groups[1].Value; break } }
        $enum = Get-RvfDriverStoreEntries
        foreach ($line in @($enum.output)) { Write-RvfLog "  enum: $line" }
        if (-not $published) {
            foreach ($entry in @($enum.entries)) {
                if ($entry.original_name -ieq 'rvfvirtualaudio.inf') { $published = $entry.published_name; break }
            }
        }
        $staged = [bool]$published
        if ($published) {
            $state.os_state.inf_published = $published
            Write-RvfLog "  published: $published"
        }
        if ($add.exit_code -ne 0 -or -not $staged) {
            $terminalStatus = 'error'
            $terminalDetail = 'PNPUTIL_ADD_FAILED'
            Add-RvfStageRecord -State $state -Stage 'driver-store' -Status error -Detail "PNPUTIL_ADD_FAILED published=$published" -ExitCode $add.exit_code
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        $mutatedOsState = $true
        Add-RvfStageRecord -State $state -Stage 'driver-store' -Status ok -Detail "published=$published" -ExitCode $add.exit_code
        Stop-RvfStageLog

        # Legacy manual-probe parity: enum-verify is a read-only pre-devnode
        # observation. Record it before creating the root-enumerated devnode.
        Start-RvfStageLog 'enum-verify-pre-devnode'
        foreach ($line in @($enum.output)) { Write-RvfLog "  enum-drivers: $line" }
        $preInfInStore = [bool]$published
        $preDevices = @(Get-PnpDevice -ErrorAction SilentlyContinue | Where-Object { $_.FriendlyName -match 'ruviform|RvfVirtualAudio' -or $_.InstanceId -match 'RvfVirtualAudio' })
        Write-RvfLog "  RvfVirtualAudio.inf in driver store: $preInfInStore"
        if ($preDevices.Count) { foreach ($preDev in $preDevices) { Write-RvfLog "  $($preDev.InstanceId) Status=$($preDev.Status) Class=$($preDev.Class) Name=$($preDev.FriendlyName)" } }
        else { Write-RvfLog '  matching PnpDevice: 0' }
        $state.os_state.pre_devnode = [pscustomobject][ordered]@{
            inf_in_store      = $preInfInStore
            published_name    = $published
            matching_devnodes = $preDevices.Count
            devices           = @($preDevices | ForEach-Object { [pscustomobject]@{ instance_id = $_.InstanceId; status = "$($_.Status)"; class = "$($_.Class)"; friendly_name = "$($_.FriendlyName)" } })
            recorded_utc      = ConvertTo-RvfUtc (Get-Date)
        }
        Write-RvfLog "  pre-devnode observation: inf_in_store=$preInfInStore matching_devnodes=$($preDevices.Count)"
        Stop-RvfStageLog

        Start-RvfStageLog 'devnode'
        $testDevice = Join-Path $SourceRoot 'tools\testsigned-driver\TestDevice.ps1'
        $devnode = Invoke-RvfPowerShellScript -ScriptPath $testDevice -Arguments @(
            '-Action', 'create',
            '-HardwareId', 'ROOT\RvfVirtualAudio',
            '-ClassName', 'MEDIA',
            '-ClassGuid', '4d36e96c-e325-11ce-bfc1-08002be10318',
            '-DeviceDescription', 'Ruviform Virtual Audio Device',
            '-InfPath', $infForInstall
        ) -TimeoutSeconds ([Math]::Min(300, $TimeoutSeconds)) -WorkingDirectory $SourceRoot
        Write-RvfProcessOutput $devnode '  '
        $lines = @($devnode.output)
        $instanceId = (($lines | Where-Object { $_ -match '^instance_id=' } | Select-Object -First 1) -replace '^instance_id=', '')
        Write-RvfLog "  instance_id=$instanceId script_exit=$($devnode.exit_code)"
        if ($instanceId) { $state.os_state.instance_id = $instanceId; $mutatedOsState = $true }
        Start-Sleep -Seconds 3
        $dev = $null
        if ($instanceId) { $dev = Get-PnpDevice -InstanceId $instanceId -ErrorAction SilentlyContinue }
        if ($dev) {
            Write-RvfLog "  Status=$($dev.Status) Class=$($dev.Class)"
            Write-RvfLog "  FriendlyName=$($dev.FriendlyName)"
            Write-RvfLog "  Service=$($dev.Service)"
            $props = @(Get-PnpDeviceProperty -InstanceId $instanceId -KeyName 'DEVPKEY_Device_ProblemCode', 'DEVPKEY_Device_DevNodeStatus', 'DEVPKEY_Device_DriverVersion', 'DEVPKEY_Device_DriverDate' -ErrorAction SilentlyContinue)
            foreach ($prop in $props) { Write-RvfLog "  $($prop.KeyName) = $($prop.Data)" }
            $sysdrv = Get-CimInstance Win32_SystemDriver -Filter "Name='RvfVirtualAudio'" -ErrorAction SilentlyContinue
            if ($sysdrv) { Write-RvfLog "  Win32_SystemDriver: State=$($sysdrv.State) Status=$($sysdrv.Status) PathName=$($sysdrv.PathName)" }
            else { Write-RvfLog '  Win32_SystemDriver RvfVirtualAudio: not found' }
            $problem = ($props | Where-Object { $_.KeyName -eq 'DEVPKEY_Device_ProblemCode' } | Select-Object -First 1).Data
            $devnodeOk = ($dev.Status -eq 'OK' -and "$problem" -eq '0')
        } else {
            Write-RvfLog "  devnode not enumerated ($instanceId)"
            $problem = $null
            $devnodeOk = $false
        }
        if (-not $devnodeOk) {
            try {
                Write-RvfLog '  --- devnode diagnostics: bcdedit / secure boot / CodeIntegrity / setupapi ---'
                $bcd = Invoke-RvfProcess -FilePath 'bcdedit.exe' -Arguments @('/enum', '{current}') -TimeoutSeconds 30
                Write-RvfProcessOutput $bcd '  bcdedit: '
                $events = Get-WinEvent -FilterHashtable @{ LogName = 'Microsoft-Windows-CodeIntegrity/Operational' } -MaxEvents 15 -ErrorAction SilentlyContinue
                foreach ($event in @($events)) { Write-RvfLog "  CodeIntegrity $($event.TimeCreated.ToString('HH:mm:ss')) id=$($event.Id) $(($event.Message -split "`n" | Select-Object -First 1))" }
                $setupLines = Get-Content 'C:\Windows\INF\setupapi.dev.log' -ErrorAction SilentlyContinue |
                    Select-String -Pattern 'RvfVirtualAudio|Ruviform|unsigned|signature' -Context 1,3 |
                    Select-Object -Last 12
                foreach ($entry in $setupLines) { Write-RvfLog "  setupapi: $($entry.Line)"; foreach ($post in $entry.Context.PostContext) { Write-RvfLog "  setupapi: $post" } }
            } catch { Write-RvfLog "  diagnostics error: $($_.Exception.Message)" }
            $terminalStatus = 'error'
            $terminalDetail = 'DEVNODE_CREATE_FAILED'
            Add-RvfStageRecord -State $state -Stage 'devnode' -Status error -Detail "DEVNODE_CREATE_FAILED instance_id=$instanceId; status=$(if ($dev) { $dev.Status } else { 'missing' }); problem_code=$problem; exit=$($devnode.exit_code)" -ExitCode $devnode.exit_code
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        Add-RvfStageRecord -State $state -Stage 'devnode' -Status ok -Detail "instance_id=$instanceId; status=$($dev.Status); problem_code=$problem; service=$($dev.Service)"
        Stop-RvfStageLog

        Start-RvfStageLog 'endpoint-probe'
        try {
            $pairs = @(Invoke-RvfEndpointProbe -State $state)
            Add-RvfStageRecord -State $state -Stage 'endpoint-probe' -Status ok -Detail "pair_count=$($state.endpoint.pair_count); probe_result=$($state.endpoint.probe_result)"
        } catch {
            Write-RvfLog "  endpoint probe error: $($_.Exception.Message)"
            Add-RvfStageRecord -State $state -Stage 'endpoint-probe' -Status error -Detail $_.Exception.Message
        }
        Stop-RvfStageLog

        Start-RvfStageLog 'mic-consent'
        if (-not $AllowMicConsentProvisioning) {
            Add-RvfStageRecord -State $state -Stage 'mic-consent' -Status not_run -Detail 'MIC_CONSENT_NOT_REQUESTED capture readiness blocked by caller policy'
            Stop-RvfStageLog
        } else {
            $consentProblems = @(Set-RvfMicConsent -State $state)
            $serviceProblems = @(Restart-RvfAudioServices -State $state)
            $mutatedOsState = $true
            $allProblems = @($consentProblems + $serviceProblems)
            if ($consentProblems.Count) {
                Add-RvfStageRecord -State $state -Stage 'mic-consent' -Status error -Detail "MIC_CONSENT_FAILED $($consentProblems -join ';')"
            } elseif ($serviceProblems.Count) {
                Add-RvfStageRecord -State $state -Stage 'mic-consent' -Status error -Detail "SERVICE_RESTART_FAILED $($serviceProblems -join ';')"
            } else {
                Add-RvfStageRecord -State $state -Stage 'mic-consent' -Status ok -Detail "consent=Allow; services=Running"
            }
            Stop-RvfStageLog
            if ($allProblems.Count) {
                $terminalStatus = 'error'
                $terminalDetail = if ($consentProblems.Count) { 'MIC_CONSENT_FAILED' } else { 'SERVICE_RESTART_FAILED' }
                throw 'RVF_STAGE_STOP'
            }
        }

        Start-RvfStageLog 'readiness'
        if ($state.endpoint.pair_count -lt 1) {
            $terminalStatus = 'error'
            $terminalDetail = 'ENDPOINT_PAIR_NOT_FOUND'
            Add-RvfStageRecord -State $state -Stage 'readiness' -Status error -Detail 'ENDPOINT_PAIR_NOT_FOUND pair_count=0'
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        if (-not $AllowMicConsentProvisioning) {
            $terminalStatus = 'blocked'
            $terminalDetail = 'MIC_CONSENT_NOT_REQUESTED'
            Add-RvfStageRecord -State $state -Stage 'readiness' -Status blocked -Detail 'MIC_CONSENT_NOT_REQUESTED endpoint pair exists but capture consent provisioning was not allowed'
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        $terminalStatus = 'ok'
        $terminalDetail = 'OK'
        Add-RvfStageRecord -State $state -Stage 'readiness' -Status ok -Detail "endpoint_pair_count=$($state.endpoint.pair_count); capture_readiness=ok"
        Stop-RvfStageLog
    } catch {
        if ($_.Exception.Message -ne 'RVF_STAGE_STOP') {
            $terminalStatus = 'error'
            if ($terminalDetail -eq 'UNSET') { $terminalDetail = 'UNHANDLED_ERROR' }
            Write-RvfLog "unhandled provision error: $($_.Exception.Message) at $($_.InvocationInfo.ScriptName):$($_.InvocationInfo.ScriptLineNumber)"
            Write-RvfLog "stack: $($_.ScriptStackTrace)"
        }
    }

    # Keep unexecuted conceptual stages as blocked. Do not make unmeasured
    # stages look ok, while letting consumers infer the causal stop point
    # from the stage array alone.
    if ($terminalStatus -ne 'ok') {
        $canonicalStages = @('environment', 'source', 'wdk', 'driver-build', 'catalog', 'cert-create', 'cert-install', 'sign', 'sign-verify', 'driver-store', 'devnode', 'endpoint-probe', 'mic-consent', 'readiness')
        foreach ($stageName in $canonicalStages) {
            $existing = @($state.stages | Where-Object { $_.stage -eq $stageName })
            if (-not $existing.Count) {
                Add-RvfStageRecord -State $state -Stage $stageName -Status blocked -Detail "BLOCKED_BY_PREREQUISITE terminal=$terminalStatus/$terminalDetail"
            }
        }
    }

    if ($terminalStatus -ne 'ok' -and $mutatedOsState -and $CleanupOnFailure -eq 'Full') {
        Write-RvfLog 'provision failed after OS state mutation - starting CleanupOnFailure=Full cleanup'
        try {
            $cleanup = Invoke-RvfCleanup -SourceState $state -Mode Full -TimeoutSeconds ([Math]::Min(900, $TimeoutSeconds))
            $state.cleanup = $cleanup
        } catch {
            $state.cleanup = [pscustomobject][ordered]@{
                mode = 'Full'
                stages = @()
                leftovers = @('cleanup-invocation-error')
                errors = @($_.Exception.Message)
            }
        }
    } elseif ($terminalStatus -ne 'ok' -and $CleanupOnFailure -eq 'Preserve') {
        $state.cleanup = [pscustomobject][ordered]@{
            mode = 'Preserve'
            stages = @()
            leftovers = @()
            errors = @()
        }
    }

    $state.status = $terminalStatus
    $state.detail = $terminalDetail
    $state.exit_code = if ($terminalStatus -eq 'ok') { 0 } else { 1 }
    $state.finished_utc = ConvertTo-RvfUtc (Get-Date)
    Write-RvfLog "provision terminal status=$($state.status) detail=$($state.detail) elapsed_s=$([int]((Get-Date) - $started).TotalSeconds)"
    Save-RvfState -State $state -Path $StatePath -EvidenceDir $EvidenceDir
    return $state.exit_code
}

function Invoke-RvfUnprovision {
    param(
        [Parameter(Mandatory)][string]$StatePath,
        [string]$WorkspaceRoot,
        [Parameter(Mandatory)][string]$EvidenceDir,
        [ValidateSet('Full', 'ReportOnly')][string]$CleanupMode = 'Full',
        [int]$TimeoutSeconds = 900
    )

    $StatePath = Resolve-RvfPath $StatePath
    $EvidenceDir = Resolve-RvfPath $EvidenceDir
    if ($WorkspaceRoot) { $WorkspaceRoot = Resolve-RvfPath $WorkspaceRoot }
    Initialize-RvfLog -EvidenceDir $EvidenceDir -Operation 'unprovision'

    $inputs = [pscustomobject][ordered]@{
        state_path      = $StatePath
        workspace_root  = $WorkspaceRoot
        evidence_dir    = $EvidenceDir
        cleanup_mode    = $CleanupMode
        timeout_seconds = $TimeoutSeconds
    }
    $state = New-RvfState -Operation 'unprovision' -Inputs $inputs
    $state.exit_code = 1
    $terminalStatus = 'error'
    $terminalDetail = 'UNSET'
    $started = Get-Date
    $outputStatePath = Join-Path $EvidenceDir 'unprovision-state.json'

    try {
        Start-RvfStageLog 'environment'
        $envInfo = Get-RvfEnvironmentInfo
        Write-RvfEnvironmentSurface $envInfo
        $state.runner.os_caption = $envInfo.os_caption
        $state.runner.os_version = $envInfo.os_version
        $state.runner.os_build = $envInfo.os_build
        $state.runner.arch = $envInfo.arch
        $state.runner.github_actions = ($envInfo.github_actions -eq 'true')
        $state.runner.runner_environment = $envInfo.runner_environment
        $state.runner.admin = $envInfo.admin
        $state.runner.testsigning = $envInfo.testsigning
        $state.runner.secure_boot = $envInfo.secure_boot
        $guard = Test-RvfEphemeralEnvironment $envInfo
        if (-not $guard.ok) {
            $terminalStatus = 'not_run'
            $terminalDetail = $guard.detail
            Add-RvfStageRecord -State $state -Stage 'environment' -Status not_run -Detail "$($guard.detail); $($guard.reason)"
            Stop-RvfStageLog
            throw 'RVF_STAGE_STOP'
        }
        Add-RvfStageRecord -State $state -Stage 'environment' -Status ok -Detail 'ephemeral github-hosted windows-2025 x64 admin surface confirmed'
        Stop-RvfStageLog

        Start-RvfStageLog 'state'
        if (-not (Test-Path -LiteralPath $StatePath)) {
            Add-RvfStageRecord -State $state -Stage 'state' -Status error -Detail 'STATE_INPUT_MISSING'
            Stop-RvfStageLog
            $terminalStatus = 'error'
            $terminalDetail = 'STATE_INPUT_MISSING'
            throw 'RVF_CONTRACT_STOP'
        }
        try {
            $sourceState = Get-Content -LiteralPath $StatePath -Raw -Encoding UTF8 | ConvertFrom-Json -ErrorAction Stop
        } catch {
            Add-RvfStageRecord -State $state -Stage 'state' -Status error -Detail "STATE_INPUT_INVALID $($_.Exception.Message)"
            Stop-RvfStageLog
            $terminalStatus = 'error'
            $terminalDetail = 'STATE_INPUT_INVALID'
            throw 'RVF_CONTRACT_STOP'
        }
        if ($sourceState.schema -ne 'rvf-virtual-audio-provisioning/v1') {
            Add-RvfStageRecord -State $state -Stage 'state' -Status error -Detail 'STATE_SCHEMA_MISMATCH'
            Stop-RvfStageLog
            $terminalStatus = 'error'
            $terminalDetail = 'STATE_SCHEMA_MISMATCH'
            throw 'RVF_CONTRACT_STOP'
        }
        $state.source = $sourceState.source
        $state.package = $sourceState.package
        $state.os_state = $sourceState.os_state
        $state.endpoint = $sourceState.endpoint
        Add-RvfStageRecord -State $state -Stage 'state' -Status ok -Detail "input_status=$($sourceState.status); instance_id=$($sourceState.os_state.instance_id); inf=$($sourceState.os_state.inf_published)"
        Stop-RvfStageLog

        $cleanup = Invoke-RvfCleanup -SourceState $sourceState -Mode $CleanupMode -TimeoutSeconds $TimeoutSeconds
        $state.cleanup = $cleanup
        foreach ($cleanupStage in @($cleanup.stages)) {
            $state.stages = @($state.stages) + $cleanupStage
        }
        if ($cleanup.leftovers.Count -or $cleanup.errors.Count) {
            $terminalStatus = 'error'
            $terminalDetail = 'CLEANUP_RESIDUE'
        } else {
            $terminalStatus = 'ok'
            $terminalDetail = 'OK'
        }
    } catch {
        if ($_.Exception.Message -notin @('RVF_STAGE_STOP', 'RVF_CONTRACT_STOP')) {
            $terminalStatus = 'error'
            if ($terminalDetail -eq 'UNSET') { $terminalDetail = 'UNHANDLED_ERROR' }
            Write-RvfLog "unhandled unprovision error: $($_.Exception.Message) at $($_.InvocationInfo.ScriptName):$($_.InvocationInfo.ScriptLineNumber)"
            Write-RvfLog "stack: $($_.ScriptStackTrace)"
        }
    }

    $state.status = $terminalStatus
    $state.detail = $terminalDetail
    $contractViolation = @('STATE_INPUT_MISSING', 'STATE_INPUT_INVALID', 'STATE_SCHEMA_MISMATCH') -contains $terminalDetail
    $state.exit_code = if ($terminalStatus -eq 'ok') { 0 } elseif ($contractViolation) { 2 } else { 1 }
    $state.finished_utc = ConvertTo-RvfUtc (Get-Date)
    Write-RvfLog "unprovision terminal status=$($state.status) detail=$($state.detail) elapsed_s=$([int]((Get-Date) - $started).TotalSeconds)"
    Save-RvfState -State $state -Path $outputStatePath -EvidenceDir $EvidenceDir
    return $state.exit_code
}

Export-ModuleMember -Function Invoke-RvfProvision, Invoke-RvfUnprovision, Invoke-RvfEndpointProbe, Initialize-RvfMmDeviceProbeType
