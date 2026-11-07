---
id: "PLAN-Hv8LcR3wQ5nTk2YbM7pXdA"
type: "plan"
status: "proposed"
title: "Issue #423 WebView 写像・focus capability・GPUI 実窓検証の実装計画"
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
    - "../designs/issue-423-webview-dock-verify.md"
  implemented-by:
    - "../tasks/issue-423-webview-dock-verify.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-423-webview-dock-verify.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
---

# Issue #423 実装計画

設計: [designs/issue-423-webview-dock-verify.md](../designs/issue-423-webview-dock-verify.md)
受入: [specs/issue-423-webview-dock-verify.md](../specs/issue-423-webview-dock-verify.md)

## 1. 作業順（優先度順）

1. **focus capability**: `driver.rs` の `focus` 宣言文言を取得に限定し、GPUI の対テストを追加（設計 §3）。
2. **WebView 写像**: `session.rs` を neutral `WindowDriver` へ移行、`host.rs` を page load / IPC decode 専用に、
   `lib.rs` の公開面を `run_multi` / `SessionConfig` / `WebViewWindowConfig` へ（設計 §2）。examples と README を追従。
3. **Late receipt 描画**: `commit_on_host` 抽出、`view_for_test`、`rvf_driver_late_receipt_projects_into_view_after_ready`（設計 §4）。
4. **GPUI 実 window**: minimal fixture の実 OS 起動確認、Dock 検証手順の文書化（設計 §5）。
5. **文書**: impl record・報告書、#386 との対応（移行表 H1–H5 の状態）。
6. **検証**: 変更 crate の lane → 全体 lane（merge 後 HEAD）。

## 2. ファイル対応

| 変更 | ファイル |
| --- | --- |
| focus 契約文言 | `crates/ruviform-ui/src/driver.rs` |
| focus 対テスト | `crates/ruviform-ui-gpui/src/lifecycle_fixture.rs` |
| WebView 写像 | `crates/ruviform-ui-webview/src/{session,host,lib}.rs`、`examples/demo_app.rs`、`examples/demo_multi.rs`、`README.md` |
| late receipt | `crates/ruviform-ui-gpui/src/{lib,render}.rs`（test seam のみ） |
| 文書 | `docs/{specs,designs,plans,tasks,impl/ir,reports}/issue-423-*.md` |

共有ファイル（`Cargo.toml` / `Cargo.lock` / `mise.toml`）は変更しない。

## 3. 検証マトリクス

| 段階 | lane（mise） | 目的 |
| --- | --- | --- |
| WebView | `check-ui-webview` / `lint-ui-webview` / `test-ui-webview` | 写像・examples のコンパイル、clippy `-D warnings`、decode テスト |
| GPUI | `test-ui-gpui` / `lint-ui-gpui` / `check-ui-gpui` | focus 対、late receipt 描画、既存 render テスト |
| 境界 | `check-ui-driver-boundary` / `test-ui-driver-contract` | 共通層に GPUI/WebView 型が出ないこと |
| Fake/headless | `test-ui-component-view` / `test-ui-headless` / `check-ui-headless` | 既存契約の退行なし |
| MSRV | `test-ui-test-msrv` | MSRV 1.89 |
| 全体 | `fmt-check` / `check` / `lint` / `test` | workspace 全体（merge 後 HEAD） |

## 4. リスク

- WebView の実 OS 動作（WebView2 上の page load・IPC 往復）は headless で再現できない → NotRun として明示。
- `focus` 契約文言の変更は `WindowCapabilities` の利用者（consumer）の解釈に影響しうる → 設計 §3 の根拠を PR に記載。
- legacy `run` の削除は同 crate の example のみに影響（設計 §2 の逸脱）。

## 5. ロールバック

各段階は独立 commit とし、写像（段階 2）は revert 単位で戻せるように分ける。
