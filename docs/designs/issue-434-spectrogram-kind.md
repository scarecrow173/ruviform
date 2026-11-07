---
id: "DESIGN-Kt8PwRz4LxMc1VqBn6HdSu"
type: "design"
status: "proposed"
title: "Issue #434 — Spectrogram ViewSpec kind の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/434"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-434-spectrogram-kind.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
  related:
    - "crates/ruviform-viz/src/spectrogram.rs"
---

# Issue #434 — Spectrogram ViewSpec kind の設計

## 概要

Waveform / Meter と同じ「解決済み payload を描くだけの kind」を Spectrogram にも与える。決めることは次の 5 点である。

1. payload の形（column-major、gap の表現）。
2. 強度の表示範囲をどこで決めるか。
3. payload 上限。
4. GPUI 描画の規則と色の出し方。
5. JSON / serde 上の制約。

## 参照する既存資産

- `ruviform-viz` の `Tile { frames, cols, rows, values }` と `TileRequest`（`crates/ruviform-viz/src/spectrogram.rs`）。Tile は row-major の `values`（`rows * cols`、row 0 = lowest Hz）を持つが、セル単位の `count` を持たない。
- `WaveformBin` / `MeterBin`（`count == 0` ⇒ 空 bin、非有限統計 ⇒ gap）。

## D1: payload は時間列ごとの column-major とする

```text
SpectrogramSpec { key, viewport, rows, magnitude_range, columns, visual, a11y }
SpectrogramColumn { start, end, count, magnitudes: Vec<f64> }  // len == rows
```

- 時間列 1 本が frame 区間 `[start, end)` と `count` を持つ。`count == 0` の列は gap 列。
- `magnitudes` は周波数行（row 0 = lowest Hz）。非有限値（NaN）はそのセルの gap。
- `Tile` の row-major 配列をそのまま採用しない理由: Tile は per-cell `count` を持たず、gap（`count == 0`）の意味を保持できない。Waveform/Meter と同じ「bin = 区間 + count + 統計値」の形に揃えることで、gap の意味論を 3 kind で一致させる。
- consumer は `Tile` を `SpectrogramColumn` へ **写し替える**（列ごとに区間と count を付ける）。値は再計算しない。写し替え規則は consumer 側の責務であり、本 PR では扱わない。

## D2: 強度の表示範囲は consumer が解決して渡す

- `magnitude_range: Range<f64>` を spec に持たせ、renderer は `(m - start) / (end - start)` を `[0, 1]` にクランプして強度とする。
- contract は dB / linear を規定しない（`MeterSpec` と同じ方針）。どちらのスケールを使うかは consumer が決め、その結果の範囲を渡す。
- 範囲を renderer 側の固定値にすると、同一 payload の表示が renderer ごとに変わるため採らない。

## D3: payload 上限と検証

- `MAX_SPECTROGRAM_CELLS = 65536`（`columns × rows`）。Waveform の `MAX_WAVEFORM_BINS` と同じく、spec を有界に保つ。
- validation（`ViewHost::mount` 時）:
  - `rows >= 1`、かつ全列の `magnitudes.len() == rows`。
  - `columns.len() * rows <= MAX_SPECTROGRAM_CELLS`（`saturating_mul`）。
  - `viewport.end >= viewport.start`。
  - `magnitude_range` は両端が有限かつ `start < end`。
- 非有限の magnitude は validation で拒否しない（gap として受理する。NaN gap の保持が要件のため）。

## D4: GPUI 描画は純関数 plan と canvas に分ける

- `spectrogram_cell_plan(&SpectrogramSpec) -> Vec<SpectrogramCellPlan>` を純関数として置く。
  - gap 列（`count == 0`）と非有限 magnitude のセルは出力しない。
  - viewport と完全に重ならない列は cull する（Meter と同じ判定）。
  - y は top-down の正規化区間。row 0（lowest Hz）は下端に来る。
  - intensity は `[0, 1]` にクランプ。
- canvas の paint closure は plan を消費するだけにする。
- 色: `ruviform-ui-theme` は本 PR で変更しない（theme を触ると影響範囲が広がるため）。既存 `viz_waveform` 色の `Hsla::opacity(intensity)` を背景 `surface_bg` の上に重ねる。専用 palette token は follow-up。

## D5: JSON 上の NaN と fixture の表現

- serde_json は NaN を `null` として直列化し、`null` を f64 へ戻すことはできない。したがって JSON payload では非有限値を表現できない（`WaveformBin` / `MeterBin` と同じ既存制約）。
- 対応: parity fixture では gap を `count == 0` の列で表す。NaN gap は Rust 単体テストで直接検証する（`ViewHost::mount` 後の spec に NaN が保持されること）。
- JSON 上の非有限値表現（例: `null` 許容の専用型）は本 PR の範囲外とし、follow-up として記録する。

## 依存方向

- `ruviform-ui` は `ruviform-viz` に依存しない。`ruviform-viz` の参照は doc のみ。
- `ruviform-ui-gpui` は `ruviform-ui` の spec を読むだけ。共通層に GPUI 型は現れない。

## 検証方針

- headless: `ruviform-ui` の単体テストと `ruviform-ui-test` の `driver_parity`（`mise run test-ui-parity`）。
- GPUI: `spectrogram_cell_plan` の単体テストと fixture の `drawnCells` 一致（`mise run test-ui-gpui`）。
- 境界: `mise run check-ui-driver-boundary`。
- lint・型: `mise run lint-ui-gpui` / `mise run lint`。
