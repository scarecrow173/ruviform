---
id: "DES-034ZaaRasterfix0000001"
type: "design"
status: "approved"
title: "Issue #532 raster-fix oracle + treatment arms — 設計"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-rasterfix.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-532-rasterfix.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-532-child-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-rasterfix.md"
    - "docs/discovery/issue-532-rasterfix.md"
    - "docs/test-specs/issue-532-rasterfix.md"
    - "evidence/rf532/rasterfix-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-rasterfix.md"
  derived-by: []
  verifies: []
  verified-by: []
metadata:
  issue: 532
---

# Issue #532 raster-fix — 設計

## 構成

### 1. Private container class（`surface.rs`）

GPUI slot container を system `STATIC` class（実測
`GCL_STYLE = 0x4088` = `CS_GLOBALCLASS|CS_PARENTDC|
CS_DBLCLKS`）から、process 内 private class
`RuviformSlotContainer`（`style: 0`、`lpfnWndProc =
DefWindowProcW`）へ置換する。

- 登録は `std::sync::Once` で process 1 回。`RegisterClassW`
  失敗時は `GetClassInfoW` で既登録を確認してから
  「already exists」を受理し、未確認の失敗は
  `SlotError::Backend` で fail する（`STATIC` fallback
  なし — class は fix contract の一部であり、黙って
  治療対象を外さない）。
- `GetModuleHandleW` の process module handle を
  `CLASS_HINSTANCE` に保持し `CreateWindowExW` へ渡す。
- `DefWindowProcW` 採用は VST3 test editor
  （`editor_view.rs`）の確立済み pattern — DLL unload で
  dangling する module-local wndproc を置かない。
- test-only top-level parent は引き続き `STATIC` を使う
  （production container のみが治療対象）。

### 2. Paint drain（`osmut::drain_paint`）

`update_pending=true` 仮説の判別用。`UpdateWindow`（同期
paint）の後、caller thread の message queue から
`PeekMessageW(hwnd, WM_PAINT, WM_PAINT, PM_REMOVE)` +
`DispatchMessageW` を bounded loop（`PAINT_PUMP_CAP = 8`、
空 read で即終了）する。`update_before`/`ops`/`dispatched`/
`update_after`/`cleared` を `PaintDrainOutcome` に記録。
非 Windows stub は `supported: false`。

### 3. `real-dcomp-rasterfix` probe（`real_dock.rs`）

`attempt_real_attach` 内、`session.open(Docked)` 成功後の
live lease 中に `rasterfix_probe_children` を走らせる。
対象は `observation.children[0]`（fixture の単一 editor
child、caller thread 所有）。

arm 順序 — 各 arm は前 arm の mutation 状態を引き継ぐ:

1. `rf_baseline_state` + baseline `rf_screen`。
   `>= GREEN_VISIBLE_FRACTION(0.5)` なら `fix_arm =
   "baseline"` で早期終了（治療単独で到達 = 最有力 fix）。
2. `pump` — `drain_paint` → `rf_raster` + `rf_settle` +
   `rf_screen`。閾値達なら `fix_arm = "pump"`。
3. `reparent` — child baseline の parent/exstyle/rect が
   読めなければ arm を skip して probe 未完了扱い
   （`executed=false` + failure — 全 arm が走ったとは
   主張しない）。`reparent_to_desktop` の 4 op
   （baseline exstyle write → frame apply → `SetParent` →
   `HWND_TOP` rect seat）に加えて
   **`osmut::set_topmost(child.hwnd)`** を発行:
   desktop reparent した `WS_CHILD` window は desktop 子
   z-list の child band に留まり `HWND_TOP` では全
   top-level sibling より下に残る（実測 37）ため、
   topmost band への昇格なしでは screen oracle が無関係な
   z-order を測ることになる。次に `drain_paint` +
   raster + screen。
4. `layered` — reparent で未達のときのみ。now-top-level
   child に `apply_layered`（exstyle OR + frame + LWA_ALPHA
   255）→ 同 evidence。
5. `rf_restore` — `restore_container` に verified
   parent/exstyle/rect を要求。`restored` は
   verification（`parent_restored && exstyle_restored &&
   rect_restored`）+ 全 restore op applied の連言。
   sibling z-order は best-effort で記録のみ。

### 4. verdict gating

`RasterfixVerification` = supported / lifecycle_ok /
lifecycle_failure / probe（evaluated/executed/restored/
failure/fix_arm/greens）/ extras。

- `!supported` → `Unsupported`
- `!lifecycle_ok` → `Fail`（lifecycle mismatch）
- `!probe.evaluated` → `NotRun`（baseline 不能）
- `!probe.executed || !probe.restored` → `Fail`
  （probe 未完了 — restore 不能な「fix」は主張しない）
- `fix_arm = Some(arm)` → `Pass`（detail + evidence
  `dcomp-rasterfix:<arm>-screen-green`）
- それ以外（arms 完走・restore 済み・green 非達）→ `Fail`
  （仮説棄却 — ruviform 側治療は足りない、残る容疑は
  GPUI 自身の visual-tree presentation）

### 5. `osmut` restore/mutation の実測駆動変更

実測で判明した Windows 制約に合わせた、共有 machinery
（`be_`/`ci_`/`rf_` 3 probe 共通）の修正:

- `restore_container` op 順を **band → exstyle+frame →
  `SetParent` → rect → sibling z-order** に変更。
  `GWL_EXSTYLE` write は対象 window の parent が別 thread
  所有だと win32 error 6 で拒否され（container は GPUI
  thread 所有）、desktop-parented の間だけ確実に成功する。
  style/band の復元は re-dock **前**に行う。
- `band_restore_op` 新設: baseline band と current band が
  異なるとき `SetWindowPos(HWND_NOTOPMOST/TOPMOST)` を
  発行。`WS_EX_TOPMOST` は `GWL_EXSTYLE` write では解除
  不能（error なしに drop される）ため band API が必要。
- `exstyle_write` の `applied` を「read-back が要求値に
  一致」で判定するよう変更。API 成功値だけでは silent
  drop（戻り値=成功、状態不変）を見逃し、refused no-op
  write（要求値=現値、error 6）を誤って失敗扱いする。
  API 拒否は `win32_error` として証跡に残る。

## 非対称な扱い

`fix_arm` が verdict 入力になるのは本 case のみ。
`be_`/`ci_` 診断 case は従来どおり実行/復元の完遂を
verdict とし、green を判定に使わない — 診断 case の
semantics 後方互換を保つ。
