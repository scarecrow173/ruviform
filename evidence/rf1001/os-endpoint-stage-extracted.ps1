RUVIFORM_RUN_REPO: ${{ github.repository }}
RUVIFORM_RUN_SHA: ${{ github.sha }}
. ./tools/testsigned-driver/probe-stage.ps1
$os = '${{ matrix.os }}'
$json = "vaudio-probe-$os.json"
$log = "stage-os-endpoint-wiring-$os.txt"
$collect = "os-endpoint-run-metadata-$os"
New-Item -ItemType File -Force $log | Out-Null
New-Item -ItemType Directory -Force $collect | Out-Null
$streamStatus = Get-StageStatus $json 'stream-roundtrip'
$midiStatus = Get-StageStatus $json 'midi-loopback'
"  prerequisite stream-roundtrip: $streamStatus / midi-loopback: $midiStatus" | Out-Probe $log
"  run metadata identity: repo=${env:RUVIFORM_RUN_REPO} sha=${env:RUVIFORM_RUN_SHA}" | Out-Probe $log
"  --- os_endpoint wiring legs (--nocapture auxiliary evidence; canonical lane: mise run test-os-endpoint-wiring) ---" | Out-Probe $log
$out = @(& mise exec -- cargo test -p ruviform-audio-test --test os_endpoint -- --ignored --test-threads=1 --nocapture 2>&1)
$testExit = $LASTEXITCODE
$out | ForEach-Object { "  $_" | Out-Probe $log }
"  test exit=$testExit" | Out-Probe $log
$verdictLines = @($out | Where-Object { "$_" -match '^verdict:\s*' })
$badVerdicts = @($verdictLines | Where-Object { "$_" -notmatch '^verdict:\s*(Pass|NotRun)' })
"  verdict lines=$($verdictLines.Count)" | Out-Probe $log
$audioCase = 'rf629.audio.virtual-endpoint.render-capture'
$midiCase = 'rf629.midi.virtual-endpoint.loopback'
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
$audioMeta = @($metadata | Where-Object { $_.case_id -eq $audioCase }) | Select-Object -Last 1
$midiMeta = @($metadata | Where-Object { $_.case_id -eq $midiCase }) | Select-Object -Last 1
$audioResult = if ($audioMeta) { $audioMeta.result } else { 'metadata-missing' }
$midiResult = if ($midiMeta) { $midiMeta.result } else { 'metadata-missing' }
"  metadata: audio=$audioResult midi=$midiResult records=$($metadata.Count)" | Out-Probe $log
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
$expectedFiles = @(
  'run-metadata-rf629-audio-virtual-endpoint-render-capture.json',
  'run-metadata-rf629-midi-virtual-endpoint-loopback.json'
)
$missingCopies = @($expectedFiles | Where-Object { -not (Test-Path -LiteralPath (Join-Path $collect $_)) })
if ($missingCopies.Count) {
  "  missing copied metadata: $($missingCopies -join ', ')" | Out-Probe $log
}
"  collected metadata files: $((Get-ChildItem $collect -Filter '*.json' -ErrorAction SilentlyContinue | ForEach-Object Name) -join ', ')" | Out-Probe $log
$metadataProblems = @()
foreach ($case in @(
    @{ label = 'audio'; meta = $audioMeta; caseId = $audioCase },
    @{ label = 'midi'; meta = $midiMeta; caseId = $midiCase }
  )) {
  if (-not $case.meta) {
    $metadataProblems += "$($case.label):missing"
    continue
  }
  if ($case.meta.level -ne 'ext-l3') { $metadataProblems += "$($case.label):level=$($case.meta.level)" }
  if ($case.meta.environment_kind -ne 'virtual-os-endpoint') { $metadataProblems += "$($case.label):environment=$($case.meta.environment_kind)" }
  if ($case.meta.repo -ne ${env:RUVIFORM_RUN_REPO}) { $metadataProblems += "$($case.label):repo=$($case.meta.repo)" }
  if ($case.meta.sha -ne ${env:RUVIFORM_RUN_SHA}) { $metadataProblems += "$($case.label):sha=$($case.meta.sha)" }
  if ($case.meta.result -notin @('pass', 'not-run')) { $metadataProblems += "$($case.label):result=$($case.meta.result)" }
}
if ($metadataProblems.Count) {
  "  metadata problems: $($metadataProblems -join '; ')" | Out-Probe $log
}
# fail-closed: Pass/NotRun は観測終端として ok。Fail・crash
# （test exit 非ゼロ）・verdict 未到達・RunMetadata 未到達/不正・
# artifact copy 不能は error。両 leg pass の AC 判定は stage status
# ではなく RunMetadata JSON 本体を読んで行う。
$status = 'ok'
if ($testExit -ne 0) { $status = 'error' }
if ($verdictLines.Count -lt 2 -or $badVerdicts.Count) { $status = 'error' }
if (-not $audioMeta -or -not $midiMeta -or $metadataParseErrors.Count -or $metadataProblems.Count) { $status = 'error' }
if ($copyFailures.Count -or $missingCopies.Count -or $copied -lt 2) { $status = 'error' }
$detail = "test_exit=$testExit; audio=$audioResult; midi=$midiResult; verdicts=$($verdictLines.Count); metadata=$($metadata.Count); copied=$copied"
if ($metadataProblems.Count) { $detail += "; metadata_problems=$($metadataProblems -join '|')" }
if ($copyFailures.Count) { $detail += "; copy_errors=$($copyFailures.Count)" }
Add-StageRecord -JsonPath $json -Stage os-endpoint-wiring -Status $status -Detail $detail -ExitCode $testExit
exit 0
