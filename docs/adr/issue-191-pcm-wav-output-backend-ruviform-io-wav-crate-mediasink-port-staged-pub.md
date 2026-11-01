---
id: "ADR-034QhrQYpNhSfuTeq7hyfp"
type: "adr"
status: "accepted"
title: "Issue #191 PCM WAV output backend: ruviform-io-wav crate, MediaSink port, staged publish, conversion policy"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/191"
    - "docs/discovery/issue-191-pcm-wav-output-backend.md"
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
  related:
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-191-pcm-wav-output-backend.md"
    - "docs/specs/issue-191-pcm-wav-output-backend.md"
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

# Issue #191 PCM WAV output backend: ruviform-io-wav crate, MediaSink port, staged publish, conversion policy

## Context and Problem Statement

Ruviform's media-I/O boundary defines `Encoder`/`Muxer` traits but has zero output implementations; `ruviform-io-symphonia` and `ruviform-io-avi` are input-only. Issue #191 needs the first real output backend (PCM WAV encode+mux) plus the missing output-side infrastructure: an output sink port (input side has `MediaSource: Read + Seek + Send`; nothing symmetric exists), and safe staging/publish so a failed export can never destroy an existing destination. The open questions are: (a) which crate owns the WAV backend, (b) where the sink port and staging primitive live, (c) what sample-conversion policy the encoder owns, and (d) how much of the WAV family (RIFF/RF64/BWF) the PoC supports.

## Decision Drivers

* Codec/family responsibility: each backend crate owns one container/codec family (`io-symphonia` = symphonia adapter, `io-avi` = AVI/RIFF input).
* Dependency DAG: `ruviform-media-io` must stay backend-agnostic (ADR `media-io-backend-abstraction`); backend crates depend on it, never the reverse.
* The migration design doc (`synvelis-driven-target-architecture-and-migration.md`) already recommends a `ruviform-io-wav`-shaped crate and assigns Ruviform the "generic FS/sink/codec finalization primitives" while Synvelis keeps product publish/overwrite/commit policy.
* Honest capability: encode capability must not be inferred from decode capability; unsupported formats must return `ErrorKind::Unsupported`, never silently convert to an unadvertised format.
* Completion honesty: a file is only complete after header backpatch + flush + sync succeed; `Drop` is best-effort and never a receipt.
* Consumer safety: `StagedOutput` must preserve the existing destination byte-for-byte on any failure path, without promising cross-FS atomicity.

## Considered Options

Backend placement:
* New `ruviform-io-wav` crate
* Inside `ruviform-io-symphonia`
* Inside `ruviform-io-avi`
* Inside `ruviform-media-io` (boundary crate)

Output-port placement:
* `MediaSink` trait in `ruviform-media-io` (`Write + Seek + Send`, mirrors `MediaSource`)
* Per-backend sink types

Staging/publish placement:
* Generic `StagedOutput` in `ruviform-media-io`
* Per-crate `publish` helpers

Conversion policy ownership:
* Defined deterministic policy in `WavEncoder` (saturate, NaN→0, no dither)
* Passthrough/no conversion (accept only frames already in wire format)
* Configurable dither/clip options on the encoder

WAV family scope:
* Classic RIFF/WAVE only (RF64/BWF → `Unsupported`)
* RIFF + RF64 + BWF now

## Decision Outcome

Chosen options:

