---
id: "DISC-034ZaaRasterfix0000001"
type: "discovery"
status: "approved"
title: "Issue #532 raster-fix oracle — 実測調査記録"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-rasterfix.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-532-child-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-rasterfix.md"
    - "docs/designs/issue-532-rasterfix.md"
    - "docs/test-specs/issue-532-rasterfix.md"
    - "evidence/rf532/rasterfix-results.md"
    - "evidence/rf532/rasterfix-pixels/"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-rasterfix.md"
  derived-by: []
  verifies: []
  verified-by:
    - "evidence/rf532/rasterfix-results.md"
metadata:
  issue: 532
---

# Issue #532 raster-fix — 実測調査記録

`real-dcomp-rasterfix` case の 3 回 real-window run（DComp on
canonical + DComp off aux + pixel-artifacts aux）で確定した
事実の記録。結論・証拠ファイルは
`evidence/rf532/rasterfix-results.md` に集約。

## 確定した事実

### raster/screen の分離結果

- baseline（docked、private class `RuviformSlotContainer`
  適用済み）: screen green 0.000 — **private class 単独では
  治らない。`CS_PARENTDC` は原因ではなかった**。
- pump（`UpdateWindow` + bounded `WM_PAINT` drain）:
  `update_before=true → update_after=false` で pending paint
  は確かに解消されるが screen 0.000 のまま —
  **caller-thread paint 欠落も原因ではない**。
  DComp off では同 arm の window-DC raster が 1.000 に
  なる（backing は即座に読める）。
- reparent（desktop reparent + `HWND_TOPMOST`）:
  window-DC raster 1.000 + **screen green 1.000** —
  child の raster は GPUI presentation plane の外に出せば
  完全に合成される。
- layered arm は reparent 達成により未到達（両 run とも）。

### Windows windowing の新規測定事実

- `SetWindowLongPtrW(GWL_EXSTYLE)` は対象 window の parent
  が**別 thread 所有**のとき win32 error 6 で拒否される
  （container = GPUI thread 所有、child = caller thread
  所有の構成で再現）。同 window が desktop-parented の
  間は write が成功する。restore の exstyle write は
  `SetParent` 戻し**前**に行う必要がある。
- desktop reparent した `WS_CHILD` style window は desktop
  子 z-list の **child band** に留まる。`SetWindowPos
  (HWND_TOP)` はその band 内でのみ先頭に上げ、全
  top-level sibling（実測 37 個）より下に残る。
  `WindowFromPoint` は依然として上の container（GPUI
  root 配下）を返す。可視には `HWND_TOPMOST` への
  topmost-band 昇格が必要（実測 above_count 9 まで縮む）。
- `WS_EX_TOPMOST` bit は `GWL_EXSTYLE` write では解除
  **できない**: `0x8 → 0` の write が error なしに
  silently drop され bit が残る。解除は
  `SetWindowPos(HWND_NOTOPMOST)` のみ有効。
- `SetWindowLongPtrW` は「API 成功 + 状態不変」の silent
  drop を起こす（上記と同様の現象）。mutation の
  `applied` は API 戻り値ではなく read-back 一致で判定
  する必要がある（拒否は `win32_error` に記録）。

### z-order の非決定性

`HWND_TOP` のみで reparent した run と `HWND_TOPMOST` を
加えた run で screen oracle が 1.000 ↔ 0.000 に揺れた
（初回 run は child band の偶然上位で 1.000、次 run は
GPUI root に覆われて 0.000）。topmost-band 昇格は
z-order 競争を oracle から除くための決定性手段であり、
「raster が合成されるか」という計測対象を変えない。

## 結論

ruviform 側だけで docked-in-place の container を直す
治療（private class / paint drain / その場 layered）は
全て棄却された。一方で editor を GPUI visual tree 外の
top-level presentation 経路に逃がせば child raster は
user-visible になることが pixel oracle で証明された。
docked-in-place を真正に直すには GPUI 側で child raster を
composition 経路に載せる必要があり、gpui-fork
workstream の根拠となる。
