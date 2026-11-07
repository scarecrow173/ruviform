---
id: "TASK-Ud6Jq1Mk8Pz3Rt5Wy2Ne4Gb"
type: "task"
status: "in-progress"
title: "Issue #431 実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/431"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-431-webview-gpui-real-verify.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-431-webview-gpui-real-verify.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../reports/issue-431-real-verify.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-431-webview-gpui-real-verify.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../reports/issue-431-real-verify.md"
  references: []
  defers: []
---

# Issue #431 実装タスク

| # | タスク | 状態 |
| --- | --- | --- |
| T1 | `e2e_smoke`（WebView2 往復）を実装し、実機で PASS を取る | done |
| T2 | `demo_app`/`demo_multi` の OS close 経路を `wm-close.ps1` で確認する | done |
| T3 | `real_window_lifecycle`（Dock 2-phase・defer/approve の OS close）を実装し、実機で PASS を取る | done |
| T4 | 陰性対照で trace 照合が失敗することを確認する | done |
| T5 | `minimal_window` の OS close（報告書 M1）を確認する | done |
| T6 | legacy trait Phase 2 の判断 ADR を書く | done（status: proposed） |
| T7 | `mise.toml` に manual タスク 2 件を追加する | done |
| T8 | 両 crate の README に example の入口を追記する | done |
| T9 | fmt / lint / test / 境界 guard を実行して結果を記録する | done |
| T10 | 報告書（検証結果・NotRun・手順）を書く | done |
| T11 | review-protocol（独立レビュー）を実施し、blocker を解消する | done（blocker なし） |
| T12 | commit・push・PR 作成（日本語 PR）、head SHA 報告、worker_done | pending |
