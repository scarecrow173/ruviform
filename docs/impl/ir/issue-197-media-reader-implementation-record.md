---
id: "IMPL-8FuQb4NPvHVC136oYJUME7"
type: "impl"
status: "completed"
title: "Issue #197 Generic Media Reader implementation record"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/197"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-media-reader/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-media-reader/README.md"
      - type: "file"
        path: "crates/ruviform-media-reader/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/src/mapping.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/src/metadata.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/src/reader.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/reader.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/rt_fill.rs"
      - type: "file"
        path: "docs/discovery/issue-197-media-reader-prefetch-rt-consumer.md"
      - type: "file"
        path: "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
      - type: "file"
        path: "docs/adr/issue-197-media-reader-boundary-additions.md"
      - type: "file"
        path: "docs/designs/issue-197-media-reader-prefetch-rt-consumer.md"
      - type: "file"
        path: "docs/plans/issue-197-media-reader-prefetch-rt-consumer.md"
      - type: "file"
        path: "docs/test-specs/issue-197-media-reader-prefetch-rt-consumer.md"
      - type: "file"
        path: "docs/tasks/task-197-t1-demuxer-seekable-make-decoder.md"
      - type: "file"
        path: "docs/tasks/task-197-t2-media-reader-crate.md"
      - type: "file"
        path: "docs/tasks/task-197-t3-tests-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-media-io/src/decode.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/demux.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/demuxer.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/tests/wav_decode_poc.rs"
      - type: "file"
        path: "crates/ruviform-io-avi/src/demuxer.rs"
      - type: "file"
        path: "crates/ruviform-io-avi/tests/avi.rs"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../plans/issue-197-media-reader-prefetch-rt-consumer.md"
    - "../../tasks/task-197-t1-demuxer-seekable-make-decoder.md"
    - "../../tasks/task-197-t2-media-reader-crate.md"
    - "../../tasks/task-197-t3-tests-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../discovery/issue-197-media-reader-prefetch-rt-consumer.md"
    - "../../specs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "../../adr/issue-197-media-reader-boundary-additions.md"
    - "../../designs/issue-197-media-reader-prefetch-rt-consumer.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../../test-specs/issue-197-media-reader-prefetch-rt-consumer.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #197 Generic Media Reader implementation record

## Summary

Implemented the Issue #197 generic media read path. `ruviform-media-io`
gains the additive boundary `DecodeMode` + `Demuxer::seekable` +
`Demuxer::make_decoder(stream, mode)` port; the new
`ruviform-media-reader` crate supplies a source-frame-addressed
`SourceReader` over the `Demuxer`/`Decoder` pair and a bounded
non-RT→RT prefetch pipeline (fixed `PcmBlock` pool + two SPSC rings)
whose `PrefetchConsumer::fill` is allocation-free, lock-free, and I/O-free.
Canonical (`f32` planar) and native (`f64`/native-scale observation)
decode modes are explicit, separate representations — the f32 playback
path never forces itself onto high-fidelity observation.

## Context

Follows the approved document chain: discovery
`docs/discovery/issue-197-media-reader-prefetch-rt-consumer.md`, spec
`docs/specs/issue-197-media-reader-prefetch-rt-consumer.md`, ADR
`docs/adr/issue-197-media-reader-boundary-additions.md`, design
`docs/designs/issue-197-media-reader-prefetch-rt-consumer.md`, plan
`docs/plans/issue-197-media-reader-prefetch-rt-consumer.md`, test-spec
`docs/test-specs/issue-197-media-reader-prefetch-rt-consumer.md`, tasks
T1…T3. Boundary additions are strictly additive: new defaulted trait
methods, a new enum, one new re-export — no existing signature changed.

## Implementation

### Changes

- `crates/ruviform-media-io/src/decode.rs` + `lib.rs` — `DecodeMode`
  (`Canonical` playback / `Native` observation, `#[non_exhaustive]`) and
  its re-export (T1).
- `crates/ruviform-media-io/src/demux.rs` — defaulted
  `Demuxer::seekable() -> Fact<()>` (`NotDeclared` baseline, honest
  capability fact) and defaulted `Demuxer::make_decoder(id, mode)`
  (`ErrorKind::Unsupported` baseline) (T1).
