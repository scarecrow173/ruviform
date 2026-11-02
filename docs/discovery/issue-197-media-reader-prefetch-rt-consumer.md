---
id: "DISC-uYfaNCLgYERA7zX0pHelL0"
type: "discovery"
status: "resolved"
title: "Issue #197 Generic Media Reader / Decoder・Prefetch・RT Playback Consumer 境界 — inventory, crate placement, contract shape"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/197"
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
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
    - "docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/adr/issue-197-media-reader-boundary-additions.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #197 Generic Media Reader / Decoder・Prefetch・RT Playback Consumer 境界 — inventory, crate placement, contract shape

## Summary

Issue #197 (M5) needs the generic read side of the media foundation: after an application resolves verified media, a backend-agnostic Reader decodes PCM in source-frame order, a bounded non-RT prefetch feeds an RT no-alloc/no-lock/no-I/O consumer, and every loss class (corrupt / truncated / EOF / underrun / invalid sample / unsupported layout / unseekable) is distinguishable — never reported as verified content. The boundary inventory shows the four trait families (`Demuxer`/`Decoder`/`Packet`/`Frame`) are well-formed but have two gaps for a generic reader: there is no trait-level decoder construction (`make_decoder`/`make_native_decoder` are backend-concrete methods) and no declared seek-capability fact (seekability is only discovered as a `seek` failure). Tentative conclusion (confidence: high): a new `ruviform-media-reader` crate — mixed-RT with per-API labels like `ruviform-audio-core` — plus two additive, defaulted `Demuxer` methods is the smallest coherent fit; existing consumers (`ruviform-io-symphonia`/`io-avi` users incl. AudioLens) are untouched.

## Exploration Goal

Confirm where Reader/Prefetch lives, which boundary additions are required vs avoidable, what the source-frame/engine-frame ownership split is, and what the prefetch ownership/generation contract must be — so the spec/ADR pair fixes scope before implementation.

## Key Issues and Assumptions

- `ruviform-media-io` is *boundary only*: `Demuxer` (streams, `read_packet` → `Packet`, time-based `seek` request), `Decoder` (send/receive pump + `drain` + `flush`), all non-RT. Every fact is classified (`Fact`: `Known`/`NotDeclared`/`Unsupported`/`Malformed`); packet timestamps are raw `TickTimestamp` ticks + provenance; `StreamFacts` carries `decodable`, `declared_units`, `start`, `leading_trim`/`trailing_trim`, `container_id`, `codec_tag`, `language`. No trait-level `make_decoder` — `SymphoniaDemuxer::make_decoder`/`make_native_decoder` and `AviDemuxer::{make_decoder,make_native_decoder}` are concrete entry points (AudioLens dispatches via its own `RuvDemux` enum — proof the gap exists).
- Seek capability: the symphonia adapter *probes* `is_seekable`/`byte_len` once at `open_demuxer` construction (`source.rs` save→End→restore) but the result is trapped inside the backend `MediaSourceStream` — the boundary cannot report it. `Demuxer::seek` failure (`ErrorKind::Unsupported`) is the only discoverable signal today. AVI `seek` is unconditionally `Unsupported`.
- `SymphoniaDecoder` decodes eagerly in `send_packet` into an **unbounded** `pending` queue (documented follow-up); canonical mode emits `Frame::Audio` (planar f32 `AudioBuffer`), native mode emits `Frame::NativeAudio` (interleaved f64, native scale, `PcmTransform` history). EOF convention: `Ok(None)` normal, `UnexpectedEos` abnormal. Decoder-side timestamp fallback: packet pts or `Estimated` extrapolation.
- `ruviform-rt` provides the SPSC `channel`/`Producer`/`Consumer` (bounded, `try_push`/`try_pop`/`pop_latest`, leak-free on full via handed-back value), `retire_channel` (leak-on-full deferred drop), `slot` snapshot exchange, `RtContext` capability token (`mint` is `#[doc(hidden)]`, drivers only). `alloc_counter` is `#[cfg(test)]`-private; other crates use `assert_no_alloc` dev-dep (ADR `rt-assert-no-alloc-thread-check`).
- `ruviform-audio::typed` runtime consumes planar `f32` slots (`AlignedSamples`, `Slot`, `NodeIo`) via `GraphProcessor::process(&mut NodeIo, &RtContext)`; plan generation/activation (`PreparedPlan.generation`, `RuntimeCommand`, `SharedStatus` fault flags) already implements the "generation-safe activation" pattern the prefetch generation contract mirrors.
- AudioLens pins ruviform `080c228d` via git dep; its `ruviform_backend.rs` uses `open_demuxer`, `make_native_decoder`/`make_decoder`, `read_packet`, `send_packet`/`receive_frame`/`drain`, `StreamFacts.index`/`decodable`, `codec.native_format`, `Packet` fields — all stable surfaces; additive change = no impact. Synvelis consumes clip-offset/placement → engine-frame mapping (caller-side per Issue scope 2).
- `ruviform-io-wav` is encode/mux only (no demux); WAV decode stays via symphonia. `ruviform-io-avi` demux+decodes PCM auds + DIB vids, `seek` = `Unsupported`, no `idx1` consult.

