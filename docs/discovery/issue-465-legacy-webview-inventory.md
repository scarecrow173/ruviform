---
id: "DISC-PZXzzHRXfnvatIchgHgLEJ"
type: "discovery"
status: "resolved"
title: "Issue #465 Legacy WebView 基盤の棚卸し — 撤去対象・移行先・consumer 影響"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "https://github.com/scarecrow173/ruviform/issues/455"
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
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-465-legacy-webview-p0p1.md"
    - "docs/designs/issue-465-legacy-webview-p0p1.md"
    - "docs/adr/issue-465-legacy-webview-retirement-status.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-465-legacy-webview-p0p1.md"
    - "docs/designs/issue-465-legacy-webview-p0p1.md"
  verifies: []
  verified-by: []
  references:
    - "crates/ruviform-ui/src/lib.rs"
    - "crates/ruviform-ui/src/event.rs"
    - "crates/ruviform-ui/src/window.rs"
    - "crates/ruviform-ui/src/ipc.rs"
    - "crates/ruviform-ui/src/render.rs"
    - "crates/ruviform-ui/src/bridge.rs"
    - "crates/ruviform-ui/src/detect.rs"
    - "crates/ruviform-ui-webview/src/lib.rs"
    - "crates/ruviform-ui-gpui/src/lib.rs"
    - "mise.toml"
    - ".github/workflows/ci.yml"
    - ".github/workflows/consumer-verify.yml"
  defers: []
  deferred-by: []
---

# Issue #465 Legacy WebView 基盤の棚卸し

Issue #465 P0/P1 の調査成果物。旧 WebView-first GUI 基盤の全構成要素を
「現行使用 / legacy 保持 / 撤去候補」に分類し、driver-neutral 側の
移行先と consumer 影響を記録する。**本書は記録のみであり、物理撤去
（P2）は #419 GPUI 採用判定と両 consumer の Native 受入を gate とする
別 PR の判断事項である**（gate 未達のため本 Issue 範囲では NotRun）。

分類凡例:

- **現行**: driver-neutral 契約または標準 Driver として維持される面。
- **legacy**: 動作は維持するが新規機能・parity 目標の対象外。P2 で
  撤去判断される候補。
- **共有**: WebView adapter と driver-neutral 契約の双方が使うため、
  WebView 撤去後も残る部分。

## 1. Ruviform 側の棚卸し

### 1.1 crate / adapter 層

| 資産 | 分類 | 現状 | 移行先 / 扱い | 撤去 owner |
| --- | --- | --- | --- | --- |
| `crates/ruviform-ui` | 現行+legacy 混在 | driver-neutral 契約（`driver`/`view`/`behavior`/`dock`/`native_slot`）と旧 WebView 向け面が同居 | 旧 API 面は本書 §1.2 で個別分類 | P2（別 PR） |
| `crates/ruviform-ui-webview` | legacy（現役 adapter） | #423 で `WindowDriver` 実装へ移行済み。公開 API は `run_multi`/`SessionConfig`/`WebViewWindowConfig`/`detect_backend`/`webview_available`/`EmbedAssets` | 撤去時は consumer の WebView feature/bin が先に退役する必要あり | P2（#465 後続） |
| `crates/ruviform-ui-gpui` | 現行（標準 Driver） | GPUI adapter。#463 等の継続改善対象 | — | 撤去対象外 |
| `crates/ruviform-ui-theme` | 現行 | DTCG theme interpreter。driver 非依存 | — | 撤去対象外 |
| `crates/ruviform-ui-test` | 現行+legacy 混在 | `FakeDriver`/driver-lab/`rvf.uitest.v1` は現行契約検証に使用。`FakeHost`/`HeadlessIo`/`run_multi` 配線の WebView 経路 lane は legacy | driver-neutral lane（`driver_parity`/`driver_contract`/boundary guard）を正本として維持 | P2 で WebView 配線 lane のみ縮退判断 |
| `crates/ruviform-poc-native-dock` | 現行（検証 harness） | GPUI Native Dock 検証用 | — | 撤去対象外 |

### 1.2 `ruviform-ui` 旧 API 面の分類

