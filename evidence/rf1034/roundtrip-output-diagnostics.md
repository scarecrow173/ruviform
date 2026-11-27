# rf1034 round-trip output timestamp diagnostics/attribution evidence

## 対象と実施条件

- 対象 Issue: `scarecrow173/ruviform#1034`
- branch: `test/1034-roundtrip-diagnostics`
- Worker / Dispatch: `task_7268196c070b` / `ctx_2bd059c9d4d6`
- 実施日時: 2026-11-27 UTC
- branch 作成時の main SHA: `823245f57d130436c1f51824f601af13e22006ef`
- 最終検証前に取り込んだ main SHA: `511e1a7944c35797942ba065c092b9263067c48a`
- provisioned 測定対象 SHA: `d2b7397494880c4bbfae14dac0b85278de456408`
- runner: GitHub-hosted `windows-2025` x64
- workflow: `.github/workflows/virtual-audio-driver-probe.yml`、manual `workflow_dispatch`、attempt はすべて `1`

本 slice は Issue #1034 の closeout ではなく、run `45940466986` の rf629 audio render→capture round-trip に残った output regression `idx=[3,330]` 型 event を、tagged diagnostics 付きで再現し、原因層を bounded に分離する diagnostic/instrumentation slice である。PR は `Refs #1034` であり closeout 宣言ではない。

## 前提資料と fresh-QPC fix を採用しなかった理由

Issue #1034、`docs/specs/issue-1034-render-timestamp-contract.md`、既存 rf1034 baseline/layer-isolation/mid-epoch evidence、run `45851204401` / `45865926990` / `45940466986` の抽出 artifact、`os_endpoint.rs` の classifier/instrument、`crates/ruviform-audio-device/src/stream.rs`、cpal 0.18.2 `timestamp.rs` と `host/wasapi/stream.rs` を確認した。

merged contract は adapter が backend 報告 timestamp を合成・平滑化せず渡す責務を持つと定める。fresh `QueryPerformanceCounter` への置換、timestamp synthesis/smoothing/clamping、`max(previous,current)` による単調化、frame position 合成は backend provenanceを捨てるため採用しなかった。本 slice の adapter 差分は deterministic unit test のみで製品挙動の変更はない。

## instrumentation diff の要約

`crates/ruviform-audio-test/tests/os_endpoint.rs` では、round-trip output path に `path=roundtrip_output case=rf629.audio.virtual-endpoint.render-capture pair=<n>` を持つ summary、bounded timestamp event、regression diagnostic、stream lifecycle event、stream error event を追加した。baseline render-only の `render_timestamp_event:` / `render_timestamp_regression_diag:` 行形式は evidence 互換のため変更していない。

- authoritative `TimestampEpochClassifier`、startup grace 24 callback、raw/effective regression 定義、Pass/Fail oracle は変更していない。
- timestamp event log は既存の `RENDER_TIMESTAMP_LOG_SLOTS=512`、lifecycle log は 32 slot、error log は 16 slot の固定長であり、RT callback 内で allocation しない。overflow は summary に記録する。
- regression diagnostic には `index/epoch/epoch_offset/startup_grace/reference/backstep/backend_two_step/host_two_step/supply_two_step/playback lead/playback interval` を残す。
- lifecycle event は start/stop/epoch_split/restart/re_stop、result、host monotonic time、直近 input/output callback index を残す。error event は direction/kind/host time/callback index を残す。
- open/start failure、PCM oracle failure、output timestamp failureのいずれでも、stream drop 後の control thread で round-trip diagnostics を emit する。zero callback なら summary が zero/no event を示し、架空 event は作らない。
- deterministic unit test で tagged round-trip/baseline 識別、epoch reset、PCM oracle failure時の diagnostics 保持、bounded overflow、error/lifecycle correlation を検証した。

