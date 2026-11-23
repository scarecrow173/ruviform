---
id: "DISC-PZXzzHRXfnvatIchgHgLEJ"
type: "discovery"
status: "resolved"
title: "Issue #465 Legacy WebView 基盤の棚卸し — 撤去対象・移行先・consumer 影響"
created: "2026-11-10"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "https://github.com/scarecrow173/ruviform/issues/455"
    - "https://github.com/scarecrow173/audio-lens/issues/189"
    - "https://github.com/scarecrow173/synvelis/issues/232"
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
    - "evidence/rf465/inventory.json"
    - "evidence/rf465/README.md"
  defers: []
  deferred-by: []
---

# Issue #465 Legacy WebView 基盤の棚卸し

Issue #465 P0/P1 の調査成果物。旧 WebView-first GUI 基盤の全構成要素を
「現行使用 / legacy 保持 / 撤去候補」に分類し、driver-neutral 側の
移行先と consumer 影響を記録する。**本書は記録のみであり、物理撤去
（P2）は両 consumer の legacy 参照・配布・CI 撤去（AL #189 / SV #232）
と Native 受入を gate とする別 PR の判断事項である**。#419 GPUI 採用
判定は 2026-11-16 に CONDITIONAL 採用（#384 ADR status=accepted、
PR #718）として決裁済み、#532 DComp 制約も 2026-11-21 に close 済み
（§4 参照）のため、採用判定 gate 自体は解消している。残る gate は
製品側撤去と Native 受入であり、P2 は引き続き NotRun。

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
| `ui/guitest` | legacy 寄り | 決定的 GUI テスト基盤（fixture corpus 共有） | `src/` は TS 実装（`ui/{binding,core}` へ相対 import・SV vendor）で **保留**、`fixtures/` は Rust/TS 共用正本で **削除禁止**（§5 行別判定） |
| `ui/showcase` | legacy | ショーケース + 検証 fixture（DOM 実装） | #263 の検証 fixture は driver-lab fixture へ移植済みのものが多い。行別判定は **削除可（`ui/browser-test` と原子的撤去）**（§5） |
| `ui/browser-test` | legacy | Playwright + 実 Chromium ハーネス（Issue #310） | 実ブラウザ DOM 検証。WebView/DOM 前提のため P2 撤去候補筆頭。行別判定は **削除可（`ui/showcase` と原子的撤去）**（§5） |

### 1.4 `poc/`・サンプル

| 資産 | 分類 | 備考 |
| --- | --- | --- |
| `poc/ui` | legacy | WebView 技術選定 PoC（wry+tao・Solid・WebGL2 実測）。ADR `ui-tech-selection-poc` の証拠元。履歴証拠として保持 — Issue 受入条件「歴史的 PoC は保管」により本監査では **例外（保持）**（§5） |
| `poc/plugin-gui` | legacy | Plugin GUI 埋込 PoC（旧 WebView 前提）。同上 **例外（保持）** |
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
| ルート `Cargo.toml` workspace members（`ruviform-ui-webview`） | legacy member | P2 で member 除外判断。現時点では除外しない。`poc/ui`・`poc/plugin-gui` は自前 `[workspace]` と lockfile を持つ独立ワークスペースで root members ではない（§1.4 参照） |
| `Cargo.lock` / `ui/*/package-lock.json` | 追跡継続 | 依存 pin の正本。変更なし |
| `README.md` / `crates/README.md` / `docs/development-environment.md` | 記述の歴史化対象 | WebView UI を現行機能として記述している箇所に legacy 注記を追加（P0） |
| `docs/adr/ui-architecture-webview-hybrid.md`/`ui-tech-selection-poc.md` | 歴史（superseded 済み） | 2026-11-07 に #384 ADR へ supersede 済み。banner あり、追記不要 |
| `docs/specs|designs|test-specs` Issue #310/#311/#312 系 | 歴史化対象 | WebView/DOM 前提の GUI テスト基盤 spec。`rvf.ipc.v1` schema と `ruviform-ui-test` の driver-neutral lane は共有/現行として区別して注記 |

## 2. Consumer repo 棚卸し（読み取りのみ・編集なし）