- `crates/ruviform-io-symphonia` — construction-time source-seekability
  probe captured before ownership transfer into the backend stream;
  `seekable()` reports `Known(())`/`Unsupported`;
  `make_decoder(id, mode)` dispatches to the existing canonical/native
  constructors (T1).
- `crates/ruviform-io-avi` — same port impls; seek declared
  `Unsupported` at construction, matching `seek()`'s error (T1).
- `crates/ruviform-media-reader` — new crate:
  - `mapping.rs` — `SourceFrame`/`EngineFrame`/`RequestedRange` and the
    caller-supplied `FrameMap` (clip offset + placement + rate ratio,
    `i128` exact math, unrepresentable → `None`, never clamped).
  - `metadata.rs` — `SourceMetadata` projection (backend/container/
    stream facts, canonical format, declared `frame_count`, `seekable`,
    decode mode) built by `stream_metadata`.
  - `reader.rs` — `SourceReader`: sequential pump over
    `Box<dyn Demuxer>`/`Box<dyn Decoder>`, pts-anchored positions,
    `read_planar` canonical windows, `next_frame` whole-frame native
    delivery, `seek_source_frame` (demuxer seek → decoder flush →
    decode-discard) reporting the real landing via `SeekReceipt`,
    `reopen` keeping stream id + mode.
  - `prefetch.rs` — bounded pipeline: fixed `PcmBlock` pool, data +
    free SPSC rings, command channel (`Seek`/`Reopen`/`Quit`), marker
    items (`Boundary`/`Fault`/`End`), `PrefetchProducer::pump`
    (manual drive) / `Prefetch::spawn` (worker thread),
    `PrefetchConsumer::fill` (RT: zero-fill fallback declared, mismatch
    detection, fault spans zero-filled+counted, stale-generation drops,
    per-class `FaultCounts`), `PrefetchTelemetry`, and close audits
    (`PrefetchAudit`/`ProducerClose`/`ConsumerClose`) accounting every
    pool block (T2).
- Workspace `Cargo.toml` member registration, `Cargo.lock`,
  `crates/README.md` index row.

### Decisions During Implementation

- **Seek is an ordered boundary, not a ring purge.** Blocks already in
  flight when a `Seek` command is issued belong to the old generation
  and play in order before the `Boundary` item flips the consumer's
  generation — the SPSC topology cannot retract published items, and a
  consumer-side "drop until boundary" would need a second channel. The
  contract is honest: `position()` always names the source position of
  delivered content, and `generation` separates lifetimes. A caller
  needing hard truncation drops and rebuilds the pipeline.
- **`End` is emitted once per generation**; the consumer latches `ended`
  and reports `eof` on every subsequent empty fill — EOF is a state,
  not an event.
- **Fault spans are zero-filled and counted, never silent.** `Fault`
  markers carry `SourceFault` classes (Truncated/Corrupt/Io/
  InvalidSample/Unsupported/Unseekable); block-level fault envelopes
  split copies so `written`/`faulted`/`underrun`/`eof`/`mismatch` stay
  distinguishable in every `FillReport`.
- **A parked block survives EOF/backpressure** so no pool block is ever
  dropped mid-flight; `close` audits reclaim `block_count` blocks on
  both producer and consumer sides.

## Related Experiments

### Adopted

Reuses `ruviform_rt` SPSC `channel`/`Producer`/`Consumer`,
`RtContext::mint`, and the `assert_no_alloc` dev-dependency pattern —
no new primitives were needed.

### Rejected

- *MPSC/multi-producer rings* — rejected for single-owner simplicity
  (one producer task per source; see ADR alternatives).
- *Consumer-side hard flush on seek* — rejected: unreachable in this
  topology without a second control path to the RT thread; the ordered
  boundary keeps the contract truthful.

## Validation

