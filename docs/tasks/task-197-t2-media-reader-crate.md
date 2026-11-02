---
id: "TASK-e5oaEOXS8FGE9PrXrLwkGi"
type: "task"
status: "done"
title: "Issue 197 T2: ruviform-media-reader crate — metadata, SourceReader, prefetch pipeline"
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
    - "docs/tasks/task-197-t1-demuxer-seekable-make-decoder.md"
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

# Issue 197 T2: ruviform-media-reader crate — metadata, SourceReader, prefetch pipeline

## Work

New workspace member `ruviform-media-reader` (Domain foundations; deps `ruviform-media-io`, `ruviform-media-core`, `ruviform-audio-core`, `ruviform-rt`; dev-deps `ruviform-io-symphonia`, `assert_no_alloc`). Modules: `mapping` (`SourceFrame`/`EngineFrame`/`RequestedRange`/`FrameMap`), `metadata` (`SourceMetadata`/`stream_metadata`), `reader` (`SourceReader` pump, position anchor, `seek_source_frame`, `read_planar`, `SeekReceipt`, `ReadOutcome`), `prefetch` (`PrefetchConfig`, `PcmBlock`, `PrefetchItem`, `SourceFault`, `FillReport`/`FaultCounts`, `PrefetchTelemetry`, `PrefetchHandle`, `PrefetchProducer` incl. spawned worker, `PrefetchConsumer::fill` `&RtContext`-gated). Wire `Cargo.toml` members + `crates/README.md` index. Per-API RT labels per crate conventions.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-197-media-reader-prefetch-rt-consumer.md`
- `docs/designs/issue-197-media-reader-prefetch-rt-consumer.md`
- `docs/adr/issue-197-media-reader-boundary-additions.md`

## Dependencies

- `task-197-t1-demuxer-seekable-make-decoder.md` (trait methods consumed)

## Affected Files

- `crates/ruviform-media-reader/` (new)
- `Cargo.toml`, `crates/README.md`, `Cargo.lock` (generated)

## Tests

- Smoke-level unit tests colocated; full acceptance tests in T3.

## Done

- Crate compiles; `SourceReader` decodes WAV end-to-end; `Prefetch` producer/consumer wire up; `mise run check` + `lint` green.
