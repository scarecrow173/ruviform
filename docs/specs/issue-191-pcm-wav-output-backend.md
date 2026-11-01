---
id: "SPEC-034QhrD80l0ZtpqNTqFvHa"
type: "spec"
status: "approved"
title: "Issue #191 PCM WAV encode/mux output backend"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/191"
    - "docs/discovery/issue-191-pcm-wav-output-backend.md"
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
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-191-pcm-wav-output-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #191 PCM WAV encode/mux output backend

## What Should Be Built

The first real output backend for Ruviform's media-I/O boundary: a PCM WAV `Encoder` + `Muxer` pair in a new `ruviform-io-wav` crate, plus two boundary additions in `ruviform-media-io` — a `MediaSink` output port (`Write + Seek + Send`, mirroring `MediaSource`) and a generic `StagedOutput` safe-publish primitive (same-directory temp file + fsync + rename). The backend serializes canonical planar `f32` `AudioBuffer` frames into a standard RIFF/WAVE file that any compliant reader — including `ruviform-io-symphonia` — can reopen and decode, and reports its capability honestly at both boundary level (`Capabilities`) and format level (`WavCapabilities`).

## Why Now

Synvelis v0.6 requires Local Audio → Project/Graph Playback → Master PCM WAV Export → reopen-and-verify. Ruviform currently has only abstract `Encoder`/`Muxer` traits and demux/decode-only backends, so the entire output path (Frame→Packet→Container→file, drain→finalize→close/sync→reopen) is an unverified contract. Issue #191 is the PoC that validates those contracts against a real implementation before downstream consumers depend on them.

## Users and Value

- Synvelis: gains generic `MediaSink`/finalize primitives and a working PCM WAV exporter behind the media-I/O boundary (product-level publish/overwrite/commit policy stays in Synvelis).
- AudioLens: gets an honest `MUX|ENCODE` capability advertisement for `CodecId::Pcm`/`ContainerFormat::Wav` and can rely on `ErrorKind::Unsupported` for anything outside the declared set.
- Future Ruviform output backends: inherit the `MediaSink` port and `StagedOutput` staging contract instead of re-inventing them.

## Scope

### In Scope

- `ruviform-media-io`: `MediaSink` trait (`Write + Seek + Send`) with blanket impls for `std::fs::File`, `std::io::Cursor`, `Box<dyn MediaSink>`-compatible wrappers; `StagedOutput` staging/publish helper; no changes to existing traits' signatures.
- `ruviform-io-wav` crate: `WavEncoder` (implements `Encoder`), `WavMuxer` (implements `Muxer`), `capabilities()` (boundary `Capabilities`), `wav_capabilities()` (format-level `WavCapabilities`), `WavReceipt` completion receipt, `make_muxer`/`make_encoder` entry points.
- Supported output set (advertised via `wav_capabilities()`): classic RIFF/WAVE little-endian only; integer PCM `U8`, `S16`, `S24`, `S32` and IEEE float `F32` wire formats (`F64` input frames convert to `F32` wire only if declared — see conversion policy); `ChannelLayout::{Mono,Stereo,Quad,FivePointOne,SevenPointOne,Custom}` (Mono/Stereo → `WAVEFORMATEX`, others → `WAVEFORMATEXTENSIBLE` with speaker mask); any `SampleRate` representable as u32; exactly one audio stream per file; RIFF data size ≤ `u32::MAX` (writing beyond → `ErrorKind::InvalidData`, no RF64 fallback).
- Completion contract: `Encoder::drain` flushes encoder-internal buffers after end-of-input; `Muxer::finish` performs seek-back header backpatch + `flush` + `sync_all` and is the *only* completion receipt. Concrete `WavMuxer::finalize()` returns `WavReceipt` (frames, bytes, duration, first-pts); `Muxer::finish()` delegates to it and discards the receipt. `Drop` is best-effort cleanup and never claimed as completion.
- `StagedOutput` contract: creates temp sibling file, exposes the `File` (as `MediaSink`), `publish()` does fsync→rename (with `replace_existing` flag); `Drop`/`abort()` removes temp; existing destination is never destroyed by failure, cancellation, disk-full, partial output, or overwrite conflict.
- Conversion policy (defined, deterministic): f32→int saturates to full-scale; `NaN → 0`; `±Inf → ±full-scale`; no dither applied by the backend (dither is an upstream processing concern); float output formats write IEEE-754 directly.
- Error policy: unsupported codec/format/bit-depth/container/multi-stream → `ErrorKind::Unsupported`; channel-count/sample-rate mismatch vs declared stream → `ErrorKind::InvalidData`; sink write failures → `ErrorKind::Io`.
- Tests: round-trip write→reopen/decode via `ruviform-io-symphonia` across sample rates (44.1k/48k/96k) and layouts (mono/stereo/5.1); zero-frame, short, long-ish, channel-mismatch, NaN/Inf, clipping cases; drain-vs-finalize distinction; write-error vs success distinction; staged publish success/abort/disk-full-simulation/overwrite-replace-and-fail cases; capability rejection tests.

### Out of Scope

