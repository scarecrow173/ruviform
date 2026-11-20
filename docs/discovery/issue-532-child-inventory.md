---
id: "DISC-034ZaaChildInvDiagnosis01"
type: "discovery"
status: "approved"
title: "Issue #532 child-level inventory + capture-plane 診断 — 調査記録"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-child-inventory.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-532-band-escape.md"
    - "docs/discovery/issue-532-uncovered-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-child-inventory.md"
    - "docs/designs/issue-532-child-inventory.md"
    - "docs/test-specs/issue-532-child-inventory.md"
    - "evidence/rf532/child-inventory-results.md"
    - "evidence/rf532/band-escape-results.md"
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

# Issue #532 child-level inventory + capture-plane 診断 — 調査記録

## 調査目的

Issue #532（GPUI 0.2.2 DirectComposition/presentation-plane
composition 下で native child HWND plugin の pixel が最終画面に
現れない）に対し、band-escape slice の残課題として示された
**child-level/capture-plane 診断**を実走で確定する:

1. docked child HWND 自身の `GWL_STYLE`/`GWL_EXSTYLE`/class/
   `GCL_STYLE`/所有 thread+process/parent の inventory。
2. editor-host class `0x4088` の `CS_PARENTDC` 有無の実測。
3. child raster（`PrintWindow` flag 変種・window DC）と
   screen-composited pixel の分離。
4. `GPUI_DISABLE_DIRECT_COMPOSITION=1` が fork の DComp path を
   実際に off にするかの検証（過去 run は env 有効性未検証）。
5. child-level `WS_EX_LAYERED` → top-level reparent retry で
   cross-thread 説と child-state 制約説の分離。

本 slice は診断であり、修正でも remediation 方式の決裁でも
ない。**Issue #532 は open のまま**。

## 前提となる確定事実（前 slice の蓄積）

- container reparent/layered は pixel 欠損を変えない
  （`band-escape-results.md`）— container の presentation plane や
  parentage は阻害原因でない。
- container-level `GWL_EXSTYLE` write は silent drop
  （read-back 不保持）、`SetLayeredWindowAttributes` error 87。
- uncovered-baseline で外部 HWND occlusion・settle 不足は棄却。
- container class_style は `0x4088`（`CS_PARENTDC` 含有予想）、
  child 側 style/exstyle は未採取だった。

## 実走結果（2026-11-21、interactive Windows x86_64）

詳細は `evidence/rf532/child-inventory-results.md`。要点:

- **inventory**: child = `RuviformTestEditor`、style
  `0x54000000`（WS_CHILD|WS_VISIBLE|WS_CLIPSIBLINGS）、
  exstyle `0`、class_style `0x3`（CS_HREDRAW|CS_VREDRAW）、
  caller thread 所有。container = `Static` class、`0x4088`
  （**CS_PARENTDC 含有を実測確定**）、WS_CLIPCHILDREN 付き、
  GPUI thread 所有。root = `Zed::Window`、exstyle
  `0x240000`（WS_EX_NOREDIRECTIONBITMAP|WS_EX_APPWINDOW）。
- **capture 分離**: `PrintWindow(0)`/`PW_CLIENTONLY` = green
  1.000、`PW_RENDERFULLCONTENT`（DWM 実表面）= 0.000、
  window DC = DComp-on 0.000 / env-off 1.000、screen = 両経路
  0.000。green は synthesized `WM_PRINT` re-render にのみ存在
  し、real backing raster には DComp-on では存在しない。
- **env arm（verified）**: env=1 で root から
  `WS_EX_NOREDIRECTIONBITMAP` が消失 → `dcomp_disabled_by_env`。
  fork の env 分岐は機能するが、swap-chain present は両経路で
  走るため screen green は不変。
- **layered 残余実験**: same-thread child で
  `GWL_EXSTYLE | WS_EX_LAYERED` は silent refusal（error null）、
  LWA error 87、baseline write error 6 — しかし
  `SetParent(desktop)` は適用し、top-level 化後の retry は
  `layered: true` で latch。**exstyle write 制約は
  child-window-state 由来で cross-thread 説は棄却**。
- restore verify: parent/exstyle/rect 全 restored（baseline
  exstyle write op の err 6 報告のみで `ops_ok=false`、
  final state は一致）。

## 確定した判別

- 阻害は **2 層に分離**: (a) DComp-on では child に real
  backing raster が無い（window DC 0.0、WM_PRINT re-render
  のみ green）、(b) raster が存在する env-off でも screen
  green 0.0 — GPUI swap-chain present が child 領域を覆う。
- `CS_PARENTDC`（container `0x4088`）含有は実測確定。ただし
  capture-plane 阻害との因果は未証明 — measured fact と
  hypothesis を分離して記録した。
- `GPUI_DISABLE_DIRECT_COMPOSITION` は fork で有効 — root
  exstyle bit の有無が DComp path の正直な proxy として
  使える（`inferred_dcomp_path` 記録）。

## 残課題（次 slice へ引き継ぐ）

- GPUI fork 内の DComp visual / swap-chain present と child
  HWND の組成順（visual clipping・`CreateTargetForHwnd`
  topmost 引数）の観測 — `CS_PARENTDC` 因果を含む fork 側
  診断。本 slice は fork 非変更の制約上ここまで。
- `update_pending = true` 持続（caller thread 非 pump）が
  raster 未供給の共犯かは未分離 — pump する caller での
  再現は残課題。
