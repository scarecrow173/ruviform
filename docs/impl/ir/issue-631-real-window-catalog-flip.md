---
id: "IMPL-034Z08gK0V0GGSoPCC5Vvl"
type: "impl"
status: "completed"
title: "Flip real-window close/dpi catalog rows to real verdicts"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/jsonl.rs"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-headless.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/verification.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-631-real-window-catalog-flip.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../discovery/issue-631-real-window-catalog-flip.md"
    - "../../specs/issue-631-real-window-catalog-flip.md"
    - "../../designs/issue-631-real-window-catalog-flip.md"
    - "../../plans/issue-631-real-window-catalog-flip.md"
    - "../../test-specs/issue-631-real-window-catalog-flip.md"
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

# Flip real-window close/dpi catalog rows to real verdicts

## Summary

Replaced the `Unsupported` verdict stubs for `real-window-close` and `real-window-dpi` with real
verdict logic driven by the `RealOsGuiTest` observation APIs delivered by the window-surface slice
(PR #709). Added deterministic `case_seed` and `receipts` fields to the JSONL projection (additive).
Catalog declarations in `real_catalog.rs` and `real_dock::real_entries` were updated in lockstep so
the feature-gated consistency tests still pass.

## Context

Follows the Issue #631 doc chain: discovery → spec → design → plan → task → test-spec. The window-surface
slice had already implemented `window_events`, `request_close`, `arm_close_response`, `respond_close`,
and `window_geometry`; this slice connects the catalog rows to those oracles. Close targets are
window B/C (sentinels keep window A alive for later cases; GPUI exits when the last window closes).

## Implementation

### Changes

- `real_dock.rs`: `real_window_close_case` now drives a 3-window session exercising veto, defer,
  deferred approve, and default approve close paths via `arm_close_response`/`request_close`/
  `respond_close`, asserting `CloseRequested`→`Destroyed` lifecycle transitions from `window_events`
  and confirming retired-window APIs fail explicitly and the sentinel window survives. Refactored into
  `close_body` plus `exercise_*` leg helpers to satisfy the 100-line function limit. `real_window_dpi_case`
  now polls `window_geometry` for physical px/scale-factor observations, counts stable reads, records
  resize/scale-factor transitions, and emits a SHA-256 digest of canonical geometry data; records
  `dpi_change_driven: false` honestly since no DPI-change verb exists. Module doc outcome table updated.
- `real_catalog.rs`: `real-window-close` `required` gained `MultiWindow`; `real-window-dpi` updated;
  doc comments aligned with the flipped verdicts.
- `jsonl.rs`: added additive `case_seed` (deterministic, case-scoped) and `receipts` fields to case
  rows; new unit tests cover seed determinism and field presence.
- Evidence JSONL files regenerated; `verification.md` gained a new slice entry (append-only history
  preserved).

### Decisions During Implementation

- Close case uses windows B/C rather than `ctx.window` because the GPUI loop exits when the last window
  closes; window A must remain as a sentinel for subsequent cases.
- DPI case passes on observed stable geometry and records that the case itself did not drive a DPI
  change (no API exists); `detail`/evidence states this explicitly.
- AC4 stretch implemented partially: `case_seed` and `receipts` were added; GPU info, semantic-tree
  serialization, and window-hash digest fields were left as follow-ups (lockfile-free scope).
- The implementation record was created after code changes began due to session resume; this is a
  process deviation recorded honestly rather than backdated.

## Related Experiments

### Adopted

None — the window-surface slice's lifecycle evidence (`evidence/rf631/real-window-surface.jsonl`) served
as the proven reference instead of a new experiment.

### Rejected

None.

## Validation

- `mise run test ruviform-plugin-dock-test` and `--features real-dock`: pass (incl.
  `real_catalog_matches_entries`, `real_slot_spec_declares_only_backend_supported_caps`, new jsonl tests).
- `mise run fmt-check`, `check` (default + real-dock), `lint` (default + real-dock, `-D warnings`): pass.
- `mise run test-ui-plugin-dock-real-headless`: pass — 21 rows `not_run` with `environment_blocked`,
  deterministic backend, no real session started.
- `mise run test-ui-plugin-dock-real` (interactive Windows desktop): 20 pass / 1 fail —
  `real-window-close` and `real-window-dpi` both pass; `real-dcomp-pixel` fails identically to the
  pre-existing baseline (known Issue #532 gate, unchanged).
- `mise run test-ui-plugin-dock-console`, `test-ui-plugin-dock-scenario`, `test-ui-plugin-dock-lab`,
  `test-ui-scenario-contract`, `check-ui-driver-boundary`: pass.
- `plugin-dock-real-scenario --blocked-probe`: 21 `not_run`, deterministic-backend, session start not
  attempted.
- DComp-disabled run (`GPUI_DISABLE_DIRECT_COMPOSITION=1`): consistent with baseline.
- `mise exec -- prek run --all-files`: pass.

## Risks

- `real-dcomp-pixel` remains `fail` (Issue #532) — intentionally not weakened; exit 1 on the real lane
  is expected until that gate resolves.
- DPI verdict covers observation only; mixed-DPI transition driving remains uncovered (no API).
- Close lifecycle timing relies on the armed-queue being consumed synchronously inside
  `CloseRequested` flush; caller-side `respond_close` arrives too late for veto/defer (documented in
  `real_window.rs`).

## Follow-ups

- Issue #532: DComp pixel oracle failure.
- #389-gated rows remain `Unsupported` (`real-window-ime`, `real-editor-resize`, `real-editor-redock`,
  `real-session-crash`, `real-session-hang`).
- Remaining AC4 fields (GPU info, semantic-tree serialization, window/plugin hash digest) deferred.
- Docs index regeneration left to Coordinator at integration.
