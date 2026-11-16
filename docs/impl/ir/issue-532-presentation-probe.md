---
id: "IMPL-034YC0Zh82xIExSOVai1Rg"
type: "impl"
status: "in-progress"
title: "Issue 532 presentation probe slice"
created: "2026-11-16"
updated: "2026-11-16"
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
    - "docs/tasks/issue-532-presentation-probe.md"
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
    adopted:
      - "../exp/issue-532-presentation-probe-exp.jsonl"
    rejected: []
---

# Issue 532 presentation probe slice

## Summary

Added the Issue #532 presentation-mechanism discrimination probe to
`crates/ruviform-plugin-dock-test` (feature `real-dock`). A new
real-lane case `real-dcomp-present` runs, while the editor child is
attached and its lease is live, a read-only observation family:
top-level window inventory (`ospres`), presentation attributes
(DWM cloaked / window region / update rect / layered attributes),
`PrintWindow` flag variants, window-DC captures, screen `BitBlt`,
and quantized screen-vs-suspect pixel fingerprints. Catalog and
console projections moved 14 → 15 real cases (23 total rows).

## Context

Follows `docs/tasks/issue-532-presentation-probe.md`, which
implements the approved `docs/designs/issue-532-presentation-probe.md`
under `docs/specs/` / `docs/plans/` / `docs/test-specs/`. It
discriminates the three residual hypotheses left open by the
`foreground-probe` slice for Issue #532 (kept open): a
hit-test-transparent covering surface, the child HWND's own
presentation/redirection path, or a screen-capture blind spot. The
probe is diagnostic only — no compositor fix, no fix selection,
no production code changes.

## Implementation

### Changes

- `crates/ruviform-plugin-dock-test/src/ospix.rs`: added
  `capture_print_window_flags(hwnd, flags)` supporting
  `PW_CLIENTONLY` / `PW_RENDERFULLCONTENT` combinations; existing
  `capture_print_window` kept as a wrapper with unchanged backend
  label and behavior.
