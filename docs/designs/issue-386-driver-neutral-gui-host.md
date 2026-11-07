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

## 旧 API → 新 API 移行表（Host / Window / Dock / Binding）

棚卸しは現行コードで確認済み（`crates/ruviform-ui/src/{event,window,render,bridge,runtime,dock,behavior}.rs`・
`crates/ruviform-ui-webview/src/{host,session}.rs`）。「状態」は本 Issue 時点の実装状況。
旧 WebView 経路は**削除せず並存**させる（Issue 本文「新 Native 完成前に旧 WebView path を削除しない」）。

### Host / Window

| # | 旧 API（型・場所） | 新 API / 移行先（型・実装ファイル） | 移行手順 | 状態 |
| --- | --- | --- | --- | --- |
| H1 | `HostHandle::eval(&mut self, script)`（`ruviform-ui/src/event.rs`） | 共通: `WindowDriver::post_event(window, &Event)`（`ruviform-ui/src/driver.rs`）。WebView 専用: `eval(dispatch_script(..))` を adapter 内に局所化 | Consumer は `post_event` のみ使う。WebView adapter は `post_event` を `eval(dispatch_script)` で実装する。旧 trait は残置 | 共通 port 追加済。WebView 側の `post_event` 実装は未着手（旧 `HostHandle` を使用中） |
| H2 | `WindowHandle::eval(window, script)`（`ruviform-ui/src/window.rs`） | H1 と同じ（`WindowDriver::post_event`） | H1 と同じ。`WindowHandle` は旧 WebView 経路で存続 | 共通 port 追加済。旧 trait 残置 |
| H3 | `HostEvent::Ready { url }`（`event.rs:23`）。WebView は `host.rs:224` で生成 | `WindowLifecycle::Ready`（payload なし。`driver.rs`） | `url` は WebView adapter 内ログへ移す。adapter は `Ready{url}` → `WindowLifecycle::Ready` へ写像する | 型追加済。WebView 側の写像は未着手（旧 `Ready{url}` を生成） |
| H4 | `HostEvent::Message(String)`（`event.rs:31`。WebView は `host.rs:220` で生成） | `WindowLifecycle::Action(Intent)`（`driver.rs`）+ WebView IPC decode（`ipc::parse_intent` を adapter 内で呼ぶ） | 生文字列を共通契約に入れない。decode 失敗は `Event::error` として adapter 側で処理 | 型追加済。WebView decode 写像は未着手。GPUI は `IntentSink::send` で `Action` を発行（済） |
| H5 | `WindowConfig { url, devtools, … }`（`window.rs:146-161`。WebView は `ruviform-ui-webview/src/host.rs:49-66` でも定義） | 中立: `WindowSpec { title, logical_size, position, resizable }`（`driver.rs`）。`url`/`devtools` は `WebViewWindowConfig`（未作成） | WebView adapter に `WebViewWindowConfig { url, devtools, … }` を作り、`WindowSpec` → 変換を提供する。旧 `WindowConfig` は残置 | `WindowSpec` 追加済。`WebViewWindowConfig` 未作成（**未達**） |
| H6 | `HostEvent::{Resized, Moved, Focused, ScaleFactorChanged, CloseRequested, Destroyed, Tick}`（`event.rs`） | `WindowLifecycle::{Resized, Moved, Focused, ScaleFactorChanged, CloseRequested, Destroyed}` / `DriverEvent::Tick`（`driver.rs`） | payload 同一。WebView adapter は既存イベントを写像するだけで済む | 追加済（payload 同一） |
| H7 | `WindowEvent`（`window.rs:88`。`Windowed`/`Tick`） | `DriverEvent`（`driver.rs`） | envelope 構造同一 | 追加済 |
| H8 | `WindowHandle::{create_window, resize, focus, request_close, respond_close, destroy_window, windows, monitors, exit}` | `WindowDriver` の同名操作（`driver.rs`）。`create_window` は `&WindowSpec`、`resize` は `LogicalSize` | 旧 trait の操作を新 trait へ 1:1 で対応。`eval` のみ除去、`post_event`・`capabilities` を追加 | 追加済（GPUI/Fake 実装あり） |
| H9 | `run` / `run_multi`（`ruviform-ui-webview/src/host.rs:82`・`session.rs:455`。`FnMut(HostEvent/WindowEvent, &mut dyn HostHandle/WindowHandle)`） | `GpuiDriver::run(tick, FnMut(DriverEvent, &mut dyn WindowDriver))`（`ruviform-ui-gpui/src/lib.rs`） | GPUI 側は新形状で実装済。WebView 側は旧形状のまま残置 | GPUI 側追加済。WebView 側は残置 |
| H10 | `UiRuntime::handle(HostEvent)` / `handle_window(WindowId, HostEvent)`（`runtime.rs:156,187`） | `UiRuntime::dispatch_window_intent(WindowId, Intent)`（`runtime.rs:227`）+ `drain_jobs_routed()`（`DriverEvent::Tick` の受け口） | `Windowed { Action(intent) }` → `dispatch_window_intent`。`Tick` → `drain_jobs_routed`。旧 `handle*` は残置 | 追加済。旧 `handle*` 残置（WebView 経路が使用） |
| H11 | `RenderTier::{Dom, Canvas2d, Webgl2}`（`render.rs:30-38`） | 新 Driver の対象外。Component/View 契約（#387）と `ruviform-ui-gpui` の描画に置換 | 旧 WebView renderer 固有 enum として `render.rs` に残置。repo 内の新 Driver 利用は 0 | 残置（**新 API 未定義。移行は旧 WebView 削除判断後**） |
| H12 | `NativeSurfaceHost` / `SurfaceRegion` / `SurfaceId`（`render.rs:47-70`。WebView-adjacent 前提） | `NativeSurfaceSlot` semantic contract（`native_slot.rs`、Issue #388 owner）。`WindowCapabilities::native_surface_slot` が宣言点 | 本 Issue は接続点のみ。旧 trait は残置 | #388 側で追加済。旧 trait 残置 |
| H13 | `JS bridge`（`bridge.rs:23` `BRIDGE_JS`、`bridge.rs:74` `dispatch_script`。`window.__rvf` / `rvf.ipc.v1` の postMessage） | WebView adapter 専用（`ruviform-ui-webview` が init script として注入）。共通契約と GPUI は JS を使わない | 旧経路として残置。共通 crate からは参照しない | 残置（webview 専用であることを明示） |

