# RF-630 spawn-timeout adapter sweep — 検証記録

- Issue: `scarecrow173/ruviform#630`（reliability slice 最終残件 —
  `ruviform-plugin-vst3`/`ruviform-plugin-clap` adapter crate の
  `SpawnFailed{Timeout}` signature-miss 穴 sweep — PR #936 の直接残件）
- Slice: 前 3 slice（`spawn-timeout-verification.md` #933・
  `spawn-timeout-signature-sweep.md` #935・
  `spawn-timeout-plugin-test-sweep.md` #936）が plugin-test /
  plugin-host / plugin-export の tests/ を閉塞したのに対し、本
  slice は最後に残った lease 範囲 — adapter crate 両 ABI の
  `tests/` — を全件 sweep し、見つかった un-retried
  `Instantiate` 経路へ #933 と同一の bounded-retry pattern を
  適用する
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints
  正本）
- 記録日: 2026-11-24
- deterministic evidence（induce 補助証拠）と real-run evidence を
  分離。canonical lane = `mise run test-l1-real-plugin`・workspace
  `mise run test` 系。cargo 直接実行は補助証拠として明記する

## sweep 結果 — signature-miss 穴

対象: `crates/ruviform-plugin-vst3/tests/*.rs` と
`crates/ruviform-plugin-clap/tests/*.rs`。worktree HEAD
（`e145a2d7` — PR #936 統合済み）で `git ls-files` を再確認した
実測では両 crate とも `l1_real_adapter_e2e.rs` 1 file のみ — 計
2 file を grep + 目視で全件判定。基準は「success 経路で
`SpawnFailed{Timeout}` を返しうる spawn/mint surface が retry
signature を持たず `.unwrap()`/panic で落とす」穴。期待値経路
（`SpawnFailed` を契約値として pin する test）は retry 対象外。

### 穴（本 slice で修正 — 2 file）

| file | surface | 内容 |
| --- | --- | --- |
| `ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs` | `instantiate` helper（`:147` 付近） | `HostCommand::Instantiate` `.unwrap()` + `other => panic!("unexpected reply")` — `SpawnFailed` を panic へ直転。全 success 経路 test（`real_adapter_l1_full_series`・`real_adapter_l1_kill_reports_failed_once`）が共有する spawn 経路 |
| `ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs` | `instantiate` helper（`:119` 付近） | `SpawnFailed` arm が `diagnostics_query` 印字後 `panic!` — retry なし（un-retried surface。印字は terminal failure 時に温存） |

いずれも composite route — public `PluginHost` API 経由の
`HostCommand::Instantiate` であり、失敗時は pool eviction で
Failed 化した child を捨て retry `Instantiate` が新規 spawn を
引く。adapter crate の test は外部 test target で
`ruviform-plugin-host` の public API 経由のみ — pub(crate)
bundled spawn surface（`L1Child::spawn_scanned` +
`create_instance`）を持たないため、#935 の `child_attempt`
sequence unit は不要（#936 と同じ判断 — bundled route の二度目
`instantiate()` が踏む pooled-child re-mint `create_instance`
wait の `Live` record freeze は helper loop の retry がそのまま
cover する）。

### 穴なし（判定記録 — file 内の残り surface）

両 file 共通（行番号は修正前 HEAD 基準）:

- 期待値経路 — `real_adapter_l1_missing_runner_is_spawn_failed`
  （vst3 `:553`・clap `:531` の直接 `HostCommand::Instantiate`、
  `SpawnFailed{cause: Spawn}` を `assert!(matches!(...))` で
  pin）は retry を載せない — 期待値経路に retry を載せると契約
  を壊すため各 file で維持。
- parent-side `HostCommand::Scan` の `.unwrap()`（vst3
  `:341`/`:516`/`:573` 付近・clap `:321`/`:494`/`:550` 付近）—
  composite の registered descriptor replay であり `SpawnFailed`
  を返しえない。spawn surface ではない（#935/#936 の判定を踏襲）。
- post-mint 経路 — `drive_to_processing`（`Initialize`/
  `Activate`/`StartProcessing`）・`SaveState`/`RestoreState`/
  `OpenGui`/`CloseGui`/`Destroy` の `execute` 結果処理、
  `kill_child`・`wait_host_failed`・`wait_host_notification`・
  `try_recv_notification`・`host_block`（`PluginHost::process`）
  — 全て `command_timeout` bound の契約 verdict であり retry
  対象になりえない。
- `assert_fixture_present`/`runner_program`/`fixture_path` —
  artifact 存在 assert であり spawn surface ではない。