- RF64 (>4 GiB) output, BWF `bext` chunk, non-WAV containers, compressed codecs — all return `ErrorKind::Unsupported`.
- Any Synvelis-specific types/policies (Track/Stem/Freeze/AssetId/Export Dialog/Undo/VCS); product-level destination publish/overwrite/project-commit policy.
- RT-safety: encode/mux/file/FS are non-RT by contract; no RT API added.
- WAV demux/decode inside `ruviform-io-wav` (Symphonia already decodes WAV; dev-dependency only for tests).
- Guaranteed cross-OS/filesystem atomicity of rename (best-effort same-volume rename is documented).
- Dithering, sample-rate conversion, channel remapping — upstream responsibilities.

## Requirements

- R1 — Encoder accepts `Frame::Audio` carrying planar `f32` `AudioBuffer`; produces `Packet`s whose `data` is interleaved wire-format bytes of whole frames; `pts` preserved in stream time base (1/sample_rate); `flags` include keyframe.
- R2 — Muxer writes a complete RIFF/WAVE header at open (fmt + data with placeholder sizes), appends packet payload to the data chunk, and at `finish()` backpatches `RIFF` size + `data` size, pads to even boundary, flushes, and `sync_all`s the sink.
- R3 — `WavMuxer::finalize()` returns `WavReceipt { total_frames, total_bytes, duration, first_pts_ticks }`; `finish()` must not be callable twice, `write_packet` after finish → `ErrorKind::InvalidData`.
- R4 — `capabilities()` reports `MUX | ENCODE` (no DEMUX/DECODE), `encode_codecs = [CodecId::Pcm]`; `wav_capabilities()` reports the full format-level supported set (sample formats, layouts, RIFF-only, single-stream, u32 data cap, RF64/BWF unsupported).
- R5 — `MediaSink = Write + Seek + Send`; muxer constructors are generic over it so tests can use `Cursor` and failing mocks.
- R6 — `StagedOutput` never destroys the existing destination: on `Drop`/`abort`, temp removed, dest byte-identical; `publish(replace_existing)` renames only after fsync succeeds; `replace_existing: false` + dest exists → error, dest untouched.
- R7 — All encode/mux errors map to the smallest honest `ErrorKind`; no silent format conversion to an unadvertised format; no fabricated `Fact` values (use `NotDeclared`/`Unsupported` on encode-side `native_format`).
- R8 — Consumer-impact classification for every public API/behavior/dependency change: Synvelis / AudioLens each `impacted | no impact`, verified by building downstream test lanes — `unknown` is not acceptable.
- R9 — Workspace integration: `crates/README.md` index, root `Cargo.toml` members, `Cargo.lock`, crate README with layer/deps/RT notes, consistent with `docs/designs/ruviform-architecture-design-baseline.md`.

## Acceptance Criteria

- [ ] AC1: Write PCM WAV at 44.1k/48k/96k × mono/stereo/5.1 × S16/S24/F32 → reopen via `ruviform-io-symphonia` → decoded length, channel count, and sample values match within the declared conversion policy.
- [ ] AC2: `make_encoder`/`make_muxer` reject unsupported codec/container/bit-depth/multi-stream with `ErrorKind::Unsupported`; no partial file is reported as completed.
- [ ] AC3: Zero-frame file produces a valid empty WAV; channel-mismatch and NaN/overflow inputs produce the declared deterministic output or `InvalidData`; no panic.
- [ ] AC4: `drain` vs `finalize` semantics verified — packets after `drain` require `finish` to be readable; `finish` twice → error; writes after `finish` → error; write-error path returns `Io` and is distinguishable from a successful `finish`.
- [ ] AC5: Same-name destination + cancel/disk-full simulation → old file preserved byte-for-byte; `publish(replace_existing: true)` replaces; `false` fails without touching dest.
- [ ] AC6: Existing `ruviform-media-io` and `ruviform-io-symphonia` tests and offline-driver regression lanes still pass; `mise run fmt-check`/`check`/`lint`/`test`/`verify-ci` all green; `cargo test --workspace --locked` green.

## Deferred Design Concerns

- RF64/BWF output — deferred until a consumer needs >4 GiB or broadcast metadata; the declared `WavCapabilities` documents them as unsupported now.
- Generic per-format capability map on boundary `Capabilities` — deferred; only one output backend exists, so format-level capability lives on the backend (`WavCapabilities`) until a second backend justifies generalization.
- Dither/noise-shaping in the encoder — deferred; upstream processing concern per conversion policy.
- `Seek`-less streaming sinks (network/pipe output) — deferred; RIFF backpatching needs `Seek`; a size-prefixed streaming mode is a future option.

## Sources and Relations

- Derives from: `docs/discovery/issue-191-pcm-wav-output-backend.md` (alternatives analysis, tentative conclusions)
- Source issue: https://github.com/scarecrow173/ruviform/issues/191 (requirements original)
- Related: `docs/adr/media-io-backend-abstraction.md` (boundary stays backend-agnostic), `docs/designs/synvelis-driven-target-architecture-and-migration.md` (recommends `ruviform-io-wav`; Ruviform↔Synvelis contract split)
- Consumer references: Synvelis #21 v0.6 design, AudioLens #127
