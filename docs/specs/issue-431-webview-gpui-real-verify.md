---
id: "SPEC-Rk4Vw9TqLb2XcJ7NhPd5Ya"
type: "spec"
status: "proposed"
title: "Issue #431 WebView 実機 e2e・GPUI 実窓 close veto/Dock 検証・legacy trait Phase 2 判断"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/431"
    - "https://github.com/scarecrow173/ruviform/issues/423"
  changes:
    added:
      - "crates/ruviform-ui-webview/examples/e2e_smoke.rs"
      - "crates/ruviform-ui-webview/examples/e2e_smoke.html"
      - "crates/ruviform-ui-gpui/examples/real_window_lifecycle.rs"
      - "evidence/issue-431-real-verify/wm-close.ps1"
    modified:
      - "crates/ruviform-ui-webview/Cargo.toml"
      - "crates/ruviform-ui-gpui/Cargo.toml"
      - "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/issue-431-webview-gpui-real-verify.md"
  depends-on:
    - "../specs/issue-423-webview-dock-verify.md"
    - "../specs/issue-386-driver-neutral-gui-host.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../reports/issue-423-gpui-real-window.md"
    - "../adr/issue-431-legacy-host-trait-phase2-retain.md"
  refines:
    - "../specs/issue-423-webview-dock-verify.md"
  refined-by: []
  derives-from:
    - "../specs/issue-423-webview-dock-verify.md"
  derived-by:
    - "../designs/issue-431-webview-gpui-real-verify.md"
  verifies: []
  verified-by:
    - "../reports/issue-431-real-verify.md"
  references:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  defers: []
---

# Issue #431 WebView 実機 e2e・GPUI 実窓 close veto/Dock 検証・legacy trait Phase 2 判断

## 背景

#423（PR #430）で WebView 写像・GPUI Dock の headless 検証・focus capability を main に統合した。
残 DoD は #431 に移管されており、次の 3 点が実機での確認または判断として未達だった。

- WebView の page load → IPC → event 往復、`Created`/`Ready` の実発行（headless では再現不可）
- GPUI 実 window の close veto・Dock 2-phase move・detach/reattach（報告書
  `docs/reports/issue-423-gpui-real-window.md` §2–3 の手動手順のまま未実施）
- `ruviform-ui` の legacy trait（`HostHandle`/`WindowHandle`/`HostEvent`/`WindowConfig`）の
  Phase 2 削除判断

## 目的

- WebView2 上で、page load → IPC → event 往復を自動で確認できる入口を持つ。
- GPUI の実 OS window で close の veto/defer/approve と Dock 2-phase move・detach/reattach を
  確認し、結果を記録する。
- legacy trait の削除/保持を、影響範囲（adapter・consumer・docs）から判断して記録する。

## 受入条件

| # | 条件 | 結果の記録先 |
| --- | --- | --- |
| A1 | WebView 実機で page load・IPC・event 往復・`Created`/`Ready` の実発行を確認する。自動化できない部分は手順を記録し NotRun と明記する | `docs/reports/issue-431-real-verify.md` §1 |
| A2 | GPUI 実 window で close の defer・approve、Dock 2-phase move（commit/abort）・detach/reattach を確認する。OS 経路の Veto は対象外とし NotRun と明記する。手順または結果を記録する | 同 §2 |
| A3 | legacy trait の削除/保持を判断し、根拠（使用箇所・実装者・Phase 2 gate）を記録する | `docs/adr/issue-431-legacy-host-trait-phase2-retain.md` |
| A4 | `mise run check-ui-driver-boundary` が pass する | PR 検証欄 |
| A5 | 追加した e2e・lifecycle の入口は `mise.toml` の manual タスクとして登録し、CI lane には入れない | `mise.toml` |

## 非目標

- legacy trait の実削除（本 PR では判断記録のみ。削除は別 PR）
- 実 OS lane のうち DPI 遷移・IME・OS 起因 focus・GPU device lost（#262 / #243 / #390 系の consumer 受入で扱う）
- Dock の UI（ドラッグ操作・描画）。Dock 2-phase は `DockWorkspace` のモデル操作を実 window と同一プロセスで実行して検証する
- `NSWindow`/macOS・Linux での実機検証（本環境は Windows 11 x64 のみ）

## 制約

- 検証入口は `mise run <task>` とし、実機専用の入口は CI に入れない（`*-manual` 相当の `example-*` タスク）
- 公開 API の変更は行わない（examples と manual 手順の追加のみ）
- 新規 dependency は追加しない