### 2.1 Synvelis（`G:/repositry/audio-products/synvelis`、2026-11-24 再実測）

| 資産 | 現状 | 影響 |
| --- | --- | --- |
| `Cargo.toml` | `ruviform-ui`（必須）+ optional `ruviform-ui-webview`/`ruviform-ui-gpui`/`ruviform-ui-test`（`Cargo.toml:77`・`crates/synvelis-app/Cargo.toml:35,46-47`）。features `webview`/`native` | WebView/GPUI 両 feature が併存。**現行 lock pin は `cd4213a1d665600f6adfefa0c3a90f2be9b4e4ce`**（`ruviform-ui{,-gpui,-test,-theme,-webview}` 同一 SHA・2026-11-24 `Cargo.lock` 実測） |
| `crates/synvelis-app/src/bin/synvelis.rs`（`synvelis` bin、feature `webview`） | **現行 pin では #423 移行後 API（`SessionConfig`/`WebViewWindowConfig`/`detect_backend`/`run_multi`、`:39`）を使用**し `rvf://app` を serve | P0/P1 時点の「`HostConfig`/`run` が HEAD に存在せず pin 更新で壊れる」記述は陳腐化 — bin は移行後 API へ改修済み。`src/assets.rs`（`EmbedAssets`・webview feature 限定）と `src/host.rs`（`ruviform_ui::window::WindowHandle` 旧 port 上の `ShellWindowHost`/`HeadlessWindowHost`）は旧 API 面に現役依存 |
| `crates/synvelis-app/src/bin/synvelis-native.rs`（`synvelis-native` bin、feature `native`） | `GpuiDriver`/`WindowDriver`/`WindowCapabilities`/`NativeShell` を使用 | GPUI 標準 Driver 経路は成立。SV PR #618（2026-11-23 merge）で headless PluginDock panel（driver-neutral ViewSpec/Command + 4-ID session/slot 状態機械）が着地、#619 で doc chain 登録 |
| `synvelis-workspace` | `WorkspaceManager<H: WindowHandle>`・`HostEvent`・`WindowHandle`・`WindowConfig` を generic に使用 | 旧 `WindowHandle` trait に依存。driver-neutral 移行には consumer 側改修が必要 |
| `ui/`（SolidJS frontend） | vendored `@ruviform/ui/{core,components,binding,viz,guitest}` を `sync-ruviform-ui.mjs`（`PACKAGES` 実測）で `Cargo.lock` の ruviform SHA から再生成。`ui/test/conformance.test.ts` が `@ruviform/ui-guitest`（`ui/tsconfig.json:24` alias → `vendor/ruviform/ui/guitest/src`）を 3 lane 共有 corpus として import | WebView frontend 本体。**`ui/guitest` は fixtures+src を vendored 利用しており consumer 現役依存**。`ui/showcase`・`ui/browser-test` は PACKAGES・参照ともにゼロ（grep 実測） |
| `ui/package.json`・`tools/` | `check-ruviform-rev` 系で lock SHA と vendor REV の整合を検証 | pin 更新時に vendor 再同期が必要。SV `.github/workflows/` には webview 参照ゼロ（実測 — webview 系は local mise task のみ） |

### 2.2 AudioLens（`G:/repositry/audio-products/audio-lens`、2026-11-24 再実測）

