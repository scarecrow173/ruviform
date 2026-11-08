---
id: "TASK-Kd2Pw7YvNs4QbTm9RxLe6J"
type: "task"
status: "in-progress"
title: "Issue #448 — 行選択と command 起動の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/448"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-448-table-selection-command.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Issue #448 — 実装タスク

## 完了

- [x] T1 `CommandIntent::Action` に `params` を追加（`behavior.rs`）。
- [x] T2 `TableSelectionSpec` / `TableSpec.selection`、`ButtonSpec` / `ComponentSpec::Button`、`A11yRole::Button`、`TABLE_SELECT_ACTION` を追加。
- [x] T3 `ComponentSnapshot.selected`、`ViewError::InvalidInput`、`check_command`（空の target/action を拒否）。
- [x] T4 `ViewHost::select_row` / `ViewHost::invoke`、`emit_intent` の共通化（gesture と action で共有）、`selection_undo` による receipt 連動の revert。
- [x] T5 contract unit test 8 件（選択 intent と snapshot、不正入力、rejected の revert、accepted/applied の維持、update の clamp、Button の intent・phase・a11y、spec 検証、remove 時の routing 解放）。
- [x] T6 parity fixture `driver-lab/parity/selection-command.json`。
- [x] T7 Fake 側 parity テスト 2 件（`ui_dvr_02_parity_selection_command_mount`、`ui_dvr_03_parity_selection_command_intents_and_snapshot`）。
- [x] T8 GPUI 側: `button_element`、`table_element` の行押下と選択強調、`RvfView::{select_row, invoke}`、`DrawPlan::Button`、`DrawnPlan::Button`、`GpuiObserver::{select_row, invoke}`。
- [x] T9 GPUI seam テスト（`observes_selection_and_command_intents_through_seam`）。
- [x] T10 `cases.json` の binding（UI-DVR-02 fake/gpui、UI-DVR-03 gpui）。

## 未達・非スコープ

- [ ] T11 webview driver での `ViewSpec` 描画と intent 発行。`ruviform-ui-webview` に描画経路がないため未着手。webview parity は NotRun。別 Issue を推奨。
- [ ] T12 行の同一性（row key）と複数選択、キーボード操作。
- [ ] T13 receipt 進行（accepted / applied / rejected）の GPUI 側検証。headless 契約（`view::tests`）で検査し、GPUI seam からは receipt を送らない。

## 最終確認

- [ ] T14 `origin/main` を取り込んだ最新 HEAD で検証マトリクス（plan §4）を全件実行し、結果を PR 本文に記録する。
- [x] T15 review-protocol のレビューを実施し、blocker が残っていないことを確認する。独立サブエージェントは使わず、Reviewer A（協調）・Reviewer B（敵対）・Validator・Judge を分離したパスとして同一エージェントで実施した（1 サイクル）。確定した欠陥は 2 件（読み取り専用化後の選択の残存、復元時の範囲外行）で、いずれも P2 として本変更の範囲内で修正し、回帰テストを追加した。残る blocker はない。
