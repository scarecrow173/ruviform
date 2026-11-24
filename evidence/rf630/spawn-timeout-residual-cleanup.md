# RF-630 spawn-timeout residual cleanup — 検証記録

- Issue: `scarecrow173/ruviform#630`（reliability slice 後処理 —
  #933→#935→#936→#937 の signature-miss sweep 4 連で閉塞した穴の
  残件一括処理: `SpawnPhase::Handshake` doc 乖離の doc 側整合 +
  `l1_proxy_e2e` timeout-convergence 別 class flake の診断・小修正）
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints
  正本）
- 記録日: 2026-11-24
- ブランチ: `fix/630-residual-cleanup`（`origin/main` `752a6003`
  — #937/#939 統合済み — から分岐）
- canonical lane = `mise run test ruviform-plugin-host`（全 test
  binary 並列実行。cargo 直接実行は補助証拠として明記）。
  induce 補助証拠と real-run evidence を分離する

## 1. Handshake 語彙の照合 — doc 乖離の洗い出し

照合の正本となる語彙の区別（worktree HEAD の実装で再確認）:

- **`SpawnPhase`**（phase — ライフサイクル位置、`l1/diag.rs`）:
  `L1Child::spawn` は `Hello` 成功直後に `set_phase(Live)`
  （`l1/mod.rs:2223`）するため、`Scan`/`CreateInstance`/
  `CreateVideoInstance` の reply wait は `Live` phase で実行
  され、そこでの `spawn_timeout` 発火は `Live` record を残す。
- **`FailureCause`**（cause — 定義済み拒否の分類、
  `ruviform-plugin-core/src/boundary.rs:623`）: `Hello` version
  不一致・decode 可能だが `Hello` でない先頭 frame・
  `Scan`/descriptor mismatch・`CreateInstance` の Err reply を
  含み phase を問わない（例: `Scan` mismatch は `Live` phase
  record に `Handshake` cause を記録 — `l1/mod.rs:2261`）。

### lease 内の照合結果

| 対象 | 判定 |
| --- | --- |
| `crates/ruviform-plugin-host/src/l1/diag.rs` `SpawnPhase::Handshake` doc comment | **乖離あり — 修正（comment のみ）**。旧記述は "`Hello`/version, `Scan` and `CreateInstance`/`CreateVideoInstance` negotiation — the mint window" と `Scan`/`CreateInstance` を `Handshake` phase に含めていたが、実装は `Hello` 成功直後に `Live` へ進める（`l1/mod.rs:2223`）。`Hello`/version 交換のみを `Handshake`、post-`Hello` の `Scan`/`CreateInstance` wait は `Live` で走り `Live` record を freeze する、という現行 phase 意味論に doc comment を整合させた。コード・enum 構造・他 doc comment は不変 |
| `docs/designs/issue-625-ci-e2e-handshake-race-design.md` | **乖離なし**。`ruviform-ui-test` の `TestDriver::handshake`（`test.hello`/`Welcome` 交換・`DriverError::Handshake` 分類正規化）の設計であり、L1 spawn の `Hello`/`SpawnPhase` とは別領域。通読して phase/cause 混同・陳腐化記述は確認されず |
| `docs/designs/issue-630-preset-spawn-fault.md` | **乖離なし**。`Handshake`/`Timeout` 言及（`PRE_HELLO_DEATH` 受理集合・厳密 pin、`:82`、`:121`）は `FailureCause`（cause）の pin であり `SpawnPhase` の主張ではない。現行の cause 分類と整合 |

### lease 外の残存乖離（列挙のみ — lease 外のため未修正）

| 対象 | 乖離内容 |
| --- | --- |
| `docs/discovery/issue-841-spawn-deadline-flake.md` wait×`SpawnPhase` 表（`:95-101`） | `Scan`/`CreateInstance` reply wait の freeze phase を `Handshake` と記述 — post-#933 実装では `Live`。同表の行参照（`l1/mod.rs:2148`・`:2594`）も #846 前の `command_timeout` 共有時代の番号で陳腐化（現行: `hello_rx` は `:2194` 付近、`request_timed` は `:2240` 付近）。`:122` C1 帰属の「`PreSpawn`/`Spawn`/`DrainInit`/`Handshake` は pre-mint deadline 系」も post-#933 では `Live` phase record が pre-mint `spawn_timeout` 失敗を含みうる点で不完全。Residual 節（`:222-228`）の coverage boundary 記述自体は正確（#849 signature が `Live` を除く保守境界だった事実の記録） |
| `docs/test-specs/issue-630-test-fault.md` `:84-89` | #849 時代の signature 記述 — retry 対象 phase 集合 `{PreSpawn, Spawn, DrainInit, Handshake}` が `Live` を含まない（post-#933 の実装は `Live` を含む）。#849 実装の歴史的記述としては正しいが、現行 signature との差分は残存する |
| `docs/designs/issue-121-l1-diagnostics.md` `Handshake` 表（`:94`・`:107-117`） | **乖離なしを確認** — `FailureCause`（cause）の分類表であり、`boundary.rs` の現行 doc と一致（`Scan`/descriptor mismatch・`CreateInstance` Err reply が `Handshake` cause — phase 不問）。lease 外のため変更も不要 |

