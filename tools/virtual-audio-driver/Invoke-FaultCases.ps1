# tools/virtual-audio-driver/Invoke-FaultCases.ps1
# Issue #1002 Slice G — provisioned fault-case controller for the
# virtual-audio-driver-probe workflow. This script is the only place that
# mutates devnode/microphone-consent/service state; Rust tests observe and
# judge through phase/result files. It is invoked only on ephemeral
# GitHub-hosted runners by the workflow.

[CmdletBinding()]
param([Parameter(Mandatory)][string]$Os)

$ErrorActionPreference = 'Stop'
. ./tools/testsigned-driver/probe-stage.ps1
$os = $Os
$json = "vaudio-probe-$os.json"
$state = "vaudio-state-$os.json"
$log = "stage-fault-cases-$os.txt"
$resultsPath = "fault-cases-results-$os.json"
New-Item -ItemType File -Force $log | Out-Null
$script:caseResults = @()
$script:harnessProblems = @()

function Write-FaultLog([string]$line) {
  "  $line" | Out-Probe $log | Out-Null
}

function Wait-FaultFile([string]$path, [int]$seconds) {
  $deadline = (Get-Date).AddSeconds($seconds)
  while ((Get-Date) -lt $deadline) {
    if (Test-Path -LiteralPath $path) { return $true }
    Start-Sleep -Milliseconds 150
  }
  return (Test-Path -LiteralPath $path)
}

function Start-FaultTestJob([string]$filter, [hashtable]$envMap) {
  foreach ($key in $envMap.Keys) { Set-Item -Path "Env:$key" -Value $envMap[$key] }
  try {
    return Start-Job -ScriptBlock {
      param($wd, $testFilter)
      Set-Location $wd
      & mise exec -- cargo test -p ruviform-audio-device --features real-device --test virtual_endpoint $testFilter -- --ignored --test-threads=1 --nocapture 2>&1
    } -ArgumentList "$PWD", $filter
  } finally {
    foreach ($key in $envMap.Keys) { Remove-Item -Path "Env:$key" -ErrorAction SilentlyContinue }
  }
}

function Read-FaultCaseResult([string]$case, [string]$resultFile, [object[]]$output) {
  $line = $null
  if (Test-Path -LiteralPath $resultFile) {
    $line = Get-Content -LiteralPath $resultFile -TotalCount 1 -ErrorAction SilentlyContinue
  }
  if (-not $line) {
    $line = @($output | Where-Object { "$_" -match '^fault_case_result:' }) | Select-Object -Last 1
  }
  if (-not $line) {
    return [ordered]@{ case = $case; result = 'Fail'; expected = 'fault-case-result-line'; observed = 'missing'; detail = 'rust-test-did-not-emit-machine-readable-result' }
  }
  function Get-FaultToken([string]$text, [string]$key) {
    $m = [regex]::Match($text, "(?:^|\s)$key=([^\s]+)")
    if ($m.Success) { return $m.Groups[1].Value }
    return ''
  }
  $raw = Get-FaultToken "$line" 'result'
  $normalized = switch ($raw) {
    'pass' { 'Pass' }
    'fail' { 'Fail' }
    'notrun' { 'NotRun' }
    'unsupported' { 'Unsupported' }
    'inactive' { 'NotRun' }
    default { 'Fail' }
  }
  $detail = Get-FaultToken "$line" 'detail'
  if ($raw -notin @('pass', 'fail', 'notrun', 'unsupported', 'inactive')) {
    $detail = "unrecognized-result-$raw"
  }
  return [ordered]@{
    case = if (Get-FaultToken "$line" 'case') { Get-FaultToken "$line" 'case' } else { $case }
    result = $normalized
    expected = Get-FaultToken "$line" 'expected'
    observed = Get-FaultToken "$line" 'observed'
    detail = $detail
    raw_line = "$line"
  }
}

function Add-FaultCase([object]$caseResult) {
  $script:caseResults += $caseResult
  Write-FaultLog ("case result: case={0} result={1} expected={2} observed={3} detail={4}" -f $caseResult.case, $caseResult.result, $caseResult.expected, $caseResult.observed, $caseResult.detail)
}

