# rf#1034 backend timestamp OS/driver observation evidence

- issue: `scarecrow173/ruviform#1034`（closeout ではなく observation slice。PR は `Refs #1034`）
- branch: `test/1034-backend-os-observation`
- base main: `446afc6abf9d88f344954db21afa737df34771ea`（PR #1044 merge commit）
- 最終 main 取り込み: `092e18e30110b889bcc6cec3720136c7463a840c` を merge した。provisioned measurement SHA は各 run matrix の head SHA であり、main merge 後の最終 branch SHA と区別する
- worker / dispatch: `orca_session_id:05640de3-beef-4899-aed7-1ea1afec3b3d` / `ctx_51a8e32eab34`
- 実施日: 2026-11-27 UTC
- workflow: `.github/workflows/virtual-audio-driver-probe.yml`、manual `workflow_dispatch`、runner `windows-2025`、attempt はすべて 1

本 slice は製品修正ではなく、rf629 audio render→capture round-trip の output timestamp regression が再現したとき、consumer-visible callback series と Windows Audio/OS side event、可能なら PortCls miniport driver event を同じ raw artifact で分離する観測 slice である。fresh QPC 置換、timestamp synthesis/smoothing/clamping、driver redesign、oracle 緩和、workflow 変更は行っていない。

## 読んだ前提資料と修正しない判断

Issue #1034、`docs/specs/issue-1034-render-timestamp-contract.md`、rf1034 baseline/layer-isolation/mid-epoch/round-trip-output-diagnostics evidence、run `45952355037` の抽出 artifact、`os_endpoint.rs`、cpal 0.18.2 `host/wasapi/stream.rs` / `timestamp.rs`、`MiniportStreamAudioEngineNode.cpp`、`minwavertstream.cpp`、`common.cpp`、`adapter.cpp`、Microsoft Learn の `IAudioClock::GetPosition` / `KSAUDIO_PRESENTATION_POSITION` / `IPortClsEtwHelper::MiniportWriteEtwEvent` を確認した。

WASAPI client 面の `qpc_position` は 100 ns units へ変換済みで、driver boundary の `KSAUDIO_PRESENTATION_POSITION.u64QPCPosition` は `KeQueryPerformanceCounter` 系 counter value である。本環境の ETW header frequency は 10,000,000 Hz であり数値上は 100 ns tick と一致するが、単位契約は混同しない。merged contract は adapter が backend-reported timestamp を合成・平滑化せず渡すことを要求するため、fresh-QPC fix や単調化は backend provenance を隠す修正として採用しなかった。

## driver diagnostic ETW schema

`drivers/rvf-virtual-audio/EndpointsCommon/MiniportStreamAudioEngineNode.cpp` の変更は additive diagnostic ETW のみである。既存 `eMINIPORT_GET_PRESENTATION_POSITION` event、position calculation、`KeQueryPerformanceCounter` 呼出位置、`m_PositionSpinLock`、DPC/timer、state machine、write/read position、loopback PCM path、notification suppression、xrun/fault 制御は変更していない。新規 emit では新たな lock、allocation、sleep/block、floating point を使わない。

`GetPresentationPosition` は既に `GetPositions` が返す `timeStamp.QuadPart` を `_pPresentationPosition->u64QPCPosition` へ写している。追加 event はこの計算済み値を複写するだけで、raw QPC を monotonic に加工しない。

schema は `eMINIPORT_IHV_DEFINED` の IHV payload として固定した。

| field | 内容 |
| --- | --- |
| Data1 | low 16 bit = schema id `0x2001`、bits 16..63 = raw performance-counter frequency |
| Data2 | bit 0 = capture、bits 8..15 = pin、bits 16..31 = device type |
| Data3 | presentation position blocks |
| Data4 | raw `KeQueryPerformanceCounter` counter value |

harness 側は同じ packing/decoding helper を unit test で round-trip 検証した。XML scan では low 16 bit だけでなく、decoded frequency が 100,000..100,000,000 Hz の範囲にあることを要求する。初版 scanner は timestamp 本文中の `8193` や low bit が一致する通常数値を schema event と誤認する余地があり、run 1 の `rvf_position_events=12` はこの false positive だった。最終 scanner ではこの誤認を排除した。

ETW emit は observer effect を持ち得る。instrumented driver 上の Pass/non-reproduction は uninstrumented baseline の修正証拠ではない。

## Windows backend trace control

