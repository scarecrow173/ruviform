---
id: "DISC-034Qohljm0FMrizvY86YLe"
type: "discovery"
status: "resolved"
title: "Issue #233 media-reader Prefetch/Seek/Pump contract doc & feature improvement candidates — inventory, option analysis"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/233"
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
    - "docs/designs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/impl/ir/issue-197-media-reader-implementation-record.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
    - "docs/adr/issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #233 media-reader Prefetch/Seek/Pump contract doc & feature improvement candidates — inventory, option analysis

## Summary

Issue #233 lists four additive improvement candidates on the merged #197 Prefetch/Seek/Pump contract, observed in Synvelis E2E-L1 real usage (Synvelis worked around them with quiesce-drain→seek; no permanent app-side workaround exists). Inventory of `crates/ruviform-media-reader/src/prefetch.rs` confirms candidates 1 and 3 are doc-only contract clarifications, candidate 2 is implementable additively as a bounded RT-safe consumer drain assist plus an optional producer-side pending purge flag, and candidate 4 is implementable additively as a `reopen_at` handle method (reopen + immediate seek fused). Tentative conclusion (confidence: high): all four candidates are additive; none requires a signature change, a `PrefetchItem`/`PrefetchCmd` breaking change, or new shared state — the design phase fixes per-candidate shape and any deferral.

## Exploration Goal

Decide per candidate whether the fix is doc-only or an additive feature, fix the feature shape where a feature is warranted, and confirm no candidate needs compatibility breakage on the #197 contract — so the spec/ADR pair can lock scope before implementation.

## Key Issues and Assumptions

- Pipeline shape (#197, verified against source): `PrefetchHandle` (non-RT control surface) sends `PrefetchCmd::{Seek,Reopen,Quit}` over `std::sync::mpsc` to `PrefetchProducer`; the producer pushes `PrefetchItem::{Block,Boundary,Fault,End}` into the bounded data SPSC ring (capacity `block_count + MARKER_SLACK(8)`); `PrefetchConsumer` (RT, `!Sync`) pops items in `step()`, adopts generation only via `Boundary`, stale-drops mismatched-generation items to the free ring.
- `fill(dst, expected_first, &RtContext)` compares `expected_first.0` against the consumer's `next_frame` stream position; mismatch → `FillReport.mismatch`, nothing consumed, dst zero-filled. The parameter is the caller's *asserted stream position in source frames* — the doc currently calls it "the engine-side contract", which reads like an engine-frame expectation.
- Seek mechanics: `handle.seek` bumps `next_gen` and queues `Seek{generation,first}`; `exec_cmd` adopts the generation, calls `reader.seek_source_frame`, then push_backs `Boundary{generation, first_frame: reader.position().0}` (honest landed position) into the producer `pending` marker queue — **nothing is purged**: data-ring items ahead of the Boundary still play through under the consumer's old generation, and already-queued `pending` markers emit before the Boundary. Only post-Boundary items tagged with the old generation are stale-dropped. Sample-exact re-anchor today requires the caller to quiesce output and drain (issue's "quiesce drain→seek choreography").
- Producer `pending: VecDeque<PrefetchItem>` holds only markers (blocks never enter it — block backpressure re-parks into `parked: Option<PcmBlock>`); it is bounded by construction but markers queued before a seek still emit ahead of the Boundary.
- `pump()`: drains **all** queued commands, then performs ≤1 work unit — pop one `pending` marker (`Pushed`/`Backpressured` on full ring), emit `End` (`Ended`), or fill+push one block (`Pushed`/`Backpressured`). Doc says only "one non-blocking work step" — the ≤1-item bound, the command-first order, and the fact that `Backpressured` means "work exists, retry" (markers can linger in `pending`) are not stated.
- `reopen(source, open)` → `PrefetchCmd::Reopen`; producer runs `open(source)` + `reader.reopen(demuxer)` (position reset to 0 inside `SourceReader::reopen`), then `Boundary{first_frame: 0}` on success or `Fault + stream_ended` on failure. No reopen-at-position variant exists; `reader.seek_source_frame` post-reopen + `Boundary{first_frame: reader.position().0}` is a clean additive path (the Boundary already reports the real landing).
- `PrefetchItem` is `pub` + `#[non_exhaustive]` (ADR `public-enum-non-exhaustive-policy`) — a new variant is additive but still not needed for the chosen shapes; `PrefetchCmd` is private — its variants may gain fields freely.
- `PrefetchTelemetry` is an `Arc`-shared relaxed-atomic struct cloned into producer and consumer; `generation`/`position` atomics exist but are producer/consumer-published — the *requested* generation is not currently published; the handle's `next_gen` is local. Designs that need the consumer to know a pending generation before Boundary arrival would require new shared state — avoided by the drain-assist shape.
- House conventions: RT methods are `&RtContext`-gated where they consume stream content; reports are explicit structs (`FillReport`, `FaultCounts`); doc comments carry **RT contract:** labels; crate README mirrors the public contract; `PumpOutcome`/`PrefetchItem`/`SourceFault` are `#[non_exhaustive]`.

