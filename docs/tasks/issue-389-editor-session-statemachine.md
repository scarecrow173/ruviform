---
id: "TASK-Q2ek2gQVRR1KvvM8kyaMei"
type: "task"
status: "done"
title: "Issue #389 Plugin Editor Session fake host driver 状態機械 fixture タスク"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/impl/ir/issue-389-editor-session-statemachine-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-389-editor-session-statemachine.md"
    - "docs/test-specs/issue-389-editor-session-statemachine.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
    - "docs/tasks/issue-389-editor-session-headless.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-389-editor-session-statemachine.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-editor-session-statemachine.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 Plugin Editor Session fake host driver 状態機械 fixture タスク

正本: `docs/plans/implement-issue-389-editor-session-statemachine.md`。
設計: `docs/designs/issue-389-plugin-editor-session.md`（改訂 1
R3–R6）・`docs/designs/issue-389-session-hardening.md`（D1–D5）。
検証仕様: `docs/test-specs/issue-389-editor-session-statemachine.md`
（SM-xx）。

## チェックリスト

- [x] T-1 `crates/ruviform-plugin-host/tests/editor_session_driver.rs`
  新設 — `FakeHostDriver`（sessions registry + `ScriptedOps` +
  共有 `FakeClock` deadline policy + pending completion queue +
  driver 側 `live_editors` 帳簿 + stale drop/reject 計数）と
  SM-xx scenario test 追加。
- [x] T-2 `docs/test-specs/issue-389-editor-session-statemachine.md`・
  `docs/tasks/`・`docs/plans/` 作成（SM-Rx rule・focus/Generic
  境界を記録）。
- [x] T-3 impl record 作成・NotRun 記録・`mise run` 検証
  （fmt-check/check/lint/test/test-rt 全て exit 0）・
  review-protocol・PR。

## 完了条件

test-spec SM-xx 全 case green + workspace fmt/check/lint/test/
test-rt green。実 plugin/実 OS/実 GUI/consumer gate は NotRun
記録のまま（本 dispatch 範囲外）。`src/` 公開契約・依存・
lockfile・既存 `tests/editor_session.rs` の差分なし。

## 追補候補（本 slice 対象外・follow-up）

- 実 driver（GPUI/composition root）からの `tick`/`tick_all` 定期
  呼出接続と `FakeHostDriver` 相当の production 側参照実装
  （hardening の deferred・実 driver 側作業）。
- 「Generic Editor」fallback UI の consumer 側実装（#388 slot/
  window 層・別 slice）。
- raw `GuiParentToken` の公開面からの除去（`Plugin` trait 契約
  変更を伴う別 Issue）。
- L1 GUI wire 動詞（issue-115 解除条件充足後の別 Issue）。
- async `complete_close` への error channel 追加の要否
  （現行は timeout 経由収束 — contract 変更を伴う別 Issue）。
