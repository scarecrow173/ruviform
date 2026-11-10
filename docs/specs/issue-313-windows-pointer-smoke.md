---
id: "SPEC-034UkPTR1W1N5M0K3P01"
type: "spec"
status: "approved"
title: "Issue #313 — Windows 実 OS SendInput ポインタ smoke（部分スコープ・後続スライス）"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "docs/test-specs/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "docs/plans/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "docs/tasks/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/verdict-pointer-windows.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/capture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/report.md"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/results.jsonl"
      - type: "file"
        path: "docs/specs/issue-313-windows-realos-smoke.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-windows-realos-smoke.md"
    - "docs/specs/issue-243-gpui-realwin-windows.md"
    - "docs/specs/issue-314-golden-capture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — Windows 実 OS SendInput ポインタ smoke（部分スコープ・後続スライス）

## What Should Be Built

キーボードスライス（`docs/specs/issue-313-windows-realos-smoke.md`）
が「pointer smoke タスク」として明示的に defer した残スコープを
実施する。実 GPUI window に mount した value control
（縦 Fader + 横 Seek）に対し、Win32 `SendInput` の絶対座標
`MOUSEEVENTF_MOVE | ABSOLUTE | VIRTUALDESK` と左ボタン
down/up を、**計測済みジオメトリ**（paint 時に記録された
component bounds + OS client-area screen rect）から算出した
物理スクリーン座標へ向けて注入し、`WindowLifecycle::Action`
の非 atomic gesture intent（Begin/Preview/Commit）が実 OS
ポインタ経路で到達することを検証する manual smoke example
（`real_os_pointer_smoke`）と、capability matrix 証拠の更新を
行う。

## Why Now

Issue #313 RF-GT-04 は pointer/keyboard/focus を driver
capability として要求する。キーボード経路は前スライスで実測
Pass 済みだが、ポインタ経路には 2 つの未整備があった:

1. 注入 API の不在 — `os_input` は keyboard のみ。
2. 照準の不在 — ハードコード座標では DPI・ウィンドウ位置・
   レイアウト差で狙いを外す。paint 済み component bounds と
   OS window geometry の計測クエリが必要。

本スライスはこの 2 点を driver capability 層
（`GpuiDriver::component_bounds` / `GpuiDriver::window_geometry`）
として整備し、座標を実測値から導出する honest な pointer smoke
を追加する。

## Acceptance（本スライス）

- `os_input` に `cursor_position`/`move_to`/`mouse_down`/
  `mouse_up` を追加。Windows は `SendInput`、非 Windows は
  `UiError::Unsupported`。部分注入は成功にしない。
- `GpuiDriver::component_bounds(window)` が paint 時記録の
  `ComponentKey → LogicalRect` を返し、未 paint/未 mount/未知
  window を誤魔化さない。`GpuiDriver::window_geometry(window)`
  が client-area の物理スクリーン geometry（原点・サイズ・
  scale factor）を返し、OS handle 不在（`gpui-test`）や非
  Windows では `Unsupported`/`Host` を返す。
- `real_os_pointer_smoke` が Ready + `Focused(true)` +
  geometry/bounds 計測後に drag（Fader 下→上）と click
  （Seek 80%）を注入し、非 atomic の Begin/Commit（同一
  `gesture_id`、期待 normalized 値 ±0.1）を検査する。
  実行前のカーソル位置を保存し終了時に復元する。
- verdict JSON（`rf313-real-os-pointer-smoke/v1`）に環境
  metadata・geometry・bounds・注入対象座標・カーソル
  read-back・観測 intent・checks を記録する。
- `mise run example-ui-gpui-real-os-pointer-smoke` manual lane
  を追加（exit 0=pass / 1=fail / 2=unsupported）。
- `evidence/rf313-os-capability-matrix/` の report/results を
  pointer 行の実測で更新。macOS/Linux は NotRun のまま。
- `SendInput` が OS automation injection であり物理手動入力
  ではないこと、GPUI simulated input
  （`dispatch_input`/`simulate_mouse_*`）とは別レイヤーの
  証拠であることを verdict/docs 双方に明記する。

## Non-Goals（本スライス）

- macOS/Linux の pointer 実装・実測（実機なし、NotRun 継続）。
- 右/中ボタン、修飾キー付きポインタ、ホイール、IME/a11y、
  DPI/モニター動的変化、2 window 同時操作。
- #243 の Multiwindow/Dock 実機受入、#388/#389 Plugin Native
  GUI の範囲、#314 の pixel 証拠所有。
- `ScenarioDriver::RealOs` adapter 本体（#464 系、別タスク）。
- Host/Browser-only consumer への OS 固有依存の追加
  （`os_input`/geometry は `gpui` feature 内の test/verification
  surface に留める）。
