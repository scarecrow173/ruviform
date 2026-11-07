---
id: "SPEC-Qm7RvT2kXc9LpW4dHs8NfA"
type: "spec"
status: "proposed"
title: "Issue #423 WebView 側写像・GPUI Dock 実窓検証・focus capability 精緻化（#386 残 DoD）"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/423"
    - "https://github.com/scarecrow173/ruviform/issues/386"
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
    - "../specs/issue-386-driver-neutral-gui-host.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  refines:
    - "../specs/issue-386-driver-neutral-gui-host.md"
  refined-by: []
  derives-from:
    - "../specs/issue-386-driver-neutral-gui-host.md"
  derived-by:
    - "../designs/issue-423-webview-dock-verify.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  defers: []
---

# Issue #423 WebView 側写像・GPUI Dock 実窓検証・focus capability 精緻化

## 背景

Issue #386 の共通 Host 契約（`WindowDriver` / `DriverEvent` / `WindowLifecycle`）、
GPUI Driver、共通 Window/Dock lifecycle fixture（Fake/GPUI 同一 trace）は main に統合済み
（`df3237e2`）。一方、移行表 H1–H5 の WebView 側写像、GPUI 実 window 上の Dock 検証、
`focus` capability の宣言と実装の不整合、Late receipt の描画までの検証は未達のまま残っており、
本 Issue で追跡する。

## 目的

- WebView adapter が共通 `WindowDriver` 契約を実装し、旧 WebView API
  （`HostHandle::eval` / `WindowHandle::eval` / `HostEvent::Ready` / `Message` /
  `WindowConfig::url` / `devtools` / `rvf.ipc.v1` の decode）が adapter 内に閉じること。
- `focus` capability の宣言と `focus(_, false)` の挙動が一致すること。
- GPUI 実 window の Dock lifecycle 検証結果（実施結果、または実機手順）が記録されていること。
- Late receipt が view の描画（binding 状態）まで届くことを検証すること。

## スコープ

含む:

1. WebView adapter の H1–H5 写像（`post_event` → `evaluate_script(dispatch_script)`、
   page load → `Ready`、IPC body → `Action(Intent)`、`WebViewWindowConfig` への局所化）。
2. `focus` capability の契約修正（宣言の意味を明確化し GPUI 挙動と一致させる）。
3. Late receipt の driver 経由描画検証（GPUI `#[gpui::test]`）。
4. GPUI 実 window の検証: minimal fixture の実 OS 起動確認と、Dock 2-phase / detach / close veto の実機手順の文書化。

含まない（未達として明示する）:

- 実 OS lane（DPI 遷移・IME・OS 起因 focus・実 close・GPU device lost）。consumer 受入連携
  （#262 / #243 / #390 系）で扱う。
- GPUI 実 window 上の `DockWorkspace` 統合（minimal fixture に Dock UI がない）。
- 旧 `ruviform-ui` の legacy trait（`HostHandle` / `WindowHandle` / `HostEvent` / `WindowConfig`）の削除。
  共通層の互換のため残置する（Phase 2 の別判断）。

## 受入条件

| ID | 受入条件 | 検証 |
| --- | --- | --- |
| A1 | WebView adapter が `WindowDriver` 相当の契約で動作し、旧 API 呼び出しが adapter 内に閉じる | `rg` で adapter 外に legacy 型が出ないこと、`mise run lint-ui-webview` / `test-ui-webview` |
| A2 | `focus` の宣言と `focus(_, false)` の挙動が一致する | `test-ui-gpui` の capability 対 release テスト |
| A3 | GPUI 実 window で Dock lifecycle の実機確認結果、または実機手順が記録される | 報告書 `docs/reports/issue-423-gpui-real-window.md` |
| A4 | Late receipt が inbox 件数ではなく view の binding 状態まで届く | `rvf_driver_late_receipt_projects_into_view_after_ready` |
| A5 | 共通層に GPUI/WebView 固有型が出ない | `mise run check-ui-driver-boundary` pass |
| A6 | public enum は `#[non_exhaustive]`、MSRV 1.89、workspace lint 準拠 | `lint` / `lint-ci` / `test-ui-test-msrv` |

## 参照

- 設計: [designs/issue-423-webview-dock-verify.md](../designs/issue-423-webview-dock-verify.md)
- 移行表（H1–H13）: [designs/issue-386-driver-neutral-gui-host.md](../designs/issue-386-driver-neutral-gui-host.md)
