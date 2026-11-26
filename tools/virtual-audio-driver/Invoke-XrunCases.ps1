# tools/virtual-audio-driver/Invoke-XrunCases.ps1
# Issue #1009 — deterministic xrun case controller for the provisioned
# virtual-audio-driver-probe workflow. The controller owns registry validation,
# write/cleanup, process lifetime, raw stdout/stderr capture, and final
# rf1009-xrun-case/v1 aggregation. Rust harness code only observes streams.

[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Os,
  [string]$OutputDir = "$PWD",
  [int]$StartPacket = 20,
  [int]$PacketCount = 3,
  [switch]$ValidateOnly
)

$ErrorActionPreference = 'Stop'
. ./tools/testsigned-driver/probe-stage.ps1

$script:RegistryServiceRoot = 'SYSTEM\CurrentControlSet\Services\RvfVirtualAudio'
$script:RegistrySubKey = "$script:RegistryServiceRoot\Parameters"
$script:RegistryPath = "HKLM:\$script:RegistrySubKey"
$script:XrunValues = @('RvfXrunScenario', 'RvfXrunStartPacket', 'RvfXrunPacketCount')
$script:CaseDefinitions = @(
  [ordered]@{ name = 'capture-overflow-drop-input'; test = 'virtual_audio_xrun_capture_overflow_drop_input'; scenario = 1; scenario_name = 'capture-overflow-drop' },
  [ordered]@{ name = 'capture-content-gap-input-negative'; test = 'virtual_audio_xrun_capture_content_gap_input_negative'; scenario = 2; scenario_name = 'capture-content-gap' }
)

New-Item -ItemType Directory -Force $OutputDir | Out-Null
$script:OutputRoot = (Resolve-Path -LiteralPath $OutputDir).Path
$script:ControllerLog = Join-Path $script:OutputRoot "xrun-controller-$Os.txt"
New-Item -ItemType File -Force $script:ControllerLog | Out-Null
$script:HarnessProblems = @()
$script:CaseResults = @()

function Write-XrunLog([string]$line) {
  "  $line" | Out-Probe $script:ControllerLog | Out-Null
}

function Test-XrunScenarioParameters([int]$scenario, [int]$start, [int]$count) {
  if ($scenario -notin @(1, 2)) { return [ordered]@{ valid = $false; reason = "unknown-scenario-$scenario" } }
  if ($start -lt 0 -or $start -gt 4096) { return [ordered]@{ valid = $false; reason = "start-packet-out-of-range-$start" } }
  if ($count -lt 1) { return [ordered]@{ valid = $false; reason = "packet-count-below-minimum-$count" } }
  if ($count -gt 16) { return [ordered]@{ valid = $false; reason = "packet-count-above-maximum-$count" } }
  return [ordered]@{ valid = $true; reason = 'valid' }
}

function Get-MapValue($map, [string]$key) {
  if ($null -eq $map) { return $null }
  if ($map -is [System.Collections.IDictionary]) {
    if ($map.Contains($key)) { return $map[$key] }
    return $null
  }
  $property = $map.PSObject.Properties[$key]
  if ($property) { return $property.Value }
  return $null
}

