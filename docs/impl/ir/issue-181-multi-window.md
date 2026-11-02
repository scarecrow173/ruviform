---
id: "IMPL-issue181-multi-window"
type: "impl"
status: "completed"
title: "Issue #181 multi-Window/WebView host — per-window event & lifecycle contracts"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/181"
  implements:
    - "docs/tasks/issue-181-01-window-contract-types.md"
    - "docs/tasks/issue-181-02-runtime-routing.md"
    - "docs/tasks/issue-181-03-fake-host-contract-tests.md"
    - "docs/tasks/issue-181-04-webview-run-multi.md"
    - "docs/tasks/issue-181-05-demo-multi-and-verification.md"
  satisfies:
    - "docs/specs/issue-181-multi-window.md"
    - "docs/designs/issue-181-multi-window.md"
    - "docs/test-specs/issue-181-multi-window.md"
---

# Issue #181 — multi-window 実装記録

## 変更内容

### ruviform-ui（コントラクト・crate 内に backend 型なし）

- `src/window.rs`（新規）: `WindowId`（NonZeroU64 ・ session 内一意・
  再利用なし・wire には出ない）、`WindowEvent`（`Windowed{window,event}`
  + loop 全体の `Tick`）、`RoutedEvent`（`window: Option<WindowId>` +
  `Event` — `None` は非 routed、sink 選択は consumer）、`CloseDecision`
  （`Approve`/`Veto`/`Defer`）、`WindowConfig`、`MonitorInfo`、
  `WindowHandle` port（`eval`/`resize`/`focus`/`create_window`/
  `request_close`/`respond_close`/`destroy_window`/`windows`/`monitors`/
  `exit`）。
- `src/event.rs`: `HostEvent` に `Moved{x,y}` と `Destroyed` を追加
  （`#[non_exhaustive]` なので additive）。`CloseRequested` の doc に
  single-window（従来: no-veto で loop 終了）と multi-window
  （veto-able・job cancel しない・loop 終了しない）両経路の意味を明記。
- `src/runtime.rs`: `JobRecord.owner: Option<WindowId>` 追加（routing
  用のみ・job lifetime は window に従わない）。新規 API:
  `handle_window(window, HostEvent) -> Vec<RoutedEvent>`、
  `drain_jobs_routed() -> Vec<RoutedEvent>`、`cancel_window_jobs(w) ->
  usize`（opt-in の window 単位 job cleanup・close では自動呼出し
  しない・既 cancel 済みは数え直さない）。既存 `handle`/`drain_jobs`/
  `dispatch_op`/`poll` は owner=None の legacy 経路として挙動不変
  （`CloseRequested` で全 job cancel する v1 互換を保持）。
- `src/service.rs`: `ServiceContext.origin: Option<WindowId>` 追加
  （service が呼出し元 window を backend 型なしで観測できる）。
- `src/lib.rs`: `pub mod window` + 再 export。

### ruviform-ui-webview（wry/tao adapter）

- `src/session.rs`（新規）: `SessionConfig{data_dir, tick, windows}` +
  `run_multi`。1 本の tao event loop が `Session` レジストリ
  （`WindowId` ⇄ tao window id ⇄ `Entry{window, webview, ready,
  backlog, close_pending, awaiting_response}`）を管理。全 window で
  1 つの `WebContext`（WebView2 UDF シングルトンロック対策・session
  一生分保持）。`CloseRequested` は emit のみ（破棄しない・loop を
  終了しない）— `respond_close` で Approve/Veto/Defer、未回答なら
  emit 復帰後に対象 window のみ既定 `Approve` → `Destroyed` を queue
  経由で emit。未 `Ready` への `eval` は有界 backlog（16・古い順
  drop）に積み `Ready` で flush、既破棄・不明宛は silent drop。
  `windows()` で生存 id 一覧、`monitors()` で `MonitorInfo` 一覧、
  `exit()` でのみ loop 終了。`SessionConfig::single(&HostConfig)` で
  既存 config との互換変換を提供。
- `src/host.rs`: `build_webview`/`bounds_for`/`full_bounds` を
  `pub(crate)` に変更し `url`/`devtools` を引数化して session 側と
  共有（単一窓 `run` の動作は不変）。
