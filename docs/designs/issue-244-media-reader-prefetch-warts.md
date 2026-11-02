---
id: "DESIGN-034QxcnAfFWswZmwSKPbKH"
type: "design"
status: "approved"
title: "Issue #244 prefetch contract wart fixes — boundary-first emission, dead-generation sequence, End commit-once, seeded-fault coverage"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/244"
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
    - "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
    - "docs/adr/issue-244-prefetch-contract-warts-boundary-first-fault-emission-dead-generation-.md"
    - "docs/discovery/issue-244-media-reader-prefetch-contract-warts.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/designs/issue-233-media-reader-prefetch-improvements.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
    - "docs/adr/issue-244-prefetch-contract-warts-boundary-first-fault-emission-dead-generation-.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #244 prefetch contract wart fixes — boundary-first emission, dead-generation sequence, End commit-once, seeded-fault coverage

## Overview

Four residual wart fixes on the #197/#233 prefetch contract inside `crates/ruviform-media-reader`, all producer-side (`PrefetchProducer::exec_cmd`/`pump`): boundary-first fault emission on rejected `seek`/`seek_discard_pending` and `reopen_at` post-reopen seek failure; a `Boundary` → `Fault` → `End` dead-generation sequence on `open`/`reader.reopen` failure; `end_emitted` commit-once so a requeued `End` is never synthesized twice; and seeded-fault contract-test coverage for the `discard_pending` fault arm, `End`-then-`Boundary` drain, and every amended path. Per the ADR (Option A), the P2 corrections are an unconditional contract-semantics amendment with a recorded migration note — no opt-in, no new API, no consumer-logic changes.

## Architecture

### Emission invariant

The producer emits exactly one `Boundary` as the **first** item of every generation; all subsequent generation-tagged items follow it. This holds after the amendment for every arm:

| `exec_cmd` arm | Emitted sequence (into `pending` → data ring) |
| --- | --- |
| `Seek` success (unchanged) | `Boundary{gen, landed}` |
| `Seek`/`seek_discard_pending` rejected | `Boundary{gen, position}` → `Fault{gen, Unseekable, target, 0}` *(was `Fault` → `Boundary`)* |
| `Reopen` success, `first: None` (unchanged) | `Boundary{gen, 0}` |
| `Reopen` success, `first: Some`, seek ok (unchanged) | `Boundary{gen, landed}` |
| `Reopen` success, `first: Some`, seek rejected | `Boundary{gen, position}` → `Fault{gen, Unseekable, target, 0}` *(reordered)* |
| `Reopen` `Err` (`open`/`reader.reopen` fails) | `Boundary{gen, 0}` → `Fault{gen, of_kind(error), 0, 0}` → `stream_ended` ⇒ `End{gen}` *(adds `Boundary`)* |
| `fill_block` fatal decode (unchanged) | `Fault{gen, of_kind, position, 0}` → `End{gen}` — already post-adoption for the current generation |

Consequences under boundary-driven adoption (consumer `step`/`discard_pending` unchanged):

- A `Fault{gen=N}` is always observed post-adoption → counted in `fault_events` and surfaced via `Step::Faulted` → `FillReport.faults`; `stale_drops` counts only genuinely stale items.
- Reopen failure becomes a well-formed terminal generation: consumer adopts `N` (`position`→0), counts the fault, consumes `End` → `ended`/`eof`; telemetry `generation`/`position` converge instead of starving at `N-1`.
- The consumer-side gen-mismatch `Fault` arm becomes unreachable through honest producer sequences (FIFO ordering makes the fault always follow its boundary); it stays as defensive sequencing — not removal-worthy.

### `end_emitted` commit-once (`pump` step 2)

```rust
if self.stream_ended && !self.end_emitted {
    let end = PrefetchItem::End { generation: self.generation };
    // Committed once synthesized: a full ring requeues the marker
    // into `pending`, where it still emits exactly once — it must
    // not be synthesized again on the next pump.
    self.end_emitted = true;
    match self.data_tx.try_push(end) {
        Ok(()) => return Ok(PumpOutcome::Ended),
        Err(full) => {
            self.pending.push_front(full.into_inner());
            return Ok(PumpOutcome::Backpressured);
        }
    }
}
```

The requeued marker still emits via `pending` exactly once; `exec_cmd`'s `end_emitted = false` reset on the next command preserves today's stale-`End`-then-`Boundary` semantics — the flag only suppresses *re-synthesis*.

### Seeded-fault fixture (test-only, `tests/prefetch.rs`)