| 資産 | 現状 | 影響 |
| --- | --- | --- |
| `crates/al-gui/Cargo.toml` | optional `ruviform-ui-webview`/`ruviform-ui-gpui` + `ruviform-ui`/`ruviform-ui-theme`/`ruviform-viz`/test crates。features `webview`/`gpui`/`gpui-test`（`Cargo.toml:77`・`crates/al-gui/Cargo.toml:16,38`） | 両 driver 併存。**現行 lock pin は `e4c9f6f0715da3ac479f8c437d21a021327e51da`**（`ruviform-ui{,-gpui,-test,-theme,-webview}` 同一 SHA・2026-11-24 `Cargo.lock` 実測） |
| `crates/al-gui/src/bin/al-gui.rs`（`al-gui` bin、feature `webview`） | `SessionConfig`/`WebViewWindowConfig`/`detect_backend`/`run_multi`/`webview_available`（`:14`）を使用し `WindowSpec`/`url`/`devtools`/16ms tick の session を構成 | adapter 移行済み API のみ使用 → 現行 pin で動作。HEAD でも `run_multi` 系は存続するため破壊的影響は小さい見込み（要 consumer 受入で確認） |
| `crates/al-gui/src/bin/al-desktop.rs`（`al-desktop` bin、feature `gpui`/`gpui-test`） | `GpuiDriver`/`WindowDriver`/`ViewMount` 組立の Native GUI | GPUI 標準 Driver 経路は成立（P0/P1 後に独立 bin 化 — AL-003 相当） |
| `crates/al-gui/src/desktop.rs`（GPUI/native 内部） | driver-neutral `WindowDriver` を内部使用しつつ、lifecycle を旧 `ruviform_ui::HostEvent` へ正規化（`map_lifecycle`・`:414-425`）して consumer へ通知。`HostEvent::Ready` は `url` が WebView 固有のため写像しない旨を明記 | 旧 `HostEvent` を「正規イベント表現」として内部利用中。driver-neutral event への移行は consumer 側の設計判断 |
| `crates/al-gui/assets/` | WebView 同梱 frontend asset（`index.html`/`app.js`/`app.css` — commit 済み実在） | WebView lane の UI 実体 |
| `ui/`（headless TS） | vendored `ui/vendor/ruviform/ui/{core,components,binding,viz}` を `tools/sync-ruviform-ui.mjs`（`PACKAGES` 実測・`ui/guitest` は含まない）で `Cargo.lock` SHA に同期。`node --test` + happy-dom | headless 検証用。WebView 本体ではないが vendored 資産は `ui/*` に依存。`ui/showcase`・`ui/browser-test`・`ui/guitest` への参照はゼロ（grep 実測） |
| `mise.toml` | `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` と `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` を別 lane で保持 | 両 lane が運用上区別されている |
| `.github/workflows/ci.yml` + `.github/actions/webview-verify` | `webview-verify`（ubuntu/windows matrix）+ `webview-verify-macos` jobs が al-gui webview lane を実行（`ci.yml:277-304`） | AL の webview lane は **CI 現役** — SV と異なり headless/local 限定ではない |

### 2.3 consumer 共通の影響まとめ（2026-11-24 再実測）

- 両 consumer とも ruviform crates を **git pin** で参照し、`ui/*`
  vendored asset を lock SHA に同期する仕組みを持つ。**現行 pin は
  synvelis `cd4213a1`・audio-lens `e4c9f6f0` で一致していない**
  （`git merge-base` 実測: `e4c9f6f0` は `cd4213a1` の ancestor —
  AL がより古い pin を保持。P0/P1 時点の per-repo 値
  `79d1db0a`/`54439fdc`・evidence/rf465 時点の共通値 `fd1d4d5f`
  から遷移し、tuple は現在 2 値に分岐）。両 pin とも現行
  `origin/main`（`fc355d5c`）の ancestor。
- P2 で `ruviform-ui-webview`/`ui/*`（consumer vendor 対象）を物理撤去
  すると、pin 更新時に consumer 側の webview feature/bin・vendored
  sync・WebView 向け mise/CI lane が連鎖的に壊れる。`ui/showcase`・
  `ui/browser-test` は両 consumer いずれからも参照されないため、
  この連鎖の対象外（§5 行別判定の「削除可」根拠）。
