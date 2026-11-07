---
id: "DESIGN-novwjdV4MyUKuG9Y0bg014"
type: "design"
status: "proposed"
title: "Issue #386 Driver-neutral GUI Host 契約・GPUI Driver adapter・最小 Window fixture"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/386"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
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
    - "../discovery/issue-386-driver-neutral-gui-host-inventory.md"
    - "../specs/issue-386-driver-neutral-gui-host.md"
  derived-by:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-386-driver-neutral-gui-host.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #386 Driver-neutral GUI Host 契約・GPUI Driver adapter・最小 Window fixture

## Overview

確定仕様 v2（Issue #386 冒頭）が要求する「共通 Driver-neutral
契約 + WebView 専用 port への分離」を、**既存 API を破壊せず**
追加型として導入する。`ruviform-ui` に新 module `driver` を設け、
`WindowSpec`・`WindowLifecycle`・`DriverEvent`・`WindowDriver`・
`WindowCapabilities`/`Capability`・`UiError::Unsupported`・
`LogicalSize`/`LogicalPosition` を定義する。新クレート
`ruviform-ui-gpui` はこれを消費する adapter で、非既定 `gpui`
feature で `dep:gpui` を opt-in し、`cargo check --workspace` の
headless 経路を維持する（`ruviform-ui-webview` 前例と同一構成）。

## 依存方向（必須）

```text
Consumer View/Domain/Headless
        │ uses
        ▼
ruviform-ui        ── driver-neutral contract（本設計で追加）
  │     ▲                  WindowSpec / WindowLifecycle / DriverEvent /
  │     │                  WindowDriver / WindowCapabilities / UiError
  │     │ implements
  │     │
  ├─ ruviform-ui-webview   ── 既存 wry/tao adapter（WebView 専用 port。
  │                          共通契約には依存しない旧 API を維持）
  └─ ruviform-ui-gpui      ── 新規 GPUI adapter（本設計）
        └─ gpui 0.2.2（crates.io）  ── feature `gpui` のみ
```

- `ruviform-ui` は GPUI/DOM/wry/OS handle/Toolkit に依存しない
  （現行どおり）。`WindowDriver` の公開シグネチャは `WindowId`/
  `WindowSpec`/`Intent`/`Event`/`MonitorInfo`/`UiError` のみ。
- `ruviform-ui-gpui` は `ruviform-ui` に**のみ** path 依存。
  `gpui` 型はこの crate 内部と `examples/`、および明示的
  driver extension のみに現れる。
- `ruviform-ui-test` の新 `FakeDriver` は `ruviform-ui` 公開 API
  のみに依存し、GPUI を知らない。

## 共通契約（`ruviform-ui::driver`）

### 型

```rust
// 新設 module: crates/ruviform-ui/src/driver.rs

pub struct LogicalSize { pub width: u32, pub height: u32 }
pub struct LogicalPosition { pub x: i32, pub y: i32 }

pub struct WindowSpec {
    pub title: String,
    pub logical_size: LogicalSize,
    pub position: Option<LogicalPosition>,
    pub resizable: bool,
}
// impl Default: title="ruviform", 1280x800, position=None, resizable=true

#[non_exhaustive]
pub enum WindowLifecycle {
    Created,
    Ready,
    Resized { width: u32, height: u32 },   // physical px
    Moved { x: i32, y: i32 },              // physical px
    Focused(bool),
    ScaleFactorChanged { factor: f64 },
    CloseRequested,
    Destroyed,
    Action(Intent),   // typed action → UiRuntime
}

#[non_exhaustive]
pub enum DriverEvent {
    Windowed { window: WindowId, event: WindowLifecycle },
    Tick,
}

#[non_exhaustive]
pub enum Capability { Supported, #[default] Unsupported }
// 実装では ADR（public-enum-non-exhaustive-policy）に従い
// `#[non_exhaustive]` 付き — driver 側構築型でも enum 公開規約は
// 例外を設けない（当初 sketch の exhaustive 2 値から変更）。

pub struct WindowCapabilities {
    pub resize: Capability,
    pub focus: Capability,
    pub scale_factor: Capability,
    pub multi_window: Capability,
    pub ime: Capability,
    pub native_surface_slot: Capability,
}
// impl Default = 全 Unsupported。driver が各値を設定して返す。

