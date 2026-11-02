---
id: "DISC-hUXakPlCDliaq86pqk2UE3"
type: "discovery"
status: "resolved"
title: "Issue #181 Multi-Window Host — existing API inventory and gap analysis"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/181"
    - "https://github.com/scarecrow173/synvelis/issues/2"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
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
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/ui-tech-selection-poc.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/plugin-gui-embedding.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/182"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 181
---

# Issue #181 Multi-Window Host — existing API inventory and gap analysis

## Summary

Issue #181 が要求する「単一ネイティブイベントループ上の複数 Window/WebView 管理・Window 単位のイベント発信元・CloseRequested/Destroy 分離・IPC 応答の宛先 Window 識別」を既存コードにマッピングした。結論: **ホスト・イベント・ランタイムの 3 層すべてが単一 Window 前提**であり、(a) Window 識別子 `WindowId` が全層に存在しない、(b) `HostEvent` が発信元 Window を持たない、(c) `UiRuntime` は `Event` を宛先情報なしで返し Job→Window の紐付けを持たない、(d) `CloseRequested` でホストループ即終了 + Job 一括取消という単一 Window セマンティクスがハードコードされている、(e) `rvf.ipc.v1` の呼出 `id` は WebView ごとのローカル採番で複数 WebView 間で衝突しうる — の 5 点が実ギャップ。一方で tao/wry は複数 Window をネイティブにサポートし、`EventLoopWindowTarget` 経由の動的 Window 生成・`window_id` 付きイベント・`WebContext` 共有が可能であり、**adapter 側の新規機構は少なく、主な設計仕事は `ruviform-ui` の契約拡張**。信頼度: 高（対象コード全件実読済み、本 worktree HEAD）。

## Exploration Goal

- Issue #181 の完了条件（汎用 API・Window ID・イベント発信元・close 承認/中止/延期・IPC 応答配送・fake-host テスト）を既存 API にマッピングし、実現可能範囲・不足・互換性制約を確定する。
- Synvelis 固有の Dock/View/Project 意味論と汎用 Window 基盤を分離する。
- 単一 Window 経路（AudioLens baseline）を壊さない互換戦略を確定する。

## Inventory (現行 API の実コード根拠)

### `ruviform-ui`（抽象契約クレート、Issue #156）

- `event.rs`: `HostEvent` は `Ready{url}`/`Message(String)`/`Resized{w,h}`/`Focused(bool)`/`ScaleFactorChanged{factor}`/`CloseRequested`/`Tick` の 7 variant（`#[non_exhaustive]`）。**Window 発信元を持たない**。`HostHandle` trait は `eval`/`resize`/`focus`/`request_close` の 4 メソッドで単一 Window に暗黙バインドされる。
- `ipc.rs`: `rvf.ipc.v1` — `Intent{v,kind,id,op,params}`/`Event{Result,Job,Notify,Lifecycle,Error}`/`ErrorBody`。`id` は `BRIDGE_JS` が WebView 内で `i-<seq>` 採番するローカル相関キー。**複数 WebView は同じ `i-1` を発行しうる**（issue 指摘どおり）。
- `runtime.rs`: `UiRuntime::handle(HostEvent) -> Vec<Event>`。`Message`→`dispatch`、`_`/`Tick`→mailbox drain、`CloseRequested`→**全 Job cancel + `lifecycle closing` event 発行**。Job は `JobRecord{cancel}` のみ — owner Window 概念なし。`dispatch_op`（headless 入口）も Window 概念なし。
- `service.rs`: `ServiceContext{data}` のみ。`Service::call` は発信 Window を知りえない。
- `bridge.rs`: `window.__rvf` — `call`/`onEvent`/`dataUrl`/`dispatch`。Window 識別機能なし。`dispatch` は `result.id` で pending Promise を解決する — **同一 WebView 内で id は一意**であれば正しく動作する。

### `ruviform-ui-webview`（wry/tao adapter、`webview` feature）

- `host.rs`: `run(config, assets, data, on_event)` — `EventLoopBuilder::<()>::with_user_event()` 1 基 + `WindowBuilder` 1 Window + wry `WebView` 1 基。`Event::WindowEvent{window_id, event}` の `window_id` は**未使用**（単一 Window 前提で無視）。`CloseRequested` → `emit(CloseRequested)` + 即 `ControlFlow::Exit`（veto 不可）。`WebHost` は `&Window`/`&WebView` を borrow した `HostHandle`。
- `WebContext` は `run` 内で 1 基生成（`data_dir` 既定 = プロセス固有 temp dir）。WebView2 UDF のシングルトンロック対策として計測済み（poc/ui）。
- `scheme.rs`: `rvf://` ハンドラは `Arc<dyn AssetSource>` + `DataRegistry` を共有参照 — 複数 WebView での共有は追加機構不要。
- `detect.rs`: `wry::webview_version()` プローブのみ。OS/モニター情報は扱わない。