- consumer 側の WebView 撤去は各 repo の Issue で追跡される。RF#465
  P1 の「実際の撤去に必要な製品固有 Issue/PR を各 repo で明示的に
  管理」と audio-lens#127 の「物理削除は独立した AudioLens の実装
  Issue/PR に分解」の要求に対応し、物理撤去の専任 Owner Issue が
  起票済み（いずれも OPEN・2026-11-09 起票）:
  - AudioLens [#189](https://github.com/scarecrow173/audio-lens/issues/189)
    — `al-gui` の webview feature/bin・`ui/` TS assets・vendor 同期・
    IPC・CI/配布の撤去を担う。`rvf.ipc.v1` schema は共有維持として
    区別済み、撤去 Gate は RF#419 採用判定 + RF#465 撤去計画 + 独立
    精度 Gate 充足が前提。
  - Synvelis [#232](https://github.com/scarecrow173/synvelis/issues/232)
    — `ruviform-ui-webview` optional dep・webview bin・HostConfig/
    assets/CI の撤去を担う。撤去 Gate は RF#419 採用判定 + RF#465
    撤去計画 + 製品 Gate 充足が前提。
  - 従来の追跡系列は synvelis #175（closed・先代 GUI lane）/#28 系・
    audio-lens #166（closed・GPUI 移行履歴）/#127 系（基盤追従）。
    本書は ruviform 側の記録であり consumer repo の編集は行わない。

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

## 4. P1 引き渡し状況と P2 ゲート状態（2026-11-24 現行化）

- **P1（per-repo Issue 管理）: 引き渡し済み**。RF#465 P1 の要求どおり、
  consumer 側の物理撤去は各 repo の専任 Issue — AudioLens #189・
  Synvelis #232（ともに OPEN）— が Owner であり、ruviform 側は本書の
  棚卸し・分類・consumer 影響記録（§2）を引き渡し証跡とする。
  ruviform 側の P0/P1 記録・分類・doc コメント注記は PR #475 で
  main へ反映済み（doc chain: spec/design/ADR/plan/task、task status
  `done`）。consumer 撤去 Owner の引き渡し記録は PR #535、現行 main
  基準の機械可読台帳は PR #616 系の `evidence/rf465/` が正本。
- **採用判定 gate: 解消済み（2026-11-16 決裁）**。#419 は CLOSED —
  GPUI 後継 ADR（`driver-neutral-gui-gpui-driver-webview-first-
  issue-384.md`）は PR #718 で status=accepted・**CONDITIONAL 採用**
  として確定（GPUI Native First を標準 Driver とする方針は維持、
  Native Dock の無条件本番採用は不可）。#262（採否 verdict）・#294
  （cross-repo CI）・#464（consumer-neutral scenario）・#627
  （virtual-first 検証契約）もすべて CLOSED。
- **#532 DComp 制約: CLOSED（2026-11-21）**。class A（compositor）=
  `WS_EX_NOREDIRECTIONBITMAP` + flip-model 提示面の child 領域被覆を
  方式 A（GPUI fork 最小修正 clip-partition、gpui v0.2.2-rf2）で根治。
  class B（#857）は処遇済み。class C（bandescape・childinv =
  `WS_EX_LAYERED` 適用不能確定）は **honest Fail として維持・
  re-baseline せず**。実走証拠 29 case = 24 pass + 3 unsupported +
  2 fail（class C）。**行別撤去判定への影響**: Plugin Native Dock の
  L3 完全受入は class C residual が残る限り未達 — 「Native Dock 成立」
  を前提とする撤去行は存在しないが、順序ルールの実質 gate である
  「両 consumer の Native 受入」はこの honest Fail を含め未完である
  ことを明記する。
- **残 gate（P2 実施条件）**: AL #189・SV #232（製品側 legacy 参照・
  配布・CI 撤去、ともに OPEN）と、Windows で実際に提供・利用する
  機能の L2/L3 Native 受入 — virtual-first の証拠経路である #630
  （実 ABI Test Native Editor）・#631（実 Window Scenario Runner）は
  ともに OPEN。SV 側は PR #618/#619（2026-11-23、headless PluginDock
  panel + doc chain）が着地し Native 側基盤は進行中。
- **P2（物理撤去）: NotRun / 対象外**。上記残 gate 未達のため、本
  slice でも削除・member 除外・CI lane 撤去を一切行わない。行別の
  削除可否判定と監査記録のみを §5 に確定する。
- Issue #465 は Open 維持。P0/P1 の記録・分類・注記は本チェーンの
  docs と `ruviform-ui` doc コメントで完結する。

## 5. 行別物理削除判定監査（2026-11-24 再実測）

Issue 本文の判定ルール（2026-11-10 節・正本）に照らした行別監査。
**本節は判定の記録のみ — 物理削除は次 slice 以降の対象**。

### 5.1 判定ルール（正本）

- 削除可は「**Legacy consumer 利用なし・独立 Oracle PASS・同一 SHA
  tuple PASS・rollback 可能**」の 4 条件をすべて満たす行だけ。
- 削除順は「**AL #189 / SV #232 の製品 legacy 参照・配布・CI の候補
  差分を独立計測 → Ruviform 共通 WebView Adapter 削除**」。製品が旧
  依存を保持したまま共通 Adapter を先に消すことは禁止。
- 判定は「**削除可 / 保留 / 例外（+理由）**」で記録する。

### 5.2 監査手順と観測値

対象 SHA: ruviform `origin/main` = `fc355d5c`（2026-11-24 作業時点）。

(a) **consumer/vendor 参照 grep**（読み取り専用・consumer 変更なし）:

- 両 repo の `tools/sync-ruviform-ui.mjs` `PACKAGES` 実測 —
  synvelis `["ui/core","ui/components","ui/binding","ui/viz","ui/guitest"]`、
  audio-lens `["ui/core","ui/components","ui/binding","ui/viz"]`。
  `ui/showcase`・`ui/browser-test` はいずれにも含まれない。
- 全ソース grep（`.ts/.tsx/.mjs/.js/.json/.rs/.toml`、node_modules・
  vendor 除外）: `showcase`/`browser-test` への参照は synvelis 0 件、
  audio-lens 0 件（`.sdp` skill cache 内の無関係な汎用文言のみ）。
- consumer lock pin 実測: 各 `Cargo.lock` の
  `name = "ruviform-ui*"` エントリ `source` — AL `e4c9f6f0715da3ac479f8c437d21a021327e51da`、
  SV `cd4213a1d665600f6adfefa0c3a90f2be9b4e4ce`（§2.3）。
- consumer 現役依存 実測: AL `Cargo.toml:77` +
  `crates/al-gui/Cargo.toml:16,38` + `src/bin/al-gui.rs:14`（`run_multi`
  系）+ `crates/al-gui/assets/` + `ci.yml:277-304` webview-verify
  jobs。SV `Cargo.toml:77` + `crates/synvelis-app/Cargo.toml:35,46-47` +
  `src/bin/synvelis.rs:39` + `src/assets.rs`（`EmbedAssets`）+
  `src/host.rs`（旧 `WindowHandle` port）+ `ui/test/conformance.test.ts`
  の `@ruviform/ui-guitest` import。

(b) **fixture 共有監査**:

- `ui/guitest/fixtures` は `crates/ruviform-ui/tests/ipc_conformance.rs:24`
  （`../../ui/guitest/fixtures` — rvf.ipc.v1 schema + rvf://data
  conformance の versioned corpus 単一正本）と
  `crates/ruviform-ui-webview/src/scheme.rs:308`
  （`fixtures/ipc/data.v1.json`）が直接読む。`ipc_conformance` は
  **現行 crate `ruviform-ui` 内の共有契約テスト**であり、webview
  adapter 撤去後も残る側の資産が本 corpus に依存する。
- driver-lab fixture（`crates/ruviform-ui-test/fixtures/driver-lab/`:
  `cases.json`/`stories.json`/`parity/viz-semantic-{audiolens,synvelis}.json`/
  `themes/{audiolens,synvelis}-dark.dtcg.json`/`boundary/`）は別 corpus
  であり、guitest の ipc/binding/scenario corpus を複製していない。
- `ui/showcase` の検証 fixture は driver-lab へ部分移植済み（theme・
  viz-semantic 系）。未移植残（DOM mount/a11y/theme-matrix の
  showcase 固有ケース）は legacy TS パッケージ自身のカバレッジであり、
  保持契約（`rvf.ipc.v1`・driver-neutral・`ui/guitest/fixtures`）を
  検証するものではない。

(c) **代替 Oracle（driver-neutral/headless）実在照合** — `mise.toml`
  で lane 定義を実在確認（`[tasks.*]` 実測）:

- `test-ui-driver-contract`/`test-ui-parity`/`check-ui-driver-boundary`/
  `test-ui-native-slot`/`test-ui-component-view`/`test-ui-golden`/
  `test-ui-scenario-contract{,-fake,-gpui}`/`test-ui-consumer-observation`/
  `test-ui-real-scenario`/`test-ui-offscreen-xvfb`/
  `test-ui-real-os-scenario`/`test-ui-windowed-uitest`/
  `test-ui-plugin-dock-real-{headless,clap,clap-headless}` — すべて存在。
- CI: `verify`（driver boundary/contract 呼出）・`ui-gpui-verify{,-macos}`・
  `ui-gpui-offscreen-verify` jobs が現行 lane を実行。legacy TS lane は
  `ui-verify`（ubuntu/windows matrix）・`ui-verify-macos` が担当。

### 5.3 行別判定結果

| # | 行 | 判定 | 根拠（5.2 の観測） |
| --- | --- | --- | --- |
| D1 | `ui/browser-test`（package 全体 — `package.json`/`src`/`pages`/`page`/`test`/`playwright.config.ts`/`tsconfig`） | **削除可（条件付き）** | consumer 利用なし（(a)）、独立 Oracle 実在（(c) — driver-lab parity/contract + GPUI lanes）、tuple 無影響（両 consumer が非参照）、rollback 可（git）。条件: D2 と原子的撤去（`@rvf/showcase` → `../showcase/dist` 依存のため）+ mise `install-ui-browser-test`/`check-ui-browser-test`/`test-ui-browser`/`install-ui-browser-chromium` + `setup-ui` 集約 + ci.yml `ui-verify`/`ui-verify-macos` の Playwright cache/install-deps/validate/test/artifact upload step + `ci/daily/tests/ci-yml-contract.test.mjs` の job 契約を同一変更で撤去 |
| D2 | `ui/showcase`（package 全体 — `src`/`test`/`examples/showcase.html`/`tsconfig*`） | **削除可（条件付き）** | consumer 利用なし（(a)）、独立 Oracle 実在（(c)）、tuple 無影響、rollback 可。src は `../../{components,viz,binding,core}/src` へ import する利用側であり逆方向依存なし。未移植 fixture 残は legacy パッケージ自身のカバレッジで保持契約を検証しない（(b)）。条件: D1 と原子的撤去 + mise `install-ui-showcase`/`check-ui-showcase`/`test-ui-showcase`/`build-ui-showcase` + ci.yml 両 job の showcase step を同一変更で撤去。残余注意: 残存 `ui/{core,components,viz,binding,guitest}` の DOM 統合カバレッジは縮退するが、各 pkg の `node --test` 単体 test と consumer 側 happy-dom/conformance test が残存し、保持契約の検証を失わない |
| D3 | `crates/ruviform-ui-webview`（crate 本体 + `webview` feature + optional deps wry/tao/http/dpi/rust-embed + `examples/` + `example-ui-webview-e2e` + `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` lanes） | **保留** | 順序ルール: 両 consumer が git dep + `webview` feature + bin で現役利用（(a)）。AL #189/SV #232 製品撤去完了前の削除は禁止 |
| D4 | `ui/core`・`ui/components`・`ui/viz`・`ui/binding`（package 全体） | **保留** | 両 consumer vendor 対象（(a) PACKAGES 実測）+ `ui/{showcase,guitest}` の import 元。consumer 撤去と vendor 解消が先 |
| D5 | `ui/guitest`（`src/` TS 実装 + package） | **保留** | SV vendor + `ui/test/conformance.test.ts`（`@ruviform/ui-guitest` alias）が現役依存（(a)）。`fixtures/` は D8 で別判定 |
| D6 | `crates/ruviform-ui` 旧 API 面（`HostEvent`/`HostHandle`/`WindowHandle`/`WindowConfig`/`RenderTier` DOM 系/`bridge`/`detect` + `assets`/`data` shared-port + 関連 tests） | **保留** | SV `host.rs`/`synvelis-workspace`・AL `desktop.rs` 正規化・`ruviform-ui-test` FakeHost/FakeWindow・`crates/ruviform-ui/tests/` が現役利用（§1.2）。consumer・テスト基盤の両方が依存 |
| D7 | mise legacy `ui-*` lanes（vendored pkg の `install/check/test/build-ui-{core,components,viz,binding,guitest}`・`install-ui-*` 集約・`setup-ui`）+ ci.yml `ui-verify`/`ui-verify-macos` + `consumer-verify.yml` の consumer task 呼出 | **保留** | consumer sync・現行 CI が現役使用。D1〜D6 の全行撤去と同時整理。`ci-yml-contract.test.mjs:269-282` が job/lane 存在を assert するため契約 test も同一変更で更新 |
| D8 | `ui/guitest/fixtures` | **例外（削除禁止・移転完了まで）** | 保持契約（`rvf.ipc.v1` + `rvf://data`）の単一正本 corpus — 現行 crate `ruviform-ui` の `ipc_conformance.rs:24` が直接読む（(b)）。webview `scheme.rs:308`・`ui/guitest` TS lane・SV vendor も共用。移転先未決定 — 移転と参照書換えが済むまで一切削除しない |
| D9 | `poc/ui`・`poc/plugin-gui` | **例外（保持）** | Issue 受入条件「歴史的 ADR/PoC/Issue/数値・GUI 試験結果は保管」。独立 `[workspace]` で root members 外 — 保持しても共通撤去の妨げにならない |
| D10 | AL 側行（情報記録・Owner は AL #189）: `ruviform-ui-webview` dep/`webview` feature/`al-gui` bin/`crates/al-gui/assets/`/`ui/` vendored layer/`webview-verify{,-macos}` jobs/`check|lint|test-ui-webview` lanes | **保留（consumer repo 側）** | 製品撤去 Issue の対象。ruviform 側 D3〜D7 の解除条件となる先行撤去 |
| D11 | SV 側行（情報記録・Owner は SV #232）: `ruviform-ui-webview` dep/`webview` feature/`synvelis` bin/`assets.rs`/`host.rs` 旧 port/`ui/`+vendored layer（`guitest` 含む）/mise webview tasks | **保留（consumer repo 側）** | 同上。SV workflows に webview 参照がないことは ruviform 側判定を変えない（cargo dep/bin/vendor は現役） |

### 5.4 判定上の注意（次 slice への引き渡し）

- **tuple の分岐**: 「同一 SHA tuple PASS」評価の対象である consumer pin
  は現時点で 2 値に分岐している（§2.3）。撤去 slice は「撤去 PR を
  含む ruviform SHA」に対して AL・SV それぞれの pin 更新 + 各 repo CI
  で検証する前提で進める — 単一 tuple 前提の検証手順は陳腐化。
- **D1/D2 の実施形態**: 2 行は 1 PR で原子的に撤去し、記載した mise
  lane・CI step・contract test を同一変更で整理する
  （`deps.test.ts` の package ポリシ記述は `ui/showcase` 内に同居し、
  package と共に除去される）。部分撤去（browser-test のみ等）は
  `ui/showcase/dist` 参照・deps ポリシが壊れるため不可。
- **D8 の移転**: `ui/guitest/fixtures` の移転先（候補:
  `crates/ruviform-ui-test/fixtures/` または `crates/ruviform-ui` 直下）
  と参照書換え（Rust 2 箇所 + `ui/guitest` TS lane + SV vendor 対応）
  は未実施 — 次 slice の検討事項として明示的に残す。
- **未監査行**: `crates/ruviform-ui-webview` 内の個別モジュール粒度
  （`embed`/`session`/`detect` 等）や `ui/guitest/src` のファイル粒度
  の撤去順は D3/D5 の保留解除後に改めて監査する。本節は package/
  資産行レベルの判定。
- **L3 判定での #532 class C の扱い**: class C residual（honest
  Fail 維持）は撤去対象行の L3 判定を「Fail → 削除不可」とはしない —
  該当機能（Plugin Native Dock・childinv 経路）は撤去対象外の現行
  機能であり、撤去判定の 4 条件とは独立。ただし「両 consumer の
  Native 受入」（順序ルールの実質 gate）の完了判定には class C の
  処遇（許容か制約記録か）を含める。
