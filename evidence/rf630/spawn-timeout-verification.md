# RF-630 spawn-timeout reliability slice — 検証記録

- Issue: `scarecrow173/ruviform#630`（reliability slice — `l1_fault` E2E
  `SpawnFailed{Timeout}` env flake の残存調査と根治）
- Slice: `SpawnFailed{cause:Timeout}` の発火経路を実コードで再確認し、
  #846（deadline 分離）・#849（test 側 bounded retry）統合後に残る
  signature-miss 穴を test 側 retry signature の拡張で閉じる
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints 正本）
- 記録日: 2026-11-21
- deterministic evidence（induce 補助証拠）と real-run evidence（canonical
  lane 連続 run）を分離。canonical = `mise run test-l1-real-plugin`、
  cargo 直接実行は補助証拠として明記する

## 原因特定（実コード確認）

### 現行の timeout 構造（#846 統合後 — HEAD 実コード）

`crates/ruviform-plugin-host/src/l1/mod.rs`:

- `SpawnSpec.spawn_timeout`/`command_timeout` は分離済み（`:219-237`、
  default 各 10s）。#846 以前の「spawn Hello が `command_timeout` に
  共有される」構造は解消済み。
- pre-mint の全 wait は `spawn_timeout` 一本で bound:
  spawn hand-off `spawn_rx.recv_timeout(spawn_timeout)`（`:2140`）、
  Hello `hello_rx.recv_timeout(spawn_timeout)`（`:2194`）、
  `Scan`/`CreateInstance` の pre-mint reply wait（`:2236-2245`・
  `:2425` 付近）。
- post-mint `request` → `request_timed` は `command_timeout`（`:2594`
  付近）のまま — 契約上 post-mint であり、本 slice の対象外。

結論: Coordinator 候補だった「spawn-Hello 待ちと `command_timeout`
の同居」は #846 で構造的に解消済み。pre-mint 全経路は
`spawn_timeout` で一様に bound され、`command_timeout` 共有の
発火経路は現構造に残らない。

### signature-miss 穴（本 slice の修正対象）

#849 で導入された `instantiate()` helper（両 ABI）の retry signature は
`cause == Timeout` かつ retained record が `Found` かつ
`phase ∈ {PreSpawn, Spawn, DrainInit, Handshake}` かつ
`record.cause == Some(Timeout)`。一方 `l1/mod.rs` は `Hello` 成功直後に
`set_phase(Live)` を進め、`Scan`/`CreateInstance` の reply wait 実行中の
record は `Live` phase を保持する。`Scan`/`CreateInstance` wait が
`spawn_timeout` で timeout した場合、`SpawnFailed{Timeout}` は
`phase == Some(Live)` の record を伴い、#849 signature では
signature-miss → retry せず即 fail する穴が残っていた。

- `SpawnPhase::Handshake` の doc（`l1/diag.rs`）は "`Scan` and
  `CreateInstance` negotiation — the mint window" を含むと記述するが、
  実装は `Hello` 直後に `Live` へ進む — doc と実装の乖離は既存
  （`l1_proxy_e2e.rs:3331/3948` が create-stage 失敗の
  `phase == Live` を pin しており、lease 外のため phase 帰属の変更は
  不可能かつ不適切）。
- `SpawnFailed` は `spawn_failed`/`create_rollback_error` でのみ生成
  — 定義上 pre-mint。`instantiate()` 内で返る `Timeout` cause は全て
  `spawn_timeout` bound の環境 wait であり、post-mint 契約 verdict を
  含まない。phase 検査自体が過剰に狭い proxy だった。
- `real_test_fault_l1_bundled_crash_fans_out`（vst3 `:804-805`・clap
  同型）のように同一 bundled route で `instantiate()` を二度呼ぶ経路は
  pooled-child の `create_instance` wait を踏み、そこでも timeout record
  は `Live` を freeze する — `set_phase` の位置を変えても消えない
  構造的な穴。

### 記録済み flake との時系列

- `evidence/rf630/fault-modes-verification.md:78,107`・
  `clap-output-drain-verification.md:98-104` の
  `real_test_fault_l1_hang_converges_on_timeout` の
  `SpawnFailed{Timeout}` 各 1 回は 2026-11-21 05:45 commit 時点の
  記録 — **#846/#849 統合前**。`command_timeout` 500ms が spawn
  Hello にも適用されるという記録中の説明は統合で stale 化済み。
- `evidence/rf631/verification.md:1736-1748` の `l1_proxy_e2e`
  timeout-convergence 系断続失敗は lease 外 test file の別 class
  （`runner_lane_*` 未 build flake とも別）— 本 slice では修正しない。
- `evidence/rf849/verification.md` は #849 統合後 canonical 6/6 PASS・
  retry trace 0 件 — 統合後の `SpawnFailed{Timeout}` 再観測は
  evidence 上未確認。

## 修正内容（test 側 — #849 Decision C の踏襲）

`crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_e2e.rs` と
`crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs` の
`instantiate()` retry signature を対称に拡張:

- `phase` 許可集合に `Live` を追加（`{PreSpawn, Spawn, DrainInit,
  Handshake, Live}`）。`Live` は mint 完了直前の `Scan`/`CreateInstance`
  reply wait timeout が freeze する phase — `SpawnFailed` は pre-mint
  定義なので、これを含めても post-mint 契約 verdict を吸収しない。
