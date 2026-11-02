---
id: "SPEC-034QoidM48Z7Yl3Xtw57h2"
type: "spec"
status: "approved"
title: "Issue #233 media-reader prefetch fill/seek/pump contract clarifications + additive discard/reopen features"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/233"
    - "docs/discovery/issue-233-media-reader-prefetch-contract-improvements.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/README.md"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/rt_fill.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/adr/issue-197-media-reader-boundary-additions.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
  refined-by: []
  derives-from:
    - "docs/discovery/issue-233-media-reader-prefetch-contract-improvements.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #233 media-reader prefetch fill/seek/pump contract clarifications + additive discard/reopen features

## What Should Be Built

Four additive improvements on the merged #197 Prefetch/Seek/Pump contract, each scoped per candidate:

1. **`PrefetchConsumer::fill` `expected_first` semantics (doc-only).** Rewrite the contract text on `fill`, `FillReport.mismatch`, the module Contract section, and the crate README so `expected_first` reads as the caller's *asserted stream position* — the source frame the caller believes the stream's `next_frame` currently equals — not an engine-frame or output-frame expectation. No signature, type, or behavior change.
2. **Seek stale pending play-through (feature).** Additive drain/discard surface so a sample-exact re-anchor does not require an output-producing quiesce-drain→seek choreography: (a) `PrefetchConsumer::discard_pending` — a bounded, RT-suitable drain that pops the in-flight block and queued data-ring items, applies the stream's sequencing rules (Boundary adopts generation and re-anchors position, markers update state, blocks return to the free ring) without writing to a destination, and reports what it dropped and whether a `Boundary` was adopted; and (b) a `PrefetchHandle` seek variant carrying `discard_pending` that makes the producer purge its `pending` marker queue before enqueueing the `Boundary`, so stale `Fault`/`End` markers never emit for the abandoned generation. `seek` keeps its current signature and default play-through semantics.
3. **`pump()`/`PumpOutcome` contract (doc-only).** Document that one `pump()` call drains all queued commands then performs at most one unit of work (≤1 item per call), that `Backpressured` means work exists but a ring has no slot (retry, not completion), and that boundary/fault/end markers may remain in the producer `pending` queue under backpressure — a driver must iterate `pump()` until `Idle`/`Ended` rather than assume a call drains the pipeline.
4. **`PrefetchHandle::reopen` direct reopen-to-seek-target (feature).** Additive `reopen_at(source, open, first)` — or the name the design fixes — that fuses reopen + seek into one command under one generation: the producer rebuilds the `SourceReader`, seeks to `first`, and emits `Boundary{first_frame: reader.position().0}` reporting the honest landing; post-reopen seek failure mirrors the Seek fault path (`Fault{Unseekable}` + `Boundary` at the unchanged position). `reopen` keeps its signature and delegates through the same command path.

## Why Now

Synvelis E2E-L1 (Local WAV Import → Verified Source → Track/Clip → Generic Graph Playback) is the first real consumer of the #197 contract. It satisfied sample-exact re-anchor only via quiesce-drain→seek, and two contract points (`expected_first`, `pump` iteration) were readable only from tests, not API docs. These are additive doc/feature improvements — no compatibility breakage — recorded now while the real-usage evidence is fresh, per the principle that Ruviform-side gaps must not be hidden by app-side workarounds.

## Users and Value

- RT playback consumers (Synvelis today; any engine driving `PrefetchConsumer`): sample-exact re-anchor becomes one bounded call instead of a drain choreography; contract semantics are readable from the API surface alone.
- Non-RT driver authors (manual `pump()` loops, tests, offline tooling): the pump iteration contract is explicit — no misreading `Backpressured` as drained.
- Mid-stream source replacement flows (format/stream switches): one-command reopen-at-position instead of reopen→seek with an intermediate position-0 boundary.

## Scope

### In Scope

- Doc-contract rewrites in `crates/ruviform-media-reader` (rustdoc on `fill`/`FillReport`/`pump`/`PumpOutcome`, module Contract section, crate README) for candidates 1 and 3.
- `PrefetchConsumer::discard_pending` + `DiscardReport` (candidate 2a) — bounded, RT-suitable, `&RtContext`-gated.
- `PrefetchHandle` additive seek variant carrying `discard_pending` → producer `pending` purge (candidate 2b); `PrefetchCmd::Seek` gains a private flag field.
- `PrefetchHandle::reopen_at` + `PrefetchCmd::Reopen` private `first: Option<u64>` field (candidate 4); `reopen` delegates through the shared command path.
- Contract tests for the two features (seek-discard semantics incl. stale-marker suppression, drain bound + Boundary adoption, reopen-at-position landed reporting) per the test-spec; RT allocation-freedom coverage for `discard_pending`.
- Doc chain for this issue (discovery → spec → ADR → design → plan → test-spec → tasks → impl record) per `doc-driven-dev-graph`.

