---
id: "IMPL-Qt95vZbeRG2yPl8uXy0kNf"
type: "impl"
status: "completed"
title: "Add command-receipt case to the headless dock lane"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/adr/issue-631-headless-command-receipt.md"
      - type: "file"
        path: "docs/designs/issue-631-headless-command-receipt.md"
      - type: "file"
        path: "docs/discovery/issue-631-headless-command-receipt.md"
      - type: "file"
        path: "docs/impl/exp/issue-631-headless-command-receipt-exp.jsonl"
      - type: "file"
        path: "docs/impl/exp/issue-631-headless-command-receipt.md"
      - type: "file"
        path: "docs/plans/issue-631-headless-command-receipt.md"
      - type: "file"
        path: "docs/specs/issue-631-headless-command-receipt.md"
      - type: "file"
        path: "docs/tasks/issue-631-headless-command-receipt.md"
      - type: "file"
        path: "docs/test-specs/issue-631-headless-command-receipt.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/plugin_dock.rs"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console-list-include-real.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console-list.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/verification.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-631-headless-command-receipt.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../discovery/issue-631-headless-command-receipt.md"
    - "../../specs/issue-631-headless-command-receipt.md"
    - "../../designs/issue-631-headless-command-receipt.md"
    - "../../plans/issue-631-headless-command-receipt.md"
    - "../../test-specs/issue-631-headless-command-receipt.md"
    - "../../adr/issue-631-headless-command-receipt.md"
    - "../../adr/issue-631-command-receipt.md"
    - "../../impl/ir/issue-631-command-receipt.md"
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

# Add command-receipt case to the headless dock lane

## Summary

Added the `command-receipt` catalog row and executable case to
the `rf631.plugin-dock` HEADLESS_MATRIX Fake binding, closing the
headless side of the AC1 Command→Receipt link and populating AC4
`receipts` evidence for the first time on the headless lane. The
case mounts the existing `sv_fader_dock_fixture` on `FakeGuiTest`,
drives two `Begin → Preview → Commit` gestures, correlates the
minted `command_id`s, posts real `Event::Result`/`Event::Notify`
through `post_event`, observes `CommandPhase` transitions
(Applied→Observed, Rejected), and records the
`CommandReceipt::from_result`-decoded receipts on the report.
Catalog grew 8→9 cells; console projection 30→31 rows. This is
the headless counterpart of the real lane's
`real-command-receipt` (PR `ruviform#748`).

## Context

Follows the Issue #631 headless doc chain: discovery → spec →
design → plan → task → test-spec → adr → impl. All driver-neutral
primitives (`dispatch_input`/`drain_intents`/`post_event`/
`observe`/`settle`), the `MintedIntent` contract,
`CommandReceipt::from_result`, `ScenarioOutcome.receipts`→JSONL
wiring, and the `sv_fader_dock_fixture` already existed; Issue
`ruviform#725`'s lease on `tests/plugin_dock.rs` cleared
(2026-11-17) making the count follow-ups possible. The real lane's
`command_receipt_body` is the flow reference but lives under
`#[cfg(feature = "real-dock")]`, so the same-shaped body was
implemented locally in `scenario.rs`.

## Implementation

### Changes

- `scenario.rs`: module doc eight→nine + table row; imports for
  `ruviform_ui::behavior`/`ipc`/`view` and `MintedIntent`/
  `ScenarioError`/`parity_gesture_scenario`/`sv_fader_dock_fixture`;
  `command_receipt_extras` claim (`semantic-command-receipt`);
  `command-receipt` `ScenarioEntry` (required=`SemanticAction`
  only) after `view-mount` in the Fake binding; new case section
  with `step`/`steps_outcome`/`component_snapshot_json`/
  `commit_for_gesture`/`commit_control_id` helpers,
  `command_receipt_body`, and `command_receipt_case`.
- `tests/plugin_dock.rs`: cells/projections/reports 8→9;
  `assert_command_receipt` helper asserting Pass + 2 receipts
  (Applied + Rejected statuses); fail-closed test asserts the
  artifact-independent case still Passes.
