# RF-630 spawn-timeout signature sweep — 検証記録

- Issue: `scarecrow173/ruviform#630`（reliability slice 続編 —
  `ruviform-plugin-host/tests/`・`ruviform-plugin-export/tests/` の
  `SpawnFailed{Timeout}` signature-miss 穴 sweep）
- Slice: 前 slice（`spawn-timeout-verification.md`）が
  `ruviform-plugin-test-vst3`/`test-clap` の
  `l1_fault_test_plugin_e2e.rs` `instantiate()` signature に
  `Live` phase を追加したのに対し、本 slice は残った lease 範囲の
  全 spawn surface を sweep し、un-retried `Instantiate`/spawn/
  mint 経路へ同一の bounded-retry pattern を適用する
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints 正本）
- 記録日: 2026-11-24
- deterministic evidence（induce 補助証拠）と real-run evidence を
  分離。canonical lane = `mise run test-l1-real-plugin`・workspace
  `mise run test` 系。cargo 直接実行は補助証拠として明記する

## sweep 結果 — signature-miss 穴

対象: `crates/ruviform-plugin-host/tests/*.rs` 全 11 file と
`crates/ruviform-plugin-export/tests/*.rs` 全 2 file を grep +
目視で全件判定。基準は「success 経路で `SpawnFailed{Timeout}` を
返しうる spawn/mint surface が retry signature を持たず
`.unwrap()`/panic で落とす」穴。期待値経路（`SpawnFailed`・
`InstanceLimit` を契約値として pin する test）は retry 対象外。

### 穴（本 slice で修正）

| file | surface | 内容 |
| --- | --- | --- |
| `l1_proxy_e2e.rs` | `composite_e2e::instantiate` | `HostCommand::Instantiate` `.unwrap()` — L0 fixture + L1 worker/runner route 双方を serve |
| 同 | `L1Instance::spawn` 各 site | dedicated route の spawn+mint 全体（`spawn_timeout` bound） |
| 同 | `L1Child::spawn_scanned` + `create_instance` | bundled route の child spawn と pooled mint — `bundled_pair`・fill loop・mid-sequence site を含む |
| `l1_supervisor_e2e.rs` | `instantiate` | ~30 call site が共有する helper — `HostCommand::Instantiate` `.unwrap()` |
| `l1_video_e2e.rs` | `instantiate`（`VideoHostCommand::Instantiate`） | ~20 site。`CreateVideoInstance` wait も `spawn_timeout` bound |
| 同 | `HostCommand::Instantiate`（audio route、cross-kind test） | `audio_counters_absent_on_the_video_route` の L1 audio contrast mint |
| `plugin_graph_e2e.rs` | `instantiate` | L0/L1 両 route を serve する shared helper（L0 は `SpawnFailed` を返しえないため retry arm は unreachable） |
| `editor_session.rs` | `l1_host` fixture | `InProcessHost` 側の同名 `instantiate` helper は L0 のため対象外 — L1 route 用 `instantiate_l1` を新設 |
| `editor_session_l1.rs` | `gui_host` fixture | real `ruviform-plugin-l1-runner` child への `Instantiate` |
| `gui_request.rs` | `l1_host` fixture | `l1-test-worker` child への `Instantiate` |

### 穴なし（判定記録）

- `editor_session_driver.rs` — spawn-path 呼出なし。
- `l1_scale_probe.rs` — 全 test が `#[ignore]` の手動計測 probe
  lane であり、契約 test の flake 対策（本 slice の基準）の対象外
  と判定。spawn/mint の `.unwrap()` は残る。
- `ruviform-plugin-export/tests/l1_runner_e2e.rs`・
  `l1_video_runner_e2e.rs` — spawn-path 呼出なし（runner bin 側の
  検証で、`Instantiate` 発行者を持たない）。
- expected-value 経路（`expect_spawn_failed` helper、handshake
  mismatch 系、`spawn_failed_attempt_record_captures_stderr` 等の
  phase pin test、`bundled_*_death_*`・`invalid_lookahead_*` 等の
  契約 pin）は retry を載せない — 各 file で維持。

## 修正内容（test 側のみ — production diff なし）

各 file に #933 対称の bounded retry を導入。signature は前 slice
と同一の四重検査: `cause == Timeout` AND retained record が
`DiagnosticQuery::Found` AND `record.phase ∈ {PreSpawn, Spawn,
DrainInit, Handshake, Live}`（enum 現行の全 variant — `Live` は
`Scan`/`Create`/`CreateVideoInstance` wait timeout が freeze する
phase）AND `record.cause == Some(Timeout)`。signature-miss は即
assert（retry しない）、bound `SPAWN_TIMEOUT_RETRIES = 3`、bound
超過は panic で honest fail。全 attempt を real stderr へ trace
（harness の per-test capture を bypass — lane log に残る）。

