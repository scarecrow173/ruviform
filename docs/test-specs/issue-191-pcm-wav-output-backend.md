---
id: "TSPEC-034Qhwm0HeEmVIU2j6ttxT"
type: "test-spec"
status: "approved"
title: "Issue #191 PCM WAV output test specification"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source:
    - "docs/specs/issue-191-pcm-wav-output-backend.md"
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
    - "docs/designs/issue-191-pcm-wav-output-backend.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-191-pcm-wav-output-backend.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #191 PCM WAV output test specification

## Purpose

Pin the observable contract of the PCM WAV output backend — what a correct write→finalize→reopen cycle guarantees, what errors honest capability returns, and what safe staging preserves — so AC1–AC6 of `docs/specs/issue-191-pcm-wav-output-backend.md` are testable. Retire only if the WAV backend or staging primitives are removed.

## Feature

PCM WAV encode → mux → finalize → reopen/verify; capability advertisement and rejection; staged publish protection.

## Rules

- R-WRITE: `WavEncoder::send_frame` accepts `Frame::Audio` matching the declared `CodecDescriptor` (codec `Pcm`, audio format = declared wire `SampleFormat`/layout/rate) and emits `Packet`s whose `data` is interleaved wire bytes of whole frames; pts in stream time base; `flags` keyframe.
- R-MUX: `WavMuxer::new` writes a complete RIFF/WAVE header immediately; `write_packet` appends `Packet.data` to the data chunk; `finalize()` backpatches RIFF+data sizes, pads to even, flushes, syncs, and returns `WavReceipt` — the only completion receipt.
- R-DRAIN: `Encoder::drain` marks end-of-input; `receive_packet` then yields remaining buffered packets then `Ok(None)`. `drain` ≠ `finalize`; `finish()` is not callable twice; `write_packet` after `finish` → `InvalidData`.
- R-CAP: `capabilities()` reports `MUX|ENCODE`, `encode_codecs=[Pcm]`, no `DEMUX`/`DECODE`; `wav_capabilities()` reports the full supported set; anything outside → `ErrorKind::Unsupported` (codec ≠ Pcm, non-audio, unsupported bit depth, >1 stream, non-WAV).
- R-CONV: f32→int saturates to full scale; NaN→0; ±Inf→±full-scale; no dither; float wire writes IEEE-754.
- R-SIZE: total data > `u32::MAX` → `InvalidData` (no RF64); Mono/Stereo → WAVEFORMATEX tag 1 (int) / tag 3 (float); other layouts → WAVEFORMATEXTENSIBLE tag 0xFFFE with channel mask.
- R-MISMATCH: frame channel count / sample rate ≠ declared stream → `InvalidData`.
- R-STAGE: `StagedOutput` writes to a sibling temp; `Drop`/`abort` removes temp; destination never modified until `publish`; `publish` fsyncs then renames; `replace_existing=false` + dest exists → error, dest byte-identical.
- R-ERR: sink write/seek/flush failures → `ErrorKind::Io`; error and success paths are distinguishable; no partial file reported complete.

## Examples

- EX1 (AC1): 48 kHz stereo S16, 4800 frames sine → `muxer.finalize()` receipt `total_frames=4800` → reopen via `ruviform-io-symphonia` → decoded 4800 frames stereo, samples within S16 quantization of source.
- EX2 (AC1): 44.1 kHz mono S24 and 96 kHz 5.1 F32 round-trips — length/channel/sample checks pass; 5.1 emits WAVEFORMATEXTENSIBLE.
- EX3 (AC3): zero frames → finalize → reopen → valid file, 0 frames, no error.
- EX4 (AC3): frame with wrong channel count → `send_frame` → `Err(InvalidData)`; frame containing NaN/±Inf → written as 0/±full-scale per R-CONV; reopen confirms.
- EX5 (AC4): `finish()` then `finish()` → `Err(InvalidData)`; `write_packet` after `finish` → `Err(InvalidData)`; failing sink mid-write → `Err(Io)`, and `finalize` receipt never produced.
- EX6 (AC5): dest `out.wav` exists with bytes B → `StagedOutput::create(out.wav)` → write → abort → `out.wav` == B; same with publish `replace_existing=true` → dest == new bytes; `false` → error + dest == B.
- EX7 (AC5): simulated disk-full (sink that errors on write N) → `write_packet`/`finalize` → `Err(Io)` → drop staged → temp gone, dest preserved.
- EX8 (AC2): `make_encoder` with `CodecId::Aac`-style non-Pcm descriptor → `Err(Unsupported)`; `make_muxer`/`open_muxer` with 2 streams → `Err(Unsupported)`; no output file created.

## Guarantees

- A finalized WAV file always parses: RIFF header sizes self-consistent, even-padded, fmt chunk valid for the layout/depth written.
- The only path that produces a completed destination is `finalize()` → `publish()`; every other path leaves the destination byte-identical or absent.
- Capability answers are honest: advertised set is exactly what constructors accept; rejections are `Unsupported`, never silent conversion.
- `ErrorKind` mappings are the smallest honest kind; `Io` vs `InvalidData` vs `Unsupported` are distinguishable.

## Non-goals

- Bit-exact decode equivalence through every third-party reader (only `ruviform-io-symphonia` reopen is the contract).
- RF64/BWF, compressed formats, multi-stream, dither quality, resampling.
- Cross-filesystem rename atomicity, OS-level crash durability beyond fsync.
- RT behavior (all encode/mux/FS APIs are non-RT by contract).

## Risk

If these guarantees are dropped: a crashed/failed export can silently truncate or corrupt the user's existing master file (data loss), capability lies cause downstream consumers to attempt impossible exports, and drain/finalize confusion produces unplayable half-written files reported as complete.