| API | 分類 | 移行先（driver-neutral） | 備考 |
| --- | --- | --- | --- |
| `ipc` モジュールの `Intent`/`Event`/`ErrorBody`/`codes`/`parse_intent`/`SCHEMA_VERSION`（`rvf.ipc.v1`） | **共有・維持** | そのまま共通契約 | `driver.rs` が `Action(Intent)` で直接使用。wire schema は driver 非依存 |
| `HostEvent::{Ready{url}, Message(String)}` | legacy | `driver::WindowLifecycle`（`Ready` は url なし、`Action(Intent)` が Message の typed 後継） | `Ready.url` は WebView ナビゲーション固有。AudioLens `desktop.rs` は `Ready` を写像しない判断済み |
| `HostEvent` の Resized/Focused/ScaleFactorChanged/Moved/CloseRequested/Destroyed/Tick | legacy（意味は共通） | `driver::WindowLifecycle` の同名相当 variant | 意味論は driver-neutral へ移植済み。旧 enum は adapter 互換のため保持 |
| `HostHandle`（`eval`/`resize`/`focus`/`request_close`） | legacy | `driver::WindowDriver`（`eval` なし、`post_event`/`capabilities` あり） | `eval` は JS 実行で WebView 固有。resize/focus/close は `WindowDriver` に同等あり |
| `window::WindowHandle`（`eval`/`resize`/`focus`/`create_window`/`request_close`/`respond_close`/`destroy_window`/`windows`/`monitors`/`exit`） | legacy | `driver::WindowDriver` + `WindowSpec` | `eval` と `WindowConfig{url,devtools}` が WebView 前提。close/monitor/lifecycle 意味は移植済み |
| `window::WindowConfig{url,devtools}` | legacy | `driver::WindowSpec`（title/size/position/resizable のみ） | `url`/`devtools` は WebView adapter 固有。`WindowSpec` に対応項目なし（#384 設計の意図どおり） |
| `window::WindowId`/`WindowEvent`/`RoutedEvent`/`CloseDecision`/`MonitorInfo` | legacy（実装上は現役） | `driver::DriverEvent`/`WindowLifecycle` 系 | WebView adapter・FakeHost・consumer（synvelis-workspace 等）が現役使用。driver-neutral 側の ID 体系は #386 設計で別途確立 |
| `render::RenderTier::{Dom,Canvas2d,Webgl2}` | legacy | GPUI 描画モデル（`ruviform-ui-gpui` の element/canvas 描画）+ driver-neutral ViewSpec | WebView/DOM 前提の tier。新規 ViewSpec はこれらを対象にしない（#455 not-planned と整合） |
| `render::RenderTier::NativeSurface`/`NativeSurfaceHost`/`SurfaceRegion`/`SurfaceId` | legacy（superseded 寄り） | `native_slot` モジュール（`SurfaceSlotId`/`SlotLease`/`NativeSurfaceSlot`/`SurfaceSlotRegistry`、Issue #388） | crate README が「native dock 埋込の正本は native_slot」と明記済み |
| `bridge::BRIDGE_JS`/`dispatch_script`（`window.__rvf`・`window.ipc.postMessage`/`evaluate_script` 輸送） | legacy | driver-neutral では `Action(Intent)` 直接配送。JS bridge は WebView adapter のみが使用 | `rvf.ipc.v1` schema 自体は共有（上記） |
| `detect::BackendReport`/`BackendState` | legacy | GPUI driver の capability/`WindowCapabilities` 機構 | WebView2/WKWebView/WebKitGTK 検出は WebView adapter 固有 |
| `assets::AssetSource`/`MapAssetSource`/`resolve_path`/`mime_for` | 共有寄り legacy | frontend asset 配信の driver-neutral 等価物は未確立（P2 以降の検討事項） | `rvf://app` scheme 経由の配信は WebView adapter 実装。型自体は backend 非依存 |
| `data::DataRegistry`/`DataSource`/`VecDataSource`/`DataRequest` | 共有寄り legacy | 同上（bounded binary pull の意味論は driver 非依存だが wire 配信は `rvf://data` = WebView scheme） | paging 契約は `ruviform-ui-webview` README が正本 |
| `service`/`runtime`（`Service`/`UiRuntime`/`JobSpec`/`JobContext`） | **現行・維持** | そのまま共通基盤 | driver-neutral `dispatch_window_intent` 経路も同一 runtime を使用 |
| `demo::DemoService` | 現行 | そのまま | headless `demo_cli`/`demo_dock` example が正本利用 |
| `dock`/`native_slot`/`driver`/`view`/`behavior` | 現行 | — | Issue #384/#386/#387/#388 の driver-neutral 契約 |

