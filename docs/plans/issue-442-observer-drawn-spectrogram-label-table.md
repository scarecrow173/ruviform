---
id: "PLAN-Vn6Pc1Rh4SxDa9Gt3KwLe8"
type: "plan"
status: "proposed"
title: "Issue #442 — 描画 plan への Spectrogram / Label / Table 追加の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/442"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-442-observer-drawn-spectrogram-label-table.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
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
  implements:
    - "docs/designs/issue-442-observer-drawn-spectrogram-label-table.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-442-observer-drawn-spectrogram-label-table.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-442-observer-drawn-spectrogram-label-table.md"
---

# Issue #442 — 描画 plan への Spectrogram / Label / Table 追加の実装計画

## 対象

設計 `docs/designs/issue-442-observer-drawn-spectrogram-label-table.md`（D1〜D6）を実装する。単一 PR・単一 crate の変更であり、分割計画は不要。

## ファイルマップ

| ファイル | 変更内容 | 対応設計 |
| --- | --- | --- |
| `crates/ruviform-ui-gpui/src/render.rs` | `DrawPlan` に Spectrogram / Label / Table を追加、`collect_drawn` に 3 arm、`SpectrogramCellPlan` / `TablePlan` を `pub(crate)` 化、doc 更新 | D1, D3 |
| `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs` | `DrawnPlan` に 3 variant、`DrawnCell` DTO、`DrawPlan` → DTO の写像、doc 更新 | D2 |
| `crates/ruviform-ui-gpui/src/lib.rs` | `observe` 再公開に `DrawnCell` を追加（export 面の最小変更） | D5 |
| `crates/ruviform-ui-gpui/tests/consumer_observation.rs` | Spectrogram cells・Label・Table の fixture 一致テスト、drawn 一覧の拡張、fixture loader の一般化 | AC-1〜AC-4 |
| `mise.toml` | `test-ui-consumer-observation` の description を Spectrogram / Label / Table を含む内容へ更新（task 名・コマンドは不変） | D5 |
| `docs/**` | spec / design / plan / task / test-spec | — |

## 依存順序

1. `consumer_observation.rs`: 期待値テストを先に書く（fixture 一致・drawn 一覧）。
2. `render.rs`: `DrawPlan` の variant と `collect_drawn` の arm、plan 構造体の可視性。
3. `observe.rs`: DTO と写像、`lib.rs` の export。
4. `mise.toml` の description 更新。
5. 検証（下記）。

## リスク

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `lib.rs` の export 面が RF388 dock PoC と競合 | rebase 時の衝突 | export 行の追加のみ。競合時は main を取り込んで解消 |
| 既存テスト `observes_drawn_meter_and_axis_plans_through_seam` の drawn 一覧が変わる | 既存テストの期待値の更新が必要 | parity fixture の `spectrogram` が paint 順で現れるため `[meter, ruler, spectrogram]` へ更新（意図された拡張） |
| Table の `CellValue` → 文字列の規則が観測側で別実装になる | paint と観測の不一致 | 観測は `table_plan` の出力（`TablePlan`）をそのまま写す。変換規則は `table_plan` のみ |
| Label の観測が paint と乖離する | 表示テキストの不一致 | paint と同じ `spec.text` を読む（設計 D1）。consumer test で fixture の `label.text` と照合 |
| `DrawnPlan` の variant 追加が既存 consumer を壊す | 下流 build 失敗 | `DrawnPlan` は既に `#[non_exhaustive]`。破壊的ではない |
| DTO が gpui 型を含む | boundary guard 失敗 | DTO は `f64` / `usize` / `String` / `Vec` / `Option` / `Range<u64>` / 契約型のみ |

## ロールバック

変更は追加的である。`DrawnPlan` の 3 variant・`DrawnCell`・`collect_drawn` の 3 arm を削除し、`SpectrogramCellPlan` / `TablePlan` の可視性を元に戻せば完全に元へ戻る。永続データ形式・lockfile の変更はない。

## 検証マトリクス

| 段階 | コマンド | 目的 |
| --- | --- | --- |
| 1 | `mise run test-ui-consumer-observation` | AC-1〜AC-4 |
| 2 | `mise run test-ui-gpui` | AC-5・AC-7（既存 render unit test の回帰） |
| 3 | `mise run check-ui-driver-boundary` | AC-6 |
| 4 | `mise run lint-ui-gpui` / `mise run check-ui-gpui` / `mise run fmt-check` | 警告ゼロ・整形 |
| 5 | `mise run check` / `mise run lint` / `mise run test` | ワークスペース全体 |
| 6 | `mise run test-ui-parity` / `mise run test-ui-driver-contract` / `mise run test-ui-headless` / `mise run test-ui-component-view` | 周辺 UI lane の回帰 |
| 7 | `mise run test-ui-test-msrv` | MSRV 検証（実行可否は環境次第。未実行なら理由を明記） |
