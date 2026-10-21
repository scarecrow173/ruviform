---
id: "ADR-034JZPGUI0000000000007"
type: "adr"
status: "accepted"
title: "プラグイン GUI 埋め込み方式（外部親ウィンドウへの wry attach）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/plugin-gui-poc.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/plugin-architecture-clap-vst3.md"
  derived-by:
    - "docs/designs/plugin-gui-poc.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# プラグイン GUI 埋め込み方式

## Context

プラグイン GUI は双方向必須（ADR `plugin-architecture-clap-vst3`）。外側を規格準拠ネイティブビュー・内側を WebView とする方針（ADR `ui-architecture-webview-hybrid`）の実現可能性が未検証だった — 特に「ホスト所有の親ウィンドウ + 外部イベントループへ WebView を attach」。

## Decision

**素 wry（0.57）の `WebViewBuilder::build_as_child` でホスト提供 HWND への子 WebView attach を採用**（Windows 実機 confirm）。`raw-window-handle` adapter で HWND を包み、外部 GetMessage ループ上で attach・描画・resize・IPC が全て動作した（[RESULTS.md](../../poc/plugin-gui/RESULTS.md)・[実験ログ](../impl/exp/plugin-gui-attach-e1-e2.jsonl)）。

## Considered Options

- **Option A — 素 wry `build_as_child`（採用）**: 外部親 HWND に直接 attach。依存最小・3 OS 対応・Issue #28 で wry は採用済み。実機で全動作確認（28ms・+15.7MB）
- **Option B — baseview 経由**: プラグインウィンドウ抽象層を噛ませる。WebView 機能を持たず wry attach が別途必要で、UI 方針と直交するため主経路には不採用（補助層として残り得る）
- **Option C — wxp**: wry + プラグインライフサイクルパッチ同梱基盤。素 wry で外部親 attach が動作したため導入必須性は低下。ホスト DAW 固有互換が必要になった場合の参照実装として保持

## Consequences

- plugin GUI 実装は「clap gui ext `set_parent` → `build_as_child`」「VST3 `attached` → `build_as_child`」の写像で構築可能（写像表は RESULTS.md AC3）
- 規格側の create/destroy/set_size/show/hide は wry/親窓操作へ対応済み
- macOS/Linux は文書適合のみ（#60 実機対象）

## Verification

- [x] E1/E2 の実機計測が RESULTS.md に記録済み（AC1/AC2/AC5 passed）
- [x] AC3/AC4 が文書評価ラベルで記録済み
- [x] ルートワークスペース無変更（AC6）
