---
id: "TSPEC-034UeyabB9HNuQvfyPVYdA"
type: "test-spec"
status: "proposed"
title: "Issue #314 slice 2 — Golden × Scenario 能力契約接続のテスト仕様"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-314-golden-scenario.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-scenario.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 2 — Golden × Scenario 能力契約接続のテスト仕様

## Purpose

`VisualGuiTest`/`golden_check`/verdict 写像が「capture できない
adapter で嘘の `pass` を出さない」契約を Fake・GPUI の両側で
固定する。slice 3 が `Captured` outcome を実装した時に、この
契約が退行していないことを検出するための土台。

## Feature: Visual golden step on SemanticGuiTest adapters

### Rule: capability declaration stays honest

- Example: `FakeGuiTest`/`GpuiGuiTest` の `capabilities()` は
  `SemanticAction` + `MultiWindow` のみを返し、
  `require_capabilities({VisualCapture})` は
  `ScenarioError::Unsupported{VisualCapture}` で失敗する。
  `golden_check` を直接呼んでも `pass` にはならない。
- 検証: `capability_preflight_rejects_unsupported_levels`（既存）
  + `gpui_capability_preflight_matches_fake`（既存）+
  新規 `visual_*` tests。

### Rule: golden_check returns an honest verdict report, not an error

- Example: live な `ScenarioTarget` で `golden_check` を呼ぶと
  `Ok(GoldenComparison)` が返り、`report.verdict ==
  GoldenVerdict::Unsupported`、`reason` が非空、`stats` が `None`、
  `diff_image` が `None`。`pass`/`fail`/`blocked` にはならない。
- 検証: `visual_golden_check_returns_unsupported_report`（Fake）
  ・`gpui_visual_golden_check_returns_unsupported_report`（GPUI）。

### Rule: stale target is a contract error before any capture claim

- Example: detach/update で generation/epoch が進んだ target に
  `golden_check`（および `capture_frame`）を呼ぶと
  `Err(ScenarioError::StaleGeneration|StaleEpoch)` — capture 不能
  adapter でも pin 契約は他操作と同じ。
- 検証: `visual_golden_check_stale_target_is_contract_error`
  （Fake）・GPUI 側同等 test。

### Rule: verdict mapping preserves the five-value vocabulary

- Example: `ScenarioVerdict::from(GoldenVerdict::X)` は全 5 値で
  同名写像。`ScenarioVerdict::lane_status()` は
  `Blocked`→`"not_run"`、他は `as_str()` と同じ lane 語彙
  （`pass`/`fail`/`not_run`/`unsupported`）。
- 検証: `scenario::visual` unit tests（`golden_verdict_maps_*`・
  `lane_status_maps_blocked_to_not_run`）。

### Rule: record_golden_step writes the canonical evidence shape

- Example: `record_golden_step(&mut report, &comparison)` は
  `report.evidence` に `golden:<canonical key>` を追加し、
  `report.observed["golden"]` に verdict/reason/detail/baseline/
  stats を記録する。`report.verdict` は変更しない
  （consumer の総合判断）。`observed` が非 object の場合は
  上書きしない。
- 検証: `record_golden_step_*` unit tests。

### Rule: Fake/GPUI parity on the visual surface

- Example: 同一の golden key/store/request で両 adapter の
  `golden_check` が同じ verdict 語彙（`unsupported`）・同じ
  evidence schema（`rvf.golden.evidence/v1`）・同じ report 形を
  返す。reason 文字列は adapter 固有でよい（parity は語彙で取る）。
- 検証: 両 `tests/scenario_contract.rs` の `visual_*`/`gpui_visual_*`
  tests が同じ assertion 形を持つ。

## Guarantees

- `pass` は pixel 比較が実際に測定した時のみ — capture 不在の
  adapter は決して `pass`/`fail` を返さない。
- `ScenarioError`（contract/tool error）と verdict（測定結果）
  は分離される — stale/missing target は report ではなく error。
- `rvf.golden.evidence/v1` evidence は env ゲート付きで emit され、
  未設定時に副作用を持たない。
- `blocked`=`not_run`+reason の lane 写像がコード化され、
  driver-lab/consumer-verify の既存規則を壊さない。

## Non-goals

- `Captured` outcome → `BaselineStore::compare` の `pass`/`fail`/
  `blocked` 経路の実測 — capture backend が存在しないため
  slice 3 の test 領域。本 slice は outcome→verdict の写像と
  unsupported/not_run 経路のみを固定する。
- diff 画像 artifact の永続化・baseline promote の scenario 統合
  — caller の責務（slice 1 設計を継承）。
- semantic oracle・draw-plan 観測との統合 — pixel と semantic の
  証拠は独立のまま。

## Risk

- capability 宣言（`capabilities()`）と golden step の実測
  outcome が乖離した場合、consumer が preflight pass を golden
  実行可能と誤読するリスク — `required`/`capabilities`/
  `observed.golden.verdict` の 3 点で観測可能にし、doc/test で
  「preflight は宣言、golden_check は実測」を固定する。
- slice 3 で `capture_frame` が `Captured` を返す実装を入れた際、
  本 spec の verdict 写像・evidence emit・stale 契約がそのまま
  効くこと — 退行検出が本 spec の役割。
