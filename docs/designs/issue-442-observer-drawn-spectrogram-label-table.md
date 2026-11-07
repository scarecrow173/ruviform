---
id: "DESIGN-Jq3Lw8Yn5RtBe2Fk7MzUc4"
type: "design"
status: "proposed"
title: "Issue #442 — GpuiObserver 描画 plan への Spectrogram / Label / Table 追加の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/442"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-442-observer-drawn-spectrogram-label-table.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-442-observer-drawn-spectrogram-label-table.md"
  depends-on:
    - "docs/designs/issue-436-observer-drawn.md"
    - "docs/specs/issue-434-spectrogram-kind.md"
    - "docs/specs/issue-435-label-table-kinds.md"
  related:
    - "docs/specs/issue-442-observer-drawn-spectrogram-label-table.md"
---

# Issue #442 — GpuiObserver 描画 plan への Spectrogram / Label / Table 追加の設計

## 概要

#436 の `drawn` 経路（`render::drawn_plans` → `DrawPlan` → observe の DTO）に、Spectrogram・Label・Table の 3 kind を追加する。既存の経路を広げるだけで、新しい走査や新しい計算は作らない。

## D1: 描画 plan の所有者は render.rs のまま（#436 D1 を踏襲）

- Spectrogram の cell 計算は `spectrogram_cell_plan`、Table の grid 計算は `table_plan` を使う。paint 経路（`spectrogram_element` / `table_element`）も同じ関数を呼ぶため、観測値は描画値と同一の関数出力になる。
- `drawn_plans` の走査規則は変えない。`collect_drawn` に 3 arm を足すだけである。
- `DrawPlan` に `Spectrogram(Vec<SpectrogramCellPlan>)` / `Label(String)` / `Table(TablePlan)` を追加する。

Label に純関数を新設しない理由: 現状の `label_element` は `spec.text.clone()` を直接描画する。値を取り出すだけの関数を新設しても計算は増えず、二重実装のリスクもないため、YAGNI に従い `spec.text` を観測する。将来 Label の描画に変換（翻訳・省略など）が入る場合は、その時点で純関数へ切り出し、plan もそれを使うように改める。

## D2: 公開 DTO は observe.rs に置く（契約型と std 型のみ）

```text
DrawnPlan                                                      // #[non_exhaustive] enum（既存）
  Spectrogram { cells: Vec<DrawnCell> }                        // 追加
  Label { text: String }                                       // 追加
  Table { header: Vec<String>, rows: Vec<Vec<Option<String>>> }  // 追加
DrawnCell { x0: f64, x1: f64, y0: f64, y1: f64, intensity: f64 }  // #[non_exhaustive]、追加
```

- 数値は `f64`。内部の `f32` からの拡張は無損失（`f64::from`）。
- `DrawnCell` は #436 の `DrawnBar` / `DrawnMark` と同じ命名・配置・属性に揃える。
- `Table` の rows は内部の `Option<String>` をそのまま写す。欠損セルは `None` で宣言位置に残る。内部表現（`CellValue`）は公開しない。
- `DrawnPlan` は既に `#[non_exhaustive]` のため、variant 追加は既存 consumer の match を壊さない。

検討して退けた案:

| 案 | 退けた理由 |
| --- | --- |
| `DrawnCell` を `DrawnBar` 系と統合する | 軸の意味（x は viewport 比率、y は高さ比率・top-down）と intensity の有無が異なり、同一型にすると契約が曖昧になる |
| Table を文字列化せず `CellValue` のまま公開する | `CellValue` は `ruviform-ui` の契約型で、表示文字列の規則（`to_string`）が観測側の責務になる。paint と同じ `table_plan` の出力を観測するという #442 の要件に反する |
| Spectrogram の cell を column 単位でネストする | paint は flat な cell 列を描く。観測も同じ単位にしないと fixture の `drawnCells`（flat）と直接照合できない |

## D3: 走査は paint と同じ tree 規則に従う

`collect_drawn` は `element_for` と同じく Panel は全子、Tabs は active の子のみを辿る。Spectrogram / Label / Table はこの規則の上で plan を出す。`_ => {}`（Knob・Toggle・Fader 等）は変えない。

## D4: 取得 API は #436 の `GpuiObserver::drawn` を流用する

API の形（`drawn(&self, window) -> Option<ViewDrawn>`）と内部経路（`view.read(app).host().spec()` → `render::drawn_plans`）は変えない。増えるのは DTO の variant と走査の対象 kind のみである。

## D5: 公開面と export

- `lib.rs` の `observe` 再公開に `DrawnCell` を追加する。他の DTO と同様、consumer が型名を参照できるようにするため。
- feature は追加しない（`gpui-test`）。

## D6: 境界と非 RT

- 新しい DTO は `String` / `Vec` / `Option` / `f64` / `usize` のみで、`gui-boundary-guard` の signature 検査で gpui 型は検出されない。
- 非 RT（UI thread）。新規 allocation は観測時（`drawn` 呼び出し）のみで、paint 経路は従来どおり plan を事前計算する。

## 影響範囲

- 公開 API: `DrawnPlan` への variant 追加（`non_exhaustive` のため非破壊）と `DrawnCell` の追加。既存 API の変更なし。
- paint 経路: 変更なし（`spectrogram_cell_plan` / `table_plan` / `spec.text` の読み方は同一）。
- 既存の内部 hook（`settle_for_test` / `inbox_len_for_test` / `view_for_test`）は変更なし。

## 限界

- 観測できるのは Spectrogram・Label・Table の plan 値のみ。pixel 出力は観測しない（実機描画は manual verification）。
- Label は純関数を経由しない。paint が `spec.text` を直接読む現状に依存するため、paint 側に変換が入ったら本設計 D1 の前提を見直す。
- JSON fixture は NaN を表現できない。非有限 magnitude の除外は crate 内 unit test（`spectrogram_plan_skips_gaps_and_clamps`）で検証する。

## 検証

| 観点 | 検証 | 実行方法 |
| --- | --- | --- |
| Spectrogram cells の一致（gap 除外・clamp） | `observes_drawn_spectrogram_cells_through_seam` | `mise run test-ui-consumer-observation` |
| Label text の一致 | `observes_drawn_label_and_table_plans_through_seam` | 同上 |
| Table header / rows の一致（欠損位置） | 同上 | 同上 |
| drawn の component 一覧（paint 順） | `observes_drawn_meter_and_axis_plans_through_seam` | 同上 |
| NaN の除外・clamp | `spectrogram_plan_skips_gaps_and_clamps` / `parity_label_table_plans_match_shared_fixture`（既存） | `mise run test-ui-gpui` |
| 公開面の gpui 非露出 | `gui-boundary-guard` | `mise run check-ui-driver-boundary` |
