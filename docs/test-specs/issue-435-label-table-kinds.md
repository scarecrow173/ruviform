---
id: "TSPEC-Mv4HxFc7BqKe1NwRj9TsZa"
type: "test-spec"
status: "approved"
title: "Issue #435 — Label/Table の契約・Fake/GPUI parity の検証契約"
created: "2026-11-08"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/435"
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
  depends-on:
    - "docs/specs/issue-435-label-table-kinds.md"
    - "docs/designs/issue-435-label-table-kinds.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-421-meter-timeaxis-gpui-parity.md"
  verifies:
    - "docs/specs/issue-435-label-table-kinds.md"
    - "docs/designs/issue-435-label-table-kinds.md"
---

# Issue #435 — 検証契約

## Feature: 表の行・列の順序と欠損セルの保持

### Rule: 行と列は宣言順のまま保持される

- Example: 列 `stage/state/ms` の順序と、各行のセル順が a11y 木と plan の両方で変わらない。
  - 検査: `table_keeps_row_and_column_order_and_missing_cells`（`ruviform-ui` unit）、`table_plan_keeps_order_and_missing_cells`（gpui-test）、`parity_label_table_payload_retained_fake`（`driver_parity`）。

### Rule: 欠損セルは値なしのセルとして位置を保持する

- Example: `None` のセルは `cell`（value なし）として残り、後続のセルがずれない。
  - 検査: `table_keeps_row_and_column_order_and_missing_cells`（`cell_texts` が `None` を返す位置）、`parity_label_table_a11y_tree_fake`（fixture の valueless cell）。

### Rule: 行長は列数と一致しなければ mount できない

- Example: 3 列の表に 2 セルの行を含むと `ViewError::InvalidSpec`。
  - 検査: `table_rejects_bad_arity_non_finite_and_oversized_payloads`。

## Feature: 値の受け渡しと検証

### Rule: 非有限の数値セルは拒否される

- Example: `CellValue::Number(NaN)` を含む表は mount されない。
  - 検査: `table_rejects_bad_arity_non_finite_and_oversized_payloads`。

### Rule: 行数・列数は上限で有界に保たれる

- Example: `MAX_TABLE_ROWS + 1` 行の表は `InvalidSpec`。
  - 検査: `table_rejects_bad_arity_non_finite_and_oversized_payloads`。

## Feature: a11y 意味論

### Rule: Label の既定 role は text で、テキストは value に保持される

- Example: `a11y.role` 未指定の Label は `role = text`、`value = テキスト`。
  - 検査: `label_a11y_defaults_to_text_role_and_keeps_text`。

### Rule: 表の a11y 木は table → row → columnHeader/cell の構造を持つ

- Example: fixture の `expect.table.a11yTree` と `ViewHost::a11y_node` の JSON が一致する。
  - 検査: `parity_label_table_a11y_tree_fake`（`driver_parity`）。

## Feature: Fake/GPUI parity（同一 fixture）

### Rule: GPUI の描画 plan は fixture の drawn 期待値と一致する

- Example: `expect.table.drawnHeader` / `drawnRows` と `table_plan` の出力が一致する（欠損は `null`）。
  - 検査: `parity_label_table_plans_match_shared_fixture`（gpui-test、`mise run test-ui-gpui`）。

### Rule: GPUI の drawn grid は contract の a11y 木の grid と一致する

- Example: 見出し行を含む drawn grid と、`ComponentSpec::a11y_node` の行・セル値の grid が等しい。
  - 検査: `parity_label_table_plans_match_shared_fixture`（同テスト内の grid 比較）。
  - 注意: platform a11y API への接続は検査対象外（spec の非スコープ）。

### Rule: Label/Table は mount と reconcile で stable key を保つ

- Example: `status` を落としても `events` の identity は残り、`status` は `Removed` になる。
  - 検査: `ui_dvr_02_parity_label_table_mount_reconcile`（`driver_parity`）、`rvf_view_parity_label_table_renders`（gpui-test で描画が通る）。

## 既存挙動の非回帰

### Rule: 既存 kind（Knob/Toggle/Meter/TimeAxis・Panel/Tabs 等）の挙動は変わらない

- Example: #421 の parity fixture・gesture・reconcile・描画テストは無変更で pass する。
  - 検査: `ui_dvr_02_parity_new_kinds_mount_reconcile` / `ui_dvr_03_parity_new_kinds_gesture` / `rvf_view_parity_new_kinds_*`、`component_view`、`consumer_observation`。