## 2. `l1_proxy_e2e` 別 class flake — 診断

rf#631 verification（`evidence/rf631/verification.md:1736-1748`）が
記録した flake: 全 test binary 並列の canonical lane で run ごと
2〜3 件が断続失敗、`l1_proxy_e2e` 単独では 187/187 pass — 並列
spawn 環境負荷由来。対象 4 test の失敗腕を post-#935（本 file に
`SPAWN_TIMEOUT_RETRIES = 3`・`spawn_timeout_signature`・
`check_spawn_failed`・`spawn_instance`・`child_attempt`・
composite `instantiate` retry が導入済み、worktree HEAD 実測）
の構造で棚卸しした。

| test | 失敗腕の診断 | post-#935 での状態 |
| --- | --- | --- |
| `hang_converges_on_command_timeout` | spawn 側 env stall → `SpawnFailed{Timeout}` は `spawn_instance` が retry。trigger `Initialize`（msg 3）は hang で `Err(Ipc)` が期待値であり、先行 command の環境腕は存在しない | **#935 で閉塞済み — 追加対応不要**（bound 超過のみ正直 fail） |
| `spawn_failed_attempt_record_captures_stderr` | 期待値経路（`expect_spawn_failed`）は retry 非搭載。`spawn_timeout`(500ms) が `Hello` wait 等 `Live` 到達前に発火すると record が `phase < Live` で凍り `record.phase == Live` pin が壊れる。stderr pin は `freeze` が bounded drain の `Done` を待つため堅牢 | **失敗腕あり — pre-`Live` `Timeout` record の env stall 腕（本 slice で修正）** |
| `wedged_child_write_converges_and_teardown_is_bounded` | `--stop-reading-after-msgs=3` は `Scan`/`CreateInstance`/`Initialize` を応答するため、`Initialize` `.unwrap()` は「先行 command が `Err(Ipc)` にならない」仮定に依存。500ms `command_timeout` が環境 stall で先行 command に発火 → `Err(Ipc)`（または収束後 `Err(Failed)`）→ unwrap panic。spawn 側は `spawn_instance` retry 済み | **失敗腕あり — pre-trigger command の `command_timeout` env 腕（本 slice で修正）** |
| `command_timeout_converges_failed_with_timeout_cause` | 同型: `--hang-after-msgs=4` は `Initialize`(msg3)/`Activate`(msg4) を応答するため両 `.unwrap()` が同仮定に依存。先行 command の env 発火は `Err(Ipc)`/`Err(Failed)` で表面化し、続く trigger `StartProcessing` が `Err(Failed)` 側へ落ちて assert 不一致にもなる | **失敗腕あり — 同上（本 slice で修正）** |

## 3. 小修正判定と内容

判定: 診断した 3 件の失敗腕はいずれも「scenario の trigger に
到達する前の環境 deadline/収束」であり、file 内完結の bounded
signature retry（`SPAWN_TIMEOUT_RETRIES = 3`、attempt ごとの
real-stderr trace、signature-miss 即 fail、bound 超過は honest
panic）— 本 file に既にある #933 系 helper と同型 — で閉塞
できる。contract assertion（timeout convergence 意味論・
record pin・bounded teardown）は一切変更せず、timeout 値・
sleep も変更しない（issue-238 方針 — 仮定そのものを消す引き
上げではなく、環境腕を signature で分離する修正）。

