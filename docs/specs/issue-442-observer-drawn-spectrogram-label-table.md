---
id: "SPEC-Hs7Kd2Mv9QpXc4Nw1ZrTb6"
type: "spec"
status: "implemented"
title: "Issue #442 — GpuiObserver の描画 plan に Spectrogram / Label / Table を追加"
created: "2026-11-08"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/442"
    - "https://github.com/scarecrow173/ruviform/issues/436"
    - "https://github.com/scarecrow173/ruviform/issues/434"
    - "https://github.com/scarecrow173/ruviform/issues/435"
    - "https://github.com/scarecrow173/audio-lens/pull/166"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-442-observer-drawn-spectrogram-label-table.md"
      - type: "file"
        path: "docs/designs/issue-442-observer-drawn-spectrogram-label-table.md"
      - type: "file"
        path: "docs/plans/issue-442-observer-drawn-spectrogram-label-table.md"
      - type: "file"
        path: "docs/tasks/issue-442-observer-drawn-spectrogram-label-table.md"
      - type: "file"
        path: "docs/test-specs/issue-442-observer-drawn-spectrogram-label-table.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-436-observer-drawn.md"
    - "docs/designs/issue-436-observer-drawn.md"
    - "docs/specs/issue-434-spectrogram-kind.md"
    - "docs/specs/issue-435-label-table-kinds.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-421-meter-timeaxis-gpui-parity.md"
  refines:
    - "docs/specs/issue-436-observer-drawn.md"
---

# Issue #442 — GpuiObserver の描画 plan に Spectrogram / Label / Table を追加

## 背景

#436 で `GpuiObserver::drawn` が公開されたが、描画 plan を持つのは Meter・TimeAxis・Waveform のみである。`DrawnPlan` の 3 variant に加え、`render.rs` の `collect_drawn` は Spectrogram・Label・Table を `_ => {}` で読み飛ばす。

そのため、#434 で追加された Spectrogram（`spectrogram_cell_plan`）と #435 で追加された Label / Table（`table_plan`）の描画結果は、consumer 側から観測できない。描画 plan の純関数は crate 内テストでのみ検査されている。

AudioLens（audio-lens#166 AL-G4）の GPUI parity test は、Spectrogram の gap 列・Table の欠損セル位置を consumer 側から照合する必要があるが、現時点では NotRun のまま残っている。

## 問題

- `DrawnPlan` に Spectrogram / Label / Table の variant がなく、`GpuiObserver::drawn` は該当 component を返さない。
- Spectrogram / Table の描画値は paint と同じ純関数（`spectrogram_cell_plan` / `table_plan`）から得られるが、その結果は crate 外から見えない。
- Label の描画値（表示テキスト）も観測手段がない。

## 目的

- `DrawnPlan` に Spectrogram のセル plan、Label のテキスト、Table の header と rows を追加する。
- `GpuiObserver::drawn` で上記を consumer から観測できる。
- 観測値は paint と同じ関数（または同じ field）から得る。観測と描画の二重実装を作らない。
- 公開 DTO に GPUI 型を含めない（#436 の方針どおり）。

## 要件

- R1: `DrawnPlan` に `Spectrogram { cells: Vec<DrawnCell> }` を追加する。`DrawnCell` は `x0` / `x1`（viewport 幅に対する比率）、`y0` / `y1`（spectrogram 高さに対する上から下への比率）、`intensity`（`[0, 1]`）を持つ。
- R2: Spectrogram の cells は `spectrogram_cell_plan` の出力そのものを、その順序（column 順、次に row 順）で返す。gap 列（`count == 0`）・非有限 magnitude・viewport 外の列は含めない。
- R3: `DrawnPlan` に `Label { text: String }` を追加する。値は `label_element` が描画する `spec.text` と同じ。
- R4: `DrawnPlan` に `Table { header: Vec<String>, rows: Vec<Vec<Option<String>>> }` を追加する。値は `table_plan` の出力そのもの。欠損セルは `None` として宣言位置に残す。
- R5: `drawn` が返す components は paint 順の tree 走査（Panel は全子、Tabs は active の子のみ）に従い、Meter・TimeAxis・Waveform・Spectrogram・Label・Table のみを含む。Knob・Toggle などは含めない。
- R6: DTO は `f64` / `usize` / `String` / `Option` / `Vec` / `Range<u64>` / 契約型（`ComponentKey` / `WindowId`）のみで構成し、`gpui` 型を含めない。
- R7: 既存の描画経路（paint の見た目・順序・計算）、既存の内部 hook、既存の DTO（`DrawnBar` / `DrawnMark` / `DrawnWaveformChannel` 等）の形を変えない。
- R8: `DrawnPlan` は既存と同じく `#[non_exhaustive]` を維持する。新 variant の追加は既存 consumer を壊さない。

## 受入条件

| ID | 条件 | 検証 |
| --- | --- | --- |
| AC-1 | `parity/new-kinds.json` の `expect.spectrogram.drawnCells`（4 cell、gap 列を除く）と consumer 側の観測が件数・順序・各値（許容誤差 `1e-3`）で一致する | `observes_drawn_spectrogram_cells_through_seam` |
| AC-2 | `parity/label-table.json` の `expect.label.text` と Label の観測 text が一致する | `observes_drawn_label_and_table_plans_through_seam` |
| AC-3 | `parity/label-table.json` の `expect.table.drawnHeader` / `drawnRows` と Table の観測 header / rows が一致し、欠損セル（`None`）の位置が保たれる | 同上 |
| AC-4 | `new-kinds.json` の `drawn` の component 一覧が paint 順 `[meter, ruler, spectrogram]` で、toggle / knob / root を含まない | `observes_drawn_meter_and_axis_plans_through_seam`（拡張） |
| AC-5 | NaN magnitude・NaN peak 等の非有限値は描画 plan に現れない（JSON fixture では表現できないため crate 内で検証） | `spectrogram_plan_skips_gaps_and_clamps`（既存、回帰確認） |
| AC-6 | 公開 signature に `gpui` 型が現れない | `mise run check-ui-driver-boundary` |
| AC-7 | 描画挙動に変化がない（既存の render unit test・parity test が通る） | `mise run test-ui-gpui` / `mise run test-ui-parity` |

## 範囲外

- Fake 層（`ruviform-ui-test`）側の drawn 検査の追加。
- AudioLens 側の `gpui_parity.rs` の実装（consumer 側の作業。本件はその前提となる観測 seam のみ提供する）。
- Knob / Toggle / Fader など、本 Issue で列挙されていない描画 primitive の観測。
- Spectrogram の振幅色（alpha 以外の色）、Label / Table の pixel 出力の観測。実機描画は manual verification。
- Label の描画経路を純関数へ切り出すリファクタリング（現状の paint は `spec.text` を直接読むため、同じ field を観測すれば一致は構造的に保たれる）。
- `ruviform-ui` の契約変更（`crates/ruviform-ui/**` は read-only）。

## 制約

- 非 RT（UI thread のみ）。
- feature は既存の `gpui-test` を用いる。新 feature は追加しない。
- MSRV 1.89、`workspace.lints` 準拠、public enum は `#[non_exhaustive]`。
- `crates/ruviform-ui-gpui/src/lib.rs` の export 面は追加のみとし、最小に保つ（RF388 dock PoC と競合しうる）。
