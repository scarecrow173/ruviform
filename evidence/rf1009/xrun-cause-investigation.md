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

目的は次の additive diagnostics であり、verdict/oracle には使わない。

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

run `45932170760` の追加 artifact copy は `evidence/rf1009/provisioned/run45932170760-attempt1/` にある。ETL binary は既存方針どおり commit せず、XML/provider log/case JSON/controller log/result JSON を commit する。run `45930911311` は driver compile failure の検証 runであり、xrun runtime evidence は無い。

### run45932170760 artifact sha256 inventory

対象は commit した machine-readable artifact copy の Git blob bytes である。ETL binary は commit していない。CRLF working-copy 表現ではなく、hook 正規化後に repository へ入る LF 正規化 bytes に対する sha256。

| file | bytes | sha256 |
| --- | ---: | --- |
| `rf1009-xrun-case-capture-content-gap-input-negative-windows-2025.json` | 2175 | `f998f20490e91510632ea6010008bcab5a6dce8caeb671ed60176dc957d79390` |
| `rf1009-xrun-case-capture-overflow-drop-input-windows-2025.json` | 2204 | `f91f5b35edd13315d29e5f6572e8bfc0bf7c1618098a47f9d40379df80234d35` |
| `rf1009-xrun-cases-windows-2025.json` | 4628 | `2abf7143e74b1019f5d7817b6d577b6e333c6adfbcd25e4d0b71aa255b1c3ccd` |
| `xrun-capture-content-gap-input-negative-windows-2025-driver-trace-providers.txt` | 19882 | `6a4fd8a633c922c423fb031c672dbfa774eade1f7897e0c42e1b6b52d432e282` |
| `xrun-capture-content-gap-input-negative-windows-2025-driver-trace.xml` | 3282 | `129c39250ff6e4b0b54fc6859a0c047ebb20b461d2c08308422ec2d5eff13bf1` |
| `xrun-capture-content-gap-input-negative-windows-2025.result.json` | 30647 | `4f4e2728e3c729e8ccf1d5d34399ed6a218c7018871f66b73d43fc3cc6b2b3e4` |
| `xrun-capture-overflow-drop-input-windows-2025-driver-trace-providers.txt` | 19882 | `6a4fd8a633c922c423fb031c672dbfa774eade1f7897e0c42e1b6b52d432e282` |
| `xrun-capture-overflow-drop-input-windows-2025-driver-trace.xml` | 3266 | `80f6b87d61ef22c0436c8b74f6eef37fe0adbdec8f97c64b74f61b9c8b381b1d` |
| `xrun-capture-overflow-drop-input-windows-2025.result.json` | 30973 | `b976d1ab2527196c17fd44ab33e1f2222a6adc1270f0108dcd972faeea670d9d` |
| `xrun-controller-windows-2025.txt` | 3683 | `1b41741de3ee26a65b0fab019bfa079165dec925e3a08d0fb53ab6b4ae2f9c21` |

## 検証記録

### ローカル検証（worker local OS: Microsoft Windows NT 10.0.26300.0）

| 受入条件 / lane | コマンド | exit code | 結果 |
| --- | --- | ---: | --- |
| canonical format | `mise run fmt-check` | 0 | Pass |
| canonical type check | `mise run check ruviform-audio-device` | 0 | Pass |
| canonical lint | `mise run lint ruviform-audio-device` | 0 | Pass |
| canonical package test | `mise run test ruviform-audio-device` | 0 | Pass（84 lib + 4 mock backend + 12 mock input + 6 doctest） |
| canonical real-device feature lint | `mise run lint-audio-real-input` | 0 | Pass |
| 対象 xrun filter lane | `mise run test -- ruviform-audio-device --features real-device --test virtual_endpoint xrun -- --test-threads=1` | 0 | Pass（14 passed / 3 ignored / 9 filtered out。endpoint-dependent case は ignored であり runtime Pass ではない） |
| canonical RT contract lane | `mise run test-rt` | 0 | Pass（workspace `rt_` filter で 329 passed） |
| driver build lane | `mise run build-vaudio-driver` | 1 | 環境不足により NotRun 相当。MSBuild はあるが `WindowsKernelModeDriver10.0` toolset が無く MSB8020。driver source correctness の build evidence は provisioned workflow 側で確認 |
| canonical repository hooks | `mise exec -- prek run --all-files` | 0 | Pass |
| controller static validation | `tools/virtual-audio-driver/Invoke-XrunCases.ps1 -ValidateOnly` | 0 | Pass（static validation only; runtime evidence ではない） |

直接 `cargo test --locked -p ruviform-audio-device` と直接 `cargo test --locked -p ruviform-audio-device --features real-device --test virtual_endpoint` は補助証拠であり、canonical lane 代替ではない。両方とも exit 0 だったが、PR 判定には上記 canonical invocation を使う。