`os_endpoint.rs` では Windows 限定で、rf629 pair attempt の stream open/start 前に `logman.exe query providers` を保存し、`logman.exe` session を開始する。stream drop と round-trip diagnostics emission の後に stop/delete し、`tracerpt.exe` で XML 化する。RT callback 内で process spawn、COM/WASAPI 追加呼出、ETW parse、file I/O、allocation を追加していない。session 名は `rvf1034_bt_<pid>_<pair>`、ETL cap は 64 MiB、clock type は `perf`、同時 session は 1 つだけである。

artifact path は workflow を変更せず、`GITHUB_WORKSPACE` 配下の既存 `os-endpoint-run-metadata-windows-2025` に書く。local/non-Windows/artifact dir 不在では honest `Unsupported` であり、case oracle の Pass/Fail を変えない。`tracerpt` 失敗時は raw ETL を残し、decode-based attribution は `Unsupported` とする設計である。

provider 選択の変遷は次のとおりである。

1. run 1 (`a1e96ca7`): Microsoft-Windows-Audio と Microsoft-Windows-KernelStreaming。driver event は採れず、scanner false positive で coverage を過大表示した。
2. run 2 (`b32d1d05`): AudioEngineVerbose GUID `{A6A00EFD-21F2-4A99-807E-9B3BF1D90285}` と portcls.sys 内 GUID `{10EB6007-818C-4DB6-A694-B518E589D07A}` を追加したが、keyword/level filter は 5/nonzero で、PortCls event は 0 件だった。
3. run 3 (`9f01fb1e`): core-audio trace guidance に合わせ level `0xff`、match-any keyword `0x0`（keywordless event を除外しない）へ変更したが、PortCls event はやはり 0 件だった。
4. run 4 (`3c837b14`): WMI `PortCls_AudioEngine` / `Miniport_AudioEngine_Event` class qualifier `{97BCD203-6286-4974-801D-A293B921F84B}` を追加し、control GUID candidate `{10EB6007...}` と区別して観測する。

`wevtutil gp` は manifest-registered な Microsoft-Windows-Audio / KernelStreaming では取得でき、AudioEngineVerbose / PortCls candidate では exit 2 を honest に記録する。delete exit `-2144337918` は stop 済み session に対する not-found 系で、stop exit 0 と cleanup residue 0 を優先して解釈する。trace failure/coverage incomplete は diagnostic status であり、oracle を変えない。

calibration では trace start/stop 前後の host `Instant` elapsed nanoseconds、Unix system time nanoseconds、ETW header `PerfFreq`/`StartTime` を記録する。ETW perf clock と host `Instant` の同一性は独立 evidence がない限り unresolved と明記する。ただし run 2/3 では OS `SetCurrentTimeStamp.Qpc` と consumer callback `callback_ns/100` が同じ 10 MHz scale で直接比較可能だった。

## provisioned serialization

各 dispatch 前に Orca worker-list、全 branch の同 workflow `in_progress` / `queued`、local/remote branch head 一致を確認した。Orca 上には別 workspace の live worker がいたが、rf1034/rf1009/driver/os_endpoint lease やこの workflow に競合する worker は確認できず、GitHub 上の active/queued provisioned run は 0 だった。同時実行は 1 本だけで、foreground `gh run watch <run-id> --exit-status`、artifact download、抽出を終えてから次を dispatch した。workflow conclusion ではなく case-level RunMetadata/tagged diagnostics/backend trace statusで判定する。

## provisioned run matrix

byte size と SHA-256 は各 run directory の `artifact-inventory.json` を正本とする。text/XML/JSON は repository hygiene のため LF/行末空白を正規化し、ETL は binary のまま保存した。hash は抽出ファイルに対するもので、原本 artifact は GitHub Actions run に残る。

