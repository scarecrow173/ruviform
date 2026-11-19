---
id: "SPEC-034XTOYvrNP8RQ01gP2AGt"
type: "spec"
status: "approved"
title: "Issue #631 Real-window native-slot dock lane（RealOsGuiTest 実 OS window 上の NativeSlot → container HWND lease → ParentSlotLease → InProcessEditorOps redeem → session.open(Docked) 実走記録）"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Real-window native-slot dock lane（RealOsGuiTest 実 OS window 上の NativeSlot → container HWND lease → ParentSlotLease → InProcessEditorOps redeem → session.open(Docked) 実走記録）

## 背景

前 slice（`issue-631-plugin-dock-scenario`）で `ruviform-plugin-dock-test`
crate の headless/semantic lane を整備した。`editor-attach` は fake slot
経由の contract 証拠であり、実 OS window・実 container HWND・実 lease
redeem を通った証拠ではない。本 spec は Issue #631 の後続 slice として、
実 window 上で production chain を実走する lane を定義する。

対象 chain:

```
NativeSlot → GpuiSurfaceAdapter（実 container HWND）→ lease_parent
→ ParentSlotLease → InProcessEditorOps redeem → session.open(Docked)
```

## Stakeholders

- GUI Test Driver / Scenario Runner の consumer（driver-neutral な
  scenario contract 経由で証拠を受け取る）。
- #630 test plugin suite の `test_editor` 実物到着後に、本 lane の
  `real-editor-attach` を Pass へ引き上げる後続作業。

## Scope

### In Scope

- `RealOsGuiTest` への NativeSlot ops 追加（declare / bind / unbind /
  lease / validate / release lease / release slot / state /
  container_token 観測 / issuer-side redeem）。`UiJob` channel seam
  経由で GPUI app thread 上の canonical `SharedSurface` を操作する。
- `ScenarioCapability::NativePluginDock` を backend 連動で
  `supported_capabilities()` に追加（`surface::BACKEND_AVAILABLE &&
  !cfg!(feature = "gpui-test")`）。
- `ruviform-plugin-dock-test` に opt-in `real-dock` feature と
  `plugin-dock-real-scenario` bin、`src/real_dock.rs` module。
- 4 case: `real-window-mount` / `real-native-slot` /
  `real-editor-attach` / `real-capture`。verdict は
  Pass/Fail/NotRun/Unsupported を正直に記録する。
- `src/jsonl.rs` への JSONL render/repo SHA helper 抽出と両 bin での共用。
- `mise.toml` に manual/opt-in task `test-ui-plugin-dock-real` を追加
  （既存 task の変更・削除なし）。
- `evidence/rf631/` への実走 JSONL と `verification.md` 追記。

### Out of Scope

- `test_editor` plugin class 実装（`createView` が実 HWND を返す実物）
  — #630 残存 scope。
- dock-pixels・child HWND input/resize/focus/re-dock の Pass 判定、
  DComp #532 判定 — 実物到着まで NotRun/Deferred。
- Test Lab Console / Scenario Console GUI、2Window Focus/DPI/IME
  実検証（IME は #389）、Synvelis/AudioLens 製品 oracle、3OS 対応、
  `*-msrv` lane、Issue auto-close。
- `crates/ruviform-ui-gpui` の `real_window.rs` 以外の変更、
  `ruviform-audio-test` 等の他 Worker 占有 crate の変更。

## Requirements

- slot ops は `UiJob`/`call` cross-thread seam 経由で app thread 上の
  `driver.surface()`（canonical `SharedSurface`）を操作する。
- slot 失敗は `ScenarioError::Driver` に `native slot \`{op}\`` prefix
  で写像する。公開 enum は `#[non_exhaustive]` 方針に従う。
- `ParentSlotLease` は opaque `u64` payload を保持し、raw OS handle
  を consumer 境界へ出さない。redeem は canonical ledger のみが行う。
- `real-editor-attach` は `test_gain` の `createView` null →
  `HostError::Unsupported` のまま **Unsupported**（Pass にしない）。
- 非対話/headless 環境では fail-closed で NotRun/
  environment_blocked を記録し、Required case を偽 Pass にしない。
- `EnvironmentKind::RealWindow` は実走経路でのみ emit する。
- GPUI 型を本 crate の API・evidence schema へ漏らさない
  （`check-ui-driver-boundary` green）。

## Acceptance Criteria

- [ ] `RealOsGuiTest` が declare/bind/lease/validate/release/state/
  container_token/redeem を first-class method として公開し、
  `cargo check`/`clippy -D warnings` が通る。
- [ ] `supported_capabilities()` が `NativePluginDock` を backend
  predicate 連動で報告し、既存 capability test が更新される。
- [ ] `plugin-dock-real-scenario --features real-dock` が 4 case の
  JSONL を出力し、`real-native-slot` で declare→bind→projection→
  lease→validate→container_token 観測→stale/forged 拒否→release が
  記録される。
- [ ] `real-editor-attach` が Unsupported、`real-capture` が
  capture 対象（window/slot 領域画素）を detail に明記する。
- [ ] `mise run test-ui-plugin-dock-real` が manual/opt-in lane として
  動作し、headless 時に environment_blocked を記録する。
- [ ] `mise run test-ui-plugin-dock` 他の非回帰 lane が green。

## Deferred Design Concerns

- `test_editor` 実物到着後の `real-editor-attach` Pass 化・dock
  pixels/input/resize/focus/re-dock 判定 — `test_editor` 到着まで
  NotRun/Unsupported 維持 | 後続 slice | 実物 fixture の merge |
  Pass 偽装のリスク。
- DComp #532 既知 Fail の再現・解消判定 — 同上。
- 3OS 対応（macOS/Linux GUI） — Deferred。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — auto-close しない）。
- 前 slice spec `docs/specs/issue-631-plugin-dock-scenario.md`、
  design `docs/designs/issue-631-plugin-dock-scenario.md`。
- `POLICIES.md`（検証レベル・Pass/Fail/NotRun/Unsupported 規範・
  仮想優先・fail-closed）。
- 先例: `real_window.rs` の `UiJob` seam、`surface.rs` の
  `lease_parent`/`BACKEND_CAPS`、`poc-native-dock`、
  `editor_session.rs` の `LeaseRedeemer` seam、
  `test-ui-real-scenario` mise task。