- `tests/console.rs`: `HEADLESS_CASES` 9 entries in catalog
  order; `--list` rows 8→9; `--include-real` rows 30→31 (skip 9);
  `--driver fake` selects 2 cells; `--scenario` selects 9.
- `README.md`: case table row; "eight"→"nine"; stale
  "twenty-one"→"twenty-two" real rows.
- Evidence JSONL regenerated: `plugin-dock-scenario.jsonl` (9
  cases), `plugin-dock-console.jsonl`,
  `plugin-dock-console-list.jsonl` (9 rows),
  `plugin-dock-console-list-include-real.jsonl` (31 rows).
- `verification.md` gained a slice record.

### Decisions During Implementation

- `command_id` correlation uses the minted `MintedIntent`'s
  `command_id` verbatim as `Event::Result.id` — never display
  text, array position, or synthetic values.
  `expect_receipt` + `CommandIntent::Commit` identify the
  receipt-expecting intent per gesture.
- Receipts are decoded with `CommandReceipt::from_result` on the
  exact `Event` objects posted to the view — the recorded receipt
  is byte-identical to what `deliver_event` applied.
- The rejected leg uses `Event::result_err` (decodes to
  `status:"rejected"` with the error detail), proving the terminal
  failure path settles `CommandPhase::Rejected`.
- Helpers duplicated from `real_dock.rs` rather than shared:
  the real-lane helpers are private under a `cfg` feature the
  headless build never compiles, and editing `real_dock.rs` is
  outside this slice's lease (recorded in the ADR).
- `step`'s unit-result closure uses `|()|` not `|_|` — clippy
  `ignored_unit_patterns` under `-D warnings`.
- `fixture_run_reports_honest_verdicts` exceeded clippy
  `too_many_lines` after the receipt assertions — extracted
  `assert_command_receipt` helper instead of an `#[expect]`.

## Related Experiments

### Adopted

- `scenario_contract.rs`'s
  `post_event_routes_receipt_and_projection_to_binding` remained
  the Fake-adapter flow precedent; the real lane's
  `command_receipt_body` remained the structural reference.

### Rejected

None.

## Validation

- `mise run fmt-check`: pass.
- `mise run check`/`lint`/`test ruviform-plugin-dock-test` for
  default, `real-dock`, and `lab-gui` features: pass
  (`-D warnings` clean; `#[expect(clippy::too_many_lines)]` on
  `command_receipt_body` per file convention).
- `mise run test-ui-plugin-dock`: pass — `command-receipt` **pass**
  with 2 populated receipts (applied `revision:1` + rejected with
  detail `consumer refused the commit`), headless `receipts`
  non-empty for the first time.
- `mise run test-ui-plugin-dock-console`: pass — `--list` emits 9
  headless rows; `--run` writes 9 case rows under lane
  `plugin-dock-console`.
- `mise run test-ui-plugin-dock-real-headless`: pass (exit 0) —
  22 real rows all `not_run` + `environment_blocked`; the
  regenerated out-of-scope JSONL was restored unchanged.
- `mise run test-ui-scenario-contract`: pass (fake 17 + gpui 7).
- `mise run check-ui-driver-boundary`: pass.
- `mise exec -- prek run --all-files`: pass.

## Risks

- The Fake receipt flow is deterministic contract evidence only —
  it never substitutes for real-window/GUI-L2 evidence; the real
  lane's `real-command-receipt` remains the OS-level record.
- Helper duplication across the cfg boundary is a known,
  documented cost (ADR) — a future shared module could consolidate
  if a third lane needs the same step/receipt helpers.

## Follow-ups

- Docs index regeneration left to Coordinator at integration.
- Issue #532 (`real-dcomp-pixel` Fail), #389-gated rows, AC4
  remaining fields, and consumer projections remain as recorded
  in `evidence/rf631/verification.md`.
