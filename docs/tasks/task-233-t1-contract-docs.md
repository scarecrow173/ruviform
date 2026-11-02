---
id: "TASK-034Qooz8UlxyXeGHTFwHXk"
type: "task"
status: "done"
title: "Issue 233 T1: prefetch contract doc rewrites — expected_first stream-position assertion + pump iteration contract"
created: "2026-11-02"
updated: "2026-11-02"
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
    - "docs/plans/issue-233-media-reader-prefetch-improvements.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-233-media-reader-prefetch-improvements.md"
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
    - "docs/test-specs/issue-233-media-reader-prefetch-improvements.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 233 T1: prefetch contract doc rewrites — expected_first stream-position assertion + pump iteration contract

## Work

Doc-only contract rewrites for candidates 1 and 3. In `crates/ruviform-media-reader/src/prefetch.rs`: (a) `fill` rustdoc — define `expected_first` as the caller's asserted stream position in source frames compared against `next_frame` (never an engine-frame/output expectation); (b) `FillReport.mismatch` field doc — the position assertion disagreed; nothing consumed; re-anchor via `seek`; (c) module Contract section — add the position-assertion wording and the pump iteration rule; (d) `pump`/`PumpOutcome` rustdoc — one call drains all queued commands then performs ≤1 sequenced item; `Backpressured` = work exists but a ring has no slot (iterate, not done); markers may wait in producer `pending`; manual drivers iterate until `Idle`/`Ended`. In `crates/ruviform-media-reader/README.md`: the same contract paragraphs. No signature, type-name, or behavior change.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-233-media-reader-prefetch-improvements.md`
- `docs/specs/issue-233-media-reader-prefetch-contract-improvements.md` (AC1, AC5)
- `docs/adr/issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md`
- `docs/designs/issue-233-media-reader-prefetch-improvements.md`

## Dependencies

- None (first task; sequential for same-file hygiene).

## Affected Files

- `crates/ruviform-media-reader/src/prefetch.rs` (rustdoc + module head only)
- `crates/ruviform-media-reader/README.md`

## Done When

- [x] `expected_first` contract is derivable from `fill`/`FillReport` rustdoc + module Contract section without reading tests.
- [x] `pump()`/`PumpOutcome` docs state ≤1 item per call, command-first order, `Backpressured` retry semantics, and `pending` marker retention.
- [x] README mirrors both contract paragraphs.
- [x] No code behavior change (doc-only diff).

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check`
- [x] `mise run lint`
- [x] `mise run test` — all pre-existing tests pass unmodified.