### Out of Scope

- Any signature/behavior change to `fill`, `seek`, `reopen`, `pump`, `step`, `PrefetchItem` variants, or `PrefetchTelemetry` fields — additive only.
- Consumer-visible generation pre-adoption, a `PrefetchItem::Discard` variant, or a shared requested-generation atomic (evaluated and declined — heavier machinery for marginal gain over the drain assist).
- True zero-play-through of items already inside the data ring at seek time (impossible without consumer-visible pre-Boundary generation; the drain assist is the bounded mechanism).
- Engine-frame↔source-frame mapping changes, Synvelis/AudioLens app-side code, new crates, backend (`Demuxer`/`Decoder`) changes.
- Parameter or field renames (`expected_first`, `mismatch`) — docs fix the confusion; renames are source-visible churn without contract value.

## Requirements

- **Additive compatibility**: every shipped change keeps existing public signatures and observable default behavior; `#[non_exhaustive]` conventions per `docs/adr/public-enum-non-exhaustive-policy.md` apply to any new public type.
- **RT contract**: `discard_pending` is RT-suitable — no allocation, no locks, no blocking, no OS I/O; its pop loop is bounded by data-ring capacity plus the in-flight block; blocks always return to the free ring (pool-size == free-capacity invariant holds).
- **Non-RT placement**: producer-side pending purge and reopen-at-position run on the producer (non-RT) side only; decode/cache paths stay non-RT.
- **Genericity**: contract text and semantics stay media-generic — no Synvelis/AudioLens-specific semantics, no engine-frame assumptions in the prefetch layer.
- **Honest reporting**: `DiscardReport`/`Boundary`/`FillReport` keep every non-content or dropped span distinguishable; no silent drops beyond the reported counters.
- **Verification**: feature changes land with contract tests first (TDD); `mise run fmt-check`, `check`, `lint`, `test` green.

## Acceptance Criteria

- [ ] AC1: A reader can derive `expected_first`'s contract — asserted stream position in source frames compared against `next_frame`; mismatch refuses the read — from `fill`/`FillReport` rustdoc and the module Contract section alone, without reading tests.
- [ ] AC2: After `handle.seek(target)` and a `Boundary` reaches the ring, `consumer.discard_pending()` adopts it (`position()` reports the landed `first_frame`) and subsequent `fill` serves no pre-boundary content; when no `Boundary` has arrived the report says so and nothing new-generation is dropped.
- [ ] AC3: `discard_pending` performs ≤ data-ring-capacity+1 pops, allocates nothing, locks nothing, and returns every dropped block to the free ring (verified with `assert_no_alloc` in the RT test).
- [ ] AC4: The discard-flagged seek variant purges producer `pending`: stale `Fault`/`End` markers queued before the seek never reach the consumer; the `Boundary` still emits with the new generation.
- [ ] AC5: `pump()`/`PumpOutcome` rustdoc and module contract state the ≤1-item bound, command-first order, `Backpressured` retry semantics, and `pending` marker retention — again without needing tests as the source of truth.
- [ ] AC6: `reopen_at` emits exactly one `Boundary` whose `first_frame` is the reader's post-seek `position()` (landed may exceed target per `SeekReceipt` rules); a failed post-reopen seek yields `Fault{Unseekable}` + `Boundary` at the unchanged position; a failed reopen keeps today's `Fault + stream_ended` outcome.
- [ ] AC7: All pre-existing media-reader tests pass unmodified (default semantics preserved).
- [ ] AC8: `mise run fmt-check`, `mise run check`, `mise run lint`, `mise run test` are green.

## Deferred Design Concerns

- Zero-play-through via a consumer-visible requested-generation cell — declined for now: requires new shared state and changes the adoption model; re-engage only if real usage shows the bounded drain is insufficient.
- `PrefetchItem::Discard` marker — declined: redundant with the Boundary mechanism under the drain-assist shape.

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/233 — four candidates, additive constraint, Synvelis E2E-L1 provenance.
- Discovery: `docs/discovery/issue-233-media-reader-prefetch-contract-improvements.md` — pipeline inventory, per-candidate option tables, confirmed mechanics.
- Upstream contract: `docs/specs/issue-197-media-reader-prefetch-rt-consumer.md`, `docs/adr/issue-197-media-reader-boundary-additions.md`, `docs/designs/issue-197-media-reader-prefetch-rt-consumer.md` (refined, not superseded).
- Policy: `docs/adr/realtime-contract-and-verification-policy.md`, `docs/adr/public-enum-non-exhaustive-policy.md`.
