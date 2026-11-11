---
id: "TSPEC-34VOSINPDRV4REAL0SIN4"
type: "test-spec"
status: "approved"
title: "Issue #313 — `RealOsPointerOrKeyboard` capability 検証契約"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-313-real-os-input-driver.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-real-os-input-driver.md"
    - "docs/designs/issue-313-real-os-input-driver-design.md"
    - "docs/test-specs/issue-313-windows-pointer-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-464-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-313-real-os-input-driver.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — `RealOsPointerOrKeyboard` capability 検証契約

## 検証対象の受入条件 → 検証方法

| 受入条件 | 検証 |
| --- | --- |
| Windows で capability claim / 他 OS は claim しない | `real_window` 単体テスト（capabilities 集合 assert）+ Linux lane の `Unsupported` 観測 |
| OS 注入経路の E2E（起動→実入力→IPC→Receipt/表示→終了） | `tests/real_window.rs` の OS 注入 scenario テストを `mise run test-ui-real-os-scenario` で実走 |
| preflight gate（environment_blocked 正直記録） | `os_input_preflight` ineffective → verdict `environment_blocked` + `panic!("environment_blocked:")` タグ |
| semantic / OS 注入の証拠分離 | verdict JSON `channel: "os-injection"` タグ + `input_method: win32-SendInput` + 分離注記 |
| 環境 metadata | verdict JSON `environment` を `environment_snapshot()` 実測値で書く |
| headless 回帰なし | canonical headless lanes |

## 検証手順

```sh
mise run fmt-check check lint test test-rt \
    check-ui-gpui lint-ui-gpui test-ui-gpui \
    test-ui-scenario-contract check-ui-driver-boundary
# 実 GUI manual lanes（対話 desktop・実カーソル占有）
mise run test-ui-real-os-scenario
mise run test-ui-real-scenario
mise run example-ui-gpui-real-os-smoke
mise run example-ui-gpui-real-os-pointer-smoke
mise run example-ui-gpui-real-os-multiwindow-smoke
```

## verdict JSON（`rf313-real-os-input-scenario/v1`）

`evidence/rf313-os-capability-matrix/verdict-real-os-input-windows.json`:

- `schema` `rf313-real-os-input-scenario/v1`、`issue` 313、
  `lane` `windows-sendinput-scenario`、
  `input_method` `win32-SendInput`（`input_method_note` で OS
  automation injection ≠ 物理手動入力を明記）、
  `driver` `real-os`、`capability` `real-os-pointer-or-keyboard`。
- `verdict`: `pass` / `fail` / `not_run` / `unsupported` /
  `environment_blocked`。
- `environment`: `environment_snapshot()`（OS/arch/kernel/GPU/
  monitors/scale_factor/revision/timestamp）。
- `preflight`: `PreflightReport`（受理 vs 実効の区別）。
- `run`: `channel: "os-injection"` の入力系列（key/pointer op、
  commanded/cursor 計測、geometry/bounds）と観測 intent・receipt
  結果。semantic 経路は別タグ。
- `failure`: error/reasons/details/elapsed_ms を `UiError`/
  `ScenarioError` から保持（retry/timeout 情報を落とさない）。

## 判定

- `pass`: capability claim 済・preflight effective・全 op が
  `OsInputOutcome::Injected`・intent の atomic pair / gesture /
  receipt / `CommandPhase::Applied` が全て観測された。
- `environment_blocked`: preflight ineffective（SendInput 未達）。
  深追いせず証拠記録で終了。
- `unsupported`: capability 未 claim（macOS/Linux）。
- `not_run`: focus 未達など op が `NotRun` を返したケース。
- `fail`: `ScenarioError`（stale pin・部分注入 `UiError::Host` 等）。
