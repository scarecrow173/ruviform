---
id: "TSPEC-034QxdOanEeX8da72nWdYo"
type: "test-spec"
status: "approved"
title: "Issue #244 prefetch boundary-first/dead-generation/End commit-once test specification"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
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
  verifies:
    - "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #244 prefetch boundary-first/dead-generation/End commit-once test specification

## Purpose

Pin the observable contract of the Issue #244 amendment — boundary-first fault emission, the dead-generation `Boundary`→`Fault`→`End` sequence on reopen failure, `End` commit-once, and the seeded-fault `discard_pending` coverage (spec AC1–AC8). Retire only if these emission semantics change again.

## Feature

Producer emission-order correction on the #197/#233 prefetch contract: faults become observable (counted + surfaced) instead of stale-dropped, failed reopens terminate observably, and `End` emits exactly once per generation under backpressure.

## Rules

- Rule: every generation's first emitted item is its `Boundary`; all subsequent generation-tagged items follow it — including a generation that cannot produce content (reopen failure → `Boundary` → `Fault` → `End`).
- Rule: a rejected `seek`/`seek_discard_pending`/post-`reopen_at` seek emits `Boundary{gen, position}` then `Fault{gen, Unseekable, target, 0}` — the consumer counts `fault_events`+1 and a `fill` bumps `faults.unseekable`; the fault is not a `stale_drops`.
- Rule: a failed `open`/`reader.reopen` emits `Boundary{gen, 0}` → `Fault{gen, of_kind}` → `End{gen}`; the consumer adopts the generation (`position`→0, telemetry `generation`→`gen`), counts the fault, and `fill` reports `eof` — never starvation.
- Rule: `end_emitted` is set when the `End` marker is committed at synthesis; a requeued `End` emits once and a following `pump` returns `Idle` — never a second synthesized `End`.
- Rule: `discard_pending` applies matching-generation `Fault`/`End` state transitions while draining — a matching `Fault` counts `fault_events`; an `End` sets `ended` until the trailing `Boundary` adoption clears it (`boundary: Some`).
- Rule: seeded-fault fixtures gate failures on an armed flag — construction-time probes always run unarmed (seekable `Fact::known`), and armed failures hit the first post-open operation atomically.
- Rule: consumer-side gen-mismatch `Fault` handling is unreachable through honest producer sequences under boundary-first — it is defensive sequencing, not a coverage target.

## Examples

- Given `handle.seek(t)` against an armed seek failure, when `pump` sequences the command, then the consumer sees `Boundary{gen=N}` first and a `fill` reports `faults.unseekable == 1` with `fault_events >= 1` — not a `stale_drops` increment.
- Given `handle.reopen_at(src, open, t)` whose post-reopen seek is `Fact::Unsupported`, then `Boundary{gen=N, 0}` precedes `Fault{gen=N, Unseekable}`; the updated `rd02` test asserts the counted fault plus the frame-0 re-anchor.
- Given `handle.reopen(src, open)` with a failing `open`, then the drain yields `Boundary` adoption (telemetry `generation` == `N`, `position` == 0), a counted fault, and `fill` `eof == true`.
- Given a data ring saturated by `block_count:1` + four failed reopens, when the synthesized `End` requeues through `pending` and the ring drains, then the next `pump` returns `Idle` — exactly one `End` exists.
- Given a seeded mid-stream read failure, `discard_pending` drains `Block` → matching `Fault` (counted) → `End` (`ended`), and a subsequent `Boundary` adoption clears `ended` and reports `boundary: Some`.

## Guarantees

- All pre-existing `ruviform-media-reader` tests pass unmodified except `rd02_reopen_at_seek_failure_faults_then_reanchors`, whose `stale_drops` assertion is replaced by the corrected observable.
- New tests live in `crates/ruviform-media-reader/tests/prefetch.rs`; no new RT-lane tests are required (consumer logic unchanged).
- `mise run fmt-check`, `check`, `lint`, `test` are green.

## Non-goals

- `frames > 0` producer fault spans (still unproduced).
- Consumer-side gen-mismatch fault-arm coverage — unreachable honestly post-amendment.
- Backend/`SourceReader` failure semantics — `seek_source_frame`'s old-position guarantee is already tested.
