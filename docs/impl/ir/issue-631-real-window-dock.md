---
id: "IMPL-034XTPwYBlfr1MLwQV3IDe"
type: "impl"
status: "in-progress"
title: "Real-window native-slot dock lane (issue-631 second slice)"
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
    - "docs/tasks/issue-631-real-window-dock.md"
  implemented-by: []
  depends-on: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Real-window native-slot dock lane (issue-631 second slice)

## Summary

<!-- Summarize what was implemented. -->

## Context

<!-- Explain which task, plan, spec, or ADR this implementation follows. -->

## Implementation

### Changes

<!-- Describe the important code or document changes. -->

### Decisions During Implementation

<!-- Record important implementation-time decisions. -->

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

<!-- Record the checks that were run. -->

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->

## Summary

`RealOsGuiTest` に NativeSlot ops（declare/bind/unbind/lease/
validate/release-lease/release-slot/state/container_token/redeem）を
`UiJob` seam 経由で追加し、`ScenarioCapability::NativePluginDock` を
`surface::BACKEND_AVAILABLE && !cfg!(feature = "gpui-test")` で gate
した。`ruviform-plugin-dock-test` に opt-in `real-dock` feature・
`plugin-dock-real-scenario` bin・`src/real_dock.rs`（4 case）・
`src/jsonl.rs`（JSONL helper 抽出）を実装し、`mise.toml` に
`test-ui-plugin-dock-real` を追加した。

## Context

`docs/tasks/issue-631-real-window-dock.md`（plan
`docs/plans/issue-631-real-window-dock.md`、spec
`docs/specs/issue-631-real-window-dock.md`）の second slice。
chain: NativeSlot → GpuiSurfaceAdapter（実 container HWND）→
lease_parent → ParentSlotLease → InProcessEditorOps redeem →
session.open(Docked)。

## 実装詳細

- `real_window.rs`: `slot_call` helper が `UiJob` closure 内で
  `driver.surface()` に op を実行。`SlotError` →
  `ScenarioError::Driver`（`native slot \`{op}\`` prefix）。
  `bind/unbind` は `driver.clone()`（`Rc<RefCell>`）で mutable 更新。
  `redeem_native_lease` は `SharedSurface::redeem` を呼ぶ。
- `real_dock.rs`: `ViewHost` + `plugin_dock_view_spec()` を
  `RealOsGuiTest::run` に mount。artifact は `resolve_artifact` +
  SHA-256 + `Vst3Library` で 1 回だけ load。`PluginEditorSessions`
  begin → `ParentSlotLease`（実 payload）→ `InProcessEditorOps` +
  `LeaseRedeemer` → `session.open(Docked)`。test_gain の
  `createView` null → Unsupported。
- `jsonl.rs`: run header + case 行 render と `repo_sha` を抽出。
  既存 bin の出力 schema は不変。
- `mise.toml`: `[tasks.test-ui-plugin-dock-real]` 追加のみ。

## 検証結果

interactive Windows session で実走（canonical = mise）:

- `fmt-check` / `check` / `lint` / `test`（`ruviform-ui-gpui` ±
  `real-scenario`、`ruviform-plugin-dock-test` ± `real-dock`）: pass
- `test-ui-plugin-dock`（headless 非回帰）/ `test-ui-scenario-contract`
  （+fake/+gpui）/ `test-ui-gpui` / `check-ui-driver-boundary`: pass
- `test-ui-plugin-dock-real`: 実走 — `real-window-mount`=pass,
  `real-native-slot`=pass（declare→bind→projection→lease_parent 実
  container→validate→container_token→release→stale/forged 拒否）,
  `real-editor-attach`=unsupported（test_gain createView null）,
  `real-capture`=pass（window/slot 領域画素）
- evidence: `evidence/rf631/plugin-dock-real-scenario.jsonl` +
  `verification.md`

### 設計からの差分（実走で判明）

- `bind_native_slot`/`unbind_native_slot` は bindings map 更新後に
  mounted view entity へ `cx.notify()` が必要 — GPUI は invalidation
  駆動のため、notify 無しでは静的 view が二度と paint せず
  `project_slot_frame` が走らず slot が永久に Hidden のままだった
  （初回実走で `project_geometry` fail として観測 → 修正後 pass）。
- `check-ui-driver-boundary`: plugin-dock-test → ui-gpui optional dep
  + `real-scenario` feature 起動は composition-root grant が必要 —
  Coordinator 承認のもと `boundary.json` に登録（poc-native-dock
  先例と同形式）。