| test | 修正 |
| --- | --- |
| `spawn_failed_attempt_record_captures_stderr` | `expect_spawn_failed` を bounded retry loop 化。`cause == Timeout` かつ record `Found`・`record.cause == Some(Timeout)`・`phase ∈ {PreSpawn, Spawn, DrainInit, Handshake}`（`Live` を除く — `Live` freeze がこの scenario の verdict）を env signature として retry。`Live` 受理後の record pin（`handle: None`・`cause: Timeout`・stderr capture 全 pin）は不変 |
| `wedged_child_write_converges_and_teardown_is_bounded` | `Initialize`（pre-trigger）の `Err(Ipc)`/`Err(Failed)` を env signature として bounded retry（fresh child で replay）。trigger 以降の assertion（256KiB `RestoreState` `Err(Ipc)`・`Failed` 収束・notification・bounded teardown）は不変 |
| `command_timeout_converges_failed_with_timeout_cause` | `Initialize`+`Activate`（pre-trigger）の `Err(Ipc)`/`Err(Failed)` を同 signature で bounded retry（`Destroy` 後 fresh instantiate）。trigger 以降の assertion（`StartProcessing` `Err(Ipc)`・`Timeout` cause・handle record・`Destroy` bounded）は不変 |
| `hang_converges_on_command_timeout` | 無修正 — spawn 腕は `spawn_instance` retry 済み、trigger が最初の post-mint command のため先行腕を持たない |

## 4. 検証

### induce 補助証拠（検証後 revert 済み）

file 内一時 patch で各 retry 腕を強制発火させ、signature 認識・
retry 動作・bound 超過の honest panic を実測（canonical =
`mise`、本項は補助 `cargo test`）:

| 対象 | patch | 観測 |
| --- | --- | --- |
| `spawn_failed_attempt_record_captures_stderr` | `spawn_timeout = Duration::ZERO` | attempt 0–3 全て `cause=Timeout phase=Some(Spawn)` を trace → 「persisted through the 3-retry bound (phase=Some(Spawn)) — an honest environment failure, not an absorbed flake」で panic — signature 認識・retry・bound panic 全て所期どおり |
| `command_timeout_converges_failed_with_timeout_cause` | `command_timeout = Duration::ZERO` | attempt 0–3 全て `reply=Err(Ipc)` を trace → bound panic — 同上 |
| `wedged_child_write_converges_and_teardown_is_bounded` | `command_timeout = Duration::ZERO` | attempt 0–3 全て `reply=Err(Ipc)` を trace → bound panic — 同上 |

受理側（環境腕が発火しない経路）は real-run で確認: 全 run で
attempt=0 が `phase=Some(Live)`（`spawn_failed_attempt`）または
初回成功（pre-trigger command）を記録し test pass。

### real-run evidence

- **修正前観測（baseline）**: canonical lane 1 run pass（`l1_proxy_e2e` 190/190）+ 5-run loop では対象 flake 0/5（1 run は `editor_session_l1` の `ruviform-plugin-l1-runner` 不在 fail — runner bin が mbx target から sweep される環境事象で本 class と無関係、`RUVIFORM_L1_RUNNER` 指名で回避。rf#631 が既記録の既知 flake class）。
- **修正後**: canonical lane 6 run 連続全 leg pass（`l1_proxy_e2e` 190/190 ×6 — trace は `spawn_failed_attempt trace: attempt=0 phase=Some(Live)` のみ、retry 未発火）。対象 4 test 単独（`--test-threads 8`、補助証拠）10 連続 iter 全 pass。
- **未再現の正直な記録**: 本環境では修正前後とも対象 flake の自然再現は観測できなかった（rf#631 観測時の全 binary 並列負荷条件が再現せず）。retry 腕の正当性は induce 補助証拠と signature の構造解析で裏付ける。

### canonical lane 結果（本 slice 最終状態）

| コマンド | 結果 |
| --- | --- |
| `mise run fmt` | pass（exit 0 — 適用済み、diff は lease 内のみ） |
| `mise run fmt-check` | pass（exit 0） |
| `mise run check ruviform-plugin-host` | pass（exit 0） |
| `mise run lint ruviform-plugin-host` | pass（exit 0、`-D warnings` ゲート — `unnested or-patterns` を初回検出・修正済み） |
| `mise run test ruviform-plugin-host` | pass（exit 0 — 全 test binary、190/190 を含む全 leg 0 failed） |
| `mise exec -- prek run --all-files` | pass（exit 0 — 全 hook Passed、workflow 未変更で actionlint も pass） |
| `git diff` lease 外 | 変更 file は `src/l1/diag.rs`（comment のみ）・`tests/l1_proxy_e2e.rs`・`docs/discovery/issue-630-spawn-timeout.md`・`evidence/rf630/spawn-timeout-residual-cleanup.md` のみ — lease 内 |
