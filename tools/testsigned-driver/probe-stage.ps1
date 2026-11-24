# tools/testsigned-driver/probe-stage.ps1
# Issue #961 — virtual-driver-install-probe workflow の stage 記録ヘルパ。
# 各 workflow step が dot-source して使う。観測結果は leg ごとの JSON
# （install-probe-<os>.json）へ {stage,status,detail,exit_code} レコードと
# して追記し、生ログは stage-<name>-<os>.txt へ tee する。
#
# status 語彙（fail-closed — 未実行・不合格を Pass にしない）:
#   ok       観測・実行が成功
#   absent   測定対象が不在（判定材料 — job 失敗ではない）
#   blocked  前 stage の結果で実施不能（理由を detail に）
#   not_run  実施しなかった（理由を detail に）
#   error    実行したが失敗（exit code・stderr を detail に）

function Out-Probe {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory, ValueFromPipeline)][AllowEmptyString()][string]$Line
    )
    process { $Line | Tee-Object -FilePath $Path -Append }
}

function Add-StageRecord {
    param(
        [Parameter(Mandatory)][string]$JsonPath,
        [Parameter(Mandatory)][string]$Stage,
        [Parameter(Mandatory)][ValidateSet('ok', 'absent', 'blocked', 'not_run', 'error')][string]$Status,
        [string]$Detail = '',
        [object]$ExitCode = $null
    )
    $records = @()
    if (Test-Path $JsonPath) {
        $existing = Get-Content -Raw $JsonPath | ConvertFrom-Json
        if ($existing) { $records = @($existing) }
    }
    $records += [ordered]@{
        stage        = $Stage
        status       = $Status
        detail       = $Detail
        exit_code    = $ExitCode
        recorded_utc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
    }
    ConvertTo-Json -InputObject @($records) | Set-Content -Path $JsonPath -Encoding utf8
}

function Get-StageStatus {
    param(
        [Parameter(Mandatory)][string]$JsonPath,
        [Parameter(Mandatory)][string]$Stage
    )
    if (-not (Test-Path $JsonPath)) { return $null }
    $records = @(Get-Content -Raw $JsonPath | ConvertFrom-Json)
    $match = @($records | Where-Object { $_.stage -eq $Stage }) | Select-Object -Last 1
    if ($match) { return $match.status }
    return $null
}

# stage 間で受け渡す小さい共有状態（証明書 thumbprint・package dir・
# published INF 名・devnode instance id）を probe-state-<os>.json に保持する。
function Set-ProbeState {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Key,
        [AllowNull()][object]$Value
    )
    $state = [ordered]@{}
    if (Test-Path $Path) {
        foreach ($p in ((Get-Content -Raw $Path | ConvertFrom-Json).PSObject.Properties)) {
            $state[$p.Name] = $p.Value
        }
    }
    $state[$Key] = $Value
    ConvertTo-Json -InputObject $state | Set-Content -Path $Path -Encoding utf8
}

function Get-ProbeState {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Key
    )
    if (-not (Test-Path $Path)) { return $null }
    $state = Get-Content -Raw $Path | ConvertFrom-Json
    if ($null -ne $state -and $state.PSObject.Properties[$Key]) { return $state.$Key }
    return $null
}

# PATH → Windows Kits bin の順で tool を解決する。Kits bin 側は見つかった
# 全 hit を返し、呼び出し側が初ヒット/版差異を記録する。
function Find-Tool {
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
    ) | Where-Object { Test-Path $_ }
    $hits = @()
    foreach ($r in $roots) {
        $hits += Get-ChildItem $r -Recurse -Depth 3 -Filter $exe -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty FullName
    }
    if ($AllKitsHits) { return $hits }
    if ($hits.Count -gt 0) { return @($hits[0]) }
    return @()
}