function Invoke-MidiWinmmProbe([string]$label) {
  $file = "fault-midi-$label-$os.txt"
  $out = @(& pwsh -NoProfile -NonInteractive -File tools/testsigned-driver/midi-winmm-probe.ps1 2>&1)
  $probeExit = $LASTEXITCODE
  $out | Out-File -FilePath $file -Encoding utf8
  $out | Select-Object -First 80 | ForEach-Object { "    midi[$label] $_" | Out-Probe $log | Out-Null }
  $present = (($out -join "`n") -match 'Ruviform MIDI Loopback')
  Write-FaultLog "midi probe[$label]: exit=$probeExit loopback_present=$present file=$file"
  return [ordered]@{ exit = $probeExit; present = $present; file = $file }
}

function Set-MicConsentState([string]$mode) {
  $micCap = '{E5323777-F976-4f5b-9B55-B94699C46E44}'
  $allow = $mode -eq 'Allow'
  $stringValue = if ($allow) { 'Allow' } else { 'Deny' }
  $policyValue = if ($allow) { 1 } else { 2 }
  $targets = @(
    @{ hive = 'HKLM:'; key = "SOFTWARE\Microsoft\Windows\CurrentVersion\DeviceAccess\Global\$micCap"; name = 'Value'; val = $stringValue; tag = 'hklm-devacc' },
    @{ hive = 'HKCU:'; key = "SOFTWARE\Microsoft\Windows\CurrentVersion\DeviceAccess\Global\$micCap"; name = 'Value'; val = $stringValue; tag = 'hkcu-devacc' },
    @{ hive = 'HKLM:'; key = 'SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone'; name = 'Value'; val = $stringValue; tag = 'hklm-cam' },
    @{ hive = 'HKCU:'; key = 'SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone'; name = 'Value'; val = $stringValue; tag = 'hkcu-cam' },
    @{ hive = 'HKLM:'; key = 'SOFTWARE\Policies\Microsoft\Windows\AppPrivacy'; name = 'LetAppsAccessMicrophone'; val = $policyValue; tag = 'policy' }
  )
  $problems = @()
  foreach ($t in $targets) {
    $key = "$($t.hive)$($t.key)"
    try {
      $before = (Get-ItemProperty -Path $key -Name $t.name -ErrorAction SilentlyContinue).($t.name)
      if (-not (Test-Path $key)) { New-Item -Path $key -Force | Out-Null }
      Set-ItemProperty -Path $key -Name $t.name -Value $t.val -ErrorAction Stop
      $after = (Get-ItemProperty -Path $key -Name $t.name).($t.name)
      Write-FaultLog "mic-consent-$mode [$($t.tag)] $key\$($t.name) : '$before' -> '$after'"
      if ("$after" -ne "$($t.val)") { $problems += "$($t.tag)=$after" }
    } catch {
      Write-FaultLog "mic-consent-$mode [$($t.tag)] error: $($_.Exception.Message)"
      $problems += "$($t.tag)=error"
    }
  }
  return ,$problems
}

function Restart-FaultAudioServices {
  $problems = @()
  foreach ($svc in 'AudioEndpointBuilder', 'Audiosrv') {
    try {
      $before = (Get-Service $svc -ErrorAction SilentlyContinue).Status
      Restart-Service $svc -Force -ErrorAction Stop
      $after = (Get-Service $svc -ErrorAction Stop).Status
      Write-FaultLog "svc $svc : $before -> $after"
      if ($after -ne 'Running') { $problems += "$svc=$after" }
    } catch {
      Write-FaultLog "svc $svc restart error: $($_.Exception.Message)"
      $problems += "$svc=error"
    }
  }
  Start-Sleep -Seconds 2
  return ,$problems
}

function Invoke-TestDeviceState([string]$action, [string]$instanceId) {
  try {
    $out = @(& ./tools/testsigned-driver/TestDevice.ps1 -Action $action -InstanceId $instanceId 2>&1)
    return [ordered]@{ success = $true; output = $out; error = '' }
  } catch {
    return [ordered]@{ success = $false; output = @(); error = $_.Exception.Message }
  }
}

