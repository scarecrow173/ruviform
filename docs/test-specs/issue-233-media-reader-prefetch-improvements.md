---
id: "TSPEC-034Qonw7qg8hdeWBuDqEqQ"
type: "test-spec"
status: "approved"
title: "Issue #233 prefetch discard/drain/reopen_at + contract-doc test specification"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
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
    - "docs/designs/issue-233-media-reader-prefetch-improvements.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #233 prefetch discard/drain/reopen_at + contract-doc test specification

## Purpose

Pin the observable contract of the Issue #233 additive improvements — `PrefetchConsumer::discard_pending` drain semantics, `PrefetchHandle::seek_discard_pending` producer pending purge, `PrefetchHandle::reopen_at` fused reopen+seek — plus the doc-only readability criteria for `expected_first` and `pump()` (spec AC1–AC8). Retire only if these additive surfaces are removed.

## Feature

Bounded RT-safe drain of the prefetch data ring; producer-side discard of stale pending markers on seek; one-command reopen to a source-frame target; contract text readable without tests.

## Rules

- Rule: `discard_pending` is bounded — at most data-ring-capacity pops plus the in-flight block; it allocates nothing, locks nothing, blocks never.
- Rule: `discard_pending` drops pending content but preserves sequencing truth — `Boundary` still adopts the generation and re-anchors `next_frame`; `End`/`Fault` markers still apply their state transitions; dropped blocks always return to the free ring.
- Rule: `discard_pending` stops after consuming the first `Boundary` — items queued behind it are new-generation content and stay queued; the report's `boundary` field carries the adopted `first_frame` (`None` when no `Boundary` was seen).
- Rule: `seek_discard_pending` behaves exactly like `seek` for the caller-visible contract (`Fact::Unsupported` → immediate `ErrorKind::Unsupported` without consuming a generation) and additionally purges the producer `pending` queue before the `Boundary` enqueues — stale `Fault`/`End`/superseded-`Boundary` markers never emit.
- Rule: `reopen_at` emits exactly one `Boundary` whose `first_frame` is the reader's post-seek `position()` — the honest landing, which may exceed the target per `SeekReceipt` rules; a failed post-reopen seek yields `Fault{Unseekable}` + `Boundary` at the reopened position; a failed reopen keeps the `Fault + stream_ended` outcome.
- Rule: `reopen` keeps its signature and frame-0 anchor; `seek` keeps default play-through; every pre-existing test passes unmodified.
- Rule: doc text (rustdoc + module Contract + README) states — without tests — that `expected_first` is an asserted stream position in source frames, and that `pump()` performs ≤1 sequenced item per call with `Backpressured` meaning retry.

## Examples

- Given a full prefetch pipeline and `handle.seek(t)` processed, when the ring holds stale blocks ahead of `Boundary{t'}` and the caller runs `consumer.discard_pending(ctx)`, then the report's `boundary` is `Some(t')`, `consumer.position()` equals `t'`, every drained block returned to the free ring, and the next `fill` serves no pre-boundary content.
- Given `discard_pending` called with no `Boundary` in flight, when the ring holds only current-generation items, then they are dropped to the free ring, `boundary` is `None`, and the position/generation are unchanged.
- Given `pending` holds a stale `Fault` marker when `seek_discard_pending(t)` is processed, then the `Fault` never reaches the consumer and `Boundary` still emits.
- Given `handle.reopen_at(src, open, t)` succeeds, then the producer emits `Boundary{first_frame: reader.position()}` — `position()` ≥ `t` allowed (landed semantics).
- Given `reopen_at` on a non-seekable replacement source, then the stream yields `Fault{Unseekable, t, 0}` followed by `Boundary` at the reopened position (0).
- Given the `rt_` test lane, `assert_no_alloc` proves `discard_pending` allocates zero bytes across backlog sizes 0..=capacity.

## Guarantees

- All pre-existing `ruviform-media-reader` tests pass unmodified.
- New tests live in `crates/ruviform-media-reader/tests/prefetch.rs` (contract) and `tests/rt_fill.rs` (`rt_`-prefixed, `assert_no_alloc`).
- `mise run fmt-check`, `check`, `lint`, `test` are green.

## Non-goals

- Zero-play-through of items pushed to the ring before a drain runs (declined per ADR option B).
- Engine-frame mapping, app-side consumer behavior, backend changes.
- Verifying rustdoc prose by assertion — doc-only ACs are satisfied by reviewed diff, not executable tests.
