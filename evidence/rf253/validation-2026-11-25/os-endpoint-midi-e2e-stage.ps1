. ./tools/testsigned-driver/probe-stage.ps1
$os = '${{ matrix.os }}'
$json = "vaudio-probe-$os.json"
$log = "stage-os-endpoint-midi-e2e-$os.txt"
$collect = "os-endpoint-midi-e2e-run-metadata-$os"
New-Item -ItemType File -Force $log | Out-Null
New-Item -ItemType Directory -Force $collect | Out-Null

$probeStatus = Get-StageStatus $json 'midi-endpoint-probe'
$midiStatus = Get-StageStatus $json 'midi-loopback'
$devnodeStatus = Get-StageStatus $json 'devnode-create'
"  prerequisite midi-endpoint-probe: $probeStatus / midi-loopback: $midiStatus / devnode-create: $devnodeStatus" | Out-Probe $log
"  run metadata identity: repo=${env:RUVIFORM_RUN_REPO} sha=${env:RUVIFORM_RUN_SHA}" | Out-Probe $log

$probeText = ''
$probeFile = "midi-endpoint-probe-$os.txt"
if (Test-Path -LiteralPath $probeFile) { $probeText = Get-Content -LiteralPath $probeFile -Raw -ErrorAction SilentlyContinue }
$provisioningObserved = ($probeText -match 'provisioning:\s*CANDIDATES_OBSERVED') -and ($probeText -match 'Ruviform MIDI Loopback')
$midiLoopbackText = ''
$midiLoopbackLog = "stage-midi-loopback-$os.txt"
if (Test-Path -LiteralPath $midiLoopbackLog) { $midiLoopbackText = Get-Content -LiteralPath $midiLoopbackLog -Raw -ErrorAction SilentlyContinue }
$midiLoopbackPass = $midiLoopbackText -match 'verdict:\s*Pass\s+—\s+round-trip observed through real OS ports'
"  RvfMidi provenance: candidates_observed=$provisioningObserved midi_loopback_pass=$midiLoopbackPass" | Out-Probe $log
"  --- provenance candidate evidence ---" | Out-Probe $log
$probeEvidenceLines = @($probeText -split "\r?\n" | Where-Object { $_ -match 'loopback-candidate|loopback-style candidates|provisioning:\s*' })
if ($probeEvidenceLines.Count) {
  $probeEvidenceLines | ForEach-Object { "  midi-endpoint-probe: $($_.Trim())" | Out-Probe $log }
} else {
  "  midi-endpoint-probe: no candidate/provisioning evidence lines" | Out-Probe $log
}
$midiEvidenceLines = @($midiLoopbackText -split "\r?\n" | Where-Object { $_ -match '^\s*candidate\[|verdict:\s*Pass\s+—\s+round-trip observed through real OS ports' })
if ($midiEvidenceLines.Count) {
  $midiEvidenceLines | ForEach-Object { "  midi-loopback: $($_.Trim())" | Out-Probe $log }
} else {
  "  midi-loopback: no candidate/round-trip Pass evidence lines" | Out-Probe $log
}
$endpointProvenance = if ($provisioningObserved -and $midiLoopbackPass) { 'rvfmidi-testsigned-driver-probe' } else { 'workflow-unverified-loopback-candidate' }
$env:RUVIFORM_VIRTUAL_ENDPOINT_PROVENANCE = $endpointProvenance
"  run metadata provenance override: $endpointProvenance" | Out-Probe $log

"  --- fixture build (canonical lane first step: mise run test-os-endpoint-midi-e2e) ---" | Out-Probe $log
$buildOut = @(& mise exec -- cargo build -p ruviform-plugin-test-vst3 --features l1-runner 2>&1)
$buildExit = $LASTEXITCODE
$buildOut | ForEach-Object { "  $_" | Out-Probe $log }
"  build exit=$buildExit" | Out-Probe $log

$out = @()
$testExit = $buildExit
if ($buildExit -eq 0) {
  "  --- os_endpoint_midi_e2e (--nocapture auxiliary evidence; canonical lane: mise run test-os-endpoint-midi-e2e) ---" | Out-Probe $log
  $out = @(& mise exec -- cargo test -p ruviform-plugin-test-vst3 --features l1-runner --test os_endpoint_midi_e2e -- --ignored --test-threads=1 --nocapture 2>&1)
  $testExit = $LASTEXITCODE
  $out | ForEach-Object { "  $_" | Out-Probe $log }
  "  test exit=$testExit" | Out-Probe $log
}

$verdictLines = @($out | Where-Object { "$_" -match '^verdict:\s*' })
$badVerdicts = @($verdictLines | Where-Object { "$_" -notmatch '^verdict:\s*(Pass|NotRun)' })
$verdict = if ($verdictLines) { (($verdictLines | Select-Object -Last 1) -replace '.*verdict:\s*', '').Trim() } else { 'no-verdict-line' }
"  verdict=$verdict lines=$($verdictLines.Count)" | Out-Probe $log

