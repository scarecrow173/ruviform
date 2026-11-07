---
id: "TASK-Rd6GjUf3LxSo9WmEb4YhNi"
type: "task"
status: "in-progress"
title: "Issue #421 — Meter/TimeAxis GPUI 描画と parity の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/421"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-421-meter-timeaxis-gpui-parity.md"
      - type: "file"
        path: "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
      - type: "file"
        path: "docs/plans/issue-421-meter-timeaxis-gpui-parity.md"
      - type: "file"
        path: "docs/test-specs/issue-421-meter-timeaxis-gpui-parity.md"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-421-meter-timeaxis-gpui-parity.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Issue #421 — 実装タスク

## 完了

- [x] T1 Meter/TimeAxis の GPUI element 描画（`meter_element` / `time_axis_element`）。`unsupported component` placeholder は Meter/TimeAxis では出ない。
- [x] T2 描画 plan の純関数（`meter_bar_plan` / `axis_mark_plan`）と単体テスト。
- [x] T3 parity fixture `driver-lab/parity/new-kinds.json`。
- [x] T4 Fake 側 parity テスト（`tests/driver_parity.rs`、3 件）。
- [x] T5 GPUI 側 parity テスト（描画 plan・gesture・reconcile、4 件）。
- [x] T6 `cases.json` の status 更新（UI-DVR-02 GPUI pass、Fake parity binding 追加）。
- [x] T7 `mise run test-ui-parity` 追加と開発環境表の更新。

## 未達（follow-up へ分離）

- [ ] T8 Consumer 観測 seam の公開（`settle_for_test` / `inbox_len_for_test` の代替）。観測 DTO と feature 境界の設計を要する。follow-up Issue を推奨（design D5）。
- [ ] T9 `NumericInput` / `Transport` 相当 Control。#387 の Non-goals。Consumer 必要性の再評価を要する。
- [ ] T10 UI-DVR-03 GPUI の Cancel と遅延・順序逆転 receipt の lab binding 接続。
- [ ] T11 UI-DVR-07 / UI-DVR-08 の GPUI lane（capability honest reporting、関連 job の cancel と terminal event）。
- [ ] T12 story `LAB-SV-FADER-DOCK` / `LAB-AL-WAVEFORM-SEEK` の 2 scale factor × 2 theme × 状態遷移の GPUI 検証。
- [ ] T13 長尺・RT snapshot・gap/NaN precision・bounded memory・GPU release の GPUI 側検証（#387 受入の未達分）。

## 最終確認

- [ ] T14 merge 後 HEAD で検証マトリクス（plan §6）を全件実行し、結果を PR 本文に記録する。
- [ ] T15 review-protocol（独立サブエージェントによる diff レビュー）を実施し、blocker がないことを確認する。
