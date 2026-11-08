---
id: "TASK-G72QPe3EioqQac8Dsnoglk"
type: "task"
status: "in-progress"
title: "Issue #479 — 値制御キーボード操作の実装タスク"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/479"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-479-keyboard-value-controls.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-479-keyboard-value-controls.md"
  implemented-by: []
  refines:
    - "docs/tasks/issue-451-keyboard-tab-nav.md"
  depends-on:
    - "docs/plans/issue-479-keyboard-value-controls.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-479-keyboard-value-controls.md"
---

# Issue #479 — 値制御キーボード操作の実装タスク

計画 `PLAN-1OVcOgiEi7tc3VMXPzF9F3` のスライスを、実施順のタスクに分けたもの。

## タスク

- [x] **T1（S1）失敗するテストを先に書く。** `control_key_input_maps_keys_to_atomic_inputs`、`control_keys_adjust_value_through_focus`、`tab_traversal_reaches_value_controls`、`observes_key_op_intents_through_seam`、`ui_dvr_03_parity_key_ops_keyboard_intents`、fixture `key-ops.json`。
- [x] **T2（S2）focus の保持と key→入力の写像。** `control_focus` フィールドと `control_focus()`、`control_key_input`。
- [x] **T3（S3）element への配線。** `control_element` / `toggle_element` に `track_focus`・`tab_stop`・`.focus`・`on_key_down`。
- [x] **T4（S4）テスト seam。** `GpuiObserver::press_control_key`、`GpuiObserver::focused_control`、`RvfView::focused_control_key`。
- [x] **T5（S5）検証を実行する。** `fmt` / `fmt-check`、`lint-ui-gpui`、`test-ui-gpui`、`test-ui-parity`、`test-ui-driver-contract`、`test-ui-component-view`、`check-ui-driver-boundary`。
- [x] **T6（S5）実窓でのキー操作を確認する（K4）。** NotRun。実 OS 入力経路は #313 の責務。
- [x] **T7（S6）main の取り込みと検証の再実行。** `git fetch origin main` 後に、取り込みが必要なら再実行する。
- [x] **T8（S6）文書の索引を再生成する。** doc-maintenance の preview で差分を確認してから apply する。
- [x] **T9（S6）review-protocol のレビューを行う。** blocker が残っていないことを確認する。
- [ ] **T10（S6）PR を作成する。** `Refs #479` を含める。キー規約と NotRun（K4）を本文に書く。

## follow-up（本 PR の外）

- **FU-a: 実窓でのキー操作の手動証拠（K4）。** `component_showcase` で key 操作を行い、スクリーンショットを残す。
- **FU-b: Space/Enter 等の離散起動キーと Button のキーボード起動。** プラットフォームの起動キー規約を決めてから扱う。
- **FU-c: control フォーカスハンドルの解放。** unmount 時に削除するかを検討する（`tab_focus` と同じ有界リーク）。
