# ruviform-ui

Ruviform の再利用可能なデスクトップ GUI 基盤の WebView 非依存コア:
ホスト抽象・IPC スキーマ・サービス/ジョブランタイム・有界バイナリ転送・
アセット供給・描画責務契約を提供する。`wry`/`tao`/`wgpu` 等の
プラットフォーム GUI 依存は持たず、headless(CLI)経路から同一の
`Service` を駆動できる(Issue #156)。

## legacy 分類（Issue #465）

GUI 標準方針は Driver-neutral 契約 + GPUI 標準 Driver（Issue #384）。
本クレートは driver-neutral 契約（`driver`/`view`/`behavior`/`dock`/
`native_slot`/`service`/`runtime`/`ipc` schema）と旧 WebView 向け互換面が
同居する。分類の正本は
[docs/discovery/issue-465-legacy-webview-inventory.md](../../docs/discovery/issue-465-legacy-webview-inventory.md)。

- **現行**: `driver`・`view`・`behavior`・`dock`・`native_slot`・
  `service`/`runtime`・`ipc` の `rvf.ipc.v1` schema（`Intent`/`Event`/
  `ErrorBody` 等 — `WindowLifecycle::Action(Intent)` が直接使用）。
- **legacy（WebView adapter 互換・P2 で撤去判断）**: `event`
  （`HostEvent`/`HostHandle`）、`window`（`WindowHandle`/`WindowConfig`
  の `url`/`devtools`/`eval`）、`bridge`（`window.__rvf` JS transport）、
  `detect`（WebView backend 検出）、`render` の DOM/Canvas2d/Webgl2
  tier と `NativeSurfaceHost`（native dock 正本は `native_slot`）。
- **共有 semantics / WebView-bound wire**: `assets`/`data` —
  `AssetSource`/`DataRegistry` の意味論は driver 非依存だが、
  `rvf://app`/`rvf://data` scheme 配信は `ruviform-ui-webview` adapter
  実装。driver-neutral 配信契約は P2 の follow-up。
- `#[deprecated]` は consumer・`ruviform-ui-webview`・`ruviform-ui-test`
  が現役利用中のため付与しない（`-D warnings` lane 保護）。物理削除は
  Issue #465 P2 の別判断（gate: #419 採用判定 + consumer Native 受入）。

## 責務

- driver-neutral Window/Host 契約(`driver` モジュール・Issue #386):
  backend 非依存の `WindowSpec`/`WindowLifecycle`/`DriverEvent`/
  `WindowDriver`/`WindowCapabilities`。gpui/DOM/`wry`/OS handle を
  含まない。`Action(Intent)` が `Message(String)` の typed 後継、
  `UiError::Unsupported` + capability 宣言で silent degrade を禁止。
  実装は `ruviform-ui-gpui`(標準)・`ruviform-ui-webview`(legacy)・
  `ruviform-ui-test::FakeDriver`(test)。
- ホスト抽象(`event` モジュール): `HostEvent`(Tick/Focus/Resize/Close 等)と
  `HostHandle` による adapter 非依存の UI ライフサイクル通知
  (legacy — `driver` モジュールへ移行中)。
- IPC v1(`ipc` モジュール): `rvf.ipc.v1` 版付き JSON エンベロープ。
  Intent(呼出)/即時 Result/Job 開始/進捗 Event/cancel/終端 Error/
  notification を `id` 相関で扱う。`sys.*` はランタイム組込み op 予約域。
  - `state:"progress"` の `job` Event は optional の `detail`
    文字列(stage 名・処理中項目名)を添付できる(Issue #172)。
    `Service` job は `JobContext::progress_detail(done,total,detail)`
    で report 単位に付与する。detail は per-report で後続 report に
    持ち越されず、detail を持たない report は `detail` キー自体を
    emit しない(additive 拡張 — detail 非対応の consumer は
    従来どおり動作する)。latest-wins 合体では最新 report の
    detail が残る。
- サービス/ジョブ(`service`・`runtime` モジュール): `Service` trait +
  `UiRuntime` が intent 振り分け・ワーカースレッド実行・有界 job
  メールボックス(進捗は輻輳時 drop、終端信号は必達)・
  `CancellationToken` 連携を担う。
- 有界バイナリ(`data` モジュール): 名前付き不変 `DataSource` を
  `DataRegistry` 経由で range pull。1 読出上限 `max_chunk`(既定 16 MiB)、
  未登録名・不正オフセット・traversal 名は拒否。大容量データを
  JSON IPC に乗せないための境界。`rvf://data` の wire 契約
  (`offset`/`len`・`X-Rvf-Total`・status 対応・paging loop 手順)は
  [`ruviform-ui-webview` README](../ruviform-ui-webview/README.md)
  の paging 契約節を参照。
  - 解放契約(Issue #171): `unregister(name)` で個別解放、
    `clear()` で全解放、`max_sources` 上限(LRU 追い出し)で
    連番名が増殖する長時間 session でも登録数を有界化できる
    (`with_max_sources`/`with_limits`)。解放・追い出し後の pull は
    `NotFound`(wire 上は 404)。メモリは最後の `Arc` clone が
    drop した時点で解放されるため、進行中の pull は完了する。
- アセット(`assets` モジュール): `AssetSource` 抽象 +
  `MapAssetSource`。パストラバーサル・バックスラッシュ・制御文字を拒否。
- JS ブリッジ(`bridge` モジュール): `BRIDGE_JS`(フロント側薄ラッパ)と
  `dispatch_script`。
- 描画責務(`render` モジュール): `RenderTier`(Dom/Canvas2d/WebGl2/
  NativeSurface)・`SurfaceRegion`・`NativeSurfaceHost` 契約のみ。
  実装は adapter 側。DOM=コントロール、Canvas2D=互換/小規模、
  WebGL2=高頻度、wgpu ネイティブ面は測定要請時の別 adapter。
- バックエンド報告(`detect` モジュール): `BackendReport`/`BackendState`
  の型のみ。検出は adapter が行う。
- Dock 基盤(`dock` モジュール, Issue #182): ウィンドウ毎の Dock Tree
  (`Split(axis,ratio,children)` + `Tabs(views,active)`)、論理
  `ViewInstanceId` の全ウィンドウ横断で唯一の owner 検証、2 相 move
  (`begin_move` で view を transfer ロック → 宛先 window/webview 準備 →
  `commit_move` で atomic に owner 切替、失敗は元 dock へ rollback)、
  detached pool、consumer 選択の window-close disposition
  (`DetachViews`/`RelocateTo`/`ReleaseViews`)、versioned
  `LayoutSnapshot`(geometry + monitor signature + trees + detached pool)
  の保存/復元と off-screen・monitor 消失の補正。view state の
  save/restore は consumer 側 — WebView 間の DOM 転送は保証しない。
- Native Surface Slot(`native_slot` モジュール, Issue #388):
  driver-neutral なネイティブ埋込領域の契約 —
  `SurfaceSlotId`(`{index,generation}` の stale 検出可能 ID)、
  `SlotLease`(opaque・generation-bound・同時 1 lease/slot の
  parent lease)、`SurfaceSlotRegistry`(mint/validate/revoke/
  generation recycle を行う headless 純粋モデル)、GUI driver が
  実装する `NativeSurfaceSlot` port。`HWND`/`NSView`/X11/GPUI 型は
  持ち込まない。旧 `render::NativeSurfaceHost` は WebView 隣接
  surface 用として保持し、native dock 埋込の正本は本モジュール。
- デモ(`demo` モジュール): `DemoService` — synthesize/sleep/fail/panic/
  peaks を提供し、`demo_cli` example(headless)と
  `ruviform-ui-webview` の `demo_app` example(GUI)で共有する。

## 層・許可依存

- 層: 利用側層(engine と同順位・engine ではない)。
- 許可依存: `ruviform-exec`・`ruviform-rt`(Primitives のみ)。
  domain foundations・adapter への依存は持たない。
- `wry`/`tao`/window 型への依存は `ruviform-ui-webview` に閉じ込める。

## RT 契約

非 RT 専用。UI イベント処理・IPC・バイナリ転送を RT スレッドから
呼ばないこと。`Service` の job ワーカーは非 RT スレッドで実行される。

## 利用

```sh
# headless デモ(同一 DemoService を GUI 側 example と共有)
cargo run -p ruviform-ui --example demo_cli

# headless Dock デモ(dock/move/rollback/detach/restore のナレーション)
cargo run -p ruviform-ui --example demo_dock
```

## 関連

- 設計: docs/designs/issue-156-gui-host-foundation.md
- 棚卸し: docs/discovery/issue-156-gui-host-foundation-inventory.md
- UI ADR: docs/adr/ui-architecture-webview-hybrid.md,
  docs/adr/ui-tech-selection-poc.md
- Dock 基盤(Issue #182): docs/specs/issue-182-dock-tree.md,
  docs/designs/issue-182-dock-tree.md,
  docs/adr/issue-182-dock-tree-placement-move-contract-and-persistence.md
- Native Surface Slot(Issue #388): docs/specs/issue-388-native-surface-slot.md,
  docs/designs/issue-388-native-surface-slot.md
- adapter: [`ruviform-ui-webview`](../ruviform-ui-webview/README.md)
