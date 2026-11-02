---
id: "PLAN-034QxdNzPfzn5oye9cCzne"
type: "plan"
status: "approved"
title: "Issue #244 media-reader prefetch contract wart fixes implementation plan"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/244"
    - "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-244-media-reader-prefetch-warts.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-244-media-reader-prefetch-warts.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #244 media-reader prefetch contract wart fixes implementation plan

## Goal

Ship the four Issue #244 residual wart fixes on `ruviform-media-reader`: boundary-first fault emission and the dead-generation `Boundary`→`Fault`→`End` sequence (P2 contract amendment per ADR), `end_emitted` commit-once, and seeded-fault coverage — verified per the test-spec, with the migration note recorded.

## Scope

One plan, one crate, three ordered slices plus closeout. All edits are confined to `crates/ruviform-media-reader` (`src/prefetch.rs`, `README.md`, `tests/prefetch.rs`) and this issue's doc chain. T1–T3 all touch `src/prefetch.rs`/`tests/prefetch.rs` and are sequential.

## Upstream Documents

- `docs/specs/issue-244-media-reader-prefetch-contract-warts.md` (approved; AC1–AC8)
- `docs/adr/issue-244-prefetch-contract-warts-boundary-first-fault-emission-dead-generation-.md` (accepted; Option A + migration note)
- `docs/discovery/issue-244-media-reader-prefetch-contract-warts.md` (resolved)

## Design Inputs

- `docs/designs/issue-244-media-reader-prefetch-warts.md` (approved) — emission invariant, per-arm sequences, `end_emitted` commit-once, `SeededSource` fixture, doc-text scope.

## File Map

- `crates/ruviform-media-reader/src/prefetch.rs` — `exec_cmd` `Seek`/`Reopen` arm reorder + `Reopen` `Err` `Boundary`; `pump` `end_emitted` commit-once; module Contract + `seek`/`seek_discard_pending`/`reopen`/`reopen_at` rustdoc.
- `crates/ruviform-media-reader/README.md` — amended contract paragraphs.
- `crates/ruviform-media-reader/tests/prefetch.rs` — `SeededSource` fixture; updated `rd02_reopen_at_seek_failure_faults_then_reanchors`; new boundary-first/dead-generation/dup-End/discard-arm tests.
- `docs/impl/ir/issue-244-*.md` — implementation record + migration note (T4).

## Dependency Graph

```text
T1 boundary-first + dead-generation (P2; TDD)
  └─ T2 end_emitted commit-once (P3a; TDD)
        └─ T3 seeded fixture + discard coverage (P3b; TDD)
              └─ T4 verification + review-protocol + impl record
```

## Task Granularity

- **T1 — P2 contract amendment**: failing tests first (seeded `seek` failure → `faults.unseekable`/`fault_events` not `stale_drops`; `reopen_at` `NoSeek` — updated `rd02` test; `reopen` failure → adopt + `eof`), then `exec_cmd` reorder + `Err`-arm `Boundary`, then contract text (module Contract, rustdoc, README).
- **T2 — End commit-once**: failing test first (ring-full `End` requeue → next `pump` returns `Idle`), then the `end_emitted` move.
- **T3 — discard coverage**: `SeededSource` decode-failure → `discard_pending` matching-gen `Fault` arm + `End`-then-`Boundary` drain assertions.
- **T4 — verification + record**: `mise run fmt-check`/`check`/`lint`/`test` green; review-protocol over diff + ADR decision; impl record with migration note.

## Risk Register

- Seeded-source arm timing — mitigated: `Arc<AtomicBool>` gate, arm only after pipeline construction (probe stays unarmed).
- Ring-full `End` requeue flakiness — mitigated: `block_count:1` + four failed reopens deterministically saturate the 9-slot ring.
- Doc/index drift — mitigated: index rows updated with each doc; audit_docs re-run before impl record.
- Observable-behavior scope creep — mitigated: amendment confined to the ADR's enumerated paths; no consumer-logic edits.

## Rollback Strategy

Each task is its own commit; rollback = revert. The P2 amendment reverts cleanly (emission order is producer-internal); the migration note stays accurate for the released revision only.

## Verification Matrix

| Spec AC | Test / check | Where |
| --- | --- | --- |
| AC1 boundary-first on `seek` | `seek` + seeded seek failure → `faults.unseekable`/`fault_events` | T1 |
| AC2 boundary-first on `reopen_at` | updated `rd02_reopen_at_seek_failure_*` | T1 |
| AC3 dead-generation sequence | `reopen` failing `open` → adopt + fault + `eof` | T1 |
| AC4 single `End` under requeue | ring-full requeue → `pump` `Idle` | T2 |
| AC5 discard fault arm + End-then-Boundary | seeded read failure drain test | T3 |
| AC6 contract text updated | doc-diff review | T1 |
| AC7 suite green (≤1 semantic update) | `mise run test` | T4 |
| AC8 fmt/check/lint/test + review | mise tasks + review-protocol | T4 |

## Self-Review

- File map covers every spec'd surface (producer arms, pump flag, docs, tests, README).
- No signature/variant/consumer-logic edits anywhere — the amendment is emission order only.
- Task order keeps same-file edits sequential; each task independently revertible.