```rust
/// MediaSource whose read/seek fail once `fail` is armed — the
/// symphonia open-probe runs unarmed (seekable `Fact::known`),
/// then the test arms the fault deterministically.
struct SeededSource { inner: Cursor<Vec<u8>>, fail: Arc<AtomicBool> }
impl Read for SeededSource   { fn read(..)  { if armed { Err } else { inner.read } } }
impl Seek for SeededSource   { fn seek(..)  { if armed { Err } else { inner.seek } } }
```

- Seek-failure arm: build pipeline unarmed (probe passes → `seekable` `Fact::known`), arm, `handle.seek` → `exec_cmd` `Err` → `Boundary` → `Fault{Unseekable}`. First post-open seek fails atomically — no partial-seek position corruption, no call counting.
- Decode-failure arm: pump unarmed content, arm mid-stream → next `fill_block` `read_planar` errors → `Fault{of_kind}` + `stream_ended` → `End` — drives the `discard_pending` matching-gen fault arm and `End`-then-`Boundary` drain.
- Ring-full `End` requeue: `block_count: 1` (data-ring capacity 9) + four failed reopens (8 markers) + one queued block saturate the ring; the synthesized `End` must requeue — duplicate-End path is deterministic, asserted producer-side via `PumpOutcome::Idle` after the requeued `End` pushes.

### Contract-text updates

- Module `## Contract`: add the boundary-first emission invariant (every generation's first item is its `Boundary`; a generation that cannot produce content still emits `Boundary` → `Fault` → `End`) and the `End` commit-once note.
- `reopen_at` rustdoc: replace the two wart-describing paragraphs with the corrected semantics (boundary-first fault; dead-generation sequence on reopen failure).
- `seek`/`seek_discard_pending`/`reopen` rustdoc: failure-path sequences stated.
- README: contract paragraphs updated to the amended semantics.

## Data and Control Flow

Rejected seek (after amendment):

```text
handle.seek(t)                       # gen+1, Seek queued
  producer exec: gen=N, reader.seek_source_frame(t) → Err
                 pending.push(Boundary{N, position})   # old position kept
                 pending.push(Fault{N, Unseekable, t, 0})
consumer step/fill:
  Boundary → adopt N, next_frame=position
  Fault    → fault_events+1 → Step::Faulted(Unseekable) → faults.unseekable+1
```

Failed reopen (after amendment):

```text
handle.reopen(src, open)             # gen+1, Reopen queued
  producer exec: gen=N, open(src)/reader.reopen → Err
                 pending.push(Boundary{N, 0})
                 pending.push(Fault{N, of_kind, 0, 0})
                 stream_ended = true                    # lazy End next pump
consumer step/fill:
  Boundary → adopt N, next_frame=0
  Fault    → fault_events+1 → Step::Faulted(kind)
  End      → ended → Step::Ended → FillReport.eof
```

## Non-Functional Constraints

- RT: no consumer-side changes; `fill`/`discard_pending` bounds and allocation-freedom unchanged.
- Producer side (non-RT): emission reorder and one extra `pending` marker per failed reopen — `pending` bound is unchanged by construction.
- Observable semantics: deliberate amendment per ADR migration note — no opt-in, no dual ordering.
- Honesty: no counter removed; `stale_drops` retains genuine stale drops only.

## Risks and Trade-offs

- Observable counter shift on failure paths — mitigated: migration note in ADR + impl record + PR consumer-impact section; exactly one in-repo test pins the old accounting and is updated.
- Dead-generation `position` anchor `0` vs requested `first` — chosen `0`: the stream never started under `N`, so frame-0 is the honest anchor; the failed target remains visible in `Fault.first_frame`.
- Gen-mismatch `Fault` arm becomes unreachable honestly — kept as defensive sequencing for malformed streams; documented in spec/design rather than removed.
- `SeededSource` fails the *first* armed seek atomically — avoids the partial-seek fidelity question entirely (a partially-seeked backend emitting mislabeled content is a pre-existing concern outside this scope, recorded in the spec's deferred concerns).

## Deferred Design Concerns

- Consumer-side gen-mismatch `Fault` arm removal — not warranted; defensive code costs nothing.
- Post-seek-failure source-position fidelity — pre-existing; out of scope (spec deferred concerns).

## Verification Notes

- `tests/prefetch.rs`: boundary-first fault observability on `seek` (seeded seek failure) and `reopen_at` (`NoSeek` reopen — updated `rd02_reopen_at_seek_failure_faults_then_reanchors`); reopen-failure `Boundary`→`Fault`→`End` adoption + `eof` (failing `open` closure); `discard_pending` matching-gen `Fault` arm + `End`-then-`Boundary` (seeded read failure); single-`End` under ring-full requeue (`PumpOutcome::Idle` assertion).
- `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` green; review-protocol over the diff + ADR decision.

## Open Questions

- None blocking; consumer-impact classification for AudioLens stays `unknown` per Issue #219 convention (enumerated in the ADR migration note).
