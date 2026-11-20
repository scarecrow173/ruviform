---
id: "SPEC-034ZaaChildInventory001"
type: "spec"
status: "approved"
title: "Issue #532 child-inventory slice（child HWND inventory + capture-plane 分離 + env arm 検証 + child-level layered retry）"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-child-inventory.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/plugin_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-band-escape.md"
    - "docs/specs/issue-532-uncovered-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "evidence/rf532/child-inventory-results.md"
    - "evidence/rf532/band-escape-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-band-escape.md"
  derived-by:
    - "docs/designs/issue-532-child-inventory.md"
    - "docs/discovery/issue-532-child-inventory.md"
    - "docs/test-specs/issue-532-child-inventory.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-child-inventory.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 child-inventory probe slice

## 背景と目的

`issue-532-band-escape` slice により、container の presentation
関係（layered/reparent）を変えても child pixel が最終画面に
現れないことが確定し、阻害は **child raster が composition
tree に到達しない層**に絞られた。残課題として示された次の
判別を 1 つの real-lane case に集約する:

- docked child HWND 自身の style/exstyle/class/所有関係の
  直接観測（child-level inventory）。
- editor-host class `0x4088` の `CS_PARENTDC` 含有実測。
- `PrintWindow` flag 変種・window DC・screen の 3 面 capture
  による raster proxy 分離。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` が fork の DComp path を
  実際に off にするかの検証（root `WS_EX_NOREDIRECTIONBITMAP`
  を proxy として使用）。
- child-level `WS_EX_LAYERED` write → 不 latch 時の desktop
  reparent retry による child-state 制約と cross-thread 説の
  分離。

本 slice は診断であり、修正でも方式決裁でもない。
**Issue #532 は open のまま**。

## スコープ

### In Scope

- `real_dock.rs` の `oschild` に `WindowInventory` を追加:
  `hwnd`/`supported`/`alive`/`style`/`exstyle`/`class_name`/
  `class_style`/`thread_id`/`process_id`/`parent`。query-only
  read（`IsWindow`/`GetWindowLongPtrW`/`GetClassNameW`/
  `GetClassLongPtrW`/`GetWindowThreadProcessId`/`GetParent`）。
  `SetLastError(0)` 済み read で legit 0 と失敗を区別。
  非 Windows stub は `supported: false`/`alive: false`（捏造なし）。
- style/exstyle/class_style の decode JSON（child/popup/
  visible/clip siblings/children、`WS_EX_NOREDIRECTIONBITMAP`/
  `WS_EX_LAYERED`/`WS_EX_TRANSPARENT`/`WS_EX_NOACTIVATE`/
  `WS_EX_APPWINDOW`/`WS_EX_TOOLWINDOW`、`CS_PARENTDC`/
  `CS_OWNDC`/`CS_CLASSDC`）。
- `real-dcomp-childinv` case（`real-dcomp-bandescape` 直後、
  `real_entries` 27 要素、required =
  `NativePluginDock`+`VisualCapture`、`CI_LABELS` prefix）:
  - `ci_inventory` — child（全 docked）/container/root の
    `WindowInventory` + decoded bits。
  - `ci_capture` — 各 chain 対象の `PrintWindow` flag 0/1/2/3
    + window DC + screen、capture dims/backend/green/sha256/
    PNG artifact を note 級 step rows に記録。
  - `ci_dcomp_env` — `GPUI_DISABLE_DIRECT_COMPOSITION` の
    presence/value と root `WS_EX_NOREDIRECTIONBITMAP` の対応、
    `inferred_dcomp_path`（`dcomp_active`/
    `dcomp_disabled_by_env`/`unknown`）。
  - `ci_layered` — child のまま `apply_layered`、不 latch なら
    `reparent_to_desktop` → retry、全 mutation/read-back を記録。
  - `ci_restore` — parent/exstyle/rect の verified restore。
- DWM proxy（`ospres::Attributes` の cloaked/update region）。
- `real_catalog.rs` row 追加（27 cases）。
- count/list 更新: `tests/console.rs`、`tests/plugin_dock.rs`、
  `README.md`、`lib.rs`、module doc table。

### Out of Scope

- GPUI fork 変更、production code 変更、修正実装。
- remediation 方式の選定・Issue #532 の closure。
- child raster 未供給と screen 未到達の最終 fix 証明。

## 受入条件

- inventory rows が全 chain 対象（child/container/root）で
  style/exstyle/class/thread/parent の measured fact を記録。
- capture rowsが flag 変種・window DC・screen の green/sha256 を
  note 級で記録し、screen oracle は既存 semantics を維持。
- env arm が root exstyle bit で `inferred_dcomp_path` を
  推論（未検証 run と区別）。
- layered arm が child-level write → reparent retry の全 op
  結果を記録し、refusal は `applied: false` のまま成功に
  改変しない。
- verdict は既存 semantics（`Pass`=観測完遂/`Fail`=必須 op
  拒否や restore 不完全/`NotRun`/`Unsupported`）を維持し、
  green pixel を verdict 入力にしない。
- headless では fail-closed（`NotRun`/honest rows）、全検証
  lane 通過。