`crates/ruviform-audio-device/src/stream.rs` には、backend-reported backward step を含む 3 点列が `StreamInstant`→`MediaTime` conversion で同じ順序・同じ nanos 値を保ち、clamp/smoothing/synthesis されないことを示す deterministic witness を追加した。cpal source inspection では、WASAPI `clock_position` が `IAudioClock::GetPosition` の `qpc_position`（100 ns units）を `*100` して `StreamInstant` を作り、output `callback` はこの値そのものであることを確認した。`playback` は buffered duration と latency を加算する。QPC input が単調なら `qpc_position * 100` は単調写像だが、cpal への入力 QPC そのものは直接観測していない。

## provisioned run serialization

各 dispatch 前に Orca `worker-list --terminal-state active --include-remote` が空、全 branch で同 workflow の `in_progress` / `queued` が空、local/remote branch head が `d2b7397494880c4bbfae14dac0b85278de456408` で一致することを確認した。同時実行は 1 本だけとし、foreground `gh run watch <run-id> --exit-status`、artifact download、抽出を終えてから次を dispatch した。run 2 後の抽出 helper で local path 誤りによる process error があったが、workflow/artifact 自体は完走しており、追加 dispatch なしに同じ artifact を再抽出した。これは dispatch attempt 数に影響しない。

