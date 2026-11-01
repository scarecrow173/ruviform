---
id: "TASK-034Qhxbnndd80h2whxhIG4"
type: "task"
status: "done"
title: "Issue 191 T1: MediaSink port + StagedOutput in ruviform-media-io"
created: "2026-11-01"
updated: "2026-11-01"
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
    - "docs/plans/issue-191-pcm-wav-output-backend.md"
  implemented-by:
    - "docs/impl/ir/issue-191-pcm-wav-encode-mux-backend-with-mediasink-stagedoutput.md"
  depends-on:
    - "docs/plans/issue-191-pcm-wav-output-backend.md"
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
    - "docs/test-specs/issue-191-pcm-wav-output-backend.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 191 T1: MediaSink port + StagedOutput in ruviform-media-io

## Work

Add the two boundary output ports to `ruviform-media-io`: `MediaSink` trait (`Write + Seek + Send` + `sync()`) with impls for `File`, `Cursor<Vec<u8>>`, `Cursor<&mut [u8]>`, `&mut T`, `Box<T>`; and `StagedOutput`/`PublishOptions` same-dir temp staging with `publish()` (fsync→rename) and `abort()`/Drop cleanup. Additive only — no existing API changes. Unit tests for staging in-module.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-191-pcm-wav-output-backend.md`
- `docs/designs/issue-191-pcm-wav-output-backend.md`

## Dependencies

- None (first task in the stream)

## Affected Files

- `crates/ruviform-media-io/src/sink.rs` (new)
- `crates/ruviform-media-io/src/staged.rs` (new)
- `crates/ruviform-media-io/src/lib.rs` (module wiring + re-exports)

## Done When

- [x] `MediaSink` + `StagedOutput` compile and are publicly exported
- [x] Staging unit tests pass (create/publish/abort/replace/fail-if-exists)
- [x] `mise run check` green for `ruviform-media-io`

## Verification

- [x] `cargo test -p ruviform-media-io` green (staged unit tests)
- [x] `mise run check` + `lint` green
