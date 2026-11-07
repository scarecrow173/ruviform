---
id: "TSPEC-Pw8DcTn5KgBy2VaQr7ZmFu"
type: "test-spec"
status: "proposed"
title: "Issue #421 — Meter/TimeAxis GPUI 描画と Fake/GPUI parity の検証契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/421"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-421-meter-timeaxis-gpui-parity.md"
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-414-projection-controlled-display.md"
---

# Issue #421 — 検証契約

## Feature: Meter/TimeAxis の GPUI 描画規則

### Rule: Meter の gap bin は描画されない

- Example: `count == 0` の bin、または非有限 peak の bin は `meter_bar_plan` の出力に含まれない。
  - 検査: `meter_and_axis_plans_skip_gaps_and_clamp`（gpui-test）。

### Rule: Meter の振幅は帯内にクランプされる

- Example: peak 3.0 は `1.0` に、非有限 RMS は `None`（peak のみ）になる。
  - 検査: `meter_and_axis_plans_skip_gaps_and_clamp`。

### Rule: TimeAxis の viewport 外 tick は描画されない

- Example: `viewport = 100..200` で `at = 99` は cull、`at = 200` は含める（閉区間）。
  - 検査: `meter_and_axis_plans_skip_gaps_and_clamp`。

## Feature: Fake/GPUI parity（同一 fixture）

### Rule: GPUI の描画 plan は shared fixture の drawn 期待値と一致する

- Example: `driver-lab/parity/new-kinds.json` の `expect.meter.drawnBars` と `expect.timeAxis.drawnMarks` に GPUI の plan が一致する。
  - 検査: `parity_new_kinds_plans_match_shared_fixture`（gpui-test、`mise run test-ui-gpui`）。

### Rule: Knob の gesture と Toggle の atomic set は契約どおりの intent 列を発行する

- Example: Knob は pointer drag で `begin` → `preview` → `commit`、Toggle の click は `begin` → `commit` で committed 値 `1.0`。
  - 検査（GPUI）: `rvf_view_parity_new_kinds_gesture_and_render`（実 pointer 入力）。
  - 検査（Fake）: `ui_dvr_03_parity_new_kinds_gesture`（`mise run test-ui-parity`）。

### Rule: reconcile は stable key を保つ

- Example: 落ちた key は `Removed`、残存 key（knob）は identity を保つ。
  - 検査（Fake）: `ui_dvr_02_parity_new_kinds_mount_reconcile`。
  - 検査（GPUI）: `rvf_view_parity_new_kinds_mount_reconcile`（`mise run test-ui-gpui`）。

### Rule: 宣言 payload は headless 側で保持される

- Example: gap bin（count 0）2 件と viewport 外 tick を含む payload が mount を通り、件数が保持される。
  - 検査（Fake）: `parity_new_kinds_payload_retained_fake`。

## Feature: manifest と境界

### Rule: manifest の status は実行された検査に限り pass と宣言する

- Example: `UI-DVR-02` GPUI は `test-ui-gpui` の実行で pass。`UI-DVR-03` GPUI は Cancel 未接続のため not_run。
  - 検査: `manifests_are_consistent`（`mise run test-ui-driver-contract`）。

### Rule: 共通層に GPUI 型が露出しない

- Example: boundary guard の dep-graph / signature 検査が pass。
  - 検査: `mise run check-ui-driver-boundary`。

## 未検証（NotRun / Unsupported）

- Consumer 観測 seam（AudioLens PR #169 の GPUI parity 観測）: 未公開のため NotRun。
- Cancel・遅延/順序逆転 receipt の GPUI 検証: NotRun（UI-DVR-03 gpui not_run）。
- 2 scale factor × 2 theme の story 検証: NotRun（stories.json 未変更）。
- 実 OS window 上の描画（pixel 比較）: Unsupported（手動検証の範囲。`example-ui-gpui-minimal` は手動入口）。
