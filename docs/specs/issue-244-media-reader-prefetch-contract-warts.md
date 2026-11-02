---
id: "SPEC-034Qx7sstYyddu4xbj5Q7d"
type: "spec"
status: "approved"
title: "Issue #244 media-reader prefetch contract residual wart fixes — boundary-first fault emission, reopen-failure dead generation, End commit-once"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/244"
    - "docs/discovery/issue-244-media-reader-prefetch-contract-warts.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/README.md"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
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
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-233-media-reader-prefetch-contract-improvements.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
  refined-by: []
  derives-from:
    - "docs/discovery/issue-244-media-reader-prefetch-contract-warts.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #244 media-reader prefetch contract residual wart fixes — boundary-first fault emission, reopen-failure dead generation, End commit-once

## What Should Be Built

Four residual wart fixes on the #197/#233 prefetch contract, scoped per item:

1. **Boundary-first fault emission (P2, contract amendment).** On `seek`/`seek_discard_pending` rejection and on `reopen_at` post-reopen seek failure, the producer emits `Boundary{gen=N, first_frame:landed}` *before* `Fault{gen=N, kind:Unseekable, first_frame:target, frames:0}` — reversing today's `Fault` → `Boundary` order. Under boundary-driven adoption the consumer then counts the fault in `fault_events` and surfaces it through `Step::Faulted` → `FillReport.faults.unseekable`, instead of stale-dropping it invisibly into `stale_drops`.
2. **Dead-generation sequence on reopen failure (P2, contract amendment).** A failed `open`/`reader.reopen` emits `Boundary{gen=N, first_frame:0}` → `Fault{gen=N, kind:of_kind(error), first_frame:0, frames:0}` → `End{gen=N}` (via the existing `stream_ended` path) — a well-formed terminal generation. The consumer adopts `N`, observes the fault, reaches `ended`/`eof`, and telemetry `generation`/`position` converge, instead of starving at `N-1` in underrun with a burned, never-adopted generation.
3. **`End` commit-once (P3, wart fix).** `end_emitted` is set when the producer *commits* the `End` marker (at synthesis, before `try_push`), not only on direct-push success — so an `End` requeued through `pending` under backpressure is never synthesized a second time for the same generation.
4. **Seeded-fault coverage (P3).** A deterministic fault-injection `MediaSource` fixture (flag-gated `read`/`seek` failures armed after construction, passing the symphonia seek probe) plus contract tests covering: `discard_pending`'s matching-generation `Fault` arm, `End`-then-`Boundary` drain sequencing, the requeued-`End` duplicate emission (asserted via `PumpOutcome`), boundary-first fault observability on `seek`/`reopen_at` failure, and generation adoption + `eof` on reopen failure.

## Why Now

The #233 review-protocol confirmed these as real defects but deferred them as #197 pre-existing semantics requiring a non-additive contract decision — exactly what this issue exists to make. They are live observability holes: failed seeks/reopens are the paths a consumer most needs classified (scope-4 of #197: every non-content span distinguishable), and today they are the only spans the contract reports *silently*. Synvelis E2E-L1 already works around seek semantics via quiesce-drain; leaving the reporting hole in place means every future consumer inherits invisible failure modes.

## Users and Value

- RT playback consumers (Synvelis today; any engine driving `PrefetchConsumer`): failed seeks and failed reopens become counted, classified faults with a well-formed terminal sequence instead of silent stale drops and generation starvation — liveness and diagnosis improve with zero consumer code change.
- Non-RT drivers/telemetry readers: `fault_events`, `generation`, `position` reflect real command outcomes; `stale_drops` regains its intended meaning (only genuinely stale items).
- Future maintainers: the emission invariant "every generation begins with its `Boundary`" is stated in the contract, and the wart paths are pinned by seeded-fault tests.

## Scope

### In Scope