## Alternatives and Comparison

Crate placement:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| New `ruviform-media-reader` crate (Domain foundations; deps: media-io, media-core, audio-core, ruviform-rt) | media-io stays a pure non-RT boundary crate (its crate-level contract says the *whole crate* is non-RT); prefetch mixes non-RT producer + RT consumer — a mixed-RT crate with per-API labels (precedent: audio-core) keeps both honest; usable by Synvelis AND AudioLens without engine dep | One more workspace member (index/Cargo.lock/docs upkeep) | yes |
| Inside `ruviform-media-io` | No new crate | Contradicts the crate's stated "non-RT only" contract; RT consumer in a crate documented as RT-forbidden is a contract violation | no |
| Inside `ruviform-audio` engine | Close to graph consumer | AudioLens (equal consumer, RD-08–10) would inherit an engine dep; reader is media foundation, not engine | no |
| Inside `ruviform-rt` | Primitives home | rt holds contract defs + primitives, no I/O — reader does I/O via media-io | no |

Decoder construction / seek capability (the two boundary gaps):

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Additive `Demuxer::make_decoder(&self, StreamId, DecodeMode) -> Result<Box<dyn Decoder>>` (default `Unsupported`) + `Demuxer::seekable(&self) -> Fact<()>` (default `NotDeclared`) | One authoritative construction point; every consumer (reader, Synvelis, AudioLens) gets backend-agnostic access incl. native path; `Fact` matches the observation contract (AVI → `Unsupported`, symphonia → probe result); default impls keep third-party impls compiling — truly additive | Boundary API grows by 2 methods + `DecodeMode` enum | yes |
| Caller-supplied factories (closures/enum per backend) | No trait change | Duplicates AudioLens's `RuvDemux` enum dispatch in every consumer — the exact "same policy in several places" smell | no |
| Reader assumes seekable, fails at seek | Minimal API | Streaming/unseekable sources can't be declared up front (scope 1 requires explicit capability) | no |

Prefetch shape:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Bounded `PcmBlock` pool + SPSC data ring + SPSC free-return ring + `PrefetchItem` control markers (Boundary/Fault/End) | Single-owner handoff provable; every stream element advances `next_frame` deterministically; generation tag per item = stale-drop rule identical to typed runtime's plan-generation pattern; telemetry counters via shared atomics | Two rings + pool bookkeeping | yes |
| Shared lock-free frame-indexed cache (random access by source frame) | True cache semantics | Random access needs an index structure — more machinery than sequential playback needs; harder single-owner proof | no |
| One SPSC of `AudioBuffer` frames | Simplest | Allocates per frame (RT-incompatible handoff unless pooled); no inline fault/boundary sequencing | no |

RT consumer contract:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `fill(&mut [&mut [f32]], expected_first: SourceFrame, &RtContext) -> FillReport` — strict position check, zero-fill + counters for gaps/faults/EOF, never fabricate | `expected_first` rejects stale-plan reads; distinct report fields (`written`/`underrun`/`faulted`/`eof`/`mismatch`) keep "real silence" vs "missing" un-collapsed (scope 4); `&RtContext` gate matches house style | Caller must track its own position (it already does — it owns the frame mapping) | yes |
| `fill(dst, frames)` sequential only | Simpler signature | Can't detect a caller position bug; stale reads served silently | no |

## Tentative Conclusions and Hypotheses