`#[deprecated]` 付与の判断: **本 P1 では付与しない**。旧 API は
`ruviform-ui-webview` adapter・`ruviform-ui-test` の WebView lane・
両 consumer が現役で使用しており、crate 外利用に warning を発生させると
`lint-ui-webview` 等の `-D warnings` lane と consumer 側 lint を破壊する。
移行先は doc コメントで明示し、物理削除は P2 の別判断に委ねる。

### 1.3 `ui/*` TypeScript 資産

| パッケージ | 分類 | 内容 | 扱い |
| --- | --- | --- | --- |
| `ui/core` | legacy | DOM 非依存の headless 契約型（Gesture/Receipt 等） | headless 契約の意味論は Rust 共通契約へ移植済み（#387/#390）。TS 実装自体は WebView frontend 資産 |
| `ui/components` | legacy | Web Components / SolidJS 部品 | 同上 |
| `ui/viz` | legacy | Canvas2D/WebGL2 Reference Viz | GPUI 側に `ruviform-ui-gpui` の meter/timeaxis/spectrogram 描画が別系統で存在 |
| `ui/binding` | legacy | `rvf.ipc.v1` TS binding | schema は共有だが TS client は WebView frontend 専用 |
| `ui/guitest` | legacy 寄り | 決定的 GUI テスト基盤（fixture corpus 共有） | fixture corpus は driver-lab と共有される部分あり。Rust 側 `ruviform-ui-test` が正本 |
| `ui/showcase` | legacy | ショーケース + 検証 fixture（DOM 実装） | #263 の検証 fixture は driver-lab fixture へ移植済みのものが多い |
| `ui/browser-test` | legacy | Playwright + 実 Chromium ハーネス（Issue #310） | 実ブラウザ DOM 検証。WebView/DOM 前提のため P2 撤去候補筆頭 |

### 1.4 `poc/`・サンプル

| 資産 | 分類 | 備考 |
| --- | --- | --- |
| `poc/ui` | legacy | WebView 技術選定 PoC（wry+tao・Solid・WebGL2 実測）。ADR `ui-tech-selection-poc` の証拠元。履歴証拠として保持、P2 で削除判断 |
| `poc/plugin-gui` | legacy | Plugin GUI 埋込 PoC（旧 WebView 前提） |
| `crates/ruviform-ui-webview/examples/*` | legacy | `demo_app`/`demo_multi`/`e2e_smoke` は WebView adapter の動作確認用 |
| `crates/ruviform-ui/examples/demo_cli.rs`/`demo_dock.rs` | 現行 | headless 検証であり driver 非依存 |

### 1.5 mise / CI / lockfile / README

