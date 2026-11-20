---
id: "SPEC-034ZaaRasterfix000001"
type: "spec"
status: "approved"
title: "Issue #532 raster-fix slice（private container class + paint drain + reparent/layered treatment arms + pixel-based fix oracle）"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-rasterfix.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/osmut.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/plugin_dock.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-child-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-532-rasterfix.md"
    - "docs/discovery/issue-532-rasterfix.md"
    - "docs/test-specs/issue-532-rasterfix.md"
    - "evidence/rf532/rasterfix-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-child-inventory.md"
  derived-by:
    - "docs/designs/issue-532-rasterfix.md"
    - "docs/discovery/issue-532-rasterfix.md"
    - "docs/test-specs/issue-532-rasterfix.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-rasterfix.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 raster-fix slice

## 背景と目的

`issue-532-child-inventory` slice で、(a) docked editor child
`RuviformTestEditor` は caller thread 所有で pending update
region を持つ、(b) container（当時 system `STATIC` class、
`CS_PARENTDC` 含有）は GPUI thread 所有、(c) `PrintWindow` は
緑を読むが screen `BitBlt` は読まない、(d) DComp off でも
user-visible pixel は戻らない — が確定した。

本 slice はその残課題である「ruviform 側の変更だけで
user-visible pixel を復元できるか」を pixel-based oracle で
判定する。**Issue #532 は open のまま**（`Refs #532` のみ、
closure でも方式決裁でもない）。

## スコープ

### In Scope

- `surface.rs` の GPUI slot container を system `STATIC`
  class から private 登録 class `RuviformSlotContainer`
  （`style: 0`、`lpfnWndProc = DefWindowProcW`、process
  `Once` + `GetClassInfoW` 確認、失敗時は `SlotError` —
  `STATIC` への silent fallback なし）へ置換。
  `CS_PARENTDC` を排除する実験的治療 + 既存 contract
  （clip/geometry/dead-parent/teardown）の保持。Windows
  test で class 名・class style を検証。
- `osmut::drain_paint` — `UpdateWindow` + bounded
  caller-thread `PeekMessageW`/`DispatchMessageW`
  （`WM_PAINT` filter、cap 付き）による paint-drain
  mutation API + 非 Windows honest stub。
- `real-dcomp-rasterfix` case（`real_entries` 28 要素、
  required = `NativePluginDock`+`VisualCapture`、
  `RF_LABELS` prefix）。arm:
  - `baseline` — container/child/root snapshot + baseline
    screen green。baseline で閾値達なら arms skip。
  - `pump` — docked のまま `drain_paint` → raster + screen。
  - `reparent` — child を desktop reparent（top-level 化）
    + `HWND_TOPMOST` 昇格 → `drain_paint` → raster + screen。
  - `layered` — reparent 未達時のみ、now-top-level child に
    `apply_layered`（`WS_EX_LAYERED` + `SWP_FRAMECHANGED` +
    `LWA_ALPHA=255`）→ 同 evidence。
  - `restore` — parent/exstyle/rect の verified restore。
- verdict semantics（fix oracle 導入）: `Pass` は
  「いずれかの arm が screen green `>= 0.5`（`fix_arm`）
  を出した」+「probe が evaluable」+「必須 mutation が
  全て実行」+「restore が検証済み」+「lifecycle が完遂」
  の連言。arms 実行済みだが green 非達は honest `Fail`
  （仮説棄却）、setup 不可は `NotRun`、platform 限定は
  `Unsupported`。
- `osmut` restore/mutation 系の実測駆動修正:
  - `restore_container` の op 順を band → exstyle →
    `SetParent` → rect に変更（`GWL_EXSTYLE` write は
    foreign-thread parent 配下で拒否されるため）。
  - z-band restore（`HWND_NOTOPMOST`/`HWND_TOPMOST`）を
    baseline band との差があるときのみ追加
    （`WS_EX_TOPMOST` は exstyle write では解除不能）。
  - `exstyle_write` の `applied` を read-back 一致判定に
    変更（refused no-op write の silent drop と実拒否を
    区別、拒否は `win32_error` に記録したまま）。
- catalog/count 同期: `real_catalog.rs`、`lib.rs`、
  `README.md`、`tests/console.rs`、`tests/plugin_dock.rs`。

### Out of Scope

- gpui fork（`scarecrow173/gpui`）への変更 — coordinator
  の明示指示があるまで別 workstream。
- editor を top-level window で提示する製品 fix 実装
  （本 slice はその ruviform 側経路の証明のみ）。
- Issue #532 の closure、docked-in-place fix の最終判断。
- `be_`/`ci_` 診断 case の verdict 変更（`restore_container`
  の共有 op 順改善はそれらにも適用されるが、case 自体の
  semantics は不変）。

## 受入条件

- `RuviformSlotContainer` が `style: 0` + `DefWindowProcW`
  で登録され、`CS_PARENTDC` が class style に存在しない
  ことを Windows test が検証する。
- `drain_paint` が `update_pending` の前後状態・dispatched
  数・cleared を JSON に記録し、bounded pump（cap 8）を
  超えない。
- rasterfix probe が各 arm の mutation op 結果・screen
  green・window-DC raster を note 級 step rows に記録し、
  `rasterfix_oracle` extras（`fix_arm`・閾値・stage 別
  green map）を emit する。
- `Pass` が screen green + executed + restored + lifecycle
  の連言でのみ出る（mutation 成功だけでは Pass しない）。
- restore が band/parent/exstyle/rect を検証済みで戻す。
- headless では fail-closed（`NotRun` +
  `environment_blocked`）、全 headless 検証 lane 通過。
- real lane で `real-dcomp-rasterfix` が実測 verdict を
  記録し、結果が `evidence/rf532/rasterfix-results.md` に
  集約される。
