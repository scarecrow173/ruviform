# RF-630 spawn-timeout plugin-test sweep — 検証記録

- Issue: `scarecrow173/ruviform#630`（reliability slice 続編 —
  `ruviform-plugin-test-vst3`/`test-clap` の non-fault test file
  `SpawnFailed{Timeout}` signature-miss 穴 sweep — PR #935 の直接残件）
- Slice: 前 slice（`spawn-timeout-signature-sweep.md`）が
  `ruviform-plugin-host/tests/`・`ruviform-plugin-export/tests/` の
  un-retried spawn surface を閉塞したのに対し、本 slice は残った
  lease 範囲 — plugin-test crate 両 ABI の non-fault test file —
  を全件 sweep し、見つかった un-retried `Instantiate` 経路へ
  #933 と同一の bounded-retry pattern を適用する
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints 正本）
- 記録日: 2026-11-24
- deterministic evidence（induce 補助証拠）と real-run evidence を
  分離。canonical lane = `mise run test-l1-real-plugin`・workspace
  `mise run test` 系。cargo 直接実行は補助証拠として明記する

## sweep 結果 — signature-miss 穴

対象: `crates/ruviform-plugin-test-vst3/tests/*.rs` 全 5 file と
`crates/ruviform-plugin-test-clap/tests/*.rs` 全 4 file（うち
`l1_fault_test_plugin_e2e.rs` 両 ABI は #933/#935 で対応済みのため
sweep 対象外 — 正本）。残る 7 file を grep + 目視で全件判定。
基準は「success 経路で `SpawnFailed{Timeout}` を返しうる
spawn/mint surface が retry signature を持たず `.unwrap()`/panic
で落とす」穴。期待値経路（`SpawnFailed` を契約値として pin する
test）は retry 対象外。

### 穴（本 slice で修正 — 5 file）

| file | surface | 内容 |
| --- | --- | --- |
| `ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs` | `instantiate` helper | `HostCommand::Instantiate` `.unwrap()` — 全 test が共有する spawn 経路 |
| `ruviform-plugin-test-vst3/tests/l1_stateful_test_plugin_e2e.rs` | `instantiate` helper | 同型 `.unwrap()` |
| `ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs` | `instantiate` helper | 同型 `.unwrap()` — `#[ignore]`-gated 手動 lane（loopback pair 要 provision） |
| `ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs` | `instantiate` helper | `SpawnFailed` arm が `diagnostics_query` 印字後 `panic!` — retry なし（un-retried surface。印字は terminal failure 時に温存） |
| `ruviform-plugin-test-clap/tests/l1_stateful_test_plugin_e2e.rs` | `instantiate` helper | 同型 — `SpawnFailed` arm 印字後 `panic!` |

いずれも composite route — public `PluginHost` API 経由の
`HostCommand::Instantiate` であり、失敗時は pool eviction で
Failed 化した child を捨て retry `Instantiate` が新規 spawn を
引く。`Scan`→`Instantiate` 経路の `HostCommand::Scan`
`.unwrap()`（vst3 `l1_real_test_plugin_e2e.rs`、clap 同型）は
parent-side の descriptor replay であり `SpawnFailed` を返しえない
— spawn surface ではない（file 内 comment・`l1/mod.rs` の
`wire::Command::Scan` が child-scope handshake である実装と整合）。

### 穴なし（判定記録）

- `ruviform-plugin-test-vst3/tests/host_path.rs` — in-process
  `Vst3Library` host 経路（L0）。`PluginHost`/
  `HostCommand::Instantiate`/`OutOfProcessSpec` の呼出なし —
  `SpawnFailed` variant を返しえない。grep で `spawn` 文字列に
  ヒットするのは `test_spawn_fail` component class の descriptor
  名のみ（in-process scan の fixture 名）。
- `ruviform-plugin-test-clap/tests/host_path.rs` — in-process host
  経路（L0）。同様に spawn-path 呼出なし。
- 期待値経路（両 ABI の
  `real_test_plugin_l1_missing_runner_is_spawn_failed` — vst3
  `:646` 付近・clap `:769` 付近の直接 `HostCommand::Instantiate`
  — `SpawnFailed{cause: Spawn}` を `assert!(matches!(...))` で
  pin）は retry を載せない — 各 file で維持。
- post-mint 経路（`Initialize`/`Activate`/`StartProcessing`/
  `Destroy`/`kill_child`/`try_recv_notification` 等の
  `.unwrap()`）は `command_timeout` bound の契約 verdict であり
  retry 対象になりえない — 変更なし。

### 対象外（sweep 対象から除く根拠）

