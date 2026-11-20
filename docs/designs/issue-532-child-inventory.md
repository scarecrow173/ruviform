---
id: "DES-034ZaaChildInventory001"
type: "design"
status: "approved"
title: "Issue #532 child-inventory probe — 設計"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-child-inventory.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-532-child-inventory.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-532-band-escape.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-child-inventory.md"
    - "docs/discovery/issue-532-child-inventory.md"
    - "docs/test-specs/issue-532-child-inventory.md"
    - "evidence/rf532/child-inventory-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-child-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 child-inventory probe — 設計

## 設計方針

- **query-only inventory**: `oschild::inventory` は
  `IsWindow`/`GetWindowLongPtrW`/`GetClassNameW`/`GetClassLongPtrW`/
  `GetWindowThreadProcessId`/`GetParent` の read のみ。各 read は
  `SetLastError(0)` を前置し、0 値と失敗を区別する（
  `last_err` は値が 0 かつ last-error 非 0 のときのみ記録）。
- **mutation は `osmut` 再利用**: child-level layered/reparent
  retry は新規 mutation 層を足さず、hwnd-generic な
  `osmut::apply_layered`/`osmut::reparent_to_desktop`/
  `osmut::restore_container`/`osmut::observe_container` を
  child HWND に直接適用する。verified write（read-back で
  要求値未確認なら `applied: false`）の honest 記録を継承。
- **capture は `ospix` 既存経路**: `PrintWindow` flag 変種
  （0/`PW_CLIENTONLY`/`PW_RENDERFULLCONTENT`/combined）・
  window DC `BitBlt`・screen `BitBlt` を step rows として記録。
  `PW_RENDERFULLCONTENT` は DWM の実表面要求で、`WM_PRINT`
  再描画経路との分離 proxy。
- **env arm は proxy 検証**: `GPUI_DISABLE_DIRECT_COMPOSITION`
  env の presence/value と root の
  `WS_EX_NOREDIRECTIONBITMAP`（fork `window.rs:409` で DComp
  有効時のみ付与）を対応付け、`inferred_dcomp_path` に
  `dcomp_active`/`dcomp_disabled_by_env`/`unknown` を記録。
  env の有効性を「設定した」と仮定せず bit で検証する。

## case 構成（`real_dcomp_childinv_case`）

1. `ci_inventory` — 全 docked child + container + root の
   `WindowInventory`、decoded style/exstyle/class_style bits。
2. `ci_capture` — chain 各対象の 4 flag 変種 + window DC +
   screen。dims/backend/green fraction/threshold/sha256/PNG。
3. `ci_dcomp_env` — env presence/value、root noredir bit、
   inferred path、cloaked/update-region。
4. `ci_layered` — child `apply_layered`（same-thread）、
   `latched_as_child` が false なら `reparent_to_desktop` →
   `apply_layered` retry → `latched_as_toplevel`。
5. settle + observation — post-mutation の capture/hit-test。
6. `ci_restore` — `restore_container`（child baseline への
   verified restore）。

## verdict 設計

既存 semantics を維持: `Pass` = 全要求観測の完遂、`Fail` =
必須 op 拒否・restore 不完全（probe/lifecycle の事実であって
compositor 判定ではない）、`NotRun`/`Unsupported` は既存どおり。
green fraction は note 級 data であり verdict 入力にしない。

## 非 Windows / headless

非 Windows stub は `supported: false`/`alive: false`、capability
不足（`NativePluginDock`/`VisualCapture` 非供給）は既存
fail-closed 経路で `NotRun` rows を吐く。捏造しない。
