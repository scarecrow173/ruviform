---
id: "DESIGN-034JZPGUI0000000000002"
type: "design"
status: "approved"
title: "プラグイン GUI PoC 設計（外部親 HWND + 外部メッセージループへの wry attach）"
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
    - "docs/adr/plugin-gui-embedding.md"
  derived-by:
    - "docs/plans/implement-issue-30-plugin-gui-poc.md"
    - "docs/test-specs/plugin-gui-poc.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-gui-poc.md"
  references: []
  defers: []
  deferred-by: []
---

# プラグイン GUI PoC 設計

## Overview

`poc/plugin-gui/` に独立ワークスペースの単一 bin `plugin-gui-poc` を置く。Windows で「ホスト提供の親 HWND（windows-rs の素 Win32 窓・素の GetMessage ループ）へ `raw-window-handle` adapter 経由 `WebViewBuilder::build_as_child` で wry を attach」する — プラグインがホストから受け取る状況を模倣。

## 実験設計

- **E1（AC1/AC2）attach 基本**: 素 Win32 親窓 + 標準メッセージループ上で wry 子 attach → `with_html` 描画 → `set_bounds` リサイズ → IPC roundtrip（evaluate_script + ipc handler）
- **E2（AC5）リソース**: attach 前後の working set・プロセス数を記録
- **判定規則**: 各計測を `poc-metric.v1` jsonl に出力。失敗は FAIL、未実施は SKIP、文書評価は `doc-eval` ラベル
- **E3（AC3/AC4）文書評価**: CLAP gui ext / VST3 IPlugView / baseview / wxp の適合表を RESULTS.md に記録

## wry 側の要件

`build_as_child` は `HasWindowHandle` を要求 → 親 HWND を `raw_window_handle::Win32WindowHandle` で包んだ adapter を渡す。wry は自前イベントループを要求しない（WebView2 はスレッドの標準メッセージポンプで動作）。

## Verification

- [x] 親窓・イベントループの所有がホスト側モデルと一致
- [x] `#28` の `build_as_child` 既知課題（リサイズ時 `set_bounds` 再適用必要）を本実験でカバー
