---
id: "DESIGN-Pw6NsJc4VhTq9BrXy1MaFe"
type: "design"
status: "approved"
title: "Issue #435 — Label/Table ViewSpec kind と parity の設計"
created: "2026-11-08"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/435"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/label-table.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-435-label-table-kinds.md"
---

# Issue #435 — 設計

## 1. 契約（`ruviform-ui` / `view.rs`）

### 1.1 型

| 型 | 役割 |
| --- | --- |
| `LabelSpec { key, text, visual, a11y }` | 静的テキスト。`text` は consumer 解決済みの表示文字列 |
| `TableSpec { key, columns: Vec<String>, rows: Vec<Vec<Option<CellValue>>>, visual, a11y }` | 読み取り専用の表。列見出しと行。`None` は欠損セル |
| `CellValue::{Text(String), Number(f64)}` | 解決済みセル値。`serde(untagged)` で JSON の文字列・数値をそのまま運ぶ |
| `A11yNode { role, name, value, children }` | contract が導出する a11y 木。renderer はこの木を消費する |
| `ComponentSpec::Label` / `ComponentSpec::Table` | enum 追加。serde tag は `label` / `table` |
| `A11yRole::{Text, Table, Row, ColumnHeader, Cell}` | a11y role 追加（`non_exhaustive` enum への追加のため非破壊） |

定数: `MAX_TABLE_ROWS = 4096`、`MAX_TABLE_COLUMNS = 64`。

### 1.2 検証（mount/update 時）

- `rows.len() <= MAX_TABLE_ROWS`、`columns.len() <= MAX_TABLE_COLUMNS`。
- 各行の長さは `columns.len()` と一致（列ずれを防ぐ。欠損は `None` で埋める）。
- `CellValue::Number` は有限値のみ（NaN/±∞ は拒否）。
- 違反は既存と同じ `ViewError::InvalidSpec`。

### 1.3 a11y 木の導出

- `ComponentSpec::a11y_node()`（`Label`/`Table` のみ `Some`）。`ViewHost::a11y_node(key)` が mounted spec から導出する。
- Table の木: 根 `table`（name = `a11y.label`）→ 先頭に見出し行 `row`（子は `columnHeader`）→ 各データ行 `row`（子は `cell`）。
- 欠損セルは `cell`（value なし）として残し、列位置を保持する。
- `a11y.role` が指定されていれば根の role を上書きする（既定は `table` / `text`）。

## 2. 決定事項

| ID | 決定 | 理由 |
| --- | --- | --- |
| D1 | `Label` の「role」は `a11y.role` とし、既定は `A11yRole::Text` | 仕様上の role の概念は `A11ySpec` のみ。見た目の強調（heading 等）は要件になく YAGNI |
| D2 | `List` は独立 kind にせず、一列の `Table` で表す | 要件の「Table または List」のうち最小。二種類の kind を持つと同じ a11y 意味論が二重になる（DRY） |
| D3 | 欠損は `Option::None`（JSON `null`）でセル位置を保持、行長は列数と一致必須 | 「欠損セル保持」の要件。欠損で列がずれると consumer が誤った値を読む |
| D4 | 数値の表示文字列は `CellValue::Display`（f64 の既定形式）を唯一の表現とする | renderer 間の表示ずれを防ぐ。locale・単位・桁区切りは consumer の解決責務 |
| D5 | a11y 木は contract が一度だけ導出し、renderer は消費する | 表の意味論（行・列・見出し）の authoritative 実装を一箇所に置く。GPUI 側で再導出しない |
| D6 | 上限は行・列を別々に固定（`4096` / `64`） | 既存の Waveform/Meter/TimeAxis と同じく payload を有界にする。単一のセル総数上限だと行数 0 の列の扱いが曖昧になる |
| D7 | Label/Table は非 control（`ControlSpec` なし）。gesture/intent は発行しない | 読み取り専用の表示 kind。interaction は要件外 |
| D8 | parity fixture は `driver-lab/parity/label-table.json` として新規作成し、`new-kinds.json`（#421）は変更しない | #421 の fixture は manifest・description が #421 固有。別 fixture で責務を分ける |

## 3. Fake 側（ViewHost）

- mount/update は既存の reconcile に乗る（Label/Table は control を持たないため `register_control` は行わない）。
- projection は `Mounted`/`Updated`/`Removed` の op で表現する。payload はそのまま保持する（既存 Meter/TimeAxis と同じ）。
- `observe(key)` は mounted であれば snapshot を返す（display/committed は None）。

## 4. GPUI 側（renderer）

- `table_plan(spec) -> TablePlan { header, rows }` は純関数。`None` は描画上は空セル。
- `table_element` は plan を消費し、見出し行（dim 色）と各データ行を描画する。`label_element` は primary 色のテキスト。
- `element_for` の `other =>` フォールバックに入らないよう、Label/Table の arm を明示追加する。
- platform a11y API への接続は行わない（非スコープ）。その代わり、plan の grid と contract の a11y 木の grid が一致することを検査する。

## 5. 検証設計

| 層 | 検証 | 場所 |
| --- | --- | --- |
| contract | 順序・欠損保持・a11y 木・既定 role・検証（arity/NaN/上限） | `ruviform-ui` の `view::tests`（unit） |
| fake | parity fixture の mount/reconcile、a11y 木の fixture 一致、payload 保持 | `ruviform-ui-test/tests/driver_parity.rs` |
| GPUI | plan の fixture 一致、drawn grid = a11y grid、順序・欠損の単体、RvfView での描画 | `ruviform-ui-gpui/src/render.rs`（gpui-test） |

## 6. 互換性

- `ComponentSpec` / `A11yRole` は既存の `non_exhaustive` のため、variant 追加は非破壊。
- 既存 kind の挙動・payload・intent は変更しない（既存テスト無変更で pass）。
- JSON 形式は `type: "label" | "table"` の追加のみ。既存の `type` 値は不変。
