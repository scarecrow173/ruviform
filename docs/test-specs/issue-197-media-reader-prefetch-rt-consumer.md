---
id: "TSPEC-JSe276zDPhy5bjOFWJMtgn"
type: "test-spec"
status: "approved"
title: "Issue #197 media reader / prefetch / RT consumer test specification"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-197-media-reader-prefetch-rt-consumer.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #197 media reader / prefetch / RT consumer test specification

## Purpose

Pin the observable contract of the generic media read path — source-frame addressing, frame mapping, generation isolation, bounded prefetch, RT fill semantics, fault taxonomy — so AC1–AC10 (RD-01–RD-10) of `docs/specs/issue-197-media-reader-prefetch-rt-consumer.md` are testable. Retire only if the reader/prefetch boundary is removed.

## Feature

`SourceReader` sequential decode + frame-accurate seek; `FrameMap` EngineFrame→SourceFrame contract; prefetch pipeline (pool + rings + generation items); `PrefetchConsumer::fill` RT contract; `SourceMetadata` capability facts.

## Rules

- **R-SRC**: `SourceReader` delivers decoded samples in source-frame order across decoder frame boundaries; `position()` is pts-anchored (`Estimated` provenance fallback documented); no fabricated frame count (`frame_count` is `Fact`).
- **R-SEEK**: `seek_source_frame(F)` = demuxer seek → decoder flush → decode-discard to F; `SeekReceipt.landed` is the pts-derived landing, reported honestly (`landed > requested` allowed, never clamped); `meta.seekable = Unsupported` → `Err(Unsupported)` without touching position.
- **R-MODE**: `read_planar` in `DecodeMode::Canonical` only — native mode → `Err`; `next_frame` yields `Frame::Audio`/`Frame::NativeAudio` per mode.
- **R-MAP**: `FrameMap::engine_to_source(e) = clip_offset + (e - placement) * ratio` under the declared contract; negative results are out-of-range, never silently clamped.
- **R-OWN**: every `PcmBlock` has exactly one owner at a time (pool → free ring → producer → data ring → consumer → free ring); `block_count` audit on `close()` = zero loss.
- **R-GEN**: every `PrefetchItem` carries `generation`; after a `Boundary{g}` the consumer drops any item whose `gen != g` to the free ring; stale samples never reach `dst`.
- **R-EOF**: `End{g}` marks stream end; fills past it report `eof=true` and zero-fill the remainder — counted as exhaustion, not underrun.
- **R-UND**: empty data ring while content expected → `underrun` count + zero-fill + telemetry increment; recovery on next fill once data arrives.
- **R-FLT**: `Fault{kind,first,frames}` advances `next_frame` and zero-fills the span; kinds: `Truncated`,`Corrupt`,`Io`,`InvalidSample`,`UnsupportedLayout`,`Unseekable`; decode/demux errors are stream-fatal (`Fault`+`End`).
- **R-MIS**: `fill(dst, expected_first ≠ next_frame)` → `mismatch`, nothing consumed, dst untouched except zero-fill? — No: dst zero-filled AND `mismatch=true`, no stream consumed (caller must seek or resync).
- **R-RT**: `PrefetchConsumer::fill` performs no allocation, lock, blocking, or OS I/O — proven by `assert_no_alloc` in `rt_*` tests across all branches (block-copy, underrun, fault, eof, mismatch, stale-drop).
- **R-META**: `SourceMetadata` carries container/backend/stream facts, canonical `AudioFormat`, `seekable` fact, `native_format` — no loss of observation fields for the native path.
- **R-ADD**: all boundary changes additive — existing `ruviform-io-symphonia`/`ruviform-io-avi` tests pass unmodified; AudioLens call surface unaffected (API review).

## Behavior Matrix

| Scenario | Given | When | Then |
| --- | --- | --- | --- |
| Sequential decode | 10 000-frame mono WAV (deterministic ramp) | `read_planar` to exhaustion | frames == synthesized content; position ends at 10000; `eof` at exactly the last frame |
| SourceFrame 0 | stereo WAV | first read | frame/channel/content match synthesis |
| Mid seek | same | `seek_source_frame(5000)` then read | content == synthesized[5000..]; `landed <= 5000` handled by discard |
| Tail | same | seek to `len-16`, read 16+ | `eof` after last frame; no over-read |
| EOF distinguish | mono WAV | read past end | `ReadOutcome.eof`/`End` marker — never reported as content |
| Mapping | clip_offset=1024, placement=2048 | engine frames 2048..2560 | consumer receives source 1024..1536 content |
| Generation isolation | prefetch running | seek mid-buffer | dst contains only gen-2 samples; `stale_drops` ≥ 1; no stale sample |
| Two sources | two pipelines | interleaved `fill` | each stream correct; pool counts independent; no bounds violation |
| Variable callback | sizes 1..2048 mixed | sequential fills | same content as contiguous read |
| Underrun | empty ring | `fill` | `underrun=frames`, `written=0`, dst zeroed |
| EOF fill | `End` reached | `fill` past end | `eof=true`; tail zeroed; `underrun=0` |
| Corrupt | WAV with corrupted data chunk | pump | `Fault{Corrupt}` classification (or `InvalidData` at reader level) |
| Truncated | WAV with short data chunk | pump | `Fault{Truncated}`/`UnexpectedEos` classification |
| Unseekable | non-seek `MediaSource` | `seekable()`/`seek` | `Fact::Unsupported` / `Err(Unsupported)` |
| Mismatch | wrong `expected_first` | `fill` | `mismatch=true`, no consumption |
| InvalidSample | fake decoder emitting NaN | pump | `Fault{InvalidSample}` + zeroed span |
| Close audit | pipeline mid-stream | `close()` | `blocks_reclaimed == block_count`; producer joined |
| Reopen | new source | `reopen()` then `fill` | new generation; new metadata; old blocks reclaimed |
| RT no-alloc | prefilled ring | `assert_no_alloc(fill)` × all branches | zero allocations |
| Regression | existing backend tests | `cargo test -p ruviform-io-symphonia -p ruviform-io-avi` | all pass unmodified |

## Non-functional

- Env recorded: OS, rustc version, profile, `mise run` commands + exit codes.
- Fixtures: SHA-256 of each synthesized WAV byte stream recorded in the impl record; tolerance = exact f32 equality (PCM S16 → f32 lossless scale) unless a test documents a threshold.
- RT lane: `rt_*` tests run under `mise run test-rt` (name-filtered).
- Unrun/unsupported lanes (mp4/aac feature tests if not run) recorded `unverified`, not `pass`.

## Cross-Repo Compatibility Notes

- AudioLens (read-only): API-surface review of `F:/repositry/audio-products/audio-lens/crates/al-media/src/ruviform_backend.rs` — confirm every called symbol unchanged; classify `no impact` or record deltas; uncheckable items `unverified`.
- Synvelis: no local copy — impact classified `impacted` (new optional API) with rationale; verification deferred to consumer issue, recorded.

## Gaps / Risks

- Corrupt/truncated classes WAV can express are limited to RIFF-size lies and data truncation; codec-internal corruption classes are exercised via a fake `Decoder`/`Demuxer` test double where the real backend cannot produce them.
- Spawned-thread timing is non-deterministic — tests needing determinism use manual `PrefetchProducer::pump`; the spawned worker is covered by a smoke test only.