route 別の retry unit:

- composite route（`host.execute(HostCommand::Instantiate)` /
  `vex(VideoHostCommand::Instantiate)`）: 失敗した child は pool
  eviction で Failed 化し、retry `Instantiate` が新規 spawn を引く
  — `instantiate` helper 内の同一 loop で完結（`l1_proxy_e2e` の
  `composite_e2e`・`l1_supervisor_e2e`・`l1_video_e2e`・
  `plugin_graph_e2e`・`editor_session{,_l1}`・`gui_request`）。
- dedicated pub(crate) route（`L1Instance::spawn`）: spawn+mint
  全体が 1 call — `spawn_instance` が fresh `DiagRegistry` 毎に
  retry し signature 検査する。
- bundled pub(crate) route（`L1Child::spawn_scanned` +
  `create_instance`）: **retry unit が異なる**。
  `request_timed` timeout は `converge_child` で child を
  `Terminal::Failed` にするため、同一 child への `create_instance`
  retry は frozen convergence record の即時 replay でしかなく意味
  を持たない（induce run で実測: 同一 diagnostics_id・同一
  `phase=Live` record が 4 attempt 分 0.1 s で返る）。そこで
  `child_attempt` が spawn+mint sequence 全体を 1 attempt とし、
  mint の `SpawnFailed` を `MintSpawned` marker で closure から
  attempt loop へ伝播 — signature 検査後に `teardown` + fresh
  child で sequence を replay する構造にした。mid-sequence mint
  （`bundled_failed_instance_destroy_recovers_handle` の handle-0
  reuse mint）は test body 全体を attempt closure に入れ replay
  可能とした。

## 検証実行記録

### fmt / check / lint（変更 crate）

- `mise run check ruviform-plugin-host` — exit 0（induce patch
  revert 後の最終 diff で実施。lint・test は下記）

### induce 補助証拠（deterministic — signature 実動作の直接証明）

canonical lane では `Live` phase `Timeout` を自然発火させられない
ため、`l1/mod.rs` に一時 patch（`RVF_INDUCE_CREATE_TIMEOUT_MS`
env で `CreateInstance` reply wait を `Duration::ZERO` に短縮し
`Live` phase `Timeout` record を強制発火）を適用して実行した
**補助証拠（cargo 直接実行、canonical = mise）**。検証後
`git checkout` で完全 revert — 最終 diff に production 変更なし。

- pub(crate) mint surface（`child_attempt` 経由、
  `bundled_create_past_capacity_is_instance_limit`）:
  - `once`（初回 `create_instance` のみ zero-timeout）: trace
    `attempt=0 cause=Timeout diagnostics_id=1 phase=Some(Live)
    record_cause=Some(Timeout)` → spawn+mint sequence を fresh
    child で replay → **test result: ok**。`Live` phase signature
    での retry 実動作を直接確認。
  - `always`（全 `create_instance` zero-timeout）: trace
    attempt=0..3 の 4 件全て `phase=Some(Live)` → bound 超過で
    `create_instance failed: ... persisted through the 3-retry
    bound` panic → **test FAILED**（honest fail — bound が
    verdict を吸収しないことの確認）。
- composite `instantiate`
  （`composite_e2e::duplicate_plugin_id_is_a_defined_rejection`）:
  - `once`: trace `plugin=org.ruviform.test.l1-test-worker
    attempt=0 cause=Timeout phase=Some(Live)` → retry →
    **test result: ok**。composite route の pool eviction + 新規
    spawn retry を確認。
  - `always`: 4 trace → `instantiate failed: ... persisted through
    the 3-retry bound` panic → **test FAILED**（honest fail）。
- `l1_supervisor_e2e` `instantiate`
  （`fail_initialize_attempt_cleans_created`）:
  - `once`: `attempt=0 phase=Some(Live)` → retry →
    **test result: ok**。第 2 file の対称実装を確認。

## 残存リスク・対象外（正直な記録）

- `ruviform-plugin-test-vst3`/`test-clap` の非 fault file
  （`l1_real_test_plugin_e2e`・`l1_stateful_test_plugin_e2e` 等）は
  本 slice の lease 外 — signature-miss 穴は残る（前 slice と同じ
  残件、Coordinator へ報告）。
- `l1_scale_probe.rs` の `#[ignore]`-gated probe は未対応（手動
  lane・契約 test でないため sweep 対象外と判定）。
- `SpawnFailed{Timeout}` の完全排除は証明しない — 観測・induce
  証拠を正直に記録するに留める。
- `Scan`/`Create` wait の timeout を `Live` phase record として残す
  現行 phase 帰属は lease 外 pin で変更不可 — production phase
  意味論の再編は本 slice の境界外（前 slice 記録と同一）。
