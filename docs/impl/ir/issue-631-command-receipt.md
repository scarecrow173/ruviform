---
id: "IMPL-Pe62sVfdQG9xKm3wNz8cRl"
type: "impl"
status: "completed"
title: "Add real-command-receipt case to the real dock lane"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/adr/issue-631-command-receipt.md"
      - type: "file"
        path: "docs/designs/issue-631-command-receipt.md"
      - type: "file"
        path: "docs/discovery/issue-631-command-receipt.md"
      - type: "file"
        path: "docs/impl/exp/issue-631-command-receipt-exp.jsonl"
      - type: "file"
        path: "docs/impl/exp/issue-631-command-receipt.md"
      - type: "file"
        path: "docs/plans/issue-631-command-receipt.md"
      - type: "file"
        path: "docs/specs/issue-631-command-receipt.md"
      - type: "file"
        path: "docs/tasks/issue-631-command-receipt.md"
      - type: "file"
        path: "docs/test-specs/issue-631-command-receipt.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
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
    - "docs/tasks/issue-631-command-receipt.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../discovery/issue-631-command-receipt.md"
    - "../../specs/issue-631-command-receipt.md"
    - "../../designs/issue-631-command-receipt.md"
    - "../../plans/issue-631-command-receipt.md"
    - "../../test-specs/issue-631-command-receipt.md"
    - "../../adr/issue-631-command-receipt.md"
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

# Add real-command-receipt case to the real dock lane

## Summary

Added the `real-command-receipt` catalog row and executable case to
the `rf631.plugin-dock-real` lane, closing the AC1 Command→Receipt
link gap and populating AC4 `receipts` evidence for the first time.
The session gained a fourth window (D, `ViewInstanceId` 634) hosting
the existing `sv_fader_dock_fixture` — a command-minting fader view —
because the dock spec carries no command bindings. The case drives
Begin→Preview→Commit gestures, correlates minted `command_id`s, posts
real `Event::Result`/`Event::Notify` through `post_event`, observes
`CommandPhase` transitions (Applied→Observed, Rejected), and records
the `CommandReceipt::from_result`-decoded receipts on the report.
`real-window-close`'s terminal assertion was generalized from
"sentinel A only" to "every non-target window survives" to account
for window D.

## Context

Follows the Issue #631 doc chain: discovery → spec → design → plan →
task → test-spec → adr → impl. All driver-neutral primitives
(`dispatch_input`/`drain_intents`/`post_event`/`observe`/`settle`),
the `MintedIntent` contract, `CommandReceipt::from_result`, and the
`ScenarioOutcome.receipts`→JSONL wiring already existed (the
catalog-flip slice wired `receipts`/`case_seed` into `case_line`);
this slice only adds the catalog row, the case, the fourth mount, and
the close-case assertion fix. The precedent
`post_event_routes_receipt_and_projection_to_binding` in
`scenario_contract.rs` validated the same flow on the fake driver.

## Implementation

### Changes

- `real_catalog.rs`: added `real-command-receipt` row
  (`required = SemanticAction`) after `real-os-input`; updated the
  module doc from twenty-one to twenty-two cases and documented the
  new row's verdict contract.
- `real_dock.rs`: added `window_d`/`view_d` to `RealDockContext`,
  mounted window D (`ViewInstanceId` 634, `sv_fader_dock_fixture`,
  position (4350,80)) as the fourth `RealWindowMount`, added the
  `real-command-receipt` `ScenarioEntry` (case 10, before
  `real-window-dpi`/`real-window-close`), implemented
  `command_receipt_body` with helpers `commit_control_id`/
  `commit_for_gesture`, populated `outcome.receipts` from
  `CommandReceipt::from_result` decodes of the actual posted events,
  and generalized `close_body`'s terminal check to assert all
  non-target windows survive with no close lifecycle (step names
  `survivors_only`/`survivors_isolated`).
- `tests/console.rs`: `--list` row count 29→30 and the typed real-row
  array gained `real-command-receipt` (8 headless + 22 real).
- Evidence JSONL regenerated: `plugin-dock-real-scenario.jsonl`
  (interactive run), `plugin-dock-real-headless.jsonl`
  (blocked-probe), `plugin-dock-console.jsonl`,
  `plugin-dock-scenario.jsonl`.
- `verification.md` gained a slice record (see below).

### Decisions During Implementation

