# rf1034 mid-epoch render timestamp reproduction/attribution evidence

## 対象と実施条件

- 対象 Issue: `scarecrow173/ruviform#1034`
- branch: `test/1034-mid-epoch-repro`
- Worker / Dispatch: `task_cbb1b2a27a76` / `ctx_bfdf06e84817`
- 実施日時: 2026-11-27 UTC
- branch 作成時の main SHA: `ffd8d73f45d3da99673a696f8567f7a14e4db5d5`
- 最終検証前に取り込んだ main SHA: `f6f77fc05799d4af960c35d6d82eccd758e5c370`
- provisioned 測定対象 SHA: `ffd8d73f45d3da99673a696f8567f7a14e4db5d5`
- runner: GitHub-hosted `windows-2025` x64、endpoint probe 上の OS version は 5 run とも `Microsoft Windows [Version 10.0.26100.33828]`
- workflow: `.github/workflows/virtual-audio-driver-probe.yml`、manual `workflow_dispatch`、attempt はすべて `1`

本 slice は Issue #1034 の closeout ではなく、baseline run `45851204401` で観測された index `250` 型の mid-epoch regression を、最大 5 本の逐次 provisioned run で再現できるかを確認する bounded reproduction/attribution slice である。実装コード、adapter、driver、cpal、workflow、oracle は変更していない。最終検証前に main `f6f77fc05799d4af960c35d6d82eccd758e5c370` を fast-forward で取り込んだが、その差分は rf#1009 の xrun 観測面であり、`crates/ruviform-audio-test/tests/os_endpoint.rs` と rf1034 timestamp harness には差分がない。

## 前提資料と修正をしなかった理由

作業前に Issue #1034、`docs/specs/issue-1034-render-timestamp-contract.md`、`evidence/rf1034/render-timestamp-contract-baseline.md`、`evidence/rf1034/render-timestamp-layer-isolation.md`、run `45851204401` / `45865926990` の抽出 artifact、および `crates/ruviform-audio-test/tests/os_endpoint.rs` の timestamp classifier と bounded event log を確認した。

merged contract は adapter が backend 報告 timestamp を合成・平滑化せずに渡す責務を持つと定める。このため、fresh `QueryPerformanceCounter` へ output callback timestamp を置き換える speculative adapter-fix は採用しなかった。その修正は原因層の帰属ではなく、backend 報告値の provenance を捨てる contract 違反になるからである。timestamp synthesis、smoothing、clamping、`max(previous,current)` による単調化も実施していない。

既存 instrumentation は、mid-epoch reproduction が起きた場合に必要な `host_two_step_ns`、`backend_two_step_ns`、供給 frame cursor、playback lead/interval、callback overlap、epoch offset、bounded log overflow をすでに記録する。したがって本 slice では `os_endpoint.rs` に新しい instrumentation を追加しなかった。非再現そのものを instrumentation gap とみなさない。

## provisioned run の serialization

各 dispatch 前に、rf#1009 の follow-up dispatch `ctx_2410bb76b701` が `succeeded` / `settled` / `released` であること、全 branch で `virtual-audio-driver-probe.yml` の `in_progress` と `queued` が空であること、local/remote branch head が `ffd8d73f45d3da99673a696f8567f7a14e4db5d5` で一致していることを確認した。5 本とも同時実行なしに逐次 dispatch し、foreground `gh run watch <run-id> --exit-status` で terminal まで待ってから artifact を抽出し、次の run を dispatch した。

`run` 列は GitHub Actions run へのリンクである。`playback` は `some/none/regressions`、`overlap` は `max_active_callbacks/overlap_events` を表す。