pub trait WindowDriver {
    fn capabilities(&self) -> WindowCapabilities; // 値返却（RefCell 実装可能に）
    fn create_window(&mut self, spec: &WindowSpec) -> Result<WindowId, UiError>;
    fn resize(&mut self, window: WindowId, size: LogicalSize) -> Result<(), UiError>;
    fn focus(&mut self, window: WindowId, focused: bool) -> Result<(), UiError>;
    fn request_close(&mut self, window: WindowId);
    fn respond_close(&mut self, window: WindowId, decision: CloseDecision) -> Result<(), UiError>;
    fn destroy_window(&mut self, window: WindowId) -> Result<(), UiError>;
    fn windows(&self) -> Vec<WindowId>;
    fn monitors(&self) -> Vec<MonitorInfo>;
    fn post_event(&mut self, window: WindowId, event: &Event); // fire-and-forget — 失敗しない
    fn exit(&mut self);
}
```

`UiError` に variant 追加（`#[non_exhaustive]` で非破壊）:

```rust
UiError::Unsupported { capability: String, message: String }
```

### 設計意図

1. **`WindowSpec` に JS URL/devtools を持たせない**。
   `WindowConfig` の `url`/`devtools` は WebView adapter の
   `WebViewWindowConfig` へ移る（移行表）。`WindowSpec` は
   window の外形記述のみ。
2. **`Ready` は payload なし**。WebView の `Ready{url}` は
   ページロード+bridge 起動の意味を持っていた。共通契約では
   「window が表示・入力可能な初回状態」として単位化し、
   url は adapter 内部ログに局所化する。`Created`（host が
   window を受理し `WindowId` を払い出した）と `Ready`（実表示・
   入力可能）は GPUI でも区別できる（`open_window` 受理 →
   初回 present/focus 化）。
3. **`Action(Intent)` は typed action port**。`Intent` は
   `rvf.ipc.v1` の型付き action で、GPUI view は Rust から
   直接構築する。WebView adapter は `postMessage` の生文字列を
   `parse_intent` で decode して `Action` を生成する
   （malformed は `Event::error` として別処理）。
   `UiRuntime` は既存の `dispatch_for`/`dispatch_op` で `Intent`
   を直接受けられる — JSON 経由を必須にしない。
4. **`post_event(window, &Event)` は eval の中立後継**。
   `Event`（rvf.ipc.v1）を宛先 window の view へ届ける
   typed push。WebView adapter は従来どおり
   `eval(dispatch_script(&event))` で実装し、GPUI adapter は
   root view の entity update で実装する。
5. **`WindowCapabilities` は Unsupported/Error の明示**。
   確定仕様「Window/Surface/Input/Focus/Scale/NativeSlot 等は
   Driver capabilities と Unsupported/Error」を満たす最小集合。
   `native_surface_slot` は #388 の `NativeSurfaceSlot` 契約が
   実装された時点で `Supported` になる宣言点で、型そのものは
   持ち込まない。
6. **close flow は `WindowHandle` と同一 semantics**。
   `request_close` は veto 可能な要求の emit、`respond_close` で
   `Approve`/`Veto`/`Defer` を回答、unanswered は対象 window のみ
   `Approve` を既定、`Destroyed` が終端 event で `WindowId`
   不再利用。Fake/GPUI の双方で同じ invariant を検証する。

## `ruviform-ui-gpui` の構成

```text
crates/ruviform-ui-gpui/
  Cargo.toml            # default=[] + `gpui` feature → dep:gpui 0.2.2
  src/lib.rs            # feature off: version() のみ / feature on:
                        # `mod gpui_driver` に GpuiDriver + IntentSink +
                        # 最小 root view（RvfRoot）を集約
  examples/minimal_window.rs  # required-features=["gpui"]
```

実装では当初 sketch（driver.rs/session.rs/view.rs 分割 +
crate 内 tests/driver_contract.rs）から、fixture 規模では単一
`mod gpui_driver` 集約へ変更。driver 契約試験は共通 semantic
fixture として `crates/ruviform-ui-test/tests/fake_driver.rs`
（14 tests）に、adapter 接続試験は
`crates/ruviform-ui/tests/runtime.rs`（
`dispatch_window_intent` parity）に置く。

### `GpuiDriver` と `run` の形