### lease 外の観測（sweep 網羅性の裏付け — 変更なし）

本 slice の残件締めとして `crates/*/tests/*.rs` 全般へ
`HostCommand::Instantiate`/`register_out_of_process`/`SpawnFailed`
を横断 grep した結果、spawn surface を持つのは #933/#935/#936・
本 slice で対応済みの crate のみ:

- `ruviform-plugin-test-{vst3,clap}/tests/` — #933/#936 で対応
  済み（lease 外・正本）。
- `ruviform-plugin-host/tests/`・`ruviform-plugin-export` —
  #935 で対応済み（lease 外）。
- `ruviform-audio-plugin-bridge/tests/`（`e2e.rs`・
  `param_delivery.rs`・`midi_wiring.rs`・`frame_base.rs`）—
  `PluginHost::execute(HostCommand::Instantiate)` を呼ぶが
  `register_factory`（L0 in-process route — composite.rs
  `:313-316`）経由であり `register_out_of_process` を持たない。
  `SpawnFailed` variant は L1 out-of-process route
  （`spawn_failed`/`create_rollback_error`）でのみ生成される
  ため返りえない — **穴なし — spawn-path 呼出なし**。lease 外
  のため観測のみ、無変更。

これにより `crates/*/tests/` 配下で signature 穴の残はゼロと
判定する（詳細は discovery doc 残件節）。

## 修正内容（test 側のみ — production diff なし）

2 file の `instantiate` helper に #933/#936 対称の bounded
retry を導入。signature は前 3 slice と同一の検査:
`cause == Timeout` AND retained record が
`DiagnosticQuery::Found` AND `record.phase ∈ {PreSpawn, Spawn,
DrainInit, Handshake, Live}`（enum 現行の全 variant — `Live`
は `Scan`/`CreateInstance` wait timeout が freeze する phase）
AND `record.cause == Some(Timeout)`。signature-miss は即
assert（retry しない）、bound `SPAWN_TIMEOUT_RETRIES = 3`、
bound 超過は panic で honest fail。全 attempt を real stderr
へ trace（harness の per-test capture を bypass — lane log に
残る）。

- vst3 file — `use std::io::Write` を追加（`writeln!` 用）、
  `ruviform_plugin_host` import に `DiagnosticQuery`・
  `SpawnPhase` を追加（`FailureCause` は `ruviform_plugin_core`
  経由で import 済み）。file 内完結 — 共通 helper 新設・src へ
  の移動・crate 間共用化は行わない（#933 設計どおり局所適用）。
- clap file — `use std::io::Write` と `SpawnPhase` import を
  追加（`DiagnosticQuery`・`FailureCause` import 済み）。既存の
  Issue #207 diagnosis aid（retained record と child stderr の
  全量印字）は terminal failure 時に温存 — attempt ごとの
  compact trace に加え、signature-miss・bound 超過の最終
  panic 直前に record 全量を印字する（中間 retry attempt で
  は印字しない — #936 plugin-test clap 両 file と同じ形）。

## 検証実行記録

### fmt / check / lint / test（変更 crate・canonical lane）

| コマンド | exit |
| --- | --- |
| `mise run fmt` | 0（整形適用 — diff は対象 2 file のみ） |
| `mise run fmt-check` | 0 |
| `mise run check ruviform-plugin-vst3` | 0 |
| `mise run check ruviform-plugin-clap` | 0（`src/adapt.rs` に pre-existing `dead_code` warning 2 件 — 下記） |
| `mise run lint ruviform-plugin-vst3` | 0 |
| `mise run lint ruviform-plugin-clap` | **101 — Fail（main 既存の破損、scope 外）**: `src/adapt.rs:823` `translate_output_event`・`:857` `translate_note_event` の `dead_code` が `-D warnings` で error 化。唯一の非 test caller が `host.rs:919`（`#[cfg(feature = "host")]`-gated — `lib.rs:77-78`）のため default feature では dead 扱い。変更を stash した pristine `e145a2d7` でも同一 fail を再現確認 — 本 slice 起因ではない。lease 外 src のため未修正（`lint-l1-real-plugin` は `host` feature 有効で caller が生きるため exit 0） |
| `mise run test ruviform-plugin-vst3` | 0（unit 44 件 — `l1_real_adapter_e2e` は `#![cfg(feature = "host")]` のため 0 test） |
| `mise run test ruviform-plugin-clap` | 0（unit 54 件 + doctest 3 件 — 同上） |
| `mise run check-l1-real-plugin` | 0（`#![cfg(feature = "host")]`-gated 修正 file を含む `--all-targets` compile） |
| `mise run lint-l1-real-plugin` | 0（同上・`-D warnings` 通過） |
| `mise run test-l1-real-plugin` | 0（全 leg `test result: ok` — adapter `l1_real_adapter_e2e` 3+3、`--lib --bins` 79+2/91+2（adapter）・87+3/116+3（test crate）、`l1_real` 11+11・`l1_fault` 20+21・`l1_stateful` 4+6。`SpawnFailed` retry trace 0 件） |

