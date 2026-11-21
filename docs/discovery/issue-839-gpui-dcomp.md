---
id: "DISC-034ZbbClipPart000000001"
type: "discovery"
status: "approved"
title: "Issue #839 GPUI fork clip-partition fix — 実測調査記録"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/839"
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-839-gpui-dcomp.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-532-rasterfix.md"
    - "docs/discovery/issue-532-child-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-839-gpui-dcomp.md"
    - "docs/designs/issue-839-gpui-dcomp.md"
    - "docs/test-specs/issue-839-gpui-dcomp.md"
    - "evidence/rf532/clipfix-results.md"
    - "evidence/rf532/clipfix-pixels/"
    - "evidence/rf532/pinned-dep-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-839-gpui-dcomp.md"
  derived-by: []
  verifies: []
  verified-by:
    - "evidence/rf532/clipfix-results.md"
metadata:
  issue: 839
---

# Issue #839 GPUI fork clip-partition — 実測調査記録

GPUI fork 側で docked child HWND raster を DirectComposition
経路に載せる根本 fix の、bisect 実実験から採否確定までの記録。
結論・証拠ファイルは `evidence/rf532/clipfix-results.md` に
集約。先行調査は `issue-532-rasterfix` / `issue-532-child-inventory`
slice の記録を継承する。

## 確定した事実

### 失敗は 2 層に分離する

- 層 1（raster 生成）: root window の `WS_EX_NOREDIRECTIONBITMAP`
  は HWND hierarchy 全体の redirection surface を否定する。
  flag ありでは child の window-DC raster 自体が **0.000** —
  合成以前に描画 backing が存在しない。flag を外すと
  window-DC raster は **1.000** に戻る。
- 層 2（presentation plane）: raster が存在しても
  screen green は **0.000** のまま — flip-model swapchain の
  presentation が child 領域を覆っている。非 topmost
  composition target（patch 1）だけでは解消しない。

### 棄却した経路（bisect 記録）

- **`CreateSurfaceFromHwnd` で child を visual に bind**:
  同 API の contract は *layered window* の rasterization を
  wrap するもの。非 layered child では空 surface（実測
  `E_INVALIDARG` / screen 0.000）。rebind-per-commit の
  snapshot 仮説・live-track 仮説ともに実測で棄却。
- **child を `WS_EX_LAYERED` 化**: `SetWindowLongPtrW` による
  exstyle 追加は owner thread からでも silent no-op、
  `WS_CHILD | WS_EX_LAYERED` の新規作成は `0x80070006` で
  失敗。cross-thread 親では作成自体が不可。`WS_POPUP` なら
  layered で描画されるが in-place hosting の要件を満たさない。
- **env-off + blit-model swapchain（`DXGI_SWAP_EFFECT_SEQUENTIAL`
  + `SCALING_STRETCH`）**: baseline screen **0.999994** —
  presentation plane 仮説の裏付けとしては決定的だが、
  flip-model による利点を捨てる製品 fix にはしない
  （検証用 variant として記録のみ）。

### 採用した機構（確定）

- content visual を clip すると、その範囲外では普通の
  非 layered child が正常に composite される（clip 実験で
  screen green **1.000**、child exstyle `0x0` のまま）。
- よって fix = **flag 除去（raster 復元）+ client 領域を
  child rect で引いた partition への swapchain visual 分割
  （plane の穴あけ）** の 2 要素。両者は独立に検証済みで、
  片方だけでは復元しない。

## 実装に反映した制約

- child の style/parentage を一切変更しない（in-place
  hosting の要件）。
- partition は `EnumChildWindows` + 可視判定 + `GetWindowRect`
  → `ScreenToClient` で毎回再計算し、変化時のみ visual を
  再構築して `Commit` + `DwmFlush`。
- sync は `present()` と 10ms `WM_TIMER` の両経路 —
  GPUI が frame を出さなくても attach 直後の child が
  遅れて uncover される。
- 再構築は state borrow 内のみで完結し、`RedrawWindow` 等の
  re-entrant API を持たない（borrow 中の `SendMessage` 由来の
  `RefCell already borrowed` panic は surface-bind 版で
  実際に踏んだ経緯あり）。

## Lane 実施記録

`mise run test-ui-plugin-dock-real`（canonical）+
`--pixel-artifacts` aux + `GPUI_DISABLE_DIRECT_COMPOSITION=1`
aux の 3 run。詳細と verdict 表は
`evidence/rf532/clipfix-results.md`。

## 再現情報（実験手順）

- gpui 側 commit: `scarecrow173/gpui` branch
  `rf839-dcomp-child-raster` rev
  `b3144f286b942fbb5f478da73db13a826cef054a`（本 doc 初版時点
  では local commit のみ — 後続 slice で tag `v0.2.2-rf2`
  として push 済み。ruviform 側は manifest tag pin +
  `Cargo.lock` で同 rev を解決、pinned dep 実測は
  `evidence/rf532/pinned-dep-results.md`）。
- ruviform worktree root `Cargo.toml` に uncommitted で
  追加した override:

  ```toml
  [patch."https://github.com/scarecrow173/gpui"]
  gpui = { path = "G:/repositry/audio-products/gpui" }
  ```

- 続けて `cargo update -p gpui` で `Cargo.lock` を実験用に
  差し替え（`cargo run --locked` の lockfile 整合のため）。
  両ファイルは commit 前に `git checkout` で main と一致
  させた。
- 測定環境: Windows interactive console session（real lane
  必須）、gpui 側単体検証は `G:/repositry/audio-products/gpui`
  で直接 `cargo check` / `cargo run --example`（一時的な
  standalone probe — 実測後に削除済み）。