`run_multi` 前例を踏襲し、consumer には event handler として
`FnMut(DriverEvent, &mut dyn WindowDriver)` を渡す。内部では
gpui `Application` を起動し、`Window` の `on_window_should_close`
（veto 化）、`on_next_frame`/`is_window_active`/`window_bounds`/
`scale_factor` の差分観測で `Resized`/`Moved`/`Focused`/
`ScaleFactorChanged` を合成する。`open_window` 受理 → `Created`、
初回 render 完了 → `Ready` を emit する。

```rust
impl GpuiDriver {
    pub fn run(
        &mut self,
        tick: Option<Duration>,
        on_event: impl FnMut(DriverEvent, &mut dyn WindowDriver) + 'static,
    ) -> Result<(), UiError>
}
```

- 実装は `GpuiDriver::run(&mut self, tick, on_event)` のメソッド形
  （当初 sketch の free `run(session: DriverSessionConfig, …)`
  から変更 — window は `create_window` で都度生成し、
  `WebContext` 相当の session 共有リソースは GPUI では
  `Application` 自体が担う）。
- `Tick` は `cx.background_executor().timer` 相当の周期 task で emit。
  `UiRuntime::poll` と同じく job drain の wakeup 点。
- `post_event` は `WindowId` → `Entity<FixtureView>` の写像から
  view を引き、`view.update` で `Event` を適用する。
- `destroy_window` は `window.remove_window()`、`request_close` は
  `on_window_should_close` のコールバックを自前で発火させる
  （consumer が veto できる経路を保持）。`respond_close(Approve)` →
  `remove_window()`、unanswered `CloseRequested` は既定 `Approve`。
- `exit` は `App::quit()`。`monitors` は `App::displays()`。
- `windows` は `App::windows()` の handle 集合から `WindowId` 写像を返す。

### 最小 fixture の root view

`FixtureView`（`Entity` + `Render`）は fixture 専用:
- `Action(Intent)` を emit する操作（button click / key で
  `demo.ping` 等の `Intent` を供給）を持つ最小 UI。
- `post_event` で届いた `Event` の kind を表示する最小表示
  （「受信した typed event の可視化」で `eval` 経路の置き換えを検証）。
- `on_window_should_close` で close veto を確認できる
  `CloseRequestPolicy`（fixture フラグで Approve/Veto/Defer を切替）。

### 拡張境界（#388 への受け渡し）

`open_window` 時に `WindowSpec` とは別に root view factory を
注入できる seam を公開する。fixture では `FixtureView` を返し、
#388 は同じ seam で `NativeSurfaceSlot` を所有する view を返す。
GPUI `Window`/`Entity`/`Context`/`AnyElement` 型は
`ruviform-ui-gpui` 内部・`examples/`・明示的 driver extension
crate のみに現れ、`ruviform-ui` の公開 API には漏出しない。

## GPUI 依存の確認記録（#262 採否 Gate への入力）

| 項目 | 確認値 | 出典 |
| --- | --- | --- |
| crate / version | `gpui 0.2.2`（crates.io、2025-10-22 公開。最新安定） | crates.io |
| license | Apache-2.0 | crates.io / docs.rs パッケージ metadata |
| publisher | `zed-industries` 公式（owners に `github:zed-industries:crates-io`） | crates.io |
| upstream 状況 | 内部 `publish = false` 依存のため upstream は現時点で新バージョンを publish できない（Zed Discussion #55271）。0.2.2 は platform split 前の最後の publish 済み snapshot | github discussion |
| Windows | `windows ^0.61`/`windows-core`/`embed-resource` 依存あり — Windows は support target | docs.rs dep list |
| macOS | `metal`/`cocoa`/`core-text`/`core-video` 等 — Metal バックエンド | docs.rs dep list |
| Linux | `blade-graphics`/`wayland-*`/`x11rb`/`cosmic-text`/`zed-xim` 等 — blade + Wayland/X11 | docs.rs dep list |
| features（default） | `font-kit`/`wayland`/`x11`/`windows-manifest`。`test-support`（`#[gpui::test]`/`TestAppContext`）・`inspector`・`screen-capture` 等は opt-in | docs.rs source Cargo.toml |
| rust-version | manifest 宣言なし（edition = "2024" 以上 = rustc ≥1.85）。本 repo pin (1.99.0) で build 確認済み | docs.rs + `cargo check` 実測 |
| transitive 規模 | docs.rs の dep list で数十 direct + target 別。`naga`/`cosmic-text`/`resvg`/`usvg`/`metal`/`windows`/`wayland-*`/`blade-*` 等 | docs.rs dep list |
| 採否の留保 | GPUI 最終採否は #262 が RF-GPUI-05/06（#388/#389）の実機証拠を待つ。本 Issue の最小 fixture は「採用 Gate の候補 Driver が GPUI 0.2.2 で build・最小 contract を満たす」までを示す | Issue #262/#388 |

