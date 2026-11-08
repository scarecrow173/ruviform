---
id: "PLAN-034UeymubB5x0zXeYGvfEl"
type: "plan"
status: "in-progress"
title: "Issue #314 slice 2 — Golden × Scenario 能力契約接続の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-314-golden-scenario.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/fake.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-314-golden-scenario.md"
    - "docs/specs/issue-314-golden-scenario.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-314-golden-scenario.md"
    - "docs/test-specs/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-scenario.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-golden-scenario.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 2 — Golden × Scenario 能力契約接続の実装計画

## Goal

`SemanticGuiTest` に visual/golden ポート（`VisualGuiTest`）を接続し、
Golden Utility の verdict/evidence を Scenario Contract の能力語彙と
`ScenarioReport` に正直に載せる。capture backend は実装しない
（slice 3）— 全 adapter は `Unsupported` outcome を返す経路のみ。

## Scope

単一 PR・2 crate（`ruviform-ui-test` + `ruviform-ui-gpui` の
`gpui-test` adapter）で収まる。分割計画は不要。

## Upstream Documents

- `docs/specs/issue-314-golden-scenario.md`（本 slice の要件・受入）
- `docs/designs/issue-314-golden-scenario.md`（D1〜D5 の設計判断）
- `docs/specs/issue-314-golden-core.md` /
  `docs/designs/issue-314-golden-core.md`（slice 1 — 接続対象）
- `docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md` /
  `docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md`
  （#464 契約の正本）

## Design Inputs

- `docs/designs/issue-314-golden-scenario.md`（approved）

## File Map

| ファイル | 変更内容 | 対応設計 |
| --- | --- | --- |
| `crates/ruviform-ui-test/src/scenario/visual.rs` | 新規 — `CaptureOutcome`・`VisualGuiTest` trait（`capture_frame` 必須 + `golden_check` 既定実装）・`record_golden_step`・`From<GoldenVerdict> for ScenarioVerdict`・unit tests | D1, D2, D4, D5 |
| `crates/ruviform-ui-test/src/scenario.rs` | `pub mod visual;` 追加・`ScenarioVerdict::lane_status` 追加 | D4 |
| `crates/ruviform-ui-test/src/scenario/fake.rs` | `impl VisualGuiTest for FakeGuiTest`（pin 再検査 + `Unsupported` outcome） | D5 |
| `crates/ruviform-ui-test/src/lib.rs` | `scenario::visual` の re-export 追加 | — |
| `crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs` | `impl VisualGuiTest for GpuiGuiTest`（同上） | D5 |
| `crates/ruviform-ui-test/tests/scenario_contract.rs` | Fake 側 golden step contract tests 追加 | AC-2,3,5,6 |
| `crates/ruviform-ui-gpui/tests/scenario_contract.rs` | GPUI 側 parity tests 追加 | AC-6 |
| `crates/ruviform-ui-test/README.md` | visual 接続の記述追加 | R8 |
| `crates/ruviform-ui-gpui/README.md` | 同上（gpui-test adapter 節） | R8 |
| `mise.toml` | `test-ui-scenario-contract{,-fake,-gpui}` の description に golden step coverage を追記 | R8 |
| `docs/development-environment.md` | task 対応表の同 3 行を同期 | R8 |
| `docs/**` | spec/design/plan/task/test-spec + 生成 index | — |

## Implementation Sequence

- [ ] `scenario/visual.rs` 骨格: `CaptureOutcome`・trait・
  `golden_check` 既定実装・`record_golden_step`・`From` impl・
  unit tests（mapping helper のみ — adapter 不要）。
- [ ] `scenario.rs`: `pub mod visual` + `lane_status`。
- [ ] `fake.rs`: `impl VisualGuiTest for FakeGuiTest`。
- [ ] `lib.rs` re-export。
- [ ] `gpui_driver/scenario.rs`: `impl VisualGuiTest for GpuiGuiTest`。
- [ ] Fake/GPUI `tests/scenario_contract.rs` に golden step tests。
- [ ] README・`mise.toml` description・`development-environment.md`。
- [ ] `cargo fmt` → `mise run test-ui-scenario-contract` →
  `mise run check-ui-driver-boundary` → `mise run lint`。
- [ ] review-protocol → commit/PR。

## Dependencies and Risks

- 依存: `golden`（slice 1, merge `2230018e`）・`scenario`（#464,
  merge `57e4e8ce`）— 共に main 入り済み。
- リスク: `ScenarioError` に `GoldenError` を載せられない
  （`Clone + PartialEq` 維持のため）→ `ScenarioError::Driver`
  文字列写像で対応（D2 記載済み）。
- リスク: boundary guard が新規 public signature を検査 →
  driver-neutral 型のみ使用で適合見込み。guard 実行で確認。

## Task Breakdown

- T1: `scenario/visual.rs` + `scenario.rs`（module 配線・lane_status）。
- T2: Fake/GPUI adapter impl。
- T3: contract/parity tests（両 crate）。
- T4: 文書・mise 説明・README 更新 + 検証 + review。

## Verification

- [ ] `mise run test-ui-scenario-contract`（fake+gpui 両 lane）
- [ ] `mise run test-ui-golden`（非回帰）
- [ ] `mise run check-ui-driver-boundary`
- [ ] `mise run check` / `mise run lint` / `mise run fmt-check`
- [ ] `mise run test-ui-test-boundary`（存在すれば）

## Open Questions / Gaps

- なし — 受入条件は task spec で確定済み。`capabilities()` への
  `VisualCapture` 非宣言は design D3 で確定。
