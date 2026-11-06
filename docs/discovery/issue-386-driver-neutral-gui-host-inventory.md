---
id: "DISC-im2uskbeNbGlhxBg1mtsTR"
type: "discovery"
status: "resolved"
title: "Issue #386 Driver-neutral GUI Host: 現行契約棚卸しと WebView 前提分離案"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/386"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/issue-156-gui-host-foundation.md"
    - "../designs/issue-181-multi-window.md"
    - "../designs/issue-182-dock-tree.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "../specs/issue-386-driver-neutral-gui-host.md"
    - "../designs/issue-386-driver-neutral-gui-host.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/390"
  defers: []
  deferred-by: []
---

# Issue #386 Driver-neutral GUI Host: 現行契約棚卸しと WebView 前提分離案

## Summary

`ruviform-ui` / `ruviform-ui-webview` の現行公開契約を棚卸しし、
Issue #386 の「横断レビュー確定仕様 v2」（2026-11-07）が要求する
Driver-neutral 共通契約と WebView 専用ポートへの分離案を確定した。
現行契約は `WindowHandle` / `HostEvent` の形ですでに backend-free
だが、`eval`・`url`・`devtools`・`RenderTier`・`NativeSurfaceHost`
という WebView 実装前提の語彙を含む。本棚卸しは分離境界の正本であり、
実装詳細は `docs/designs/issue-386-driver-neutral-gui-host.md` を参照。

## Exploration Goal

- 現行 `ruviform-ui` のどの型・メソッド・イベントが WebView/JS
  実行モデルを前提にしているかを列挙する。
- 確定仕様 v2 の移行対象（`WindowConfig::url/devtools`、
  `HostHandle`/`WindowHandle::eval`、`HostEvent::Ready{url}`/
  `Message(String)`、`RenderTier::{Dom,Canvas2d,Webgl2}`、
  `NativeSurfaceHost`）を、共通契約 / WebView 専用 / 他 Issue
  （#388/#389）所有のどれに分離するか確定する。
- GPUI Driver が消費する最小契約と、#388 が早期利用する
  最小 Window fixture の拡張境界を定義する。

## 現行公開契約の棚卸し

| 型 / ポート | 位置 | 内容 | WebView 前提の度合い |
| --- | --- | --- | --- |
| `WindowId` | `window.rs` | 非ゼロ単調・破棄後不再利用の window 識別子 | 中立（そのまま共通契約へ） |
| `HostEvent` | `event.rs` | `Ready{url}`/`Message(String)`/`Resized`/`Focused`/`ScaleFactorChanged`/`Moved`/`CloseRequested`/`Destroyed`/`Tick` | `Ready{url}` はページロード前提、`Message(String)` は `window.ipc.postMessage` の生文字列前提。その他の variant は中立 |
| `HostHandle` | `event.rs` | `eval`/`resize`/`focus`/`request_close`（単一 window） | `eval` は JS 前提。他は中立 |
| `WindowEvent` | `window.rs` | `Windowed{window, event: HostEvent}` + `Tick` envelope | envelope 構造は中立。payload が `HostEvent` なので同上 |
| `RoutedEvent` | `window.rs` | `Event`（rvf.ipc.v1）+ 宛先 window | 中立（Event は wire schema であり backend 型ではない） |
| `CloseDecision` | `window.rs` | `Approve`/`Veto`/`Defer` | 中立 |
| `WindowConfig` | `window.rs` | `title`/`width`/`height`/`position`/`url`/`devtools` | `url`/`devtools` は WebView 専用 |
| `MonitorInfo` | `window.rs` | name/物理矩形/scale_factor | 中立 |
| `WindowHandle` | `window.rs` | `eval`/`resize`/`focus`/`create_window`/`request_close`/`respond_close`/`destroy_window`/`windows`/`monitors`/`exit` | `eval` が JS 前提。残りは中立ポートとして再利用可能 |
| `RenderTier` | `render.rs` | `Dom`/`Canvas2d`/`Webgl2`/`NativeSurface` | `Dom`/`Canvas2d`/`Webgl2` は旧 WebView renderer 階層の固有 enum。repo 内で**利用箇所 0**（render.rs 定義と lib.rs re-export のみ） |
| `NativeSurfaceHost` / `SurfaceRegion` / `SurfaceId` | `render.rs` | WebView 隣接 surface の create/move/destroy ポート | 「webview の兄弟」前提（clip/visibility/z-order/focus/IME を持たない旧設計）。#388 の `NativeSurfaceSlot` semantic contract が後継。repo 内利用箇所 0 |
| `UiRuntime` | `runtime.rs` | Intent → Service → JobSpec dispatch、job mailbox、`handle`/`handle_window`/`dispatch_op` | `HostEvent::Message` を JSON として `parse_intent` するが、`dispatch_op`/`dispatch_for` は `Intent` を直接受ける — 型付き action の受入口は既に中立 |
| `Intent`/`Event`/`ErrorBody`（rvf.ipc.v1） | `ipc.rs` | typed message schema | 中立（JSON シリアライズ可能な data schema。`Message(String)` との結合は adapter 側の decode） |
| `DockWorkspace`/`DockTree`/`ViewInstanceId` | `dock.rs` | 2-phase move/rollback、layout snapshot | 中立（論理 dock モデル。driver はこのモデルを drop/block しない） |
| `FakeHost` | `ruviform-ui-test` | `WindowHandle` の headless fake（eval backlog・close veto・Destroyed 終端） | `WindowConfig`/`WindowHandle` ベース — 新契約版の Fake Driver が別途必要 |
| `run`/`run_multi` | `ruviform-ui-webview` | tao/wry 実装の host loop | WebView adapter 実装。新 Driver の `run` 形状の前例（`FnMut(Event, &mut dyn Port)`） |

