# Issue #243 Window/Job 不変条件 Headless 検証 — 実装・検証記録（2026-11-14）

- 正本の実装記録: `docs/impl/ir/issue-243-window-job-headless.md`
- 文書チェーン: `docs/specs/issue-243-window-job-headless.md` →
  `docs/designs/issue-243-window-job-headless-design.md` →
  `docs/plans/issue-243-window-job-headless-plan.md` →
  `docs/tasks/issue-243-window-job-headless-task.md` /
  `docs/test-specs/issue-243-window-job-headless.md`
- 証拠: `evidence/rf243/window-job-headless-coverage.md` /
  `evidence/rf243/window-job-headless-verification.md`

## 1. 変更概要

Issue #243 残存受入の headless スライスとして、
`UiRuntime` + `FakeDriver` を本番 consumer 配線で合成する
in-process integration test `window_job_headless.rs` を追加。
production コード変更なし。既存被覆と重複しない 4 scenario:

- `window_job_cancel_is_exact_and_spares_unowned_jobs`
- `close_policy_cancels_only_the_closed_windows_jobs`
- `job_outliving_its_window_routes_terminal_to_the_dead_window_only`
- `cancel_window_jobs_reaches_orphans_via_stale_window_id`

## 2. 検証条件（比較条件の固定値）

| 項目 | 値 |
| --- | --- |
| 実施日 | 2026-11-14 |
| OS | Windows 11 x86_64 |
| toolchain | mise 管理（`mise.toml` pin、rustc/cargo は mise task 経由） |
| profile | dev / test（`--locked`） |
| 対象 SHA | ブランチ `test/243-window-job-headless` 上の作業 tree |

## 3. 実行結果（canonical = mise）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Pass（差分なし） |
| `mise run check ruviform-ui-test` | 0 | Pass |
| `mise run lint ruviform-ui-test` | 0 | Pass（clippy `-D warnings` clean） |
| `mise run test ruviform-ui-test` | 0 | Pass（全 test target、新規 4 件含む） |

補助証拠: `cargo test --locked -p ruviform-ui-test --test
window_job_headless` → 4 tests / 0 failed / 約 0.22s。
初回コンパイル時に `spawn_sleep(&WindowId)` の型不一致
（`E0308`）と `for event in events` 重複行を検出し修正後に
全件 pass を確認。

## 4. 実行状態の正直な記録

| 項目 | 状態 | 理由 |
| --- | --- | --- |
| Window/Job ownership・close/cancel scoping・stale Window/Receipt の Fake/Headless 不変条件 | Pass | `window_job_headless.rs` 4 件 + 既存被覆の棚卸し（coverage map） |
| Windows 実 OS multiwindow/preflight | Pass（別 Gate・既存証拠） | `evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json`（16/16、本スライスの Pass 根拠には不使用） |
| 実 Window 起動・実入力・動的 DPI/IME | NotRun | GUI-L2 gate（#313）。Fake/Headless Pass を GUI-L2 PASS と主張しない |
| macOS/Linux native 実行 | NotRun | 環境外。CI Run URL 収集は本スライス必須外（Coordinator が Daily/手動 CI で確認） |
| Native Plugin Dock / 実 plugin・device | NotRun | EXT-L3 gate（#388/#389・Synvelis #114/#176） |
| `HeadlessIo`/共有 fixture 経由の scenario 化 | NotRun | `window-lifecycle.v1.json` は共有資産。別 Issue 候補 |
| consumer 実機非回帰（audio-lens/synvelis） | NotRun | cross-repo、別 Owner |

## 5. 備考

- doc graph router は repo-wide の既存 broken relation により
  blocked を返したため、文書は個別 skill の scaffold script と
  手動 front matter で作成。index は doc-maintenance の
  generator で再生成。
- `node` script 実行時に `Buffer()` deprecation warning が
  出力される（repo tooling 由来、変更対象外）。警告抑制は
  行っていない。
