---
id: "SPEC-Lk3RvQ8mTxWn5HcZb2GdYu"
type: "spec"
status: "proposed"
title: "Issue #435 — Label/Table ViewSpec kind と Fake/GPUI parity"
created: "2026-11-08"
updated: "2026-11-08"
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
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-387-component-view-specs.md"
    - "docs/specs/issue-421-meter-timeaxis-gpui-parity.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
---

# Issue #435 — Label/Table ViewSpec kind と Fake/GPUI parity

## 背景

AudioLens の製品画面（Job / Run / Analysis / Source の要約）は、状態テキスト・stage 表・events 表・ラベルを表示する（scarecrow173/audio-lens#166 AL-G3）。現行 `ComponentSpec`（#387・#421）は Panel / Tabs / Fader / Seek / Knob / Toggle / Waveform / Meter / TimeAxis / NativeSlot のみで、文字情報の運び手は Panel の `title` と `A11ySpec.label` / `value_text` だけだった。行と列を持つ表は表現できず、AudioLens が独自規約で詰めると契約の迂回（重複実装）になる。

## 目的

- 最小の表示 kind として `Label`（テキストと role）と `Table` を `ComponentSpec` に追加する。`List` は一列の `Table` として表現し、独立 kind は追加しない。
- 値はすべて consumer が解決済みの文字列・数値として渡す。contract 側で値を再計算・整形しない。
- Label/Table を含む parity fixture で、Fake（ViewHost）と GPUI の projection と a11y tree を同一 fixture で検査する。
- 行・列の順序と欠損セルが保持されることを単体テストで保証する。

## 受入条件（Issue #435 本文に対応）

| # | Issue #435 受入 | 本 PR での扱い |
| --- | --- | --- |
| A1 | `Label`（テキストと role）と `Table`/`List` を `ComponentSpec` に追加する | 達成。`Label`・`Table` を追加。`List` は一列 `Table` として表現（決定 D2） |
| A2 | 列定義・行データ・行列の a11y 意味論を持つ | 達成。`A11yNode` 木（table → row → columnHeader/cell）を contract が導出。欠損セルは値なしの cell として位置を保持 |
| A3 | 値は consumer が解決済みの文字列・数値として渡す | 達成。`CellValue::{Text, Number}`。非有限数は mount 時に拒否 |
| A4 | ラベルと表を含む parity fixture で Fake/GPUI の projection と a11y tree が一致する | 部分達成。Fake は `ViewHost::a11y_node` が fixture の `a11yTree` と一致。GPUI は drawn grid が contract の a11y grid と一致することを検査。GPUI の platform a11y API への接続は未実装（renderer に a11y 接続は現状なく、a11y の実 OS 接続は別 Gate。下記非スコープ） |
| A5 | 行・列の順序と欠損セルが保持される単体テスト | 達成。contract 側（`table_keeps_row_and_column_order_and_missing_cells`）、GPUI plan 側（`table_plan_keeps_order_and_missing_cells`）、fixture 側（`parity_label_table_payload_retained_fake`） |
| A6 | 既存 kind の挙動を変えない | 達成。既存 `new-kinds.json` 系の parity・gesture・render テストは無変更で pass |

## スコープ（本 PR）

- `ruviform-ui`: `LabelSpec` / `TableSpec` / `CellValue` / `A11yNode`、`A11yRole` の 5 variant 追加、mount 時の検証、`ViewHost::a11y_node`。
- `ruviform-ui-gpui`: `label_element` / `table_element`、純関数 `table_plan`、gpui-test 3 件。
- `ruviform-ui-test`: parity fixture `driver-lab/parity/label-table.json`、`driver_parity.rs` に 3 件、`cases.json` の UI-DVR-02 binding（Fake・GPUI）。
- 文書 chain（spec → design → plan → task → test-spec）。

## 非スコープ（未達として明示）

- platform a11y API（OS の accessibility tree）への接続。renderer は a11y 情報を描画に使っておらず（現状 `A11ySpec` は contract の semantic として保持されるのみ）、実 OS の a11y 検証は別 Gate（`ruviform-ui-test/README.md` の a11y 区分）。
- ソート・選択・行クリック・編集などの interaction。本 kind は読み取り専用で、gesture/intent を発行しない。
- 列幅・配置・書式（locale・単位・桁区切り）の policy。数値の表示文字列は `CellValue` の `Display`（f64 の既定形式）を唯一の表現とし、必要なら consumer が文字列で渡す。
- AudioLens 側の Job/Run/Analysis の ViewSpec 化（consumer 側作業で、本 Issue の kind が入った後の別 PR）。
- 仮想化・大量行のスクロール。上限は `MAX_TABLE_ROWS` / `MAX_TABLE_COLUMNS` で固定し、超過は mount 時に拒否する。

## 不変条件

- 共通層（`ruviform-ui`）に GPUI 型を出さない。`check-ui-driver-boundary` が pass すること。
- 公開 enum は `#[non_exhaustive]`（`CellValue`・`A11yRole` を含む）、MSRV 1.89、workspace lint（`clippy -- -D warnings`）準拠。
- Label/Table は非 control（`ControlSpec` を持たない）。`dispatch_input` は no-op で、intent を発行しない。
- 表の行は必ず列数と同じセル数を持つ。欠損は `None`（JSON `null`）で位置を保持し、列はずれない。
