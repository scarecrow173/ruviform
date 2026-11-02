---
id: "DESIGN-034QolOnuF7kf2drxSsvvH"
type: "design"
status: "approved"
title: "Issue #233 prefetch contract improvements — discard_pending drain, seek discard flag, reopen_at structural design"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/233"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/src/lib.rs"
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
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
    - "docs/adr/issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md"
    - "docs/discovery/issue-233-media-reader-prefetch-contract-improvements.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
    - "docs/adr/issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #233 prefetch contract improvements — discard_pending drain, seek discard flag, reopen_at structural design

## Overview

Four additive improvements on the #197 `ruviform-media-reader` prefetch contract. Two are doc-only (`fill`'s `expected_first` semantics; `pump()`/`PumpOutcome` iteration contract). Two are additive features: a bounded RT-safe `PrefetchConsumer::discard_pending` drain assist plus a `PrefetchHandle` seek variant carrying a `discard_pending` flag that purges the producer's `pending` marker queue, and a fused `PrefetchHandle::reopen_at` (reopen + immediate seek under one generation). All changes live in `crates/ruviform-media-reader` (`src/prefetch.rs`, `src/lib.rs` re-exports, `README.md`, `tests/`); no signature, enum variant, or observable default behavior changes.

## Architecture

### Command surface (`PrefetchCmd`, private)

```rust
enum PrefetchCmd {
    Seek {
        generation: u64,
        first: u64,
        discard_pending: bool,   // NEW — purge producer `pending` before Boundary
    },
    Reopen {
        generation: u64,
        source: Box<dyn MediaSource>,
        open: OpenFn,
        first: Option<u64>,      // NEW — post-reopen seek target
    },
    Quit,
}
```

Private enum, additive fields — free to extend. `seek` sends `discard_pending: false`; a new handle method sends `true`. `reopen` delegates to the shared send path with `first: None`; `reopen_at` sends `Some(first.0)`.

### Handle additions (`PrefetchHandle`, non-RT)

```rust
/// Same contract as `seek`, plus producer-side `pending` purge.
pub fn seek_discard_pending(&mut self, first: SourceFrame) -> Result<()>;

/// Reopen + immediate source-frame seek under one generation.
pub fn reopen_at(
    &mut self,
    source: Box<dyn MediaSource>,
    open: OpenFn,
    first: SourceFrame,
) -> Result<()>;
```

`seek_discard_pending` keeps every `seek` rule: `Fact::Unsupported` seekable → immediate `ErrorKind::Unsupported` without consuming a generation; otherwise bump `next_gen`, send `Seek{discard_pending: true}`. `reopen_at` never pre-checks `seekable` (the `Fact` belongs to the *replacement* source, unknown until `open` runs); a failed post-reopen seek reports through the stream (`Fault{Unseekable}` + `Boundary`), same shape as a rejected `seek`.

### Producer changes (`PrefetchProducer`, non-RT)

`exec_cmd` gains two behaviors, both bounded:

- `Seek{discard_pending: true}` → `self.pending.clear()` *before* enqueueing the `Boundary`. `pending` holds only markers (blocks never enter it — block backpressure re-parks into `parked`), so clearing it removes exactly the stale `Fault`/`End`/superseded-`Boundary` markers of abandoned generations. Clearing is O(capacity-of-a-few-markers) — bounded by construction.
- `Reopen{first: Some(f)}` success → `reader.seek_source_frame(SourceFrame(f))`; on success `Boundary{first_frame: reader.position().0}` (honest landed, identical to Seek); on failure `Fault{kind: Unseekable, first_frame: f, frames: 0}` then `Boundary{first_frame: reader.position().0}` (position 0 after reopen reset — identical to the rejected-seek path). `Reopen{first: None}` is byte-identical to today's behavior.

### Consumer addition (`PrefetchConsumer`, RT)

```rust
/// Bounded drain of the in-flight block and the data ring —
/// applies the stream's sequencing rules without producing output.
pub fn discard_pending(&mut self, _ctx: &RtContext) -> DiscardReport;
```

```rust
/// Outcome of `PrefetchConsumer::discard_pending`.
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiscardReport {
    /// Data-ring items popped during the drain.
    pub items: usize,
    /// Pool blocks dropped back to the free ring
    /// (includes the in-flight `current` block when abandoned).
    pub blocks: usize,
    /// A `Boundary` was consumed: generation adopted, `next_frame`
    /// re-anchored to the reported `first_frame`.
    pub boundary: Option<u64>,
}
```

Drain loop semantics — reuse `step()`'s marker rules, minus content output:

