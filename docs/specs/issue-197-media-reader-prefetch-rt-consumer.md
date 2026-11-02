---
id: "SPEC-jhXBM8Vy5edyp0fsJ6Nqbb"
type: "spec"
status: "approved"
title: "Issue #197 Generic Media Reader / Decoder・Prefetch・RT Playback Consumer 境界"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/197"
    - "docs/discovery/issue-197-media-reader-prefetch-rt-consumer.md"
  changes:
    added: []
    modified: []
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
  related:
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-197-media-reader-prefetch-rt-consumer.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #197 Generic Media Reader / Decoder・Prefetch・RT Playback Consumer 境界

## What Should Be Built

The generic read side of the Ruviform media foundation: a backend-agnostic `SourceReader` that pulls decoded PCM in **source-frame** order over any `Demuxer`/`Decoder` pair, a `SourceMetadata` capability/metadata projection, a caller-supplied **EngineFrame→SourceFrame** mapping contract, and a bounded prefetch pipeline — non-RT decode into a fixed `PcmBlock` pool, single-owner handoff through two SPSC rings, generation-tagged stream items, and an RT consumer that is no-alloc, no-lock, no-I/O and reports every non-content span (underrun, fault, EOF, position mismatch) instead of fabricating content. Two additive boundary extensions close the construction/capability gaps: `Demuxer::seekable()` and `Demuxer::make_decoder(StreamId, DecodeMode)`.

## Why Now

M5 of the Synvelis migration assigns Ruviform "Generic Source/Decoder/NonRT Prefetch/RT Buffer" while the application keeps SourceId/AssetId/Remote Provider/Project lifecycle. The boundary proves real media opens and decodes (PoC precedent), but no bounded RT playback consumer exists, and two gaps — no trait-level decoder construction, no declared seek capability — would otherwise be re-invented per consumer. Issue #197 is the contract that makes stable RT audio playback a verifiable property rather than an assumption, and it must serve Synvelis (bounded prefetch → RT consumption) and AudioLens (lossless native observation) without forcing the f32 playback representation on either.

## Users and Value

