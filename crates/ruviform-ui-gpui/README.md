# ruviform-ui-gpui

`ruviform-ui` の driver-neutral Window/Host 契約(`ruviform_ui::driver`、
Issue #386)に対する GPUI 0.2.2 標準 Driver 実装。`gpui` 型は
crate 内 `mod gpui_driver` に封じ込め、公開面は中立型のみ —
`GpuiDriver`(`WindowDriver` 実装)と `IntentSink`(view 側 typed
Rust action ingress)。

## 責務

- `GpuiDriver`(`mod gpui_driver`・feature `gpui`): gpui
  `Application` イベントループ上で中立 `WindowDriver` を実装。
  `create_window` は `WindowId` を即時払い出し、OS 生成は
  deferred op として loop 内 drain で適用。close flow は
  `on_window_should_close` で veto 化し unanswered は対象 window
  のみ `Approve`。`Destroyed` は `on_window_closed` +
  `sweep_destroyed` で全経路到達。`Resized`/`Moved`/
  `ScaleFactorChanged`/`Focused` は差分観測で合成。
- `IntentSink`: GPUI view が Rust から `Intent` を emit する
  `Action` ingress(`WebView` adapter の `postMessage` decode に相当)。
- 最小 root view `RvfRoot`: fixture 表示面(RF388/RF389 が差替する
  拡張 seam)。

## feature

- `gpui`(非 default): `gpui` 依存と Driver 実装を有効化。
  feature なしでは `version()` のみコンパイルされ、
  `cargo check --workspace` は headless のまま維持される
  (`ruviform-ui-webview` の `webview` feature と同じ前例)。

## capability 宣言

`resize`/`focus`/`scale_factor`/`multi_window` = Supported、
`ime`/`native_surface_slot` = Unsupported(Issue #388/#389 が実装時に
宣言点を反転。silent degrade はしない — `focus(false)` は gpui
0.2.2 に OS-level deactivate が無いため `UiError::Unsupported`)。

既知の近似: `PlatformDisplay` は logical bounds のみで per-display
scale API が無いため、monitor の physical 寸法は live window の
scale factor で推定(#388 の拡張点)。

## 層・許可依存

- 層: Optional platform adapter(利用側層)。
- 許可依存: `ruviform-ui` のみ(foundations への下向き強制・
  adapter→adapter 依存は作らない)。

## RT 契約

非 RT 専用。ウィンドウ/イベントループ/UI スレッド処理であり
RT パスからは呼ばない。

## 利用・検証

```sh
# 最小 Window fixture(実 OS session でのみ実行 — manual 検証入口)
mise run example-ui-gpui-minimal
```

検証タスク(feature 有効 lane):
`mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`。
実 OS Window 表示/入力は `example-ui-gpui-minimal` による manual
検証であり、headless の green を実機受入 Pass とは区別する。

実機 lifecycle（manual・CI 対象外。Issue #431）:

2 window の生成・Dock 2-phase move（commit/abort）・detach/reattach・inspector の defer→approve・
main の approve を実行し、trace 照合で exit 0 を返す。OS close の送り方は次のどちらか。

```sh
# (a) 手動: 起動し、inspector の close ボタンを 2 回・main の close ボタンを 1 回押す
mise run example-ui-gpui-lifecycle

# (b) 自動: exe を起動して WM_CLOSE（close ボタンと同じメッセージ）を順に送る
powershell -NoProfile -ExecutionPolicy Bypass -File evidence/issue-431-real-verify/wm-close.ps1 \
  -Exe target/debug/examples/real_window_lifecycle.exe -Steps "inspector,inspector,main"
```

## 関連

- 共通契約: [`ruviform-ui`](../ruviform-ui/README.md) `driver` module
- test fake: [`ruviform-ui-test`](../ruviform-ui-test/README.md) `FakeDriver`
- 設計: docs/designs/issue-386-driver-neutral-gui-host.md
- 既存 adapter: [`ruviform-ui-webview`](../ruviform-ui-webview/README.md)
