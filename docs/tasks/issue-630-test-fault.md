---
id: "TASK-3bwAN8UwWS5HsnXVLLCAap"
type: "task"
status: "done"
title: "Issue #630 test_fault slice — task"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-630-test-fault.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-test-fault.md"
  implements:
    - "docs/plans/issue-630-test-fault.md"
  implemented-by:
    - "docs/impl/ir/issue-630-test-fault.md"
  depends-on:
    - "docs/plans/issue-630-test-fault.md"
    - "docs/tasks/issue-630-l1-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-fault.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-630-test-fault.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-test-fault.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_fault slice — task

- [x] VST3: `component.rs`/`controller.rs` 一般化 + `fault.rs` +
      factory 4 class + host_path 更新
- [x] CLAP: `fault.rs` + custom entry (2 plugin) + host_path 更新
- [x] 両 crate `tests/l1_fault_test_plugin_e2e.rs` 8 case
      (descriptor/clean, abort, exit(42), exit(0), hang+timeout,
      bundled fan-out, dedicated isolation, error defined
      rejection)
- [x] `mise.toml` `test-l1-real-plugin` fault e2e 行
- [x] fmt/check/lint/test 両 crate、check/lint/test-rt/l1 lane
- [x] doc chain + evidence + index 再生成
- [x] review-protocol、commit/push/PR `Refs #630`
