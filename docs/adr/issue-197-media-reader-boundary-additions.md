---
id: "ADR-I20b5KtQfHowI9YL4J5I0u"
type: "adr"
status: "accepted"
title: "Issue #197 Media Reader / Prefetch — crate placement, boundary additions, prefetch ownership/generation contract"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/197"
    - "docs/discovery/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
  changes:
    added:
      - type: "dir"
        path: "crates/ruviform-media-reader/"
    modified:
      - type: "file"
        path: "crates/ruviform-media-io/src/demux.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/decode.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/demuxer.rs"
      - type: "file"
        path: "crates/ruviform-io-avi/src/demuxer.rs"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #197 Media Reader / Prefetch — crate placement, boundary additions, prefetch ownership/generation contract

## Context and Problem Statement

Issue #197 (M5) requires a generic Reader/Decoder/Prefetch boundary with an RT playback consumer. Three open design questions: (a) where the reader/prefetch code lives given `ruviform-media-io` declares its whole crate non-RT while a prefetch pipeline has an RT endpoint; (b) how a backend-agnostic reader constructs a `Decoder` and learns seek capability today — `make_decoder`/`make_native_decoder` are backend-concrete methods and `is_seekable` is trapped inside each backend's `MediaSourceStream`; (c) what ownership/generation/fault contract a bounded prefetch needs so RT fills never alloc/lock/I-O and stale data can never leak across a seek.

## Decision Drivers

- media-io crate contract: "the *whole crate* is non-RT" — placing an RT consumer inside would violate it.
- AudioLens precedent: its `RuvDemux` enum hand-dispatches `make_native_decoder` per backend — proof the boundary lacks a construction port and that every consumer reinvents it.
- Scope 1 requires seekability as a declared capability fact before any seek attempt.
- Typed runtime precedent (#195): `PreparedPlan.generation` + `SharedStatus` fault flags already implement generation-safe activation — prefetch should mirror that proven pattern.
- RD-03: SPSC/pool single-owner semantics must be *provable*, no bounds violations, no RT alloc/lock/I-O.
- Scope 4: every non-content span distinguishable — corrupt/truncated/EOF/underrun/invalid-sample/unsupported-layout/unseekable — never collapsed into "verified content".
- Additive migration: no existing signature/behavior changes; third-party `Demuxer` impls must keep compiling.

## Considered Options

**(1) Crate placement**

- A1: new `ruviform-media-reader` crate (Domain foundations; deps media-io/media-core/audio-core/ruviform-rt). Mixed-RT with per-API labels — precedent `ruviform-audio-core`.
- A2: inside `ruviform-media-io`. Rejected: contradicts the documented whole-crate non-RT contract.
- A3: inside `ruviform-audio` engine. Rejected: AudioLens (equal consumer, RD-08–10) would inherit an engine dependency; a reader is media foundation.
- A4: inside `ruviform-rt`. Rejected: ruviform-rt holds contract defs + primitives, performs no I/O.

**(2) Decoder construction + seek capability**

- B1: additive `Demuxer::make_decoder(&self, StreamId, DecodeMode) -> Result<Box<dyn Decoder>>` (default `Err(Unsupported)`) and `Demuxer::seekable(&self) -> Fact<()>` (default `Fact::NotDeclared`).
- B2: caller-supplied factories/closures per backend. Rejected: duplicates AudioLens's enum dispatch in every consumer — same policy in several places.
- B3: assume seekable, discover at `seek` failure. Rejected: scope 1 requires explicit capability metadata.

**(3) Prefetch structure**

- C1: bounded `PcmBlock` pool + two SPSC rings (data + free-return) + `PrefetchItem` markers (`Block`/`Boundary`/`Fault`/`End`), every item generation-tagged; control via non-RT `std::sync::mpsc` channel (Seek/Quit/Reopen); telemetry via `PrefetchTelemetry` shared atomics.
- C2: shared lock-free frame-indexed cache. Rejected: random-access index machinery exceeds sequential playback needs; single-owner proof harder.
- C3: one SPSC of `AudioBuffer` frames. Rejected: per-frame alloc handoff; no inline fault/boundary sequencing.

**(4) RT consumer fill contract**

- D1: `fill(&mut [&mut [f32]], expected_first: SourceFrame, &RtContext) -> FillReport` — strict position check; `written`/`underrun`/`faulted`/`eof`/`mismatch` distinct; zero-fill declared bounded fallback, always reported.
- D2: sequential `fill(dst, frames)` only. Rejected: cannot detect caller position bugs; stale reads served silently (violates scope-5 stale-plan rejection).

## Decision Outcome

Chosen: **A1 + B1 + C1 + D1**.

- New crate `ruviform-media-reader`; per-API `RT contract` labels (construction/pump/handle = non-RT; `PrefetchConsumer::fill`/`close` = RT-suitable under documented preconditions).
- `ruviform-media-io` gains `DecodeMode { Canonical, Native }`, `Demuxer::make_decoder`, `Demuxer::seekable` — all defaulted, strictly additive. Symphonia + AVI implement real dispatches; symphonia plumbs its construction-time probe into `seekable()`; AVI reports `Fact::Unsupported`.
- Prefetch: `PrefetchConfig { block_frames, block_count, channels, sample_rate }`; pool sized so free-ring capacity equals `block_count` — the free push is therefore infallible and the consumer never deallocates on RT. `PrefetchItem` is the single sequencing unit: every item advances `next_frame` by a known span, so gaps are structural, not guessed. Generation `u64` bumps on every seek/reset/reopen/close; items carrying stale generations are dropped to the free ring (proven by test).
- Faults: `SourceFault` markers are in-stream sequence elements advancing `next_frame`; invalid samples (non-finite decoded PCM) are producer-detected, zeroed in-block, and fault-marked. Decode/demux errors are stream-fatal → `Fault` + `End`.
- `fill` semantics: `expected_first != next_frame` → `mismatch`, nothing consumed. Past-`End` region → `eof`, remaining dst zeroed, counted as written-silence-by-exhaustion not underrun. Empty data ring while content expected → `underrun` count + zero-fill. All counters also published to `PrefetchTelemetry` atomics for non-RT observability.

### Positive Consequences

- One authoritative decoder-construction point; AudioLens-style enum dispatch becomes optional, not required (its existing calls still work — additive).
- Seek capability is a `Fact` — AVI `Unsupported`, symphonia probed, unknown backends `NotDeclared`.
- Generation+item-sequencing makes stale-drop provable and leak audit trivial (blocks reclaimed = pool size on close).
- Fault-vs-silence never collapses: `FillReport` separates `written`/`underrun`/`faulted`/`eof`.

### Negative Consequences

- Two new defaulted trait methods widen `Demuxer` (mitigated: defaults, `#[non_exhaustive]` on new enums where future growth is plausible).
- Sequential-stream prefetch cannot rewind in-place — backward jumps require `seek` (generation bump), matching the engine's caller-supplied mapping model.
- mpsc-based control adds a std dependency on the non-RT side only.

## Pros and Cons of the Options

Covered per-option above; A1/B1/C1/D1 is the only combination preserving the media-io non-RT contract, avoiding consumer-side policy duplication, giving provable ownership, and keeping every loss class observable.

## Links

- Issue: https://github.com/scarecrow173/ruviform/issues/197
- Discovery: `docs/discovery/issue-197-media-reader-prefetch-rt-consumer.md`
- Spec: `docs/specs/issue-197-media-reader-prefetch-rt-consumer.md`
- Prior art: `docs/adr/media-io-backend-abstraction.md`, `docs/adr/realtime-contract-and-verification-policy.md`, typed runtime generation activation (#195)
