---
id: "TASK-034TXoGba4bLlZ1oiI3L2z"
type: "task"
status: "in-progress"
title: "Issue #461 — canvas painter paint batch の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/461"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-461-canvas-paint-layer.md"
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
    - "docs/plans/issue-461-canvas-paint-layer.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-461-canvas-paint-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Issue #461 — canvas painter paint batch の実装タスク

## 実装

- [x] T1: headless `TestAppContext` の実 paint 経路で各 spec サイズ比例 canvas painter の契約上限 payload repaint コストを計測し、修正前値（Spectrogram 6.37 s / Waveform 45.19 s / Meter 13.06 s / TimeAxis 906 ms）を取得する。`BoundsTree::insert` / `find_max_ordering` と `Window::paint_layer` の order 共有を gpui 0.2.2 ソースで確認する。
- [x] T2: Spectrogram の cell 描画を `window.paint_layer(bounds, ...)` で囲む（背景 quad は layer 外）。
- [x] T3: Waveform・Meter・TimeAxis も同様に、要素ごとの quad 列を 1 つの `paint_layer` で囲む（背景 quad は layer 外）。
- [x] T4: Fader / Knob / Seek / NativeSlot は quad 数が固定で spec サイズに比例しないため対象外とすることを監査結果に記録する。
- [x] T5: 修正後の repaint コストを再計測し（Spectrogram 53.3 ms / Waveform 27.3 ms / Meter 11.5 ms / TimeAxis 806 ms）、監査表を design に記録する。TimeAxis は label `div` コスト支配のため budget 対象外とする判断を記録する。
- [x] T6: `rvf_view_max_payload_canvas_repaint_budget` を追加し、Spectrogram / Waveform / Meter を契約上限 payload でマウントして repaint が 500 ms 未満であることを検証する。

## 検証

- [x] V1: `rvf_view_max_payload_canvas_repaint_budget` が通過する（debug、実測 Spectrogram 約 34 ms / Waveform 約 16 ms / Meter 約 8 ms、budget 500 ms）。
- [x] V2: `mise run fmt-check` / `lint-ui-gpui` が通過する。
- [x] V3: `mise run test-ui-gpui` / `test-ui-consumer-observation` / `test-ui-parity` / `test-ui-driver-contract` / `test-ui-component-view` / `check-ui-driver-boundary` が通過する。
- [x] V4: `mise run check` / `lint` / `test` / `build` が通過する。

## 文書とコミット

- [x] D1: spec / design / test-spec / plan / task を作成する。
- [x] D2: review-protocol のレビューを実施し、blocker が残らないことを確認する（1 サイクル、判定は ACCEPT WITH NON-BLOCKING NOTES。非ブロッキング指摘: budget テストは notify→paint の実経路を修正前計測で実証済み、gpui-test feature の clippy 非対象は既存の repo 全体のギャップ、TimeAxis の label div コストは別 Issue 候補として design に記録）。
- [x] D3: managed index を再生成し、doc_status lint で本件の文書に指摘がないことを確認する。
- [x] D4: `git fetch origin main && git merge origin/main` を実行し、取り込み後に index を再生成する（origin/main は `129786ba` のままで既に最新。Already up to date）。
- [ ] D5: 論理単位でコミットし、`feature/461-spectrogram-paint-layer` を push する。PR を作る（`Refs #461`）。

## 残課題（本 Issue の範囲外）

- TimeAxis の repaint 約 0.8 s は tick ごとの label `div` + text 描画が支配的。canvas quad ではなく element tree 規模の別問題であり、別 Issue 候補。
- Table の行×列 `div` 生成も同様に element tree 規模のコストであり、本 Issue の対象外。
- 実 GPUI 窓での実機 paint 計測は NotRun（headless で `Scene` 構築の全経路を通すため O(n²) 回帰は検知可能。実機の体感確認は consumer 側の pin 更新後に委ねる）。
- audio-lens 側の pin 更新は本 Issue merge 後に Coordinator が管理する。
