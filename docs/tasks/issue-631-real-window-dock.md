---
id: "TASK-034XTPgAXjEOXFWn2Rv1Iy"
type: "task"
status: "in-progress"
title: "Issue #631 T1 — Real-window native-slot dock lane"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-631-real-window-dock.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-real-window-dock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 T1 — Real-window native-slot dock lane

## Work

<!-- Describe the implementation slice. -->

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- <!-- prerequisite, blocker, or related task -->

## Affected Files

- <!-- file, directory, module, or interface -->

## Done When

- [ ] <!-- completion criterion -->

## Verification

- [ ] <!-- command, test, review step, or observable behavior -->

## Work

1. `real_window.rs`: `UiJob` 経由の slot ops（declare/bind/unbind/
   lease/validate/release-lease/release-slot/state/container_token/
   redeem）を `RealOsGuiTest` public method として追加。
   `slot_call` helper で `SlotError` → `ScenarioError::Driver`
   （`native slot \`{op}\`` prefix）に写像。
   `supported_capabilities()` に backend-gated `NativePluginDock` を追加
   し、module docs と capability test を更新。
2. `ruviform-plugin-dock-test`: `real-dock` feature・optional dep・
   gated bin・`src/real_dock.rs`・`src/jsonl.rs` を実装し、
   既存 bin を helper 利用に refactor。README 更新。
3. `mise.toml` に `test-ui-plugin-dock-real` を追加。
4. 検証実行・evidence JSONL/verification.md 記録・doc chain 完成。

## Affected Files

- `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs`
- `crates/ruviform-plugin-dock-test/`（Cargo.toml, src/, tests/, README.md）
- `mise.toml`（追加のみ）
- `Cargo.lock`（再生成のみ）
- `docs/*/issue-631-real-window-dock.*`、`evidence/rf631/`

## Done When

- [ ] slot ops が compile/lint/test を通り、capability が backend
  predicate 連動で報告される。
- [ ] real-dock lane が 4 case の正直な verdict JSONL を出力する。
- [ ] 非回帰 lane（test-ui-plugin-dock 他）が green。
- [ ] evidence/rf631 に実走結果と verification.md が記録される。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check/lint/test ruviform-ui-gpui`
- [ ] `mise run check/lint/test ruviform-plugin-dock-test`（default + real-dock）
- [ ] `mise run test-ui-plugin-dock` / `test-ui-scenario-contract` 系 / `test-ui-gpui` / `check-ui-driver-boundary`
- [ ] `mise run test-ui-plugin-dock-real`
- [ ] `mise exec -- prek run --all-files`