| 資産 | 分類 | 内容 |
| --- | --- | --- |
| `mise.toml` `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` | legacy lane | `ruviform-ui-webview --features webview` の検証 lane。CI `ui-verify` が呼ぶ |
| `mise.toml` `install-ui-*`/`setup-ui`/`check-ui-headless`/`test-ui-headless`/`check-ui-components`/`test-ui-components`/`build-ui-components`/`check-ui-viz`/`test-ui-viz`/`build-ui-viz`/`check-ui-binding`/`test-ui-binding`/`build-ui-binding`/`check-ui-guitest`/`test-ui-guitest`/`check-ui-showcase`/`test-ui-showcase`/`build-ui-showcase`/`check-ui-browser-test`/`test-ui-browser`/`install-ui-browser-chromium` | legacy lane | `ui/*` 各パッケージの npm 検証。CI `ui-verify`/`ui-verify-macos` が呼ぶ |
| `mise.toml` `test-ui-driver-contract`/`check-ui-driver-boundary`/`test-ui-parity`/`test-ui-native-slot`/`test-ui-component-view`/`test-ui-gpui`/`check-ui-gpui`/`lint-ui-gpui`/`test-ui-consumer-observation` | **現行** | driver-neutral 契約・GPUI driver の検証 lane |
| `mise.toml` `example-ui-webview-e2e` | legacy | WebView E2E 手動 example |
| `mise.toml` `poc-native-dock`/`poc-native-dock-layout` | 現行 | GPUI Native Dock 検証 |
| `.github/workflows/ci.yml` `ui-verify`/`ui-verify-macos` jobs | legacy lane | WebViewGTK 依存導入 + Playwright browser cache を含む。driver-neutral lane（`verify` 内の contract/boundary 呼出）とは別 job |
| `.github/workflows/consumer-verify.yml` の consumer 側 `setup-ui`/`check-ui`/`test-ui`/`build-ui` step | consumer 側 lane | consumer repo の mise task を呼ぶ情報 lane。consumer の WebView UI が残る間は必要 |
| `Cargo.toml` workspace members（`ruviform-ui-webview`、`poc/*`） | legacy member | P2 で member 除外判断。現時点では除外しない |
| `Cargo.lock` / `ui/*/package-lock.json` | 追跡継続 | 依存 pin の正本。変更なし |
| `README.md` / `crates/README.md` / `docs/development-environment.md` | 記述の歴史化対象 | WebView UI を現行機能として記述している箇所に legacy 注記を追加（P0） |
| `docs/adr/ui-architecture-webview-hybrid.md`/`ui-tech-selection-poc.md` | 歴史（superseded 済み） | 2026-11-07 に #384 ADR へ supersede 済み。banner あり、追記不要 |
| `docs/specs|designs|test-specs` Issue #310/#311/#312 系 | 歴史化対象 | WebView/DOM 前提の GUI テスト基盤 spec。`rvf.ipc.v1` schema と `ruviform-ui-test` の driver-neutral lane は共有/現行として区別して注記 |

## 2. Consumer repo 棚卸し（読み取りのみ・編集なし）

### 2.1 Synvelis（`F:/repositry/audio-products/synvelis`）

| 資産 | 現状 | 影響 |
| --- | --- | --- |
| `Cargo.toml` | `ruviform-ui`（必須）+ optional `ruviform-ui-webview`/`ruviform-ui-gpui`/`ruviform-ui-test`。features `webview`/`native` | WebView/GPUI 両 feature が併存 |
| `src/bin/synvelis.rs`（`synvelis` bin、feature `webview`） | `HostConfig`/`detect_backend`/`run`/`UiRuntime`/`dispatch_script`/`HostEvent::Ready`/`HostHandle::eval`/`request_close` を使用し `rvf://app` を serve | **lock pin `79d1db0a7ba82324f05a1a5ae934dbfa6f19d59d` では `HostConfig`/`detect_backend`/`run` が ruviform-ui-webview HEAD に存在しない**（#423 で削除済み）ため、webview lane は現行 HEAD への pin 更新で壊れる見込み。native 移行が前提 |
| `src/bin/synvelis-native.rs`（`synvelis-native` bin、feature `native`） | `GpuiDriver`/`WindowDriver`/`WindowCapabilities`/`NativeShell` を使用 | GPUI 標準 Driver 経路は成立 |
| `synvelis-workspace` | `WorkspaceManager<H: WindowHandle>`・`HostEvent`・`WindowHandle`・`WindowConfig` を generic に使用 | 旧 `WindowHandle` trait に依存。driver-neutral 移行には consumer 側改修が必要 |
| `ui/`（SolidJS frontend） | vendored `@ruviform/ui/*` を `sync-ruviform-ui.mjs` で `Cargo.lock` の ruviform SHA から再生成。`sync-deps`/typecheck/test/build script あり | WebView frontend 本体。vendored 物は ruviform `ui/*` の複製 |
| `ui/package.json`・`tools/` | `check-ruviform-rev` 系で lock SHA と vendor REV の整合を検証 | pin 更新時に vendor 再同期が必要 |

### 2.2 AudioLens（`F:/repositry/audio-products/audio-lens`）