| run / attempt | dispatch UTC | SHA | artifact id | watch exit | rf1034 case | callbacks/frames | raw/effective | regression idx | boundary cb | max backstep ns | playback | overlap | log overflow | cleanup |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: | --- | ---: | ---: | --- | --- | ---: | --- |
| [`45936921481`](https://github.com/scarecrow173/ruviform/actions/runs/45936921481) / 1 | 2026-11-27T03:55:14Z | `ffd8d73` | `15171605840` | 0 | pass | 332/158976 | 1/0 | `[3]` | `302` | `-10082000` | `332/0/0` | `1/0` | `0` | ok |
| [`45937774163`](https://github.com/scarecrow173/ruviform/actions/runs/45937774163) / 1 | 2026-11-27T04:04:22Z | `ffd8d73` | `15171338060` | 0 | pass | 332/158976 | 2/0 | `[3,4]` | `302` | `-10713900` | `332/0/0` | `1/0` | `0` | ok |
| [`45938772142`](https://github.com/scarecrow173/ruviform/actions/runs/45938772142) / 1 | 2026-11-27T04:14:24Z | `ffd8d73` | `15172002671` | 0 | pass | 332/158976 | 1/0 | `[3]` | `302` | `-9975100` | `332/0/0` | `1/0` | `0` | ok |
| [`45939796706`](https://github.com/scarecrow173/ruviform/actions/runs/45939796706) / 1 | 2026-11-27T04:24:52Z | `ffd8d73` | `15172278025` | 0 | pass | 333/159456 | 2/0 | `[3,4]` | `302` | `-9753900` | `333/0/0` | `1/0` | `0` | ok |
| [`45940466986`](https://github.com/scarecrow173/ruviform/actions/runs/45940466986) / 1 | 2026-11-27T04:31:41Z | `ffd8d73` | `15173006911` | 0 | pass | 333/159456 | 2/0 | `[3,4]` | `302` | `-10170700` | `333/0/0` | `1/0` | `0` | ok |

workflow の created/completed、job 時間、artifact download UTC は次のとおりである。

| run | created / completed | job time | artifact download UTC | `os-endpoint-wiring` stage |
| --- | --- | ---: | --- | --- |
| `45936921481` | 2026-11-27T03:55:18Z / 04:03:04Z | 7m41s | 2026-11-27T04:03:35Z | `ok`; audio/midi pass |
| `45937774163` | 2026-11-27T04:04:25Z / 04:13:01Z | 8m31s | 2026-11-27T04:13:36Z | `ok`; audio/midi pass |
| `45938772142` | 2026-11-27T04:14:27Z / 04:22:09Z | 7m38s | 2026-11-27T04:23:10Z | `ok`; audio/midi pass |
| `45939796706` | 2026-11-27T04:24:55Z / 04:30:48Z | 5m48s | 2026-11-27T04:31:07Z | `ok`; audio/midi pass |
| `45940466986` | 2026-11-27T04:31:45Z / 04:39:05Z | 7m17s | 2026-11-27T04:39:29Z | `error`; rf1034 pass、rf629 audio fail、rf629 midi pass |

artifact は run id を指定して download し、rf1034 判定に必要な 6 file だけを `evidence/rf1034/provisioned/run<run-id>-attempt1/` に抽出した。各 directory の `artifact-inventory.json` に byte size、SHA-256、artifact id、head SHA、download UTC を記録している。hash は repository hygiene 用に LF/終端 newline を正規化した抽出 file に対する値であり、原本 artifact は GitHub Actions run に残る。binary package、xrun/fault artifact、driver 実体は複製していない。5 run とも endpoint provisioning は成立し、`endpoint-probe` stage は `ok`、cleanup stage は `ok` で、`leftovers=0`、`errors=0` だった。

## rf1034 baseline の判定

本 slice の reproduction 判定は spec どおり rf1034 baseline case に限定する。startup grace 外の同一 epoch 内 effective regression、complete diagnostics、前後 event log の 3 条件がそろった場合だけを mid-epoch reproduction とみなす。

5 run ではすべて rf1034 case が `pass` で、raw regression は index `3` または `[3,4]` に限定された。これらは initial epoch の `startup_grace=1` であり、effective regression はすべて `0` だった。baseline run `45851204401` の index `250` に相当する mid-epoch regression は、5 本中 1 本でも再現しなかった。したがって本 slice の結果は bounded non-reproduction であり、修正済みや問題不存在の evidence ではない。

workflow/job conclusion `success` も修正証拠ではない。run `45940466986` では workflow conclusion が `success` でも `os-endpoint-wiring` stage は `error` であり、case-level metadata を優先して読んだ。

## startup regression と mid-epoch index の観測

startup regression の診断は、過去の layer-isolation evidence と同じ backward-then-catch-up 形だった。代表して run `45940466986` の診断行を示す。

```text
render_timestamp_regression_diag: index=3 epoch=0 epoch_offset=3 startup_grace=1 reference_index=2 reference_callback_ns=850483502100 backstep_ns=-10170700 backend_two_step_ns=-76800 host_two_step_ns=19833800 supply_two_step_frames=576 playback_lead_before_ns=0 playback_lead_ns=22000000 playback_lead_after_ns=31979167 playback_interval_ns=11829300 next_playback_interval_ns=20073067
render_timestamp_regression_diag: index=4 epoch=0 epoch_offset=4 startup_grace=1 reference_index=2 reference_callback_ns=850483502100 backstep_ns=-76800 backend_two_step_ns=20089400 host_two_step_ns=20086900 supply_two_step_frames=960 playback_lead_before_ns=22000000 playback_lead_ns=31979167 playback_lead_after_ns=31979167 playback_interval_ns=20073067 next_playback_interval_ns=9995500
```

index `3/4` では playback lead が `0 → 22 ms → 32 ms` へ再構成され、index `4` では backend two-step interval `20089400 ns` と host two-step interval `20086900 ns` がほぼ一致する。他の 4 run でも同じ startup 形で、playback regression、callback overlap、供給 cursor 逆行、log overflow は観測されなかった。

mid-epoch 領域の代表として、baseline run で regression した index `250` の前後は 5 run とも安定していた。例えば run `45939796706` では index `248..252` の callback interval が `9996700`、`10000300`、`10006600`、`9537600`、`9998500` ns、host interval が `9996500`、`10000200`、`10006700`、`9537700`、`9998300` ns、playback lead は `31979167` ns で、`regression_backstep_ns` はすべて `none` だった。index `250` 型の深い mid-epoch regression は今回の baseline 系列には現れていない。

## 隣接する rf629 round-trip の非対象観測

run `45940466986` では、rf1034 baseline は pass だった一方、同じ `os-endpoint-wiring` stage 内の rf629 audio render→capture round-trip が fail だった。case metadata の観測は次のとおりである。

```text
rf629.audio.virtual-endpoint.render-capture result=fail
observed=no pair candidate completed the round-trip: pair[0]: output timestamp regressions: 1
timestamps: out regressions=2 effective=1 epoch_resets=1 boundary_cb=302 first..last=3..330 max_backstep_ns=-9917200 idx=[3,330]
```

index `330` は boundary callback `302` 以後の epoch `1` 内で、epoch offset は約 28 callback である。startup grace を超えるため rf629 の oracle 上は effective regression だが、この event は rf1034 baseline case の reproduction ではない。また、rf629 round-trip には enhanced `render_timestamp_event` / `render_timestamp_regression_diag` が残らず、host/backend two-step interval、playback lead/interval、供給 cursor、overlap をこの event について照合できない。したがって、この観測を本 slice の mid-epoch reproduction や layer attribution の根拠には転用しない。

ただし、この隣接観測は次 slice に対する具体的情報を持つ。epoch boundary 直後の effective regression が baseline 以外の経路でも出た可能性を示すが、診断値がないため restart bookkeeping の延長なのか独立した mid-stream event なのかを区別できない。非再現そのものではなく、この診断値の欠けた effective regression が、後続で閉じるべき instrumentation gap である。

## hypothesis との照合

| hypothesis | 判定 | 根拠と限界 |
| --- | --- | --- |
| consumer harness | weakened（baseline の誤判定説） | 5 run とも `max_active_callbacks=1`、`overlap_events=0`、playback presence は全 callback、playback regression 0、log overflow 0 で、representative mid-epoch index では供給 cursor が連続していた。baseline series を harness の重畳や cursor 逆行で説明する根拠は弱い。rf629 index `330` には同じ診断値がないため、その event の harness 健全性は unresolved。 |
| runner scheduling/environment | unresolved（timing/environment dependent であることは強い） | baseline run `45851204401` では index `250` が出て、layer-isolation run と今回の 5 run では出なかった。この差は timing/environment dependent な事象であることを示すが、今回は対象 event 自体がないため host/backend interval のずれから scheduling 説を判定できない。 |
| WASAPI/audiodg correlation bookkeeping | startup では strengthened、mid-epoch は unresolved | startup index `3/4` は playback lead `0→22ms→32ms` と backward-then-catch-up により epoch-start re-anchor 説と強く整合する。rf629 index `330` は epoch boundary に近いが、playback/host/cursor 診断がないため mid-stream 更新か restart bookkeeping の延長かは判定できない。 |
| cpal conversion/API use | conversion-only 説は weakened、API 入力条件は unresolved | cpal の QPC→nanos conversion は単調写像であり、入力 QPC が単調なら conversion 単独では逆行しない。`callback` を clamp しない cpal の構造も既知である。今回の baseline では対象 event がないため新たな切り分けはない。 |
| driver interaction | unresolved | driver position bookkeeping 単独では startup regression を説明できない過去 evidence を維持する。今回の baseline では mid-epoch event がなく、driver/OS interaction を直接観測する値もない。犯人とは書けない。 |
| ruviform adapter | weakened / unresolved | adapter は contract どおり backend 報告値を pass-through する前提であり、変換 overflow fallback や順序入替を示す evidence はない。したがって adapter 合成説は弱いが、backend 入力値そのものの provenance をさらに遡る evidence はない。 |

## bounded non-reproduction の統計的限界

5 本の同一 SHA run で mid-epoch reproduction が 0 件だったことは、この workflow・runner・時間帯・測定条件で index `250` 型の regression が deterministic に必ず出るわけではないことを示す。一方で、0/5 は低頻度または条件依存の事象を否定しない。

仮に run ごとの reproduction 確率が独立同一分布だと単純化しても、真の確率が 0.2 なら 5 連続非再現の確率は約 32.8%、0.1 なら約 59.0% である。0.5 でも約 3.1% 残る。実際の GitHub-hosted runner は時間帯、host 配置、OS audio subsystem の状態により独立同一分布とは限らない。したがって、今回の pass 群は「再現率が高くない可能性」を示すにとどまり、run `45865926990` の pass と同様に修正証拠ではない。

## contract への反映判断

contract の規範部分、startup grace 24 callback、epoch classifier、raw/effective regression 定義、adapter pass-through 責務は変更していない。今回の baseline non-reproduction は既存 contract の未確定範囲をそのまま確認する結果であり、規範を変える根拠にならない。

rf629 index `330` の隣接観測は epoch boundary 近くで effective regression が残り得ることを示すが、診断値がない 1 件だけでは startup grace の境界や oracle を見直す根拠にしない。oracle weakening を避けるため、contract spec には今回の run matrix を追記しなかった。確定範囲/未確定範囲の更新が必要なら、次の診断付き reproduction evidence と一緒に行うべきである。

## canonical local verification

最終検証は main `f6f77fc05799d4af960c35d6d82eccd758e5c370` を取り込んだ worktree で実施した。OS は local Windows、canonical command は mise である。直接 cargo 実行は canonical ではない。

| コマンド | exit code | verdict | 観測 |
| --- | ---: | --- | --- |
| `mise run fmt-check` | 0 | Pass | `cargo fmt --all -- --check` pass |
| `mise run check ruviform-audio-test` | 0 | Pass | `cargo check --locked --all-targets -p ruviform-audio-test` pass |
| `mise run lint ruviform-audio-test` | 0 | Pass | clippy `-D warnings` pass |
| `mise run test ruviform-audio-test` | 0 | Pass | deterministic/unit/doc tests pass。`os_endpoint` unit test は 11 pass、manual endpoint 3 test は ignored |
| `mise run test-os-endpoint-wiring` | 0 | NotRun（runtime evidence ではない） | process exit 0、3 test ok、0.09s。local host の provisioned endpoint runtime evidence とはみなさず、rf1034 runtime verdict は provisioned run matrix を使う |
| `mise exec -- prek run --all-files` | 0 | Pass | trailing whitespace/end-of-file/YAML/TOML/mixed line ending/BOM/private key/actionlint hooks pass |

## 採取 evidence

各 run directory に次の 6 file と `artifact-inventory.json` を残した。

- `endpoint-probe-windows-2025.txt`
- `run-metadata-rf1034-audio-virtual-endpoint-render-timestamp-baseline.json`
- `run-metadata-rf629-audio-virtual-endpoint-render-capture.json`
- `run-metadata-rf629-midi-virtual-endpoint-loopback.json`
- `stage-os-endpoint-wiring-windows-2025.txt`
- `vaudio-probe-windows-2025.json`

`actions.jsonl` には、instrumentation gap assessment、canonical local verification、serialization precondition、dispatch、watch、artifact download/extraction、rf1034 case analysis、run `45940466986` の隣接 rf629 observation を追記した。

## 次 slice への引き継ぎ

1. rf1034 baseline の index `250` 型 reproduction は未解決のまま残す。今回の 0/5 を修正や不存在と読み替えない。
2. run `45940466986` の rf629 index `330` 型 event を調べるには、round-trip output timestamp path にも baseline と同じ bounded event / regression diagnostic を残せるかを検討する。少なくとも epoch offset、host/backend two-step interval、playback lead/interval、供給 cursor、callback overlap が必要である。これは oracle を変えない read-only instrumentation であるべきである。
3. epoch boundary 後 24 callback を少し超えた regression が restart bookkeeping の延長なのか独立した mid-stream event なのかを区別する必要がある。その場合も startup grace を広げる結論へ先回りせず、診断付き reproduction を先に取る。
4. driver/OS interaction を確定するには、consumer-visible timestamp だけでなく、`IAudioClock::GetPosition` の device position/QPC correlation、audiodg/WASAPI の再 anchor タイミング、driver position bookkeeping を同時刻に照合する外部観測が必要である。cpal や adapter で timestamp を合成する修正は引き続き contract 違反である。
5. Synvelis consumer round-trip の Pass/Fail は本 Issue の upstream evidence に転用しない。必要なら別 gate の対象 SHA evidence で扱う。
