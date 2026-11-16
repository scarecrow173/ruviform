---
id: "DISC-034YnFUCO4ivyDKz4EYxVt"
type: "discovery"
status: "approved"
title: "Issue #532 dcomp-occlusion discovery — 遮蔽層診断の探索結果"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-dcomp-occlusion.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/designs/issue-532-dcomp-occlusion.md"
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
    - "evidence/rf631/verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/designs/issue-532-dcomp-occlusion.md"
  derived-by:
    - "docs/plans/issue-532-dcomp-occlusion.md"
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
    - "docs/tasks/issue-532-dcomp-occlusion.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 dcomp-occlusion discovery — 遮蔽層診断の探索結果

## Summary

`real-dcomp-pixel` の画素が screen 合成に出ない原因を HWND 層
（sibling z-order・`WS_EX_NOREDIRECTIONBITMAP` の実効確認）と
合成器層（DComp visual / flip-model 提示 plane）に二分するため、
query-only の Win32 診断（root exstyle・z-order 走査・
`WindowFromPoint`）を既存 `ospix`/`oschild` 規約に載せられるか
を探索した。対象 API は全て既存 `windows-sys` optional edge の
feature 内にあり、依存追加なしで実装可能と確認。window 階層
（GPUI root → container → plugin child）の前提も実装で裏付けた。

## Exploration Goal

Issue #532 の遮蔽がどの層で起きるかを query-only 観測で切り分け
るため、(a) どの Win32 API が必要か、(b) それらが既存依存で
賄えるか、(c) 既存 `ospix`/`oschild`/`real_dock` の構造にどう
載せるか、(d) `WindowFromPoint` の GPU 合成下での limitation を
どう記録するか、を確定する。

## Key Issues and Assumptions

- `WS_EX_NOREDIRECTIONBITMAP` は GPUI `window.rs` で
  `!disable_direct_composition` のときのみ root HWND に付与
  される（`evidence/rf532/gpui-public-surface-audit.md` 確定）
  — exstyle flag の有無が env var 実効の run ごとの証拠になる。
  **confirmed**（audit + vendored source 確認）。
- `WindowFromPoint` は WM の z-order/visibility のみを見る —
  DComp visual や flip-model 提示 plane は HWND ではないため
  hit-test に現れない。child が返るのに screen = 0.0 なら合成器
  レベル遮蔽の傍証だが、GPU 合成下での決定性は未検証。
  **hypothesis** — 実機観測で確認。
- window 階層: GPUI window（top-level）→ container HWND
  （`STATIC` class、`WS_CHILD|WS_VISIBLE|WS_CLIPCHILDREN|
  WS_CLIPSIBLINGS`、`GpuiSurfaceAdapter` backend 作成）→
  plugin child HWND（`test_editor` の `IPlugView`）。
  `GetAncestor(child, GA_ROOT)` は GPUI window に到達
  （`ospix::presented` が既に同じ walk を使用）。**confirmed**
  （実装確認済み）。
- `EnumChildWindows` は全子孫を返し順序保証がないため z-order
  には使わず、`GetWindow(GW_CHILD)` → `GW_HWNDNEXT` 反復で
  direct children の z-order（先頭が topmost）を取る。
  **confirmed**（Win32 仕様）。
- `GetWindowTextW` は cross-process で message を伴いうるため
  使わず、`GetClassNameW`（内部 class atom の直接読み）に留める
  — 厳密な query-only 境界。**confirmed**（設計判断）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 新規 `osoccl.rs` module | `oschild`/`ospix` と同じ境界規約・単一責務・test-only | module 数 +1 | yes |
| `ospix.rs` 拡張 | 変更ファイル数最小 | pixel capture と遮蔽診断の責務混在 | no |
| `EnumChildWindows` z-order | 1 call で全子孫 | 順序保証なし・孫まで含む | no |
| `RealChildWindowFromPoint`/`ChildWindowFromPoint` | client 座標系の hit-test | 追加 API・本 slice の二分には `WindowFromPoint` で十分 | maybe（deferred） |

