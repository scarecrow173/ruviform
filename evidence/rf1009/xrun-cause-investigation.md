# Issue #1009 xrun 原因調査証跡

## 目的と範囲

本書は `test/1009-xrun-cause-investigation` での原因調査を記録する。対象は、既実装の deterministic virtual-audio xrun injection が provisioned run `45837299116` attempt `1` で adapter-visible oracle を生まなかった原因の切り分けである。oracle は変更していない。positive case は typed `DeviceError::Xrun`、forward `frame_position` gap、`ClockDiscontinuity`、callback 継続を要求し、negative control は injected PCM zero span の検出を要求する。

## 既存 provisioned evidence の確定事実

run `45837299116` / attempt `1` / SHA `ad188873144e93a3fd384fca0a64b2fe0f49ce16` では次が成立していた。

- endpoint preflight は成功し、exact endpoint pair が存在した。
- registry write/readback、cleanup、controller の stream-open reload boundary、harness process completion が成立した。
- それでも positive case は typed xrun 0、clock discontinuity 0、forward gap 0、negative case は sample gap 未検出で case-level Fail だった。
- input callback は約 299 回、output callback は約 302 回進行し、max callback stall は 20ms だった。adapter surface で観測可能なストリーム停止や invalidation はなかった。

したがって、registry write/readback や harness completion だけでは driver が scenario を適用したことの証拠にならない。本 slice は registry snapshot → target pin 判定 → notification suppression/zero fill → OS packet skip → WASAPI position gap → adapter error mapping のどこで観測が切れるかを判別するための additive instrumentation を追加した。

## H1-H7 判定マトリクス

| 仮説 | 現時点判定 | 根拠 / 未決事項 |
| --- | --- | --- |
| H1: registry path/name/type mismatch | 静的には否定。runtime 確認を instrumentation へ寄せた | controller は `HKLM:\SYSTEM\CurrentControlSet\Services\RvfVirtualAudio\Parameters` の `RvfXrunScenario/StartPacket/PacketCount` を DWORD で write/readback し、driver も同名を `RtlQueryRegistryValues` で読む。ただし stream snapshot に入ったことまでは従来 artifact だけでは証明できない。 |
| H2: target pin/device 判定 mismatch | 静的には否定寄り。runtime 判定は未証明 | driver は `eMicInDevice && KSPIN_WAVEIN_HOST && IsSystemCapturePin` を target とする。`micinwavtable.h` では該当 pin が streaming capture pin であり、命名上の `loopback-capture` は Cable Output の capture streaming pin を意味する。実 stream で target flag が立つかは snapshot event で確認する。 |
| H3: suppression window/完了 index 不整合 | 未決。静的な大きな off-by-one は見つからない | `TimerNotifyRT` は completion index が `[start,start+count)` のときのみ `KeSetEvent` を抑制し、position/packet counter は進める。実際に遷移したかは suppression transition event で確認する。 |
| H4: notification suppression が OS packet skip を生まない | 最有力候補の一つ。従来 artifact では決定不能 | `GetReadPacket` には OS skip を示す `droppedPackets` 計算があったが emit がなく、notification suppression が OS 側 skip を起こしたか不明だった。bounded gap event を追加した。 |
| H5: adapter が position gap を検出/マッピングできない | 否定 | `wasapi_input.rs` は各 packet の `GetBuffer` device position を verbatim で `frame_position` に渡し、forward gap で `Xrun`→`ClockDiscontinuity`、backward/reset または flag-only で `ClockDiscontinuity` を発行する。drain/chunking は gap を隠せない。unit test も存在する。 |
| H6: content-gap byte span または zero-run detector の不整合 | 未決。従来 artifact の説明不足が大きい | byte span は `startPacket * packetBytes` から count 分で、write position も absolute stream byte position なので静的には aligned に見える。一方、従来 oracle artifact には nonzero/peak/channel 情報がなく、loopback PCM に十分な nonzero 内容があったかを判別できない。zero-fill boundary event と harness diagnostics で切り分ける。 |
| H7: ETW/artifact observability 不足 | 成立。本 slice の直接対象 | 従来 artifact には driver 内部 fact がなく、controller/harness 完了が injection application の証拠として使えなかった。IHV-defined bounded event、ETL/XML capture、raw harness diagnostics を追加した。 |

## 追加した additive instrumentation

### driver (`drivers/rvf-virtual-audio/EndpointsCommon/minwavertstream.{h,cpp}`)

oracle や injection semantics は変更せず、`eMINIPORT_IHV_DEFINED` の bounded diagnostic event を追加した。

- `0x1001` scenario snapshot: device type、pin id、target flag、scenario/start/count、registry query status。
- `0x1002` zero range: packet byte size、notification count、zero start/end byte。
- `0x1003` suppression transition: completion index、suppress on/off、suppression end packet。状態遷移のみなので stream あたり高々 2 回。
- `0x1004` zero-fill boundary: zero fill start/end。boundary のみ。
- `0x1005` OS packet gap: `GetReadPacket` の `droppedPackets`、available/last OS packet。16 回 cap。

RT path では per-packet 無制限 event は追加していない。suppression/zero-fill は状態遷移または boundary のみであり、既存 ETW helper を再利用する。

### controller (`tools/virtual-audio-driver/Invoke-XrunCases.ps1`)

- harness process 起動直前に `Microsoft-Windows-Audio` provider (`ae4bd3be-f36f-45b6-8d21-bdd6fb832853`) の logman trace を開始し、process 終了後に stop/delete する。
- ETL が残れば `tracerpt` で XML へ変換し、両方を workflow artifact として残す。provider 登録状態の `wevtutil gp` 出力も `*-driver-trace-providers.txt` として残し、ETW collectability と provider 不在を区別する。
- trace 開始/変換に失敗しても case を Fail にせず、final case JSON の additive `diagnostics.driver_trace` に unsupported/captured と detail を記録する。
- registry cleanup は従来どおり harness process 終了後に行い、stream-open reload boundary を変えていない。
- final schema validation は既存必須 key を緩めず、追加 diagnostics を必須化していない。

