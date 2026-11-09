---
id: "TASK-034UmKPCkhRiRPGs873bWz"
type: "task"
status: "done"
title: "Issue #410 — view command ledger bounded retention & correlation semantics"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  implemented-by:
    - "docs/impl/ir/issue-410-view-command-ledger-bounded-retention-and-correlation-hardening.md"
  depends-on:
    - "docs/plans/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    - "docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-410-view-command-ledger-retention-correlation-guarantees.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #410 — view command ledger bounded retention & correlation semantics

## Work

`crates/ruviform-ui/src/view.rs` と `behavior.rs` に対し、
Issue #410 の view.rs/behavior.rs slice（bounded retention +
correlation semantics hardening）を実装する:

1. `CommandBinding::record_ledger_state`（`pub(crate)`）と
   `expire_command`（配送喪失 → `expired` 記録）を追加。
2. `CommandBinding::outbox` に `MAX_OUTBOX_DISPATCHES` bound。
3. `ViewHost::command_index` に `MAX_COMMAND_INDEX` bound +
   liveness-class eviction。
4. `push_event` の drop 契約修正 + `Intent` victim 退役。
5. `validate_spec` の command owner id dedup + `key_for_control`
   canonical 優先。
6. 単調 `u64` カウンタの checked_add 化。
7. 上記全ての回帰 unit test。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md`
- `docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md`
- `docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md`
- `docs/test-specs/issue-410-view-command-ledger-retention-correlation-guarantees.md`

## Dependencies

- なし（blocked by 無し）。

## Affected Files

- `crates/ruviform-ui/src/view.rs`
- `crates/ruviform-ui/src/behavior.rs`

## Done When

- [x] `command_index`/`outbox` が bounded で in-flight 保護の
  yield を持つ
- [x] `push_event` の non-droppable drop が契約化され、`Intent`
  喪失時に `Timeout` + unrouted receipt で観測可能
- [x] `validate_spec` が重複 command owner id を reject し、
  tab alias を誤 reject しない
- [x] 単調カウンタが checked_add 化されている
- [x] 対応 unit test が追加され pass する

## Verification

- [x] `mise run fmt-check` exits 0
- [x] `mise run check` exits 0
- [x] `mise run lint` exits 0
- [x] `mise run test` exits 0
- [x] `mise run test-ui-component-view` exits 0