- `l1_fault_test_plugin_e2e.rs`（両 ABI）— #933（`Live` phase を
  signature 許可集合に追加）・#935 統合済みの正本。本 slice の
  手本として読むのみ、無変更。
- `ruviform-plugin-host/tests/`・`ruviform-plugin-export/tests/`
  — #935 で全件 sweep・修正済み。lease 外。
- `ruviform-plugin-vst3/tests/`・`ruviform-plugin-clap/tests/` の
  `l1_real_adapter_e2e.rs` 等 — 同型穴を持ち得るが lease 外。
  修正せず残件として記録（PR 本文「未対応の作業」に列挙）。
- `l1_scale_probe.rs`（plugin-host tests）の `#[ignore]`-gated
  手動計測 probe — #935 と同じく契約 test の flake 対策の範囲外。
- `l1_proxy_e2e` の timeout-convergence flake（rf#631 記録）は
  別 class — 本 signature の対象外として残件化。

## 修正内容（test 側のみ — production diff なし）

5 file の `instantiate` helper に #933 対称の bounded retry を
導入。signature は前 2 slice と同一の検査: `cause == Timeout`
AND retained record が `DiagnosticQuery::Found` AND
`record.phase ∈ {PreSpawn, Spawn, DrainInit, Handshake, Live}`
（enum 現行の全 variant — `Live` は `Scan`/`CreateInstance`
wait timeout が freeze する phase）AND `record.cause ==
Some(Timeout)`。signature-miss は即 assert（retry しない）、
bound `SPAWN_TIMEOUT_RETRIES = 3`、bound 超過は panic で
honest fail。全 attempt を real stderr へ trace（harness の
per-test capture を bypass — lane log に残る）。

route 別の retry unit:

- 全 5 file とも composite route の `instantiate` helper loop —
  `HostCommand::Instantiate` 失敗時、pool eviction で Failed 化
  した child を捨て、retry が新規 spawn を引く（#935 の
  composite route と同一）。plugin-test crate の test は public
  `PluginHost` API 経由のみで pub(crate) bundled spawn surface
  （`L1Child::spawn_scanned` + `create_instance`）を持たないため、
  #935 の `child_attempt` sequence unit は不要 — bundled route
  の二度目の `instantiate()` が pooled-child re-mint の
  `create_instance` wait（`Live` record freeze）を踏む点は、
  helper loop の retry がそのまま cover する。
- clap 両 file は既存の Issue #207 diagnosis aid（retained record
  と child stderr の全量印字）を terminal failure 時に温存 —
  attempt ごとの compact trace に加え、signature-miss・bound
  超過の最終 panic 直前に record 全量を印字する（中間 retry
  attempt では印字しない — trace が attempt 単位の記録を担う）。

## 検証実行記録

### fmt / check / lint / test（変更 crate・canonical lane）

| コマンド | exit |
| --- | --- |
| `mise run fmt` | 0（整形適用 — diff は対象 5 file のみ） |
| `mise run fmt-check` | 0 |
| `mise run check ruviform-plugin-test-vst3` | 0 |
| `mise run lint ruviform-plugin-test-vst3` | 0 |
| `mise run check ruviform-plugin-test-clap` | 0 |
| `mise run lint ruviform-plugin-test-clap` | 0 |
| `mise run test ruviform-plugin-test-vst3` | 0（非 `l1-runner` gated 分 — `host_path.rs` 等） |
| `mise run test ruviform-plugin-test-clap` | 0 |
| `mise run check-l1-real-plugin` | 0（`#![cfg(feature = "l1-runner")]`-gated test file を実際に compile） |
| `mise run lint-l1-real-plugin` | 0 |
| `mise run test-l1-real-plugin` | 0（全 leg `test result: ok` — adapter `l1_real_adapter_e2e` 3+3、`--lib --bins`、両 ABI `l1_real` 11+11・`l1_fault` 20+21・`l1_stateful` 4+6。`SpawnFailed` retry trace 0 件） |
| `mise run test-os-endpoint-midi-e2e` | 0（test binary exit 0 — ただし実測 verdict は `NotRun — NOT_PROVISIONED`: 本 host に loopMIDI/Windows MIDI Services loopback pair 未 provision。endpoint 列挙が `instantiate` より先に NotRun を返す構造のため、本 host では file 内の retry 経路は runtime 未到達 — compile 検証は `check-l1-real-plugin`/`lint-l1-real-plugin` の exit 0 で示し、lane を Pass と見せかけない） |

### induce 補助証拠（deterministic — signature 実動作の直接証明）

