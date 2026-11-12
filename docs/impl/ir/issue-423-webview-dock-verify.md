---
id: "IMPL-Rt5Bv8wKm2NcXd7PqLe4Yg"
type: "impl"
status: "in-progress"
title: "Issue #423 WebView 写像・focus capability 精緻化・GPUI 実窓検証 — 実装記録"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/423"
    - "../../specs/issue-423-webview-dock-verify.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-423-webview-dock-verify.md"
      - type: "file"
        path: "docs/designs/issue-423-webview-dock-verify.md"
      - type: "file"
        path: "docs/plans/implement-issue-423-webview-dock-verify.md"
      - type: "file"
        path: "docs/tasks/issue-423-webview-dock-verify.md"
      - type: "file"
        path: "docs/reports/issue-423-gpui-real-window.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/driver.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/session.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/host.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/examples/demo_app.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/examples/demo_multi.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/README.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lifecycle_fixture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    generated: []
    deleted: []
    renamed: []
    moved: []
  implements:
    - "../../plans/implement-issue-423-webview-dock-verify.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../reports/issue-423-gpui-real-window.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../../designs/issue-423-webview-dock-verify.md"
  defers: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #423 実装記録

設計: [designs/issue-423-webview-dock-verify.md](../../designs/issue-423-webview-dock-verify.md)
報告: [reports/issue-423-gpui-real-window.md](../../reports/issue-423-gpui-real-window.md)

## 1. 実装の内容

| 項目 | 内容 | 根拠 |
| --- | --- | --- |
| focus 契約 | `WindowCapabilities::focus` の文言を「取得（`focused = true`）」に限定し、release は操作単位で `Unsupported` を返しうると明記 | design §3。`lifecycle.rs` の既存の許容と一致 |
| focus 対テスト | `gpui_focus_capability_covers_acquire_and_reports_release_unsupported` | 宣言 Supported・acquire Ok・release Unsupported の組を固定 |
| WebView 写像 | `session.rs` を neutral `WindowDriver` へ移行。`host.rs` は page load → `Ready`、IPC → `Action(Intent)`（`inbound_message`）、decode 失敗は `Event::error` を origin window へ返す | design §2（H1–H5） |
| WebView 公開面 | `run_multi` / `SessionConfig` / `WebViewWindowConfig` のみ。`run` / `HostConfig` / `SessionConfig::single` / `dispatch_script` 再 export を削除 | design §2 の逸脱節 |
| examples | `demo_app` / `demo_multi` を neutral routing（`dispatch_window_intent` / `drain_jobs_routed` / `post_event`）へ移行 | 同上 |
| late receipt | `commit_on_host` を抽出し、`rvf_driver_late_receipt_projects_into_view_after_ready` で driver 経由の描画投影を検証 | design §4 |
| test seam | `GpuiDriver::view_for_test`（`cfg(test)` + `gpui-test` のみ） | 同上 |

## 2. 受入の対応表

| 受入 | 状態 | 証拠 |
| --- | --- | --- |
| A1 WebView adapter が共通契約で動き、旧 API が adapter 内に閉じる | 達成（headless 範囲） | `rg "HostHandle\|WindowHandle\|HostEvent\|HostConfig\|\beval\("` で adapter の src/examples に一致なし。`lint-ui-webview` / `test-ui-webview`（decode 2 件 pass） |
| A2 focus の宣言と実装が一致 | 達成 | `test-ui-gpui`（36 pass、新規 1 件含む） |
| A3 GPUI 実 window の検証結果・手順が記録 | 部分達成 | 実 OS 起動で Created/Ready/Focused/Resized/Moved を確認（報告 §1）。close・Dock 2-phase・detach は手順のみ（報告 §2–3） |
| A4 late receipt が view の描画まで届く | 達成 | `rvf_driver_late_receipt_projects_into_view_after_ready`（inbox 0 件・phase Applied） |
| A5 共通層に GPUI/WebView 固有型が出ない | 達成 | `check-ui-driver-boundary`: verdict pass（dep-graph / signature / allowlist） |
| A6 `#[non_exhaustive]` / MSRV 1.89 / lint | 達成 | `lint`（workspace `-D warnings`）pass、`test-ui-test-msrv` pass。新規 public enum なし |

## 3. 検証結果（作業ツリー、merge 前の origin/main = `34d34b0a` 基点）

| mise task | 結果 |
| --- | --- |
| `fmt-check` | 修正後 pass（`fmt` で整形後） |
| `check` | pass |
| `lint` | pass |
| `test` | pass（`test result: ok` 175 ブロック、failed 0） |
| `check-ui-driver-boundary` | pass（verdict: pass） |
| `test-ui-driver-contract` | pass（7 件） |
| `check-ui-headless` / `test-ui-headless` | pass |
| `test-ui-component-view` | pass（15 件） |
| `test-ui-test-msrv` | pass |
| `test-ui-gpui` | pass（36 件） |
| `lint-ui-gpui` / `check-ui-gpui` | pass |
| `check-ui-webview` / `lint-ui-webview` | pass |
| `test-ui-webview` | pass（10 件、うち新規 2 件） |

## 4. 未実施・NotRun

- WebView の実 WebView2 上の end-to-end（page load → IPC → event の往復）: NotRun。headless では再現できず、`demo_app` の手動実行も行っていない。
- GPUI 実 window の close veto・Dock 2-phase・detach/reattach: NotRun（手順は報告 §2–3）。
- 実 OS lane（DPI・IME・OS focus・実 close・GPU device lost）: NotRun（consumer 受入 #262/#243/#390 系）。
- 旧 `ruviform-ui` の legacy trait の削除: 対象外（Phase 2 の別判断）。
- review-protocol（独立サブエージェントによる diff レビュー）: 本記録の後段で実施し、結果を PR 本文に記載する。