## Alternatives and Comparison

Candidate 1 — `fill` `expected_first` semantics (**doc-only** is the issue's own classification; verified):

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Doc-only: rewrite `fill` + `FillReport.mismatch` + module Contract text + README to define `expected_first` as the asserted stream position (source frames), not an engine-frame value | Zero API change; fixes the actual gap (contract unreadable without tests); house style already documents generation/position/fault invariants in the module head — extend the same pattern | None structural | yes |
| Rename parameter (e.g. `expected_first` → `expected_stream_frame`) | Self-documenting signature | Parameter names are source-visible churn without contract value; the type is already `SourceFrame`; docs fix the confusion | no |

Candidate 2 — seek stale pending play-through (feature candidate; additive only):

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Consumer drain assist `PrefetchConsumer::discard_pending()` — bounded pop loop applying `step()`'s marker rules without writing dst; returns blocks to free ring; report tells whether a `Boundary` was adopted | Single bounded RT-safe call replaces output-producing drain; covers stale ring backlog AND stale markers uniformly (they all flow through the ring); no new cmd/marker/shared state; `!Sync` consumer means no concurrency hazards | Caller still seeks + converges on `Boundary` (same async contract as today) — but drain is one call, not a choreography | yes |
| `Seek{discard_pending:true}` flag → producer clears `pending` before appending `Boundary` | Kills stale marker emission at source | Pending holds only markers — the audio-relevant backlog is the data ring, which the producer cannot touch (consumer pops it); alone it does not fix play-through | yes (paired with drain assist) |
| `PrefetchItem::Discard` marker / shared requested-generation atomic → consumer stale-drops pre-Boundary | True zero-play-through | New shared state or consumer-visible contract change; `PrefetchItem` is public — heavier machinery for marginal gain over drain assist | no |

Candidate 3 — `pump()`/`PumpOutcome` contract (**doc-only**; verified): state ≤1 item per call, command-first order, `Backpressured` = bounded retry not completion, markers may remain in producer `pending`, callers driving manual pump must iterate until `Idle`/`Ended`. No alternative — the behavior is already correct; only the text is thin.

Candidate 4 — `reopen` direct reopen-to-seek-target (feature candidate; additive only; lowest priority):

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Add `PrefetchHandle::reopen_at(source, open, first)` + `PrefetchCmd::Reopen{first: Option<u64>}`; producer seeks post-reopen and reports landed via `Boundary{first_frame: reader.position().0}`; seek failure mirrors the Seek fault path (Fault{Unseekable}+Boundary at unchanged position) | Fuses reopen→seek two-step into one command; Boundary already conveys honest landing; private cmd field is free to extend; `reopen` becomes thin wrapper over shared path | New public method (additive surface); handle cannot pre-check `seekable` on the new source (Fact is per-source) — documented via Fault path | yes |
| `reopen(source, open)` + existing `seek(first)` two-step | No new API | Two commands, two generations, an intermediate Boundary at 0 — the exact choreography the issue wants simplified | no |

## Tentative Conclusions and Hypotheses

- **Confirmed**: candidates 1 and 3 are doc-only contract clarifications — API docs, module contract head, crate README; no signature, type-name, or behavior change.
- **Confirmed**: candidate 2 is implementable additively as (a) `PrefetchConsumer::discard_pending` bounded drain assist returning a small report (`blocks`/`markers`/`boundary`) and (b) `PrefetchHandle` seek variant carrying `discard_pending` → producer `pending.clear()` before Boundary enqueue. Both are optional-but-complementary; the drain assist alone closes the play-through surface.
- **Confirmed**: candidate 4 is implementable additively as `PrefetchHandle::reopen_at` + `PrefetchCmd::Reopen.first: Option<u64>`; consumer unchanged; `reopen` delegates through the same command path.
- **Confirmed**: every change stays inside `crates/ruviform-media-reader` (src + tests + README); RT paths remain allocation-free/non-blocking/bounded (drain loop bound = ring capacity + in-flight block); decode/cache remains non-RT.
- **Hypothesis → verify in impl**: `discard_pending` must also drop the consumer's in-flight `current` block and clear `fault_remaining` when a `Boundary` is adopted, mirroring `step()`'s Boundary handling — without that, a partially consumed block would leak stale frames past the re-anchor.
- **Hypothesis**: `reopen_at` seek-failure semantics should reuse the Seek failure shape (Fault{Unseekable, first}+Boundary at actual position) rather than `stream_ended` — the reopen itself succeeded, only the post-anchor failed.

## Open Questions

- Whether the `discard_pending` handle-side flag is worth shipping alongside the drain assist, or recorded as evaluated-and-declined (pending markers emit rarely; drain covers them anyway). Spec/ADR resolves.
- Naming: `discard_pending` (issue's own term) vs `drain_pending`/`discard_stale` on the consumer; `reopen_at` vs `reopen_to`/`reopen_seeking`. Design fixes names.
- Whether `DiscardReport` should expose the adopted `first_frame` or only `boundary_adopted: bool` (position is already readable via `position()`).

## Promotion Candidates

- [x] spec-doc needed: per-candidate outcome (doc-only for 1/3; feature shape for 2/4), acceptance criteria per candidate, additivity/compat guarantees, RT bounds.
- [x] adr-doc needed: (1) drain assist shape vs flag vs marker alternatives; (2) `reopen_at` fused command vs two-step; (3) doc-only classification for 1/3; (4) naming.

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/233 (four candidates, additive constraint, Synvelis E2E-L1 provenance, low priority)
- `crates/ruviform-media-reader/src/prefetch.rs` — `PrefetchCmd`, `PrefetchProducer::{pump,exec_cmd}`, `pending`/`parked`/`data_tx`/`free_rx`, `PrefetchConsumer::{fill,step}`, `PrefetchHandle::{seek,reopen}`, `PrefetchTelemetry`, `MARKER_SLACK` (read)
- `crates/ruviform-media-reader/src/reader.rs` — `SourceReader::reopen` resets `position = 0`; `seek_source_frame`/`SeekReceipt` landed-vs-requested semantics (read)
- `crates/ruviform-media-reader/tests/{prefetch,rt_fill}.rs` — existing contract tests incl. play-through coverage (read)
- `docs/specs|adr|designs/...issue-197...` — merged contract this work extends additively
- Synvelis `docs/impl/ir/l1-e2e-local-wav-verified-playback.md` Known Limitations / Follow-ups (issue-referenced canonical consumer record)

## Evidence

- 2026-11-02 — `exec_cmd` source confirmed: `Seek` pushes `Boundary` to `pending` back with `first_frame: reader.position().0`; `Reopen` pushes `Boundary{first_frame: 0}` on success, `Fault + stream_ended` on failure; neither purges `pending` nor the data ring.
- 2026-11-02 — `step()` source confirmed: consumer adopts generation only via `Boundary`; old-generation `Block`/`Fault`/`End` items stale-drop through `stale()` + `return_block`; `Boundary` clears `fault_remaining` and `ended` and sets `next_frame`.
- 2026-11-02 — `pump()` source confirmed: drains all `cmd_rx` commands, then ≤1 item: one `pending` marker, or `End`, or one `fill_block`; `Backpressured` re-queues marker to `pending` front or re-parks block.
- 2026-11-02 — `fill()` source confirmed: `expected_first.0 != next_frame` → `mismatch`, zero-fill, nothing consumed; fault spans cover positions like blocks.
