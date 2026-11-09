---
id: "SPEC-rf388DockWindowing01"
type: "spec"
status: "approved"
title: "Issue #388 NativeSurfaceSlot slice 3 — Windows/VST3/L0 ancestor clip・keyboard/focus routing・re-dock/float 2-phase の実測仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
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
    - "docs/test-specs/issue-388-native-dock-windowing-slice3.md"
    - "docs/designs/issue-388-native-surface-slot.md"
    - "evidence/rf388-native-dock-windowing/report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-388-native-surface-slot.md"
  derived-by:
    - "docs/designs/issue-388-native-dock-windowing-slice3.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  defers: []
  deferred-by: []
metadata:
  issue: 388
---

# Issue #388 NativeSurfaceSlot slice 3 — windowing 挙動の実測仕様

## 背景

Issue #388 エピックの slice 2（PR #438）で GPUI Panel/Tab 内への
VST3 editor attach・tab hide/show・resize 追従・container focus・
close は Windows/VST3/L0 で実測済みだが、capability matrix の
ancestor clip・keyboard/focus routing・re-dock/float 行は `NotRun`
のまま残っている。本 slice はそれら 3 領域を実 plugin（Vinyl
1.12.0）で実測し、証拠つきで matrix を更新する。

## スコープ

### In Scope（Windows / VST3 / L0 のみ）

- **ancestor clip / overhang**: slot projection が ancestor
  content mask と要素 bounds の積を clip として port に投影する
  こと、および `set_clip` → `SetWindowRgn` が実 HWND に効くことの
  実測。window 縮小による viewport overhang（`min_size` 強制）と、
  port 直叩きの部分 clip の 2 経路で確認する。
- **keyboard / focus routing**: container focus 時にキーが plugin
  editor HWND に届かず container に留まること、plugin editor が
  focus を持つときキーが plugin 側 HWND に届くこと、の実入力
  （`keybd_event`/`mouse_event`）+ `WH_GETMESSAGE` 観測による実測。
  IME・popup は `NotRun` のまま。
- **re-dock / float 2-phase**: `DockWorkspace` の
  `begin_move`/`commit_move`（MovePrepared → busy → MoveCommitted）
  と `detach_view`/`attach_view`（detached pool = float 候補）を
  実モデルで駆動し、GUI 側は `update_view` で NativeSlot を
  抑制/復帰させて、slot・container・editor HWND が detach 中も
  生存し再 attach で同一 HWND が再表示されることの実測。
  busy 中の `lease_parent` 拒否（`SlotBusy`）も実 adapter で確認。

### Out of Scope

- macOS / Linux X11・Wayland、AU / CLAP、L1 isolation（Unsupported
  維持）。plugin 側 resize（`onSize` 経路・Unsupported 維持）、
  DPI/複数モニタ、2 window、crash/hang 注入、swapchain z-order。
- view 語彙への clipping container（overflow_hidden 等）の追加 —
  本 slice では viewport overhang と port 直 clip で実測し、
  中間 ancestor clip は将来課題として記録する。
- Issue #388 の Dock PASS 判定そのもの — 本 slice の結果をもって
  coordinator が判定する。

## 受入条件

1. `mise run poc-native-dock-windowing`（新設 manual lane）が
   Windows 11 + Vinyl.vst3 で完走し、`[poc] check=` 行が証拠になる。
2. 実測できた行だけ capability matrix（design doc D6）を更新し、
   未実測は `NotRun`/`Unsupported` を維持する。
3. `evidence/rf388-native-dock-windowing/report.md` に OS・plugin
   version・実行日時・経路・各 check 結果を記録する。