function Get-XrunRegistryValueState([string]$name) {
  $key = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($script:RegistrySubKey, $false)
  if ($null -eq $key) {
    return [ordered]@{ Present = $false; Value = $null; Kind = $null; KeyMissing = $true }
  }
  try {
    $value = $key.GetValue($name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    if ($null -eq $value) {
      return [ordered]@{ Present = $false; Value = $null; Kind = $null; KeyMissing = $false }
    }
    return [ordered]@{ Present = $true; Value = $value; Kind = $key.GetValueKind($name); KeyMissing = $false }
  } finally {
    $key.Close()
  }
}

function Format-XrunRegistryState([object]$state) {
  if (-not $state.Present) { return 'absent' }
  return "present kind=$($state.Kind) value=$($state.Value)"
}

function Write-XrunScenarioRegistry([int]$scenario, [int]$start, [int]$count) {
  $before = [ordered]@{}
  foreach ($name in $script:XrunValues) {
    $before[$name] = Get-XrunRegistryValueState $name
    Write-XrunLog "registry before $name=$(Format-XrunRegistryState $before[$name])"
  }
  $result = [ordered]@{ attempted = $true; applied = $false; before = $before; error = '' }
  try {
    $key = [Microsoft.Win32.Registry]::LocalMachine.CreateSubKey($script:RegistrySubKey, $true)
    try {
      $key.SetValue('RvfXrunScenario', $scenario, [Microsoft.Win32.RegistryValueKind]::DWord)
      $key.SetValue('RvfXrunStartPacket', $start, [Microsoft.Win32.RegistryValueKind]::DWord)
      $key.SetValue('RvfXrunPacketCount', $count, [Microsoft.Win32.RegistryValueKind]::DWord)
    } finally {
      $key.Close()
    }
    $after = [ordered]@{}
    $ok = $true
    foreach ($name in $script:XrunValues) {
      $after[$name] = Get-XrunRegistryValueState $name
      Write-XrunLog "registry after-write $name=$(Format-XrunRegistryState $after[$name])"
      if (-not $after[$name].Present -or $after[$name].Kind -ne [Microsoft.Win32.RegistryValueKind]::DWord) { $ok = $false }
    }
    if ([int]$after['RvfXrunScenario'].Value -ne $scenario -or [int]$after['RvfXrunStartPacket'].Value -ne $start -or [int]$after['RvfXrunPacketCount'].Value -ne $count) { $ok = $false }
    $result.applied = $ok
    if (-not $ok) { $result.error = 'registry-write-verification-failed' }
  } catch {
    $result.error = $_.Exception.Message
  }
  Write-XrunLog "registry write scenario=$scenario start=$start count=$count applied=$($result.applied) error=$($result.error)"
  return $result
}

function Restore-XrunScenarioRegistry([object]$before) {
  $errors = @()
  try {
    $key = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($script:RegistrySubKey, $true)
    if ($null -eq $key) {
      $errors += 'parameters-key-missing-during-cleanup'
    } else {
      try {
        foreach ($name in $script:XrunValues) {
          $state = $before[$name]
          if ($state.Present) {
            $key.SetValue($name, $state.Value, $state.Kind)
          } else {
            $key.DeleteValue($name, $false)
          }
        }
      } finally {
        $key.Close()
      }
    }
  } catch {
    $errors += $_.Exception.Message
  }
  foreach ($name in $script:XrunValues) {
    $state = $before[$name]
    $after = Get-XrunRegistryValueState $name
    Write-XrunLog "registry after-cleanup $name=$(Format-XrunRegistryState $after)"
    if ($state.Present) {
      if (-not $after.Present -or $after.Kind -ne $state.Kind -or "$($after.Value)" -ne "$($state.Value)") {
        $errors += "$name-restore-mismatch"
      }
    } elseif ($after.Present) {
      $errors += "$name-delete-mismatch"
    }
  }
  if ($errors.Count) {
    Write-XrunLog "registry cleanup clear_failed: $($errors -join ';')"
    return [ordered]@{ state = 'clear_failed'; errors = $errors }
  }
  Write-XrunLog 'registry cleanup cleared'
  return [ordered]@{ state = 'cleared'; errors = @() }
}

function Test-XrunPreflight {
  $problems = @()
  $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
  if (-not $isAdmin) { $problems += 'administrator-required' }
  if (-not (Test-Path "HKLM:\$script:RegistryServiceRoot")) { $problems += 'driver-service-key-missing' }
  if (-not (Get-Command mise -ErrorAction SilentlyContinue)) { $problems += 'mise-not-found' } else {
    $cargoVersion = @(& mise exec -- cargo --version 2>&1)
    if ($LASTEXITCODE -ne 0) { $problems += 'cargo-through-mise-unavailable' }
    Write-XrunLog "preflight cargo: $($cargoVersion -join ' ') exit=$LASTEXITCODE"
  }

  $probeFile = Join-Path $script:OutputRoot "endpoint-probe-xrun-$Os.txt"
  $probeOut = @(& ./tools/virtual-audio-driver/Invoke-EndpointProbe.ps1 -OutFile $probeFile 2>&1)
  $probeExit = $LASTEXITCODE
  $probeText = (($probeOut | ForEach-Object { "$_" }) -join "`n")
  if (Test-Path -LiteralPath $probeFile) { $probeText += "`n" + (Get-Content -LiteralPath $probeFile -Raw -Encoding UTF8) }
  Write-XrunLog "preflight endpoint probe exit=$probeExit file=$probeFile"
  $exactPair = $probeText -match 'render "Ruviform Cable Input \(Ruviform Virtual Audio Device\)"[^\r\n]*capture "Ruviform Cable Output \(Ruviform Virtual Audio Device\)"[^\r\n]*stem="ruviform cable ruviform virtual audio device"'
  if ($probeExit -ne 0) { $problems += 'endpoint-probe-failed' }
  if (-not $exactPair) { $problems += 'ruviform-cable-endpoint-pair-missing' }
  return [ordered]@{ ok = ($problems.Count -eq 0); problems = $problems; admin = $isAdmin; probe_file = $probeFile }
}

function Read-XrunHarnessObservation([string]$resultFile) {
  if (-not (Test-Path -LiteralPath $resultFile)) {
    return [ordered]@{ ok = $false; problem = 'result-file-missing'; observation = $null }
  }
  try {
    $observation = Get-Content -LiteralPath $resultFile -Raw -Encoding UTF8 | ConvertFrom-Json
  } catch {
    return [ordered]@{ ok = $false; problem = "json-parse-failed:$($_.Exception.Message)"; observation = $null }
  }
  $problems = @()
  if ((Get-MapValue $observation 'schema') -ne 'rf1009-xrun-observation/v1') { $problems += 'schema-mismatch' }
  foreach ($key in 'case', 'endpoint', 'injection', 'oracle') {
    if ($null -eq (Get-MapValue $observation $key)) { $problems += "missing-$key" }
  }
  $oracle = Get-MapValue $observation 'oracle'
  foreach ($key in 'typed_xruns', 'clock_discontinuities', 'stream_invalidated', 'backend_errors', 'input_callbacks', 'output_callbacks', 'forward_frame_position_gaps', 'max_forward_gap_frames', 'sample_gap_detected', 'max_callback_stall_ms') {
    if ($null -eq (Get-MapValue $oracle $key)) { $problems += "missing-oracle-$key" }
  }
  if ($problems.Count) {
    return [ordered]@{ ok = $false; problem = ($problems -join ';'); observation = $observation }
  }
  return [ordered]@{ ok = $true; problem = ''; observation = $observation }
}

function Get-ObservationValue($observation, [string]$path, $default) {
  $current = $observation
  foreach ($part in ($path -split '\.')) {
    $current = Get-MapValue $current $part
    if ($null -eq $current) { return $default }
  }
  return $current
}

function Test-XrunFinalSchema($caseObject) {
  $problems = @()
  if ((Get-MapValue $caseObject 'schema') -ne 'rf1009-xrun-case/v1') { $problems += 'schema' }
  foreach ($path in @(
    'run.id', 'run.attempt', 'run.repo', 'run.sha', 'run.os', 'run.recorded_utc',
    'case.name', 'case.direction', 'case.result', 'case.detail',
    'endpoint.render_name', 'endpoint.capture_name', 'endpoint.capture_instance_id', 'endpoint.target_pin',
    'injection.control_surface', 'injection.scenario', 'injection.start_packet', 'injection.packet_count', 'injection.applied', 'injection.cleanup', 'injection.reload_boundary',
    'oracle.typed_xruns', 'oracle.clock_discontinuities', 'oracle.stream_invalidated', 'oracle.backend_errors', 'oracle.input_callbacks', 'oracle.output_callbacks', 'oracle.forward_frame_position_gaps', 'oracle.max_forward_gap_frames', 'oracle.sample_gap_detected', 'oracle.max_callback_stall_ms',
    'artifacts.raw_stdout', 'artifacts.raw_stderr', 'artifacts.controller_log'
  )) {
    if ($null -eq (Get-ObservationValue $caseObject $path $null)) { $problems += "missing-$path" }
  }
  if ((Get-ObservationValue $caseObject 'case.result' '') -notin @('Pass', 'Fail', 'NotRun', 'Unsupported')) { $problems += 'bad-case-result' }
  if ((Get-ObservationValue $caseObject 'injection.cleanup' '') -notin @('cleared', 'clear_failed', 'not_applicable')) { $problems += 'bad-cleanup-vocabulary' }
  if ((Get-ObservationValue $caseObject 'case.detail' '') -eq '') { $problems += 'empty-detail' }
  return ,$problems
}

function New-XrunFinalCase([string]$caseName, [string]$scenarioName, [string]$result, [string]$detail, [bool]$applied, [string]$cleanup, $observation, [string]$stdoutName, [string]$stderrName, [string]$controllerName) {
  $runId = if ($env:GITHUB_RUN_ID) { $env:GITHUB_RUN_ID } else { 'not-applicable-local' }
  $attempt = if ($env:GITHUB_RUN_ATTEMPT) { $env:GITHUB_RUN_ATTEMPT } else { 'not-applicable-local' }
  $repo = if ($env:RUVIFORM_RUN_REPO) { $env:RUVIFORM_RUN_REPO } elseif ($env:GITHUB_REPOSITORY) { $env:GITHUB_REPOSITORY } else { 'unknown-local' }
  $sha = if ($env:RUVIFORM_RUN_SHA) { $env:RUVIFORM_RUN_SHA } elseif ($env:GITHUB_SHA) { $env:GITHUB_SHA } else { 'unknown-local' }
  return [ordered]@{
    schema = 'rf1009-xrun-case/v1'
    run = [ordered]@{
      id = "$runId"; attempt = "$attempt"; repo = "$repo"; sha = "$sha"; os = $Os
      recorded_utc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
    }
    case = [ordered]@{ name = $caseName; direction = 'input'; result = $result; detail = $detail }
    endpoint = [ordered]@{
      render_name = "$(Get-ObservationValue $observation 'endpoint.render_name' '')"
      capture_name = "$(Get-ObservationValue $observation 'endpoint.capture_name' '')"
      capture_instance_id = "$(Get-ObservationValue $observation 'endpoint.capture_instance_id' '')"
      target_pin = 'loopback-capture'
    }
    injection = [ordered]@{
      control_surface = 'static-registry-stream-snapshot'; scenario = $scenarioName
      start_packet = $StartPacket; packet_count = $PacketCount; applied = $applied
      cleanup = $cleanup; reload_boundary = 'stream-open'
    }
    oracle = [ordered]@{
      typed_xruns = [int](Get-ObservationValue $observation 'oracle.typed_xruns' 0)
      clock_discontinuities = [int](Get-ObservationValue $observation 'oracle.clock_discontinuities' 0)
      stream_invalidated = [int](Get-ObservationValue $observation 'oracle.stream_invalidated' 0)
      backend_errors = [int](Get-ObservationValue $observation 'oracle.backend_errors' 0)
      input_callbacks = [int](Get-ObservationValue $observation 'oracle.input_callbacks' 0)
      output_callbacks = [int](Get-ObservationValue $observation 'oracle.output_callbacks' 0)
      forward_frame_position_gaps = [int](Get-ObservationValue $observation 'oracle.forward_frame_position_gaps' 0)
      max_forward_gap_frames = [int](Get-ObservationValue $observation 'oracle.max_forward_gap_frames' 0)
      sample_gap_detected = [bool](Get-ObservationValue $observation 'oracle.sample_gap_detected' $false)
      max_callback_stall_ms = [int](Get-ObservationValue $observation 'oracle.max_callback_stall_ms' 0)
    }
    artifacts = [ordered]@{
      raw_stdout = $stdoutName; raw_stderr = $stderrName; controller_log = $controllerName
      driver_trace = $null; pcm_capture = $null
    }
  }
}

function Stop-XrunProcessTree([int]$processId) {
  $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId=$processId" -ErrorAction SilentlyContinue)
  foreach ($child in $children) {
    Stop-XrunProcessTree ([int]$child.ProcessId)
  }
  Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
}

function Invoke-XrunHarnessCase([object]$definition) {
  $caseName = $definition.name
  $stdoutFile = Join-Path $script:OutputRoot "xrun-$caseName-$Os.stdout.txt"
  $stderrFile = Join-Path $script:OutputRoot "xrun-$caseName-$Os.stderr.txt"
  $resultFile = Join-Path $script:OutputRoot "xrun-$caseName-$Os.result.json"
  Remove-Item $stdoutFile, $stderrFile, $resultFile -Force -ErrorAction SilentlyContinue
  New-Item -ItemType File -Force $stdoutFile | Out-Null
  New-Item -ItemType File -Force $stderrFile | Out-Null
  $stdoutName = [IO.Path]::GetFileName($stdoutFile)
  $stderrName = [IO.Path]::GetFileName($stderrFile)
  $controllerName = [IO.Path]::GetFileName($script:ControllerLog)

  $validation = Test-XrunScenarioParameters ([int]$definition.scenario) $StartPacket $PacketCount
  if (-not $validation.valid) {
    Write-XrunLog "case $caseName validation failed: $($validation.reason); registry not written"
    return New-XrunFinalCase $caseName $definition.scenario_name 'NotRun' "scenario-validation-failed:$($validation.reason)" $false 'not_applicable' $null $stdoutName $stderrName $controllerName
  }

  Write-XrunLog "--- case $caseName scenario=$($definition.scenario) start=$StartPacket count=$PacketCount ---"
  $write = [ordered]@{ attempted = $false; applied = $false; before = $null; error = 'not-attempted' }
  $cleanup = [ordered]@{ state = 'not_applicable'; errors = @() }
  $process = $null
  $measurementStarted = $false
  try {
    $write = Write-XrunScenarioRegistry ([int]$definition.scenario) $StartPacket $PacketCount
    if (-not $write.applied) {
      Write-XrunLog "case $caseName registry write not applied: $($write.error)"
    } else {
      foreach ($pair in @{
        RUVIFORM_RF1009_XRUN_CASE = $caseName
        RUVIFORM_RF1009_XRUN_RESULT_FILE = $resultFile
        RUVIFORM_RF1009_INJECTION_SCENARIO = "$($definition.scenario)"
        RUVIFORM_RF1009_INJECTION_START_PACKET = "$StartPacket"
        RUVIFORM_RF1009_INJECTION_PACKET_COUNT = "$PacketCount"
      }.GetEnumerator()) { Set-Item -Path "Env:$($pair.Key)" -Value $pair.Value }
      try {
        $mise = (Get-Command mise -ErrorAction Stop).Source
        $args = @('exec', '--', 'cargo', 'test', '-p', 'ruviform-audio-device', '--features', 'real-device', '--test', 'virtual_endpoint', $definition.test, '--', '--ignored', '--test-threads=1', '--nocapture')
        $process = Start-Process -FilePath $mise -ArgumentList $args -WorkingDirectory "$PWD" -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile -NoNewWindow -PassThru
        $measurementStarted = $true
        Write-XrunLog "case $caseName harness process id=$($process.Id) started after registry write"
      } finally {
        Remove-Item Env:RUVIFORM_RF1009_XRUN_CASE, Env:RUVIFORM_RF1009_XRUN_RESULT_FILE, Env:RUVIFORM_RF1009_INJECTION_SCENARIO, Env:RUVIFORM_RF1009_INJECTION_START_PACKET, Env:RUVIFORM_RF1009_INJECTION_PACKET_COUNT -ErrorAction SilentlyContinue
      }
      if ($process) {
        Wait-Process -Id $process.Id -Timeout 150 -ErrorAction SilentlyContinue
        $process.Refresh()
        if (-not $process.HasExited) {
          Write-XrunLog "case $caseName harness timeout; stopping process"
          Stop-XrunProcessTree ([int]$process.Id)
          $script:HarnessProblems += "$caseName-harness-timeout"
          $process.Refresh()
        }
        $exitCode = $null
        if ($process.HasExited) {
          try { $exitCode = $process.ExitCode } catch { $exitCode = 'unavailable' }
        }
        Write-XrunLog "case $caseName process exited=$($process.HasExited) exit_code=$exitCode"
      }
    }
  } finally {
    if ($process -and -not $process.HasExited) {
      Stop-XrunProcessTree ([int]$process.Id)
    }
    if ($measurementStarted) {
      Write-XrunLog "stream close/open boundary: harness process ended before registry cleanup; reload boundary=stream-open"
    }
    if ($write.attempted) {
      $cleanup = Restore-XrunScenarioRegistry $write.before
      if ($cleanup.state -eq 'clear_failed') { $script:HarnessProblems += "$caseName-registry-clear-failed" }
    }
  }

  if (-not $write.applied) {
    return New-XrunFinalCase $caseName $definition.scenario_name 'NotRun' "registry-write-not-applied:$($write.error)" $false $cleanup.state $null $stdoutName $stderrName $controllerName
  }
  if (-not $measurementStarted) {
    return New-XrunFinalCase $caseName $definition.scenario_name 'NotRun' 'harness-process-did-not-start' $true $cleanup.state $null $stdoutName $stderrName $controllerName
  }

  $read = Read-XrunHarnessObservation $resultFile
  $observation = $read.observation
  if (-not $read.ok) {
    $script:HarnessProblems += "$caseName-$($read.problem)"
    return New-XrunFinalCase $caseName $definition.scenario_name 'Fail' "controller-harness-problem:$($read.problem)" $true $cleanup.state $observation $stdoutName $stderrName $controllerName
  }
  $result = "$(Get-ObservationValue $observation 'case.result' '')"
  if ($result -notin @('Pass', 'Fail', 'NotRun', 'Unsupported')) {
    $script:HarnessProblems += "$caseName-unrecognized-result"
    $result = 'Fail'
  }
  $detail = "$(Get-ObservationValue $observation 'case.detail' 'missing-detail')"
  if ($detail -eq '') { $detail = 'empty-detail' }
  if ($process -and $process.HasExited -and $process.ExitCode -ne 0) {
    $script:HarnessProblems += "$caseName-harness-exit-$($process.ExitCode)"
    if ($result -eq 'Pass') {
      $result = 'Fail'
      $detail = "harness-nonzero-exit-$($process.ExitCode); original=$detail"
    }
  }
  if ($cleanup.state -eq 'clear_failed' -and $result -eq 'Pass') {
    $result = 'Fail'
    $detail = "registry-cleanup-failed; original=$detail"
  }
  return New-XrunFinalCase $caseName $definition.scenario_name $result $detail $true $cleanup.state $observation $stdoutName $stderrName $controllerName
}

function Invoke-XrunValidateOnly {
  $checks = @()
  foreach ($item in @(
    @{ s = 1; st = 20; c = 3; expect = $true; label = 'valid-positive-scenario' },
    @{ s = 2; st = 20; c = 3; expect = $true; label = 'valid-negative-scenario' },
    @{ s = 99; st = 20; c = 3; expect = $false; label = 'unknown-scenario' },
    @{ s = 1; st = 4096; c = 3; expect = $true; label = 'start-upper-limit' },
    @{ s = 1; st = 4097; c = 3; expect = $false; label = 'start-above-limit' },
    @{ s = 1; st = 20; c = 1; expect = $true; label = 'count-lower-limit' },
    @{ s = 1; st = 20; c = 16; expect = $true; label = 'count-upper-limit' },
    @{ s = 1; st = 20; c = 0; expect = $false; label = 'count-below-limit' },
    @{ s = 1; st = 20; c = 17; expect = $false; label = 'count-above-limit' }
  )) {
    $validation = Test-XrunScenarioParameters $item.s $item.st $item.c
    $ok = ($validation.valid -eq $item.expect)
    $checks += [ordered]@{ label = $item.label; valid = $validation.valid; expected = $item.expect; result = $ok; reason = $validation.reason }
  }
  $sample = New-XrunFinalCase 'capture-overflow-drop-input' 'capture-overflow-drop' 'NotRun' 'validate-only-no-runtime-evidence' $false 'not_applicable' $null 'validate-only.stdout.txt' 'validate-only.stderr.txt' 'validate-only-controller.txt'
  $schemaProblems = Test-XrunFinalSchema $sample
  foreach ($state in 'cleared', 'clear_failed', 'not_applicable') {
    $sample.injection.cleanup = $state
    if ((Test-XrunFinalSchema $sample) -contains 'bad-cleanup-vocabulary') {
      $schemaProblems += "cleanup-vocabulary-rejected-$state"
    }
  }
  $sample.injection.cleanup = 'not_applicable'
  $path = Join-Path $script:OutputRoot "validate-only-xrun-case-$Os.json"
  ConvertTo-Json -InputObject $sample -Depth 8 | Set-Content -LiteralPath $path -Encoding utf8
  $failed = @($checks | Where-Object { -not $_.result })
  foreach ($check in $checks) { Write-XrunLog "validate-only $($check.label): valid=$($check.valid) expected=$($check.expected) result=$($check.result) reason=$($check.reason)" }
  Write-XrunLog "validate-only schema problems=$(@($schemaProblems).Count) sample=$path (not runtime evidence)"
  if ($failed.Count -or @($schemaProblems).Count) {
    Write-Output "validate-only: Fail checks_failed=$($failed.Count) schema_problems=$(@($schemaProblems).Count)"
    exit 1
  }
  Write-Output 'validate-only: Pass (static validation only; not runtime evidence)'
  exit 0
}

if ($ValidateOnly) {
  Invoke-XrunValidateOnly
}

Write-XrunLog "xrun controller start os=$Os output=$script:OutputRoot start_packet=$StartPacket packet_count=$PacketCount"
$preflight = Test-XrunPreflight
Write-XrunLog "preflight ok=$($preflight.ok) problems=$($preflight.problems -join ',')"

if (-not $preflight.ok) {
  foreach ($definition in $script:CaseDefinitions) {
    $stdoutName = "xrun-$($definition.name)-$Os.stdout.txt"
    $stderrName = "xrun-$($definition.name)-$Os.stderr.txt"
    New-Item -ItemType File -Force (Join-Path $script:OutputRoot $stdoutName) | Out-Null
    New-Item -ItemType File -Force (Join-Path $script:OutputRoot $stderrName) | Out-Null
    $script:CaseResults += New-XrunFinalCase $definition.name $definition.scenario_name 'NotRun' "preflight-not-satisfied:$($preflight.problems -join ',')" $false 'not_applicable' $null $stdoutName $stderrName ([IO.Path]::GetFileName($script:ControllerLog))
  }
} else {
  foreach ($definition in $script:CaseDefinitions) {
    $script:CaseResults += Invoke-XrunHarnessCase $definition
  }
}

foreach ($caseResult in $script:CaseResults) {
  $schemaProblems = Test-XrunFinalSchema $caseResult
  if (@($schemaProblems).Count) {
    $script:HarnessProblems += "$(Get-MapValue (Get-MapValue $caseResult 'case') 'name')-schema-$($schemaProblems -join ';')"
  }
  $caseName = Get-MapValue (Get-MapValue $caseResult 'case') 'name'
  $casePath = Join-Path $script:OutputRoot "rf1009-xrun-case-$caseName-$Os.json"
  ConvertTo-Json -InputObject $caseResult -Depth 8 | Set-Content -LiteralPath $casePath -Encoding utf8
  Write-XrunLog "case json: $casePath result=$(Get-MapValue (Get-MapValue $caseResult 'case') 'result') schema_problems=$(@($schemaProblems).Count)"
}

$aggregatePath = Join-Path $script:OutputRoot "rf1009-xrun-cases-$Os.json"
ConvertTo-Json -InputObject @($script:CaseResults) -Depth 8 | Set-Content -LiteralPath $aggregatePath -Encoding utf8
Write-XrunLog "aggregate case json: $aggregatePath harness_problems=$($script:HarnessProblems.Count)"

$json = "vaudio-probe-$Os.json"
$failedCases = @($script:CaseResults | Where-Object { (Get-MapValue (Get-MapValue $_ 'case') 'result') -eq 'Fail' })
$notRunCases = @($script:CaseResults | Where-Object { (Get-MapValue (Get-MapValue $_ 'case') 'result') -eq 'NotRun' })
if (-not $preflight.ok) {
  $stageStatus = 'not_run'
} elseif ($script:HarnessProblems.Count -or $failedCases.Count) {
  $stageStatus = 'error'
} else {
  $stageStatus = 'ok'
}
$summary = ($script:CaseResults | ForEach-Object { "$(Get-MapValue (Get-MapValue $_ 'case') 'name')=$(Get-MapValue (Get-MapValue $_ 'case') 'result')" }) -join ','
$detail = "cases=$summary; harness_problems=$($script:HarnessProblems.Count)"
if ($notRunCases.Count -eq $script:CaseResults.Count -and $preflight.ok) { $detail += '; all_cases_not_run' }
Add-StageRecord -JsonPath $json -Stage xrun-cases -Status $stageStatus -Detail $detail
Write-XrunLog "stage xrun-cases status=$stageStatus detail=$detail"
