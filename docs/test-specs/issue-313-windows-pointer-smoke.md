---
id: "TSPEC-034UkPTR1W1N5M0K3P03"
type: "test-spec"
status: "approved"
title: "Issue #313 — Windows 実 OS SendInput ポインタ smoke 検証契約"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-windows-pointer-smoke.md"
    - "docs/designs/issue-313-windows-pointer-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-313-windows-realos-smoke.md"
    - "docs/test-specs/issue-243-gpui-realwin-windows.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-313-windows-pointer-smoke.md"
    - "docs/designs/issue-313-windows-pointer-smoke.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-313-windows-pointer-smoke.md"
    - "docs/designs/issue-313-windows-pointer-smoke.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — Windows 実 OS SendInput ポインタ smoke 検証契約

## Purpose

Issue #313 RF-GT-04 の Windows ポインタ部分スコープ: 実 OS の
ポインタ入力経路（Win32 `SendInput` 絶対座標 move + 左ボタン
遷移）で GPUI window の value control が gesture intent
（非 atomic Begin/Preview/Commit）を生むことを、計測ジオメトリ
由来の照準で検証する契約を定める。simulated input
（`dispatch_input`/`simulate_mouse_*`）や `#[ignore]` で
skip された試験を real-os の Pass に数えない。

## Feature: `os_input` ポインタ注入 API

### Rule: Windows では `SendInput` で絶対 move・左ボタン down/up を投入し、部分注入を失敗として返す

- Example: `move_to(x,y)` は
  `MOUSEEVENTF_MOVE|ABSOLUTE|VIRTUALDESK` 1 イベント、
  `mouse_down`/`mouse_up` は各 1 イベントを投入する。
  `SendInput` の受付数が期待未満なら `UiError::Host`。
  非 Windows では `UiError::Unsupported`。
- 検査: verdict JSON の `inject_result` が `"ok"` であり、
  `cursor_reads` の各 read-back が commanded 座標 ±4px 以内。

### Rule: 座標は virtual-desktop 全体で正規化される

- Example: `SM_XVIRTUALSCREEN`/`SM_YVIRTUALSCREEN`（負になり得る
  主モニタ外原点）と `SM_CX/CYVIRTUALSCREEN` から 0..=65535 へ
  正規化する。主モニタ原点 (0,0) 仮定を置かない。
- 検査: `cursor_position` が負座標をそのまま返せること
  （副モニタ環境では実測で確認。本環境は 2 monitor @1.0）。

## Feature: driver 計測ジオメトリ

### Rule: `component_bounds` は paint 記録の実測 bounds のみを返す

- Example: `GpuiDriver::component_bounds(window)` は最後の paint
  で記録された `ComponentKey → LogicalRect` を返す。未 paint・
  view 未 mount・未知 window は `UiError::Host`。ゼロサイズを
  捏造しない。
- 検査: verdict JSON の `component_bounds` に `gain`/`seek` の
  非ゼロ rect が記録される。

### Rule: `window_geometry` は client-area の物理スクリーン rect を返す

- Example: `origin`（`ClientToScreen` 実測）・`width`/`height`
  （`GetClientRect` 実測・物理 px）・`scale_factor` を返す。
  未 open/未知 window は `Host`、`gpui-test`（OS handle なし）と
  非 Windows は `Unsupported`。
- 検査: verdict JSON の `geometry` に実測値が記録され、
  `scale_factor` が `WindowDriver::monitors()` の値と整合する。

## Feature: `real_os_pointer_smoke` 実機 smoke

### Rule: Ready + OS focus + 計測 geometry/bounds が揃うまで注入しない

- Example: `WindowLifecycle::Ready` と `Focused(true)` を観測し、
  `window_geometry` と `component_bounds`（gain+seek 両方）が
  取得できた時点でのみ injector へ `Go` を送る。タイムアウト・
  Destroyed・focus 未取得は Fail（Pass にしない）。
- 検査: checks の `ready`/`os_focused`/`geometry_and_bounds` が
  全て ok。

### Rule: ドラッグとクリックが非 atomic gesture intent を生む

- Example: Fader へ下端→上端のドラッグで Begin≈0.25 /
  Commit≈0.75（同一 `gesture_id`、`atomic != true`）、Seek へ
  80% 位置クリックで Begin+Commit≈0.8（同一 `gesture_id`）。
  Preview は coalescible のため合格条件にせず件数のみ記録。
  normalized tolerance ±0.1。
- 検査: checks の `gain_drag_begin_preview_commit` /
  `seek_click_begin_commit` が ok。

### Rule: 実行前のカーソル位置を保存し終了時に復元する

- Example: 起動時に `cursor_position()` を保存し、verdict 確定・
  window close 後に `move_to(home)` で best-effort 復元する。
  復元失敗は stderr 記録のみ。
- 検査: verdict JSON に復元の成否を偽らない記録（失敗時は
  実行ログに残る）。

### Rule: unsupported/失敗を Pass に偽装しない

- Example: 非 Windows では窓を開かず verdict=`unsupported` /
  exit 2。`SendInput` 失敗・read-back 逸脱・intent 不到達は
  verdict=`fail` / exit 1 で reasons に保持する。
- 検査: exit code が verdict を反映する。`checks` 配列が
  全項目の ok を列挙する。

## Feature: capability matrix 証拠

### Rule: case 別に Pass/Fail/Unsupported/NotRun と根拠を記録する

- Example: `results.jsonl` の `real-os-pointer-input`(windows)
  行が実測 Pass + verdict JSON パスを持ち、macOS/Linux の
  同 case は NotRun のまま。report.md の判定表・実行検証表・
  NotRun/owner 表が実測と一致する。
- 検査: `evidence/rf313-os-capability-matrix/` の 3 ファイルが
  整合していること。