canonical lane では `Live` phase `Timeout` を自然発火させられない
ため、`crates/ruviform-plugin-host/src/l1/mod.rs` の audio
`CreateInstance` reply wait（`request_timed(...,
spec.spawn_timeout)` — `:2417` 付近）に一時 patch を適用:
`RVF_INDUCE_CREATE_TIMEOUT_MS` env で reply wait を
`Duration::ZERO` に短縮（`once` = 先頭 1 call だけ zero —
process-wide `AtomicBool` で 1 回発火、`always` = 全 call zero）
し、`Live` phase `Timeout` record を強制発火。**補助証拠（cargo
直接実行、canonical = mise）**。`Duration::ZERO`（reply は
request 送信前に到着し得ないため決定的に timeout）。検証後
`git checkout` で完全 revert — 最終 diff に production 変更なし
（`git diff` で `src/` 差分ゼロを確認）。

- `ruviform-plugin-test-vst3` `l1_real_test_plugin_e2e`
  （`real_test_plugin_l1_full_series`）:
  - `once`: trace `plugin=218a4f6d… attempt=0 cause=Timeout
    diagnostics_id=1 phase=Some(Live) record_cause=Some(Timeout)`
    → retry → **test result: ok**。`Live` phase signature での
    retry 実動作を直接確認。
  - `always`: trace attempt=0..3 の 4 件全て `phase=Some(Live)`
    → bound 超過で `instantiate failed: ... persisted through
    the 3-retry bound` panic → **test FAILED**（honest fail —
    bound が verdict を吸収しないことの確認、exit 101 は test
    failure の期待値）。
- `ruviform-plugin-test-vst3` `l1_stateful_test_plugin_e2e`
  （`real_stateful_test_plugin_l1_save_mutate_restore_save`）:
  - `once`: `attempt=0 phase=Some(Live)` → retry → **test result:
    ok**。
  - `always`: 4 trace → bound 超過 panic → **test FAILED**。
- `ruviform-plugin-test-clap` `l1_real_test_plugin_e2e`
  （`real_test_plugin_l1_full_series`）:
  - `once`: `attempt=0 phase=Some(Live)` → retry → **test result:
    ok**。両 ABI の対称実装を確認。
  - `always`: 4 trace + terminal failure での diagnostics record・
    child stderr 全量印字（Issue #207 aid 温存の実測確認）→
    bound 超過 panic → **test FAILED**。
- `ruviform-plugin-test-clap` `l1_stateful_test_plugin_e2e`
  （`real_stateful_test_plugin_l1_save_mutate_restore_save`）:
  - `once`: `attempt=0 phase=Some(Live)` → retry → **test result:
    ok**。
  - `always`: 4 trace + record 全量印字 → bound 超過 panic →
    **test FAILED**。
- `ruviform-plugin-test-vst3` `os_endpoint_midi_e2e`
  （`os_endpoint_midi_loopback_drives_test_instrument`）: 本 host
  では loopback pair 未 provision のため test が endpoint 列挙で
  `verdict: NotRun` を返し、`instantiate` に runtime 未到達 —
  induce 実測は不可能。file の `instantiate` helper は他 4 file
  と同一構造（同じ composite `HostCommand::Instantiate`
  route・同一 signature 検査）であり、compile 検証は
  `check-l1-real-plugin`/`lint-l1-real-plugin` exit 0 で示す。

## 残存リスク・対象外（正直な記録）

- `SpawnFailed{Timeout}` の完全排除は証明しない — induce 補助
  証拠と canonical lane 実走（`SpawnFailed` trace 0 件）を正直に
  記録するに留める。
- `ruviform-plugin-vst3`/`ruviform-plugin-clap` adapter crate の
  `tests/l1_real_adapter_e2e.rs` 等は同型穴を持ち得るが lease 外
  — 残件として PR 本文に列挙。
- `l1_scale_probe.rs` の `#[ignore]`-gated probe は未対応（手動
  lane・契約 test でないため sweep 対象外 — #935 判定を踏襲）。
- `l1_proxy_e2e` の timeout-convergence flake（rf#631 記録）は別
  class・lease 外 — 未対応。
- `Scan`/`CreateInstance` wait の timeout を `Live` phase record
  として残す現行 phase 帰属は lease 外 pin（`l1_proxy_e2e`）で
  変更不可 — production phase 意味論の再編は本 slice の境界外
  （前 slice 記録と同一）。
- `SpawnPhase::Handshake` doc と実装の乖離（`Scan`/
  `CreateInstance` を mint window に含める doc vs `Hello` 直後に
  `Live` へ進む実装）は既知の観測面の不一致として記録に留める。
- clap editor verb 3（GUI・設計決裁要）は対象外として残件化。