| 資産 | 現状 | 影響 |
| --- | --- | --- |
| `crates/al-gui/Cargo.toml` | optional `ruviform-ui-webview`/`ruviform-ui-gpui` + `ruviform-ui`/`ruviform-ui-theme`/`ruviform-viz`/test crates。features `webview`/`gpui`/`gpui-test` | 両 driver 併存 |
| `src/bin/al-gui.rs`（`al-gui` bin、feature `webview`） | `SessionConfig`/`WebViewWindowConfig`/`detect_backend`/`run_multi`/`webview_available` を使用し `WindowSpec`/`url`/`devtools`/16ms tick の session を構成 | adapter 移行済み API のみ使用 → pin `54439fdc` では動作。HEAD でも `run_multi` 系は存続するため破壊的影響は小さい見込み（要 consumer 受入で確認） |
| `src/desktop.rs`（GPUI/native 内部） | driver-neutral `WindowDriver` を内部使用しつつ、lifecycle を旧 `ruviform_ui::HostEvent` へ正規化して consumer へ通知。`HostEvent::Ready` は `url` が WebView 固有のため写像しない旨を明記 | 旧 `HostEvent` を「正規イベント表現」として内部利用中。driver-neutral event への移行は consumer 側の設計判断 |
| `crates/al-gui/assets/` | WebView 同梱 frontend asset（`index.html`/`app.js`/`app.css`） | WebView lane の UI 実体 |
| `ui/`（headless TS） | vendored `ui/vendor/ruviform/ui/{core,components,binding}` を `tools/sync-ruviform-ui.mjs` で `Cargo.lock` SHA に同期。`node --test` + happy-dom | headless 検証用。WebView 本体ではないが vendored 資産は `ui/*` に依存 |
| `mise.toml` | `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` と `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` を別 lane で保持 | 両 lane が運用上区別されている |

### 2.3 consumer 共通の影響まとめ

- 両 consumer とも ruviform crates を **git pin**（synvelis `79d1db0a`、
  audio-lens `54439fdc`）で参照し、`ui/*` vendored asset を lock SHA に
  同期する仕組みを持つ。P2 で `ruviform-ui-webview`/`ui/*` を物理撤去
  すると、pin 更新時に consumer 側の webview feature/bin・vendored
  sync・WebView 向け mise lane が連鎖的に壊れる。
- consumer 側の WebView 撤去は各 repo の Issue（synvelis #175/#28 系、
  audio-lens #166/#127 系）で追跡される。本書は ruviform 側の記録であり
  consumer repo の編集は行わない。

## 3. 移行先の決定（記録）

| 旧面 | 移行先 | 状態 |
| --- | --- | --- |
| `HostEvent`/`HostHandle`/`WindowHandle`/`WindowConfig` | `driver::{WindowDriver,WindowSpec,WindowLifecycle,DriverEvent,WindowCapabilities}`（Issue #386） | 契約確立済み。adapter 実装は webview/gpui/test に存在 |
| `HostEvent::Message(String)` raw IPC | `WindowLifecycle::Action(Intent)`（typed） | WebView adapter が `postMessage` decode を担当 |
| `HostHandle::eval`/`WindowHandle::eval` | なし（driver-neutral に eval 相当なし。必要なら driver-specific extension） | WebView 固有のため撤去候補 |
| `RenderTier::{Dom,Canvas2d,Webgl2}` | GPUI element/canvas 描画 + ViewSpec | WebView 新規 parity 目標にはしない（#455 not-planned） |
| `render::NativeSurfaceHost` | `native_slot`（Issue #388） | 移行済み（README 明記） |
| `bridge`/`detect`/`rvf://` scheme 配信 | WebView adapter 内に封じ込め。driver-neutral 公開面からは外す方向 | P2 判断 |
| `ui/*` TS 資産 | headless 契約意味論は Rust 側（`behavior`/`view`/driver-lab fixture）へ移植済み。TS 実装の存続は consumer frontend の要否次第 | P2 判断 |
| `ui/browser-test`（Playwright） | driver-lab の Fake/GPUI parity fixture（`test-ui-driver-contract`/`test-ui-parity`）が新標準 | legacy lane として保持、P2 で縮退判断 |

## 4. P2 ゲート状態

- **P2（物理撤去）: NotRun / 対象外**。前提条件は #419 GPUI 採用判定
  （#262 verdict + #388/#389 実機証拠）と両 consumer の Native 受入。
  いずれも未達のため、本 P0/P1 では削除・member 除外・CI lane 撤去を
  一切行わない。
- Issue #465 は Open 維持。P0/P1 の記録・分類・注記は本チェーンの
  docs と `ruviform-ui` doc コメントで完結する。
