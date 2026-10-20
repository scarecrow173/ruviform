---
id: "IMPL-034J7u3ZaCkeiCc8zzuQwd"
type: "impl"
status: "completed"
title: "Small fixes from followup triage"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/41"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/buffer.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/layout.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/time.rs"
      - type: "file"
        path: "docs/tasks/small-fixes-from-followup-triage.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/small-fixes-from-followup-triage.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "implement-audio-core-data-model.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Small fixes from followup triage

## Summary

Applied the three Issue #41 scope-3 small fixes to the data-model crates. (a) `AudioBuffer::try_new(spec) -> Result<Self>` added with the shape check (`channels * frames` `usize` overflow + `isize::MAX` byte bound) factored into a shared `checked_slab_len` helper used by `try_new`, `new`, and `from_interleaved`; `new` keeps its `-> Self` panic contract by delegating to `try_new`. (b) `ChannelLayout::Custom` payload changed from `SpeakerMask` to the new non-empty newtype `SpeakerMaskSet` (`new(SpeakerMask) -> Result<Self>` + `mask()` accessor), closing the zero-channel-layout hole structurally; `SpeakerMask` itself is unchanged. (c) Doc-only note on rounding accumulation added to `time.rs` (module docs, `MediaTime::convert`, `MediaTime::checked_add`): each call applies at most one rounding; chained conversions accumulate error — convert from a stable anchor at display/IO boundaries.

## Context

Implements task `docs/tasks/small-fixes-from-followup-triage.md` (Phase T3 of plan `PLAN-034J6OTwge4rS96WZWkZgd`, spec `SPEC-034J6OCXHiP1izKRzbMmgz` AC3), after T1 (enum `#[non_exhaustive]`) and T2 (time arithmetic saturation) which already landed on this branch.

## Implementation

### Changes

- `crates/ruviform-audio-core/src/buffer.rs`
  - New private `checked_slab_len<S: Sample>(spec) -> Result<usize>`: `channels * frames` `usize` overflow → `Err(Error::InvalidValue("frames"))`; `len * size_of::<S>() > isize::MAX` → `Err(Error::InvalidValue("frames"))`. Single authoritative shape check.
  - `AudioBuffer::try_new(spec) -> Result<Self>` — allocating constructor returning `Err` on shape failure, never panics. Doc cross-references `new`.
  - `AudioBuffer::new(spec) -> Self` — delegates to `try_new` + `expect` (panic contract kept, message updated); `# Panics` doc points to `try_new` for fallible use.
  - `from_interleaved` — uses `checked_slab_len` for the slab-length check and `Self::try_new(spec)?` for allocation; `# Errors` doc now lists the `isize::MAX` byte bound (previously reachable only as a panic inside `new`).
  - Struct doc no longer mentions the degenerate zero-channel buffer (unrepresentable after (b)).
  - Tests: `zero_channel_layout_yields_degenerate_buffer` removed (unconstructible); added `try_new_returns_buffer_for_valid_spec`, `try_new_rejects_oversized_shapes` (usize overflow + `isize::MAX` byte bound), `new_keeps_its_panic_contract` (`#[should_panic]`).
- `crates/ruviform-audio-core/src/layout.rs`
  - New `pub struct SpeakerMaskSet(SpeakerMask)` — `Copy` + standard derives; `const fn new(SpeakerMask) -> Result<Self>` returns `Err(Error::InvalidValue("mask"))` on empty; `const fn mask(self) -> SpeakerMask` accessor. Doctest included.
  - `ChannelLayout::Custom(SpeakerMask)` → `Custom(SpeakerMaskSet)`; enum + variant docs rewritten (invariant is now structural, not a documented contract violation).
  - `speaker_mask()` returns `mask_set.mask()`; `custom(mask)` keeps `const fn(SpeakerMask) -> Result<Self>` signature and the `Err(Error::InvalidValue("mask"))` contract, delegating to `SpeakerMaskSet::new`.
  - Tests: `custom_layout_keeps_its_mask` updated for the new payload; `directly_constructed_empty_custom_is_degenerate` replaced by `custom_payload_is_structurally_non_empty`.
- `crates/ruviform-audio-core/src/lib.rs` — `SpeakerMaskSet` added to `pub use layout::{...}` and the crate-level doc list.
- `crates/ruviform-media-core/src/time.rs` — doc-only: module doc paragraph + `MediaTime::convert` paragraph + `MediaTime::checked_add` sentence stating single-rounding-per-call and the stable-anchor guidance.

### Decisions During Implementation

- Validation lives in `checked_slab_len` (module-private) rather than duplicating between `new`/`try_new`/`from_interleaved` — the task's "one place" requirement; `from_interleaved` gains the `isize::MAX` bound as `Err` where it previously panicked inside `new` (consistent with its `Result` contract).
- `SpeakerMaskSet` named per the plan's example; placed in `layout.rs` and re-exported because it appears in a public enum payload.
- `custom()` wraps via `SpeakerMaskSet::new` (match, not `?` — `?` on `Result` is not allowed in `const fn` on this toolchain) so the non-empty rule has one implementation.
- "Err on zero channels" from the triage note is untestable after (b): `channels()` derives from `ChannelLayout`, which can no longer be zero-channel. Zero-frames stays legal and is covered.
- Doc note placed on the `time.rs` module doc + `MediaTime::convert` + `checked_add`; `MediaDuration::convert` deliberately untouched (module doc covers the policy — minimal doc-only change).

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

- `mise run fmt` — applied; `mise run fmt-check` — clean.
- `mise exec -- cargo test -p ruviform-audio-core` — 32 unit tests + 20 doctests passed, 0 failed (includes new `try_new` tests and `SpeakerMaskSet` doctest).
- `mise exec -- cargo test -p ruviform-media-core` — 30 unit tests + 24 doctests passed, 0 failed.
- `mise exec -- cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings.
- `mise exec -- cargo test --workspace` — 88 unit tests + 53 doctests = 141 total, 0 failed (video-core doctests unaffected).

## Risks

- Breaking change accepted per spec (`publish = false`): `Custom(SpeakerMaskSet)` alters the enum payload — all in-workspace construction sites were updated (grep-verified: layout.rs, buffer.rs only).
- `try_new`'s `Err(InvalidValue("frames"))` field name mirrors `from_interleaved`'s existing convention for the same failures.

## Follow-ups

- T4 (plan Verification phase): combined verification + deferred-item Issue filing remains outside this task.
