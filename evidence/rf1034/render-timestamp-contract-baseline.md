# rf1034 render callback timestamp contract / baseline evidence

## 対象と実施条件

- 対象 Issue: `scarecrow173/ruviform#1034`
- branch: `test/1034-timestamp-contract-baseline`
- 作業開始時の main SHA: `9578713cd03d3b5c89dc32125a8bcba4c0685885`
- implementation/spec commit: `193b52e3348e2540f16f9ecfa290f75298226de5`
- provisioned workflow run: `45851204401` attempt `1`
- run URL: https://github.com/scarecrow173/ruviform/actions/runs/45851204401
- artifact: `virtual-audio-driver-probe-windows-2025-attempt-1`
- 測定対象 head SHA: `193b52e3348e2540f16f9ecfa290f75298226de5`
- runner OS: GitHub-hosted `windows-2025` x64（endpoint probe 観測 OS `10.0.26100.33828`）
- workflow 実施時刻: 2026-11-26T15:39:06Z 開始、job `vaudio-probe` は 7m1s
- watch: foreground `gh run watch 45851204401 --exit-status`、exit code 0
- artifact download: 2026-11-26T15:47Z 頃、run id を指定して取得
- 実施者/Worker: Orca Worker `orca_session_id:8ef5a474-2ae8-4d61-a113-8ecad464568f`

この文書は render callback timestamp contract slice の evidence である。Issue #1034 を close する結論ではなく、後続の driver / adapter / OS 境界切り分けに使う baseline を確定する。workflow/job conclusion の `success` を case-level Pass と混同しない。実際には `os-endpoint-wiring` stage record は `error`、rf1034 baseline case は `Fail` だった。

## provisioned run 実行条件の確認

本 slice の provisioned run は、実行条件を満たしたため実施した。

| 条件 | 観測 |
| --- | --- |
| rf#1009 dispatch `ctx_cf4fabe8206d` | `state=succeeded`、`stage=settled`、`terminalState=released`。完了・release 済みを Orca の正本で確認 |
| 同 workflow の active run | dispatch 前に `in_progress=[]`、`queued=[]` |
| 対象 SHA | branch `test/1034-timestamp-contract-baseline` の head SHA `193b52e3348e2540f16f9ecfa290f75298226de5` に対して dispatch。main や rf#1009 branch の SHA で代用していない |
| watch | foreground `gh run watch 45851204401 --exit-status`、background watch は使っていない |

## canonical local verification

これらは provisioned dispatch 前に branch 上で実行したローカル検証である。ローカル host は provisioned Ruviform endpoint を持たないため、manual lane の process exit 0 を runtime Pass とみなしていない。

| コマンド | exit code | verdict | 要点 |
| --- | ---: | --- | --- |
| `mise run fmt-check` | 0 | Pass | rustfmt check 完了 |
| `mise run check ruviform-audio-test` | 0 | Pass | `cargo check --locked --all-targets` 完了 |
| `mise run lint ruviform-audio-test` | 0 | Pass | clippy `-D warnings` 完了 |
| `mise run test ruviform-audio-test` | 0 | Pass | deterministic/unit/doc tests pass。`os_endpoint` では classifier / instrumentation unit test を含む 7 test pass、manual endpoint 3 test は ignored |
| `mise run test-os-endpoint-wiring` | 0 | NotRun（runtime evidence ではない） | local host では rf1034 baseline が `NOT_PROVISIONED — no virtual render endpoint candidate`、rf629 audio/midi も NotRun。test process exit 0 を Pass と読まない |
| `mise exec -- prek run --all-files` | 0 | Pass | trailing whitespace、EOF、YAML/TOML、mixed line ending、BOM、private key、actionlint hooks pass |

補助証拠（canonical = mise ではない）として、実装中に `cargo test -p ruviform-audio-test --test os_endpoint` を直接実行し、最初に classifier/instrumentation test が未実装で失敗すること（RED）と、実装後に通ることを確認した。最終報告は上記 canonical lane に従う。

## provisioned endpoint / stage provenance

run `45851204401` の provisioning は成立した。`vaudio-probe-windows-2025.json` では driver-build、sign-verify、pnputil-add-driver、devnode-create、endpoint-probe、stream-roundtrip、midi-loopback、cleanup が `ok` で、cleanup は `leftovers=0`、`errors=0` だった。

endpoint probe は次を観測した。

