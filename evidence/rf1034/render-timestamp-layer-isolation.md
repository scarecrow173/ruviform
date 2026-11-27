# rf1034 render callback timestamp layer isolation evidence

## 対象と実施条件

- 対象 Issue: `scarecrow173/ruviform#1034`
- branch: `test/1034-timestamp-layer-isolation`
- implementation/spec commit: `5843719861fa8ca6da5c963a3f63c3b32047c4ff`
- provisioned workflow run: `45865926990` attempt `1`
- run URL: https://github.com/scarecrow173/ruviform/actions/runs/45865926990
- artifact: `virtual-audio-driver-probe-windows-2025-attempt-1`
- 測定対象 head SHA: `5843719861fa8ca6da5c963a3f63c3b32047c4ff`
- runner OS: GitHub-hosted `windows-2025` x64
- workflow 実施時刻: 2026-11-26T17:13:50Z 開始、job `vaudio-probe` は 7m17s
- watch: foreground `gh run watch 45865926990 --exit-status`、exit code 0
- 比較 baseline: run `45851204401`（branch `test/1034-timestamp-contract-baseline`、SHA `193b52e3348e2540f16f9ecfa290f75298226de5`）

この文書は layer isolation slice の evidence である。adapter/driver/cpal/workflow は変更していない。Run `45865926990` の rf1034 case は `Pass` だが、これは修正検証ではない。本 slice は timestamp 生成系を一切修正しておらず、baseline run `45851204401` の mid-epoch regression が今回出なかったという非決定性の観測にすぎない。

## canonical local verification

これらは branch 上で実行したローカル検証である。ローカル検証は変更範囲の検証であり、merge 後の統合検証ではない。

| コマンド | exit code | verdict | 要点 |
| --- | ---: | --- | --- |
| `mise run fmt-check` | 0 | Pass | rustfmt check 完了 |
| `mise run check ruviform-audio-test` | 0 | Pass | `cargo check --locked --all-targets` 完了 |
| `mise run lint ruviform-audio-test` | 0 | Pass | clippy `-D warnings` 完了 |
| `mise run test ruviform-audio-test` | 0 | Pass | deterministic/unit/doc tests pass。`os_endpoint` では classifier / instrumentation / regression diagnostic unit test を含む 11 test pass、manual endpoint 3 test は ignored |
| `mise run test-os-endpoint-wiring` | 0 | NotRun（runtime evidence ではない） | local host は provisioned virtual endpoint ではなく manual endpoint tests は 0.07s で終了。process exit 0 を runtime Pass と読まない。runtime evidence は provisioned run `45865926990` |
| `mise exec -- prek run --all-files` | 0 | Pass | 最終 evidence 追加後を含む hygiene hooks pass |

補助証拠（canonical = mise ではない）として、実装中に `cargo test -p ruviform-audio-test --test os_endpoint` を直接実行し、新規 unit test を含む 11 test pass・3 ignored を確認した。最終報告は上記 canonical lane に従う。

## provisioned run 実行条件の確認

| 条件 | 観測 |
| --- | --- |
| 同 workflow の active run | dispatch 前に `in_progress=[]`、`queued=[]` |
| 対象 SHA | branch head SHA `5843719861fa8ca6da5c963a3f63c3b32047c4ff` に対して dispatch。main や別 branch SHA で代用していない |
| watch | foreground `gh run watch 45865926990 --exit-status` |
| artifact | run id を指定して download し、rf1034 判定に必要な file のみを `evidence/rf1034/provisioned/run45865926990-attempt1/` へ抽出 |

run `45865926990` の provisioning は成立した。`endpoint-probe`、`stream-roundtrip`、`midi-loopback`、`os-endpoint-wiring`、`cleanup` は `ok` で、cleanup は `leftovers=0`、`errors=0` だった。`os-endpoint-wiring` detail は `test_exit=0; audio=pass; midi=pass; verdicts=3; metadata=3; copied=3` だった。

## rf1034 render-only baseline case result

case: `rf1034.audio.virtual-endpoint.render-timestamp-baseline`

| 項目 | run `45865926990` | baseline run `45851204401` |
| --- | --- | --- |
| case result | **Pass** | **Fail** |
| callbacks / frames | `callbacks=333`、`frames=159456` | `callbacks=331`、`frames=158496` |
| raw regressions / effective regressions | `raw=2`、`effective=0` | `raw=3`、`effective=1` |
| regression index | `[3,4]` | `[3,4,250]` |
| epoch resets / boundary | `epoch_resets=1`、`boundary_cb=302` | `epoch_resets=1`、`boundary_cb=302` |
| max backstep | `-10140900 ns` | `-10116700 ns` |
| playback | `playback_some=333`、`playback_none=0`、`playback_regressions=0` | `playback_some=331`、`playback_none=0` |
| harness overlap | `max_active_callbacks=1`、`overlap_events=0` | 未計測 |
| bounded event log | `events=333`、`overflow=0`、`bounded_slots=512` | `events=331`、`overflow=0`、`bounded_slots=512` |