| run / artifact id | head SHA | dispatch UTC / watch exit | driver-build | rf1034 | rf629 audio | raw/effective idx | backend trace coverage | 判定 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [`45966722893`](https://github.com/scarecrow173/ruviform/actions/runs/45966722893) / `15182804591` | `a1e96ca7dbb7872fc339efdbf76c0d8441c3098b` | 2026-11-27T08:34:30Z / 0 | ok | pass | pass | 4/0 `[3,4,304,316]` | harness 上は false positive `driver-diagnostic-event-present`、再 scan では 0 | bounded non-reproduction + trace pipeline captured、coverage incomplete |
| [`45968714712`](https://github.com/scarecrow173/ruviform/actions/runs/45968714712) / `15183699374` | `b32d1d05be70df072864c7b153a6acb9b970ffc8` | 2026-11-27T08:49:50Z / 0 | ok | pass | fail | 4/2 `[3,4,47,79]` | `incomplete-no-driver-event-decoded` | reproduction with incomplete backend observation |
| [`45970610888`](https://github.com/scarecrow173/ruviform/actions/runs/45970610888) / `15184578552` | `9f01fb1e361a33d2eaa521fa29530fc058732de5` | 2026-11-27T09:04:04Z / 0 | ok | pass | fail | 4/3 `[3,57,86,328]` | `incomplete-no-driver-event-decoded` | reproduction with incomplete backend observation |
| [`45972492570`](https://github.com/scarecrow173/ruviform/actions/runs/45972492570) / `15185318979` (`sha256:fa313592ca45e5c09c20b8cc11b9cf63564b3d64f33c4488c7224182803c7514`) | `3c837b147a0960b1a399863afad0defbb5ca2dfa` | 2026-11-27T09:17:34Z / 0 | ok | pass | fail | 4/2 `[3,4,116,242]` | `incomplete-no-driver-event-decoded` | reproduction with incomplete backend observation |

run 1 は rf629 pass/effective 0 であり、complete reproduction ではない。raw regression `[3,4]` は startup grace 内、`[304,316]` は epoch 1 側であり、非再現を修正証拠とはしない。run 2/3/4 はいずれも startup grace外 effective regression を持つが、driver/PortCls event が 0 件なので spec の complete observed reproduction ではなく `reproduction with incomplete backend observation` である。3 run連続で Microsoft-Windows-Audio は captured なのに PortCls/miniport coverage が incomplete だったため、残り budget の run 5 は同じ environmental coverage gap を消費するだけと判断し dispatch しなかった。

## run 45968714712: regression window と OS correlation

rf629 metadata は `result=fail`、`observed=no pair candidate completed the round-trip: pair[0]: output timestamp regressions: 2` だった。summary は output callbacks 332、frames 158,976、raw/effective `4/2`、effective idx `47/79`、max backstep `-9,711,000` ns（startup grace内 idx 3）、playback some 332、overlap 0、timestamp log overflow 0、input/output error 0 だった。

effective diagnostics は次のとおりである。

```text
index=47 epoch_offset=47 startup_grace=0 backstep_ns=-474500 backend_two_step_ns=18648200 host_two_step_ns=28574600 supply_two_step_frames=960 playback_lead_before_ns=30875000 playback_lead_ns=40895834 playback_lead_after_ns=31895834
index=79 epoch_offset=79 startup_grace=0 backstep_ns=-1364100 backend_two_step_ns=18854200 host_two_step_ns=28798400 supply_two_step_frames=960 playback_lead_before_ns=30875000 playback_lead_ns=42895834 playback_lead_after_ns=31895834
```

consumer callback series は idx 47 で `652604000700 → 652603526200` ns（`-474,500` ns）へ逆行し、idx 48 で `652622648900` ns へ追いついた。idx 79 でも `652923544000 → 652922179900` ns（`-1,364,100` ns）へ逆行し、idx 80 で `652942398200` ns へ追いついた。host monotonic interval は正、supply cursor は連続、playback presence は継続し、playback lead は約 30.9 ms から 40.9/42.9 ms へ跳んで戻る re-anchor 形だった。

`regression-correlation.json` で Microsoft-Windows-Audio `SetCurrentTimeStamp` を objInstance ごとに走査した結果、両 object とも Qpc/DevicePosition/StreamPosition の negative step は 0 だった。run 2 の object `0x26315B062D0` は 330 events、`0x26315B01370` は 329 eventsで、いずれも単調だった。idx 47 window でも idx 79 window でも OS correlation event の Qpc は逆行していない。

この事実は、consumer-visible callback backward step が OS `SetCurrentTimeStamp` correlation seriesそのものの逆行ではないことを示す。ただし `SetCurrentTimeStamp` は audio engine内部の correlation/update event であり、driver raw QPC の直接証拠ではない。object の render/capture 識別も driver direction/pin event がないため unresolved である。

## run 45970610888: regression window と OS correlation

rf629 metadata は `result=fail`、output timestamp regressions 3 だった。summary は output callbacks 333、frames 159,456、raw/effective `4/3`、effective idx `57/86/328`、max backstep `-10,045,700` ns（startup grace内 idx 3）、playback some 333、overlap 0、timestamp log overflow 0、input/output error 0 だった。idx 328 は epoch boundary 303 後の epoch 1 offset 25 で、epoch-local startup grace 24 を超えているため effective である。

run 3 の `regression-correlation.json` でも、Microsoft-Windows-Audio `SetCurrentTimeStamp` は両 object で Qpc/DevicePosition/StreamPosition negative step 0 だった（`0x1E02D0062D0`: 331 events、`0x1E02D001370`: 329 events）。したがって run 2 と独立に、consumer callback の backward-then-catch-up に対して OS correlation series は単調だった。

## run 45972492570: regression window と OS correlation

rf629 metadata は `result=fail`、output timestamp regressions 2 だった。summary は output callbacks 331、frames 158,496、raw/effective `4/2`、effective idx `116/242`、max backstep `-10,989,300` ns（startup grace内 idx 3）、playback some 331、overlap 0、timestamp log overflow 0、input error 2/input xrun 1/input discontinuity 1、output error 0 だった。input discontinuity は regression 観測そのものを否定しない。

effective diagnostics は idx 116 で `-2,101,400` ns、idx 242 で `-1,384,400` ns backward stepだった。どちらも epoch 0、startup grace外で、host monotonic intervalは正、supply cursorは 960 frame/2 callbacksで連続し、playback leadは約 30.9 ms から 42.9 ms へ跳んで戻る re-anchor 形だった。例えば idx 116 は `556145881100 → 556143779700 → 556163517600` ns、idx 242 は `557404996400 → 557403612000 → 557424288100` ns だった。

run 4 の `regression-correlation.json` でも、Microsoft-Windows-Audio `SetCurrentTimeStamp` は両 object で Qpc/DevicePosition/StreamPosition negative step 0 だった（`0x1AE57BF62D0`: 329 events、`0x1AE57BF1370`: 328 events）。regression window ±200,000 100ns 内にも negative step はなかった。WMI event GUID `{97BCD203-6286-4974-801D-A293B921F84B}` を追加しても PortCls event countは 0、additive rf schema eventは 0、standard miniport presentation-position eventは 0 だった。raw ETL binary scanでも `{97BCD203...}`、control candidate `{10EB6007...}`、root PortClsEvent `{9502CBC6-AA74-4EFF-BA91-D9329BCCE758}` は出現しなかった。

## backend attribution

driver/PortCls event が採れていないため、spec の complete observed reproduction ではない。それでも run 2/3 の Microsoft-Windows-Audio verbose inventoryから、次の bounded attribution が言える。

| layer | 判定 | 根拠 |
| --- | --- | --- |
| driver position generation | **unresolved / weakly weakened** | OS `SetCurrentTimeStamp.DevicePosition`/`StreamPosition` は regression window で単調だったが、audio engine が interpolation/correlation後の値を記録している可能性があり、driver raw presentation position を直接観測していない。eliminated ではない。 |
| driver raw QPC correlation | **unresolved** | additive diagnostic eventも standard miniport eventも trace に現れず、raw `KeQueryPerformanceCounter` seriesを直接観測できていない。absence は driver innocence の証拠にしない。 |
| WASAPI/audiodg/PortCls correlation/conversion | **strengthened** | driver raw inputが未観測でも、OS `SetCurrentTimeStamp` correlation seriesが単調なのに consumer callbackだけが backward-then-catch-up で逆行し、playback leadが 30.9ms→40.9/42.9ms→31.9msへ re-anchor した。OS boundary内の extrapolation/re-anchor/correlation conversion が consumer-visible regression生成に関与する説が強い。 |
| cpal conversion | **weakened / unresolved** | cpalは `IAudioClock::GetPosition` の `qpc_position` を `*100`するだけで、単調 inputを conversion単独で逆行させる経路は見えない。ただしcpalへの実入力QPCは直接観測していない。 |
| adapter passthrough | **weakened / unresolved** | PR #1044の deterministic witnessどおり backend backward stepを clamp/smoothing/synthesisせず保持する。runtimeのraw backend値まで遡れないため unresolvedは残る。 |
| render scheduling / environment | **unresolved / trigger possibility** | regression直前にhost callback集中があり得るが、host intervalは正、max active callbacks=1、overlap=0、input/output error=0。ETW observer effectも否定できない。triggerとしては残るが主原因とは確定しない。 |
| test oracle / consumer harness | **weakened** | supply cursor連続、frame/channel mismatch、log overflow、epoch misclassification、diagnostic欠損はない。oracleは既存定義どおりFailにしており、harnessが系列を生成した説は弱い。 |

## trace coverage gap の評価

run 1〜4では raw ETL/XML に Microsoft-Windows-Audio events が多数入り、`AE Position`、`AE Timestamp`、`SetCurrentTimeStamp`、glitch inventoryは取得できた。一方で KernelStreaming、AudioEngineVerbose、PortCls candidateの event countは 0、standard miniport presentation-position eventも 0、additive rf schema eventも 0 だった。ETL binary scanでもPortCls candidate GUIDは出ず、tracerpt decode問題だけではなく capture自体が成立していない可能性が高い。

この absence は driver innocence の証拠ではない。可能性としては、(1) PortCls classic/WMI providerの control/event GUID対応が違う、(2) `MiniportWriteEtwEvent` が glitch/context buffering用途で通常position呼出では永続traceへ出ない、(3) virtual driverの `GetPresentationPosition`/write-position pathが想定ほど呼ばれていない、(4) session enablementがkernel classic providerへ届いていない、が残る。WMI class qualifier では `{97BCD203...}` が `GuidType=Event`、`{10EB6007...}` が binary 上の PortCls/WMI trace関連GUIDであることが確認できたが、run 4では両方とも event を生じなかった。`IPortClsEtwHelper::MiniportWriteEtwEvent` の引数説明は、position/write-position/pin-stateなどを context として保持し、glitch 時や error 系で有用な情報とする設計を示唆する。したがって通常 round-trip の presentation-position 呼出ごとに永続 ETW event が出る前提は成立しない可能性が高い。少なくとも本 harness/provider enablement では、3 run連続で同じ coverage unsupported/incomplete を再現したため、lease内の provider 追加だけでは解決できない environmental/driver-observability gap として次 slice へ引き継ぐ。

## local canonical verification

canonical は mise タスクであり、直接 cargo 実行は補助証拠である。OS は local Windows。driver-build lane の local failure は provisioned driver-build stage と区別する。

| command | exit | 判定 |
| --- | ---: | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-audio-test` | 0 | Pass |
| `mise run lint ruviform-audio-test` | 0 | Pass（clippy `-D warnings`） |
| `mise run test ruviform-audio-test` | 0 | Pass。unit/doc tests pass、backend trace focused tests 5/5 pass |
| `mise run build-vaudio-driver` | 1 | Unsupported/NotRun 相当。local VS2022には `WindowsKernelModeDriver10.0` toolsetがなく MSB8020。driver build Passと読まない。provisioned runのdriver-build stageは各runとも `ok` で、instrumented `RvfVirtualAudio.sys` build/sign/installが実行された |
| `mise run test-os-endpoint-wiring` | 0 | process exit 0だが manual endpoint tests は 0.06sで、local virtual endpoint provisioningがないため runtime Passではなく NotRun相当 |
| `mise exec -- prek run --all-files` | 0 | Pass。初回は endpoint-probe 4件の終端 newline を修正したため、修正後に再実行して exit 0。修正後の byte size/SHA-256 は新規 run inventoryへ反映した |

## raw artifact inventory

各 run directoryには少なくとも endpoint probe、vaudio probe、driver-build/os-endpoint/cleanup stage log、rf1034/rf629 audio/rf629 midi RunMetadata、raw ETL、tracerpt XML、provider discovery、provider list、wevtutil/logman/tracerpt logs、summary JSON、`roundtrip-analysis.json`、`backend-os-observation.json`、`artifact-inventory.json` を保存した。run 2/3には `regression-correlation.json` もある。hash mismatchや欠損は inventory の `missing` が空であることで確認する。normalized JSONは raw ETL/XML/stage logを上書きしない。

## 製品修正判断と次sliceへの引き継ぎ

本 slice で製品修正は行わない。position/QPC correlation、timer/DPC、write/read position、loopback path、state machine、cpal patch、adapter contract、oracle/classifier/startup grace、workflow/provisioning/xrun toolingは変更していない。

現時点の最有力な次slice候補は、consumer callback regressionと playback lead re-anchorを生む WASAPI/audiodg/OS correlation/conversion 層の直接観測である。ただし driver raw position/QPCが未観測なので、driver position generation/raw QPCを排除してはいけない。次は少なくとも次のどちらかが必要である。

1. PortCls classic/WMI eventの正しい enable/decode経路を確定し、run regression windowで additive rf schema eventまたは standard miniport eventを採取する。
2. それが不可能なら、PortCls ETWに依存しない additive driver observation（ただし製品挙動を変えない範囲）を別途設計し、raw position/QPC/pin/directionを regression windowと時間相関させる。

Synvelis consumer round-tripの Pass/Failは本 upstream evidenceに転用しない。

## actions.jsonl との対応

本 evidenceに対応する action記録は `evidence/rf1034/actions.jsonl` の backend-os-observation seriesを参照する。