- Synvelis: hands a `MediaSource` + demuxer + clip/placement mapping; gets canonical f32 PCM at RT-consumer latency with explicit underrun/fault evidence (Synvelis #17 Source, #18 Playback Graph, #21 Media/Jobs).
- AudioLens: keeps the `Demuxer`/`Decoder`/`NativeAudioFrame` lossless path untouched; gains trait-level `make_decoder(.., DecodeMode::Native)` and `seekable` facts without changing its working calls (AudioLens #127).
- Future Ruviform engines/adapters: a canonical prefetch→RT-consumer boundary they compose rather than re-design.

## Scope

### In Scope

1. `ruviform-media-io` (additive only): `DecodeMode { Canonical, Native }`; `Demuxer::seekable(&self) -> Fact<()>` (default `Fact::NotDeclared`); `Demuxer::make_decoder(&self, StreamId, DecodeMode) -> Result<Box<dyn Decoder>>` (default `Err(Unsupported)`). No existing signature, behavior, or type changes.
2. `ruviform-io-symphonia`: plumb the construction-time `is_seekable` probe into `SymphoniaDemuxer`; implement both new trait methods (canonical + native dispatch).
3. `ruviform-io-avi`: implement both trait methods — `seekable` reports `Fact::Unsupported` (its `seek` is unimplemented); `make_decoder` dispatches canonical/native.
4. New `ruviform-media-reader` crate (Domain foundations; deps `ruviform-media-io`/`media-core`/`audio-core`/`ruviform-rt` only):
   - `SourceFrame`/`EngineFrame` newtypes, `RequestedRange`, `FrameMap` (caller-supplied clip offset + engine placement + rate ratio — loop/stretch semantics are caller inputs, never reader policy).
   - `SourceMetadata`: `ContainerInfo`, `BackendInfo`, `StreamInfo` (incl. `native_format`, `decodable`, `declared_units`), canonical `AudioFormat`, `frame_count: Fact<u64>` (declared duration → rate), `seekable: Fact<()>`.
   - `SourceReader`: `open(Box<dyn Demuxer>, StreamId, DecodeMode)`, `metadata()`, `position()` (pts-derived, `Estimated` fallback), `next_frame()` (mode-appropriate `Frame`), `read_planar(&mut [&mut [f32]], frames)` (canonical), `seek_source_frame(SourceFrame) -> SeekReceipt` (demuxer seek → decoder flush → decode-discard to target; `landed` reported honestly).
   - Prefetch: `PrefetchConfig` (block_frames/block_count/channels/rate), `PcmBlock` pool, `PrefetchItem` = `Block`/`Boundary`/`Fault`/`End` (each generation-tagged; every item advances `next_frame` deterministically), `PrefetchProducer` (non-RT pump + spawned worker), `PrefetchConsumer::fill` (RT-suitable, `&RtContext`-gated), `PrefetchHandle` (seek/close/reopen via control channel + `PrefetchTelemetry` atomic counters).
   - Fault taxonomy: `SourceFault { Truncated, Corrupt, Io, InvalidSample, Unseekable, UnsupportedLayout }` markers + `FillReport { written, underrun, faulted, eof, mismatch, faults }`; zero-fill is the declared bounded fallback and is always reported — never "verified content".
5. Tests: in-memory WAV fixtures (mono/stereo S16), generation isolation, two-source SPSC + pool single-owner, `rt_*` no-alloc fill via `assert_no_alloc` dev-dep, error/fault taxonomy matrix, leak/retire audit, regression of the direct demux/decode path.

### Out of Scope

- SourceId/AssetId/Remote Provider/Project lifecycle/copy-commit (application responsibility).
- Remote/network sources, non-local providers as implementations (the API must not assume local files, but no non-local provider ships here).
- Resampling/rate conversion, channel remixing, loop/stretch *semantics themselves* (caller mapping inputs only).
- Video streams, subtitle/data streams.
- A `GraphProcessor` adapter node in `ruviform-audio` (composition contract is documented; the node is caller glue).
- Project Asset DB, VCS, remote asset download, record writer, WAV encode/publish (#191 boundary).
- MP4/AAC fixture expansion beyond what the default symphonia build + `mp4`/`aac` features already cover (RD-10 records what is measured vs unverified).

## Acceptance Criteria

- **AC1 (RD-01)** Decoding known mono and stereo PCM WAV fixtures returns correct frames/channels/content at SourceFrame 0, after a mid-stream seek, at the stream tail, and at EOF; EOF is distinguishable from content.
- **AC2 (RD-02)** `EngineFrame→SourceFrame` mapping with arbitrary clip offset/placement is consumed per the input contract; prefetch results from a previous generation can never appear in a new generation's fill (loop/seek isolation proven by test).
- **AC3 (RD-03)** Variable callback frame counts and two independent simultaneous sources keep SPSC/pool single-owner with no bounds violations; `PrefetchConsumer::fill` is verified allocation-free in the `rt_*` lane.
- **AC4 (RD-04)** Corrupt, truncated, unsupported-codec/layout, unseekable, EOF, and prefetch-underrun each surface as a distinct classification (`ErrorKind`/`SourceFault`/`FillReport` field); no missing-data span is reported as verified content.
- **AC5 (RD-05)** Cache-full backpressure, decoder delay, cancelled/closed sources, and reopen leave no buffer/resource leaks; retire happens non-RT; faults stay bounded; block-pool accounting audits zero-loss on close.
- **AC6 (RD-06)** Existing decoder consumers (`ruviform-io-symphonia`/`ruviform-io-avi` tests, AudioLens call surface review) show no regression; every public API change is additive (defaulted trait methods, new crate).
- **AC7 (RD-07)** Test evidence records execution environment, fixture hashes, frame counts, comparison tolerances, and exit codes; `mise run test` (cargo test --workspace) and target-crate tests pass and are recorded; unrun/unsupported codecs are never reported as passed.
- **AC8 (RD-08)** Native PCM sample format/bit depth/scale, PTS/DTS + timebase + provenance, source frame, and the corrupt/EOF/underrun-vs-true-silence distinction are preserved for the existing AudioLens consumer — no forced f32 normalization on the observation path.
- **AC9 (RD-09)** The reader carries the shared metadata contract while the prefetch consumption path is an *option*; every conversion/drop is an explicit Capability/Result/metadata report, never implicit.
- **AC10 (RD-10)** Both-consumer evidence recorded: Synvelis-side WAV clip/seek/underrun/generation tests + AudioLens-side backend/feature regression by API review and/or local-path check; unmeasurable items recorded `unverified`, not `pass`.

## Success Metrics

- `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` all green; `cargo test --workspace --locked` recorded in the impl record.
- Zero changes to existing public API signatures; boundary diff is strictly additive.
- RT fill path proven allocation-free by `assert_no_alloc` in `rt_*` tests.

## Non-goals / Rollback

- New prefetch machinery is independent of the legacy demux/decode path — disable by not constructing it. Old consumers (`Decoder` pump users incl. AudioLens) are not migrated and must not break.
- Filling a missing span and reporting it as verified content is prohibited as a rollback or fallback strategy.
- Any boundary API addition judged wrong is reverted as a whole; no consumer-visible half-measures.

## Constraints

- RT path: no allocation, no locks, no blocking, no OS I/O — enforced by `assert_no_alloc` tests and per-API `RT contract` doc labels.
- No backend type, error, or feature name in `ruviform-media-reader`'s public API (type purity, same rule as media-io); backend crates are dev-dependency test fixtures only.
- No `File`/`Path`/URL in reader/prefetch public signatures — input is `Box<dyn MediaSource>` / `Box<dyn Demuxer>`.
- Conventional Commits; worker commits only (push/PR by coordinator).