- `crates/ruviform-plugin-dock-test/src/ospres.rs` (new): query-only
  Windows module — `inventory()` (`EnumWindows`, z-order/class/
  rect/visible/alive/disabled/exstyle/owner/pid, `INVENTORY_CAP=256`
  + truncation flag) and `attributes()` (`DWMWA_CLOAKED`,
  `GetWindowRgn`/`GetRgnBox`, `GetUpdateRect`,
  `GetLayeredWindowAttributes`, each degrading independently to
  `None` + error). Plain values only; HWND narrowed to `u64`;
  non-Windows honest stub.
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`:
  `real-dcomp-present` case after `real-dcomp-foreground`;
  `PresentationProbeOutcome`/`PresentationVerification`;
  `presentation_probe_children`/`probe_one_child_presentation`;
  cover-candidate selection (`COVER_CAP=16`), capture recording
  (`record_present_capture`), fingerprint comparison
  (`fingerprint_match`/`top_colors`/`record_fingerprint`,
  `FINGERPRINT_CAP=8`). Verdict uses only evaluability, family
  completion, and lifecycle convergence — captures/hit-tests/
  fingerprints are `note`-level evidence, never verdict inputs.
- `real_catalog.rs`, `lib.rs`, `tests/console.rs`,
  `plugin-dock-console.rs`, `README.md`: 15-row real catalog,
  23-row console projection, docs updated.
- `docs/{discovery,specs,designs,plans,tasks,test-specs}/
  issue-532-presentation-probe.md`: doc chain.
- `evidence/rf532/presentation-probe.md`,
  `evidence/rf631/verification.md` append,
  `evidence/rf631/plugin-dock-real-{scenario,scenario-dcomp-disabled,
  headless}.jsonl`, `evidence/rf631/presentation-probe/*.png`:
  interactive-lane evidence.

### Decisions During Implementation

- Kept `osoccl`/`osmut` untouched: `osoccl::probe` facts feed only
  suspect selection; no mutation APIs were reused or added.
- `PW_RENDERFULLCONTENT` variants recorded as data even though they
  returned flat-dark rasters in practice — flag support is part of
  the observation contract, usefulness is evidence, not a gate.
- Inventory truncation and own-process absence are recorded
  honestly in `pres_inventory` (`truncated`, per-window
  `own_process`) rather than compensated by unbounded enumeration.
- Fingerprint suspects deduplicate hit-test HWNDs and cap at 8;
  `PrintWindow`-based suspect rasters are labeled with backend so
  Chromium-class windows' unreliable `PrintWindow` output is
  distinguishable in evidence.

## Related Experiments

### Adopted

- `../exp/issue-532-presentation-probe-exp.jsonl` — implementation and
  both interactive runs; discrimination recorded in
  `evidence/rf532/presentation-probe.md`.

### Rejected

- None — no alternative approaches were implemented.

## Validation

- `cargo fmt --all -- --check`: pass.
- `mise run check/lint/test ruviform-plugin-dock-test` (default):
  pass (18 tests).
- Same lanes with `--features real-dock`: pass.
- `mise run test-ui-plugin-dock`, `test-ui-plugin-dock-console`,
  `test-ui-scenario-contract` (fake+gpui),
  `check-ui-driver-boundary`, `prek run --all-files`: pass.
- `plugin-dock-real-scenario --blocked-probe`: pass — 15 cases all
  `not_run`, fail-closed headless preserved.
- `mise run test-ui-plugin-dock-real` (baseline) and
  `GPUI_DISABLE_DIRECT_COMPOSITION=1` run: exit 1 by design
  (`real-dcomp-pixel` honest `Fail`); `real-dcomp-present` = `pass`
  in both; `real-dcomp-pixel`/`real-dcomp-foreground` verdicts
  unchanged.

## Risks

- The 256-entry inventory cap can exclude own-process windows on
  busy desktops (observed: 0 own-process rows of 449 total); cover
  conclusions are bounded to the recorded prefix.
- `PrintWindow` results vary by paint state and are unreliable for
  Chromium-class windows; recorded as labeled evidence only.
- Topmost-stage screen captures run without a settle delay, so a
  pre-paint frame cannot be excluded — flagged as the key residual
  ambiguity in the discrimination record.

## Follow-ups

- Settle-aware re-observation after `HWND_TOPMOST` (few frames)
  to separate pre-paint frames from genuine non-presentation.
- Uncovered-baseline environment (foreground the test window or
  clear the covering foreign window) for a clean screen oracle.
- Consider own-process-prioritized or raised inventory cap.
- Issue #532 remains open; compositor fix selection is out of
  scope for this slice.

## Review Round (post-implementation)

Two independent reviewers audited the slice against spec/design/
test-spec (reviewer A: requirements conformance; reviewer B:
adversarial robustness). No P0/P1 findings. Confirmed findings
fixed before merge:

- **Unbounded cross-process `PrintWindow`** (B, P2): suspect
  rasters now skip `IsHungAppWindow`-reported hung targets
  (`ospix::window_hung`) — `WM_PRINT`/`SendMessage` would
  otherwise block the lane on a wedged foreign window. Recorded
  as `skipped: hung_app_window`, never hidden.
- **Coordinate-space ambiguity** (B, P2): `pres_attributes`
  region/update rects now carry `bounds_space: "window"` /
  `rect_space: "client"` per the approved design's
  disambiguation requirement.
- **`pid`/`owner_pid` fabricated `Some(0)`** (A+B, P3):
  `GetWindowThreadProcessId` thread-id return is now checked —
  query failure records `None`, matching field docs.
- **`evaluated` baseline missing root-resolvable precondition**
  (A, P3): `probe_one_child_presentation` returns
  `ChildProbeResult::NotEvaluable` when `facts.root` is `None`;
  the case reports `NotRun`, never a degraded pass.
- **`families` omitted `cover`** (B, P3): family list now
  `["inventory","attributes","cover","capture_variants",
  "screen","fingerprint"]`.
- **Suspect role mislabeling** (B, P3): `suspect_role` maps any
  suspect HWND to its chain identity (`root`/`container`) before
  falling back to `hit_test`/`cover_candidate` — one HWND never
  carries two labels.
- **`UpdateFacts` had no error channel** (A, P4→fixed):
  `SetLastError`/`GetLastError` around `GetUpdateRect` now
  distinguishes a dead handle from an empty update region;
  `error` field added and serialized.
- **`SelectObject` unchecked** (B, P3): `dib_capture` now errors
  out on selection failure instead of rendering into the wrong
  surface (fabricated-pixels risk closed).
- **Doc honesty**: `ospix`/`ospres` module docs and
  `evidence/rf532/presentation-probe.md` no longer claim "no
  message sending" — `PrintWindow`'s `WM_PRINT` is the documented
  exception; design/test-spec step-name drift corrected
  (`pres_capture`, global `pres_inventory`); `backend` suffix
  design text updated to the implemented flags-field mechanism;
  exp record's `osoccl`→`osmut` module slip fixed.
- **Coverage**: 5 unit tests added for the pure helpers
  (`rect_intersection`, `rect_contains`, `color_bin`,
  `fingerprint_match`, `top_colors`) — 8 real-dock unit tests
  total, all pass.

Accepted residuals (P3, documented not fixed): `IsHungAppWindow`
is a snapshot — a window can hang between check and capture;
non-Windows `region`/`update` `Option` shape is platform-level
only; inventory truncation limits already recorded.

Post-review interactive lanes were re-run so the committed
evidence JSONL/PNG artifacts reflect the final code (new
step-row fields: `bounds_space`, `rect_space`, `update.error`,
`families` includes `cover`).
