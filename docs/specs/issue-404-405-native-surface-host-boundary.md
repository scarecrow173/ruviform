---
id: "SPEC-xCsjb8OsiMswabSn30uVVs"
type: "spec"
status: "draft"
title: "Issue #404/#405 NativeSurfaceSlot host 境界修復（clip 解除/再投影 + owner window teardown 伝播）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/404"
    - "https://github.com/scarecrow173/ruviform/issues/405"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-388-native-surface-slot.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/specs/issue-386-driver-neutral-gui-host.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-388-native-surface-slot.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/399"
  defers: []
  deferred-by: []
metadata:
  issues: [404, 405]
---

# Issue #404/#405 NativeSurfaceSlot host 境界修復

## 背景

PR #399（Issue #388/#389、e8adcc0 merge）で導入された
`GpuiSurfaceAdapter`（`crates/ruviform-ui-gpui/src/gpui_driver/
surface.rs`）と Windows `Container` backend に、Issue #387 の
独立 review で同一 host adapter 所有の 2 欠陥群が検出された。
本 spec は Issue #404・#405 本文を受入条件の正本として、
clip 系不具合と owner window teardown 未伝播を 1 作業で修復する。

## スコープ

### In Scope

- **Issue #404 clip 系 2 不具合 + OS 操作失敗時の ledger 整合**
  - `Container::set_clip(None)` が `SetWindowRgn(hwnd, NULL, TRUE)`
    を発行せず、一度適用した clip が container 破棄まで残留する。
  - `GpuiSurfaceAdapter::set_geometry` が保持中の `binding.clip` を
    新 geometry で再投影せず、clip 適用後の slot 移動で region が
    旧 origin 基準のまま残る。
  - OS 操作が失敗した場合に adapter 側記録（`binding.clip`）だけが
    先行更新される経路を、失敗を typed error で返す整合へ修復する。
- **Issue #405 owner window teardown の adapter 伝播**
  - `GpuiDriver` の window 破棄経路（`retire`／`apply_destroy`／
    `sweep_destroyed`）が adapter の slot ledger・`binding.container`
    に一切反映されず、死んだ HWND が live のまま `Ok`/event を
    返す偽成功を排除する。
  - 破棄時の契約を port 契約として確定する（backend 触れうる op の
    typed error／auto-release・terminal event・container 破棄・
    `state()`/`redeem()`/`drain_events()` の意味）。
- standalone `GpuiSurfaceAdapter::new(&driver)` と RF387 の
  canonical `GpuiDriver::surface()` adapter の**両方で成立する**
  修復であること（`retire`→adapter への到達機構を共有の driver
  状態に置くか、adapter 側 reconciliation に置くかを設計で確定）。
- `mise run test-ui-gpui`（Windows 実機）で
  `SetWindowRgn`/geometry 再投影/parent destroy 後の ledger 収束を
  実 HWND 上で自動検証する最小 test。

### Out of Scope

- renderer/behavior 実装（RF387 candidate PR406・別 Worker 所有）。
- 実 Dock レイアウト・tab・input・実外部 VST3 editor の受入（別 Gate）。
- L1/cross-process、macOS/Linux backend（未実装・`Unsupported` のまま）。
- `ruviform-ui` neutral 契約への新語彙追加（`SlotError`/`SlotEvent`
  の既存 variant で表現できる範囲に留める）。
- driver 未 `run()` 終了後の window 管理仕様変更。

## 受入条件（Issue 本文の検証条件）

### Issue #404

- AC-404-1: Windows 上で `lease_parent` 済み slot に
  `set_clip(Some)` → `set_clip(None)` → `GetWindowRgn` が
  region なしを返す。
- AC-404-2: `set_clip(Some(C))` → `set_geometry(移動後)` →
  container region が新 geometry 基準で再計算されている。
- AC-404-3: backend OS 操作の失敗が `Ok` のまま通過しない —
  失敗可能な操作（clip 適用・再投影）は `SlotError::Backend` 系
  typed error を返し、adapter 記録は成功時のみ更新される。

### Issue #405

- AC-405-1: slot declare/lease 後に owner window を destroy →
  決めた契約どおりに `binding.container` が破棄され、
  `drain_events()` が `LeaseRevoked`+`Destroyed`（live lease 時）/
  `Destroyed`（declare のみ）の terminal event を返す。
- AC-405-2: destroy 後の backend 触れうる op（`set_geometry`/
  `set_visibility`/`set_clip`/`set_zorder`/`request_focus`/
  `lease_parent`/`mark_busy`/`clear_busy`/`release_lease`/
  `release_slot`）が fake `Ok` でなく typed error（`Released`/
  `StaleGeneration`/`Backend`/`LeaseRevoked`）を返す。
- AC-405-3: `adapter.state(slot)`・`adapter.container_token(slot)`・
  `adapter.redeem(payload)`/`validate_lease` が destroy 後に
  live を報告しない（`Released`/`None`/typed error）。
- AC-405-4: window open/close を繰り返しても `bindings`・
  registry record が無期限に単調増大しない（released index は
  次回 declare で generation++ 再利用・`bindings` から除去）。

### 両 Issue 共通

- AC-X-1: consumer が明示した `declare_slot`/`lease_parent`/
  `release_lease`/`release_slot` の所有責務は変えない —
  adapter は「OS container の死」の記録のみ負う。
- AC-X-2: standalone `new(&driver)` と RF387 canonical
  `surface()`（`bound(&inner)` 経由）の両経路で同じ契約が成立する。
- AC-X-3: 正規検証入口 `mise run fmt-check`/`check`/`lint`/`test`/
  `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` が green。
- AC-X-4: dead generation・repeated close・release after destroy・
  再 window create・slot index reuse・adapter drop・複数 adapter
  併存が回帰しない（比例した回帰 test）。

## 非目標

- 新規 feature/optional 依存・新 workflow・PR gate の追加。
- main 直 push・PR merge・Issue close（coordinator 権限）。
- `Clip`/`ZOrder` の multi-slot ordered chain（既存
  single-top approximation のまま）。

## 確定すべき設計判断（Issue #405「要設計判断」より）

1. teardown 伝播機構: driver `retire` が adapter へ push するか、
   adapter が共有 `inner` の `destroyed`/`windows` + container
   `is_alive()` を自身の op 境界で reconcile するか。
2. 死亡 slot の帳簿処理: auto-release（`LeaseRevoked`+`Destroyed`
   終端化・index 再利用可）か、dead 印を保持し consumer release
   まで残すか — 無期限 orphan 保持は不可。
3. `state()`/`spec()`/`container_token()`/`validate_lease()`/
   `redeem()`（`&self`、reconcile 不可）が死亡未収束時に返す値。

決定は `docs/designs/issue-404-405-native-surface-host-boundary.md`
で独立 review 後に確定する。