- `mise run fmt-check` — pass.
- `mise run check` (`cargo check --workspace --all-targets`) — pass.
- `mise run lint` (`cargo clippy --workspace --all-targets -- -D warnings`) — pass.
- `mise run test` (`cargo test --workspace`) — pass; media-reader suite
  29/29 green (3 lib + 13 prefetch + 11 reader + 2 rt_fill), symphonia
  boundary tests 3/3, AVI boundary tests 2/2, all legacy suites
  unchanged and green (old `Demuxer`/`Decoder` consumer regression).
- `mise run test-rt` (`cargo test --workspace rt_`) — pass;
  `rt_consumer_fill_no_alloc` + `rt_consumer_underrun_fill_no_alloc`
  prove zero allocation in the RT fill path via `assert_no_alloc`.
- `mise run verify-ci` — pass (CI profile, post-merge; fmt-check →
  check-ci → lint-ci → test-ci all green, exit 0).
- `doc-status` audit over `docs/` — zero findings on all Issue #197
  artifacts (spec/adr/design/plan/test-spec/3 tasks/impl record;
  the 81 repo-wide blocking findings are pre-existing legacy-front-matter
  issues on unrelated main docs, outside this change's scope).
- `doc_maintenance apply` — managed indexes regenerated;
  `plan --type all` re-check reports 0 remaining actions and no
  findings on Issue #197 artifacts.
- `route_graph` — chain walked to `exit-audit-to-complete` with
  `implementation-verified` + `followup-terminal` + `exit-audit-pass`
  signals; zero blockers.
- Acceptance coverage: RD-01 (mono/stereo WAV, source-frame 0/mid-seek/
  tail/EOF, exact f32 equality via dyadic S16→f32 fixtures), RD-02
  (ClipOffset/Placement `FrameMap`, two-seek generation isolation,
  post-boundary landing at the exact target), RD-03 (variable callback
  sizes 33…1000, two simultaneous sources, bounded pool, underrun
  classified with telemetry), RD-04 (fault classes — truncated
  `UnexpectedEos`, corrupt `InvalidData`/`Unsupported`, unseekable,
  mismatch, per-class `FaultCounts`), RD-05 (cache-full backpressure,
  close/reopen under new generation, `reclaimed == block_count` audits,
  spawned-worker close with `worker_joined`, zero `dropped`), RD-06
  (regression — every pre-existing test passes unmodified), RD-07
  (fixtures are in-test synthesized WAV; env Windows 11 x64, rustc
  1.99.0, tolerance exact via dyadic fixtures).

## Consumer Impact

- **synvelis**: no impact (additive API; no existing calls changed).
- **audio-lens**: verified by code review — `al-media` calls inherent
  `SymphoniaDemuxer::make_decoder(id)`/`make_native_decoder(id)` and
  uses `&mut dyn Demuxer`; all preserved. New trait methods are
  defaulted and object-safe; `DecodeMode`/`seekable` are opt-in.
  A local-path `cargo check` of audio-lens against this worktree is
  **unverified** (consumer repo is read-only from this dispatch; git-rev
  pin unchanged). Precision/timestamp regressions: none — decode paths
  untouched, only dispatch wrappers added (RD-08/09 verified; RD-10
  compile-equivalence marked `unverified`).

## Risks

- Ordered-boundary seek means queued pre-seek content plays before the
  generation flip — callers requiring hard truncation must rebuild the
  pipeline (documented in crate docs).
- `stale_drops` is defensive depth: with strictly ordered emission,
  same-generation items never arrive post-Boundary; the counter exists
  for future multi-marker paths.
- `run()`'s 1 ms `recv_timeout` bounds worker wake latency to ~1 ms per
  pump step — adequate for prefetch depth ≥ a few callbacks; tunable via
  `PrefetchConfig` capacity, not a spin loop.

## Follow-ups

- Symvelis-side consumer wiring (issues #17/#18/#21/#24/#28) can now
  consume `Prefetch` + `FrameMap`; AudioLens (#127) can adopt
  `SourceReader`/`DecodeMode::Native` at its own pace — adoption is
  opt-in, not required by this boundary.
- AVI seek support remains `Unsupported` by construction — a future
  `movi` index reader would upgrade the declared `Fact`.
