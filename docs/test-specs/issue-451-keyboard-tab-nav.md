---
id: "TSPEC-Yn5Gk3Rt9MvQc2BxLw7DeJ"
type: "test-spec"
status: "approved"
title: "Issue #451 FU-1 — Tabs のキーボード切替の検証契約"
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
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  verifies:
    - "docs/specs/issue-451-keyboard-tab-nav.md"
    - "docs/designs/issue-451-keyboard-tab-nav.md"
  depends-on:
    - "docs/specs/issue-451-keyboard-tab-nav.md"
    - "docs/designs/issue-451-keyboard-tab-nav.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-451-tabs-pointer-descriptor.md"
---

# Issue #451 FU-1 — 検証契約

## Feature: Tabs のキーボード切替

### Rule: 移動先は左右の循環と Home/End で求め、表示中のタブと同じ移動は何もしない

- Example: 3 タブで表示中のタブ 0 から Right で 1、タブ 2 から Right で 0（循環）。タブ 0 から Left で 2、Home で 0、End で 2。タブ 2 の End と タブ 0 の Home は移動なし（`None`）。未知の key（`enter` など）は `None`。タブ数 1 と 0 では移動しない。
  - 検査: `tab_key_target_steps_wraps_and_jumps`（GPUI render unit、gpui-test feature）。

### Rule: フォーカスしていない strip は key を受けない。ヘッダ押下でフォーカスし、以後の key で選択が変わる

- Example: 起動直後に Right を送っても選択は 0 のまま。タブ 1 のヘッダを押すと選択は 1 になり、Left・Left・Home・End を送ると選択は 0・1・0・1 と変わる。発行された `tab-select` intent の index は 1, 0, 1, 0, 1 の順。
  - 検査: `tab_strip_keys_select_tabs_through_focus`（GPUI gpui-test。実フォーカスと `simulate_keystrokes` を使う）。

### Rule: キーによる選択は pointer と同じ `tab-select` intent と snapshot を生む

- Example: 観測 seam で `mixer_tabs` にフォーカスして Right を送ると、intent は `{kind: action, target: mixer.tabs.select, controlId: mixer_tabs, action: tab-select, params: {index: 1}}` の 1 件。snapshot の `selected` は 1。
  - 検査: `observes_tab_key_intents_through_seam`（GPUI consumer seam）。fixture の `expect.tabs.keyboard` を両側が参照する。

### Rule: Fake 側でも、キーボード経路の選択結果は同じ intent と snapshot を生む

- Example: fixture の `keyboard.selectedAfterKey`（1）を契約の `select_tab` に渡すと、intent は `keyboard.intents` と一致し、snapshot の `selected` は 1。Fake 側は key 入力を持たないため、この検査は契約の intent 形状の一致に限られる。
  - 検査: `ui_dvr_03_parity_tab_keyboard_intents`（Fake、`ruviform-ui-test`）。

## 未検証（NotRun）

- **実窓でのキー操作。** GPUI の実ウィンドウ上で key を送って切り替えることは、本作業では行っていない。gpui-test の仮想プラットフォームでの検査に限る。手動証拠は FU-1a で扱う。
- **Tab 循環。** ホストアプリの `focus_next` バインドがない既定の状態では、Tab で strip に到達しない。ホスト配線（FU-1b）の検査は後続とする。
