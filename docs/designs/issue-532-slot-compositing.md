---
id: "DESIGN-rf532sltcmp0stn1t3rp"
type: "design"
status: "proposed"
title: "Issue #532 スロット合成 — fork-free PoC 機構と Option-A GPUI fork 設計"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added:
      - type: "file"
        path: "evidence/rf532/poc-interposition-results.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
---

# Issue #532 スロット合成 — PoC 機構と Option-A fork 設計

本ドキュメントは PoC スライスで実装した fork-free 介入機構と、判定結果として
採用される Option-A（GPUI fork）の設計指針を記述する。実測結果と判定は
`docs/discovery/issue-532-slot-compositing.md` /
`evidence/rf532/poc-interposition-results.md` を参照。

## 実装済み PoC 機構（opt-in、既定無効）

### `HoleMode`（surface.rs）

`RVF_SLOT_HOLE` env で `none|region|alpha|both` を選択。既定は `None` で、
実験機構は完全に opt-in — 既定挙動は baseline と同一（canonical run で確認済）。

### Region hole（surface.rs, Windows のみ）

- `hole_rect_of`: 可視・live スロットの geometry（clip 指定時は clip）を
  slot の scale factor で物理ピクセルへ写像し、ゼロ面積を除外。
- `sync_surface_regions`: suppression パス末尾（render.rs、
  `hole_mode().region()` 時のみ呼出し）で全オーナーウィンドウを再調整。
  `WINDOWINFO` からクライアントオフセット・フレーム寸法を取得し、
  全フレーム HRGN からスロット矩形を `CombineRgn(..., RGN_DIFF)` で除外して
  `SetWindowRgn` 適用。`AppliedRegion` 台帳（offset/frame/整列済み rect）で
  同一形状の再適用を抑止し、適用失敗は次フレームでリトライ。
  可視 hole が無くなれば `SetWindowRgn(hwnd, NULL)` で既定復帰。
- OS 呼び出しを跨ぐ `RefCell` 借用は保持しない（同期メッセージ配送の回避）。
- 非 Windows は no-op。決定論的テスト: `region_hole_apply_and_clear` /
  `sync_surface_regions_applies_and_restores_parent_hole`。

### Alpha hole（render.rs）

`hole_mode().alpha()` 時: bound スロットの `.bg(surface_raised)` と
プレースホルダ `paint_quad` を省略し、`subtree_has_bound_slot` が true の
祖先 Panel の `.bg` も省略（PoC 近似 — 製品版は quad 分割 carve が必要）。
`sync_surface_regions` の policy 判定は render.rs に置き、adapter 側は
呼ばれれば常に再調整する機械的実装とした（責務分離: policy=renderer、
mechanism=adapter）。

## Option-A 設計指針（本スライスでは実装しない）

1. **変更点は 1 箇所**: `gpui` crate の `directx_renderer.rs` で
   `CreateTargetForHwnd(hwnd, true)` → `false`。crate 内の該当呼出しは
   この 1 箇所のみ（`gpui-public-surface-audit.md`）。
2. **影響**: 注入ビジュアルがウィンドウコンテンツ＝子 HWND 群の下に置かれる。
   半透明 GPUI コンテンツが子の上に重なる場合の描画は GPUI 自身の
   premultiplied alpha で正しく合成されるはず（別途実測）。
3. **fork 方法の選択肢**（要決定）: vendor + `[patch]`、git dep fork、
   または上流への修正提案。`gpui = "0.2.2"` の version pin 戦略との整合が課題。
4. **回帰面**: non-topmost 化で GPUI コンテンツが子に隠れるケース
   （オーバーレイ UI、IME 候補窓、メニュー等）は real lane の
   `real-window-ime` 等で要検証。
5. **下流伝播**: `scarecrow173/synvelis#176` が依存対象。

## 残課題

- Option-A 実施スライス（fork 取得・patch 適用・real lane 再検証）。
- `RVF_SLOT_HOLE` 機構の去留: 既定 `None` で無害だが、Option-A 成功後に
  撤去するか診断用に残すかを follow-up で決める。
