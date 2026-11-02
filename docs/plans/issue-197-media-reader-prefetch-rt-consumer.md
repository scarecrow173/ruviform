---
id: "PLAN-YNOz2WL10n4v4Avi6qlRE9"
type: "plan"
status: "approved"
title: "Issue #197 media reader / prefetch / RT consumer implementation plan"
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
  implements:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
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
  verifies: []
  verified-by:
    - "docs/test-specs/issue-197-media-reader-prefetch-rt-consumer.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #197 media reader / prefetch / RT consumer implementation plan

## Goal

Deliver the generic media read path: additive `Demuxer` boundary ports (`seekable`, `make_decoder` + `DecodeMode`), backend impls in symphonia + AVI, and a new `ruviform-media-reader` crate carrying `SourceMetadata`/`SourceReader`/`FrameMap` and the bounded prefetch → RT `PrefetchConsumer` contract per the approved design.

## Scope

One stream, three ordered tasks. T1 (boundary additions) precedes T2 (reader crate); T3 (tests + verification) covers RD-01–RD-07 evidence after T2. Doc chain per `doc-driven-dev-graph`.

## Upstream Documents

- `docs/specs/issue-197-media-reader-prefetch-rt-consumer.md`
- `docs/adr/issue-197-media-reader-boundary-additions.md`
- `docs/discovery/issue-197-media-reader-prefetch-rt-consumer.md`

## Design Inputs

- `docs/designs/issue-197-media-reader-prefetch-rt-consumer.md`

## File Map

- `crates/ruviform-media-io/src/decode.rs` — `DecodeMode` enum
- `crates/ruviform-media-io/src/demux.rs` — `seekable()`/`make_decoder()` defaulted methods
- `crates/ruviform-media-io/src/lib.rs` — re-export wiring
- `crates/ruviform-io-symphonia/src/demuxer.rs` + `source.rs` + `lib.rs` — seekable probe plumbing + trait impls
- `crates/ruviform-io-avi/src/demuxer.rs` — `seekable=Unsupported`, `make_decoder` dispatch
- `crates/ruviform-media-reader/Cargo.toml` + `README.md` — new member (deps: media-io, media-core, audio-core, ruviform-rt; dev-deps: io-symphonia, assert_no_alloc)
- `crates/ruviform-media-reader/src/{lib,mapping,metadata,reader,prefetch}.rs` — public API
- `crates/ruviform-media-reader/tests/{reader,prefetch,rt_fill}.rs` — acceptance evidence
- `Cargo.toml` (workspace members) + `crates/README.md` + `Cargo.lock` — index/generated consistency

## Task Graph

- `docs/tasks/task-197-t1-demuxer-seekable-make-decoder.md` — boundary additions + backend impls
- `docs/tasks/task-197-t2-media-reader-crate.md` — `ruviform-media-reader` (metadata/reader/prefetch)
- `docs/tasks/task-197-t3-tests-verification.md` — RD-01..RD-07 tests, rt lane, verification record

Dependencies: T1 → T2 → T3.

## Implementation Order

1. T1: `DecodeMode` + defaulted `Demuxer` methods in media-io; plumb symphonia probe → `seekable()`; trait impls in symphonia + AVI. `mise run check` green.
2. T2: new crate skeleton (manifest/README/workspace member) → `mapping`/`metadata` → `SourceReader` (pump/position/seek/read_planar) → prefetch (pool/items/producer/consumer/telemetry/lifecycle). `mise run check` + `lint` green.
3. T3: tests per test-spec; `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`; record env/hash/tolerance/exit codes in impl record.

## Verification

- `mise run fmt-check`, `check`, `lint`, `test`, `test-rt` — all green.
- `cargo test --workspace --locked` recorded (RD-07), plus target-crate runs.
- Consumer impact classification (RD-06/RD-10): AudioLens `ruviform_backend.rs` API review → `no impact`; Synvelis → `impacted` (additive API, no forced change). Unverified items labeled.

## Risks and Open Items

- Symphonia `make_decoder` returns `Self`, not `Box<dyn Decoder>` — wrap at the trait boundary (`Ok(Box::new(self.make_decoder(id)?))`); mode `Native` wraps `make_native_decoder`.
- Seek landing position: WAV symphonia seek is frame-accurate via `SeekMode::Accurate` + pts ticks = sample index; codec backends may land early → decode-discard forward is implemented in `SourceReader::seek_source_frame`; landed > requested is reported via `SeekReceipt`.
- `block_count` capacity must cover markers: data ring capacity = `block_count + MARKER_SLACK` (Boundary/Fault/End share the data ring) so markers never stall on a full ring.