### Dock

| # | 旧 API（型・場所） | 新 API / 移行先（型・実装ファイル） | 移行手順 | 状態 |
| --- | --- | --- | --- | --- |
| D1 | `DockWorkspace` / `DockTree` / `DockNode` / `ViewInstanceId`（`ruviform-ui/src/dock.rs`） | 変更なし（driver-neutral。`WindowId` のみ参照し、JS / GPUI 型を持たない） | Consumer は `create_window` が返す `WindowId` を `register_window` する | 変更不要。共通 fixture で検証（`ruviform-ui-test/src/lifecycle.rs`） |
| D2 | Dock 2-phase move / rollback（`begin_move`・`commit_move`・`abort_move`・`move_view`） | 変更なし | 同上。Late receipt と組み合わせる場合も Dock API は不変 | 変更不要。Fake/GPUI 両方で同一 fixture 検証（**GPUI の実 OS 上の DockWorkspace 統合は未実施**） |
| D3 | Window 破棄時の view 処理（`remove_window(window, WindowCloseDisposition)`） | 変更なし。Consumer が `WindowLifecycle::Destroyed` 受信後に呼ぶ | `DriverEvent::Windowed { Destroyed }` を受けて `remove_window` を呼ぶ（明示的な処分選択） | 変更不要。fixture で検証（`RelocateTo` / `DetachViews`） |

