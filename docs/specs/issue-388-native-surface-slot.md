---
id: "SPEC-zrIXGa6QONDm4o1DVVUwaz"
type: "spec"
status: "approved"
title: "Issue #388 Driver-neutral NativeSurfaceSlot 契約（早期 Gate 設計・契約実装）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/262"
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
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-182-dock-tree.md"
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
  derived-by:
    - "docs/designs/issue-388-native-surface-slot.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 388
---

# Issue #388 Driver-neutral NativeSurfaceSlot 契約

## 背景

Issue #388（2026-11-07 横断レビュー確定仕様 v2）の**最優先早期採用
Gate**の契約側。GPUI Main Dock 内へ外部プラグインの実ネイティブ
Editor を収容する能力を、OS/format/L0-L1 別に実証する前段として、
Driver-neutral な `NativeSurfaceSlot` の型付き契約・opaque
generation-bound parent lease・headless 検証可能なモデルを確立する。

**Dock PASS の定義（固定）**: GPUI Main Window 内の実 Dock レイアウト
矩形と同じ clip/重なり/tab 可視性/resize/入力/focus 管理下で、
プラグイン自身の実ネイティブ Editor が動作すること。Floating 位置
合わせ・Generic Editor・Fake Surface・静止画像・remote screenshot
は PASS としない。

## スコープ

### In Scope

- `ruviform-ui` 新モジュール `native_slot`:
  `SurfaceSlotId`・`SlotLease`（opaque・generation-bound・
  lifetime-checked）・`SlotGeometry`/`SlotVisibility`/`SlotClip`/
  `SlotCapabilities`/`SlotError`/`SlotEvent`/`SlotSpec`・
  `SurfaceSlotRegistry`（mint/validate/revoke の純粋モデル）・
  `NativeSurfaceSlot` port trait（Driver 実装点）。
- 旧 `NativeSurfaceHost`（render.rs、WebView 隣接前提）からの移行
  写像表。`NativeSurfaceHost` 自体は存置（旧 WebView 経路は縮退
  判断まで保持）。
- headless 契約テスト（`crates/ruviform-ui/tests/native_slot.rs`）:
  lease の mint/validate/revoke/stale/double-issue/busy/teardown/
  generation recycle。
- OS/format/L0-L1 事前評価マトリクス（行省略なし・Pass/Fail/
  Unsupported/NotRun/Blocked 区別・実機未実施は NotRun）。

### Out of Scope

- GPUI/`ruviform-ui-gpui` の実 Driver 実装（RF386 所有・最小 Window
  fixture 到着待ち = 本 dispatch の実機行 Blocked）。
- 実プラグイン binary での Dock 実証（adapter `open_gui` 未実装 +
  fixture 未入手 → NotRun）。
- CLAP/VST3 bridge の `open_gui` 実装本体（別 Issue 級）。
- 旧 WebView/SolidJS/Canvas 成果の削除・`NativeSurfaceHost` 削除。

## 受入条件

- [ ] `native_slot` が GPUI/DOM/wry/tao/OS handle を含まない
  driver-neutral 公開 API で、`cargo check -p ruviform-ui` が
  headless で通る。
- [ ] `SlotLease` は caller が中身を解釈しない opaque 値であり、
  `SurfaceSlotRegistry` 以外が有効な lease を mint できない
  （generation/seq/lifetime を registry が検査）。
- [ ] stale generation・slot release 後の lease・未発行 lease・
  double-lease busy・release 後再発行の世代更新が headless 検証済み。
- [ ] `NativeSurfaceSlot` port が attach/geometry/visibility/clip/
  z-order/focus/lease の Driver 語彙を持ち、`HWND`/`NSView`/X11/
  GPUI 型を露出しない。
- [ ] 2-phase Dock move（issue-182 `DockWorkspace`）との接続点
  （slot の View 所有・move 時の detach/reattach・abort/rollback で
  元 slot/lease を回復）が契約として記述されている。
- [ ] マトリクス全行が Pass/Fail/Unsupported/NotRun/Blocked の
  いずれか + 根拠で記録され、未試験が PASS に繰り上がっていない。
- [ ] `mise run fmt-check`/`check`/`lint`/`test` が green。

## 制約・禁止

- 公開 API に `gpui`/`wry`/`tao`/`HWND`/`NSView`/X11 Window/GPU
  handle を入れない（#384 依存方向）。`SurfaceRegion` 等の既存
  backend-neutral 型は再利用可。
- plugin 系 ↔ ui 系の相互依存禁止（baseline 確定規則）。
- Fake/headless の green を実 Dock PASS にしない。
- `unsafe` 原則禁止（暫定規約）。全公開 enum `#[non_exhaustive]`。
- 新規外部依存なし。

## 関連

- 実装入力: `docs/discovery/issue-388-389-native-dock-current-state.md`
- 共同検証: `docs/specs/issue-389-plugin-editor-session.md`（session
  が lease を消費する契約）
- 再利用: `docs/designs/issue-182-dock-tree.md`（2-phase move）、
  `docs/designs/issue-181-multi-window.md`（WindowId）
- Gate 接続: Issue #262（採否 Owner）、Synvelis #176（製品 E2E）
