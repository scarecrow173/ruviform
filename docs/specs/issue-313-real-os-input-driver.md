---
id: "SPEC-034VOSINPDRV4REAL0SIN1"
type: "spec"
status: "approved"
title: "Issue #313 — `RealOsPointerOrKeyboard` capability 実装（real-window scenario adapter 経由の OS 注入 E2E）"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-313-real-os-input-driver-design.md"
      - type: "file"
        path: "docs/plans/issue-313-real-os-input-driver-plan.md"
      - type: "file"
        path: "docs/tasks/issue-313-task1-real-os-input-driver.md"
      - type: "file"
        path: "docs/test-specs/issue-313-real-os-input-driver.md"
      - type: "file"
        path: "docs/impl/ir/issue-313-real-os-input-driver-slice.md"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/verdict-real-os-input-windows.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/real_window.rs"
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
    - "docs/specs/issue-313-windows-realos-smoke.md"
    - "docs/specs/issue-313-windows-pointer-smoke.md"
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-314-real-window-adapter.md"
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

# Issue #313 — `RealOsPointerOrKeyboard` capability 実装（real-window scenario adapter 経由の OS 注入 E2E）

## What Should Be Built

Issue #313 RF-GT-04 slice S1。`RealOsGuiTest`（`ScenarioDriver::RealOs`、
`real-scenario` feature）が `SemanticAction` + `MultiWindow` +
`VisualCapture` のみ claim し、`RealOsPointerOrKeyboard` を
「Issue #313 scope」注記つきで未実装のままにしている構造的残件を
解消する。`os_input` の SendInput 注入層（`OsKey`/`press_key`/
`move_to`/`mouse_down`/`mouse_up`/`cursor_position`/`preflight`）は
既に存在するが、scenario contract 経由で駆動する経路がない —
`SemanticGuiTest`/`GuiAction` に OS-input op がないためである。

本スライスは `RealOsGuiTest` に **OS-input port**（concrete 型上の
`os_*` メソッド群）を追加し、Windows でのみ
`ScenarioCapability::RealOsPointerOrKeyboard` を claim させる。
実 OS で「起動 → 実入力（OS 注入経路）→ IPC intent → Receipt /
表示 → 終了」を `tests/real_window.rs` 配下の scenario テストで
再現し、`evidence/rf313-os-capability-matrix/` に verdict JSON と
capability matrix 更新を記録する。

## Why Now

- Issue #313 の中核残件が capability 実装であり、example 直接駆動の
  smoke（keyboard/pointer）では contract 経路の証拠にならない。
- semantic dispatch と OS 注入経路を証拠で明確に分離する要件があり、
  `SemanticGuiTest::dispatch_input`/`GuiAction`（semantic 層）と
  混線しないポート分離が必要。
- `os_input::preflight`（Issue #552）は「注入受理 ≠ 実効」を区別する
  既存 gate であり、capability claim の実行時 gate として組み込む
  先例が確立済み。過去に本機で SendInput 無効化が発生しており、
  environment_blocked の正直な記録経路が必須。

## Acceptance（本スライス）

- `RealOsGuiTest::capabilities()` が Windows（`os_input::supported()`）
  で `RealOsPointerOrKeyboard` を claim し、macOS/Linux は claim
  しない（`require_capabilities` → `ScenarioError::Unsupported` /
  `ScenarioVerdict::Unsupported` の既存経路で正直に返る）。
- `RealOsGuiTest` に OS-input port を追加: `os_input_preflight`
  （Issue #552 gate を session cache 化）、`request_focus` /
  `window_focused`（OS フォアグラウンド前提の focus 経路）、
  `os_press_key` / `os_pointer_move_to` / `os_mouse_down` /
  `os_mouse_up`。各 op は `OsInputOutcome`（`Injected` /
  `Unsupported` / `EnvironmentBlocked` / `NotRun`）か
  `ScenarioError` を返し、部分注入・未 focus・未レイアウトを
  成功に偽装しない。
- pointer aim は `component_bounds` + `window_geometry` の計測値から
  導出し、カーソル位置は `cursor_position` read-back で記録する
  （座標のハードコードなし）。
- `tests/real_window.rs` に OS 注入 scenario テストを追加し、
  Windows 実 OS で `run_scenario`（contract runner）経由の
  「`request_focus` → `Focused(true)` → `os_press_key` /
  pointer drag/click → `drain_intents` → `post_event` receipt →
  `observe`/`capture_frame` → teardown」を再現する。
- verdict JSON（環境 metadata: OS/arch/GPU/monitor/revision/
  timestamp 実測）を `evidence/rf313-os-capability-matrix/` に
  記録。verdict 語彙は `pass`/`fail`/`not_run`/`unsupported`/
  `environment_blocked`。SendInput = OS automation injection で
  物理手動入力でない旨、および semantic dispatch
  （`dispatch_input`/`GuiAction`/`dispatch_keystroke` 系）との
  証拠分離を明記する。
- `os_input::preflight` が `effective == false` を返したらその
  lane を `environment_blocked` で記録し深追いしない。
- `mise.toml` に既存 `example-ui-gpui-real-os-*` 系と同型の
  opt-in manual task を 1 件追加（verdict JSON を capability
  matrix へ書く lane）。
- Ruviform Host/Browser-only Consumer の必須依存を増やさず、
  OS 固有部分は `gpui`/`real-scenario` feature 内に閉じる。
- 既存 headless lane が回帰しないこと。

## Non-Goals（本スライス）

- macOS/Linux の OS 注入実装・実機実測（NotRun 維持）。
- 動的 DPI・マルチモニター変化、IME capability probe。
- `GuiAction`/`SemanticGuiTest` への OS-input op 追加（設計判断は
  design doc に記録 — `OsInputOutcome` の honest 語彙を運べず
  environment_blocked が fail に偽装されるため）。
- windowed `TestIo`/`rvf.uitest.v1` 配線（slice S2）、consumer 受入。
- 右/中ボタン・修飾キー・ホイール等、`os_input` 未公開の入力種。