## 旧 API → 新 API 移行表

| 旧 API（`ruviform-ui` / `-webview`） | 新 API / 移行先 | 物理位置 | 状態 |
| --- | --- | --- | --- |
| `WindowConfig`（`url`/`devtools` を含む） | 中立 `WindowSpec`（`title`/`logical_size`/`position`/`resizable`） | `ruviform-ui::driver` | 本 Issue で追加。`url`/`devtools` は `WebViewWindowConfig` へ |
| `WindowConfig::url` | `WebViewWindowConfig::url` | `ruviform-ui-webview`（本 Issue は移行表記録のみ） | 旧 `WindowConfig` は当面維持 |
| `WindowConfig::devtools` | `WebViewWindowConfig::devtools` | 同上 | 同上 |
| `HostHandle::eval` / `WindowHandle::eval` | `WebViewScriptPort` 相当（adapter 固有） | `ruviform-ui-webview` | 移行表記録。共通側は `WindowDriver::post_event` |
| `HostEvent::Ready { url }` | `WindowLifecycle::Ready`（payload なし） | `ruviform-ui::driver` | url は adapter 内部ログへ |
| `HostEvent::Message(String)` | `WindowLifecycle::Action(Intent)` + WebView IPC decode adapter | `ruviform-ui::driver` + `ruviform-ui-webview` | 生文字列は共通契約に入れない |
| `HostEvent::Resized/Moved/Focused/ScaleFactorChanged/CloseRequested/Destroyed/Tick` | `WindowLifecycle`/`DriverEvent::Tick` に中立のまま対応 | `ruviform-ui::driver` | payload 型は同一 |
| `WindowEvent` envelope | `DriverEvent` envelope | `ruviform-ui::driver` | 構造は同一 |
| `RoutedEvent` | そのまま利用（`Event` + window） | `ruviform-ui::window` | 中立のまま再利用 |
| `WindowHandle` の `resize`/`focus`/`create_window`/`request_close`/`respond_close`/`destroy_window`/`windows`/`monitors`/`exit` | `WindowDriver` の同名操作 | `ruviform-ui::driver` | `eval` のみ除去・`post_event`/`capabilities` 追加 |
| `RenderTier::{Dom,Canvas2d,Webgl2}` | 旧 WebView renderer 固有 enum（`render.rs` に残置） | `ruviform-ui::render` | repo 内利用 0。新 Driver は扱わない（#387 側） |
| `NativeSurfaceHost`/`SurfaceRegion`/`SurfaceId` | `NativeSurfaceSlot` semantic contract（**#388 owner**） | `ruviform-ui::render` → 後継は #388 | 本 Issue は接続点のみ記録 |
| `run`/`run_multi` の `FnMut(WindowEvent, &mut dyn WindowHandle)` | `run` の `FnMut(DriverEvent, &mut dyn WindowDriver)` | `ruviform-ui-gpui::session` | 同一形状 |
| `UiRuntime::handle`/`handle_window`（`HostEvent` 消費） | `UiRuntime::dispatch_for`/`dispatch_op` への `Intent` 直受け + `Action(Intent)` 経路 | `ruviform-ui::runtime` | `Action` で `handle_window` 経由の既存 dispatch を再利用 |
| `FakeHost`（`ruviform-ui-test`） | `FakeDriver`（`WindowDriver` 実装） | `ruviform-ui-test` | 新設。旧 fake は維持 |

## 検証と残 Gate

- 本 Issue で実行: `cargo check --workspace`（GPUI なし headless
  経路が残ること）・`check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`
  （feature lane）・Fake Driver と GPUI Driver の同一 lifecycle
  接続試験。
- 残 Gate（本 Issue では NotRun/別 Issue へ委譲）: 実 OS window・
  実 plugin Native Dock・IME/DPI・a11y・GPU device lost・両
  Consumer gate・Dock 2-phase move/rollback の実 Driver
  fixture。これらは #262/#388/#389/#390/#294 の実機検証で
  扱い、本 Issue の最小 fixture Pass で代替しない。
