---
id: "REPORT-Gw6Tn2hKqP8rYc5VbL3sMd"
type: "report"
status: "completed"
title: "Issue #423 GPUI 実 window 検証の記録と手動手順（Windows 11 x64）"
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
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/issue-423-webview-dock-verify.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-423-webview-dock-verify.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-423-webview-dock-verify.md"
  derived-by: []
  verifies:
    - "../specs/issue-423-webview-dock-verify.md"
  verified-by: []
  references: []
  defers: []
---

# Issue #423 GPUI 実 window 検証の記録と手動手順

対象: `ruviform-ui-gpui` の `minimal_window` fixture（`GpuiDriver` の実 OS window）。
環境: Windows 11 x64・Git Bash。実施日: 2026-11-08。

## 1. 実施済みの確認（自動化されていない実機スモーク）

`mise run example-ui-gpui-minimal` を起動し、実 OS の GPUI window の lifecycle イベントを取得した。

```text
[fixture] capabilities: WindowCapabilities { resize: Supported, focus: Supported, scale_factor: Supported, multi_window: Supported, ime: Unsupported, native_surface_slot: Supported }
[fixture] window id: w1
[event] w1: Created
[event] w1: Ready
[event] w1: Focused(true)
[event] w1: Resized { width: 1280, height: 800 }
[event] w1: Moved { x: 320, y: 152 }
```

- `Created` → `Ready` → `Focused(true)` → `Resized` → `Moved` が実機で届くことを確認。
- 確認後、この agent からは window の閉じる操作ができないため、プロセスを強制終了した
  （終了コード 1。`CloseRequested` を経由しない終了であり、正常終了の確認にはならない）。
- `ime` は `Unsupported`、`native_surface_slot` は `Supported` と宣言されている（GPUI の OS 判定に従う）。

## 2. 手動で実施する手順（未実施・NotRun）

以下は人が GUI 上で実施する。結果は本節の表に追記する。

| # | 手順 | 期待結果 | 結果 |
| --- | --- | --- | --- |
| M1 | `mise run example-ui-gpui-minimal` を実行し、window の閉じるボタンを押す | `CloseRequested` → `respond_close(Approve)` → `Destroyed` → `[fixture] exited cleanly`、終了コード 0 | NotRun |
| M2 | window をドラッグして移動し、リサイズする | `Moved` / `Resized` のイベントが届く | NotRun |
| M3 | 別アプリを前面にしてから window をクリックする | `Focused(false)` → `Focused(true)` | NotRun |
| M4 | 別モニタへ移動する（DPI の異なるモニタがある場合） | `ScaleFactorChanged` | NotRun（実 OS lane） |

M1 の close veto（Defer / Veto）は `minimal_window` が Approve 固定のため実機では再現されない。
veto の実機確認は別の検証 entry が必要で、本 Issue では扱わない。

## 3. Dock の実 window 検証について

- 2-phase move（`begin_move` / `commit_move` / `abort_move`）・detach/reattach・close veto の
  同一 trace は headless fixture（`ruviform-ui-test` の共通 lifecycle、Fake と GPUI の
  `TestAppContext` の両方）で検証済み。
- 実 window 上の `DockWorkspace` 統合は未実施。`minimal_window` には Dock UI がないため、
  Dock を実 window で動かすには専用の entry が必要になる。
  `poc-native-dock`（RF388/389 の native dock PoC）は別経路であり、本 Issue の GPUI Dock 統合の証拠にはならない。
- 実 OS lane（DPI・IME・OS 起因 focus・実 close・GPU device lost）は consumer 受入連携
  （#262 / #243 / #390 系）の範囲として NotRun。

## 4. 再現方法

```sh
mise run example-ui-gpui-minimal
```

実 OS の window を開くため、CI や headless 環境では実行しない（manual verification entry）。
