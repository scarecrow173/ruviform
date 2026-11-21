---
id: "DES-034ZbbClipPart00000001"
type: "design"
status: "approved"
title: "Issue #839 GPUI fork clip-partition fix — 設計"
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
        path: "docs/designs/issue-839-gpui-dcomp.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-839-gpui-dcomp.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-532-rasterfix.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-839-gpui-dcomp.md"
    - "docs/discovery/issue-839-gpui-dcomp.md"
    - "docs/test-specs/issue-839-gpui-dcomp.md"
    - "evidence/rf532/clipfix-results.md"
    - "evidence/rf532/pinned-dep-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-839-gpui-dcomp.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 839
---

# Issue #839 clip-partition — 設計

実装 commit: gpui fork `rf839-dcomp-child-raster` branch
`b3144f2`（`G:\repositry\audio-products\gpui`）— tag
`v0.2.2-rf2` として push 済み。ruviform 側は
`crates/ruviform-ui-gpui/Cargo.toml` の manifest tag pin +
`Cargo.lock` で参照（`[patch]`/path override 不使用）、
pinned dep 実測は `evidence/rf532/pinned-dep-results.md`。

## 機構の要点

失敗は 2 層だった（discovery 参照）。fix も 2 要素の合成:

1. **`window.rs` — `WS_EX_NOREDIRECTIONBITMAP` を付けない**。
   root window のこの flag は hierarchy 全体の redirection
   surface を消すため、child に raster が生成されない。
   外すことで child の描画 backing が復元される
   （window-DC green `0.000 → 1.000`）。
2. **`directx_renderer.rs` — swapchain content の clip
   partition**。flip-model swapchain の presentation plane が
   child 領域を覆うため、raster があっても screen に出ない。
   client 領域から可視 descendant rect を除いた partition に
   分割し、rect ごとに 1 つの visual（同一 swapchain を
   `SetContent` で共有）に `CreateRectangleClip` で clip を
   掛ける。child rect 相当の領域は swapchain に覆われず、
   child は通常の child-window 経路で合成される。

## DirectComposition 木構造

```
comp_target (CreateTargetForHwnd, topmost=false)   ← patch 1
└─ root_visual (container)
   ├─ visual[0]  SetContent(swapchain)  clip=partition_rect[0]
   ├─ visual[1]  SetContent(swapchain)  clip=partition_rect[1]
   └─ ...       rect 数 = partition の分割数（子 1 つで最大 4）
```

- partition は `(l,t,r,b)` の `Vec` で保持し、`sync` ごとに
  再計算して **差分があるときだけ** visual 全破棄 → 再構築
  → `Commit` → `DwmFlush`。
- child rect は `EnumChildWindows` で全 descendant を列挙し、
  `IsWindowVisible` で絞り、`GetWindowRect` → `ScreenToClient`
  で client 座標に変換。分割は `subtract_rect`（band 分割:
  top/bottom 全幅 + middle の left/right）で最大 4 rect/child。

## 同期駆動

| 駆動 | 契機 | 目的 |
| --- | --- | --- |
| `present()` 内 `sync_child_windows` | GPUI frame ごと | frame 出力と同じ commit で plane を更新 |
| `WM_TIMER`（`CHILD_WINDOW_SYNC_TIMER_ID`、10ms） | `handle_create_msg` で arm、DComp 有効時のみ | GPUI が frame を出さない間も attach 直後の child を uncover |

timer handler は `state.borrow_mut()` 内で
`renderer.sync_child_windows()` を呼ぶだけ — `RedrawWindow`
等の再入可能な Win32 呼出しを持たないため、borrow 中の
reentrancy（旧 surface-bind 版で起きた
`RefCell already borrowed`）は構造上起きない。

## 棄却した設計

- **`CreateSurfaceFromHwnd` で child を visual 化** —
  contract 上 layered window 専用。非 layered child には空
  surface しか返らず、bind 時点 snapshot / live-track 両仮説を
  実測で棄却。
- **child の `WS_EX_LAYERED` 化** — foreign child には
  適用不能（silent no-op / `0x80070006`）。in-place hosting を
  壊す popup 化も要件外。
- **blit-model swapchain への切替** — 実験的には child が
  見えるが flip-model の利点を捨てる根本解決にならない。

## device-lost / resize の扱い

- `handle_device_lost` で `DirectXRenderer::new` 相当が再
  実行され、`DirectComposition` も作り直される。
  `clip_partition` は空から再計算されるため、次回 sync で
  最新 geometry に基づき再構築される。
- window resize は client rect 変化 → partition 変化 →
  次 sync で rebuild。`set_swap_chain` も resize 時に再
  呼出しされ、保持中の partition で即座に rebuild される。
