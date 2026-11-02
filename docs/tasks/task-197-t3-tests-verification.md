---
id: "TASK-02vjFSak80ajcQNqOMIF0s"
type: "task"
status: "done"
title: "Issue 197 T3: RD-01..RD-10 acceptance tests + verification record"
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
    - "docs/tasks/task-197-t2-media-reader-crate.md"
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

# Issue 197 T3: RD-01..RD-10 acceptance tests + verification record

## Work

Write the acceptance tests per `docs/test-specs/issue-197-media-reader-prefetch-rt-consumer.md`: `tests/reader.rs` (WAV mono/stereo at frame 0/mid-seek/tail/EOF, native mode, unseekable source, corrupt/truncated), `tests/prefetch.rs` (mapping, generation isolation, two sources, variable callback sizes, close audit, reopen), `tests/rt_fill.rs` (`rt_*` `assert_no_alloc` fill branches), fake `Decoder`/`Demuxer` doubles for fault classes WAV cannot express. Run `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` + `cargo test --workspace --locked`; record env/fixture hashes/tolerances/exit codes; AudioLens API-surface review + consumer impact classification.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-197-media-reader-prefetch-rt-consumer.md`
- `docs/designs/issue-197-media-reader-prefetch-rt-consumer.md`
- `docs/test-specs/issue-197-media-reader-prefetch-rt-consumer.md`

## Dependencies

- `task-197-t2-media-reader-crate.md`

## Affected Files

- `crates/ruviform-media-reader/tests/`
- `docs/impl/ir/` (impl record)

## Tests

- The tests are the deliverable.

## Done

- All lanes green; impl record with evidence written; consumer impact classified (no `unknown` left unlabeled).
