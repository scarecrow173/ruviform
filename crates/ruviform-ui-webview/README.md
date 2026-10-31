# ruviform-ui-webview

`ruviform-ui` ポートの `wry` + `tao` 実装: ウィンドウ/イベントループ・
WebView ホスト・`rvf://` カスタムプロトコル・組込みアセット・
バックエンド検出を提供するオプションのプラットフォーム adapter
(Issue #156)。採択済み UI ADR(WebView-first hybrid)に従う。

## 責務

- ホスト(`host` モジュール・feature `webview`): tao イベントループ上に
  wry WebView を構築し、`HostEvent` へ変換して `UiRuntime` へ流す。
  `EventLoopProxy` による WebView2 コールバックの wakeup、resize /
  scale-factor での WebView bounds 再適用、Close/Focus/Tick 転送を担う。
- スキーム(`scheme` モジュール・feature 非依存): `rvf://app` アセット配信と
  `rvf://data/<name>` 有界 range pull。WebView2 が書き換える
  `http://rvf.app` / `http://rvf.data` ホスト形も受理する。
  ナビゲーションはアプリ起点 URL と `about:blank` のみ許可。
- 組込みアセット(`embed` モジュール): `rust-embed` で
  `examples/demo_app.html` 等を単一バイナリへ埋め込む。
- 検出(`detect` モジュール): `wry::webview_version` による
  `BackendReport` 生成(バックエンド未導入時は `BackendState::Missing`)。

## feature

- `webview`(非 default): `wry`/`tao`/`http`/`dpi`/`rust-embed` を有効化し
  ホスト実装を公開する。feature なしでは `scheme` の純粋ロジックのみ
  コンパイルされるため、headless 環境でもスキーム単体テストが可能。

## OS 依存

| OS | バックエンド | 要件 |
| --- | --- | --- |
| Windows | WebView2 | WebView2 Runtime(Win10 1803+ の多くに同梱・未導入時は Evergreen ブートストラップ)。プロセスごとの user-data フォルダを使う(PoC 実測のシングルトンロック対策) |
| macOS | WKWebView | macOS 10.15+(wry 要件)。実機未検証 |
| Linux | WebKitGTK | `libwebkit2gtk-4.1-dev` `libgtk-3-dev` `libsoup-3.0-dev` `libjavascriptcoregtk-4.1-dev`。実機未検証 |

既知の制約(PoC 実測由来・詳細は discovery 文書):
WebView2 はカスタムプロトコル URL を `http://` ホストへ書き換える、
composition でネイティブ面が常に最背面にならない制限があるため
`wgpu` 面は別 adapter、小さな pull(~64 KiB)は JSON RTT より遅い
(閾値以上のバルクのみ pull を使う)。

## 層・許可依存

- 層: Optional platform adapter(利用側層)。
- 許可依存: `ruviform-ui` のみ(foundations への下向き強制・
  adapter→adapter 依存は作らない)。

## RT 契約

非 RT 専用。ウィンドウ/イベントループ/UI スレッド処理であり
RT パスからは呼ばない。

## 利用

```sh
# GUI デモ(ruviform-ui の demo_cli と同一 DemoService)
cargo run -p ruviform-ui-webview --features webview --example demo_app
```

検証タスク(feature 有効 lane):
`mise run check-ui-webview` / `lint-ui-webview` / `test-ui-webview`。
CI は `ui-verify` ジョブが 3 OS matrix で実行する
(Linux は WebKitGTK 開発パッケージをジョブ内で導入)。

## 関連

- コア: [`ruviform-ui`](../ruviform-ui/README.md)
- 設計: docs/designs/issue-156-gui-host-foundation.md
- UI ADR: docs/adr/ui-architecture-webview-hybrid.md
