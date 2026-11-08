---
id: "TASK-CgXiI8C68Y81CRHocDi0QY"
type: "task"
status: "in-progress"
title: "Issue #453 — 残 canvas のサイズ規則監査と是正の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/453"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-453-canvas-size-audit.md"
      - type: "file"
        path: "docs/designs/issue-453-canvas-size-audit.md"
      - type: "file"
        path: "docs/test-specs/issue-453-canvas-size-audit.md"
      - type: "file"
        path: "docs/plans/issue-453-canvas-size-audit.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-453-canvas-size-audit.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-453-canvas-size-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Issue #453 — 残 canvas のサイズ規則監査と是正の実装タスク

## 実装

- [x] T1: 全 paint canvas の prepaint で bounds を記録する（`record_bounds`）。control_element の既存の記録を置き換える。
- [x] T2: 監査テストを追加し、修正前に Waveform（`1920 × 0`）・Meter（`1920 × 0`）・Spectrogram（`1920 × 0`）・TimeAxis（`1920 × 18`）の失敗を確認する。
- [x] T3: Waveform / Meter / Spectrogram の slot を `viz_slot`（flex column、`flex_grow`、`min_h(120)`、canvas `size_full`）に統一する。
- [x] T4: Waveform の `element_for` 分岐の余分な wrapper を外す。
- [x] T5: TimeAxis slot に `flex_shrink_0` を付ける。`ruler` の 24 px 検証が、外すと失敗し、付けると通過することを確認する。
- [x] T6: NativeSlot の canvas bounds 非 0 の検証を既存テストに追加する。

## 検証

- [x] V1: `mise run fmt-check` が通過する（exit 0）。
- [x] V2: `mise run lint-ui-gpui` が通過する（`-D warnings`、exit 0）。
- [x] V3: `mise run test-ui-gpui` が通過する（57 件、exit 0）。
- [x] V4: `mise run test-ui-consumer-observation` が通過する（9 件、exit 0）。
- [x] V5: `mise run test-ui-parity` / `test-ui-driver-contract` / `test-ui-component-view` / `check-ui-driver-boundary` が通過する（exit 0）。
- [x] V6: `mise run check` / `lint` / `test` / `build` が通過する（ワークスペース全体、main 取り込み後に exit 0）。
- [x] V7: `component_showcase` の実 GPUI 窓で修正前後を撮影した。修正前は Waveform が空白、修正後は bins が表示された。
- [ ] V8: `synvelis-native` 実窓の確認は NotRun（consumer 環境が必要）。PR 本文に明記する。

## 文書とコミット

- [x] D1: spec / design / test-spec / plan / task を作成する。
- [x] D2: review-protocol のレビューを実施し、blocker が残らないことを確認する（1 サイクル、判定は ACCEPT WITH NON-BLOCKING NOTES）。
- [x] D3: `doc_maintenance` の plan/apply で managed index を再生成し、doc_status lint で本件の文書に指摘がないことを確認する。
- [x] D4: `git fetch origin main && git merge origin/main` を実行する（main は RF#456 の `a5b1db13` まで進んでおり、衝突なく取り込んだ。取り込み後に index を再生成した）。
- [ ] D5: 論理単位でコミットし、`feature/453-canvas-size-audit` を push する。PR を作る（`Refs #453`）。

## 残課題（本 Issue の範囲外）

- Fader / Knob の `h_full` による兄弟の押し出し（設計書の残課題）。
- Synvelis 実窓の確認（NotRun）。