今回の regression は initial epoch の index 3/4 のみで、既存契約どおり epoch-start grace に分類された。baseline run で Fail になった index 250 の mid-epoch regression は今回出なかった。したがって、再現性は少なくとも timing/environment dependent であり、mid-epoch regression の原因確定には至っていない。

## regression diagnostic の観測

run `45865926990` の診断行は次のとおりである。

```text
render_timestamp_regression_diag: index=3 epoch=0 epoch_offset=3 startup_grace=1 reference_index=2 reference_callback_ns=420593397300 backstep_ns=-10140900 backend_two_step_ns=-521700 host_two_step_ns=19305200 supply_two_step_frames=576 playback_lead_before_ns=0 playback_lead_ns=22000000 playback_lead_after_ns=32000000 playback_interval_ns=11859100 next_playback_interval_ns=19619200
render_timestamp_regression_diag: index=4 epoch=0 epoch_offset=4 startup_grace=1 reference_index=2 reference_callback_ns=420593397300 backstep_ns=-521700 backend_two_step_ns=19681000 host_two_step_ns=19680600 supply_two_step_frames=960 playback_lead_before_ns=22000000 playback_lead_ns=32000000 playback_lead_after_ns=32979167 playback_interval_ns=19619200 next_playback_interval_ns=11040967
```

代表 event は次のとおりである。

| index | callback_ns | interval_ns | host_interval_ns | playback_interval_ns | playback_lead_ns | frames | supply_frame_cursor | regression_backstep_ns |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 420573397300 | none | none | none | 0 | 480 | 0 | none |
| 1 | 420583397300 | 10000000 | 93400 | 10000000 | 0 | 480 | 480 | none |
| 2 | 420593397300 | 10000000 | 60100 | 10000000 | 0 | 96 | 960 | none |
| 3 | 420583256400 | -10140900 | 9692700 | 11859100 | 22000000 | 480 | 1056 | -10140900 |
| 4 | 420592875600 | 9619200 | 9612500 | 19619200 | 32000000 | 480 | 1536 | -521700 |
| 5 | 420602937400 | 10061800 | 10068100 | 11040967 | 32979167 | 480 | 2016 | none |
| 6 | 420612948900 | 10011500 | 10004300 | 9011500 | 31979167 | 480 | 2496 | none |
| 250 | 423053481800 | 10005500 | 10003100 | 10005500 | 31979167 | 480 | 119616 | none |
| 302 | 423573213000 | none | none | none | 22217167 | 480 | 144576 | none |

host monotonic interval は regression callback でも正であり、index 4 から index 5 までの 2 step では backend callback interval `19681000 ns` と host interval `19680600 ns` がほぼ一致する。supply frame cursor は逆行せず、active callback depth は 1、overlap は 0 だった。初期 index 1/2 の host interval が極小なのは stream start 時の priming/buffering を示す観測であり、以後は host/backend interval が約 10 ms に揃う。

## layer hypothesis との照合

### epoch-start re-anchor 説 — 強く補強

index 3/4 は initial epoch 先頭 24 callback 内で起き、`playback` は逆行せず `playback - callback` lead が `0 → 22 ms → 32 ms` へ再構成されている。index 5 以降は lead が約 32 ms に安定し、callback interval も約 10 ms に戻る。この backward-then-catch-up 形は、start 直後に OS 側が position↔QPC correlation を再 anchor した説と強く一致する。ただしこれは mid-epoch regression の説明ではない。

### audiodg / WASAPI position clock bookkeeping 説 — startup について補強、mid-epoch は未確定

過去の rf977/rf952 evidence では、render stream の driver-reported position/counter を STOP 跨ぎで monotonic 化しても regression 件数が変わらなかった。今回も `callback` が逆行する一方で `playback` と host clock は前進し、consumer 側の供給 cursor は連続だった。startup regression については、driver が報告する position bookkeeping よりも audiodg/WASAPI 側の correlation bookkeeping で生じる説が引き続き最も強い。

baseline index 250 の mid-epoch regression は今回再現しなかったため、この run だけでは audiodg の mid-stream 更新、runner scheduling、cpal conversion、driver interaction のどれかを決められない。新 instrumentation は次回再現時に同じ診断値を採取できる状態になった。