### tao/wry の複数 Window 能力（backend 事実）

- tao `EventLoop` は複数 `Window` をネイティブサポート。`Event::WindowEvent{window_id,..}` に `tao::window::WindowId` が付き、`EventLoopWindowTarget`（run_return クロージャの第 2 引数）経由でイベントループ実行中に `WindowBuilder::build(&elwt)` による動的 Window 生成が可能。`available_monitors()` で `MonitorHandle`（name/size/position/scale_factor）を取得可能。
- wry `WebViewBuilder::new_with_web_context(&mut WebContext)` で共有 `WebContext` 配下の複数 WebView を生成可能。WebView2 は UDF がプロセス/環境単位のシングルトンロック — 同一 `WebContext` 共有が正規構成。WebKitGTK の `WebContext` もセッション/ネットワーク共有単位。`WebView` は `Window` drop 時に backend が子コンテンツを破棄（wry の RAII 契約に依存）。
- `WindowEvent::CloseRequested` は「要求」であって強制破棄ではない — tao/winit モデルでは Window を drop しない限り生存し続けるため、**承認/拒否/延期の分離は backend 能力として成立する**。

## Gap Analysis（issue 対象項目 → 実ギャップ）

| Issue 要求 | 現状 | ギャップ |
| --- | --- | --- |
| 単一イベントループで複数 Window 管理 | `run` は 1 loop 1 window | `host.rs` に Window レジストリ（`HashMap<WindowId, {Window, WebView}>`）と `window_id` → `WindowId` 写像が必要。新規コード、既存機構の流用 |
| 安定 Window ID + イベント発信元 | `HostEvent` に ID なし | `ruviform-ui` に `WindowId` 型とイベント envelope（`WindowEvent{window,event}` 相当）が必要。`HostEvent::Tick` はループ全体共有のため envelope 外で扱う規約が要る |
| IPC 応答の宛先 Window 識別 | `handle() -> Vec<Event>` は宛先なし | Job→origin Window の紐付けと `RoutedEvent{window,event}` 相当の返却型が必要。wire 形式（`rvf.ipc.v1`）は変更不要 — `id` は WebView ローカル一意で、応答は発信元 WebView へ返すため。wire 変更なしを明示する結論 |
| create/focus/move/resize/DPI/monitor/CloseRequested/destroy 抽象化 | `HostEvent` に Moved/Destroyed/MonitorInfo なし、`HostHandle` に create/destroy/monitors なし | `HostEvent` へ `Moved`/`Destroyed` 追加、`WindowHandle` port（eval/resize/focus/create_window/request_close/respond_close/destroy_window/monitors/exit）新設 |
| CloseRequested と Destroy の分離 | `run` は Close→即 Exit、`UiRuntime` は Job 一括 cancel | `CloseDecision{Approve,Veto,Defer}` 契約、`Destroyed` イベント、未回答時の既定方針が必要。**単一 Window 経路（`handle`/`run`）の既存セマンティクスは維持**（AudioLens baseline、追加 gate） |
| サブ Window Close ≠ アプリ終了/Job 取消 | `handle(CloseRequested)` が全 Job cancel | multi-window 経路の `handle_window` では Job 自動取消を行わない。Job 寿命は Window と分離（routing のみ owner 参照）。`cancel_window_jobs` の opt-in API で「Window-owned Job」語彙を提供 |
| 失敗時の安全なフォールバック | create 失敗は `run` 未起動エラーのみ | `create_window -> Result<WindowId,UiError>`、eval の dead-destination drop、WebView not-ready 時の bounded 保留 push、window loss → `Destroyed` 終端化の規約が必要 |
| RT 契約維持 | 両クレート非 RT 専用明記済み | 変更不要（契約文書の再確認のみ） |
| backend 型の非露出 | 適合済み | `WindowId`/`WindowConfig`/`MonitorInfo`/`CloseDecision` 等に wry/tao 型を含めない設計審査 |

## Design Forces