- render: `Ruviform Cable Input (Ruviform Virtual Audio Device)`、active、F32/2ch/48k shared supported
- capture: `Ruviform Cable Output (Ruviform Virtual Audio Device)`、active、F32/2ch/48k shared supported
- pair candidate: stem `ruviform cable ruviform virtual audio device`、pair count 1

この名前一致は probe の注記どおり candidate provenance であり、それだけで virtual endpoint を証明しない。ただし driver install/devnode/endpoint enumeration/cleanup まで同一 run の stage evidence が揃っている。

## rf1034 render-only baseline case result

case: `rf1034.audio.virtual-endpoint.render-timestamp-baseline`

| 項目 | 観測 |
| --- | --- |
| case result | **Fail** |
| stage | `os-endpoint-wiring` は `error`（`test_exit=101`）。workflow job 自体は absence-is-data contract に従い最後まで完走し conclusion `success` |
| callbacks / frames | `callbacks=331`、`frames=158496` |
| raw regressions / effective regressions | `raw=3`、`effective=1` |
| regression index | `[3,4,250]` |
| epoch resets / boundary | `epoch_resets=1`、`boundary_cb=302` |
| max backstep | `-10116700 ns` |
| playback | `playback_some=331`、`playback_none=0` |
| bounded event log | `events=331`、`overflow=0`、`bounded_slots=512` |
| RunMetadata | `result=fail`、`repo=scarecrow173/ruviform`、`sha=193b52e3348e2540f16f9ecfa290f75298226de5` |

regression のうち index 3/4 は initial epoch の先頭 24 callback 内で、既存契約どおり startup transient に分類された。index 250 は同じ initial epoch の深部であり、epoch boundary `302` より前なので startup grace では除外されず、effective regression `1` として Fail になった。oracle を緩めていない。

代表イベントは次のとおりである。

| index | epoch | callback_ns | playback_ns | interval_ns | regression_backstep_ns |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 2 | 0 | 607996523500 | 607996523500 | 10000000 | none |
| 3 | 0 | 607986406800 | 608008406800 | -10116700 | -10116700 |
| 4 | 0 | 607996436500 | 608028415667 | 10029700 | -87000 |
| 249 | 0 | 610447610500 | 610478318834 | 10000000 | none |
| 250 | 0 | 610446155500 | 610489093000 | -1455000 | -1455000 |
| 251 | 0 | 610466291100 | 610498270267 | 20135600 | none |
| 302 | 1 | 610986156100 | 611008156100 | none | none |

index 250 では `callback` が 1.455 ms 逆行した一方、`playback` は index 249 の `610478318834` から index 250 の `610489093000` へ進んでいる。これは playback timestamp 欠損や harness 側 frame position 合成では説明できない、backend 報告 callback instant の逆行である。

## 同一 run の rf629 round-trip との関係

同一 `os-endpoint-wiring` stage では、rf629 audio round-trip と MIDI loopback は `Pass` だった。audio round-trip の timestamp summary は次である。

- input: `regressions=0 effective=0 epoch_resets=1 boundary_cb=298`
- output: `regressions=1 effective=0 epoch_resets=1 boundary_cb=303 first..last=3..3 max_backstep_ns=-9910100 idx=[3]`

したがって round-trip Pass は timestamp regression が消失したことを意味しない。output regression は index 3 のみで startup grace 内だったため effective 0 だった。rf1034 baseline では index 250 の mid-epoch regression が出たため Fail である。両者は矛盾せず、bounded grace の挙動と流動的な OS timestamp bookkeeping を示している。

## 原因層の切り分け判断

本 run で確定したことは次のとおりである。

1. **capture loopback は再現の必要条件ではない。** rf1034 baseline は capture endpoint を開かず render endpoint のみを駆動したが、mid-epoch regression を再現した。
2. **consumer harness の Pass/Fail 転用や stereo format contract の副作用ではない。** baseline の oracle は render callback timestamp だけであり、PCM round-trip や stereo channel width に依存しない。RunMetadata は rf629 round-trip Pass と独立に rf1034 case Fail を記録した。
3. **harness が timestamp を合成した結果ではない。** instrumentation は `OutputTimestamp.callback` / `playback` の backend 報告値を atomics に記録し、frame position や代替 timestamp を合成していない。index 250 の `callback` 逆行と `playback` 前進は同じイベント log 内で観測された。
4. **adapter 単独の timestamp 合成という仮説は弱い。** adapter は cpal `StreamInstant` を `MediaTime` に変換して渡し、cpal output path の frame position は公開しない。Windows では overflow fallback が発動する大きな epoch ではなく、イベントは連続した QPC 相当の nanosecond 値を示す。adapter が順序を入れ替える機構は見つかっていない。
5. **現時点で最も支持される原因層は OS audio engine / WASAPI/audiodg の position↔QPC correlation bookkeeping を含む render backend chain である。** ただし本 slice は driver/adapter/cpal を変更しておらず、driver、cpal WASAPI implementation、audiodg のどこで逆行が最初に発生したかを完全には分離できていない。過去の driver position monotonic化で regression が消えなかったという PROVENANCE と、今回の playback 前進/callback 逆行パターンは OS epoch bookkeeping 説を強めるが、確定診断ではない。

