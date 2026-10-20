---
id: "IMPL-034J7bMHds4IPYPvoxiMZR"
type: "impl"
status: "completed"
title: "Fix time arithmetic saturation"
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
        path: "crates/ruviform-media-core/src/time.rs"
      - type: "file"
        path: "docs/tasks/fix-time-arithmetic-saturation.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/fix-time-arithmetic-saturation.md"
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

# Fix time arithmetic saturation

## Summary

Fixed the early-saturation / early-`Err` bug in `crates/ruviform-media-core/src/time.rs` time arithmetic (Issue #41 scope 2). Delta conversion now stays an exact `i128` rational quotient with rounding applied once, and the `i64` range check applies only to the final `start + delta` value. `MediaRange::end()` remains total and saturates only on true final overflow; new `MediaRange::checked_end(rounding) -> Result<MediaTime>` exposes the same computation as `Err(Error::TimeOverflow)`; `MediaTime::checked_add`/`checked_sub` share the semantics. The shared conversion internals were refactored so `scaled_fraction` is provably total (`|num| <= 2^63 * (2^32 - 1)^2 < 2^127`), `round_quotient` owns the magnitude-rounding + sign policy once, and `convert_value`/`convert_value_exact` keep their existing public contracts.

## Context

Implements task `docs/tasks/fix-time-arithmetic-saturation.md` (Phase T2 of plan `PLAN-034J6OTwge4rS96WZWkZgd`, spec `SPEC-034J6OCXHiP1izKRzbMmgz` AC2, Issue #41 scope 2).

Bug analysis (reproduced before fixing): `MediaRange::end()` converted the duration via `MediaDuration::convert`, whose `i64` range check applies to the *intermediate* delta. When `start + delta` is representable but `delta` alone exceeds `i64`, `end()` saturated the delta and returned a wrong value (`-1` instead of `i64::MAX - 1`), while `checked_add`/`checked_sub` surfaced the conversion `Err` instead of judging the final sum. Repro: `cargo test -p ruviform-media-core` — `range_end_delta_wider_than_i64` and `checked_add_sub_delta_wider_than_i64` FAILED against the pre-fix code with exactly the predicted wrong values.

## Implementation

### Changes

`crates/ruviform-media-core/src/time.rs`:

- Internal helpers: `scaled_fraction` now returns `(i128, u128)` unconditionally — its `Option` carried a phantom failure mode (`|num| <= 2^63 * (2^32 - 1)^2 < 2^127` proves `i128` cannot overflow); extracted `round_quotient(num, den, rounding) -> i128` (the pre-existing magnitude-rounding + sign-reapply policy, unchanged); added `convert_value_wide(value, from, to, rounding) -> i128` as the total conversion used by end/add/sub arithmetic. `convert_value` and `convert_value_exact` are thin wrappers on the same internals with unchanged external contracts.
- `MediaRange::end()` = clamp `end_wide(NearestEven)` to the `i64` range — total function preserved, saturates only on true final overflow. New `MediaRange::checked_end(rounding) -> Result<MediaTime>` returns `Err(Error::TimeOverflow)` only when the final value is unrepresentable. Both share the private `end_wide(rounding) -> i128` (`|start| + |delta| < 2^127`, cannot overflow).
- `MediaTime::checked_add`/`checked_sub` compute `i128::from(self.value) ± convert_value_wide(...)` and `Err` only when the final value is out of `i64` range; `checked_sub` negates the `i128` delta, never the `i64` duration value (`i64::MIN` edge).
- Docs: `# Errors` on `checked_add`/`checked_sub`/`checked_end`/`convert` updated to describe final-value-only range checks; `end()` doc states saturation happens only on true final overflow. RT contract labels preserved.
- Tests: 6 new cases — `range_end_delta_wider_than_i64`, `checked_add_sub_delta_wider_than_i64` (regression repros incl. negative mirror and `i64::MIN`-duration sub), `checked_end_errors_only_on_final_overflow`, `checked_end_applies_each_rounding_mode_once` (all 5 `Rounding` modes at ±1.5/2.5-tick boundaries), `negative_duration_moves_both_directions`, `checked_end_agrees_with_checked_add` (Ok and Err agreement over an input matrix). New doctest on `checked_end`.

### Decisions During Implementation

- Fix placed in the shared conversion internals (`scaled_fraction`/`round_quotient`/`convert_value_wide`) rather than duplicating wide-arithmetic per caller — one authoritative implementation of "round once, check the final value".
- `scaled_fraction` made total instead of threading an unreachable `Option`: the proven bound makes the failure mode phantom, and a total helper is what the total `end()` path needs. Documented the bound at the definition site.
- `end()` and `checked_end` share `end_wide` rather than `end()` unwrapping `checked_end` (which would lose the saturation direction on `Err` without recomputing).
- `i64::try_from(...).ok().ok_or(Error::TimeOverflow)` matches the file's existing conversion style.
- The rounding-accumulation doc note is intentionally untouched — it is T3(c) scope (`small-fixes-from-followup-triage.md`).

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- Repro first: the two regression tests FAILED against pre-fix code with exactly the predicted wrong values (`end()` returned `-1` for `i64::MAX - 1`; `checked_add` returned `Err(TimeOverflow)` for `Ok(i64::MAX - 1)`).
- `mise exec -- cargo test -p ruviform-media-core` — 30 unit tests + 24 doctests, 0 failed.
- `mise exec -- cargo clippy -p ruviform-media-core --all-targets -- -D warnings` — 0 warnings.
- `mise run fmt-check` — no diff.
- `mise run check` / `mise run lint` / `mise run test` (workspace) — all green; no callers of the changed APIs exist outside `time.rs`, so sibling crates are unaffected.

## Risks

- `scaled_fraction` is now total under the proven bound `|num| <= 2^63 * (2^32 - 1)^2 < 2^127`; if `TimeBase` fields ever widen beyond `u32`/`i64`, that bound must be re-derived (the bound is documented at the definition site).
- `round_quotient`'s `unwrap_or(i128::MAX)` narrowing fallback is unreachable by the same bound; it exists to satisfy `TryFrom` without panicking machinery.

## Follow-ups

- `small-fixes-from-followup-triage.md` (T3) adds the single-rounding-per-call / error-accumulation doc note to `MediaTime::convert` / `checked_add` — adjacent to the docs touched here.
