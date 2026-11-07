---
id: "PLAN-Hn3Lx7Qv2Wc9Ab5Ej1Rm8Tp"
type: "plan"
status: "proposed"
title: "Issue #431 実機 e2e・実窓 lifecycle 検証・legacy 判断の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/431"
  changes:
    added:
      - "crates/ruviform-ui-webview/examples/e2e_smoke.rs"
      - "crates/ruviform-ui-webview/examples/e2e_smoke.html"
      - "crates/ruviform-ui-gpui/examples/real_window_lifecycle.rs"
      - "evidence/issue-431-real-verify/wm-close.ps1"
    modified:
      - "crates/ruviform-ui-webview/Cargo.toml"
      - "crates/ruviform-ui-gpui/Cargo.toml"
      - "crates/ruviform-ui-webview/README.md"
      - "crates/ruviform-ui-gpui/README.md"
      - "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/issue-431-webview-gpui-real-verify.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-431-webview-gpui-real-verify.md"
    - "../adr/issue-431-legacy-host-trait-phase2-retain.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-431-webview-gpui-real-verify.md"
  derived-by:
    - "../tasks/implement-issue-431-webview-gpui-real-verify.md"
  verifies: []
  verified-by: []
  references:
    - "../reports/issue-423-gpui-real-window.md"
  defers: []
---

# Issue #431 実装計画

## 1. 範囲と分割

単一 PR で扱う。変更は次の 3 つに分かれ、互いに依存しない。

| 単位 | 内容 | 変更ファイル |
| --- | --- | --- |
| P1 | WebView 実機 e2e | `ruviform-ui-webview/examples/e2e_smoke.{rs,html}`、`Cargo.toml` |
| P2 | GPUI 実窓 lifecycle と OS close 駆動 | `ruviform-ui-gpui/examples/real_window_lifecycle.rs`、`Cargo.toml`、`evidence/issue-431-real-verify/wm-close.ps1` |
| P3 | legacy trait Phase 2 判断 | `docs/adr/issue-431-legacy-host-trait-phase2-retain.md`（コードは変更しない） |

共通: `mise.toml` に manual タスク 2 件（`example-ui-webview-e2e`、`example-ui-gpui-lifecycle`）を追加する。
文書: spec・design・plan・task・report・ADR。両 crate の README に example の入口を追記する。

## 2. 順序

1. P1: `e2e_smoke` を実装し、`cargo run` で実 WebView2 上の往復を確認する
2. P1: `demo_app`/`demo_multi` を起動し、`wm-close.ps1` で OS close の経路を確認する
3. P2: `real_window_lifecycle` を実装し、`wm-close.ps1` で defer/approve の OS close と Dock 遷移を確認する
4. P2: 陰性対照（`inspector,main`）で trace 照合が失敗することを確認する
5. P3: 使用箇所の棚卸しから ADR を書く
6. 文書と README を更新し、`mise run` の検証入口を通す
7. review-protocol（独立レビュー）を実施し、blocker を解消する

## 3. リスクと対処

| リスク | 対処 |
| --- | --- |
| WebView2 の子プロセスが stdio pipe を継承し、読み取りが終わらない | `wm-close.ps1` は pipe を使わず `cmd /c` のファイルリダイレクトにする |
| `Start-Process` の `ExitCode` が取得できない | `System.Diagnostics.Process` で `cmd` を起動し、子 exe の exit code を `cmd` 経由で得る |
| PowerShell 5.1 が UTF-8 の BOM なしスクリプトを誤読する | `wm-close.ps1` は ASCII のみ。タイトルの em dash は照合せず部分一致の ASCII 語を使う |
| Defer 後の OS close が二重に発火し、window が閉じない | GPUI の `on_should_close` の経路を読み、2 回目の close が `CloseRequested` を再 emit することを確認する（実機で確認済み） |

## 4. ロールバック

追加はすべて新規 example・新規 evidence・manual タスクであり、既存 API・CI lane を変えない。
問題があれば該当 example と `mise.toml` のタスクを削除すれば元に戻る。