- **Confirmed**: `SourceReader` owns `Box<dyn Demuxer>` + `Box<dyn Decoder>` + selected `StreamInfo`; reads in *source frames* (sample instants at the stream's decoded rate), position anchored by frame pts→sample conversion (`MediaTime` convert to `1/rate` base) with `Estimated` fallback when pts/base unavailable — never a fabricated base.
- **Confirmed**: `SourceMetadata` = `ContainerInfo` + `BackendInfo` + `StreamInfo` + canonical `AudioFormat` + `frame_count: Fact<u64>` (derived from declared duration → rate, `NotDeclared` when unknown — `declared_units` is *encoded* units and must not be conflated with decoded frames for codecs like AAC) + `seekable: Fact<()>` + `native_format` (RD-08/09 lossless path metadata stays attached).
- **Confirmed**: `DecodeMode { Canonical, Native }` belongs in media-io (trait param); symphonia + AVI get real impls — AudioLens's native path is thus reachable through the trait too, though its existing concrete calls keep working unchanged.
- **Confirmed**: seek contract = `demuxer.seek(MediaTime)` coarse reset → `decoder.flush()` → decode-discard forward to the target source frame; `SeekReceipt { requested, landed }` reports the pts-derived landing honestly (landing > target is reported, not silently accepted).
- **Confirmed**: generation = `u64` bumped on every seek/reset/reopen/stop; every `PrefetchItem` carries it; consumer drops non-matching blocks to the free ring — the same generation-safe pattern as `RtRuntime` plan activation.
- **Confirmed**: producer drive = dedicated non-RT thread (`std::thread`; `ruviform-exec` thread-priority not required for the boundary PoC) plus a synchronous `pump` path so tests can drive deterministically; control channel = `std::sync::mpsc` (non-RT side may block — recv_timeout cadence).
- **Hypothesis → verify in impl**: `block_frames`/`block_count` sizing (`PrefetchConfig`) and free-ring capacity = `block_count` makes the free push infallible — every block always has a slot, so the consumer can never be forced to dealloc on RT.
- **Hypothesis**: invalid samples (NaN/Inf in decoded PCM) are producer-detected → `Fault{InvalidSample}` marker + zeroed span in-block; the consumer counts them as faulted, never as content.
- **Confirmed**: prefetch can be disabled by simply not constructing it (old `Demuxer`/`Decoder` direct use unchanged — additive migration); "fill missing data and report success" rollback is prohibited by contract (`FillReport` distinguishes every non-content span).

## Open Questions

- `frame_count` provenance name: report as `Fact<u64>` *declared* count (from duration) and let `position`/`decoded total` be the measured truth — label docs carefully.
- Whether `Reopen` on the prefetch handle should carry a caller-supplied `open` closure (`Box<dyn Fn(Box<dyn MediaSource>) -> Result<Box<dyn Demuxer>>>`) — leaning yes: reopen = re-probe on a fresh source under a new generation; no `File`/`Path` in the API (provider interchange, scope 6).
- Thread driver: `std::thread` inside the crate vs. caller-supplied pump — both offered; the spawned worker is a convenience, manual `PrefetchProducer::pump` stays the testable core.

## Promotion Candidates

- [x] spec-doc needed: reader/prefetch scope — metadata contract, source-frame model, generation/ownership, fault taxonomy, RT bounds, acceptance RD-01–RD-10.
- [x] adr-doc needed: (1) `ruviform-media-reader` crate vs alternatives; (2) boundary additions `Demuxer::make_decoder`/`seekable`/`DecodeMode`; (3) prefetch ownership/generation/telemetry contract; (4) fault-vs-silence policy and rollback/no-fabrication rule.

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/197 (scope 1–6, RD-01–RD-10, rollback/non-goals); consumer refs Synvelis #17/#18/#21, AudioLens #127; prior gates #184 (typed IR), #195 (PreparedPlan/RtRuntime/PDC/generation activation), #191 (WAV backend + MediaSink/StagedOutput + symphonia re-read precedent)
- `crates/ruviform-media-io/src/{lib,demux,decode,frame,packet,codec,capability,error,source}.rs` — boundary inventory (read)
- `crates/ruviform-io-symphonia/src/{lib,demuxer,decoder,source,map,error}.rs` — decode backend; construction-time seekable probe trapped inside `MediaSourceStream` (read)
- `crates/ruviform-io-avi/src/{lib,demuxer}.rs` — `seek` → `Unsupported`; `make_decoder`/`make_native_decoder` concrete (read)
- `crates/ruviform-rt/src/{spsc,retire,snapshot,context,processor}.rs` — SPSC/retire/slot/RtContext (read)
- `crates/ruviform-audio/src/typed/{buffer,io,runtime,compile}.rs` — borrowed PCM slot contract + generation activation precedent (read)
- `F:/repositry/audio-products/audio-lens/crates/al-media/src/ruviform_backend.rs` — equal consumer's native-path usage (read)
- `docs/designs/synvelis-driven-target-architecture-and-migration.md` — M5 row: "Generic Source/Decoder/NonRT Prefetch/RT Buffer" is Ruviform's responsibility; SourceId/Asset/Verified-Blob is the app's

## Evidence

- 2026-11-02 — Trait inventory: `Demuxer` = `container`/`backend_info`/`streams`/`stream`/`read_packet`/`seek`; `Decoder` = `send_packet`/`receive_frame`/`drain`/`flush`; both `Send`, non-RT only. `ErrorKind` = `{Io, Unsupported, InvalidData, InvalidValue, TimeOverflow, UnexpectedEos}` — sufficient for fault taxonomy; reader-level faults compose these + pipeline states (underrun/invalid-sample/mismatch). `Fact` is `Copy` when `T: Copy`, `is_known`/`value()`/`map` API ready.
- 2026-11-02 — Symphonia decode PoC test (`tests/wav_decode_poc.rs`) synthesizes deterministic S16 WAV in-memory: packet cap 1152 frames/packet, `pts` ticks = sample index in `1/rate` base — establishes source-frame addressing for WAV fixtures without LFS assets.
- 2026-11-02 — AudioLens `ruviform_backend.rs` confirms every API it touches is untouched by this design (enum dispatch over `make_native_decoder`/`make_decoder`, `StreamFacts`, `Packet` fields); `ruviform` pin `080c228d` predates this work so any boundary additions cannot regress it.
