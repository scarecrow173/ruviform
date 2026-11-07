---
id: "TASK-034T6RWVy5Sd3Hb9jG7wN4"
type: "task"
status: "proposed"
title: "T2 Headless behavior(ruviform-ui::behavior)"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "../plans/implement-issue-387-component-renderer.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-387-component-renderer.md"
  implemented-by: []
  depends-on: []
  blocks:
    - "../tasks/t1-component-view-spec-issue-387.md"
    - "../tasks/t3-semantic-fixtures-issue-387.md"
    - "../tasks/t4-gpui-renderer-issue-387.md"
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/issue-387-component-renderer.md"
    - "../tasks/t1-component-view-spec-issue-387.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-387-component-view-specs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T2: Headless behavior

## Work

`ruviform-ui::behavior` 新設 — `ui/core/{value,interaction}.ts` と
`ui/binding/src/contract.ts` の意味論 port。`ValueDescriptor`/
`ValueMapping`(Linear/Log/Custom trait)/`StepDomain`/`ValueMapper`/
`DisplayValue`/`ControlInput`/`InteractionEvent`/`InteractionControl`/
`InteractionDispatcher`/`DetachPolicy`、command 語彙(`CommandIntent`/
`IntentType`/`CommandReceipt`/`ReceiptStatus`/`StateProjection`/
`CommandPhase`)+ `CommandBinding` state 機械 + `intent_from_gesture`。
TS 不変条件を保持(safe Begin/terminal 後 noop/detach→cancel/
availability→cancel/receipt rank/projection revision guard)。

## Acceptance

- behavior.rs 内 unit test: mapper 往復、gesture 全遷移、
  noop/rejected/unavailable、receipt rank、detach→stale が pass。