- Producer emission-order corrections in `exec_cmd` (`crates/ruviform-media-reader/src/prefetch.rs`): boundary-before-fault on the `Seek` failure arm and `Reopen{first:Some}` seek-failure arm; `Boundary` + `Fault` + `stream_ended` on the `Reopen` `Err` arm.
- `end_emitted` commit-once semantics in `pump()`.
- Contract text updates: module `## Contract` section (new emission invariant + reopen-failure sequence), `seek`/`seek_discard_pending`/`reopen`/`reopen_at` rustdoc failure-path descriptions (the `reopen_at` rustdoc currently documents the warts verbatim), crate README.
- Seeded-fault `MediaSource` fixture + contract tests in `crates/ruviform-media-reader/tests/prefetch.rs`; update of `rd02_reopen_at_seek_failure_faults_then_reanchors` whose `stale_drops` assertion pins the wart.
- Consumer analysis + migration note (ADR + impl record + PR consumer-impact section per Issue #219 convention).
- Doc chain for this issue (discovery → spec → ADR → design → plan → test-spec → tasks → impl record) per `doc-driven-dev-graph`.

### Out of Scope

- Any signature, type, or `PrefetchItem` variant change; no new public API and no opt-in flag — the amendment is unconditional per the ADR decision.
- Consumer-side `step`/`discard_pending`/`fill`/`copy_block` logic changes — the adoption model is the invariant being restored, not changed.
- `frames > 0` producer fault-span emission (still unproduced; consumer span handling is already in place for synthesized gaps/`BlockFault`).
- Reader/demuxer/backend changes (`seek_source_frame` failure position semantics are already correct); engine-frame mapping; Synvelis/AudioLens app-side code.
- RT-path semantics: no new RT surface; `discard_pending`/`fill` bounds and allocation-freedom unchanged.

## Requirements

- **Contract amendment, not silent drift**: the emission-order change is recorded as a deliberate amendment of #197 failure-path semantics (ADR + migration note + PR consumer-impact classification); no opt-in or dual orderings.
- **Emission invariant**: every generation's first item is its `Boundary`; all subsequent generation-tagged items follow it. After the amendment this holds for every arm — including a generation that can never produce content (reopen failure: `Boundary` → `Fault` → `End`).
- **Honest reporting preserved**: corrected faults count in `fault_events`/`FillReport.faults` and dead generations reach `ended`/`eof`; nothing is added to hidden state — stale drops still count genuinely stale items.
- **Consumer compatibility**: no signature/type changes; consumers already handle `Boundary`/`Fault`/`End` in every order the new sequences produce. Observable changes are enumerated in the ADR migration note.
- **RT contract unchanged**: all changes are producer-side (non-RT); consumer logic untouched.
- **Verification**: new/changed behavior pinned by contract tests (fault injection, boundary ordering, duplicate `End`, generation adoption); `mise run fmt-check`/`check`/`lint`/`test` green; review-protocol over the diff and the contract decision.

## Acceptance Criteria

- [ ] AC1: A failed `seek`/`seek_discard_pending` emits `Boundary{gen=N}` then `Fault{gen=N, Unseekable}`; the consumer counts it in `fault_events` and a `fill` bumps `FillReport.faults.unseekable` — `stale_drops` is not incremented by the fault.
- [ ] AC2: A failed post-reopen seek under `reopen_at` emits `Boundary{gen=N, landed=0}` then `Fault{gen=N, Unseekable}`; observable via `fault_events`/`faults.unseekable` with the frame-0 re-anchor.
- [ ] AC3: A failed `reopen`/`reopen_at` (`open` or `reader.reopen` errors) emits `Boundary{gen=N,0}` → `Fault{gen=N}` → `End{gen=N}`; the consumer adopts `N`, the fault is counted, `fill` reports `eof`, and `telemetry().generation()` converges to `N`.
- [ ] AC4: `end_emitted` is set at `End` commit, so an `End` requeued through `pending` under a full ring emits exactly once — a following `pump` returns `Idle`, never a second synthesized `End`.
- [ ] AC5: `discard_pending` covers its matching-generation `Fault` arm and `End`-then-`Boundary` sequencing under the seeded-fault fixture (fault counted, `ended` applied then cleared by the boundary adoption).
- [ ] AC6: Contract text (module Contract section, `seek`/`seek_discard_pending`/`reopen`/`reopen_at` rustdoc, README) states the boundary-first invariant and the dead-generation sequence; the wart descriptions are removed.
- [ ] AC7: All other pre-existing media-reader tests pass with at most the `rd02_reopen_at_seek_failure` semantic update (the wart assertion replaced by the corrected observable).
- [ ] AC8: `mise run fmt-check`, `mise run check`, `mise run lint`, `mise run test` green; review-protocol verdict recorded.

## Deferred Design Concerns

- Consumer-side gen-mismatch `Fault` arm in `step`/`discard_pending` — unreachable through honest producer sequences once boundary-first holds; kept as defensive sequencing for malformed streams. Not removal-worthy.
- Post-seek-failure source-position fidelity (a partially-seeked backend could emit mislabeled content after a rejected seek) — pre-existing, not caused by this change; report separately if observed.

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/244 — four residual warts; #233 review-protocol provenance; #197 pre-existing contract semantics.
- Discovery: `docs/discovery/issue-244-media-reader-prefetch-contract-warts.md` — verified emission/pump/adoption mechanics, seeded-fault feasibility, option tables.
- Upstream contract being amended: `docs/specs/issue-197-media-reader-prefetch-rt-consumer.md`, `docs/adr/issue-197-media-reader-boundary-additions.md` (refined — failure-path emission semantics only); `docs/adr/issue-233-...` (additive surface unchanged).
- Policy: `docs/adr/realtime-contract-and-verification-policy.md`, `docs/adr/public-enum-non-exhaustive-policy.md`.