- **互換性**: 公開 API の追加は additive が原則。`HostEvent` は `#[non_exhaustive]` のため variant 追加は合法。`UiRuntime::handle`/`dispatch_op`/`drain_jobs` と `host::run` の既存シグネチャ・既存セマンティクス（CloseRequested→Job cancel→exit）は AudioLens baseline として**不変**。
- **Routing authority**: `WindowId` はホスト（adapter）が採番するランタイム識別子。`rvf.ipc.v1` のペイロードには載せない — wire 形式は v1 不変を明示的結論とする。WebView 内の `id` 相関は既存のまま。
- **Close 状態機械**: `CloseRequested` → consumer の `CloseDecision` → `Approve` は即破棄 + `Destroyed` 発行、`Veto` は取消、`Defer` は close-pending 継続（後で `destroy_window` か次回要求の承認）。未回答の既定 = `Approve`（その Window のみ破棄 — ループ・アプリ終了はしない）。最終 Window/アプリ終了ポリシーは consumer 所有（`exit()` port で駆動）。
- **Job 寿命の分離**: `UiRuntime` の Job は「発信元 Window への配送先」としてのみ Window を参照し、寿命制御には使わない。Window 破棄後の Job event は宛先不在で配送層が drop（プロセスを壊さない）。長寿命 Job（Application/Project 所有）は常時このセマンティクス。「Window-owned Job」は consumer が `cancel_window_jobs` で opt-in 実装する語彙。
- **WebView2 UDF / WebContext 寿命**: 1 セッション 1 `WebContext`（単一 UDF）を全 Window が共有。`data_dir` は per-Window ではなくセッション設定。`WebContext` は全 WebView より長く生きる必要（adapter が所有）。
- **順序保証**: adapter はイベントを発生順にキューイングして UI スレッドへ配送（既存 `VecDeque`+`EventLoopProxy` 機構の拡張）。`Destroyed` 後の同 Window 宛イベントは発生しない（entry 削除で送信元も消える）。create→`Ready` 間の eval は bounded 保留（not-ready フォールバック）。

## Key Issues and Assumptions

- 複数 Window の汎用契約は `ruviform-ui` の責務。`WindowId`/`WindowEvent`/`RoutedEvent`/`CloseDecision`/`WindowConfig`/`MonitorInfo`/`WindowHandle` trait 新設、`HostEvent` への `Moved`/`Destroyed` variant 追加、`UiRuntime` への `handle_window`/`drain_jobs_routed`/`cancel_window_jobs` 追加が最小構成。
- `ServiceContext` に `origin: Option<WindowId>` を additive 追加し、Service が発信 Window を識別できるようにする（複数 Window op の実装前提 — 例「この Window を閉じる」系 op の発信元判定）。
- `host::run`（単一 Window）と新 `run_multi`/`SessionConfig`（複数 Window）は別入口として共存 — 単一 Window 経路の挙動不変を構造的に保証し、AudioLens 回帰を防ぐ。
- fake-host テストは `ruviform-ui` 側の `WindowHandle` 実装 + `WindowEvent` 駆動で headless に検証可能（adapter 実機依存を切り離す）。
- 実機検証（複数 Window 生成・独立フォーカス・サブ Window 閉鎖・別モニター移動・異 DPI）は本 PoC の CI 対象外 — `demo_multi` example を用意し、実行結果は実施した範囲のみ `docs/impl` に記録、未検証は明示。

## Out of Scope（issue と整合）

- Synvelis 固有: Arrangement/Mixer/Inspector 配置、Dock Tree、Window 間 View 移動（#182）、Project/Command Router 意味論。
- プラグイン規格のネイティブ GUI 親子関係再設計（#30/#115 と整合する範囲に留める）。
- 実機未検証の OS 別保証。
- WebView レンダラクラッシュの自動復帰（`Destroyed` で終端化する規約のみ定義）。
- メニュー/トレイ等のシェル機能。

## Open Questions → 設計で決定

- `WindowEvent`/`RoutedEvent`/`WindowHandle`/`UiRuntime` 経路の具体的な API 形（ADR + design で確定）。
- 未回答 `CloseRequested` の既定（`Approve` を推奨 — 破棄しない既定は「閉じられない Window」を生む）。
- Job event の宛先: owner Window 単一宛先（broadcast しない — `None` owner は unrouted で consumer が決める）。
- pre-`Ready` eval の保留上限（bounded、drop-oldest）。
- `WindowId` の世代/再利用ポリシー（同一セッション内で再利用しない、単調採番）。
