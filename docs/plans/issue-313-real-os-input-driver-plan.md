---
id: "PLAN-034VOSINPDRV4REAL0SIN3"
type: "plan"
status: "approved"
title: "Issue #313 — `RealOsPointerOrKeyboard` capability 実装計画"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-313-task1-real-os-input-driver.md"
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
  implements:
    - "docs/designs/issue-313-real-os-input-driver-design.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-313-real-os-input-driver-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# Issue #313 — `RealOsPointerOrKeyboard` capability 実装計画

## File map

| file | 変更 |
| --- | --- |
| `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs` | `OsInputOutcome` enum 追加（public `#[non_exhaustive]`） |
| `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs` | focused map・preflight cache・`os_*` ops・capability claim・module doc・単体テスト更新 |
| `crates/ruviform-ui-gpui/tests/real_window.rs` | OS 注入 scenario テスト（Windows E2E + 非 Windows `unsupported` 証拠） |
| `mise.toml` | `test-ui-real-os-scenario` manual task 1 件 |
| `evidence/rf313-os-capability-matrix/` | verdict JSON 新規・report.md / results.jsonl 更新 |
| `evidence/rf552-sendinput-preflight/` | scenario lane の preflight verdict（必要な場合） |
| `evidence/issue-314-real-scenario/driver-lab-evidence.jsonl` | emit 行追加（実走時） |

## Task 分割

1. T1: `os_input::OsInputOutcome` + `RealOsGuiTest` OS-input port（
   focused map・preflight gate・ops・capability claim）。
2. T2: `tests/real_window.rs` の OS 注入 scenario テスト +
   `test-ui-real-os-scenario` lane + verdict JSON。
3. T3: 実 GUI lane 実走・証拠化・matrix 更新・impl record。

## 検証行列

| 対象 | 検証 |
| --- | --- |
| 契約回帰 | `mise run fmt-check` `check` `lint` `test` `test-rt` `check-ui-gpui` `lint-ui-gpui` `test-ui-gpui` `test-ui-scenario-contract` `check-ui-driver-boundary` |
| Windows 実 OS E2E | `mise run test-ui-real-os-scenario`（実走必須） + `test-ui-real-scenario`（回帰） |
| smoke 回帰 | `example-ui-gpui-real-os-smoke` / `-pointer-smoke` / `-multiwindow-smoke` |
| environment_blocked | preflight ineffective → `environment_blocked` 証拠 + tag panic |

## リスク / ロールバック

- SendInput 環境遮断（過去発生・原因未同定）: preflight gate が
  `environment_blocked` を返したら深追いせず証拠記録で仕上げる。
- 実 GUI lane は実カーソル/キーボードを占有 — unattended 実行前提を
  PR に記録。
- フォーカス観測未達: `window_focused` が立たなければ ops は
  `NotRun` を返し lane は fail — timeout 記録を保持。
