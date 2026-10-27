---
id: "SPEC-034JZPGUI0000000000001"
type: "spec"
status: "approved"
title: "プラグイン GUI PoC（規格準拠ネイティブビューへの WebView 埋め込み検証）"
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
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/plugin-architecture-clap-vst3.md"
  derived-by:
    - "docs/designs/plugin-gui-poc.md"
    - "docs/adr/plugin-gui-embedding.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# プラグイン GUI PoC

## 背景

採否 ADR（plugin-architecture-clap-vst3・ui-architecture-webview-hybrid）確定: プラグイン GUI は双方向必須だが独自 GUI 強制なし、外側は規格準拠ネイティブウィンドウ/ビュー・内側は WebView 可。Issue #28 はスタンドアロン窓で検証済み（wxp パッチ差分の精査は本 Issue へ引き継ぎ）。残論点は「**ホスト所有の親ウィンドウ・外部イベントループへ WebView を attach**」できるか。

## 検証対象（AC）

- AC1: ホスト提供の外部親 HWND（非 tao 生成・外部メッセージループ）へ wry WebView を子として attach できる（Windows 実機）
- AC2: attach 後の表示・`set_bounds` リサイズ追従・IPC 往復が動作する
- AC3: attach/detach のライフサイクルが CLAP gui ext（create/set_parent/set_size/destroy）と VST3 IPlugView（attached/onSize/removed）に対応付け可能 — 文書評価
- AC4: baseview / wxp 経路の文書評価（メンテ状況・3 OS・ライフサイクル適合）
- AC5: リソース消費（プロセスメモリ・子プロセス数）を `poc-metric.v1` で記録
- AC6: ルートワークスペースへ変更しない（`poc/plugin-gui/` 独立ワークスペース）

## Out of Scope

- macOS/Linux 実機（文書評価のみ — #60 の対象）
- 実ホストアプリ（CLAP host）での統合検証
- `ruviform-ui`/`ruviform-plugin-*` への本実装反映（後続 Issue）

## Acceptance Criteria

- [x] AC1–AC5 の結果が `poc/plugin-gui/RESULTS.md` + `docs/impl/exp/` jsonl に記録
- [x] 採否 ADR が `docs/adr/` に作成され WebView-in-plugin 経路の採否を記録
- [x] ルートワークスペース無変更（Cargo.toml / crates 非接触）

## 関連文書

- 先行 PoC: [issue-28 UI PoC](../plans/issue-28-ui-poc.md)・[ui-tech-selection-poc ADR](../adr/ui-tech-selection-poc.md)
