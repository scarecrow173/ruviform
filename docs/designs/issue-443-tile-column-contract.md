---
id: "DESIGN-Vc3Nh8PtRq5LkXw2MdBy7Fj"
type: "design"
status: "proposed"
title: "Issue #443 — Tile 列区間と count の上流契約の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/443"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-viz/src/spectrogram.rs"
      - type: "file"
        path: "crates/ruviform-viz/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-viz/README.md"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-443-tile-column-contract.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-434-spectrogram-kind.md"
  related:
    - "crates/ruviform-viz/src/spectrogram.rs"
---

# Issue #443 — Tile 列区間と count の上流契約の設計

## 概要

#434 の payload `SpectrogramColumn { start, end, count, magnitudes }` へ `Tile` を写す規則を、consumer（AudioLens）から `ruviform-viz` へ移す。決めることは次の 4 点である。

1. 列区間の規則をどこで定義し、どの形で公開するか（D1）。
2. `count` が何を数えるか（D2）。
3. 割り切れない span と `cols > span` のときの列区間（D3）。
4. `ruviform-ui` の契約をどこまで変えるか（D4）。

## 現状の根拠

- `crates/ruviform-viz/src/spectrogram.rs` の `Tile { frames, cols, rows, values }` は列区間も count も持たない。`values` は row-major（`rows * cols`、row 0 = lowest Hz）。
- 列区間は `Spectrogram::tile` 内の式でのみ定まり、公開されていなかった。
- `Tile` の各セルは `Aggregate::Max` / `Mean` で集約済みで、元の有限 sample 数は保持していない。
- `ruviform-ui` の `SpectrogramColumn::count` は「有限 sample 数」と書かれていたが、Tile から導けるのは出力セルの有限数だけである。

## D1: 列区間規則は単一の private 関数で定義し、`Tile::columns` で公開する

- 規則は `column_offsets(span, cols, c) -> Range<u64>`（frame offset、`span = max(len, 1)`）の 1 関数に置く。`cf0 = span·c/cols`、`cf1 = max(span·(c+1)/cols, cf0+1)`。現行 `tile` の式をそのまま移す。
- `Spectrogram::tile` と `Tile::columns` の両方がこの関数を使う。consumer の複製は不要になる。
- 公開形は `Tile::columns(&self) -> impl Iterator<Item = TileColumn>`。列ごとに `frames`（絶対 frame の `Range<u64>`）・`count`・`magnitudes`（`rows` 個）を返す。
- 列区間関数を個別に公開する案は採らない。consumer は列の値も必要とし、区間だけ切り出すと列 → 値の添字計算が再び consumer に残るためである（YAGNI）。

## D2: count は「有限セル数」（`0..=rows`）とする

決定: `TileColumn::count` = その列の `magnitudes` のうち有限（非 NaN）の数。`0` は gap 列。`ruviform-ui` の `SpectrogramColumn::count` の説明もこの意味に合わせる。

根拠:
- Tile の値は集約済みで、元の有限 sample 数は Tile から復元できない。`count` を sample 数で定義すると、Tile からは導けない量を consumer が別途持つことになる。
- 有限セル数は `Tile::values` だけから導ける。Issue #443 の検証条件「NaN のみの列が count 0 として識別できること」を満たす。
- gap の意味論（`count == 0` ⇒ 何も覆わない、NaN ⇒ 欠測セル）が `WaveformBin` / `MeterBin` と揃う。

退けた案:
- (a) `Tile` に列ごとの有限 sample 数を追加する。`Tile` の公開フィールド変更と、`Aggregate` 別の集計経路の追加が必要になり、RowSpec 依存の量を Tile に持ち込むことになるため採らない。
- (b) 「有限 sample 数」のまま doc を残す。consumer が導けない量を契約に書くことになるため採らない。

## D3: 列区間は隙間なく frame 範囲を覆う。`cols <= span` では分割、`cols > span` では重なりを許す

- `cols <= frames.len()` のとき、列区間は `Tile::frames` を重なりなく分割する（割り切れない span を含む。例: 10 frame / 3 列 → `0..3, 3..6, 6..10`）。
- `cols > frames.len()` のとき、各列は最低 1 frame を覆うため（`cf1 >= cf0 + 1`）、隣接列が frame を共有する。全体は依然として隙間なく覆う。例: 2 frame / 4 列 → `0..1, 0..1, 1..2, 1..2`。
- 空 tile（`frames` が空）では、全列の frame 範囲は空（`start..start`）になる。`min(end)` でクランプし、範囲外を指さないようにする。
- 現行 `tile` の挙動（集約・クランプ）は変えない。`cols > span` の重なりも現行の `tile` が既に持つ性質であり、規則を変えていない。

## D4: `ruviform-ui` は doc のみ修正する

- `SpectrogramColumn::count` の doc を「有限セル数（`0..=rows`、0 = gap 列）。sample 数ではなく出力セル数で、consumer は `TileColumn::count` から導く」に修正する。
- 型・フィールド・validation は変えない。`count <= rows` の validation は追加しない（受理条件の変更になるため、follow-up）。
- `ruviform-ui` は `ruviform-viz` に依存しない。doc の参照は文字列だけに留める。

## 依存方向・API 面

- 依存方向は不変。`ruviform-viz` は `ruviform-ui` に依存せず、`ruviform-ui` も `ruviform-viz` に依存しない。
- 追加する公開 API: `TileColumn`（struct、フィールド pub）、`Tile::columns`。公開 enum は追加しない。`TileColumn` の non_exhaustive は付けない（既存の `Tile` と同じく pub フィールドの struct である）。
- lib.rs の export に `TileColumn` を追加する。

## 既存挙動への影響

- `Spectrogram::tile` の出力は変わらない（同じ式を共有関数へ移すのみ）。既存テスト `tile_aggregates_frames_and_bins` / `tile_clamps_and_marks_empty` / `stft_*` が無変更で pass する。
- `encode_tile` は `Tile` を読むのみで影響なし。

## 既知の未解決点（本 PR の範囲外）

- 空 frame 範囲（`frames` が空）かつ bins が非空の `tile` 呼出しで、`mag` の添字参照が範囲外になりうる（`read_tile` が空の `mag` を返すため）。既存挙動であり、本 PR では変更しない。別途報告する。
- AudioLens 側 `spectrogram_columns` の複製除去は consumer の後続作業。