- `examples/demo_multi.rs`（新規）: 2 window の `run_multi` デモ —
  routed delivery、close approve + last-window exit の consumer
  policy を実装例として提示（実 backend 必須・CI 対象外の手動
  検証面として明記）。`Cargo.toml` に `[[example]]` 登録。
- `README.md`: `session` モジュールの責務と `demo_multi` 実行方法を追記。

### IPC wire 互換性

`rvf.ipc.v1` の wire 形式は**不変**（`SCHEMA_VERSION = 1` のまま）。
routing は host 側 `WindowId` で行い、WebView-local correlation id
（衝突し得る）は wire 上そのまま — 応答は発行元 webview にのみ
返るため衝突しない。wire/schema 変更なしを docs で明示。

## 検証結果（mise タスクのみ・全て実実行）

| タスク | 結果 |
| --- | --- |
| `mise run fmt-check` | PASS（fmt 適用後） |
| `mise run check` | PASS（workspace・headless） |
| `mise run lint` | PASS（workspace・`-D warnings`） |
| `mise run test` | PASS（workspace 全テスト。ruviform-ui は既存 16 + 新規 multi_window 13） |
| `mise run check-ui-webview` | PASS（`webview` feature・全 target） |
| `mise run lint-ui-webview` | PASS（`-D warnings`） |
| `mise run test-ui-webview` | PASS（scheme 7 tests） |

新規 `crates/ruviform-ui/tests/multi_window.rs`（13 tests）は
fake-host（`WindowHandle` 実装）で以下を検証:

- 2 window の event が `WindowId` で分離される
- WebView-local correlation id（両方 `i-1`）が window 経由で独立
  routing される
- `CloseRequested` は veto 可能・`Destroyed` と別物・未回答なら既定
  Approve
- 既破棄 window 宛 delivery は panic なし（silent drop / `UiError`）
- 未 `Ready` への eval は backlog → `Ready` で flush
- sub-window の close は他 window の job を cancel せず loop も
  終了しない（`CloseRequested` は job を一切 cancel しない）
- owner 付き job の terminal event が `drain_jobs_routed` で owner
  window へ route される・headless dispatch は `window: None`
- `ServiceContext::origin` が呼出し window を観測する
- malformed body の error は起点 window 宛のみ

## 未検証（正直な能力旗）

- **実 OS/backend 検証は未実施**: `run_multi`/`demo_multi` の実 window
  動作（WebView2・WKWebView・WebKitGTK）は headless 契約と
  コンパイル/lint まで。fake-host 契約テストは `ruviform-ui` 内の
  port 実装として実証済みだが、wry/tao の実挙動（複数 window 同時
  webview・`WebContext` 共有・Moved/Destroyed 実イベント）は未実機。
- `monitor info` の backend 精度（name が取れない platform 等）は
  backend 依存として未保証。
- `verify-ci`（CI プロファイル lane）は時間の関係で未実行 —
  dev プロファイルの check/lint/test は全 PASS。

## consumer 影響 / 移行ノート

- 既存 API は additive のみ（`HostEvent` non_exhaustive variant 追加
  + `UiRuntime`/`ServiceContext` の新規 public 要素）。既存の
  `host::run` + `UiRuntime::handle` 利用は挙動不変（close で job
  cancel する v1 挙動を保持）。
- multi-window 利用側は `run_multi` + `handle_window` +
  `drain_jobs_routed` + `WindowHandle` の routed path を使う。
  `handle` と `handle_window` の混用は owner tracking の意味を
  失わせるため非推奨（doc に明記）。
- last-window/app-exit・close-veto・window 単位 job cleanup
  （`cancel_window_jobs`）は全て consumer 側の方針 — host/runtime は
  強制しない。

## 残課題

- 実 OS multi-window PoC（Issue #182 以降の docking/移動文脈で実機
  検証を想定）。
- `demo_multi` の UI 側「新規 window 作成」op は未搭載（現状は
  session 初期 window 2 つ + consumer close policy のみ — 動的
  `create_window` は `WindowHandle` API で実装可能）。
