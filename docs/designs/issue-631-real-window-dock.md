---
id: "DESIGN-034XTP681X6oGd0SwnvpbS"
type: "design"
status: "approved"
title: "Issue #631 Real-window native-slot dock lane 設計"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-real-window-dock.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Real-window native-slot dock lane 設計

## Context

<!-- Describe the system context and problem boundaries. -->

## Scope

### In Scope

- <!-- capability or flow -->

### Out of Scope

- <!-- explicit non-goal -->

## Components and Boundaries

- <!-- component and responsibility -->

## Data and Control Flow

- <!-- major data/control flow steps -->

## Non-Functional Constraints

- <!-- reliability, performance, security, operations -->

## Risks and Trade-offs

- <!-- risk and mitigation -->

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- <!-- concern | reason | re-engagement trigger | risk if ignored -->

## Verification Notes

- [ ] <!-- what validates this design before planning -->

## 概要

`RealOsGuiTest`（`crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs`）
に NativeSlot ops を追加し、`ruviform-plugin-dock-test` に opt-in
`real-dock` feature の real-window lane を新設する。slot ledger の
正本は GPUI app thread 上の canonical `SharedSurface` のままとし、
consumer（plugin-dock-test）には `ruviform-ui` の語彙だけを返す。

## 詳細設計

### 1. `RealOsGuiTest` native-slot API（real_window.rs のみ変更）

既存 `UiJob`/`call` cross-thread seam を再利用し、以下を public
method として追加する。全て `Result<_, ScenarioError>` — slot ledger
の `SlotError` は `ScenarioError::Driver` に
`native slot \`{op}\`: {error}` prefix で写像する。

- `declare_native_slot(spec: SlotSpec) -> Result<SurfaceSlotId, _>`
- `bind_native_slot(slot: SurfaceSlotId, key: ComponentKey) -> Result<(), _>`
- `unbind_native_slot(key: ComponentKey) -> Result<(), _>`
- `lease_native_slot(slot: SurfaceSlotId) -> Result<SlotLease, _>`
- `validate_native_lease(lease: &SlotLease) -> Result<LeaseInfo, _>`
- `release_native_lease(lease: SlotLease) -> Result<(), _>`
- `release_native_slot(slot: SurfaceSlotId) -> Result<(), _>`
- `native_slot_state(slot: SurfaceSlotId) -> Result<SlotState, _>`
- `native_slot_container_token(slot: SurfaceSlotId) -> Result<Option<u64>, _>`
- `redeem_native_lease(payload: u64) -> Result<u64, _>`（issuer-side
  redeem — `SharedSurface::redeem` を呼ぶ）

内部 helper `slot_call<T, F>` が `UiJob` closure 内で
`driver.surface()` に対して op を実行し、`SlotError` を
`ScenarioError::Driver` に写像する。`bind_native_slot` は
`driver.clone().bind_native_slot(...)`（`GpuiDriver` は `Rc<RefCell>`
+ `Clone`）で mutable 状態を更新する。

`native_dock_backend_available()` =
`surface::BACKEND_AVAILABLE && !cfg!(feature = "gpui-test")`。
`supported_capabilities()` はこの predicate が真のときのみ
`ScenarioCapability::NativePluginDock` を含める。`gpui-test` feature
は native capture/container を stub するため real container HWND を
mint できず capability を名乗れない。

### 2. plugin-dock-test `real-dock` feature

`Cargo.toml`:

```toml
[features]
real-dock = ["dep:ruviform-ui-gpui", "ruviform-ui-gpui/real-scenario"]

[dependencies]
ruviform-ui-gpui = { path = "../ruviform-ui-gpui", optional = true }

[[bin]]
name = "plugin-dock-real-scenario"
path = "src/bin/plugin-dock-real-scenario.rs"
required-features = ["real-dock"]
```

optional dep は consumer 層に閉じ、default/headless lane の依存木に
GPUI を入れない。`Cargo.lock` 差分は optional dep 追加による再生成のみ。

### 3. `src/real_dock.rs`（feature-gated module）

scenario id `rf631.plugin-dock-real`、4 case:

- `real-window-mount`: `plugin_dock_view_spec()` を `ViewHost` に mount
  し `RealOsGuiTest::run` で実 window を起こす。semantic mount と
  target 解決を確認。
- `real-native-slot`: declare → `PLUGIN_DOCK_SLOT_KEY` bind →
  renderer projection settle（`settle_until` で geometry/visibility
  更新を待つ）→ lease → validate → `container_token` 観測 →
  forged/stale payload の validate/redeem 拒否 → lease release →
  slot release。実走できた step のみ pass、拒否は evidence detail に記録。
- `real-editor-attach`: 実 `test_gain` artifact を resolve/load し、
  `PluginEditorSessions` begin → `ParentSlotLease`（実 lease payload）
  → `InProcessEditorOps` + `LeaseRedeemer`（`redeem_native_lease`
  呼出）→ `session.open(Docked)`。`createView` null →
  `HostError::Unsupported` で **Unsupported** verdict。
- `real-capture`: `VisualGuiTest::capture_frame` で実 window/slot
  領域の画素を取得。capture 対象が plugin editor 画素ではない旨を
  detail に明記（editor 不在のため）。

headless/非対話では `RealOsGuiTest::run` の window 生成失敗を
`EnvironmentBlocked`/`NotRun` に写像する blocked-report helper を用意。
`EnvironmentKind::RealWindow` は実走経路のみで emit。

### 4. `src/jsonl.rs` + bins

既存 `plugin-dock-scenario.rs` bin の JSONL render（run header +
case 行）と `repo_sha` を `src/jsonl.rs` に抽出し、両 bin で共用。
schema は不変。`plugin-dock-real-scenario.rs` は lane 名
`test-ui-plugin-dock-real` で同 schema を出力し、Fail のみ非 0 終了
（Unsupported/NotRun は正直な非 Pass として exit 0）。

### 5. `mise.toml`

`[tasks.test-ui-plugin-dock-real]` を追加のみ（既存 task 不変）。
`test-ui-plugin-dock`（test_gain cdylib build 内包）と
`test-ui-real-scenario`（interactive session・evidence env var）の
先例を踏襲。description に manual/opt-in・非対話時
environment_blocked・CI 必須でない旨を明記。出力は
`evidence/rf631/plugin-dock-real-scenario.jsonl`。

## 代替案と却下理由

- **slot ops を `lib.rs` re-export 付きの専用 error enum で公開**:
  `lib.rs` は編集範囲外。`ScenarioError::Driver` + op prefix で
  十分に区別可能。
- **`real_dock` を既存 bin に統合**: headless lane の意味を変えず、
  feature-gated 別 bin で opt-in を明示する方が fail-closed に適合。
- **fake slot で editor attach を Pass 化**: contract 証拠を real
  window 証拠に格上げすることになり POLICIES.md 違反。

## リスク

- `RealOsGuiTest::run` が interactive session を要求 — CI では
  environment_blocked 記録に留まる（意図どおり）。
- renderer projection の settle が timing 依存 — `settle_until` の
  timeout を case timeout として記録し、timeout は Fail ではなく
  観測事実として detail に残す。
- `Cargo.lock` 差分 — optional dep 追加の再生成のみ。PR 本文に明記。
