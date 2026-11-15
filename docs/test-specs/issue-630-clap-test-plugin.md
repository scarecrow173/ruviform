---
id: "TSPEC-034XTBjvQhJKJMQHxSptND"
type: "test-spec"
status: "approved"
title: "Issue #630 CLAP test_gain — テスト仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-clap-test-plugin.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-clap-test-plugin.md"
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
  related:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-clap-test-plugin.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-630-clap-test-plugin.md"
    - "docs/designs/issue-630-clap-test-plugin.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 CLAP test_gain — テスト仕様

## Purpose

Pin the observable contract of the CLAP `test_gain` fixture so the
production host path is verified against an independent oracle — the
same guarantee shape as the VST3 slice's test spec. Retirable when the
suite's CLAP coverage is superseded by a broader fixture family.

## Feature

CLAP `test_gain` fixture verified through the production host path.

## Rules

- Rule: Discovery — `ClapLibrary::load` on the built cdylib yields
  exactly one descriptor named `test_gain`, vendor `"Ruviform Test"`,
  version `0.0.0`, features containing `audio-effect`.
- Rule: Parameter surface — `plugin.params()` reports exactly one param:
  id `0`, name `Gain`, plain range `[0.0, 2.0]`, default `1.0`.
- Rule: DSP law — `out[c][n] = in[c][n] * gain(n)`; stereo main ports;
  deterministic per-activation reset to default gain.
- Rule: Sample-accurate automation — a normalized `ParamValue` event
  (`0.75`) at frame offset `8` maps through host `denormalize` to plain
  `1.5` and switches gain at frame 8: frames 0–7 unity, 8–63 ×1.5;
  the value persists into the next event-free block.
- Rule: Domain honesty — normalized↔plain mapping is non-identity
  (`plain = normalized * 2`), so domain confusion produces wrong
  output.
- Rule: Lifecycle honesty — neutral `LifecycleState` transitions map
  CLAP init/activate/start/stop/deactivate/destroy; protocol violations
  (process before start, illegal transitions) are rejected, not
  silently repaired.
- Rule: RT safety — the `process` path allocates nothing
  (`assert_no_alloc` under the `rt_` lane).
- Rule: Independence — plugin-side logic shares only the `clack-*`
  binding layer and neutral contracts with the adapter; no
  `ruviform-plugin-clap` internals are reused.

## Examples

- Example: 64-frame stereo block, inputs `0.5` (L) / `-0.25` (R),
  normalized param event `0.75` at offset 8 → L frames 0–7 `0.5`,
  frames 8–63 `0.75`; R frames 0–7 `-0.25`, frames 8–63 `-0.375`;
  next block without events keeps ×1.5.
- Example: `params()` on a fresh instance reports `Gain` min `0.0`,
  max `2.0`, default `1.0`.
- Example: `process` invoked while lifecycle is `Activated` (not
  `Processing`) returns an error and leaves state unchanged.
- Example: in-place stereo block scales the shared input region in
  place (no aliased `&`/`&mut` construction).

## Guarantees

- A correct fixture passes all rules through the production
  `ClapLibrary` path with no fake plugin port.
- A host-side or plugin-side domain-mapping bug fails the golden
  vectors.
- Any allocation on the RT process path aborts the `rt_` test.
