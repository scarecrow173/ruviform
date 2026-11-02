---
id: "DISC-034Qx5mnsXSbsrFmJqSCM1"
type: "discovery"
status: "resolved"
title: "Issue #244 media-reader prefetch contract residual warts — boundary-first fault emission, reopen-failure dead generation, End commit-once"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/244"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/adr/issue-197-media-reader-boundary-additions.md"
    - "docs/adr/issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md"
    - "docs/impl/ir/issue-233-media-reader-prefetch-contract-improvements.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
    - "docs/designs/issue-233-media-reader-prefetch-improvements.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
    - "docs/adr/issue-244-prefetch-contract-warts-boundary-first-fault-emission-dead-generation-.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #244 media-reader prefetch contract residual warts — boundary-first fault emission, reopen-failure dead generation, End commit-once

## Summary

Issue #244 tracks the four residual warts the #233 review-protocol confirmed but deferred as #197 pre-existing contract semantics. Source inventory confirms two real defects and one benign wart, all inside `crates/ruviform-media-reader/src/prefetch.rs`: failure paths enqueue `Fault{gen=N}` *before* `Boundary{gen=N}`, so the fault is stale-dropped consumer-side and invisible to `fault_events`/`FaultCounts`; the reopen-failure path emits no `Boundary` at all, so the burned generation is never adopted and the consumer starves in underrun; and the lazy `End` synthesis sets `end_emitted` only on the direct-push path, so an `End` that detours through `pending` under backpressure is synthesized a second time. Tentative conclusion (confidence: high): the correct contract invariant is "every generation's first emitted item is its `Boundary`" — fixing the two P2 items is a deliberate non-additive contract-semantics correction that spec/ADR must explicitly ratify (the issue forbids a silent behavior change), while the two P3 items are ordinary wart fixes plus seeded-fault test coverage.

## Exploration Goal

Decide per wart the correct contract semantics, decide explicitly whether the P2 corrections land as an unconditional contract amendment (with consumer analysis + migration note) or an opt-in/additive shape, and confirm the seeded-fault fixture shape needed for the deferred coverage — so spec/ADR can lock the contract amendment before implementation.

## Key Issues and Assumptions

