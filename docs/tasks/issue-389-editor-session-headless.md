---
id: "TASK-hCLO8YPWLtzGOIGdhW5yxp"
type: "task"
status: "done"
title: "Issue #389 Plugin Editor Session headless fixture タスク（contract 認定 + DSP/RT 非干渉 fixture）"
created: "2026-11-09"
updated: "2026-11-09"
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
    - "docs/impl/ir/issue-389-editor-session-headless-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-389-editor-session-headless.md"
    - "docs/test-specs/issue-389-editor-session-headless.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-389-editor-session-headless.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-editor-session-headless.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 Plugin Editor Session headless fixture タスク

正本: `docs/plans/implement-issue-389-editor-session-headless.md`。
設計: `docs/designs/issue-389-plugin-editor-session.md`（改訂 1
R3–R6）・`docs/designs/issue-389-session-hardening.md`（D1–D5）。
検証仕様: `docs/test-specs/issue-389-editor-session-headless.md`
（EH-01..05）。

## チェックリスト

- [x] T-1 `crates/ruviform-plugin-host/tests/editor_session.rs` —
  EH-01 `visibility_noop_emits_no_event`・EH-02
  `l0_session_open_close_leaves_dsp_processing_untouched`・
  EH-03 `l0_session_abnormal_termination_leaves_dsp_processing_untouched`・
  EH-04 `l0_refused_open_leaves_dsp_processing_untouched` 追加 +
  `drive_to_processing`/`process_block`/`assert_dsp_untouched`
  helper（実 `InProcessHost` `Processing` 経路）。
- [x] T-2 `docs/test-specs/issue-389-editor-session-headless.md`・
  `docs/tasks/`・`docs/plans/` 作成（EH-R1..R5・focus 境界 EH-R5
  を記録）。
- [x] T-3 impl record 作成・NotRun 記録・`mise run` 検証
  （fmt-check/check/lint/test/test-rt 全て exit 0）・
  review-protocol・PR。

## 完了条件

test-spec EH-01..04 green + workspace fmt/check/lint/test/test-rt
green。実 plugin/実 OS/共同検証行は NotRun 記録のまま（本
dispatch 範囲外）。src 公開契約・依存・lockfile の差分なし。

## 追補候補（本 slice 対象外・follow-up）

- 実 Plugin/Format の L0/L1 capability 実機受入・GUI hang 非波及
  実機検証（Issue #389 残 DoD・NotRun 継続）。
- driver（GPUI/composition root）からの `tick`/`tick_all` 定期
  呼出接続（hardening の deferred・実 driver 側作業）。
- raw `GuiParentToken` の公開面からの除去（`Plugin` trait 契約
  変更を伴う別 Issue）。
- L1 GUI wire 動詞（issue-115 解除条件充足後の別 Issue）。