### 観測された構造的事実

1. 共通 crate `ruviform-ui` の公開シグネチャに wry/tao/wgpu/JS 型は
   **既に存在しない**。WebView 前提は型依存ではなく**語彙・意味論**
   （eval = JS 評価、url/devtools、生文字列 Message）として残っている。
2. `UiRuntime` は `Intent` 直受け経路（`dispatch_op`/`dispatch_for`）
   をすでに持つため、「typed action」を新設する必要はなく、
   driver は `Intent` を直接供給できる。WebView adapter 側は
   `parse_intent` で `Message(String)` → `Action(Intent)` に decode する。
3. `RenderTier`・`NativeSurfaceHost` 系は repo 内で**消費者ゼロ**の
   型のみの port であり、移行は低コスト。ただし #388 が
   `NativeSurfaceSlot` の owner であり、本 Issue は削除ではなく
   「後継契約への接続先」を移行表に記録するに留める。
4. `WindowHandle` の `eval` 以外の操作（create/resize/focus/close flow/
   monitors/exit）は中立であり、新 Driver port はほぼ同一形状を
   維持できる（`eval` → typed event push に置き換え）。

## 分離案（確定仕様 v2 の移行マッピング）

### 共通側（Driver-neutral・新設 — `ruviform-ui` に追加）

| 新型 | 収める意味 | 対応する旧 API |
| --- | --- | --- |
| `WindowSpec` | `title`・`logical_size`・`position`・`resizable` のみ。JS URL・devtools なし | `WindowConfig` の中立部分 |
| `WindowLifecycle`（event payload） | `Created`/`Ready`/`Resized`/`Moved`/`Focused`/`ScaleFactorChanged`/`CloseRequested`/`Destroyed`/`Action(Intent)` | `HostEvent` の中立 variant + `Ready{url}` を payload なし `Ready` へ、`Message(String)` を `Action(Intent)` へ |
| `DriverEvent`（envelope） | `Windowed{window, event: WindowLifecycle}` + `Tick` | `WindowEvent`（envelope 構造は同じ） |
| `WindowDriver`（port） | `create_window(spec)`/`resize`/`focus`/`request_close`/`respond_close`/`destroy_window`/`windows`/`monitors`/`post_event(window,&Event)`/`exit`/`capabilities()` | `WindowHandle` − `eval` + typed event push + capability 取得 |
| `WindowCapabilities` + `Capability` | `resize`/`focus`/`scale_factor`/`multi_window`/`ime`/`native_surface_slot` の Supported/Unsupported | 新設（確定仕様「capability と Unsupported/Error」要求） |
| `UiError::Unsupported` | capability 未対応の明示エラー | 新設（`UiError` は `#[non_exhaustive]` で追加は非破壊） |
| `LogicalSize`/`LogicalPosition` | logical 単位の size/position 値型 | `WindowConfig` の width/height/position の型化 |

### WebView 専用側（`ruviform-ui-webview` に局所化 — 本 Issue では**移行表記録のみ**、物理移動は別判断）