- Wart P2-a (pre-boundary `Fault` stale-drop, verified in `exec_cmd`): the `Seek` failure arm and the `Reopen{first:Some}` seek-failure arm push `Fault{gen=N, kind:Unseekable, first_frame:target, frames:0}` then `Boundary{gen=N, first_frame:position}`. The consumer adopts a generation only at `Boundary`; a `Fault{gen=N}` arriving while the consumer still serves `N-1` is stale-dropped (`stale()` → `stale_drops`), never counted in `fault_events`/`FaultCounts` — the exact observability class #197 scope-4 ("every non-content span distinguishable") wants is silently lost.
- Wart P2-b (reopen failure emits no `Boundary`, verified): the `Reopen` `Err` arm pushes `Fault{gen=N, first_frame:0, frames:0}` and sets `stream_ended`, which later synthesizes `End{gen=N}` — neither item is ever adopted, so the consumer stays at `N-1` underrunning forever while `next_gen` already burned `N`. Telemetry `generation`/`position` lag the real command history; the generation model leaks a phantom.
- Wart P3-a (duplicate `End`, verified): `pump()` step 2 synthesizes `End{gen}` lazily when `stream_ended && !end_emitted`; on a full data ring the marker is `push_front`-ed into `pending` **without** setting `end_emitted`. The requeued marker emits normally, but the next pump synthesizes a second `End{gen}` — consumer-side the same-generation `End` is idempotent (`ended` flag only), so it is benign but a wart.
- Wart P3-b (coverage gap, verified): `discard_pending`'s matching-generation `Fault` arm and `End`-then-`Boundary` sequencing have no direct test; a deterministic fault-injection source was the missing piece. All producer `Fault` emissions today use `frames:0` (point events); `frames>0` fault spans exist only via the consumer's internal `fault_remaining` gap synthesis and in-block `BlockFault`.
- Generation-adoption model (invariant the fix relies on): `step()`/`discard_pending` adopt `generation` only at `Boundary`, which also re-anchors `next_frame`, clears `fault_remaining`/`ended`. Under boundary-first emission a `Fault{gen=N}` is always seen post-adoption → counted in `fault_events` and surfaced through `Step::Faulted` → `FillReport.faults` — strictly more observable, same report vocabulary.
- Seeded-fault feasibility (verified): the symphonia adapter probes `Seek` once at `open_demuxer` (`SourceAdapter::new` → save/`End`/restore ≈ 3 seek calls). A `MediaSource` wrapper gating `read`/`seek` on an `Arc<AtomicBool>` armed by the test after construction passes the probe as seekable, then deterministically fails the first post-open seek (`Fault{Unseekable}` path) or mid-stream read (`Fault{Io}` + `stream_ended` path) — no counting guesses, no partial seeks.
- Ring-full `End` requeue is reachable: data ring capacity is `block_count + MARKER_SLACK(8)`; with `block_count:1` (pool=1 block) one block + eight queued markers (four failed `reopen`s = `B+F` each under the new ordering) saturates the ring, so the synthesized `End` must requeue through `pending` — the duplicate-End arm is deterministic.
- Post-fix reachability: with every generation's first item being its `Boundary`, a `Fault{gen≠consumer.gen}` becomes unreachable through any producer path (FIFO pending + ring order); the consumer-side gen-mismatch fault arm stays as defensive sequencing for malformed streams, not reachable coverage.
- Consumers in this workspace: `PrefetchHandle`/`PrefetchConsumer` are exercised only by `ruviform-media-reader` tests — external consumers are Synvelis (#24 E2E-L1, uses quiesce-drain→seek workaround which remains valid) and AudioLens (prefetch adoption unverified from this repo).

## Alternatives and Comparison

P2 contract handling (the decision the issue reserves for design/review):

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. Unconditional boundary-first correction: every generation's first emitted item is its `Boundary` — seek/reopen_at seek-failure emits `Boundary` then `Fault`; reopen failure emits `Boundary` → `Fault` → `End` | Restores the #197 intent (faults observable, never silently dropped); makes the dead generation well-formed (`Fault` counted, `End` delivered, `eof` reachable instead of starvation); no API surface added; consumer code already handles every item type | Observable telemetry/report change: failed seeks now bump `faults.unseekable`/`fault_events` instead of `stale_drops`; failed reopen now adopts the generation + ends the stream — a deliberate non-additive contract amendment requiring explicit migration note | yes |
| B. Opt-in flag (config or per-command) for boundary-first | Default behavior untouched | Two observable orderings double the contract surface; the wart has no legitimate consumer — nobody benefits from invisible faults; permanent complexity for a bug | no |
| C. Consumer-side: count pre-boundary faults as real faults instead of stale | No producer change | Conflates stale detection with fault reporting; does not fix the unadopted-generation starvation at all; violates boundary-driven adoption | no |

P3 fixes (not decision-bearing):

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Set `end_emitted = true` when the `End` marker is *committed* (synthesized), before `try_push` — the requeued marker still emits once | One-line semantics fix matching the "emit `End` once" doc text; stale `End` still sequences ahead of a later `Boundary` exactly as today | none | yes |
| Seeded-fault `MediaSource` fixture (`Arc<AtomicBool>` gate on `read`/`seek`) | Deterministic failure injection without seek-count guessing or partial-seek position corruption; reusable for seek-fail and decode-fail arms | Small test-only type | yes |

## Tentative Conclusions and Hypotheses

- **Confirmed**: P2-a and P2-b are the same invariant violation — failure-path markers are enqueued before (or without) the generation's `Boundary`, which the boundary-driven adoption model drops. The correct invariant: *the producer emits exactly one `Boundary` as the first item of every generation; all subsequent items of that generation follow it*. This holds after reorder for every arm (seek ok/fail, reopen ok/seek-fail/reopen-fail, plus the unchanged `fill_block` fatal-decode `Fault{current_gen}` which is already post-adoption).
- **Confirmed**: P2 corrections must be unconditional — Option A. They change observable `PrefetchTelemetry` (`fault_events` up, `stale_drops` down for these arms), `FillReport.faults` (failed seeks now bump `unseekable`), generation telemetry on reopen failure (adopts `N` then `eof` instead of starving at `N-1`), and `Step`/`FillReport` outcomes on those paths. No signature or type changes; consumer code requires no edits. Migration note required in the ADR + impl record + PR consumer-impact section (Issue #219 convention: Synvelis `no impact` — workaround still valid, observability improves; AudioLens `unknown` — no in-repo usage, flagged for reviewer resolution).
- **Confirmed**: P3-a fix is `end_emitted = true` at synthesis commit (before `try_push`); the `pending`-requeued `End` still emits exactly once, and `exec_cmd`'s `end_emitted=false` reset on the next command keeps stale-`End`-then-boundary semantics unchanged.
- **Confirmed**: P3-b coverage lands via the seeded-fault fixture: `discard_pending` matching-gen `Fault` arm + `End`-then-`Boundary` drain + the requeue path that produced duplicate `End` (asserted producer-side via `PumpOutcome::Idle` after the requeued `End` pushes — the duplicate is otherwise invisible consumer-side by design).
- **Hypothesis → verify in impl**: after the reorder, `rd02_reopen_at_seek_failure_faults_then_reanchors` must be updated — its `stale_drops >= 1` assertion describes the wart; the corrected observable is `fault_events`/`faults.unseekable` ≥ 1 with the same frame-0 re-anchor.
- **Hypothesis**: no consumer relies on post-reopen-failure starvation for liveness detection — the generation burn is already observable via `handle`'s `next_gen` bump regardless; the fix adds a well-formed terminal sequence, it does not remove a usable signal.

## Open Questions

- AudioLens prefetch adoption is not verifiable from this repo — consumer impact classification records `unknown` for the reviewer gate rather than guessing (Issue #219 convention).
- Naming: the corrected semantics need no new API names; doc text must name the new invariant ("every generation begins with its `Boundary`").

## Promotion Candidates

- [x] spec-doc needed: corrected contract semantics per wart (boundary-first emission invariant, dead-generation well-formedness, commit-once `End`), observable-change enumeration, consumer-analysis + migration requirement, acceptance criteria.
- [x] adr-doc needed: unconditional contract amendment (Option A) vs opt-in — decision rationale, #197 amendment scope, migration note, declined alternatives.

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/244 — four residual warts; #233 review-protocol provenance; #197 pre-existing semantics.
- `docs/impl/ir/issue-233-media-reader-prefetch-contract-improvements.md` — Follow-ups section, the canonical wart descriptions this issue tracks.
- `crates/ruviform-media-reader/src/prefetch.rs` — `exec_cmd` `Seek`/`Reopen` arms (~lines 585–680), `pump` `End` synthesis (~lines 505–545), consumer `step`/`discard_pending` (~lines 1000–1176) — read.
- `crates/ruviform-media-reader/src/reader.rs` — `seek_source_frame`/`reopen` position semantics on failure — read.
- `crates/ruviform-io-symphonia/src/source.rs` — `SourceAdapter` seek probe (≈3 seek calls at open) — seeded-fault fixture feasibility.
- `crates/ruviform-media-reader/tests/prefetch.rs` — existing contract tests incl. `rd02_reopen_at_seek_failure_faults_then_reanchors` (asserts the wart) — read.
- Upstream contract: `docs/specs|adr|designs/...issue-197...`, `docs/adr/issue-233-...` — semantics this work amends.

## Evidence

- 2026-11-02 — `exec_cmd` source confirmed: `Seek` failure arm pushes `Fault{Unseekable, first, 0}` then `Boundary{position}`; `Reopen` `Some(target)` seek-failure arm is identical; `Reopen` `Err` arm pushes only `Fault{of_kind, 0, 0}` + `stream_ended` (no `Boundary`).
- 2026-11-02 — `pump` source confirmed: `End` synthesized when `stream_ended && !end_emitted`; `end_emitted` set only on `try_push` success — `Err` arm `push_front`s into `pending` leaving the flag false → second `End` synthesized after the requeue drains.
- 2026-11-02 — `step()`/`discard_pending` source confirmed: gen-mismatched `Fault`/`End`/`Block` → `stale()` counter; `Boundary` adopts generation + clears `ended`; matching `End` → `ended`/`eof_seen`; matching `Fault` → `fault_events` (+ `fault_remaining` span only when `frames>0` and `first_frame == next_frame`, else `stale()`).
- 2026-11-02 — `PrefetchItem::Fault` producer emissions are all `frames: 0` point events (seek fail, reopen-at seek fail, reopen fail, fatal decode); `frames>0` consumer span handling exists for synthesized gaps/`BlockFault` coverage, not producer markers.
- 2026-11-02 — `SourceAdapter::new` probe confirmed: `stream_position` + `seek(End)` + `seek(Start)` at `open_demuxer` — a flag-gated `MediaSource` reports `Fact::known` seekable while armed-later failures hit the `exec_cmd` fault arms deterministically.
- 2026-11-02 — `SourceReader::seek_source_frame` confirmed: early `Fact::Unsupported` / demuxer `seek` `Err` returns before any `position` write — "a rejected seek keeps the old position".
- 2026-11-02 — `SourceReader::reopen` confirmed: on `Err` (metadata/decoder build) the old demuxer/decoder stay installed — the producer's generation has still advanced, so continued blocks would emit `gen=N` and stale-drop regardless; the dead-generation `Boundary→Fault→End` sequence is the honest outcome.
