---
id: "IMPL-034QhS434AsmoWfCforctH"
type: "impl"
status: "completed"
title: "Issue 191: PCM WAV encode/mux backend with MediaSink + StagedOutput"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/191"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-media-io/src/sink.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/staged.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/"
    modified:
      - type: "file"
        path: "crates/ruviform-media-io/src/lib.rs"
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
  implements:
    - "docs/tasks/task-191-t1-media-io-sink-staged.md"
    - "docs/tasks/task-191-t2-wav-backend.md"
    - "docs/tasks/task-191-t3-tests-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-191-pcm-wav-output-backend.md"
    - "docs/designs/issue-191-pcm-wav-output-backend.md"
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
    - "docs/specs/issue-191-pcm-wav-output-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-191-pcm-wav-output-backend.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-191-pcm-wav-output-backend.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 191: PCM WAV encode/mux backend with MediaSink + StagedOutput

## Summary

Implemented a real PCM WAV encode/mux backend in a new `ruviform-io-wav`
crate: `WavEncoder` (planar `f32` frames → interleaved wire packets) and
`WavMuxer` (packets → classic little-endian RIFF/WAVE with header
backpatch, flush, and sink sync at finalize). Added the output-side
boundary ports to `ruviform-media-io`: the `MediaSink` port trait
(`Write + Seek + Send + sync()`) and `StagedOutput`, a generic
same-directory temp-file staging/publish primitive that never touches an
existing destination until a successful fsync→rename publish.

## Context

Implements `docs/plans/issue-191-pcm-wav-output-backend.md` (approved),
following `docs/designs/issue-191-pcm-wav-output-backend.md` (approved)
and ADR
`docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md`
(accepted: new backend crate, additive media-io ports, staging owned by
media-io). Verified against
`docs/test-specs/issue-191-pcm-wav-output-backend.md`.

## Implementation

### Changes

- `ruviform-media-io`: new `sink` module — `MediaSink` blanket-style
  trait mirroring `MediaSource`, impls for `File` (`sync` →
  `sync_all`), `Cursor<Vec<u8>>`, `Cursor<&mut [u8]>`, `&mut T`,
  `Box<T>`. New `staged` module — `StagedOutput::create`,
  `publish(sync)`, `abort`, Drop cleanup, `PublishOptions
  { replace_existing, sync_before_publish }`; same-dir temp name with
  `.rvfstage-<pid>-<counter>` uniqueness; overwrite-off conflict maps to
  `ErrorKind::InvalidValue`.
- `ruviform-io-wav` (new workspace member, 30th crate): `wav.rs`
  serialization primitives (RIFF/fmt/data, `WAVEFORMATEX` for
  mono/stereo, `WAVEFORMATEXTENSIBLE` + channel mask for 5.1 and other
  masked layouts, odd-length pad, `u32::MAX` RIFF cap);
  `convert.rs` canonical `f32` → U8/S16/S24/S32/F32 wire conversion
  (saturating, NaN→0, ±Inf→±full-scale, no dither/SRC/remap);
  `encoder.rs` `WavEncoder` (frame validation, planar→interleave,
  PTS conversion to stream time base, drain semantics, send-after-drain
  rejected); `muxer.rs` `WavMuxer` (single audio stream, immediate
  header write, packet validation incl. stream-id and block-align,
  backpatch+flush+`sink.sync()` finalize, `WavReceipt` with
  frames/bytes/duration/first-PTS, `into_sink`, write-after-finish and
  double-finish rejected); `lib.rs` capability surface
  (`MUX | ENCODE`, `encode_codecs = [CodecId::Pcm]`, no demux/decode,
  `native_format = Fact::NotDeclared`, RF64/BWF unsupported, RIFF size
  cap documented).

### Decisions During Implementation

- `make_muxer` without a sink returns `Unsupported`; real writing goes
  through `open_muxer(sink, streams)` — a sinkless muxer has no honest
  behavior.
- `Drop` on `WavMuxer` performs best-effort finalize only and is never
  a completion receipt; only `finish()` returns the `WavReceipt`.
- Nonzero first PTS cannot be represented as padding in plain WAV; it
  is retained diagnostically on `WavReceipt.first_pts` and packets are
  treated as contiguous.
- Destination-overwrite conflicts use `ErrorKind::InvalidValue` (the
  closest honest existing kind; documented in spec/ADR).
- Intentional float→int casts after clamp carry narrowly-scoped
  `#[allow]` with justification rather than weaker checked-conversion
  noise; `clippy::cast_possible_truncation`/`cast_sign_loss` only.

## Related Experiments

None (no experiment logs created for this chain).

## Validation

- `cargo test -p ruviform-media-io`: 53 unit + 20 doc tests green.
- `cargo test -p ruviform-io-wav`: unit tests green;
  `tests/roundtrip.rs` 9/9 green (44.1k/48k/96k × mono/stereo/5.1 ×
  S16/S24/S32/F32 write→reopen via `ruviform-io-symphonia`→decode→
  length/channel/sample-value verification; zero-frame, NaN/Inf/
  clipping, drain, send-after-drain, short file);
  `tests/contract.rs` 19/19 green (capability honesty, unsupported
  codec/container/sample-format/multi-stream rejection, channel and
  sample-rate mismatch, stream-id and partial-payload validation, sink
  write/finalize errors → `Io`, finish-after-finish, write-after-finish,
  staged drop/abort/no-overwrite/disk-full/replace preserving old
  destination byte-for-byte).
- `mise run fmt-check`, `check`, `lint` (`-D warnings`), `test`
  (workspace) green.

## Risks

- `verify-ci` (CI profile) still runs as the final gate before the graph
  exit-audit; dev-profile lanes are all green.
- Cross-filesystem rename atomicity is explicitly not guaranteed by
  `StagedOutput` (documented); same-directory staging keeps the common
  case atomic on NTFS/POSIX.
- Staged-output drop cleanup is best-effort (no `sync` guarantee on
  temp-file removal), as designed.

## Follow-ups

- RF64/`bext`/BWF remain unsupported; if long-form (>4 GiB) export is
  needed, a follow-up issue should extend `ruviform-io-wav` with RF64
  `ds64` handling and advertise it via capabilities.
- `native_format` for encode-side is `Fact::NotDeclared`; a follow-up
  can populate honest facts once the encode pipeline demands them.