したがって、本 slice の cause-layer verdict は「consumer harness/capture loopback 起因ではなく、render backend chain で再現。最有力は OS/WASAPI/audiodg epoch bookkeeping。driver・cpal・adapter の最終犯人は後続 slice」である。Issue #1034 を close する根拠にはしない。

## 採取 evidence

`evidence/rf1034/provisioned/run45851204401-attempt1/` に、artifact 全体ではなく判定に必要なファイルを抽出した。

| file | 内容 |
| --- | --- |
| `artifact-inventory.json` | 抽出ファイルの byte size と SHA-256、run/artifact/SHA provenance |
| `vaudio-probe-windows-2025.json` | stage 状態。provisioning、cleanup、`os-endpoint-wiring error` を含む |
| `endpoint-probe-windows-2025.txt` | endpoint 実名、format、pair candidate、OS version |
| `stage-os-endpoint-wiring-windows-2025.txt` | rf1034 baseline 331 events、regression stats、rf629 metadata/verdict を含む抽出ログ（repository hygiene のため LF/行末空白正規化） |
| `run-metadata-rf1034-audio-virtual-endpoint-render-timestamp-baseline.json` | rf1034 case `fail` の Case Schema v1 record |
| `run-metadata-rf629-audio-virtual-endpoint-render-capture.json` | 同一 stage の audio round-trip `pass` record |
| `run-metadata-rf629-midi-virtual-endpoint-loopback.json` | 同一 stage の MIDI loopback `pass` record |

## 受入条件対応（本 slice 範囲）

| 受入条件 | 判定 | 対応 evidence |
| --- | --- | --- |
| render callback timestamp の単調性、frame position、capture timestamp との関係、責務分担を spec で確定 | Pass | `docs/specs/issue-1034-render-timestamp-contract.md` |
| provisioned Windows virtual endpoint で baseline render stream の regression を再現する最小 case、または再現しないことを示す検査 | Pass（再現） | run `45851204401`、rf1034 case `fail`、index `[3,4,250]`、effective regression `1` |
| 原因層が driver、adapter、consumer harness、環境のいずれにあったかを evidence 付きで記録 | Pass（切り分け途中を明示） | consumer/capture 起因否定、render backend chain で再現、OS/WASAPI/audiodg bookkeeping が最有力。driver/cpal/adapter の最終確定は未解決 |
| timestamp 単調性が必須なら修正後 provisioned 実走で regression count 0、必須でないなら正式契約と oracle 更新 | 未達/後続 | 単調性は同一 epoch 内で必須と確定。修正は本 slice 対象外であり regression count 0 は未達 |
| Synvelis consumer round-trip の Pass/Fail を upstream evidence に転用しない | Pass | 本 evidence は Synvelis を使っていない。rf629/Ruviform 内部 round-trip と rf1034 baseline を区別して記録 |
| `mise run` の対象 lane が pass | Pass（ローカル検証範囲） | 上記 canonical table。local manual lane は未 provisioned NotRun であり runtime Pass と読まない |

## 残件

- driver、cpal WASAPI timestamp 生成、audiodg epoch bookkeeping のどこで最初に逆行するかを分離する後続 slice。必要なら OS position/QPC correlation の追加観測を adapter 契約内で設計する
- regression count 0 の修正後 provisioned 実走。本 slice の baseline case はその再測定に使える
- `docs/**/README.md` の docs index は本 slice で編集・再生成していない。Coordinator が統合時に整備する
- rf#1009 xrun cases と fault cases は同じ workflow artifact に含まれるが、本 slice の対象外であり、それらの Pass/Fail を rf1034 evidence に転用していない