- Window D (not spec-swap on A, not reusing B/C) hosts the fader
  fixture — recorded as `docs/adr/issue-631-command-receipt.md`.
  The dock spec mounts no command bindings, so a dedicated window
  was the only non-invasive mount form.
- `command_id` correlation uses the minted `MintedIntent`'s
  `command_id` verbatim as `Event::Result.id` — never display text
  or position. `expect_receipt` + `CommandIntent::Commit` identify
  the receipt-expecting intent per gesture.
- Receipts are decoded with `CommandReceipt::from_result` on the
  exact `Event` objects posted to the view — the recorded receipt is
  byte-identical to what `deliver_event` applied, never a stand-in.
- The rejected leg uses `Event::result_err` (decodes to
  `status:"rejected"` with the error detail), proving the terminal
  failure path settles `CommandPhase::Rejected` while the last
  authoritative projection (`observed.revision=2`) stays bound.
- `close_body` survivor generalization: instead of hardcoding
  `[window_a]`, the case captures `windows()` at entry, removes the
  two close targets, and asserts the remainder — strictly stronger
  (every non-target is verified lifecycle-clean, not just A).
- First interactive run exposed the close-case regression
  (`expected only sentinel window A to remain, got [w1, w4]`) —
  fixed by the survivor generalization and re-verified.

## Related Experiments

### Adopted

None new — `evidence/rf631/real-window-surface.jsonl` and the
catalog-flip slice's session/lifecycle evidence remained the
reference; `scenario_contract.rs`'s receipt/projection test was the
flow precedent.

### Rejected

None.

## Validation

- `mise run fmt-check`: pass.
- `mise run check`/`lint`/`test ruviform-plugin-dock-test` for
  default, `real-dock`, and `lab-gui` features: pass
  (`-D warnings` clean; `#[expect(clippy::too_many_lines)]` used per
  file convention on `command_receipt_body`, `close_body`, and
  `run_all_with_pixel_artifacts`).
- `mise run test-ui-plugin-dock-console`: pass — `--list` emits 30
  rows (8 headless + 22 real) with `real-command-receipt` projected.
- `mise run test-ui-plugin-dock`: pass (headless scenario lane,
  non-regression).
- `mise run test-ui-plugin-dock-real-headless`: pass — 22 case lines,
  all `not_run` + `environment_blocked` + `deterministic-backend`,
  exit 0.
- `mise run test-ui-plugin-dock-real` (interactive Windows desktop):
  15 pass / 1 fail / 6 unsupported — `real-command-receipt` **pass**
  with 2 populated receipts (applied `revision:1` + rejected with
  detail), `real-window-close` **pass** after the survivor
  generalization, `real-dcomp-pixel` **fail** (known Issue #532 DComp
  occlusion, intentionally preserved). Lane exit 1 is the expected
  honest signal.
- `mise run test-ui-scenario-contract`: pass (fake 17 + gpui 7).
- `mise run check-ui-driver-boundary`: pass.
- `mise exec -- prek run --all-files`: pass.
- `mise run test-ui-plugin-dock-lab`: window launched and exited
  cleanly (exit 0) on WM_CLOSE; the interactive Run-all→Quit
  dogfooding step requires an operator — catalog projection verified
  via console `--list` and `catalog_rows_cover_headless_and_real`.

## Risks

- `real-dcomp-pixel` remains `fail` (Issue #532) — intentionally not
  weakened; real-lane exit 1 is expected until that gate resolves.
- The fader gesture depends on real OS timing; bounded `settle`
  waits plus honest `Fail` cover mint/observe failures (stable in
  the recorded run).
- `test-ui-plugin-dock-lab` interactive dogfooding (Run all → Quit)
  was not executed unattended — computer-use is unavailable on this
  host (accessibility provider blocked); recorded as a remaining
  manual step, not a fabricated pass.

## Follow-ups

- Issue #532: DComp pixel oracle failure (keeps `real-dcomp-pixel`
  at `fail`).
- #389/#630-gated rows remain `Unsupported` (`real-window-ime`,
  `real-editor-resize`, `real-editor-redock`, `real-session-crash`,
  `real-session-hang`).
- HEADLESS_MATRIX-side command-receipt row deferred until the
  ruviform#725 lease on `tests/plugin_dock.rs`/`Cargo.toml` clears.
- Remaining AC4 fields (GPU info, full semantic tree, window/plugin
  hash) deferred — no lockfile-free API surface yet.
- Consumer projections remain gated on synvelis#114/#176 and
  audio-lens#127.
- Docs index regeneration left to Coordinator at integration.
