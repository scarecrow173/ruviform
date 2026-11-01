---
id: "PLAN-034Qhwlp4j8Gy5S5uG46Qr"
type: "plan"
status: "approved"
title: "Issue #191 PCM WAV output backend implementation plan"
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
  implements:
    - "docs/specs/issue-191-pcm-wav-output-backend.md"
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
  verifies: []
  verified-by:
    - "docs/test-specs/issue-191-pcm-wav-output-backend.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #191 PCM WAV output backend implementation plan

## Goal

Deliver the first real output backend for Ruviform media-I/O: a `ruviform-io-wav` crate implementing PCM WAV `Encoder`/`Muxer` with honest capability, verified finalize/reopen contract, and boundary-level `MediaSink` + `StagedOutput` in `ruviform-media-io`.

## Scope

One implementation stream, three ordered tasks (T1 boundary ports, T2 backend, T3 tests+verification). T2 depends on T1; T3 depends on T2. Doc chain: discovery→spec→adr→design→plan→test-spec→task→impl-record per `doc-driven-dev-graph`.

## Upstream Documents

- `docs/specs/issue-191-pcm-wav-output-backend.md`
- `docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md`
- `docs/discovery/issue-191-pcm-wav-output-backend.md`

## Design Inputs

- `docs/designs/issue-191-pcm-wav-output-backend.md`

## File Map

- `crates/ruviform-media-io/src/sink.rs` - `MediaSink` trait (`Write + Seek + Send` + `sync()`), impls for `File`, `Cursor`, `&mut T`, `Box<T>`
- `crates/ruviform-media-io/src/staged.rs` - `StagedOutput`, `PublishOptions`, `StagedState`
- `crates/ruviform-media-io/src/lib.rs` - module wiring + re-exports (additive only)
- `crates/ruviform-io-wav/Cargo.toml` - crate manifest (deps: `ruviform-media-io`; dev-deps: `ruviform-io-symphonia`, `ruviform-audio-core`, `ruviform-media-core`)
- `crates/ruviform-io-wav/README.md` - layer/deps/RT notes per crates README convention
- `crates/ruviform-io-wav/src/lib.rs` - public surface, entry points, module docs
- `crates/ruviform-io-wav/src/encoder.rs` - `WavEncoder` (`Encoder` impl, interleave + conversion)
- `crates/ruviform-io-wav/src/muxer.rs` - `WavMuxer` (`Muxer` impl, RIFF header, backpatch finalize)
- `crates/ruviform-io-wav/src/wav.rs` - RIFF/WAVE serialization primitives (LE writers, fmt chunk, WAVEFORMATEXTENSIBLE)
- `crates/ruviform-io-wav/src/convert.rs` - f32→wire sample conversion (saturate/NaN policy)
- `crates/ruviform-io-wav/tests/roundtrip.rs` - write→reopen via `ruviform-io-symphonia` matrix
- `crates/ruviform-io-wav/tests/errors.rs` - capability rejection, invalid input, write-error vs finish
- `crates/ruviform-media-io/src/staged.rs` unit tests + `crates/ruviform-io-wav/tests/staged.rs` - publish/abort/disk-full/overwrite matrix
- `Cargo.toml` (root), `Cargo.lock`, `crates/README.md` - workspace wiring

## Implementation Sequence

- [x] T1: `MediaSink` trait + `StagedOutput` in `ruviform-media-io`, unit tests for staging
- [x] T2: `ruviform-io-wav` crate — `wav.rs`/`convert.rs`/`encoder.rs`/`muxer.rs`, capabilities, entry points, workspace wiring
- [x] T3: integration tests (round-trip matrix, edge cases, staged publish E2E), fix fallout, `mise` verification lanes
- [ ] Doc chain close-out: impl record, followup-triage, exit-audit, commits

## Dependencies and Risks

- `ruviform-io-symphonia` must decode our WAVEFORMATEXTENSIBLE output for >2ch tests; fallback verified form is plain WAVEFORMATEX.
- `StagedOutput` uses `fs::rename` over existing dest on Windows via `MOVEFILE_REPLACE_EXISTING` — verify behavior in tests.
- `ErrorKind` has no dedicated "conflict" variant; dest-exists conflicts map to `InvalidValue` (documented).

## Task Breakdown

- task-191-t1-media-io-sink-staged: `MediaSink` + `StagedOutput` + unit tests (media-io)
- task-191-t2-wav-backend: `ruviform-io-wav` crate implementation + workspace wiring
- task-191-t3-tests-verification: integration tests, staged E2E, verification lanes, impl record inputs

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `test` / `verify-ci` all green
- [x] `cargo test --workspace --locked` green
- [x] Round-trip reopen/decode via symphonia across the documented matrix
- [x] Staged publish failure matrix: dest preserved byte-for-byte
- [x] `git merge origin/main` before final verification; `Cargo.lock` regenerated

## Open Questions / Gaps

- `make_muxer(streams)` sinkless entry point semantics — resolved in design: returns `Unsupported`; real path is `open_muxer(sink, streams)`.
- Encode-side `native_format` = `Fact::NotDeclared` (resolved in design).

## Self-Review

- [x] Every spec requirement maps to a task (R1–R5 → T2, R5–R6 → T1, R7–R9 spread across T2/T3).
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution (single worker).
- Independent work streams: none — T1→T2→T3 sequential.
- User approval required before dispatch: no (dispatched worker, coordinator-approved task).

## Review Handoff

- Focus areas: `MediaSink::sync` trait shape (fsync-level op on `Write + Seek`); `StagedOutput` Drop/abort semantics; WAVEFORMATEXTENSIBLE emission rules; `ErrorKind` mapping table.
