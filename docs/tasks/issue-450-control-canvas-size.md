---
id: "TASK-Xryam9aweXgtkqZzZupujs"
type: "task"
status: "in-progress"
title: "Issue #450 — Seek/Fader canvas サイズ是正の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/450"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-450-control-canvas-size.md"
      - type: "file"
        path: "docs/designs/issue-450-control-canvas-size.md"
      - type: "file"
        path: "docs/test-specs/issue-450-control-canvas-size.md"
      - type: "file"
        path: "docs/plans/issue-450-control-canvas-size.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-450-control-canvas-size.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-450-control-canvas-size.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Issue #450 — Seek/Fader canvas サイズ是正の実装タスク

## 実装

- [x] T1: 回帰テスト `rvf_view_seek_and_fader_canvas_bounds_nonzero` を追加し、修正前に Seek の `0px × 24px` で失敗することを確認する。
- [x] T2: `control_element` の canvas に `size_full()` を付け、slot と同じ寸法にする。
- [x] T3: 回帰テストが修正後に通過することを確認する（`mise run test-ui-gpui`）。

## 検証

- [x] V1: `mise run fmt-check` が通過する。
- [x] V2: `mise run lint-ui-gpui` が通過する（`-D warnings`）。
- [x] V3: `mise run test-ui-gpui` が通過する（54 件）。
- [x] V4: `mise run test-ui-consumer-observation` が通過する（8 件）。
- [x] V5: `mise run test-ui-parity` / `test-ui-driver-contract` / `test-ui-component-view` / `check-ui-driver-boundary` が通過する。
- [x] V6: `mise run check` / `lint` / `test` / `check-ui-gpui` / `build` が通過する（ワークスペース全体）。
- [x] V7: ruviform `component_showcase` の実 GPUI 窓で Fader と Seek の描画を目視確認した（修正前後の比較を含む）。
- [ ] V8: `synvelis-native` 実窓で Seek を視認するスクリーンショットは NotRun（consumer 側の環境と実機撮影が必要）。PR 本文に明記する。

## 文書とコミット

- [x] D1: spec / design / test-spec / plan / task を作成する。
- [x] D2: review-protocol のレビュー（1 サイクル、blocker なし）。判定は ACCEPT WITH NON-BLOCKING NOTES。
- [x] D3: `doc_maintenance.js apply` で managed index を再生成し、doc_status lint で本件文書のエラーが無いことを確認する。
- [x] D4: `git fetch origin main` を実行し、origin/main が `d3be1ad9` のまま（取り込み不要）であることを確認する。
- [ ] D5: 論理単位でコミットし、`fix/450-gpui-control-canvas-size` を push する。PR を作る。

## 残課題（本 Issue の範囲外）

- Waveform / Meter / TimeAxis / Spectrogram の canvas サイズ規則（設計書の残課題を参照）。
