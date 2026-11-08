---
id: "TSPEC-Js8Lv3XqRb6NwKt1YcMg5F"
type: "test-spec"
status: "approved"
title: "Issue #451 — Tabs pointer 切替と descriptor 更新の検証契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  verifies:
    - "docs/specs/issue-451-tabs-pointer-descriptor.md"
    - "docs/designs/issue-451-tabs-pointer-descriptor.md"
  depends-on:
    - "docs/specs/issue-451-tabs-pointer-descriptor.md"
    - "docs/designs/issue-451-tabs-pointer-descriptor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-448-table-selection-command.md"
---

# Issue #451 — 検証契約

## Feature: Tabs の pointer 切替と intent

### Rule: タブ選択は tab-select の Action intent を発行し、表示タブを snapshot で観測できる

- Example: 選択可能な Tabs のタブ 2 を選ぶと、`action: "tab-select"`・`params: {"index": 2}`・spec の `target` を持つ Action intent が 1 件出る。snapshot の `selected` は `Some(2)`、phase は `pending`。
  - 検査: `tab_select_emits_action_and_observes_displayed_tab`（`ruviform-ui` unit）、`ui_dvr_03_parity_selection_command_intents_and_snapshot`（fake）、`observes_selection_and_command_intents_through_seam`（GPUI seam）。

### Rule: 範囲外・非選択 Tabs・誤った key・古い世代の入力は intent を出さない

- Example: selection を持たない Tabs への選択、タブ数を超える index は `InvalidInput`。存在しない key は `UnknownComponent`、旧 generation は `StaleGeneration`。いずれも intent を出さず、表示タブを変えない。
  - 検査: `tab_selection_rejects_inputs_that_do_not_apply_without_emitting`。

### Rule: rejected / unsupported のタブ選択は直前のタブへ戻る

- Example: タブ 1 を applied にした後、タブ 2 を rejected にすると表示は タブ 1 に戻る。
  - 検査: `rejected_tab_selection_restores_the_previous_tab`。

### Rule: 同一 key の update は spec の active の変化を採用し、変化がなければ表示タブを保持する

- Example: 利用者がタブ 2 を選んだ後、`active` が変わらない update（内容だけ変更）では表示はタブ 2 のまま。`active` が 1 に変わった update では表示は 1 になる。タブ数が減って表示中のタブが無くなった場合は `active` に戻る。
  - 検査: `tab_update_adopts_a_new_active_tab_and_keeps_the_displayed_one`。

### Rule: タブヘッダの押下は、その実要素の mouse 入力経路で選択を変える（GPUI）

- Example: TestAppContext で、2 番目のタブヘッダ（`mixer_tabs-tab-1`）の bounds の中心に mouse-down を送ると、ホストの表示タブが 1 になる。
  - 検査: `tab_header_press_selects_the_tab_through_the_pointer`（`ruviform-ui-gpui` の `gpui-test`）。

### Rule: 描画は表示タブの本文だけを描く（GPUI）

- Example: タブ 2 を選択した後、drawn plan には `routing.body`（タブ 2 の本文）があり、`mixer.body`（タブ 1 の本文）は無い。
  - 検査: `observes_selection_and_command_intents_through_seam`（drawn の検査）、`drawn_plans_follow_paint_tree`。

## Feature: mount 後の descriptor 更新

### Rule: descriptor 更新は generation を変えず、新しい range で値を解釈する

- Example: range 0〜240 の Seek を range 0〜600 へ update する。generation は変わらない。その後に 300 の projection を採用すると、表示値は 300 になる（旧 range の外の値）。
  - 検査: `descriptor_update_widens_seek_range_without_remount`。

### Rule: descriptor 更新は進行中の command の receipt を失わない

- Example: Seek の commit intent を発行した後（receipt 待ち）に range を変える update を行い、その commit の `applied` receipt を渡すと binding は `applied` になる。
  - 検査: `descriptor_update_keeps_in_flight_receipt_routing`（修正前は失敗することを確認済み）。
  - 既存の回帰: `update_reregister_keeps_binding_epoch`、`remount_absorbs_stale_receipts`（key の削除と remount では古い receipt が引き続き無効）。

## Feature: 共有 fixture と契約の境界

### Rule: Fake と GPUI は同じ fixture から同じ intent と snapshot を得る

- Example: `selection-command.json` の `expect.tabs`（intent、`selectedAfterSelect`、phase、`drawnBody`、`undrawnBody`）を Fake と GPUI が検査する。
  - 検査: `driver_parity.rs`（fake）、`consumer_observation.rs`（gpui）。

### Rule: 共通層は GPUI 型に依存しない

- Example: `ruviform-ui` に GPUI 型が現れない。
  - 検査: `mise run check-ui-driver-boundary`。

## 実機・未検証（NotRun）

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| GPUI 実窓でのタブのクリック切替 | 手動証拠あり（Windows、`component_showcase`、OS 実ポインタ） | 自動化されていない。CI では再現しない。スクリーンショットは PR に記載 |
| GPUI のキーボードによるタブ切替 | NotRun | 未実装（FU-1） |
| webview の Tabs 描画と intent | NotRun | `ruviform-ui-webview` に `ViewSpec` の描画経路がない（FU-2） |
| consumer 側の project length 変化の接続 | NotRun（本 PR 外） | consumer 作業（FU-3） |
