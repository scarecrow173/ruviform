---
id: "TASK-Hk8LsWq3RnVc6YbTd1PeMz"
type: "task"
status: "in-progress"
title: "Issue #429 — Consumer 観測 seam の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/429"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-429-consumer-observation-seam.md"
      - type: "file"
        path: "docs/designs/issue-429-consumer-observation-seam.md"
      - type: "file"
        path: "docs/plans/issue-429-consumer-observation-seam.md"
      - type: "file"
        path: "docs/test-specs/issue-429-consumer-observation-seam.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lifecycle_fixture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-429-consumer-observation-seam.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Issue #429 — 実装タスク

## 完了済み（本 PR）

- [x] T1: raw parent 捕捉の gate を `feature = "gpui-test"` に揃える（外部 crate test の panic を解消）
- [x] T2: 通常ビルドに影響しない形で、mount 済み view の参照を window slot に保持する（`set_view`）
- [x] T2a: `RvfView::host()`（読み取り専用）と `input` の `pub(crate)` 化
- [x] T3: `GpuiObserver`（`observe` / `intents` / `dispatch_input` / `settle` / `inbox_len` / `driver` / `driver_mut`）の実装
- [x] T4: `lifecycle_fixture` を `GpuiObserver` 経由に移し、`settle_for_test` / `inbox_len_for_test` を撤去
- [x] T5: 外部 crate test `tests/consumer_observation.rs`（4 件）と `[[test]]` 登録
- [x] T7: `mise.toml` に `test-ui-consumer-observation` を新設し、検証表に追記
- [x] 文書 chain（spec / design / plan / task / test-spec）

## 未完了（Issue #429 の残 DoD。本 PR では未達として報告）

- [ ] `NumericInput` / `Transport` Control（#387 Non-goals。Consumer 必要性の再評価が前提）
- [ ] Waveform/Meter/Spectrum の長尺・RT snapshot・gap/NaN precision・bounded memory/cancel/stale・GPU release の GPUI 検証
- [ ] driver-lab manifest 残 lane: UI-DVR-03 GPUI の Cancel・遅延/順序逆転、UI-DVR-07/08 GPUI
- [ ] `LAB-SV-FADER-DOCK` / `LAB-AL-WAVEFORM-SEEK` の 2 scale factor × 2 theme × 状態遷移の GPUI 実行

## 既知の問題（本 PR の範囲外）

- `cargo clippy -p ruviform-ui-gpui --all-targets --features gpui-test -- -D warnings` は、既存の unit test 側に 2 件の違反（`render.rs` の doc comment の backtick 欠落、`rvf_view_native_slot_public_api_cannot_starve_pushes` の行数超過）で失敗する。`lint-ui-gpui` は `gpui` feature のみを対象とするため、mise の検証ゲートでは検出されない。本 PR では触れていない。
