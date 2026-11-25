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
