# Invoke-EndpointProbe.ps1 — Issue #977 用の endpoint_probe 実行ヘルパ。
# `cargo run -p ruviform-audio-device --example endpoint_probe`（mise exec 経由）
# を実行し、verdict 行から計測値を `key=value` 行として標準出力へ出す。
# probe 本体のテキスト出力は -OutFile へそのまま保存する（stage log 用）。
#
# probe の出力は観測 evidence であり判定ではないため、pair 未検出でも
# このスクリプト自体は exit 0（検出数は pair_count= で伝える）。
# 実行不能（cargo 起動失敗等）のみ exit 1。
param(
    [string]$OutFile = 'endpoint-probe.txt'
)

$ErrorActionPreference = 'Continue'

$probeArgs = @('exec', '--', 'cargo', 'run', '-p', 'ruviform-audio-device', '--example', 'endpoint_probe')
$probeOut = & mise @probeArgs 2>&1 | Out-String
$probeExit = $LASTEXITCODE
$global:LASTEXITCODE = 0

$probeOut | Out-File -FilePath $OutFile -Encoding utf8

$endpoints = ($probeOut | Select-String -Pattern 'endpoints enumerated:\s*(\d+)' | ForEach-Object { $_.Matches[0].Groups[1].Value } | Select-Object -First 1)
$candidates = ($probeOut | Select-String -Pattern 'virtual-endpoint candidates \(name heuristic, unconfirmed\):\s*(\d+)' | ForEach-Object { $_.Matches[0].Groups[1].Value } | Select-Object -First 1)
$pairCount = ($probeOut | Select-String -Pattern 'pair candidates \(name-stem match, unconfirmed\):\s*(\d+)' | ForEach-Object { $_.Matches[0].Groups[1].Value } | Select-Object -First 1)
$pairLines = [regex]::Matches($probeOut, 'render "([^"]+)" . capture "([^"]+)"')
$pairList = ($pairLines | ForEach-Object { "$($_.Groups[1].Value) <-> $($_.Groups[2].Value)" }) -join ' | '

Write-Output "probe_exit=$probeExit"
Write-Output "endpoints_enumerated=$endpoints"
Write-Output "virtual_candidates=$candidates"
Write-Output "pair_count=$pairCount"
Write-Output "pairs=$pairList"

if ($probeExit -ne 0 -and -not $pairCount) {
    exit 1
}
exit 0
