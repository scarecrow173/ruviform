---
id: "DISC-034QhqgG9EF5ipLnFJv0Ka"
type: "discovery"
status: "resolved"
title: "Issue #191 PCM WAV encode/mux backend — crate placement, output contract, safe publish"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/191"
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
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-191-pcm-wav-output-backend.md"
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #191 PCM WAV encode/mux backend — crate placement, output contract, safe publish

## Summary

Issue #191 requires the first *real* output backend for Ruviform's media-I/O boundary: a PCM WAV `Encoder`/`Muxer` with honest capability advertisement, a verified finalize/close/reopen contract, and safe staging/publish primitives. The codebase has well-formed abstract traits (`Encoder`, `Muxer`, `Packet`, `StreamDescriptor`, `Capabilities`, `ErrorKind`) but zero output implementations — `ruviform-io-symphonia` is demux/decode only and `ruviform-io-avi` is input-only. Tentative conclusion (confidence: high): a new `ruviform-io-wav` backend crate plus a small `MediaSink` output port and a generic staged-publish primitive in `ruviform-media-io` is the cleanest fit for the existing layer/DAG rules and the migration design doc's own recommendation.

## Exploration Goal

Confirm where a PCM WAV encoder/muxer backend should live, which boundary additions (output sink, publish primitive, capability granularity) are required vs. avoidable, and what the concrete completion contract must be — so the spec/ADR pair can fix scope before implementation.

## Key Issues and Assumptions

- The `Muxer`/`Encoder` traits exist but have never been driven by a real backend; several contract details (packet granularity, `finish()` vs `drain()`, timestamp semantics, `native_format` on the encode side) are unverified assumptions in the API surface.
- There is no output-side port: `MediaSource` covers `Read + Seek + Send` input only. A WAV muxer needs a `Write + Seek + Send` sink (seek is required for RIFF header backpatching at finalize).
- `Capabilities` is codec-granular (`encode_codecs: Vec<CodecId>`); WAV format constraints (bit depth, layout, RIFF size cap, RF64 unsupported) need a backend-level capability declaration or they can only be discovered at construction failure time.
- Safe publish (temp-file staging + rename) is generic FS machinery, not WAV-specific; placing it inside a WAV crate would force duplication when the next output backend lands.
- `AudioBuffer` is planar canonical `f32`; a WAV writer must interleave and convert to the declared wire `SampleFormat`. Clipping/NaN/Inf/dither policy is currently undefined anywhere.
- `ruviform-io-avi` has a private RIFF *reader*; writing is a different primitive set (little-endian writers, size backpatching, WAVEFORMATEXTENSIBLE).

## Alternatives and Comparison

