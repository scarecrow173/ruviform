---
id: "TSPEC-Bn6Xc1RqLw3KtYv8ZmHd2G"
type: "test-spec"
status: "approved"
title: "Issue #448 — 行選択と command 起動の検証契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/448"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  verifies:
    - "docs/specs/issue-448-table-selection-command.md"
    - "docs/designs/issue-448-table-selection-command.md"
  depends-on:
    - "docs/specs/issue-448-table-selection-command.md"
    - "docs/designs/issue-448-table-selection-command.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-435-label-table-kinds.md"
---

# Issue #448 — 検証契約

## Feature: 行選択の intent と選択状態

### Rule: 行選択は Action intent を発行し、選択状態を snapshot で観測できる

- Example: 選択可能 Table の行 1 を選ぶと `action: "select"`・`params: {"row": 1}`・`target` が spec のものになった Action intent が 1 件出る。snapshot の `selected` は `Some(1)`、phase は `pending`。
  - 検査: `table_row_selection_emits_action_and_observes_selected_row`（`ruviform-ui` unit）、`ui_dvr_03_parity_selection_command_intents_and_snapshot`（fake）、`observes_selection_and_command_intents_through_seam`（GPUI seam）。

### Rule: 範囲外・非選択 Table・誤った kind への入力は intent を出さない

- Example: 読み取り専用 Table・行数を超える row・Label への select は `InvalidInput`、Table への invoke も `InvalidInput`。いずれも Action intent と選択状態を変えない。
  - 検査: `selection_rejects_inputs_that_do_not_apply_without_emitting`。

### Rule: 世代が古い入力は拒否される

- Example: 旧 generation での select は `StaleGeneration`。
  - 検査: `selection_rejects_inputs_that_do_not_apply_without_emitting`。

### Rule: rejected / unsupported の選択は直前の行へ戻る

- Example: 行 0 を applied にした後、行 2 を rejected にすると選択は行 0 に戻り、phase は `rejected`。
  - 検査: `rejected_selection_restores_the_previous_row`。

### Rule: 復元は現在の表に存在する行だけを選ぶ

- Example: 行 2 を選んだ後に行 0 を選び、その間に表が 2 行へ縮んだ状態で行 0 の選択が rejected になっても、存在しない行 2 は復元されず `None` になる。
  - 検査: `rejected_selection_does_not_restore_a_row_the_table_lost`。

### Rule: accepted / applied の選択は行を変えない

- Example: 行 1 を accepted → applied と進めても選択は行 1 のまま。phase は `accepted` → `applied`。
  - 検査: `accepted_and_applied_selection_keep_the_row`。

### Rule: 同一 key の更新では、行が残る限り選択を保持する

- Example: 行 2 を選んだ表を同じ行数で更新すると選択は残り、行数 2 に縮むと `None` になる。
  - 検査: `table_update_keeps_selection_only_while_the_row_exists`。

### Rule: 選択可能でなくなった表は選択を持たない

- Example: 選択中の表を読み取り専用の表へ更新すると選択は `None` になり、以後の `select_row` は `InvalidInput`。
  - 検査: `selection_is_dropped_when_the_table_becomes_read_only`。

## Feature: command 起動（Button）

### Rule: Button の押下は spec の target・action・params を運ぶ Action intent を発行する

- Example: `run.start` / `start` / `{"dataset": "ds-1"}` の Button を押すと、同じ 3 値を持つ Action intent が 1 件出る。phase は `pending` → receipt `accepted` で `accepted` → `applied` で `applied`。
  - 検査: `button_invoke_emits_spec_target_action_and_params`（unit）、`ui_dvr_03_parity_selection_command_intents_and_snapshot`（fake）、`observes_selection_and_command_intents_through_seam`（GPUI seam）。

### Rule: Button の a11y 木は role `button` と名前を持つ

- Example: 名前は `a11y.label`、無ければ `label`。
  - 検査: `button_invoke_emits_spec_target_action_and_params`（unit）、`ui_dvr_02_parity_selection_command_mount`（fake、fixture の `a11yTree` と一致）。

### Rule: target か action が空の command 要素は mount できない

- Example: 空の Button target、空の Button action、空の selection target はいずれも `InvalidSpec`。
  - 検査: `button_and_selection_specs_require_a_target_and_an_action`。

### Rule: 削除された要素の receipt は routing されない

- Example: 選択と Button の command を発行した後に両方を削除すると、その receipt は `apply_receipt` が false を返す。
  - 検査: `removed_action_components_drop_their_receipt_routing`。

## Feature: 互換性

### Rule: 既存の Table JSON と Action intent の出力は変わらない

- Example: `selection` を持たない Table は読み取り専用のまま。`params` を持たない Action は `params` キーを出力しない。
  - 検査: 既存の `table_keeps_row_and_column_order_and_missing_cells`、`parity_label_table_*`（#435 の fixture が無変更で pass）、`observes_drawn_label_and_table_plans_through_seam`。

## Feature: 未検証として記録する範囲

### Rule: webview は NotRun として扱い、Pass にしない

- Example: `ruviform-ui-webview` は `ViewSpec` の描画経路を持たないため、webview の intent 発行は検査しない。`cases.json` に webview の binding は追加しない。PR 本文に NotRun と理由を記載する。
- 検査: なし（未対応の明示のみ）。

### Rule: GPUI 側は receipt を送らない

- Example: GPUI seam の検査は intent と発行直後の snapshot（pending）まで。accepted / applied / rejected の遷移は headless 契約（`view::tests`）で検査する。
- 検査: 上記 Rule の参照先。
