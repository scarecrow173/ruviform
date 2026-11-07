---
id: "SPEC-Vn3Qa7Lk2RwXe9TdYc5MbHg"
type: "spec"
status: "proposed"
title: "Issue #434 — Spectrogram ViewSpec kind（解決済み bounded payload）"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/434"
    - "https://github.com/scarecrow173/audio-lens/pull/166"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-434-spectrogram-kind.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
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
    - "crates/ruviform-viz/src/spectrogram.rs"
---

# Issue #434 — Spectrogram ViewSpec kind（解決済み bounded payload）

## 背景

AudioLens は Waveform / Meter / TimeAxis / Seek を driver-neutral な ViewSpec へ 1 対 1 で射影している。Spectrogram の表示 payload は `ruviform-viz` の `Tile` / `TileRequest` が正本だが、`ComponentSpec`（`ruviform-ui`）に Spectrogram kind が無いため、AudioLens は Spectrogram を射影できない（AudioLens 側は保留中）。

## 目的

- Waveform と同様に、**解決済みの bounded payload**（時間 × 周波数の bin 値）を持つ `Spectrogram` kind を `ComponentSpec` に追加する。
- `count == 0`（gap 列）と `NaN`（gap セル）を payload 上で保持する。
- 数値は consumer 側で解決済みのものを写すだけとし、contract 側で再計算しない（bounded query / STFT / LOD は `ruviform-viz` の責務のまま）。
- Fake（`ViewHost` projection）と GPUI renderer の両方で、同一 fixture を評価できること。

## 受入条件（Issue #434 本文の検証条件に対応）

| # | Issue #434 検証条件 | 本 PR での扱い |
| --- | --- | --- |
| A1 | parity fixture で Fake / GPUI の mount key と projection が一致する | Fake: `parity_spectrogram_payload_retained_fake`（mount key・shape・gap 列）。GPUI: `parity_new_kinds_plans_match_shared_fixture` が `expect.spectrogram.drawnCells` と一致検査 |
| A2 | NaN gap と count 0 が保持される単体テストがある | `spectrogram_keeps_gap_cells_and_rejects_unbounded_payloads`（`ruviform-ui`、NaN セルと count==0 列の保持）、`spectrogram_plan_skips_gaps_and_clamps`（GPUI plan 側で gap を描画しない） |
| A3 | 既存の Waveform / Meter / TimeAxis の挙動に変化が無い | 既存テスト（`component_view` 17 件、`driver_parity` 既存 3 件、`ruviform-ui` lib）が全件 pass。既存 variant・描画経路は変更しない |

## スコープ（本 PR）

- `ruviform-ui`: `SpectrogramColumn` / `SpectrogramSpec` / `MAX_SPECTROGRAM_CELLS`、`ComponentSpec::Spectrogram`、validation、単体テスト。
- `ruviform-ui-gpui`: `spectrogram_element`（canvas）、純関数 `spectrogram_cell_plan`、gpui-test。
- `ruviform-ui-test`: parity fixture への Spectrogram 追加、Fake 側 parity テスト。
- `mise.toml` / `docs/development-environment.md`: `test-ui-parity` の説明を Spectrogram を含む表現へ更新。
- 文書 chain（spec → design → plan → task → test-spec）。

## 非スコープ（未達・follow-up として明示）

- AudioLens 側の Spectrogram 射影（audio-lens#166 の残課題）。本 PR は ruviform 側の kind 追加までとし、AudioLens は NotRun。
- `ruviform-viz` の `Tile` から `SpectrogramSpec` への変換（consumer 側の写し替え）。`ruviform-ui` は `ruviform-viz` に依存しない。
- Hz 軸・時間軸の tick 表示（`TimeAxis` との組み合わせで consumer が解決する）。
- 色パレット（専用 theme token）。`ruviform-ui-theme` は本 PR で変更しないため、既存 `viz.waveform` の alpha で強度を表す。
- 非有限値（NaN）の JSON 表現。serde_json は NaN を `null` に直列化し f64 へ戻せないため、JSON fixture では gap を `count == 0` で表し、NaN は Rust 単体テストで検証する（既存 `WaveformBin` / `MeterBin` と同じ制約）。
- `driver-lab/cases.json` の manifest status 変更（Spectrogram は既存 UI-DVR ケースの範囲外）。

## 不変条件

- 共通層（`ruviform-ui` / `ruviform-ui-theme`）に GPUI 型を出さない。`check-ui-driver-boundary` が pass すること。
- 公開 enum は `#[non_exhaustive]`（既存 `ComponentSpec` の属性を踏襲し、新 variant を追加）、MSRV 1.89、workspace lint（`-D warnings`）準拠。
- `ruviform-ui` は `ruviform-viz` に依存しない（依存方向は不変）。
- 既存 kind の挙動を変えない。
