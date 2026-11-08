---
id: "PLAN-cXVjL8azFccQWa3dVmLi1a"
type: "plan"
status: "in-progress"
title: "Issue #450 — Seek/Fader canvas サイズ是正の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/450"
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
    - "docs/specs/issue-450-control-canvas-size.md"
    - "docs/designs/issue-450-control-canvas-size.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  verified-by:
    - "docs/test-specs/issue-450-control-canvas-size.md"
  refines: []
  refined-by: []
  derives-from: []
---

# Issue #450 — Seek/Fader canvas サイズ是正の実装計画

## 範囲

- 変更対象は `crates/ruviform-ui-gpui/src/render.rs` の `control_element`（canvas 1 箇所）と、そのテストのみ。
- `ruviform-ui`・`driver-lab` fixtures・`Cargo.lock` は変更しない（契約とサイズの責務境界は設計書のとおり）。
- ドキュメント（spec / design / test-spec / plan / task）と、生成 index の再生成のみ docs 側で行う。

## 作業順序

1. 再現: 回帰テスト `rvf_view_seek_and_fader_canvas_bounds_nonzero` を追加し、修正前に失敗することを確認する（Seek が `0px × 24px`）。
2. 修正: `control_element` の canvas に `size_full()` を付け、slot と同じ寸法にする。
3. 検証: 対象 lane（GPUI、consumer-observation、parity、driver-contract、component-view、driver-boundary）と静的検証（fmt-check、lint-ui-gpui）を実行する。
4. 文書: spec → design → test-spec → plan → task の文書チェーンを整え、`doc_maintenance.js plan/apply` で managed index を再生成する。
5. 最新 main の取り込み（`git fetch origin main && git merge origin/main`）後に最終検証を再実行する。
6. コミットを論理単位に分けて push し、PR を作る。

## ファイル割当

| ファイル | 変更 | 目的 |
| --- | --- | --- |
| `crates/ruviform-ui-gpui/src/render.rs` | 変更 | canvas の `size_full()`（修正）と回帰テスト・`seek_spec` helper（テスト） |
| `docs/specs/issue-450-control-canvas-size.md` | 追加 | 受入条件と非スコープ |
| `docs/designs/issue-450-control-canvas-size.md` | 追加 | サイズ決定契約と残課題 |
| `docs/test-specs/issue-450-control-canvas-size.md` | 追加 | 検証契約と NotRun の区別 |
| `docs/plans/issue-450-control-canvas-size.md` | 追加 | 本書 |
| `docs/tasks/issue-450-control-canvas-size.md` | 追加 | 実行単位の追跡 |
| `docs/designs/README.md`・`docs/specs/README.md` 等の managed index | 再生成 | doc_maintenance の `apply` による生成物 |

## リスク

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `size_full` の割合指定が親の定義済み寸法に依存する | 親の高さが不定だと canvas も 0 になる | Seek は `h(24)`、Fader は `w(32)` を wrapper が持つことを回帰テストで確認する |
| 他の canvas（Waveform / Meter / TimeAxis / Spectrogram）の同種の問題 | 本 Issue では未修正 | 設計書の残課題に記録し、計測して別 Issue で扱う |
| 実機の描画結果が headless と異なる | 実窓で依然として見えない可能性 | 実機スクリーンショットは実行可否を明記し、実行できない場合は NotRun とする |

## ロールバック

`render.rs` の `size_full()` 1 行と、追加したテスト・helper を削除すれば元に戻る。文書は独立しており、削除してもコードに影響しない。