1. Return the in-flight `current` block to the free ring (`return_block`), counted in `blocks`, and reset `cursor`. Rationale: `current` is pending stale-position content; abandoning it is the point of the call. If no `Boundary` follows, the resulting position gap surfaces through the normal sequence-gap → fault-span path — honest loss, no silent skip.
2. `try_pop` the data ring, applying the same transitions as `step()`: `Block` → `stale()` counter + `return_block` + `blocks++` (regardless of generation — the drain exists to discard pending content; a post-`Boundary` block is only reachable when it precedes the consumed `Boundary` in ring order, which cannot happen — anything popped *before* the first `Boundary` is pre-re-anchor by definition); `Boundary` → adopt generation + `next_frame = first_frame` + clear `fault_remaining`/`ended`, record `boundary = Some(first_frame)`, **stop** — items behind the newest `Boundary` are new-generation content and stay queued; `Fault`/`End` → same generation/state rules as `step()` (fault_events counter, `ended` flag), no faulted-frame accounting into a report.
3. Terminate on ring-empty or on the first consumed `Boundary`. Bound: `current` (≤1) + ring capacity (`block_count + MARKER_SLACK`) pops — O(1) worst case for fixed config.
4. Return `DiscardReport`; publish counters to telemetry through the same `stale()` path (drained items are stale drops by definition).

RT contract: `&RtContext`-gated like `fill`; no allocation, no locks, no blocking, no OS I/O; bounded by ring capacity + 1.

### Doc-only changes (candidates 1 and 3)

- `fill` rustdoc: `expected_first` defined as the caller's asserted stream position — the source frame the caller believes `next_frame` equals; it is compared against the stream position, never against an engine/output frame counter. `FillReport.mismatch` doc gains "the caller's position assertion disagreed with the stream's; nothing consumed — re-anchor via `seek` or drop the call".
- Module Contract section: add the position-assertion wording and the pump iteration rule.
- `pump`/`PumpOutcome` rustdoc: one call drains all queued commands then performs ≤1 item of sequenced work; `Backpressured` = work exists but a ring has no slot — iterate, not done; markers may wait in producer `pending`; a manual driver iterates until `Idle`/`Ended`.
- `README.md`: same contract paragraphs at crate level + `discard_pending`/`seek_discard_pending`/`reopen_at` API entries.

## Data and Control Flow

Sample-exact re-anchor (candidate 2):

```text
handle.seek_discard_pending(t)          # gen+1, Seek{discard:true} queued
  producer exec: gen=t-gen, pending.clear(), reader.seek(t),
                 pending.push(Boundary{gen, landed})
consumer.discard_pending(ctx)           # RT, bounded
  drop current → free ring
  pop ring: stale blocks → free ring; stale markers → counters
  on Boundary{gen, landed}: adopt gen, next_frame=landed → stop
  → DiscardReport{items, blocks, boundary: Some(landed)}
consumer.fill(dst, landed, ctx)         # resumes sample-exact at landed
```

Mid-stream source replacement (candidate 4):

```text
handle.reopen_at(src, open, t)          # gen+1, Reopen{first:Some(t)}
  producer exec: open(src) → reader.reopen(demuxer) → position 0
                 reader.seek_source_frame(t) → Boundary{gen, landed}
                 (seek fail → Fault{Unseekable,t,0} + Boundary{gen,0})
consumer converges on Boundary as today
```

## Non-Functional Constraints

- RT: `discard_pending` — no alloc/lock/block/I/O; bound = data-ring capacity + 1 pop; `assert_no_alloc` coverage in the RT test lane. `free_tx` push is infallible by the pool-size == free-capacity invariant.
- Non-RT: `pending.clear()`, post-reopen `seek_source_frame` — producer side only; decode/cache remains non-RT.
- Additivity: no public signature/variant/field changes; `DiscardReport` is a new `#[non_exhaustive]` public type per `public-enum-non-exhaustive-policy`.
- Honesty: drained items count as `stale_drops`; `boundary: Option<u64>` reports adoption instead of a boolean so the landed frame is observable without a second read.

## Risks and Trade-offs

- Drain before `Boundary` arrival drops the stale backlog but cannot see the not-yet-pushed `Boundary` — residual: fresh items pushed post-drain still queue normally; the `boundary: None` report tells the caller to call again or converge on telemetry. Same async contract as today; documented.
- Dropping `current` without a subsequent `Boundary` produces a sequence-gap → fault span — caller-misuse path, documented as "call after seek/reopen".
- `seek_discard_pending` name: the flag scopes producer `pending` AND conceptually pairs with the consumer drain; alternatives (`seek_with_options`, `seek_and_discard`) were weighed — `seek_discard_pending` names the observable behavior and matches the issue's term.
- `reopen_at` cannot fail fast on `Fact::Unsupported` — the replacement source's `seekable` is only known after `open`; the Fault+Boundary path keeps the failure honest and the stream re-anchored.

## Deferred Design Concerns

- Consumer-visible requested-generation cell / `PrefetchItem::Discard` marker for true zero-play-through — declined (ADR option B); re-engage only if real usage shows the bounded drain insufficient.

## Verification Notes

- Contract tests (`tests/prefetch.rs`): seek_discard_pending purges pending markers; discard_pending drains backlog and adopts Boundary; reopen_at lands at seeked position; reopen_at seek-failure → Fault{Unseekable}+Boundary at 0; reopen unchanged (None path).
- RT test (`tests/rt_fill.rs`): `rt_`-prefixed — `discard_pending` under `assert_no_alloc`, bounded pops, blocks returned.
- `mise run fmt-check`/`check`/`lint`/`test` green.

## Open Questions

- None blocking; naming (`seek_discard_pending`, `reopen_at`, `DiscardReport.boundary: Option<u64>`) ratified in this design.