### 統合検証 / provisioned 実測

| lane | run / SHA | exit/status | 結果 |
| --- | --- | ---: | --- |
| provisioned virtual-audio-driver-probe driver build | `45932170760` / `f7b2029a` | workflow completed | driver build成功。最初の run `45930911311` では `BOOL`→`BOOLEAN` warning-as-error を検出し、`f7b2029a` で修正済み |
| provisioned xrun cases | `45932170760` / `f7b2029a` | workflow completed | xrun stage は positive/negative とも oracle Fail。これは acceptance Pass ではなく、原因調査の実測結果として記録する |
| merge後対象 SHA の CI / Daily | なし | NotRun | この repository workflow は PR/push trigger を持たず、branch `test/1009-xrun-cause-investigation` の CI run は 0 件。merge 後 SHA は未発生のため、post-merge CI/Daily は coordinator 側の後続判定である |
| pstack Runtime Verification | 本 worker | Run | canonical lane、xrun filter lane、provisioned workflow の実 artifact 観測を実施。結果は上表のとおりで、xrun oracle は Fail を Fail として記録した |
| pstack Formal Verification | 本 worker | NotRun | 本 slice には対応する formal verification artifact/skill が存在しない。driver ETW/event 観測の runtime 調査であり、formal proof を追加する scope ではない |

これらは local validation と provisioned/integrated validation を混同しない。特に positive/negative case Fail を Pass と読み替えない。

## run 45932170760 の追加事実

- driver build、endpoint provisioning、registry write/readback/cleanup、per-case trace capture は成立した。
- positive/negative とも injection applied=true で harness は完走したが、oracle は従前どおり Fail だった。positive detail は `typed-xrun-missing;clock-discontinuity-missing;forward-frame-position-gap-missing;max-forward-gap-zero` に限定され、gap 不在時の before/after-injection 誤記は消えた。
- input callbacks は 300/299、output callbacks は 303/302、position transitions はすべて contiguous（forward/backward/reset 0）だった。error trace は空で typed xrun も backend error もなかった。
- zero-run diagnostics は leading zero（positive 529 frames、negative 961 frames）のみ大きく、interior zero-run は最大 14 frames、trailing は 0 だった。注入 3 packet（約1440 frames）に相当する interior zero span は観測されなかった。
- driver trace ETL/XML は capture されたが、XML には session header のみで `0x1001`-`0x1005` の IHV event は一件も無かった。`EventsLost=0` であり、provider log は `Microsoft-Windows-Audio` GUID `ae4bd3be-f36f-45b6-8d21-bdd6fb832853` が `audioses.dll` の user-mode provider であることを示す。したがって本 slice の driver ETW collection は provider 選定不一致により成立しなかった（H7の具体因）。

## 残課題と後続 slice の境界

- H1（registry snapshot/load timing）、H2（target pin 判定）、H3（suppression window）、H4（notification suppression → OS packet skip）、H6（content-gap byte span/zero-run detector）は未解決である。本 PR はこれらを確定したと主張しない。
- 次件は kernel driver が実際に emit する ETW provider GUID/manifest を同定し、controller の logman provider 指定をその正しい provider へ修正することである。`Microsoft-Windows-Audio` は user-mode `audioses.dll` provider であり、今回の IHV-defined driver event 取得には使えないことが run `45932170760` で判明した。
- capability matrix/runbook の実測更新、および output render underrun の調査/対応は後続 slice であり、本 PR の scope ではない。本 PR は xrun cause investigation の観測面と証跡整備に限定する。

## 結論

確実に排除できるのは H5（adapter mapping defect）である。run `45932170760` の harness diagnostics は、WASAPI `frame_position` が全 contiguous で error callback が空であり、adapter へ forward gap が入力されていないことを強く支持する。ただし driver 内部イベントが provider 選定不一致で採取できなかったため、H1-H4/H6 の runtime 判定は未完了である。次の修正対象は oracle ではなく、kernel driver が実際に emit する ETW provider GUID/manifest の同定と controller の provider 指定である。

1. `0x1001` が無い/値が disabled → H1 または registry snapshot/load timing。
2. `0x1001` はあるが target flag が 0 → H2。
3. target 1 でも `0x1003` suppression transition が無い → H3。
4. suppression があるのに `0x1005` OS packet gap が無い → H4。notification suppression は shared-mode engine の packet skip を生まない設計不整合。
5. `0x1005` があるのに harness/adapter position gap が無い → WASAPI engine position accounting または adapter 直前の observation loss。adapter 本体の mapping は H5 で排除済み。
6. `0x1004` zero boundary があるのに zero-run diagnostics に interior gap が無い → H6 の detector/PCM alignment。zero boundary が無ければ scenario application 側。

この evidence は原因確定ではなく、原因を一意に切り分けるための観測面を追加した調査 slice の記録である。
