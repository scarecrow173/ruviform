# ruviform-ui

Ruviform の再利用可能なデスクトップ GUI 基盤の WebView 非依存コア:
ホスト抽象・IPC スキーマ・サービス/ジョブランタイム・有界バイナリ転送・
アセット供給・描画責務契約を提供する。`wry`/`tao`/`wgpu` 等の
プラットフォーム GUI 依存は持たず、headless(CLI)経路から同一の
`Service` を駆動できる(Issue #156)。

## 責務

- ホスト抽象(`event` モジュール): `HostEvent`(Tick/Focus/Resize/Close 等)と
  `HostHandle` による adapter 非依存の UI ライフサイクル通知。
- IPC v1(`ipc` モジュール): `rvf.ipc.v1` 版付き JSON エンベロープ。
  Intent(呼出)/即時 Result/Job 開始/進捗 Event/cancel/終端 Error/
  notification を `id` 相関で扱う。`sys.*` はランタイム組込み op 予約域。
- サービス/ジョブ(`service`・`runtime` モジュール): `Service` trait +
  `UiRuntime` が intent 振り分け・ワーカースレッド実行・有界 job
  メールボックス(進捗は輻輳時 drop、終端信号は必達)・
  `CancellationToken` 連携を担う。
- 有界バイナリ(`data` モジュール): 名前付き不変 `DataSource` を
  `DataRegistry` 経由で range pull。1 読出上限 `max_chunk`(既定 16 MiB)、
  未登録名・不正オフセット・traversal 名は拒否。大容量データを
  JSON IPC に乗せないための境界。
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
```

## 関連

- 設計: docs/designs/issue-156-gui-host-foundation.md
- 棚卸し: docs/discovery/issue-156-gui-host-foundation-inventory.md
- UI ADR: docs/adr/ui-architecture-webview-hybrid.md,
  docs/adr/ui-tech-selection-poc.md
- adapter: [`ruviform-ui-webview`](../ruviform-ui-webview/README.md)
