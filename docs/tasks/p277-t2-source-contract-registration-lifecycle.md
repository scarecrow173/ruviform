---
id: "TASK-034Rp2bMkXB7Xa0FKSWPw9"
type: "task"
status: "done"
title: "T-2 source contract: descriptors/SourceId/Registrar/Lease/lifecycle + tests + README"
created: "2026-11-03"
updated: "2026-11-03"
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
    - "docs/plans/implement-issue-277-observability-core-source.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-277-observability-core-source.md"
    - "docs/tasks/p277-t1-core-primitives-gauge-histogram.md"
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

# T-2 source contract: descriptors/SourceId/Registrar/Lease/lifecycle + tests + README

## Work

`src/source/` に descriptor.rs（CounterDescriptor/MetricDescriptor/MetricKind/
ObservationUnit/ReportingHint/ReportingImpact/AggregationPolicy/SourceSemantics/
SourceTemperature/Confidentiality）・scope.rs・identity.rs（SourceId(NonZeroU64)/
SourceGeneration/SourceIdentity）・source.rs（CounterSource/MetricSource/
MetricReader/SourceBuildError）・lifecycle.rs（SourceLifecycle/SourceState/
LifecycleError）・lease.rs（RegistrationLease/RetiredSource）・registrar.rs
（ObservationRegistrar/RegistrationError）を実装。決定的試験（寿命遷移・
失効 Error・abandon・object safety/Send+Sync）+ README Domain/Consumer 例 +
crates/README.md 更新。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-277-observability-core-source.md`
- `docs/designs/issue-277-observability-core-source-contract.md`
- `docs/specs/issue-277-observability-core-source-contract.md`

## Dependencies

- `docs/tasks/p277-t1-core-primitives-gauge-histogram.md`（MetricReader が core 型を包む）

## Affected Files

- `crates/ruviform-observability/src/{lib.rs,source/*}`、`README.md`、`crates/README.md`

## Verification

- [ ] `mise run check`/`lint`/`test`
- [ ] 寿命遷移・失効 Error・object safety の決定的試験 pass
