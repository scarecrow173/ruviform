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
      - type: "file"
        path: "evidence/rf532/option-a-results.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
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

## 実装済み PoC 機構（opt-in、既定無効） — Option-A スライスで撤去済み

> 以下の機構は Option-A スライスで全撤去した（`RVF_SLOT_HOLE` env・
> `HoleMode`・`AppliedRegion`・`sync_surface_regions`・`subtree_has_bound_slot`
> ・alpha 抑制・決定論テスト 2 件）。`NativeSurfaceSlot` 契約と `set_clip`
> 等の既存 slot 機能は維持。設計記録として本節を残置する。

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

## Option-A 設計指針（実施済み — 結果は fail、Issue open）

実施形態と結果を以下に記録する（実測詳細:
`evidence/rf532/option-a-results.md`）。

1. **変更点は 1 箇所**: `gpui` crate の `directx_renderer.rs` で
   `CreateTargetForHwnd(hwnd, true)` → `false`。crate 内の該当呼出しは
   この 1 箇所のみ（`gpui-public-surface-audit.md`）。
2. **影響**: 注入ビジュアルがウィンドウコンテンツ＝子 HWND 群の下に置かれる。
   半透明 GPUI コンテンツが子の上に重なる場合の描画は GPUI 自身の
   premultiplied alpha で正しく合成されるはず（別途実測）。
3. **fork 方法**: git dep fork を採用（`[patch.crates-io]` は consumer
   workspace に伝播しないため不採用）。`gpui = { git =
   "https://github.com/scarecrow173/gpui", tag = "v0.2.2-rf1",
   version = "=0.2.2", optional = true }` — crate 名・`=0.2.2` version
   制約を維持し、`Cargo.lock` を git ソース（commit `f77e44e6`）へ更新。
4. **回帰面**: non-topmost 化で GPUI コンテンツが子に隠れるケース
   （オーバーレイ UI、IME 候補窓、メニュー等）は real lane の
   `real-window-ime` 等で要検証 → 実走では `real-window-ime` は
   unsupported のまま、`real-dcomp-present` 等 presentation 系ケースは
   全て pass で回帰なし。
5. **下流伝播**: `scarecrow173/synvelis#176` が依存対象。

## 実施結果と残課題

- **`real-dcomp-pixel` は引き続き fail**。verdict 構成・topmost ステージの
  画面 capture SHA256 が baseline と完全一致し、注入ビジュアルの
  z-order 変更は最終合成に観測可能な差を生じなかった。遮蔽主体は
  注入ビジュアルではなく presentation plane 自体（swapchain の
  合成/flip 経路）と見るのが全証拠と整合 — Option-A の前提は実測で否定。
- `RVF_SLOT_HOLE` 機構は本スライスで撤去済（失効した PoC 機構を残さない
  方針に従う）。
- 残課題（次スライス候補）: swapchain 生成/present 経路の遮蔽機構の特定
  （independent flip / MPO 相当の plane が子 HWND を覆う仮説）、
  slot を別 top-level（owned/layered）ウィンドウとして追従配置する
  airspace 回避方式の評価。Issue #532 は open のまま。
