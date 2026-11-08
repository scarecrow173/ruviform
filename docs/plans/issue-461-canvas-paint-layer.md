---
id: "PLAN-034TXozGzl5Y5HRbsrn4t3"
type: "plan"
status: "in-progress"
title: "Issue #461 — canvas painter paint batch の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/461"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-461-canvas-paint-layer.md"
      - type: "file"
        path: "docs/test-specs/issue-461-canvas-paint-layer.md"
      - type: "file"
        path: "docs/plans/issue-461-canvas-paint-layer.md"
      - type: "file"
        path: "docs/tasks/issue-461-canvas-paint-layer.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-461-canvas-paint-layer.md"
    - "docs/designs/issue-461-canvas-paint-layer.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-453-canvas-size-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/plans/issue-453-canvas-size-audit.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-461-canvas-paint-layer.md"
    - "docs/designs/issue-461-canvas-paint-layer.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-461-canvas-paint-layer.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #461 — canvas painter paint batch の実装計画

## 範囲

- 変更対象は `crates/ruviform-ui-gpui/src/render.rs` のみ（4 つの spec サイズ比例 canvas painter の `paint_layer` 適用と、契約上限 payload の repaint budget 回帰テスト）。
- `ruviform-ui`・`ruviform-ui-test`・consumer 側・`Cargo.lock` は変更しない。公開契約（`*Spec` の形・値域）は不変。
- 文書（spec / design / test-spec / plan / task）と、managed index の再生成のみ docs 側で行う。

## 作業順序

1. 診断: headless `TestAppContext` の実 paint 経路（`Scene` 構築・`BoundsTree::insert` を含む）で、各 spec サイズ比例 canvas painter の契約上限 payload repaint コストを計測し、修正前値を得る。併せて `BoundsTree::insert` → `find_max_ordering` の走査構造と `Window::paint_layer` の order 共有を gpui 0.2.2 ソースで確認する。
2. 修正: Spectrogram / Waveform / Meter / TimeAxis の paint closure で、固定の背景 quad を layer の外に描き、要素ごとの quad 列を `window.paint_layer(bounds, |window| { ... })` で囲む。Fader / Knob / Seek / NativeSlot（quad 数が固定で spec サイズに比例しない）は対象外とする。
3. 検証: 計測を再実行して修正後値を得、監査表を design に記録する。paint 順・clip・1 px clamp が変わらないことを drawn plan / parity テストで確認する。
4. 回帰ゲート: `rvf_view_max_payload_canvas_repaint_budget` を追加し、Spectrogram / Waveform / Meter を契約上限 payload でマウントして repaint が budget（500 ms、修正後実測の約 10 倍）未満であることを検証する。TimeAxis は label `div` コストが支配的で budget の意味を持たないため対象外とし、理由を design に記録する。
5. 文書: spec → design → test-spec → plan → task を作成し、managed index を再生成する。doc_status で本件の文書エラーがないことを確認する。
6. review-protocol のレビュー（blocker が残らないこと）。
7. `git fetch origin main && git merge origin/main` で最新 main を取り込み、生成物（docs index 等）を再生成する。
8. 最終検証（fmt / fmt-check / check-ui-gpui / lint-ui-gpui / test-ui-gpui / test-ui-consumer-observation / test-ui-parity / test-ui-driver-contract / test-ui-component-view / check-ui-driver-boundary / check / lint / test / build）。
9. 論理単位でコミットし、`feature/461-spectrogram-paint-layer` を push する。PR を作る（`Refs #461`）。

## ファイル割当

| ファイル | 変更 | 目的 |
| --- | --- | --- |
| `crates/ruviform-ui-gpui/src/render.rs` | 変更 | 4 painter の `paint_layer` 適用、回帰テスト `rvf_view_max_payload_canvas_repaint_budget` |
| `docs/specs/issue-461-canvas-paint-layer.md` | 追加 | 受入条件と非スコープ |
| `docs/designs/issue-461-canvas-paint-layer.md` | 追加 | 監査結果表・原因の位置・是正の形 |
| `docs/test-specs/issue-461-canvas-paint-layer.md` | 追加 | 検証契約と NotRun の区別 |
| `docs/plans/issue-461-canvas-paint-layer.md` | 追加 | 本書 |
| `docs/tasks/issue-461-canvas-paint-layer.md` | 追加 | 実行単位の追跡 |
| `docs/*/README.md` の managed index | 再生成 | doc tooling による生成物 |

## リスク

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `paint_layer` が layer bounds を `bounds ∩ content_mask` に clip するため、従来 canvas content_mask で clip されていた範囲と差が出る | 描画欠け | `paint_layer` は primitive の content_mask を変更せず、layer bounds は canvas bounds と一致する。各 quad は従来どおり ambient content_mask で clip される。drawn plan テストで確認する |
| layer の order 共有で、cell の 1 px clamp による端の重なり順が変わる | 微細な描画差 | `Scene::finish` の stable sort が同一 order 内の挿入順を維持するため、paint 順どおりに重なる。parity / drawn plan テストで確認する |
| 回帰テストの時間 budget が低速環境で偽陽性を出す | CI 不安定 | budget は修正後実測（最大約 55 ms）の約 10 倍（500 ms）とし、O(n²) 回帰（秒オーダー）との識別を優先する |
| 並行 Worker（`ctx_214a6e834e8f`、AL pin 更新のみ）との競合 | merge 競合 | 当該 Worker は `render.rs` を編集しない。衝突領域なし |

## ロールバック

`render.rs` の差分を `git revert` すれば元に戻る。文書は独立しており、削除してもコードに影響しない。

## 残課題

- TimeAxis の repaint 約 0.8 s は tick ごとの label `div` + text 描画が支配的。canvas quad の問題ではなく element tree 規模の別問題であり、design の残課題に記録した（別 Issue 候補）。
- Table の行×列 `div` 生成も同様に element tree 規模のコストであり、本 Issue の対象外。
- audio-lens 側の pin 更新は本 Issue merge 後に Coordinator が管理する。

## 検証

- [x] 診断計測: headless `TestAppContext` で契約上限 payload の repaint を計測し、修正前値を取得した（Spectrogram 6.37 s / Waveform 45.19 s / Meter 13.06 s / TimeAxis 906 ms、debug build）。
- [x] `mise run fmt-check` が通過する。
- [x] `mise run lint-ui-gpui` が通過する（`-D warnings`）。
- [x] `mise run test-ui-gpui` が通過する（`rvf_view_max_payload_canvas_repaint_budget` を含む）。
- [x] `mise run test-ui-consumer-observation` / `test-ui-parity` / `test-ui-driver-contract` / `test-ui-component-view` / `check-ui-driver-boundary` が通過する。
- [x] `mise run check` / `lint` / `test` / `build` が通過する。
- [ ] 実 GPUI 窓（`component_showcase` 等）での実機 paint 計測は NotRun（headless で `Scene` 構築の全経路を通すため O(n²) 回帰は検知可能。実機の体感確認は consumer 側の pin 更新後に委ねる）。
