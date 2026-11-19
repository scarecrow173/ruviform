---
id: "TSPEC-034XGrxwj0Si1Ax9A1b2X9"
type: "test-spec"
status: "approved"
title: "Issue #389 dock-test session-lifecycle テスト仕様"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-389-session-lifecycle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-389-session-lifecycle.md"
    - "docs/designs/issue-389-session-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "crates/ruviform-plugin-dock-test/tests/plugin_dock.rs"
    - "docs/test-specs/issue-631-plugin-dock-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-session-lifecycle.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-389-session-lifecycle.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #389 dock-test session-lifecycle テスト仕様

## Purpose

`rf631.plugin-dock` scenario の session-lifecycle 系 3 case が、実 `test_gain` VST3 バイナリ経路で `PluginEditorSessions` の契約を正直に観測することを保証する。実装が変わっても「実走のみ Pass・推測 Pass 禁止・既存 case 非回帰」の verdict 規律を保つための仕様であり、case 削除・harness 撤去時に退役する。

## Feature

Plugin-dock scenario session lifecycle cases — 実バイナリ経路の session begin/busy/refused-open/close-converge/end/re-begin、host 破棄時の `notify_terminated` exactly-once、open-or-degrade の double-refusal honesty を検証する。

## Rules

- Rule: 新 case は artifact 解決成功かつ実バイナリ経路の実走時のみ `Pass`。artifact 不在では fail-closed `NotRun`（`DeterministicBackend`）。
- Rule: 既存 5 case の verdict は不変（`view-mount`=pass・`artifact`=pass・`host-lifecycle`=pass・`editor-attach`=unsupported・`dock-pixels`=not_run）。
- Rule: `session-lifecycle` は `SessionBusy` 拒否・`Ops(Unsupported)` refused open・`Failed{Unsupported}`・adapter close 非 dispatch（`editor_live=false` 収束）・`end_session` 後の instance binding 解放・slot 再利用+generation bump を全て実走して観測する。
- Rule: `host-terminate` は `Destroy` → `notify_terminated(HostLost)` で `Terminated{cause:HostLost}` がちょうど 1 回、再 notify は追加 event なし、`Closed` 収束を観測する。born-`Closed` session ではなく refused-open で `Failed` の live session を使う。
- Rule: `open-or-degrade` は Docked/Floating 両 mode 拒否で `Err(Ops(Unsupported))`・`Failed{Unsupported}`・`Degraded`/`Attached`/`Recovered`/`Reattached`/`Shown` event 0 件を観測する（偽 degrade を出さない）。
- Rule: 各 step・event 列・state 遷移は `observed` に記録し、setup 段階の失敗は `Fail`（黙った skip 禁止）。
- Rule: `environment_kind`=`real-abi-test-plugin` は実走 case のみが名乗る。

## Examples

- Example: artifact 実走環境で `run_all` を実行 → 8 case が返り、新 3 case が `pass`・`environment_kind`=`real-abi-test-plugin`、既存 5 case の verdict が上表どおり。
- Example: artifact 未解決環境で `run_all` を実行 → 新 3 case を含む dependent case が全て `not_run`（`missing_artifact_fails_closed_as_not_run` テストが断言）。
- Example: `session-lifecycle` の `observed.steps` に `begin_session`/`begin_session_again`(session-busy)/`open_docked`(refused-unsupported)/`close_converge`/`end_session`/`rebegin_session`/`destroy` が `ok` で並び、event 列に `Failed{cause:Unsupported}`→`Closed` が記録される。
- Example: `host-terminate` の `observed` に `Terminated{cause:HostLost}` が 1 件だけ含まれ、2 回目の `notify_terminated` 後の drain が空であることが記録される。

## Guarantees

- session registry の状態機械・終端 event exactly-once・lease/binding 解放が実 VST3 バイナリ経路で観測される。
- degrade の非発行（double-refusal で `Degraded` event を捏造しない）が観測される。
- fail-closed 規律と `real-abi-test-plugin`/`deterministic-backend` の environment_kind 分離が維持される。

## Non-goals

- 実 editor attach 成功（`OpenOutcome::Attached`/`Degraded` の実観測）— `test_editor` fixture 未整備。
- 実 window・実 Dock・Native Dock 画素・Input/Focus/Resize — `dock-pixels`=`not_run` 維持。
- session deadline/timeout の wall-clock 実走 — fake-clock 契約は plugin-host 側テストが担保。
- macOS/Linux・3OS・`*-msrv`・`*-full` lane。

## Risk

- verdict 正直性の保証が失われると、fake slot 上の契約適合を実 Plugin/native dock の Pass と誤認する証拠が生まれる — Issue #389 の受入判定を歪める。
