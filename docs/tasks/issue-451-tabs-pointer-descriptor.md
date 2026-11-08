---
id: "TASK-Ym6Rc1EkVw8PsTn4ZhDa3G"
type: "task"
status: "in-progress"
title: "Issue #451 — Tabs pointer 切替と descriptor 更新の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-451-tabs-pointer-descriptor.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-451-tabs-pointer-descriptor.md"
  implemented-by: []
  refines:
    - "docs/tasks/issue-448-table-selection-command.md"
  depends-on:
    - "docs/plans/issue-451-tabs-pointer-descriptor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-451-tabs-pointer-descriptor.md"
---

# Issue #451 — Tabs pointer 切替と descriptor 更新の実装タスク

計画 `PLAN-Wp3Nd7TzKb5QxHv2RcLf9J` のスライスを、実施順のタスクに分けたもの。

## タスク

- [x] **T1（S1）descriptor 更新で receipt routing を保つ。** 失敗するテスト `descriptor_update_keeps_in_flight_receipt_routing` を先に追加し、`detach_control` を導入して修正する。
- [x] **T2（S1）Seek range の更新を確認する。** `descriptor_update_widens_seek_range_without_remount` を追加する。
- [x] **T3（S2）Tabs の選択モデルを追加する。** `TabsSpec.selection`、`TAB_SELECT_ACTION`、`select_tab`、`emit_selection`、`carried_selection` を追加する。
- [x] **T4（S2）view::tests を追加する。** 発行、拒否、revert、update の採用規則。
- [x] **T5（S3）GPUI の表示と pointer を実装する。** `on_mouse_down`、`select_tab`、`displayed_tab`、`drawn_plans(host)`、`debug_selector`、`GpuiObserver::select_tab`。
- [x] **T6（S3）GPUI の pointer テストを追加する。** `tab_header_press_selects_the_tab_through_the_pointer`。
- [x] **T7（S4）共有 fixture と Fake 側を更新する。** `selection-command.json`、`driver_parity`、`consumer_observation`、`TabsSpec` literal の追従。
- [x] **T8（S5）手動の実窓証拠を取る。** `component_showcase` で OS 実ポインタにより `Ch 1 → Ch 2 → Ch 1` を確認。
- [ ] **T9（S7）検証を実行する。** `fmt-check`・`lint`・`lint-ui-gpui`・`test`・UI タスク群を main 取り込み後に再実行する。
- [ ] **T10（S6）文書の索引を再生成する。** 各 `README.md` に行を追加し、front matter を検証する。
- [ ] **T11（S7）PR を作成する。** 検証結果、NotRun、実窓の証拠、残課題（キーボード、webview、consumer 側の project length 接続）を PR 本文に書く。

## follow-up（本 PR の外）

- **FU-1: GPUI タブのキーボード操作。** タブヘッダへのフォーカスと key 入力を追加し、`ViewHost::select_tab` を呼ぶ。契約は変更不要。
- **FU-2: webview driver での Tabs 描画と intent。** `ruviform-ui-webview` に `ViewSpec` の描画経路ができた時点で parity に加える。
- **FU-3: consumer の project length 変化の接続。** Synvelis 側で length 変化を検知して `update_view` を呼ぶ（consumer 側作業）。
