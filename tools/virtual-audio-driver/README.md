# tools/virtual-audio-driver

Issue #977 Slice B stage 1（virtual audio endpoint driver の build→test-sign→
install→endpoint 列挙）で使う helper。Issue #979 Slice E で MIDI 側 probe の
`Invoke-MidiEndpointProbe.ps1` を追加。

## Invoke-EndpointProbe.ps1

`crates/ruviform-audio-device` の `endpoint_probe` example を `mise exec` 経由で
実行し、probe 出力を `-OutFile`（default `endpoint-probe.txt`）へ保存した上で
verdict 行から次の `key=value` を標準出力へ出す。workflow の stage step が
この出力を stage record の `detail` として使う想定。

| key | 内容 |
| --- | --- |
| `probe_exit` | probe プロセスの exit code |
| `endpoints_enumerated` | `endpoints enumerated: N` |
| `virtual_candidates` | name-heuristic 候補数 |
| `pair_count` | render↔capture pair 候補数 |
| `pairs` | pair 名一覧（`render <-> capture`、区切り `|`） |

pair 未検出は観測値（`pair_count=0`）として扱い exit 0。実行不能のみ exit 1。

## Invoke-MidiEndpointProbe.ps1

`crates/ruviform-midi-device` の `midi_endpoint_probe` example を `mise exec`
経由で実行し、probe 出力を `-OutFile`（default `midi-endpoint-probe.txt`）へ
保存した上で verdict 行から次の `key=value` を標準出力へ出す。workflow の
`midi-endpoint-probe` stage step がこの出力を stage record の `detail` と
して使う想定（`Invoke-EndpointProbe.ps1` 同型）。

| key | 内容 |
| --- | --- |
| `probe_exit` | probe プロセスの exit code |
| `midi_in_ports` | WinMM `midiIn` 列挙ポート数 |
| `midi_out_ports` | WinMM `midiOut` 列挙ポート数 |
| `loopback_candidates` | bidirectional name match の loopback 候補数 |
| `provisioning` | `NOT_PROVISIONED` / `CANDIDATES_OBSERVED` |
| `candidate_ports` | 候補 port 名一覧（重複除去・区切り `|`） |

候補未検出は観測値（`loopback_candidates=0`）として扱い exit 0。実行不能の
み exit 1。

## Provision-RvfVirtualAudioEndpoint.ps1 / Unprovision-RvfVirtualAudioEndpoint.ps1

sv#115 Slice A で抽出した virtual audio endpoint provisioning primitive。
`Provision-RvfVirtualAudioEndpoint.ps1` は repo 内 source から driver package
を build し、catalog 作成、ephemeral self-signed test certificate 作成・登録、
catalog signing / verification、driver store staging、root-enumerated MEDIA
devnode 作成、MMDevice endpoint pair 候補確認、opt-in の microphone consent
provisioning と audio service restart、readiness 判定を行う。
`Unprovision-RvfVirtualAudioEndpoint.ps1` は provision state JSON を入力に、
devnode/driver store/certificate/consent/service を idempotent に cleanup し、
residue を state JSON へ記録する。

対象環境は **GitHub-hosted `windows-2025` ephemeral runner かつ管理者権限**
だけである。`GITHUB_ACTIONS=true` かつ `RUNNER_ENVIRONMENT=github-hosted` を
確認できない local machine、開発者 machine、persistent/self-hosted runner では
OS state を変更せず `not_run`（exit 1）で終了する。local/self-hosted で実行
しないのは、driver store・Root/TrustedPublisher certificate store・HKLM/HKCU
microphone consent registry・root devnode・audio service を変更し、完全な
restore 契約が ephemeral runner 前提だからである。`bcdedit` 変更と reboot は
行わない。

```powershell
pwsh tools\virtual-audio-driver\Provision-RvfVirtualAudioEndpoint.ps1 `
  -SourceRoot D:\a\ruviform\ruviform `
  -ExpectedSourceRev <commit-sha> `
  -WorkspaceRoot $env:RUNNER_TEMP\rvf-virtual-audio-workspace `
  -StatePath $env:RUNNER_TEMP\rvf-provision-state.json `
  -EvidenceDir $env:RUNNER_TEMP\rvf-provision-evidence `
  -AllowWdkInstall -AllowMicConsentProvisioning

pwsh tools\virtual-audio-driver\Unprovision-RvfVirtualAudioEndpoint.ps1 `
  -StatePath $env:RUNNER_TEMP\rvf-provision-state.json `
  -WorkspaceRoot $env:RUNNER_TEMP\rvf-virtual-audio-workspace `
  -EvidenceDir $env:RUNNER_TEMP\rvf-unprovision-evidence `
  -CleanupMode Full
```

主要な provision 入力は `SourceRoot`、`ExpectedSourceRev`、`WorkspaceRoot`、
`StatePath`、`EvidenceDir`、`Arch`（初期は `x64` のみ）、`Configuration`、
`AllowWdkInstall`、`AllowMicConsentProvisioning`、`TimeoutSeconds`、
`CleanupOnFailure`（`Full`/`Preserve`）。script は caller の current
directory や consumer checkout に依存せず、これらを absolute path に正規化
して使う。secret・token・consumer repo path・consumer test 名は入力にしない。

両 script は human-readable log に加えて schema
`rvf-virtual-audio-provisioning/v1` の UTF-8 state JSON を出力する。terminal
status と stage status の語彙は `tools\testsigned-driver\probe-stage.ps1` と
同じ `ok`/`absent`/`blocked`/`not_run`/`error`。exit code は `0`=contract
上の目的完了、`1`=state JSON を出力した非 `ok` 観測、`2`=引数不正や state
JSON を出力できない等の interface/contract violation。artifact manifest の
hash は SHA-256。self-signed certificate の private key は export しない。

`Unprovision` の `CleanupMode` は通常 `Full`。`ReportOnly` は residue 観測
だけを行う診断用で、acceptance workflow の通常 cleanup には使わない。
cleanup は primitive が作成したと記録できない pre-existing driver/cert/
devnode を無条件削除せず、evidence/log/artifact manifest も削除しない。
cleanup の成否は consumer test verdict や probe verdict と混同しない。

`.github/workflows/virtual-audio-driver-probe.yml` はこの script pair を
`AllowWdkInstall`・`AllowMicConsentProvisioning`・`CleanupOnFailure Full`・
`CleanupMode Full` で呼び、従来の manual probe artifact 利用者向けに stage
record/state key/log 名を投影する。primitive の exit 1 は absence-is-data の
観測結果として artifact に残し、exit 2 だけを probe 機構破損として扱う。
stream roundtrip、MIDI probe/loopback、os-endpoint wiring/e2e、fault casesは
primitive の責務に含めず、従来どおり consumer/test stage 側で実行する。

## 使い方

```powershell
pwsh tools\virtual-audio-driver\Invoke-EndpointProbe.ps1 -OutFile stage-endpoint-probe.txt
pwsh tools\virtual-audio-driver\Invoke-MidiEndpointProbe.ps1 -OutFile stage-midi-endpoint-probe.txt
```

## 再利用している既存 helper

- `tools\testsigned-driver\probe-stage.ps1` — stage record / probe state の
  共通 helper（workflow の各 stage step が dot-source する）。
- `tools\testsigned-driver\TestDevice.ps1` — setupapi/newdev 経由の root
  devnode 作成・削除。`-ClassName MEDIA -ClassGuid {4d36e96c-e325-11ce-bfc1-08002be10318}`
  を指定すれば audio class としてそのまま使える（変更不要）。