### Binding

| # | 旧 API（型・場所） | 新 API / 移行先（型・実装ファイル） | 移行手順 | 状態 |
| --- | --- | --- | --- | --- |
| B1 | `rvf.ipc.v1` `Intent`/`Event`/`parse_intent`（`ipc.rs`） | 共通維持。`Intent` は `Action` と `post_event` の型 | 変更なし | 維持 |
| B2 | `window.__rvf` JS dispatch（`bridge.rs` の `BRIDGE_JS` / `dispatch_script`） | H13 と同じ（WebView 専用） | 旧経路として残置 | 残置 |
| B3 | Headless value / gesture / command binding（`behavior.rs`、`view.rs`。Issue #387） | 変更なし（driver-neutral）。GPUI 描画は `ruviform-ui-gpui/src/render.rs` で投影 | 変更なし | 変更不要 |
| B4 | GPUI view からの Intent 発行 | `IntentSink::send`（`ruviform-ui-gpui/src/lib.rs`）→ `WindowLifecycle::Action` | 済 | 追加済 |

### 移行段階（Phase）

1. **Phase 0（本 Issue の範囲）**: 共通契約（`driver.rs`）・Fake Driver・GPUI Driver・共通 lifecycle fixture（2 Window / Dock 2-phase move・rollback / detach・reattach / close veto・defer・approve / unanswered close default / late receipt / id non-reuse / tear-down）を追加。旧 API は残置。
2. **Phase 1（後続 Issue）**: WebView adapter に `WebViewWindowConfig` を導入し、H1・H3・H4・H5・H9 を adapter 内の写像として実装する。旧 `HostHandle`/`WindowHandle`/`run` は、この写像で同じ挙動を保つ接続試験が通るまで残す。
3. **Phase 2（別明示判断）**: Synvelis/AudioLens の native 最小 Slice・実 OS・Native Dock・両 Consumer gate 合格後に、旧 API（H1〜H5・H9・H10・H11・H13）の削除/移動を判断する。

## 検証と残 Gate

- 本 Issue で実行: `cargo check --workspace`（GPUI なし headless
  経路が残ること）・`check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`
  （feature lane）・Fake Driver と GPUI Driver の同一 lifecycle
  接続試験。
- 同一 lifecycle fixture: `ruviform-ui-test::lifecycle::run_lifecycle`
  （共通シナリオ・`expected_trace()`）を、Fake は
  `crates/ruviform-ui-test/tests/lifecycle_fixture.rs`（`mise run test`）、
  GPUI は `crates/ruviform-ui-gpui/src/lifecycle_fixture.rs`（`cfg(all(test, gpui-test))`）
  （`mise run test-ui-gpui`、`TestAppContext` の 1 turn ごとの settle）で
  同じ trace に対して実行する。GPUI 側の settle は `gpui-test` 専用の
  crate 内 seam（`GpuiDriver::settle_for_test`、`pub(crate)`）で、本番の `run` は変更しない。公開 signature に gpui 型を出さないため `pub` にしない（境界 guard の signature 検査で確認）。
- gpui 0.2.2 の test platform は OS window handle を持たず
  `window_handle()` が panic する。`gpui-test` ビルドでは surface 親
  HWND の取得を行わない（`capture_raw_parent`）。本番ビルドの挙動は不変。
- NotRun（実 OS のみ）: DPI/scale 遷移・IME・OS 起因の focus 遷移・
  GPU device lost・実 plugin Native Dock・両 Consumer gate。
- 残 Gate（本 Issue では NotRun/別 Issue へ委譲）: 実 OS window・
  実 plugin Native Dock・IME/DPI・a11y・GPU device lost・両
  Consumer gate・Dock 2-phase move/rollback の実 Driver
  fixture。これらは #262/#388/#389/#390/#294 の実機検証で
  扱い、本 Issue の最小 fixture Pass で代替しない。