### induce 補助証拠（deterministic — signature 実動作の直接証明）

canonical lane では `Live` phase `Timeout` を自然発火させられ
ないため、#933/#936 と同じ手順で
`crates/ruviform-plugin-host/src/l1/mod.rs` の audio
`CreateInstance` reply wait（`request_timed(...,
spec.spawn_timeout)` — worktree HEAD `:2417-2426`）に一時
patch を適用: `RVF_INDUCE_CREATE_TIMEOUT_MS` env で reply
wait を `Duration::ZERO` に短縮（`once` = process-wide
`AtomicBool` で先頭 1 call のみ zero、`always` = 全 call
zero）し、`Live` phase `Timeout` record を強制発火。**補助
証拠（cargo 直接実行、canonical = mise）**。`Duration::ZERO`
（reply は request 送信前に到着し得ないため決定的に
timeout）。検証後 `git checkout` で完全 revert — 最終 diff に
production 変更なし（`git diff` で `src/` 差分ゼロを確認）。

- `ruviform-plugin-vst3` `l1_real_adapter_e2e`
  （`real_adapter_l1_full_series`）:
  - `once`（`RVF_INDUCE_CREATE_TIMEOUT_MS=once cargo test -p
    ruviform-plugin-vst3 --features host --test
    l1_real_adapter_e2e -- --ignored --exact
    real_adapter_l1_full_series`）: trace
    `plugin=3d2c1b5a… attempt=0 cause=Timeout diagnostics_id=1
    phase=Some(Live) record_cause=Some(Timeout)` → retry →
    **test result: ok**。`Live` phase signature での retry
    実動作を直接確認。
  - `always`: trace attempt=0..3 の 4 件全て `phase=Some(Live)`
    → bound 超過で `instantiate failed: the pre-mint
    SpawnFailed{cause:Timeout} signature persisted through the
    3-retry bound` panic（`l1_real_adapter_e2e.rs:227`）→
    **test FAILED**（exit 101 — honest fail。bound が verdict
    を吸収しないことの確認）。
- `ruviform-plugin-clap` `l1_real_adapter_e2e`
  （`real_adapter_l1_full_series`）:
  - `once`: `plugin=org.ruviform.clap-gain-poc attempt=0
    phase=Some(Live)` → retry → **test result: ok**。
  - `always`: 4 trace + terminal failure での diagnostics
    record・child stderr 全量印字（Issue #207 aid 温存の実測
    確認 — `DiagnosticRecord { diagnostics_id: 4, phase: Live,
    cause: Some(Timeout), ... }` 印字後 panic）→ bound 超過
    panic → **test FAILED**（exit 101）。

## 残存リスク・対象外（正直な記録）

- `SpawnFailed{Timeout}` の完全排除は証明しない — induce 補助
  証拠と canonical lane 実走（`SpawnFailed` trace 0 件）を正直
  に記録するに留める。
- `l1_scale_probe.rs` の `#[ignore]`-gated 手動計測 probe は
  未対応（plugin-host tests — #935/#936 と同じく契約 test の
  flake 対策の範囲外・lease 外）。
- `l1_proxy_e2e` の timeout-convergence flake（rf#631 記録）は
  別 class・lease 外 — 未対応。
- `Scan`/`CreateInstance` wait の timeout を `Live` phase record
  として残す現行 phase 帰属は lease 外 pin（`l1_proxy_e2e`）で
  変更不可 — production phase 意味論の再編は本 slice の境界外
  （前 slice 記録と同一）。
- `SpawnPhase::Handshake` doc と実装の乖離（`Scan`/
  `CreateInstance` を mint window に含める doc vs `Hello` 直後
  に `Live` へ進める実装）は既知の観測面の不一致として記録に
  留める。
- clap editor verb 3（GUI・設計決裁要）は対象外として残件化。
- external legs（Issue #630 umbrella の外部実環境 leg）は対象外。
- `mise run lint ruviform-plugin-clap` の main 既存破損
  （`adapt.rs` dead_code — default feature で `host.rs` caller
  が cfg 外）は本 slice 起因でなく lease 外のため未修正 — 報告
  のみ。