$caseId = 'rf253.midi.virtual-endpoint.host-plugin-e2e'
$metadata = @()
$metadataParseErrors = @()
foreach ($line in $out) {
  $text = "$line"
  if ($text -match '^run_metadata:\s*(\{.*\})\s*$') {
    try {
      $metadata += ($Matches[1] | ConvertFrom-Json -ErrorAction Stop)
    } catch {
      $metadataParseErrors += $_.Exception.Message
      "  run_metadata parse error: $($_.Exception.Message)" | Out-Probe $log
    }
  }
}
$caseMeta = @($metadata | Where-Object { $_.case_id -eq $caseId }) | Select-Object -Last 1
$caseResult = if ($caseMeta) { $caseMeta.result } else { 'metadata-missing' }
"  metadata: case=$caseResult records=$($metadata.Count)" | Out-Probe $log

$copyFailures = @()
$copied = 0
foreach ($line in $out) {
  $text = "$line"
  if ($text -match '^run_metadata_path:\s*(.+?)\s*$') {
    $source = $Matches[1]
    try {
      if (-not (Test-Path -LiteralPath $source)) { throw "run metadata source missing: $source" }
      Copy-Item -LiteralPath $source -Destination $collect -Force -ErrorAction Stop
      $copied += 1
      "  copied run metadata: $source -> $collect" | Out-Probe $log
    } catch {
      $copyFailures += $_.Exception.Message
      "  run metadata copy error: $($_.Exception.Message)" | Out-Probe $log
    }
  }
}
$expectedFile = 'run-metadata-rf253-midi-virtual-endpoint-host-plugin-e2e.json'
$missingCopy = -not (Test-Path -LiteralPath (Join-Path $collect $expectedFile))
if ($missingCopy) { "  missing copied metadata: $expectedFile" | Out-Probe $log }
"  collected metadata files: $((Get-ChildItem $collect -Filter '*.json' -ErrorAction SilentlyContinue | ForEach-Object Name) -join ', ')" | Out-Probe $log

$metadataProblems = @()
$passWithoutProvenance = $false
if (-not $caseMeta) {
  $metadataProblems += 'case:missing'
} else {
  if ($caseMeta.level -ne 'ext-l3') { $metadataProblems += "level=$($caseMeta.level)" }
  if ($caseMeta.environment_kind -ne 'virtual-os-endpoint') { $metadataProblems += "environment=$($caseMeta.environment_kind)" }
  if ($caseMeta.repo -ne ${env:RUVIFORM_RUN_REPO}) { $metadataProblems += "repo=$($caseMeta.repo)" }
  if ($caseMeta.sha -ne ${env:RUVIFORM_RUN_SHA}) { $metadataProblems += "sha=$($caseMeta.sha)" }
  if ($caseMeta.result -notin @('pass', 'not-run')) { $metadataProblems += "result=$($caseMeta.result)" }
  if ($caseMeta.result -eq 'pass') {
    $metadataEvidenceText = @($caseMeta.evidence | ForEach-Object { "$_" }) -join "`n"
    $metadataHasRvfMidiProvenance = $metadataEvidenceText -match 'virtual endpoint provenance:\s*rvfmidi-testsigned-driver-probe'
    if (-not ($provisioningObserved -and $midiLoopbackPass) -or -not $metadataHasRvfMidiProvenance) {
      $passWithoutProvenance = $true
      $metadataProblems += 'pass_without_rvfmidi_provenance'
      "  fail-closed: case result is pass but RvfMidi provenance is not established (workflow=$endpointProvenance metadata_provenance=$metadataHasRvfMidiProvenance)" | Out-Probe $log
    }
  }
}
if ($metadataProblems.Count) {
  "  metadata problems: $($metadataProblems -join '; ')" | Out-Probe $log
}

# fail-closed: provenance 付き Pass/NotRun は観測終端として ok。
# Fail・crash（test exit 非ゼロ）・verdict 未到達・RunMetadata
# 未到達/不正・artifact copy 不能は error。provenance 不在そのものは
# absence-is-data であり、case が honest NotRun なら stage ok だが、
# RvfMidi provenance なしの pass は証拠価値がないため error にする。
$status = 'ok'
if ($buildExit -ne 0 -or $testExit -ne 0) { $status = 'error' }
if ($verdictLines.Count -lt 1 -or $badVerdicts.Count) { $status = 'error' }
if (-not $caseMeta -or $metadataParseErrors.Count -or $metadataProblems.Count) { $status = 'error' }
if ($copyFailures.Count -or $missingCopy -or $copied -lt 1) { $status = 'error' }
if ($passWithoutProvenance) { $status = 'error' }
$detail = "build_exit=$buildExit; test_exit=$testExit; verdict=$verdict; case=$caseResult; metadata=$($metadata.Count); copied=$copied; provisioning_observed=$provisioningObserved; midi_loopback_pass=$midiLoopbackPass; endpoint_provenance=$endpointProvenance; pass_without_provenance=$passWithoutProvenance"
if ($metadataProblems.Count) { $detail += "; metadata_problems=$($metadataProblems -join '|')" }
if ($copyFailures.Count) { $detail += "; copy_errors=$($copyFailures.Count)" }
Add-StageRecord -JsonPath $json -Stage os-endpoint-midi-e2e -Status $status -Detail $detail -ExitCode $testExit
exit 0
