---
id: "IMPL-034J6ZxczGEFBgX52lYD1y"
type: "impl"
status: "completed"
title: "Apply non_exhaustive to public enums"
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
        path: "crates/ruviform-media-core/src/error.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/time.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/transport.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/layout.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/color.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/lib.rs"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/tasks/apply-non-exhaustive-to-public-enums.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/apply-non-exhaustive-to-public-enums.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# Apply non_exhaustive to public enums

## Summary

Applied the unified public-enum `#[non_exhaustive]` policy (ADR-034J6OnZouPI8HQy6cIg9D) to the three data-model crates: added enum-level `#[non_exhaustive]` to the 9 public enums that lacked it, removed `#![allow(clippy::exhaustive_enums)]` from all three `lib.rs` files so clippy now enforces the policy, updated the adjacent policy comments, and recorded the policy in `AGENTS.md` 暫定規約. `PixelFormat` and `SampleFormat` were already annotated and are unchanged.

## Context

Implements task `docs/tasks/apply-non-exhaustive-to-public-enums.md` (Phase T1 of plan `PLAN-034J6OTwge4rS96WZWkZgd`, spec `SPEC-034J6OCXHiP1izKRzbMmgz` AC1) per ADR `docs/adr/public-enum-non-exhaustive-policy.md` — Option 1: every public enum reachable from outside the crate gets enum-level `#[non_exhaustive]`; sibling workspace crates count as external users; crate-internal enums are exempt.

## Implementation

### Changes

Enum audit — every `pub enum` in the workspace (grep-verified; `ruviform-math` / `ruviform-rt` / `ruviform-exec` contain no `pub enum`, confirming the audit's expectation):

| Enum | Crate / file | Result |
| --- | --- | --- |
| `Error` | media-core `error.rs` | attribute applied |
| `StreamKind` | media-core `stream.rs` | attribute applied |
| `Rounding` | media-core `time.rs` | attribute applied |
| `TransportStatus` | media-core `transport.rs` | attribute applied |
| `ChannelLayout` | audio-core `layout.rs` | attribute applied |
| `ColorPrimaries` | video-core `color.rs` | attribute applied |
| `TransferCharacteristics` | video-core `color.rs` | attribute applied |
| `MatrixCoefficients` | video-core `color.rs` | attribute applied |
| `ColorRange` | video-core `color.rs` | attribute applied |
| `PixelFormat` | video-core `pixel.rs` | already annotated — kept |
| `SampleFormat` | audio-core `format.rs` | already annotated — kept |

No crate-internal enums exist to exempt (only 11 `pub enum` workspace-wide, all listed above).

- Each newly annotated enum also gained a one-line rustdoc note ("The enum is `#[non_exhaustive]` per the unified public-enum API policy — new variants may be added without breaking downstream `match` arms."), matching `SampleFormat`'s existing style.
- The three `lib.rs` files: removed `#![allow(clippy::exhaustive_enums)]` and rewrote the adjacent comment — it previously documented the deliberate exhaustive-matching decision for a `publish = false` crate; it now states exhaustive matching is reserved for crate-internal code, external users (sibling crates included) get forward compatibility via enum-level `#[non_exhaustive]`, and references the ADR. `#![allow(clippy::module_name_repetitions)]` and `#![allow(clippy::exhaustive_structs)]` are unchanged.
- `AGENTS.md` 暫定規約: added one bullet after the lint bullet — public enums must carry enum-level `#[non_exhaustive]` (crate-internal enums exempt), no `exhaustive_enums` allow, enforced by clippy pedantic, with a link to the ADR.

### Decisions During Implementation

- Doc note phrasing mirrors `SampleFormat`'s existing "The enum is `#[non_exhaustive]` — …" pattern rather than introducing a new wording.
- `#[non_exhaustive]` placed immediately above `#[derive(...)]`, matching the existing `PixelFormat`/`SampleFormat` layout.
- lib.rs comments keep the "literal construction" wording for structs only, since `exhaustive_structs` remains allowed; the enum half now points to the ADR.

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

- `mise run fmt` — ran; no remaining diffs (fmt applied cleanly).
- `mise exec -- cargo check --workspace --all-targets` — passed, 0 warnings.
- `mise exec -- cargo clippy --workspace --all-targets -- -D warnings` — passed, 0 warnings with the `exhaustive_enums` allow removed (policy now lint-enforced).
- `mise exec -- cargo test --workspace` — passed: 80 unit tests (media-core 24, audio-core 30, video-core 23, math/rt/exec 1 each) + 50 doc tests (media-core 23, audio-core 18, video-core 6, math/rt/exec 1 each) = 130 total, 0 failed.
- `grep -rn "#\[non_exhaustive\]" crates/*/src` — attribute present on all 11 audited enums.
- `grep -rn "exhaustive_enums" crates/*/src/lib.rs` — 0 hits.
- Cross-crate exhaustive `match` re-check: all `match` sites in audio-core / video-core are same-crate `match self` on their own enums or matches on `Result`/tuples; no sibling-crate match on media-core enum variants exists. No doctest matches exhaustively on these enums (no `match` in doctests at all). `if let`/`let` destructuring on these enums: none found.
- `PixelFormat`'s `finish_non_exhaustive` Debug impl (`frame.rs:409`, on `VideoFrame` — a struct) is unaffected.

## Risks

- Clippy `exhaustive_enums` only fires for enums it can see; a future public enum added without the attribute will be caught by `lint` / `lint-ci` (-D warnings gate) — this is the intended enforcement mechanism.
- External users must now use a `_` arm when matching these enums; the ADR accepts this trade-off. No such external match sites exist in the workspace today, so nothing had to be rewritten.

## Follow-ups

- Subsequent phases of the same plan: `fix-time-arithmetic-saturation.md` (T2) and `small-fixes-from-followup-triage.md` (T3, touches audio-core `layout.rs` / `lib.rs` again).