function Invoke-TestDeviceCreate {
  try {
    $inf = "$PWD\vaudio-pkg\RvfVirtualAudio.inf"
    $out = @(& ./tools/testsigned-driver/TestDevice.ps1 -Action create `
      -HardwareId 'ROOT\RvfVirtualAudio' `
      -ClassName 'MEDIA' `
      -ClassGuid '4d36e96c-e325-11ce-bfc1-08002be10318' `
      -DeviceDescription 'Ruviform Virtual Audio Device' `
      -InfPath $inf 2>&1)
    $newId = (($out | Where-Object { $_ -match '^instance_id=' } | Select-Object -First 1) -replace '^instance_id=', '')
    return [ordered]@{ success = [bool]$newId; output = $out; error = ''; instance_id = $newId }
  } catch {
    return [ordered]@{ success = $false; output = @(); error = $_.Exception.Message; instance_id = '' }
  }
}


function Invoke-AudioEndpointProbe([string]$label) {
  $file = "fault-audio-$label-$os.txt"
  $out = @(& ./tools/virtual-audio-driver/Invoke-EndpointProbe.ps1 -OutFile $file 2>&1)
  $probeExit = $LASTEXITCODE
  $pairCount = 0
  foreach ($line in $out) {
    if ("$line" -match '^pair_count=(\d+)') {
      $pairCount = [int]$Matches[1]
      break
    }
  }
  Write-FaultLog "audio probe[$label]: exit=$probeExit pair_count=$pairCount file=$file"
  return [ordered]@{ exit = $probeExit; pair_count = $pairCount; file = $file }
}

function Wait-EndpointSurfaceRecovery([int]$seconds) {
  $deadline = (Get-Date).AddSeconds($seconds)
  $poll = 0
  $midiLast = $null
  $audioLast = $null
  while ((Get-Date) -lt $deadline) {
    $poll += 1
    $midiLast = Invoke-MidiWinmmProbe "recovery-poll-$poll"
    $audioLast = Invoke-AudioEndpointProbe "recovery-poll-$poll"
    if ($midiLast.present -and $audioLast.pair_count -ge 1) {
      Write-FaultLog "endpoint surfaces recovered on poll=$poll"
      return [ordered]@{ success = $true; midi = $midiLast; audio = $audioLast; polls = $poll }
    }
    Start-Sleep -Seconds 3
  }
  Write-FaultLog "endpoint surface recovery timeout after $seconds seconds (midi_present=$($midiLast.present) audio_pair_count=$($audioLast.pair_count))"
  return [ordered]@{ success = $false; midi = $midiLast; audio = $audioLast; polls = $poll }
}

function Get-StageDetail([string]$stage) {
  if (-not (Test-Path $json)) { return '' }
  $records = @(Get-Content -Raw $json | ConvertFrom-Json)
  $match = @($records | Where-Object { $_.stage -eq $stage }) | Select-Object -Last 1
  if ($match) { return "$($match.detail)" }
  return ''
}

$prerequisites = [ordered]@{
  'devnode-create' = Get-StageStatus $json 'devnode-create'
  'endpoint-probe' = Get-StageStatus $json 'endpoint-probe'
  'midi-endpoint-probe' = Get-StageStatus $json 'midi-endpoint-probe'
  'stream-roundtrip' = Get-StageStatus $json 'stream-roundtrip'
  'midi-loopback' = Get-StageStatus $json 'midi-loopback'
  'os-endpoint-wiring' = Get-StageStatus $json 'os-endpoint-wiring'
}
foreach ($k in $prerequisites.Keys) { Write-FaultLog "prerequisite ${k}: $($prerequisites[$k])" }
$iid = Get-ProbeState $state 'instance_id'
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-FaultLog "controller context: instance_id=$iid admin=$isAdmin"
$endpointDetail = Get-StageDetail 'endpoint-probe'
$midiEndpointDetail = Get-StageDetail 'midi-endpoint-probe'
$audioProvisioned = $endpointDetail -match 'pair_count=[1-9]'
$midiProvisioned = $midiEndpointDetail -match 'loopback_candidates=[1-9]'
Write-FaultLog "provisioning details: audio_pair=$audioProvisioned midi_candidates=$midiProvisioned"
$canRun = [bool]$iid -and $isAdmin -and $audioProvisioned -and $midiProvisioned -and $prerequisites['devnode-create'] -eq 'ok' -and $prerequisites['endpoint-probe'] -eq 'ok' -and $prerequisites['midi-endpoint-probe'] -eq 'ok'

if (-not $canRun) {
  foreach ($case in 'device_loss_audio', 'permission_denial_audio', 'midi_device_loss_observation', 'xrun_probe') {
    Add-FaultCase ([ordered]@{ case = $case; result = 'NotRun'; expected = 'provisioned-devnode-admin-controller'; observed = "instance_id=$iid admin=$isAdmin"; detail = 'fault-controller-prerequisites-not-satisfied' })
  }
} else {
  # ---- microphone permission denial ----
  $case = 'permission_denial_audio'
  $observed = "fault-permission-observed-$os.txt"
  $restore = "fault-permission-restore-$os.txt"
  $resultFile = "fault-permission-result-$os.txt"
  Remove-Item $observed, $restore, $resultFile -Force -ErrorAction SilentlyContinue
  Write-FaultLog "--- case $case ---"
  $job = $null
  $jobOutput = @()
  $permissionControllerProblems = @()
  try {
    $denyProblems = Set-MicConsentState 'Deny'
    $restartProblems = Restart-FaultAudioServices
    $permissionControllerProblems += $denyProblems + $restartProblems
    if ($permissionControllerProblems.Count) {
      Write-FaultLog "permission controller setup problems: $($permissionControllerProblems -join ',')"
    }
    $job = Start-FaultTestJob 'virtual_audio_permission_denial_fault' @{
      RUVIFORM_FAULT_PERMISSION_DENIAL = '1'
      RUVIFORM_FAULT_OBSERVED_FILE = "$PWD\$observed"
      RUVIFORM_FAULT_RESTORE_FILE = "$PWD\$restore"
      RUVIFORM_FAULT_RESULT_FILE = "$PWD\$resultFile"
    }
    Write-FaultLog "permission test job id=$($job.Id)"
    $permissionObserved = Wait-FaultFile "$PWD\$observed" 75
    Write-FaultLog "permission observed signal: $permissionObserved"
    if (-not $permissionObserved) { $script:harnessProblems += 'permission-observed-signal-missing' }
  } finally {
    Write-FaultLog 'restoring mic consent Allow and audio services'
    $allowProblems = Set-MicConsentState 'Allow'
    $restartAllowProblems = Restart-FaultAudioServices
    $restoreProblems = @($allowProblems + $restartAllowProblems)
    if ($restoreProblems.Count) {
      Write-FaultLog "permission restore problems: $($restoreProblems -join ',')"
      $script:harnessProblems += 'permission-restore-incomplete'
    } else {
      "restored $([DateTime]::UtcNow.ToString('o'))" | Set-Content -LiteralPath "$PWD\$restore" -Encoding utf8
    }
  }
  $finished = Wait-Job $job -Timeout 50
  if (-not $finished) {
    Write-FaultLog 'permission test job timeout; stopping job'
    Stop-Job $job -ErrorAction SilentlyContinue
    $script:harnessProblems += 'permission-test-timeout'
  }
  $jobOutput = @(Receive-Job $job -ErrorAction SilentlyContinue)
  $jobOutput | ForEach-Object { "    test: $_" | Out-Probe $log }
  if ($job) { Remove-Job $job -Force -ErrorAction SilentlyContinue }
  $permissionResult = Read-FaultCaseResult $case "$PWD\$resultFile" $jobOutput
  if ($permissionControllerProblems.Count -or $restoreProblems.Count) {
    $permissionResult.result = 'Fail'
    $permissionResult.detail = "permission-controller-failure; original=$($permissionResult.detail)"
  }
  Add-FaultCase $permissionResult

  # ---- bounded xrun observation ----
  $case = 'xrun_probe'
  $resultFile = "fault-xrun-result-$os.txt"
  Remove-Item $resultFile -Force -ErrorAction SilentlyContinue
  Write-FaultLog "--- case $case ---"
  $env:RUVIFORM_FAULT_XRUN_PROBE = '1'
  $env:RUVIFORM_FAULT_XRUN_SPIN_THREADS = '2'
  $env:RUVIFORM_FAULT_RESULT_FILE = "$PWD\$resultFile"
  try {
    $xrunOut = @(& mise exec -- cargo test -p ruviform-audio-device --features real-device --test virtual_endpoint virtual_audio_xrun_probe -- --ignored --test-threads=1 --nocapture 2>&1)
    $xrunExit = $LASTEXITCODE
  } finally {
    Remove-Item Env:RUVIFORM_FAULT_XRUN_PROBE, Env:RUVIFORM_FAULT_XRUN_SPIN_THREADS, Env:RUVIFORM_FAULT_RESULT_FILE -ErrorAction SilentlyContinue
  }
  $xrunOut | ForEach-Object { "    test: $_" | Out-Probe $log }
  Write-FaultLog "xrun probe exit=$xrunExit"
  if ($xrunExit -ne 0) { $script:harnessProblems += 'xrun-probe-nonzero-exit' }
  $xrunResult = Read-FaultCaseResult $case "$PWD\$resultFile" $xrunOut
  Add-FaultCase $xrunResult
  # ---- device-loss audio + MIDI disappearance observation ----
  $case = 'device_loss_audio'
  $ready = "fault-device-loss-ready-$os.txt"
  $observed = "fault-device-loss-observed-$os.txt"
  $restore = "fault-device-loss-restore-$os.txt"
  $resultFile = "fault-device-loss-result-$os.txt"
  Remove-Item $ready, $observed, $restore, $resultFile -Force -ErrorAction SilentlyContinue
  Write-FaultLog "--- case $case ---"
  $midiBefore = Invoke-MidiWinmmProbe 'before-disable'
  $disableAttempted = $false
  $stimulus = 'none'
  $enableOk = $false
  $observedSignal = $false
  $job = $null
  $jobOutput = @()
  try {
    $job = Start-FaultTestJob 'virtual_audio_device_loss_fault' @{
      RUVIFORM_FAULT_DEVICE_LOSS = '1'
      RUVIFORM_FAULT_READY_FILE = "$PWD\$ready"
      RUVIFORM_FAULT_OBSERVED_FILE = "$PWD\$observed"
      RUVIFORM_FAULT_RESTORE_FILE = "$PWD\$restore"
      RUVIFORM_FAULT_RESULT_FILE = "$PWD\$resultFile"
    }
    Write-FaultLog "device-loss test job id=$($job.Id)"
    if (Wait-FaultFile "$PWD\$ready" 60) {
      Write-FaultLog 'ready signal observed; disabling devnode'
      $disableInvoke = Invoke-TestDeviceState 'disable' $iid
      $disableInvoke.output | ForEach-Object { "    disable: $_" | Out-Probe $log | Out-Null }
      if ($disableInvoke.error) { Write-FaultLog "disable error: $($disableInvoke.error)" }
      Write-FaultLog "disable success=$($disableInvoke.success)"
      $disableAttempted = [bool]$disableInvoke.success
      if ($disableAttempted) {
        $stimulus = 'disable-enable'
      } else {
        # Disable-PnpDevice/pnputil can refuse a devnode while audio
        # streams are open. In that case fall back to a true PnP
        # removal while the streams are live, then recreate the same
        # root-enumerated devnode after the test has closed them.
        # The preferred disable/enable result and this fallback are
        # distinguished in the log and case detail.
        Write-FaultLog 'disable failed; attempting remove/recreate fallback while streams are live'
        $removeInvoke = Invoke-TestDeviceState 'remove' $iid
        $removeInvoke.output | ForEach-Object { "    remove-fallback: $_" | Out-Probe $log | Out-Null }
        if ($removeInvoke.error) { Write-FaultLog "remove fallback error: $($removeInvoke.error)" }
        Write-FaultLog "remove fallback success=$($removeInvoke.success)"
        if ($removeInvoke.success) {
          $disableAttempted = $true
          $stimulus = 'remove-recreate'
        } else {
          $script:harnessProblems += 'device-loss-stimulus-failed'
        }
      }
      if ($disableAttempted) {
        $midiDuring = Invoke-MidiWinmmProbe 'during-disable'
      } else {
        $midiDuring = [ordered]@{ exit = 1; present = $true; file = 'not-run' }
      }
      $observedSignal = Wait-FaultFile "$PWD\$observed" 35
      Write-FaultLog "observed signal: $observedSignal"
      if ($stimulus -eq 'disable-enable') {
        $enableInvoke = Invoke-TestDeviceState 'enable' $iid
        $enableInvoke.output | ForEach-Object { "    enable: $_" | Out-Probe $log | Out-Null }
        if ($enableInvoke.error) { Write-FaultLog "enable error: $($enableInvoke.error)" }
        Write-FaultLog "enable success=$($enableInvoke.success)"
        $enableOk = [bool]$enableInvoke.success
        if (-not $enableOk) { $script:harnessProblems += 'device-loss-enable-failed' }
      } elseif ($stimulus -eq 'remove-recreate') {
        $createInvoke = Invoke-TestDeviceCreate
        $createInvoke.output | ForEach-Object { "    recreate: $_" | Out-Probe $log | Out-Null }
        if ($createInvoke.error) { Write-FaultLog "recreate error: $($createInvoke.error)" }
        Write-FaultLog "recreate success=$($createInvoke.success) new_instance_id=$($createInvoke.instance_id)"
        $enableOk = [bool]$createInvoke.success
        if ($enableOk) {
          $iid = $createInvoke.instance_id
          Set-ProbeState $state 'instance_id' $iid
        } else {
          $script:harnessProblems += 'device-loss-recreate-failed'
        }
      }
      if ($enableOk) {
        "restored stimulus=$stimulus $([DateTime]::UtcNow.ToString('o'))" | Set-Content -LiteralPath "$PWD\$restore" -Encoding utf8
        if ($stimulus -eq 'remove-recreate') {
          # Recreation returns after driver binding, before PortCls/WinMM and
          # the MMDevice endpoint surface necessarily finish re-enumerating.
          # Restart the user-mode audio services once, then poll both surfaces
          # instead of mistaking that asynchronous setup window for a negative
          # recovery measurement.
          $recoveryServiceProblems = Restart-FaultAudioServices
          if ($recoveryServiceProblems.Count) {
            Write-FaultLog "recovery service restart problems: $($recoveryServiceProblems -join ',')"
            $script:harnessProblems += 'device-loss-recovery-service-restart-failed'
          }
        }
        $surfaceRecovery = Wait-EndpointSurfaceRecovery 45
        if (-not $surfaceRecovery.success) {
          Write-FaultLog 'endpoint surfaces did not both re-enumerate within the bounded recovery window'
        }
        $midiAfter = $surfaceRecovery.midi
      } else {
        $midiAfter = Invoke-MidiWinmmProbe 'after-enable'
      }
    } else {
      Write-FaultLog 'ready signal timeout — devnode was not disabled'
      $script:harnessProblems += 'device-loss-ready-timeout'
      $midiDuring = [ordered]@{ exit = 0; present = $true; file = 'not-run' }
      $midiAfter = [ordered]@{ exit = 0; present = $true; file = 'not-run' }
    }
    $finished = Wait-Job $job -Timeout 50
    if (-not $finished) {
      Write-FaultLog 'device-loss test job timeout; stopping job'
      Stop-Job $job -ErrorAction SilentlyContinue
      $script:harnessProblems += 'device-loss-test-timeout'
    }
    $jobOutput = @(Receive-Job $job -ErrorAction SilentlyContinue)
    $jobOutput | ForEach-Object { "    test: $_" | Out-Probe $log }
  } finally {
    if ($disableAttempted -and -not $enableOk) {
      if ($stimulus -eq 'remove-recreate') {
        Write-FaultLog 'finally restore: attempting devnode recreate'
        $finallyInvoke = Invoke-TestDeviceCreate
      } else {
        Write-FaultLog 'finally restore: attempting devnode enable'
        $finallyInvoke = Invoke-TestDeviceState 'enable' $iid
      }
      $finallyInvoke.output | ForEach-Object { "    finally-restore: $_" | Out-Probe $log | Out-Null }
      if ($finallyInvoke.error) { Write-FaultLog "finally-restore error: $($finallyInvoke.error)" }
      if ($finallyInvoke.success) {
        $enableOk = $true
        if ($stimulus -eq 'remove-recreate' -and $finallyInvoke.instance_id) {
          $iid = $finallyInvoke.instance_id
          Set-ProbeState $state 'instance_id' $iid
        }
        "restored-by-finally stimulus=$stimulus $([DateTime]::UtcNow.ToString('o'))" | Set-Content -LiteralPath "$PWD\$restore" -Encoding utf8
      } else {
        $script:harnessProblems += 'device-loss-finally-restore-failed'
      }
    }
    if ($job) { Remove-Job $job -Force -ErrorAction SilentlyContinue }
  }
  if (-not $observedSignal) { $script:harnessProblems += 'device-loss-observed-signal-missing' }
  if (-not $enableOk) { $script:harnessProblems += 'device-loss-restoration-incomplete' }
  $deviceResult = Read-FaultCaseResult $case "$PWD\$resultFile" $jobOutput
  $deviceResult.detail = "stimulus=$stimulus; $($deviceResult.detail)"
  if (-not $enableOk -or -not $observedSignal) {
    $deviceResult.result = 'Fail'
    $deviceResult.detail = "controller-or-synchronization-failure; stimulus=$stimulus; original=$($deviceResult.detail)"
  }
  Add-FaultCase $deviceResult

  $midiObserved = "before=$($midiBefore.present)/$($midiBefore.exit); during=$($midiDuring.present)/$($midiDuring.exit); after=$($midiAfter.present)/$($midiAfter.exit)"
  if (-not $disableAttempted) {
    $midiResult = 'NotRun'
    $midiDetail = 'devnode-loss-stimulus-was-not-applied'
  } elseif ($midiBefore.exit -eq 0 -and $midiDuring.exit -eq 0 -and $midiAfter.exit -eq 0) {
    if ($midiBefore.present -and -not $midiDuring.present) {
      $midiResult = 'Pass'
      $midiDetail = if ($midiAfter.present) {
        "ruviform-midi-loopback-disappeared-during-$stimulus-and-reappeared-after-restore"
      } else {
        "ruviform-midi-loopback-disappeared-during-$stimulus; reappearance-not-observed-in-bounded-recovery"
      }
    } else {
      $midiResult = 'Fail'
      $midiDetail = 'port-disappearance-oracle-not-met'
    }
  } else {
    $midiResult = 'Fail'
    $midiDetail = 'midi-winmm-probe-measurement-failed'
    $script:harnessProblems += 'midi-probe-nonzero-exit'
  }
  Add-FaultCase ([ordered]@{
    case = 'midi_device_loss_observation'
    result = $midiResult
    expected = 'present-before+absent-during; reappearance-recorded'
    observed = $midiObserved
    detail = $midiDetail
  })

}

$enrichedResults = @(foreach ($caseResult in $script:caseResults) {
  [ordered]@{
    schema_version = 'rf1002-fault-case/v1'
    repo = "$env:GITHUB_REPOSITORY"
    sha = "$env:GITHUB_SHA"
    run_id = "$env:GITHUB_RUN_ID"
    run_attempt = "$env:GITHUB_RUN_ATTEMPT"
    os = $os
    case = $caseResult.case
    result = $caseResult.result
    expected = $caseResult.expected
    observed = $caseResult.observed
    detail = $caseResult.detail
    raw_line = $caseResult.raw_line
    recorded_utc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
  }
})
ConvertTo-Json -InputObject $enrichedResults -Depth 5 | Set-Content -Path $resultsPath -Encoding utf8
Write-FaultLog "machine-readable case results: $resultsPath"
if ($script:harnessProblems.Count) {
  Write-FaultLog "harness problems: $($script:harnessProblems -join '; ')"
}

$failedCases = @($script:caseResults | Where-Object { $_.result -eq 'Fail' })
$notRunCases = @($script:caseResults | Where-Object { $_.result -eq 'NotRun' })
if (-not $canRun) {
  $stageStatus = 'not_run'
} elseif ($script:harnessProblems.Count -or $failedCases.Count) {
  $stageStatus = 'error'
} else {
  $stageStatus = 'ok'
}
$caseSummary = ($script:caseResults | ForEach-Object { "$($_.case)=$($_.result)" }) -join ','
$detail = "cases=$caseSummary; harness_problems=$($script:harnessProblems.Count)"
if ($notRunCases.Count -eq $script:caseResults.Count -and $canRun) { $detail += '; all_cases_not_run' }
Add-StageRecord -JsonPath $json -Stage fault-cases -Status $stageStatus -Detail $detail
