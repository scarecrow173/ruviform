---
id: "TASK-Wd8Cp2Lr6XsNb4KqVf1TzG"
type: "task"
status: "in-progress"
title: "Issue #451 FU-1 — Tabs のキーボード切替の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-451-keyboard-tab-nav.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-451-keyboard-tab-nav.md"
  implemented-by: []
  refines:
    - "docs/tasks/issue-451-tabs-pointer-descriptor.md"
  depends-on:
    - "docs/plans/issue-451-keyboard-tab-nav.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-451-keyboard-tab-nav.md"
---

# Issue #451 FU-1 — Tabs のキーボード切替の実装タスク

計画 `PLAN-Qf3Nr9WxKc7TbVz2LmHd4P` のスライスを、実施順のタスクに分けたもの。

## タスク

- [x] **T1（S1）失敗するテストを先に書く。** `tab_key_target_steps_wraps_and_jumps`、`tab_strip_keys_select_tabs_through_focus`、`observes_tab_key_intents_through_seam`、`ui_dvr_03_parity_tab_keyboard_intents`、fixture の `expect.tabs.keyboard`。
- [x] **T2（S2）表示タブの読み出しと focus の保持。** `displayed_tab(host, key, active)`、`RvfView::tab_focus`。
- [x] **T3（S3）ヘッダのフォーカスと key 処理。** `track_focus`、`tab_stop(true)`、`focus` の枠線、`on_key_down`、mouse-down での `window.focus`、`tab_key_target`。
- [x] **T4（S4）テスト seam。** `GpuiObserver::press_tab_key`、`GpuiDriver::window_of`。
- [x] **T5（S5）検証を実行する。** `fmt` / `fmt-check`、`lint-ui-gpui`、`test-ui-gpui`（render 59 件・consumer_observation 11 件 pass）、`test-ui-parity`（11 件 pass）、`test-ui-driver-contract`、`test-ui-component-view`、`check-ui-driver-boundary`。
- [ ] **T6（S5）実窓でのキー操作を確認する（K3）。** NotRun。`component_showcase` での手動確認は本作業では行っていない。
- [ ] **T7（S6）main の取り込みと検証の再実行。** `git fetch origin main` 後に、取り込みが必要なら再実行する。
- [ ] **T8（S6）文書の索引を再生成する。** `doc-maintenance` の preview で差分を確認してから apply する。
- [ ] **T9（S6）review-protocol のレビューを行う。** blocker が残っていないことを確認する。
- [ ] **T10（S6）PR を作成する。** `Refs #451` を含める。キー割当（WAI-ARIA）と NotRun（K3）を本文に書く。

## follow-up（本 PR の外）

- **FU-1a: 実窓でのキーボード切替の手動証拠（K3）。** `component_showcase` で key 操作を行い、スクリーンショットを残す。
- **FU-1b: ホストアプリの Tab 循環配線。** `focus_next` のバインドを host 側に用意する。
- **FU-1c: Seek / Fader / Knob のキーボード操作。** 共通の key 規約を決めてから扱う。
- **FU-1d: strip のフォーカスハンドルの解放。** unmount 時に削除するかを検討する（現時点では有界）。