### harness (`crates/ruviform-audio-device/tests/virtual_endpoint.rs`)

作業中の変更を含む。目的は次の additive diagnostics であり、verdict/oracle には使わない。

- bounded input packet trace: callback/capture timestamp、frame position、frames、timestamp delta、position delta、error event ordering、overflow counter。
- inferred burst/stall と forward/backward/contiguous transition summary。
- zero-run の leading/interior/trailing と channel coverage の postprocess diagnostics。
- negotiated format、requested fixed buffer、backend buffer size、latency、fallback 情報。

## artifact inventory

| path | 種別 | 内容 |
| --- | --- | --- |
| `evidence/rf1009/xrun-provisioned-evidence.md` | 既存証跡 | run `45837299116` の provisioned evidence と解釈。 |
| `evidence/rf1009/provisioned/run45837299116-attempt1/rf1009-xrun-cases-windows-2025.json` | 既存 artifact copy | stage aggregate。positive/negative case-level Fail。 |
| `evidence/rf1009/provisioned/run45837299116-attempt1/xrun-capture-overflow-drop-input-windows-2025.result.json` | 既存 artifact copy | positive raw observation。typed xrun/discontinuity/forward gap すべて 0。 |
| `evidence/rf1009/provisioned/run45837299116-attempt1/xrun-capture-content-gap-input-negative-windows-2025.result.json` | 既存 artifact copy | negative raw observation。sample gap 未検出。 |
| `evidence/rf1009/provisioned/run45837299116-attempt1/artifact-inventory.json` | 既存 inventory | 元 artifact の hash/path inventory。raw log/binary は非 commit。 |
| `drivers/rvf-virtual-audio/EndpointsCommon/minwavertstream.cpp` | 変更 | bounded driver diagnostic events。 |
| `drivers/rvf-virtual-audio/EndpointsCommon/minwavertstream.h` | 変更 | diagnostic state fields。 |
| `tools/virtual-audio-driver/Invoke-XrunCases.ps1` | 変更 | optional driver trace capture と additive diagnostics。 |
| `crates/ruviform-audio-device/tests/virtual_endpoint.rs` | 変更 | bounded harness diagnostics と、gap 不在時に before/after-injection 未確認を実測 callback 欠如のように誤記していた detail 表示の修正。 |
| `evidence/rf1009/xrun-cause-investigation.md` | 本書 | cause-analysis evidence。 |

新しい provisioned run では、少なくとも次が追加 artifact になる想定である。

- `xrun-<case>-windows-2025-driver-trace.etl`
- `xrun-<case>-windows-2025-driver-trace.xml`（tracerpt 変換が使える場合）
- raw stdout に埋め込まれた harness diagnostics
- final case JSON の `diagnostics.driver_trace`

## provisioned run serialization

調査時点で `virtual-audio-driver-probe` の workflow_dispatch run `45865926990` が `in_progress` であった（head SHA `5843719861fa8ca6da5c963a3f63c3b32047c4ff`）。同一 provisioned workflow はシリアル利用が必要なため、本 worker からは並行 dispatch していない。したがって、ここに記録するのは local validation と既存 provisioned artifact の分析であり、post-merge CI/provisioned acceptance ではない。

## ローカル検証

| lane | 結果 |
| --- | --- |
| `mise run fmt-check` | Pass |
| `mise run check ruviform-audio-device` | Pass |
| `mise run lint ruviform-audio-device` | Pass |
| `cargo test --locked -p ruviform-audio-device` | Pass（84 lib + 4 mock backend + 12 mock input + 6 doctest） |
| `cargo test --locked -p ruviform-audio-device --features real-device --test virtual_endpoint` | Pass（19 passed / 7 ignored） |
| `mise run test-ci-daily` | Pass（113/113） |
| `mise run test-ci-tracker` | Pass（108/108） |
| `mise exec -- prek run --all-files` | Pass |
| `tools/virtual-audio-driver/Invoke-XrunCases.ps1 -ValidateOnly` | Pass（static validation only; runtime evidence ではない） |
| driver binary build | NotRun（この local 環境には WDK build 環境がなく、provisioned workflow の driver build lane に委ねる） |

これらは local validation であり、provisioned/CI acceptance ではない。最終 validation は merge 後 commit に対して再実行する。

## 結論

現時点で確実に排除できるのは H5（adapter mapping defect）である。H1/H2 は静的には否定寄りだが runtime snapshot 未証明、H3/H4/H6 は従来 artifact だけでは決定不能であり、本 slice の instrumentation により次回 provisioned run で次のように判定できる。

1. `0x1001` が無い/値が disabled → H1 または registry snapshot/load timing。
2. `0x1001` はあるが target flag が 0 → H2。
3. target 1 でも `0x1003` suppression transition が無い → H3。
4. suppression があるのに `0x1005` OS packet gap が無い → H4。notification suppression は shared-mode engine の packet skip を生まない設計不整合。
5. `0x1005` があるのに harness/adapter position gap が無い → WASAPI engine position accounting または adapter 直前の observation loss。adapter 本体の mapping は H5 で排除済み。
6. `0x1004` zero boundary があるのに zero-run diagnostics に interior gap が無い → H6 の detector/PCM alignment。zero boundary が無ければ scenario application 側。

この evidence は原因確定ではなく、原因を一意に切り分けるための観測面を追加した調査 slice の記録である。
