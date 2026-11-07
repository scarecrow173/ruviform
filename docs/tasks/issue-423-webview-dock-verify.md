---
id: "TASK-Ka4Yp9dWmR7sXq2LnV5TcE"
type: "task"
status: "in-progress"
title: "Issue #423 WebView 写像・focus capability・GPUI 実窓検証（実装スライス）"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/423"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-423-webview-dock-verify.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-423-webview-dock-verify.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
---

# Issue #423 実装スライス

計画: [plans/implement-issue-423-webview-dock-verify.md](../plans/implement-issue-423-webview-dock-verify.md)

| # | スライス | 状態 | 受入 |
| --- | --- | --- | --- |
| T1 | focus 契約文言の精緻化と GPUI 対テスト | completed | A2 |
| T2 | WebView adapter を neutral `WindowDriver` へ移行（H1–H5、legacy `run` 削除） | completed | A1, A5 |
| T3 | Late receipt の driver 経由描画テスト | completed | A4 |
| T4 | GPUI 実 window の minimal fixture 起動確認・Dock 手順の文書化 | completed（手動 Dock 手順は NotRun） | A3 |
| T5 | impl record・報告書・PR 本文の受入対応表 | in-progress | A1–A6 |
| T6 | merge 後 HEAD での全体検証と review-protocol | pending | A5, A6 |

未達（NotRun）は設計 §7 のとおり。