| 旧 API | WebView 専用の後継 | 備考 |
| --- | --- | --- |
| `WindowConfig::url`/`devtools` | `WebViewWindowConfig`（`WindowSpec` を内包 + `url`/`devtools`） | 現行フィールドを持つ `WindowConfig` は当面維持 |
| `HostHandle::eval`/`WindowHandle::eval` | `WebViewScriptPort` 相当の adapter 固有 port | dispatch_script 経路はそのまま |
| `HostEvent::Ready{url}` | adapter 内部で共通 `Ready` + url を adapter 固有ログへ分解 | 共通契約に url を持ち込まない |
| `HostEvent::Message(String)` | WebView IPC decode adapter（`parse_intent` → `Action(Intent)`、malformed は `Event::error`） | 生文字列は共通契約に持ち込まない |
| `RenderTier::{Dom,Canvas2d,Webgl2}` | 旧 WebView renderer 固有 enum として `render.rs` に残置 | 新 Driver は renderer tier を表現しない（component/view spec 側の責務、#387） |
| `NativeSurfaceHost`/`SurfaceRegion`/`SurfaceId` | `NativeSurfaceSlot` semantic contract（**#388 が owner**） | 本 Issue は接続先の記録のみ。削除・互換換えは行わない |
| `rvf://` scheme / `AssetSource` / `DataRegistry` / `dispatch_script` / `BRIDGE_JS` | WebView adapter 資産として維持 | GPUI Driver は消費しない（DataRegistry の service 境界は共通残置のままとするか別判断 — 本 Issue では触らない） |

### 他 Issue が所有する境界（本 Issue は消費・接続記録のみ）

- `NativeSurfaceSlot`（#388）— `WindowCapabilities::native_surface_slot`
  が接続点。HWND/NSView/X11/GPU handle は共通契約に持ち込まない。
- `PluginEditorSession`/`GuiParentToken`（#389）— L1 GUI は現状
  Unsupported であり、本 Issue は偽らない。session 型は所有しない。
- `DriverContractFixture`/Fake Driver（#390）— 共通 semantic
  fixture の owner。本 Issue の接続試験は新 Fake Driver を使うが、
  fixture registry/case manifest は #390 スコープ。
- Design Token/ResolvedTheme（#385）— `WindowSpec` に theme を
  持ち込まない。Component/View Spec（#387）にも踏み込まない。

## GPUI Driver の最小 fixture と拡張境界

- 新クレート `ruviform-ui-gpui`（adapter 層、`ruviform-ui` のみに
  path 依存。`default = []` + `gpui` feature で `dep:gpui` を opt-in、
  `cargo check --workspace` の headless 経路を維持）。
- 最小 fixture = `GpuiDriver`（`WindowDriver` 実装）+ `run` 入口
  （gpui `Application` loop 上で `FnMut(DriverEvent, &mut dyn
  WindowDriver)` を駆動）+ `examples/` デモ。2 window・close veto・
  focus/resize 追跡までを fixture 範囲とする。
- 拡張境界（#388 への受け渡し点）: `WindowSpec` とは別に、
  driver 側の `open_window` seam で root view factory を受ける。
  GPUI `Window`/`Entity`/`Context` 型は `ruviform-ui-gpui` 内部と
  明示的な driver extension のみに留め、共通契約へ漏出しない。

## Gap / 未解決

- GPUI 本体の OS 別 Window/IME/a11y/GPU device lost の実機評価は
  #262/#388 の PoC スコープ。本 Issue は「ビルド・最小契約・接続
  試験」までを提供し、実 OS PASS を主張しない。
- `RenderTier`/`NativeSurfaceHost` の物理的削除時期は移行戦略の
  「別明示判断」待ち。本 Issue は表記上の legacy 位置付けと
  接続先記録に留める。
- `run` 形状（`FnMut(DriverEvent, &mut dyn WindowDriver)`）は
  `run_multi` 前例の踏襲であり、GPUI event loop への最適化は
  fixture の実測後に判断する。

## Sources consulted

- `crates/ruviform-ui/src/{event,window,render,ipc,runtime,dock,error,lib}.rs`
- `crates/ruviform-ui-webview/src/{lib,session,host,scheme,detect,embed}.rs`
- `crates/ruviform-ui-test/src/fake.rs`
- Issue #386/#384/#262/#388/#389/#390 の 2026-11-07 確定仕様 v2 節
- GPUI 0.2.2 crates.io/docs.rs 公開情報（version・license・target 別依存）