- `cause == Timeout`・retained record `Found`・
  `record.cause == Some(Timeout)` の三重検査は不変 — 非 Timeout
  cause・`NotFound`/`Evicted` record は signature-miss のまま。
- retry bound `SPAWN_TIMEOUT_RETRIES = 3` 不変。bound 超過時は
  panic で honest fail。
- `instantiate_spawn_failed`（期待値 fail 経路）は retry 対象外のまま
  — `SpawnFailed` が期待値の test 契約は維持。

production（`ruviform-plugin-host/src/**`）は最終 diff で**無変更**。
根拠: `set_phase(Live)` の位置変更は lease 外の
`l1_proxy_e2e.rs:3331/3948` の pin を破壊し、pooled-child mint
（`Live` phase のまま `create_instance` を呼ぶ）には効かないため、
phase 帰属の変更では穴を閉じられない。発火経路は正当な環境期限
（`spawn_timeout` bound の wait）であり、分離策は契約上 retry のみ
— rf#841 Decision C・`oos_instantiate` 先例と同じ境界判定。

## 検証実行記録

### fmt / check / lint（変更 crate）

- `mise run fmt` — exit 0（整形適用）
- `mise run fmt-check` — exit 0（非破壊チェック・CI 同一入口）
- `mise run check-l1-real-plugin` — exit 0
- `mise run lint-l1-real-plugin` — exit 0

### canonical lane 連続 run（real-run evidence — flake 再現率）

`mise run test-l1-real-plugin` を 6 連続実行。全 run で
`--test l1_fault_test_plugin_e2e -- --ignored` leg を含む全 16 suite
ok。生ログ `spawn-timeout-logs/canonical-run{1..6}.log` は先例
（rf763・rf768・rf841・rf846・rf849）どおりローカル保存で commit
対象外 — `*.log` は repo ルート `.gitignore` が除外する。

| run | exit | `test result: ok` suites | FAILED suites | `SpawnFailed` retry trace |
| --- | --- | --- | --- | --- |
| 1 | 0 | 16 | 0 | 0 件 |
| 2 | 0 | 16 | 0 | 0 件 |
| 3 | 0 | 16 | 0 | 0 件 |
| 4 | 0 | 16 | 0 | 0 件 |
| 5 | 0 | 16 | 0 | 0 件 |
| 6 | 0 | 16 | 0 | 0 件 |

観測 `SpawnFailed{Timeout}` 発生率: **0/6 run**（各 run 複数
instantiate を含む）。#846/#849 統合後の稀少化と整合 — rf#849
verification（0/6）と同水準。0/N は「観測できず」の記録であり、
flake の完全排除を主張しない。残存リスクは
`Scan`/`CreateInstance` wait timeout が環境 stall で発火しうる点に
あり、本 slice の signature 拡張でその retry 経路を閉じた。

### induce 補助証拠（deterministic — signature 実動作の直接証明）

canonical lane では `Live` phase `Timeout` を自然発火させられない
ため、`l1/mod.rs` に一時 patch（`RVF_INDUCE_CREATE_TIMEOUT_MS`
env で `CreateInstance` reply wait を `Duration::ZERO` に短縮し
`Live` phase `Timeout` record を強制発火）を適用して実行した
**補助証拠（cargo 直接実行、canonical = mise）**。patch は
`Duration::ZERO`（reply は request 送信前に到着し得ないため決定的に
timeout）を使い、検証後に `git checkout` で完全 revert — 最終 diff
に残らない。

- VST3 `once`（初回 `create_instance` のみ zero-timeout）:
  `RVF_INDUCE_CREATE_TIMEOUT_MS=once cargo test -p
  ruviform-plugin-test-vst3 --features l1-runner --test
  l1_fault_test_plugin_e2e -- --ignored --exact
  real_test_fault_l1_hang_converges_on_timeout`
  → trace `attempt=0 cause=Timeout phase=Some(Live)
  record_cause=Some(Timeout)` → retry → **test result: ok**。
  `Live` phase signature での retry 実動作を直接確認。
- VST3 `always`（全 `create_instance` zero-timeout）:
  trace attempt=0..3 の 4 件全て `phase=Some(Live)` →
  bound 超過で `instantiate failed: ... persisted through the
  3-retry bound` panic → **test FAILED**（honest fail — bound が
  verdict を吸収しないことの確認）。
- CLAP `once`: 同じく `attempt=0 phase=Some(Live)` → retry →
  **test result: ok**。両 ABI の対称実装を確認。

## 残存リスク・対象外（正直な記録）

- `l1_real_test_plugin_e2e.rs`・`l1_stateful_test_plugin_e2e.rs` 等の
  他 lease 外 test file は `SpawnFailed` で即 panic（retry なし）—
  同 signature の穴は残るが lease 外のため本 slice では未修正。
  Coordinator に残件として報告する。
- `l1_proxy_e2e` の timeout-convergence flake（rf#631 記録）は別
  class・lease 外 — 未対応。
- `SpawnFailed{Timeout}` の完全排除は証明しない — 6 run の観測
  0/6 を正直に記録するに留める。
- `Scan`/`CreateInstance` wait の timeout を `Live` phase record と
  して残す現行 phase 帰属は lease 外 pin（`l1_proxy_e2e`）で変更
  不可 — production phase 意味論の再編は本 slice の境界外。