Crate placement (the issue's explicit comparison axis — codec responsibility / dependency DAG / public API):

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| New `ruviform-io-wav` crate | Matches `ruviform-io-avi` naming/layer convention; keeps codec family ownership clean (WAV RIFF output); migration design doc recommends "ruviform-io-wav相当"; zero risk to existing backends; dev-dep on io-symphonia only for round-trip tests | One more workspace member (index/docs upkeep) | yes |
| Inside `ruviform-io-symphonia` | Reuses existing crate slot | Category error: crate is "symphonia adapter"; hand-rolled WAV writer is unrelated to symphonia; symphonia has no write infra anyway | no |
| Inside `ruviform-io-avi` | Both are RIFF-family containers | io-avi is an input-family for AudioLens; AVI demux ≠ WAV mux; private reader primitives don't help a writer; conflates direction | no |
| Inside `ruviform-media-io` | No new crate | Boundary crate must stay backend-agnostic per ADR `media-io-backend-abstraction` | no |

Output-port placement:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `MediaSink` trait in `ruviform-media-io` (`Write + Seek + Send`, mirrors `MediaSource`) | Symmetric boundary; all future output backends share it; muxer signature stays concrete-agnostic | Small public API addition (additive, non-breaking) | yes |
| Per-backend sink type | No boundary change | Every output backend invents its own; no uniform staging story | no |

Safe-publish placement:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `StagedOutput` primitive in `ruviform-media-io` (path-generic, works with any `MediaSink`/`File`) | One authoritative implementation; usable by every future output backend; product overwrite policy stays with consumer via a minimal `replace_existing` flag | Adds FS-aware helper to boundary crate (acceptable: it is already the IO boundary) | yes |
| Per-crate `publish` helpers | Localized | Duplicated policy, exactly the "same policy in several places" structural smell | no |

Capability granularity:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Keep boundary `Capabilities` codec-level; add `wav_capabilities()` returning a backend `WavCapabilities` struct (bit depths, layouts, RIFF-only, max size, single audio stream) | Honest at both levels; no boundary churn; queryable before construction | Two-level model must be documented | yes |
| Extend `Capabilities` with a generic format-capability map | One level | Premature generality (YAGNI) for a single backend | no |

## Tentative Conclusions and Hypotheses

- **Confirmed**: `Encoder::send_frame`/`receive_packet`/`drain` and `Muxer::streams`/`write_packet`/`finish` signatures are sufficient for WAV; the *semantics* need pinning (packet = wire-encoded span of whole frames; `drain` flushes encoder's internal buffer after end-of-input; `finish` = backpatch + flush + sync, the only completion receipt).
- **Confirmed**: `CodecId::Pcm` and `ContainerFormat::Wav` already exist — no boundary enum churn needed.
- **Hypothesis → to verify in impl**: `native_format` on encode-side `CodecDescriptor` is best declared `Fact::Unsupported`/`NotDeclared` (the wire encoding *is* `format`); document rather than force a fabricated value.
- **Confirmed**: Plain RIFF/WAVE only for PoC — RF64 (>4 GiB) and BWF `bext` are `Unsupported`; data size tracked and rejected at u32 limit, not wrapped.
- **Confirmed**: `WAVEFORMATEXTENSIBLE` emitted for layouts other than Mono/Stereo or when a speaker mask must be carried; plain `WAVEFORMATEX` (tag 1 PCM int / tag 3 IEEE float) otherwise — maximum decoder compatibility including symphonia.
- **Confirmed**: Nonzero initial PTS is not representable in plain WAV; muxer treats packets as contiguous (documented), receipt reports `first_pts_ticks` for diagnostics rather than silently padding.
- **Confirmed**: Staging contract — `StagedOutput` creates `<dest>.tmp-*` in same directory (same volume ⇒ same-FS rename), writes through `File` (implements `MediaSink`), optional `sync_all`, then rename. Drop/abort removes temp; destination byte-for-byte untouched. Overwrite policy = caller choice (`replace_existing` flag); no cross-FS atomicity claimed.
- **Hypothesis**: f32→int conversion saturates to full-scale; NaN→0, ±Inf→±full-scale; dither is upstream responsibility (declared non-goal). Round-trip tests pin exact sample-level behavior.

## Open Questions

- Encode-side `CodecDescriptor::native_format`: `Fact::NotDeclared` vs `Fact::Unsupported` — decide in design doc (both honest; `Unsupported` reads stronger, `NotDeclared` reads "not meaningful").
- Whether `Muxer::finish()` should keep `Result<()>` (trait) while concrete `WavMuxer::finalize() -> Result<WavReceipt>` carries the receipt — leaning yes (no trait change; receipt retrievable).
- Temp-file naming pattern and whether collision retry count needs a bound — trivial implementation detail, default 8 attempts.

## Promotion Candidates

- [x] spec-doc needed: PCM WAV output backend scope — supported formats/layouts/depths, completion contract (drain/finalize/close/reopen), staging/publish semantics, capability advertisement, error policy.
- [x] adr-doc needed: (1) crate placement `ruviform-io-wav` vs alternatives; (2) `MediaSink` + `StagedOutput` as boundary additions; (3) conversion/clipping/NaN policy ownership; (4) RIFF-only scope (no RF64/BWF).

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/191 (requirements original: generic target capability, acceptance PoC, M6 gate, deps/out-of-scope)
- `crates/ruviform-media-io/src/{lib,encode,mux,capability,codec,error,frame,packet,demux,source}.rs` — abstract boundary (read)
- `crates/ruviform-io-symphonia/src/lib.rs` — decode-only precedent; `make_muxer`/`make_encoder` return `Unsupported` (read)
- `crates/ruviform-io-avi/` — RIFF input family precedent (read)
- `docs/designs/synvelis-driven-target-architecture-and-migration.md` — recommends `ruviform-io-wav`相当; contract split Ruviform (generic FS/sink/codec finalize primitives) vs Synvelis (product publish/overwrite/commit policy)
- `docs/adr/media-io-backend-abstraction.md` — boundary crate stays backend-agnostic
- `docs/discovery/issue-194-m0-baseline-audit.md` — workspace baseline (28 crates, DAG clean)
- Synvelis #21 v0.6 design (issue link) — consumer side of the export contract

## Evidence

- 2026-11-01 — Trait inventory: `Encoder: Send` = `send_frame`/`receive_packet -> Result<Option<Packet>>`/`drain` (default); `Muxer: Send` = `streams`/`write_packet`/`finish` (checked); both doc-marked non-RT. `Packet` carries `data` + stream id/index + optional pts/dts/duration/trim/flags. `Capabilities` splits `DEMUX|MUX|DECODE|ENCODE|NATIVE_AUDIO` and `can_encode` requires flag + `encode_codecs` membership. `ErrorKind` = `{Io, Unsupported, InvalidData, InvalidValue, TimeOverflow, UnexpectedEos}` — sufficient for WAV needs. `AudioFrame` wraps planar `f32` `AudioBuffer` + `MediaTimestamp`. `SampleFormat` includes `F32/F64/U8/S8/U16/S16/U24/S24/U32/S32`. `ChannelLayout` has named layouts + validated custom masks. `MediaSource: Read + Seek + Send` exists; no output counterpart.