| run / attempt | dispatch UTC | artifact id | watch exit | rf1034 | rf629 audio | in/out cb | frames | raw/effective | regression idx | boundary | max backstep ns | playback some/none/reg | overlap max/events | log ovf | in/out err |
| --- | --- | ---: | ---: | --- | --- | --- | ---: | --- | --- | ---: | ---: | --- | --- | ---: | --- |
| [`45949180113`](https://github.com/scarecrow173/ruviform/actions/runs/45949180113) / 1 | 2026-11-27T06:00:52Z | 15176337917 | 0 | pass | pass | 328/332 | 158976 | 4/0 | `[3,4,313,321]` | 302 | -9895700 | 332/0/0 | 1/0 | 0 | 2/0 |
| [`45950037377`](https://github.com/scarecrow173/ruviform/actions/runs/45950037377) / 1 | 2026-11-27T06:09:16Z | 15176910684 | 0 | pass | pass | 329/331 | 158496 | 2/0 | `[3,4]` | 302 | -10042900 | 331/0/0 | 1/0 | 0 | 0/0 |
| [`45950872106`](https://github.com/scarecrow173/ruviform/actions/runs/45950872106) / 1 | 2026-11-27T06:17:07Z | 15177450519 | 0 | pass | pass | 329/333 | 159456 | 2/0 | `[3,4]` | 302 | -10765300 | 333/0/0 | 1/0 | 0 | 0/0 |
| [`45951647390`](https://github.com/scarecrow173/ruviform/actions/runs/45951647390) / 1 | 2026-11-27T06:24:21Z | 15177362598 | 0 | pass | pass | 330/333 | 159456 | 2/0 | `[3,4]` | 303 | -10862100 | 333/0/0 | 1/0 | 0 | 0/0 |
| [`45952355037`](https://github.com/scarecrow173/ruviform/actions/runs/45952355037) / 1 | 2026-11-27T06:30:57Z | 15177014850 | 0 | pass | fail | 328/331 | 158496 | 3/2 | `[3,145,201]` | 302 | -9947700 | 331/0/0 | 1/0 | 0 | 2/0 |

5 run とも endpoint provisioning と cleanup は `ok`、workflow conclusion は `success` だったが、case-level 判定は conclusion ではなく RunMetadata と tagged diagnostics で行った。artifact は run id を指定して download し、必要な 6 file を `evidence/rf1034/provisioned/run<run-id>-attempt1/` に抽出した。`artifact-inventory.json` に byte size/SHA-256/artifact id/head SHA/download UTC を記録し、`roundtrip-analysis.json` に抽出後の判定用 summary を残した。run 5 で complete reproduction が得られたため診断フェーズはここで終了した。

## reproduction run 45952355037

rf629 metadata は `result=fail`、`observed=no pair candidate completed the round-trip: pair[0]: output timestamp regressions: 2` だった。summary は次のとおりである。

```text
roundtrip_output_timestamp_summary: path=roundtrip_output case=rf629.audio.virtual-endpoint.render-capture pair=0 input_callbacks=328 output_callbacks=331 output_frames=158496 raw_regressions=3 effective_regressions=2 epoch_resets=1 boundary_cb=302 regression_first_cb=3 regression_last_cb=201 regression_max_backstep_ns=-9947700 regression_idx=[3,145,201] playback_some=331 playback_none=0 playback_regressions=0 max_active_callbacks=1 overlap_events=0 timestamp_events=331 timestamp_log_overflow=0 input_errors=2 input_xruns=1 input_discontinuities=1 input_invalidated=0 input_backend_errors=0 output_errors=0 output_xruns=0 output_invalidated=0 output_backend_errors=0 error_events=2 error_log_overflow=0 lifecycle_events=10 lifecycle_log_overflow=0
```

effective regression は epoch 0 の index `145`（epoch offset 145）と index `201`（同 201）であり、24 callback grace 外かつ epoch boundary 前の同一 stream epoch 内である。event log overflow、regression index overflow、playback absence、callback overlap はなく、331 個の tagged output event、3 個の diagnostic、10 個の lifecycle event、2 個の input error eventが読める。したがって spec の complete round-trip reproduction を満たす。

```text
index=145 epoch_offset=145 startup_grace=0 backstep_ns=-1460700 backend_two_step_ns=8600500 host_two_step_ns=18536100 supply_two_step_frames=960 playback_lead_before_ns=30937500 playback_lead_ns=42958334 playback_lead_after_ns=42958334 playback_interval_ns=10560134 next_playback_interval_ns=10061200
index=201 epoch_offset=201 startup_grace=0 backstep_ns=-1481500 backend_two_step_ns=18813000 host_two_step_ns=28766400 supply_two_step_frames=960 playback_lead_before_ns=30958334 playback_lead_ns=42958334 playback_lead_after_ns=31916667 playback_interval_ns=10518500 next_playback_interval_ns=9252833
```

index `143..147` では callback が `860492306100 → 860502306100 → 860500845400 → 860510906600 → 860520956400` で、index `145` だけ逆行し `146` で追いつく backward-then-catch-up 形だった。host monotonic interval は正（`11054400 → 66100 → 8474100 → 10062000 → 10046700` ns）で、supply cursor は 480 frame ずつ進み、playback presence は継続した。index `199..203` でも同じ形で、index `201` が `-1481500` ns 逆行し、index `202` が `+20294500` ns で追いついた。

playback lead は index `144` の `30937500` ns から index `145` で `42958334` ns へ跳び、index `201` でも `42958334` ns へ跳んだ後、index `202` で `31916667` ns へ戻った。これは backend の position↔QPC correlation または buffering bookkeeping の re-anchor を示唆する。lifecycle stop/restart は host time `3036` ms 以降、input xrun/discontinuity は `3039` ms で、regression は host time `1456` ms と `2016` ms だった。したがって今回の effective regression は restart bookkeeping の延長ではなく、stop/restart 前の independent mid-stream event である。output error は 0 だった。

## layer attribution

| layer | 判定 | 根拠 |
| --- | --- | --- |
| backend timestamp | **strengthened** | host monotonic interval は正、supply cursor と frame 供給は連続、overlap なし、playback presence 継続なのに `callback` だけが backward-then-catch-up で逆行した。playback lead の再 anchor 形は WASAPI/audiodg/driver correlation bookkeeping を示唆する。driver/WASAPI/audiodg の内部は分離できないため backend timestamp 層どまり。 |
| cpal conversion | **weakened / unresolved** | cpal 0.18.2 は `GetPosition` の `qpc_position` を `*100` して callback instant を作るだけで、単調 QPC input を conversion 単独で逆行させる経路は見えない。playback 計算には saturating subtraction があるが callback には使われない。ただし cpal へ入力された生 QPC/position は直接観測していないため eliminated とはしない。 |
| adapter passthrough | **weakened / unresolved** | deterministic witness で `StreamInstant`→`MediaTime` が通常域 pass-through で、backend backward step を clamp/smoothing/synthesis せず保持することを確認した。runtime の生 backend 値までは遡れないので eliminated ではない。 |
| render scheduling | **unresolved / weakly strengthened as trigger only** | index `144`/`200` の直前 host interval は `66100`/`54300` ns と callback 到着が集中し、scheduling burst が backend re-anchor を誘発した可能性は残る。ただし regression 自体の host interval は正で、max active callbacks は 1、overlap は 0。host clock 逆行や callback 重畳が直接原因という説は弱い。 |
| test oracle | **weakened** | supply cursor は 480 frame ずつ進み、frame/channel mismatch、overlap、log overflow、epoch misclassification、diagnostic 欠損はなかった。oracle は artifact どおり effective regression を正しく検出しており、harness が系列を生成した説は弱い。backend 原因と oracle の正しさは両立する。 |

final attribution は「backend timestamp 層が strengthened、cpal conversion/adapter passthrough は weakened だが直接観測ではない、render scheduling は trigger possibility として unresolved、test oracle は weakened」である。driver/WASAPI/audiodg のどの内部かは本観測面では分離できない。

## 製品修正判断

製品修正は行わなかった。complete reproduction は得られたが、確定した最も強い層は backend timestamp であり、lease 内の adapter conversion/passthrough または test oracle に単一 root cause とは確定しなかった。cpal patch、driver redesign、workflow 変更、timestamp synthesis/smoothing/clamping/fresh-QPC 置換は禁止条件にも触れる。したがって evidence-only とし、post-fix validation run は実施していない。

## bounded non-reproduction run の意味

run 1〜4 は rf629 case pass で effective regression 0 だったが、これは修正証拠ではない。run 1 では epoch 1 の index `313/321`（offset 11/19）に小さな backward step があったが startup grace 内であり、effective ではない。低頻度かつ timing/environment dependent な事象を 4 回の非再現で否定はできない。run 5 の reproduction により、run `45940466986` の diagnostics gap は再現可能な実在の観測面として閉じた。

## local canonical verification

canonical は mise タスクであり、直接 cargo 実行は補助証拠にすぎない。最終検証前に main `84abc76f4674ba587482197d314005054d7bda6f` を取り込んだ。OS は Windows local である。

| command | exit | 判定 |
| --- | ---: | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-audio-test` | 0 | Pass |
| `mise run lint ruviform-audio-test` | 0 | Pass（clippy `-D warnings`） |
| `mise run test ruviform-audio-test` | 0 | Pass（os_endpoint unit 15 pass、manual 3 ignored を含む） |
| `mise run check ruviform-audio-device` | 0 | Pass |
| `mise run lint ruviform-audio-device` | 0 | Pass |
| `mise run test ruviform-audio-device` | 0 | Pass |
| `mise run test-os-endpoint-wiring` | 0 | NotRun 相当。process exit 0、3 test ok、0.09s 終了だが local runtime evidence とはみなさない |
| `mise exec -- prek run --all-files` | 0 | Pass（evidence 追加後の最終 hygiene。inventory hash も mismatch 0） |

## 次 slice への引き継ぎ

1. backend timestamp 層の内部を分離するには、driver/WASAPI/audiodg の position↔QPC correlation、clock discontinuity、stream re-anchor を外部 OS/driver 観測で直接記録する必要がある。consumer-visible timestamp だけでは backend 内訳は unresolved のままである。
2. cpal input QPC/position と adapter 出力を同一 event で比較するには cpal upstream patch または構造変更が必要であり、本 slice では行わなかった。必要なら別 lease/別 Issue で検討する。
3. regression は backward-then-catch-up かつ playback lead re-anchor を伴う。render scheduling burst が trigger かを分けるには、runner timing や callback 到着集中の外部観測が必要である。
4. 本 slice は製品修正をしていないため regression-count-zero の修正検証ではない。Synvelis consumer round-trip の Pass/Fail は upstream evidence に転用していない。

`actions.jsonl` には branch/verification、serialization precheck、dispatch/watch/download、case analysis、evidence 作成、最終 hygiene を同じ内容で追記する。
