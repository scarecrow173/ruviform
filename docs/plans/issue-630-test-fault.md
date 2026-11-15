---
id: "PLAN-DLESE2YF8AQ5RSEOnkUhaZ"
type: "plan"
status: "approved"
title: "Issue #630 test_fault slice — 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-test-fault.md"
  implements:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
  implemented-by:
    - "docs/tasks/issue-630-test-fault.md"
  depends-on:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
    - "docs/plans/issue-630-l1-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-630-test-fault.md"
    - "docs/test-specs/issue-630-test-fault.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-630-test-fault.md"
  verified-by:
    - "docs/test-specs/issue-630-test-fault.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_fault slice — 実装計画

## File map

| file | 変更 |
| ---- | ---- |
| `crates/ruviform-plugin-test-vst3/src/component.rs` | `ComponentState<P>`/`TestComponent<P>` 一般化、`collect_param_events` free fn 化 (`wanted` param) |
| `crates/ruviform-plugin-test-vst3/src/controller.rs` | `ParamController::new(&ParamInfo, step_count)` data-driven 化 |
| `crates/ruviform-plugin-test-vst3/src/fault.rs` | 新規: `FAULT_PARAM`/CID/`TestFaultProcessor` |
| `crates/ruviform-plugin-test-vst3/src/factory.rs` | 4 class (gain comp/ctrl + fault comp/ctrl) |
| `crates/ruviform-plugin-test-vst3/src/lib.rs` | `mod fault` |
| `crates/ruviform-plugin-test-vst3/tests/host_path.rs` | 2 descriptor/factory、fault metadata/pass-through/error |
| `crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_e2e.rs` | 新規: 8 case |
| `crates/ruviform-plugin-test-clap/src/fault.rs` | 新規: `TestFaultClapPlugin` |
| `crates/ruviform-plugin-test-clap/src/lib.rs` | custom `Entry`+`PluginFactoryImpl` (2 plugin) |
| `crates/ruviform-plugin-test-clap/tests/host_path.rs` | 同上 mirror |
| `crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs` | 新規: 8 case |
| `mise.toml` | `test-l1-real-plugin` に fault e2e 行追加 |

## Dependency graph

fixture 実装 → host_path (in-process oracle) → l1 e2e (out-of-process
oracle) → mise lane → docs/evidence。production crate への依存なし
(test crate → production host bridge のみ、逆流なし)。

## Risk register

- **armed submit drop**: fire-and-forget submit が busy child で
  drop され得る → `arm_fault` 4 回送出 + error phase は毎 block
  event 同梱で対処 (実機で確認済み)。
- **error 観測面の誤認**: `results_failed_observed` は synthesized
  failure 専用 — plugin process error は `ProcessStatus::Error`
  として `process` 戻り値に現れる (実機で確認済み、oracle 修正)。
- **exit(0) 判定**: voluntary exit(0) も `Failed` 収束 —
  `voluntary_exit_zero_is_failure` 先例どおり (実機 pass)。
- **hang の child 処置**: watchdog kill まで含めて case 成功条件。

## Verification matrix

`mise run fmt-check` / `check` / `lint` / `test` (両 crate) /
`check-l1-real-plugin` / `lint-l1-real-plugin` /
`test-l1-real-plugin` (fault e2e 16 case 実走) / `test-rt` /
`prek run --all-files` / `git diff` 禁止パス空。
