---
id: "TASK-034UtEeCVQCg44aM7iUrV3"
type: "task"
status: "done"
title: "F2 正規検証・evidence・CI handoff 記録"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-306-io-e2e-fault-fuzz.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-306-io-e2e-fault-fuzz.md"
    - "docs/tasks/issue-306-f1-fixture-extension.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
  references: []
  defers: []
  deferred-by: []
---

# F2 正規検証・evidence・CI handoff 記録

## Work

<!-- Describe the implementation slice. -->

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- <!-- prerequisite, blocker, or related task -->

## Affected Files

- <!-- file, directory, module, or interface -->

## Done When

- [ ] <!-- completion criterion -->

## Verification

- [ ] <!-- command, test, review step, or observable behavior -->

## 内容

正規検証（mise lane）を実行し、`evidence/rf306/fault-fuzz-report.md`
・`docs/impl/ir/issue-306-io-e2e-fault-fuzz.md`・README 更新・
#219/#294 handoff 記述を作成する。

## 完了条件

- `mise run fmt-check`/`check-io-e2e`/`lint-io-e2e`/`test-io-e2e`/
  workspace `check`/`lint`/`test` の結果を証跡として記録
- NotRun/Unsupported が実測範囲と一致
