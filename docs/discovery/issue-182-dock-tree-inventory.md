---
id: "DISC-GLEiDNdU7qC07pVNVGO6O1"
type: "discovery"
status: "resolved"
title: "Issue #182 Dock Tree / View Move / Layout Restore — existing API inventory and gap analysis"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/182"
    - "https://github.com/scarecrow173/synvelis/issues/2"
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
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-181-multi-window.md"
    - "docs/adr/issue-181-multi-window-contract-placement-close-decision-ipc-routing.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/181"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 182
---

# Issue #182 Dock Tree / View Move / Layout Restore — existing API inventory and gap analysis

## Summary

Issue #182 は「任意の制作 View をタブ化・分割・切り離し・再ドッキングし、複数 Window/モニターで再配置する」ための再利用可能基盤を要求する。前提となる Issue #181（複数 Window ホスト）は merged 済みで、`WindowId`・`WindowHandle`・`run_multi`・`MonitorInfo`・`CloseDecision` が揃っている。現行コードには Dock/View/レイアウト永続化の概念が一切存在しないため、本 Issue はほぼ全量が新規契約となる。

## Current State (実コード棚卸し)

### `ruviform-ui`（WebView 非依存コア、`crates/ruviform-ui/src/`)

- `window.rs`（Issue #181 で追加）: `WindowId`（NonZeroU64・session 内一意・単調・破棄後再利用なし）、`WindowEvent{Windowed{window,event},Tick}`、`RoutedEvent{window,event}`、`CloseDecision{Approve,Veto,Defer}`、`WindowConfig{title,width,height,url,devtools}`、`MonitorInfo{name,width,height,x,y,scale_factor}`、`WindowHandle` trait（eval/resize/focus/create_window/request_close/respond_close/destroy_window/windows/monitors/exit）。
- `event.rs`: `HostEvent` は `Moved{x,y}`・`Destroyed` を含む（#181 で additive 追加済み）。座標は物理ピクセル・仮想デスクトップ座標系。
- `runtime.rs`: `UiRuntime::handle_window`/`drain_jobs_routed`/`cancel_window_jobs`、Job の `owner: Option<WindowId>`（routing 専用・寿命非連動）。
- `service.rs`: `ServiceContext.origin: Option<WindowId>`。
- その他: `ipc`（rvf.ipc.v1）、`data`/`assets`/`bridge`/`render`/`detect`/`demo`/`error`。`serde`/`serde_json` は既存依存（公開 API 面に露出済み）。

### `ruviform-ui-webview`（wry/tao adapter、`webview` feature）

- `session.rs`: `SessionConfig{data_dir,tick,windows}` + `run_multi`。`Session` が `WindowId ⇄ tao window id ⇄ Entry{window,webview,ready,backlog,close_pending,awaiting_response}` のレジストリを管理。`create_entry` は `WindowBuilder`（`with_title`/`with_inner_size(LogicalSize)` のみ — **position 指定なし**）。
- `host.rs`: 単一 Window 版 `run`/`HostConfig`/`build_webview`（`pub(crate)` 共有）。
- `scheme.rs`: `rvf://app`/`rvf://data` ハンドラ。

### テスト/デモ資産

- `crates/ruviform-ui/tests/multi_window.rs`: `FakeHost`（`WindowHandle` 実装・eval 配送記録・ready/backlog・close シミュレート）— headless で multi-window 契約を駆動済み。#182 の move/close 検証にそのまま再利用可能なパターン。
- `examples/demo_cli.rs`（ruviform-ui・headless）、`examples/demo_multi.rs`（webview・実機）。

## Gap Analysis（Issue #182 要求 vs 現状）

| # | 要求 | 現状 | ギャップ |
| --- | --- | --- | --- |
| G1 | 論理 View Instance ID・所有先高々 1 箇所 | View の概念が存在しない | 新規: `ViewInstanceId` + 全 Window 横断の所有不変条件 |
| G2 | `Split(axis,ratio,children)`+`Tabs(views,active)` の Dock Tree（≤1 tree/Window） | 存在しない | 新規: Dock モデル + 正規化/拒否規約 |
| G3 | Split/Tabs/並べ替え/移動/切り離し/再結合の操作層（アプリ固有 UI と分離） | 存在しない | 新規: workspace 上の純粋操作 API + drop 語彙 |
| G4 | 移動先 Window/WebView 準備後に所有権切替・失敗時ロールバック・移行中の二重編集禁止 | `WindowHandle::create_window`/`Ready` イベントはあるが移動契約なし | 新規: two-phase move（begin/commit/abort + busy lock）|
| G5 | Window 幾何（位置・大きさ・モニター・DPI）の保存/復元 | `MonitorInfo`・`Moved{x,y}`・`Resized` はあるが、位置の書き込み経路がない（`WindowConfig` に position なし・`WindowHandle` に set_position なし） | 新規: snapshot 契約 + `WindowConfig.position`（tao `with_position` に通す小規模 adapter 拡張） |
| G6 | レイアウト保存 schema（version・検証・復元失敗時の安全デフォルト） | 存在しない（`serde` 依存はあり） | 新規: `LayoutSnapshot` 版付き schema + validate + default fallback |
| G7 | モニター消失・画面外座標の補正、ディスプレイ ID 非永続 | `MonitorInfo` 取得のみ | 新規: signature ベースの monitor 照合 + 可視領域クランプ |
| G8 | Window Close 時の docked View 扱い = consumer ポリシー | `CloseDecision` は Window 単位のみ・View 扱いの語彙なし | 新規: `WindowCloseDisposition`（Detach/Relocate/Release）を workspace の明示引数に |
| G9 | drag/drop プラットフォーム制約の検証 + メニュー移動フォールバック | 検証・語彙ともになし | 新規: 制約の設計文書化 + drag 非依存の move API（menu-move = 同一経路） |
| G10 | DAW 固有 View 型/Project model を Ruviform に入れない | — | 境界維持（view state は consumer 管理、Ruviform は不透明に扱う） |

## 前提コードの不変条件（#181 から継承）

- `WindowId` は session 内の ephemeral routing token であり、**再起動を跨いで永続化しない**。したがって snapshot は WindowId ではなく window 順序で復元対応付けする。
- `run_multi` の `CloseRequested` 既定は window 単位の `Approve` — docked views の扱いは dock 層が別途決める（host は View を知らない）。
- `eval` は fire-and-forget、`Destroyed` が terminal event。View 移動中の Window 消失は `Destroyed` で観測可能。
- `WebView2` UDF は session 共有 `WebContext` — 新規 Window 生成は `create_window` 経由で同一 context に載る。

## Platform Constraint Research（drag/drop）

- `wry` は cross-webview ネイティブドラッグの API を持たない。HTML5 drag/drop は同一 WebView 内では動作するが、**WebView 境界を跨ぐドラッグは WebView2/WebKitGTK/WKWebView いずれも保証しない**。
- よって Window 間の View 移動は (a) drag メタファーを各 WebView 内の視覚表現に留めて commit を host 側 intent で行う、(b) drag 機構に一切依存しない明示的メニュー移動、の 2 経路を同一 move API に集約する設計が安全。

## Conclusion

新規契約はすべて `ruviform-ui` 側に収まる（backend 型を持たない UI 基盤基盤層の責務: 「Generic Dock/Window/Reusable View State を UI 層へ」— target architecture 合意）。adapter 側の必須変更は `WindowConfig.position` のみ（tao `with_position` への直通）。WindowHandle への新 trait メソッド追加は不要（位置は生成時パラメータとして運ぶ）。