## Tentative Conclusions and Hypotheses

- **confirmed**: 対象 API（`GetAncestor`/`GetWindowLongPtrW`/
  `GetWindow`/`GetParent`/`GetClassNameW`/`GetWindowRect`/
  `IsWindow`/`IsWindowVisible`/`WindowFromPoint`・
  `GWL_EXSTYLE`/`WS_EX_NOREDIRECTIONBITMAP`/`GW_CHILD`/
  `GW_HWNDNEXT`/`GA_ROOT`）は全て `windows-sys 0.61` の
  `Win32_UI_WindowsAndMessaging` + `Win32_Foundation`（`POINT`）
  feature 内に存在 — 依存追加なし。
- **confirmed**: 診断は note 級 step row として `verdict`・
  `PixelScan`・`PixelObservation`・case verdict に非干渉で
  載せられる（`observe_one_child_pixels` の既存構造）。
- **hypothesis**: env-on（`GPUI_DISABLE_DIRECT_COMPOSITION=1`）
  で root exstyle の `WS_EX_NOREDIRECTIONBITMAP` が外れれば
  env var が GPUI 初期化に実効した証拠になる — 観測で確認。
- **hypothesis**: `WindowFromPoint` が child rect 内で
  `editor_child` を返し screen = 0.0 なら遮蔽は合成器レベル
  （DComp visual / flip-model 提示 plane）— ただし
  `WindowFromPoint` の GPU 合成下での決定性は未検証、limitation
  として記録。

## Open Questions

- `WindowFromPoint` が DComp/flip-model 合成下で child を返すか
  — 実機依存、観測結果をそのまま記録。二分不能なら
  「unresolved」と正直に記述。
- env-on で `WS_EX_NOREDIRECTIONBITMAP` が外れるか — GPUI の
  env var 受理値（`"1"`/`"true"`）に依存、観測で確認。
- GPUI が内部で container 以外の子 HWND を持つか — 観測して
  初めて分かる、列挙結果はありのまま記録。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-532-dcomp-occlusion.md`
  （作成済み・approved）。
- [x] adr-doc needed: なし — 方式採否は後続 slice。

## Sources and Relations

- Issue `scarecrow173/ruviform#532`（OPEN 継続）。関連:
  `#388`・`#630`・`#631`・`scarecrow173/synvelis#176`。
- `crates/ruviform-plugin-dock-test/src/osoccl.rs`（新規 —
  設計上の配置、実装は plan/task で確定）。
- `crates/ruviform-plugin-dock-test/src/oschild.rs`・
  `src/ospix.rs`（query-only 規約の先例）。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`（
  `observe_one_child_pixels` への配線先）。
- `evidence/rf532/gpui-public-surface-audit.md`（env var 意味論・
  `WS_EX_NOREDIRECTIONBITMAP` 分岐・非 DComp swapchain）。
- `evidence/rf631/verification.md`（前回 slice の baseline/env-on
  実機記録）。
- `docs/discovery/issue-532-gpui-dcomp-surface.md` F5（exit-127
  仮説 H1-H5）。

## Evidence

- 2026-11-16: `windows-sys 0.61` installed source で対象 API の
  存在を確認（`Win32_UI_WindowsAndMessaging` feature 内）。
- 2026-11-16: `oschild.rs`/`ospix.rs`/`real_dock.rs` の構造を
  読み、`observe_one_child_pixels` が note step row を既に
  出していること・`oschild::observe_children` が container の
  child 列挙に `GW_CHILD`/`GW_HWNDNEXT` を既に使用していることを
  確認。
- 2026-11-16: `GpuiSurfaceAdapter` backend の container HWND
  作成（`STATIC` class・`WS_CHILD|WS_VISIBLE|WS_CLIPCHILDREN|
  WS_CLIPSIBLINGS`）と `lease_parent` 経路を確認。
