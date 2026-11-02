---
id: "PLAN-034Qonvu2E5YQaWUHEcTeL"
type: "plan"
status: "approved"
title: "Issue #233 media-reader prefetch contract improvements implementation plan"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/233"
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-233-media-reader-prefetch-improvements.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-233-media-reader-prefetch-improvements.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #233 media-reader prefetch contract improvements implementation plan

## Goal

Ship the four additive Issue #233 improvements on `ruviform-media-reader`: doc-only `expected_first`/`pump()` contract rewrites, the `discard_pending` consumer drain assist + `seek_discard_pending` producer pending purge, and the fused `reopen_at` — all additive on the #197 contract, verified per the test-spec.

## Scope

One plan, one crate, three ordered slices plus closeout. All edits are confined to `crates/ruviform-media-reader` (source, tests, README) and this issue's doc chain. T2 and T3 both touch `src/prefetch.rs` and are sequential; T1 is doc-only but shares the file, so it lands first to keep diffs reviewable.

## Upstream Documents

- `docs/specs/issue-233-media-reader-prefetch-contract-improvements.md` (approved; AC1–AC8)
- `docs/adr/issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md` (accepted)
- `docs/discovery/issue-233-media-reader-prefetch-contract-improvements.md` (resolved)

## Design Inputs

- `docs/designs/issue-233-media-reader-prefetch-improvements.md` (approved) — command surface fields, handle methods, drain semantics, producer/exec_cmd changes, naming.

## File Map

- `crates/ruviform-media-reader/src/prefetch.rs` — module Contract section text; `fill`/`FillReport`/`pump`/`PumpOutcome` rustdoc; `PrefetchCmd::{Seek.discard_pending,Reopen.first}` fields; `PrefetchProducer::exec_cmd` pending purge + post-reopen seek; `PrefetchConsumer::discard_pending`; `DiscardReport`; `PrefetchHandle::{seek_discard_pending,reopen_at}` + shared send path.
- `crates/ruviform-media-reader/src/lib.rs` — re-export `DiscardReport`.
- `crates/ruviform-media-reader/README.md` — contract paragraphs + new API entries.
- `crates/ruviform-media-reader/tests/prefetch.rs` — contract tests: seek-discard purge, drain backlog + Boundary adoption, reopen_at landed/failure paths, reopen regression.
- `crates/ruviform-media-reader/tests/rt_fill.rs` — `rt_`-prefixed `discard_pending` allocation-free/bounded test.
- `docs/impl/ir/issue-233-*.md` — implementation record (T4).

## Dependency Graph

```text
T1 doc-contract rewrite (candidates 1+3)
  └─ T2 discard_pending + seek_discard_pending (candidate 2; TDD)
        └─ T3 reopen_at (candidate 4; TDD)
              └─ T4 verification + impl record
```

T2 depends on T1 only textually (same file regions are adjacent); T3 depends on T2 (shared `exec_cmd`/`PrefetchCmd` region); T4 depends on all.

## Task Granularity

- **T1 — contract docs**: rewrite `fill`/`FillReport.mismatch`/`pump`/`PumpOutcome` rustdoc, module Contract section, crate README paragraphs for candidates 1 and 3. No behavior change.
- **T2 — discard surface**: failing tests first (drain adopts Boundary; in-flight+queued blocks return to free ring; `boundary` report field; discard-flagged seek purges `pending`), then `PrefetchCmd::Seek.discard_pending`, `pending.clear()` in `exec_cmd`, `PrefetchHandle::seek_discard_pending`, `PrefetchConsumer::discard_pending` + `DiscardReport`, `lib.rs` re-export, RT `rt_` test.
- **T3 — reopen_at**: failing tests first (landed Boundary, seek-failure Fault+Boundary, reopen regression), then `PrefetchCmd::Reopen.first: Option<u64>`, post-reopen seek in `exec_cmd`, `PrefetchHandle::reopen_at`, `reopen` delegation, README entry.
- **T4 — verification + record**: `mise run fmt-check`/`check`/`lint`/`test` green; per-candidate outcome + verification evidence in `docs/impl/ir/issue-233-*.md`.

## Risk Register

- Drain-before-Boundary residual play-through — mitigated: `boundary: Option<u64>` report + documented convergence rule; risk: caller misreads `None` as failure → doc text + test pin semantics.
- Dropping `current` without a following Boundary creates a sequence-gap → fault span — mitigated: documented "call after seek/reopen" misuse path; risk: silent misuse → telemetry `fault_events`/`stale_drops` observable.
- `PrefetchItem`/enum accidental breaking change — mitigated: changes confined to private `PrefetchCmd` + new public `DiscardReport`; compile catches it.
- RT violation in drain — mitigated: bound = ring capacity + 1, `assert_no_alloc` test; risk: `try_pop` error handling mis-bounds loop → test asserts drop count.

## Rollback Strategy

Each task is its own commit; rollback = revert the task commit. T2/T3 are additive methods + private fields — removal is a clean revert with no migration. T1 reverts doc text only. No persistent data formats, no feature flags.

## Verification Matrix

| Spec AC | Test / check | Where |
| --- | --- | --- |
| AC1 `expected_first` readable from docs | doc-diff review; rustdoc text | T1 |
| AC2 drain adopts Boundary, no pre-boundary content after | `discard_pending_*` contract tests | T2 |
| AC3 bounded + alloc-free drain | `rt_` test with `assert_no_alloc` | T2 |
| AC4 discard-flagged seek purges pending | `seek_discard_pending_*` contract test | T2 |
| AC5 pump iteration contract readable from docs | doc-diff review | T1 |
| AC6 reopen_at landed/failure paths | `reopen_at_*` contract tests | T3 |
| AC7 existing tests unmodified | `mise run test` full suite | T4 |
| AC8 fmt/check/lint/test green | mise tasks | T4 |

## Self-Review

- File map covers every spec'd surface (rustdoc, module head, README, tests, lib re-export).
- No `PrefetchItem`/telemetry/public signature edits anywhere — additivity holds by construction.
- Task order keeps same-file edits sequential; each task is independently revertible.