1. **New `ruviform-io-wav` crate**, because it matches the `ruviform-io-<family>` convention, keeps codec responsibility clean (WAV RIFF *output* is unrelated to symphonia's codec adapters and unrelated to `io-avi`'s AVI input family), keeps the DAG acyclic (`io-wav → media-io → media-core/audio-core`), and is what the migration design doc recommends. Rejected: `io-symphonia` (a hand-rolled WAV writer has nothing to do with symphonia; conflating them corrupts the adapter's purpose), `io-avi` (input-only family; its private RIFF *reader* shares no primitives with a writer; mixing directions corrupts both responsibilities), `media-io` (the boundary crate must not contain backends).
2. **`MediaSink` trait in `ruviform-media-io`**, because output needs a symmetric boundary port to `MediaSource` — all future output backends share it, and `WavMuxer` stays generic so tests inject `Cursor`/failing mocks. Rejected: per-backend sinks (no uniform staging story, every backend re-invents the port).
3. **`StagedOutput` in `ruviform-media-io`**, because same-dir temp + fsync + rename is generic FS policy every output backend needs; one authoritative implementation avoids the "same policy in several places" smell. `replace_existing` is the only policy knob — product-level overwrite/commit policy stays with consumers (Synvelis). Rejected: per-crate helpers (duplication).
4. **Deterministic conversion policy in `WavEncoder`**: f32→int saturates to full scale; NaN→0; ±Inf→±full-scale; integer wire formats {`U8`,`S16`,`S24`,`S32`} and float {`F32`}; no dither (upstream concern, documented). Rejected: passthrough-only (canonical frames are always planar f32 — a passthrough encoder can't accept them); configurable dither (YAGNI for PoC, upstream responsibility).
5. **Classic RIFF/WAVE only**, because the acceptance PoC doesn't need >4 GiB or broadcast metadata; RF64 and BWF are declared `Unsupported` in `WavCapabilities`. Rejected: RF64/BWF now (scope creep, unneeded complexity).

### Consequences

* Good, because the workspace gains one new crate + two boundary additions and nothing existing changes shape; Symphonia's `Unsupported` encode/mux answers stay true and are now verifiably honest.
* Good, because the completion contract (`finalize()` → `WavReceipt` → `publish()`) is testable end-to-end and never confuses a dropped temp file with a finished export.
* Good, because `StagedOutput` is immediately reusable by the next output backend.
* Bad, because the boundary crate gains an FS-aware helper (`StagedOutput`), widening its surface — accepted because it is already the IO boundary and the helper is path-generic.
* Bad, because two-level capability (boundary `Capabilities` + `WavCapabilities`) must be documented so consumers know where to look — accepted over premature generalization of `Capabilities`.

### Confirmation

Verified by: round-trip tests writing WAV via `WavMuxer`+`WavEncoder` and reopening via `ruviform-io-symphonia` decode across rates/layouts/depths; staged-publish abort/disk-full/overwrite tests asserting destination byte-identical; capability rejection tests asserting `ErrorKind::Unsupported`; `mise run fmt-check`/`check`/`lint`/`test`/`verify-ci` and `cargo test --workspace --locked` all green; DAG check that `ruviform-media-io` gains no backend dependency.

## Implementation Plan

* Affected paths: `crates/ruviform-media-io/src/{lib.rs,sink.rs(new),staged.rs(new)}`; new `crates/ruviform-io-wav/`; root `Cargo.toml` (`members`), `Cargo.lock`, `crates/README.md` index.
* Patterns to follow: `ruviform-io-avi` crate shape/layout/README conventions; `MediaSource` trait style for `MediaSink`; `ErrorKind` mapping discipline from `io-symphonia`.
* Patterns to avoid: silent format conversion, fabricated `Fact` values, `Drop`-as-completion, product-policy leaks (no Track/Stem/AssetId/undo concepts), RT claims on file IO.
* Dependencies or configuration: `ruviform-io-wav` depends on `ruviform-media-io` (+ transitively `ruviform-audio-core`/`ruviform-media-core` types); dev-dependency on `ruviform-io-symphonia` for round-trip tests only — no symphonia code in the backend itself. `tempfile` avoided: `StagedOutput` generates sibling temp names itself to keep the temp file on the destination volume.

## Verification

* [ ] `mise run fmt-check` / `check` / `lint` / `test` / `verify-ci` green
* [ ] `cargo test --workspace --locked` green
* [ ] Round-trip tests (44.1k/48k/96k × mono/stereo/5.1 × S16/S24/F32) pass via symphonia reopen/decode
* [ ] Staged publish failure/cancel/disk-full/overwrite tests assert destination preserved
* [ ] Capability/`Unsupported` rejection tests pass; no fabricated capabilities advertised
* [ ] `ruviform-media-io` public API diff is additive only (no signature changes)

## Pros and Cons of the Options

### New `ruviform-io-wav` crate

* Good, because it mirrors `ruviform-io-avi` naming/layer convention and isolates the WAV family cleanly.
* Good, because the migration design doc already recommends this shape, so it aligns with the target architecture.
* Bad, because it adds one workspace member requiring index/README upkeep.

### Inside `ruviform-io-symphonia`

* Good, because it reuses an existing crate slot.
* Bad, because a hand-rolled WAV muxer is unrelated to symphonia's codec adapter role — a category error that confuses both responsibilities.

### Inside `ruviform-io-avi`

* Good, because WAV and AVI share the RIFF container heritage.
* Bad, because `io-avi` is an *input* family for AudioLens; its private RIFF reader shares no primitives with a writer; mixing mux into a demux crate conflates direction.

### `MediaSink` in `ruviform-media-io`

* Good, because it is the symmetric output port `MediaSource` never had, shared by all future output backends.
* Bad, because it is a public API addition to the boundary crate (additive, non-breaking, but surface grows).

### Deterministic conversion in `WavEncoder`

* Good, because canonical frames are planar f32 — a real encoder must own the conversion, and a documented saturating policy is honest and testable.
* Bad, because the policy is fixed (no dither knob) — acceptable for PoC, documented as upstream responsibility.

## More Information

* Spec: `docs/specs/issue-191-pcm-wav-output-backend.md` (requirements R1–R9, acceptance AC1–AC6)
* Discovery: `docs/discovery/issue-191-pcm-wav-output-backend.md` (alternatives tables, tentative conclusions)
* Contract boundary: `docs/adr/media-io-backend-abstraction.md`
* Consumer split: `docs/designs/synvelis-driven-target-architecture-and-migration.md`
* Issue: https://github.com/scarecrow173/ruviform/issues/191
