# Issue #243 Window/Job Headless — 実行検証記録

- 作成日: 2026-11-14
- 対象 SHA: ブランチ `scarecrow173/rf243-window-job` 上の
  作業 tree（`git rev-parse HEAD` = `71445679`、
  `origin/main` merge 済みの最新 main と同一）
- 正本コマンド: `mise.toml` のタスク。cargo 直接実行は
  補助証拠として明記。

## 検証条件（比較条件の固定値）

| 項目 | 値 |
| --- | --- |
| 実施日 | 2026-11-14 |
| OS | Windows 11 x86_64 |
| toolchain | mise 管理 rustc/cargo 1.99.1（`mise exec -- rustc --version` 実測） |
| profile | dev / test（`--locked`） |
| 変更範囲 | `crates/ruviform-ui-test/tests/window_job_headless.rs`（新規）+ docs/evidence |

## 実行結果（canonical = mise、全て exit 0）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | `cargo fmt --all -- --check` clean（差分なし） |
| `mise run check ruviform-ui-test` | 0 | `cargo check --locked --all-targets -p ruviform-ui-test` Pass |
| `mise run lint ruviform-ui-test` | 0 | clippy `-D warnings` clean |
| `mise run test ruviform-ui-test` | 0 | 全 test target Pass。`window_job_headless` 4 tests / 0 failed / 0.22s を含む（lib 72 + component_view 17 + driver_boundary 11 + driver_contract 7 + driver_parity 14 + e2e_process 18 + fake_driver 14 + golden 8 + headless_contract 33 + lifecycle_fixture 1 + native_slot_contract 13 + scenario_contract 16 + window_job_headless 4） |

### 新規テスト個別の結果（`window_job_headless.rs`）

| テスト | 結果 | 検証した不変条件 |
| --- | --- | --- |
| `window_job_cancel_is_exact_and_spares_unowned_jobs` | pass | `cancel_window_jobs(w)` の戻り値が正確（`== 1`）、他 window job・ownerless（`dispatch_op` = App/Project-owned 相当）job を cancel しない、ownerless terminal は `window: None` のまま |
| `close_policy_cancels_only_the_closed_windows_jobs` | pass | `CloseRequested` Veto で window/job 生存、Approve→`Destroyed` 後に consumer policy が閉じた window の job のみ cancel、sibling/ownerless 生存、sibling は自身宛 intent に応答継続 |
| `job_outliving_its_window_routes_terminal_to_the_dead_window_only` | pass | owner 破棄後も job が終端、terminal `job` event と command receipt（`cmd-doom` `ok:true`）が記録済み stale `WindowId` 宛に routing、delivery 層が dead-window 宛を drop（`delivered` に追加なし）、sibling へ leak なし、stale `Action` は result を産出しない、`destroy_window` 再呼出 error・`WindowId` 非再利用 |
| `cancel_window_jobs_reaches_orphans_via_stale_window_id` | pass | stale `WindowId` が自身の orphan job のみを cancel（戻り値 `== 1`）、orphan terminal は stale id 宛に routing され delivery で drop、別 window の job 生存、id 非再利用 |

### 補助証拠（canonical = mise を補助する直接 cargo 実行）

- `cargo test --locked -p ruviform-ui-test --test
  window_job_headless` → 4 tests / 0 failed / 約 0.22s
  （exit 0。mise test の一部としても同一 binary が実行済み）

### 期待/観測の対応（抜粋）

- 期待: `cancel_window_jobs(w1) == 1` → 観測: assert 通過
  （w2 job・ownerless job は `has_running_jobs()` で生存確認）
- 期待: dead window 宛 terminal が sibling に leak しない
  → 観測: j1/j2 の全 `Event::Job` が期待 owner のみ宛、
  `events_for(w1)` の件数が destroy 後に増えない
- 期待: stale inbound `Action` が result を産出しない
  → 観測: routed journal に `stale-intent` の `Result`
  が存在しない

## 実行状態の正直な記録（NotRun / Unsupported / Deferred）

| 項目 | 状態 | 理由・後続 Gate |
| --- | --- | --- |
| Fake/Headless での Window/Job 不変条件 | **Pass（OS-L1 契約証拠）** | 本スライスの 4 テスト + 既存被覆（coverage map 参照）。実 Window・実入力を意味しない |
| Windows 実 OS multiwindow/preflight | Pass（**別 Gate・既存証拠**） | `evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json`（16/16、PR #551/#570 系）。本スライスの Pass 根拠には使用しない |
| 実 Window 起動・実入力・動的 DPI/IME・実マウス/キーボード | **NotRun** | GUI-L2 gate。owner: Issue #243 GUI-L2 スライス / #313 real-OS lane。Fake/Headless Pass を GUI-L2 PASS と主張しない |
| macOS/Linux native 実行 | **NotRun** | 本 dispatch 環境外（Windows のみ）。CI Run URL 収集は本スライス必須外 — Coordinator が既存 Daily/手動 CI で確認。owner: 後続 #243 dispatch（OS 別） |
| Native Plugin Dock・実 plugin/device | **NotRun** | EXT-L3 gate。owner: #388/#389・Synvelis #114/#176 |
| `WindowLifecycle::Moved` の FakeDriver 側発火 | **NotRun** | FakeDriver は move 要求 API を持たず `Moved` は inject のみ。view move（dock 2-phase）は `dock.rs`/`native_slot_contract.rs`/`driver_contract.rs` UI-DVR-05 で被覆済み |
| consumer 実機非回帰（audio-lens/synvelis） | **NotRun** | cross-repo、別 Owner |
| GPUI simulated/real lane での同一不変条件 | **NotRun（本スライス対象外）** | 既存 `scenario_contract.rs`/`windowed_uitest.rs`/`real_window.rs` が部分被覆。GPUI 側追加は別 slice |
| `HeadlessIo`/共有 fixture（`window-lifecycle.v1.json`）経由の scenario 化 | **NotRun** | 共有資産変更を伴うため本スライス対象外。別 Issue 候補 |

## 備考

- 上記 4 コマンドは作業 tree の未コミット状態で実行し、
  その後 `git merge origin/main`（`71445679` → `3d981b75`、
  docs/他 crate のみの差分）後に同一 4 コマンドを再実行して
  全て exit 0 を再確認した。merge は index 生成物を含まない
  コード差分を `ruviform-ui-test` に持ち込まないが、
  念のため canonical lane を merged tree で再検証した。
- `node` script（docs 生成系）実行時に `Buffer()` deprecation
  warning が出力される（repo tooling 由来、変更対象外）。
  警告抑制は行っていない。
- ローカル検証 = 変更範囲（`ruviform-ui-test`）の検証。
  統合検証は merge 後の対象 SHA の CI 証拠（Coordinator 確認）。