### cpal timestamp conversion 説 — 弱められたが完全否定ではない

cpal 0.18.2 WASAPI output path の source inspection では、`callback` は `IAudioClock::GetPosition` の QPC position 由来で、cpal の monotonic wrapper は `playback` のみを clamp し `callback` を clamp しない。今回 `playback_regressions=0` で、`callback` regression は cpal の playback monotonic wrapper が作った/隠したものではない。QPC→nanos conversion は単調写像なので、入力 QPC が単調なら cpal conversion だけでは逆行しない。残る可能性は、cpal が参照する OS 側 position/QPC correlation 自体、または cpal の API 使用条件である。cpal 単独の conversion bug と断定はできない。

### consumer harness 誤判定説 — 弱められた

consumer harness は backend 報告値を補正せず、regression oracle も変更していない。今回の診断値では `max_active_callbacks=1`、`overlap_events=0`、供給 frame cursor の逆行なし、host monotonic interval は正、`playback_regressions=0` だった。これらは harness が複数 callback を重畳して series を壊した、frame cursor を巻き戻した、または playback 欠損を合成したという説を弱める。classification は従来どおり単一の authoritative classifier であり、diagnostic line は Pass/Fail を変えない。

### runner scheduling/environment 説 — mid-epoch では未解決

host clock と backend callback interval が startup catch-up 後にほぼ一致するため、startup regression を単なる harness scheduling delay で説明する根拠は弱い。一方、baseline index 250 は今回出ず、mid-epoch regression が timing/environment dependent であることは示された。再現時に host interval が正で backend only が逆行するか、host/backend 双方が乱れるかを比較するまでは、runner scheduling 説を除外できない。

## 採取 evidence

`evidence/rf1034/provisioned/run45865926990-attempt1/` に、artifact 全体ではなく判定に必要なファイルを抽出した。

| file | 内容 |
| --- | --- |
| `artifact-inventory.json` | 抽出ファイルの byte size と SHA-256、run/artifact/SHA provenance |
| `vaudio-probe-windows-2025.json` | stage 状態。provisioning、`os-endpoint-wiring ok`、cleanup を含む |
| `endpoint-probe-windows-2025.txt` | endpoint 実名、format、pair candidate、OS version |
| `stage-os-endpoint-wiring-windows-2025.txt` | enhanced render timestamp event 333 件、regression diagnostic、rf629 metadata/verdict を含む抽出ログ（repository hygiene のため LF/行末空白正規化） |
| `run-metadata-rf1034-audio-virtual-endpoint-render-timestamp-baseline.json` | rf1034 case `pass` の Case Schema v1 record |
| `run-metadata-rf629-audio-virtual-endpoint-render-capture.json` | 同一 stage の audio round-trip `pass` record |
| `run-metadata-rf629-midi-virtual-endpoint-loopback.json` | 同一 stage の MIDI loopback `pass` record |

## 受入条件対応（本 slice 範囲）

| 受入条件 | 判定 | 対応 evidence |
| --- | --- | --- |
| render callback timestamp の単調性、frame position、capture timestamp との関係、責務分担を spec で確定 | Pass | `docs/specs/issue-1034-render-timestamp-contract.md` に確定範囲/未確定範囲を追記 |
| provisioned Windows virtual endpoint で baseline regression を再現する最小 case、または再現しないことを示す検査 | Pass（両方の観測を記録） | baseline run `45851204401` で index `[3,4,250]` を再現。layer-isolation run `45865926990` では index `[3,4]` のみで mid-epoch は非再現。非決定性を隠さない |
| 原因層の切り分け、修正または契約更新につながる evidence | Pass（切り分け途中を明示） | consumer harness/capture loopback 起因否定を維持。startup は OS/WASAPI/audiodg correlation re-anchor 説を強化。cpal playback wrapper 起因は否定。mid-epoch の最終帰属は未確定 |
| regression-count-zero 修正検証 | NotRun/未達 | 本 slice は adapter/driver/cpal を変更しておらず、今回の Pass を修正証拠にしない |
| Synvelis consumer round-trip の Pass/Fail を upstream evidence に転用しない | Pass | 本 evidence は Synvelis を使っていない |

## 残件

- mid-epoch regression 再現時の enhanced diagnostic 採取。`host_two_step_ns`、`backend_two_step_ns`、`playback` lead/interval、供給 cursor、overlap を比較して driver/OS/cpal/runner の最終帰属を行う
- adapter/device/stream 実装が必要に見える変更は本 slice では実施していない。Coordinator 判断後の後続 slice
- `docs/**/README.md` の docs index は本 slice で編集・再生成していない。Coordinator が統合時に整備する
