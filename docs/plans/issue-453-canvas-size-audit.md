---
id: "PLAN-yicQRN5SwNiNcaiqCAvmpf"
type: "plan"
status: "in-progress"
title: "Issue #453 — 残 canvas のサイズ規則監査と是正の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/453"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-453-canvas-size-audit.md"
    - "docs/designs/issue-453-canvas-size-audit.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-450-control-canvas-size.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  verified-by:
    - "docs/test-specs/issue-453-canvas-size-audit.md"
  refines: []
  refined-by: []
  derives-from: []
---

# Issue #453 — 残 canvas のサイズ規則監査と是正の実装計画

## 範囲

- 変更対象は `crates/ruviform-ui-gpui/src/render.rs` のみ（canvas 記録の共通化、viz slot、Waveform 分岐、TimeAxis slot、テスト）。
- `ruviform-ui`・`ruviform-ui-test`・consumer 側・`Cargo.lock` は変更しない。
- 文書（spec / design / test-spec / plan / task）と、managed index の再生成のみ docs 側で行う。

## 作業順序

1. 監査: 各 paint canvas の prepaint で bounds を記録する（`record_bounds`）。監査テストを追加し、修正前に失敗させて kind ごとの実測値を得る。
2. 修正: Waveform / Meter / Spectrogram の slot を `viz_slot` に統一する。Waveform の余分な wrapper を外す。TimeAxis slot に `flex_shrink_0` を付ける。
3. 検証: 監査テストが通過すること、TimeAxis の縮小禁止テストが `flex_shrink_0` の有無で赤緑になることを確認する。
4. 実窓: `component_showcase` の修正前後を撮影する（修正前は `render.rs` を HEAD の版に一時的に戻して撮影し、修正版をバックアップから復元して差分が元どおりであることを確認する）。
5. 文書: spec → design → test-spec → plan → task を作成し、`doc_maintenance` の plan/apply で managed index を再生成する。doc_status で本件の文書エラーがないことを確認する。
6. review-protocol のレビュー（blocker が残らないこと）。
7. `git fetch origin main && git merge origin/main`（現時点で origin/main は `e3f9db44` のまま）。
8. 最終検証（fmt-check / lint-ui-gpui / test-ui-gpui / test-ui-consumer-observation / test-ui-parity / test-ui-driver-contract / test-ui-component-view / check-ui-driver-boundary / check / lint / test / build）。
9. 論理単位でコミットし、`feature/453-canvas-size-audit` を push する。PR を作る（`Refs #453`）。

## ファイル割当

| ファイル | 変更 | 目的 |
| --- | --- | --- |
| `crates/ruviform-ui-gpui/src/render.rs` | 変更 | `record_bounds`、`viz_slot`、Waveform 分岐、TimeAxis `flex_shrink_0`、監査テスト 2 件、NativeSlot 監査の 1 行 |
| `docs/specs/issue-453-canvas-size-audit.md` | 追加 | 受入条件と非スコープ |
| `docs/designs/issue-453-canvas-size-audit.md` | 追加 | 監査結果と是正の規則 |
| `docs/test-specs/issue-453-canvas-size-audit.md` | 追加 | 検証契約と NotRun の区別 |
| `docs/plans/issue-453-canvas-size-audit.md` | 追加 | 本書 |
| `docs/tasks/issue-453-canvas-size-audit.md` | 追加 | 実行単位の追跡 |
| `docs/*/README.md` の managed index | 再生成 | doc_maintenance の apply による生成物 |

## リスク

| リスク | 影響 | 対策 |
| --- | --- | --- |
| Panel の溢れで viz slot が縮む | 0 サイズの再発 | `min_h(120)` を slot が持ち、監査テストは溢れる構成（Fader の `h_full` 付き）でマウントする |
| TimeAxis が溢れで縮む | 宣言 24 px と不一致 | `flex_shrink_0` と `ruler` の高さ検証 |
| 実機の描画が gpui-test と異なる | 実窓で見えない可能性 | component_showcase の実窓撮影で確認する |
| `render.rs` を並行 Worker（RF#456, `drain_inbox`/receipt）と競合する | merge 競合 | 競合領域が異なる（canvas 要素 vs drain_inbox）。コミットを分けて merge 時に対処する |

## ロールバック

`render.rs` の差分を `git revert` すれば元に戻る。文書は独立しており、削除してもコードに影響しない。

## 残課題

- Fader / Knob の `h_full` による兄弟の押し出し（設計書の残課題）。
