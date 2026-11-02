---
id: "TASK-voSb34RCLdA14Lc1kTfNlD"
type: "task"
status: "done"
title: "Issue 197 T1: Demuxer::seekable + make_decoder + DecodeMode boundary additions"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-197-media-reader-prefetch-rt-consumer.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-197-media-reader-prefetch-rt-consumer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-197-media-reader-prefetch-rt-consumer.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 197 T1: Demuxer::seekable + make_decoder + DecodeMode boundary additions

## Work

Add `DecodeMode { Canonical, Native }` (`#[non_exhaustive]`) to `ruviform-media-io::decode`; add defaulted `Demuxer::seekable() -> Fact<()>` (default `NotDeclared`) and `Demuxer::make_decoder(&self, StreamId, DecodeMode) -> Result<Box<dyn Decoder>>` (default `Err(Unsupported)`) to `ruviform-media-io::demux`; re-export. Implement in `SymphoniaDemuxer` (plumb construction-time `is_seekable` probe through; dispatch canonical/native) and `AviDemuxer` (`seekable` = `Fact::Unsupported`; `make_decoder` dispatch). Additive only; unit test each backend's declared facts.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-197-media-reader-prefetch-rt-consumer.md`
- `docs/designs/issue-197-media-reader-prefetch-rt-consumer.md`
- `docs/adr/issue-197-media-reader-boundary-additions.md`

## Dependencies

- None (first task in stream)

## Affected Files

- `crates/ruviform-media-io/src/decode.rs`, `demux.rs`, `lib.rs`
- `crates/ruviform-io-symphonia/src/{demuxer,source,lib}.rs`
- `crates/ruviform-io-avi/src/demuxer.rs`

## Tests

- symphonia: `seekable()` = `Known` for `Cursor`, `NotDeclared`/`Unsupported` for a non-seek `MediaSource`; `make_decoder` both modes decode a frame.
- AVI: `seekable()` = `Unsupported`; `make_decoder` canonical decodes.
- Regression: existing crate tests unchanged.

## Done

- Both methods exist with defaults; both backends report/construct correctly; `mise run check` + crate tests green.
