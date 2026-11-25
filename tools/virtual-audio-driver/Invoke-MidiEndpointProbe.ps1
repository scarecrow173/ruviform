# Invoke-MidiEndpointProbe.ps1 — Issue #979 用の midi_endpoint_probe 実行ヘルパ。
# `cargo run -p ruviform-midi-device --example midi_endpoint_probe`（mise exec 経由）
# を実行し、verdict 行から計測値を `key=value` 行として標準出力へ出す。
# probe 本体のテキスト出力は -OutFile へそのまま保存する（stage log 用）。
#
# probe の出力は観測 evidence であり判定ではないため、loopback 候補未検出でも
# このスクリプト自体は exit 0（検出数は loopback_candidates= で伝える）。
# 実行不能（cargo 起動失敗等）のみ exit 1。
param(
    [string]$OutFile = 'midi-endpoint-probe.txt'
)

$ErrorActionPreference = 'Continue'

$probeArgs = @('exec', '--', 'cargo', 'run', '-p', 'ruviform-midi-device', '--example', 'midi_endpoint_probe')
$probeOut = & mise @probeArgs 2>&1 | Out-String
$probeExit = $LASTEXITCODE
$global:LASTEXITCODE = 0

$probeOut | Out-File -FilePath $OutFile -Encoding utf8

$ports = [regex]::Match($probeOut, 'winmm input ports:\s*(\d+),\s*output ports:\s*(\d+)')
$inPorts = if ($ports.Success) { $ports.Groups[1].Value } else { '' }
$outPorts = if ($ports.Success) { $ports.Groups[2].Value } else { '' }
$candidates = ($probeOut | Select-String -Pattern 'loopback-style candidates \(bidirectional name match, unconfirmed\):\s*(\d+)' | ForEach-Object { $_.Matches[0].Groups[1].Value } | Select-Object -First 1)
$provisioning = ($probeOut | Select-String -Pattern 'provisioning:\s*(\w+)' | ForEach-Object { $_.Matches[0].Groups[1].Value } | Select-Object -First 1)
$candidateNames = @([regex]::Matches($probeOut, '\[(?:input|output)\]\s+(.+?)\s+index=\d+[^\r\n]*\[loopback-candidate') | ForEach-Object { $_.Groups[1].Value } | Select-Object -Unique)
$portList = $candidateNames -join ' | '

Write-Output "probe_exit=$probeExit"
Write-Output "midi_in_ports=$inPorts"
Write-Output "midi_out_ports=$outPorts"
Write-Output "loopback_candidates=$candidates"
Write-Output "provisioning=$provisioning"
Write-Output "candidate_ports=$portList"

if ($probeExit -ne 0 -and -not $candidates) {
    exit 1
}
exit 0
