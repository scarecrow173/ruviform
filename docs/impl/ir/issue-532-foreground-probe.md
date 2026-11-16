---
id: "IMPL-034Yy34gIfucX5jGQYgsJ6"
type: "impl"
status: "in-progress"
title: "Issue 532 foreground probe mutation slice"
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
    - "docs/tasks/issue-532-foreground-probe.md"
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
      - "../exp/issue-532-foreground-probe-exp.jsonl"
    rejected: []
---

# Issue 532 foreground probe mutation slice

## Summary

Added the Issue #532 foreground/compositor mutation probe slice to
`crates/ruviform-plugin-dock-test` (feature `real-dock`). A new real-lane
case `real-dcomp-foreground` snapshots the GPUI root HWND reachable from
each observed plugin child via `GA_ROOT`, then applies staged mutations —
raise to `HWND_TOP` (`SetWindowPos`), `SetForegroundWindow`, topmost band
(`SetWindowPos`) — re-observing `WindowFromPoint` hit-tests and screen
green-pixel fraction after each stage, and restoring baseline state before
the editor child detaches. All Win32 mutation lives in the new
`src/osmut.rs` module behind plain-value records; `osoccl`/`ospix` remain
query-only. Catalog and console projections moved 13 → 14 cases.

## Context

Follows `docs/tasks/issue-532-foreground-probe.md`, which implements the
approved `docs/designs/issue-532-foreground-probe.md` under
`docs/specs/issue-532-foreground-probe.md` / `docs/plans/` /
`docs/test-specs/`. It bisects the unresolved GPUI 0.2.2 Windows
DirectComposition occlusion reported in Issue #532 (left open): the prior
query-only `dcomp-occlusion` slice established real HWND-level coverage by
an external `Chrome_RenderWidgetHostHWND` but could not separate
compositor-internal DComp occlusion from external stacking.

## Implementation

### Changes

- `crates/ruviform-plugin-dock-test/src/osmut.rs` (new): `RootSnapshot`,
  `MutationOutcome`, `RestoreOutcome` plain-value records; Windows impl
  (`observe`, `raise_to_top`, `set_foreground`, `set_topmost`, `restore`)
  with bounded z-order walk, `GetLastError` capture on refused calls, and
  topmost-band restore verification; non-Windows honest stub.
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`: `real-dcomp-foreground`
  entry after `real-dcomp-pixel`; `ProbeOutcome`, `ForegroundVerification`,
  snapshot/mutation/restore JSON helpers, `record_foreground_observation`,
  `observe_all_children`, `record_apply_row`, `run_probe_stages`,
  `foreground_probe_children`, `foreground_extras`; catalog/entry counts
  13 → 14.
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`: metadata row for
  the new case.
- `crates/ruviform-plugin-dock-test/src/lib.rs`: `mod osmut;` under
  `feature = "real-dock"`.
- `crates/ruviform-plugin-dock-test/tests/console.rs`, `README.md`,
  `Cargo.toml` comments: case-list/count/doc updates.

### Decisions During Implementation

- `SetWindowPos` z-order refusal on the process's own root is treated as
  anomalous → `fail` step and probe failure; `SetForegroundWindow`
  foreground-lock refusal is spec-expected and recorded as `note` data,
  never a verdict input.
- `WindowFromPoint` answers and screen green fraction are recorded per
  stage as bisect evidence only; the case verdict depends on probe
  execution, required-mutation success, restore verification, and
  lifecycle — never on whether green pixels appeared.
- Restore guarantees only the topmost band (`topmost_restored` verified);
  z-order and foreground restoration are best-effort and reported as
  `zorder_before/after` / `foreground_before/final` without claiming
  completeness Windows cannot guarantee.
- `osmut` exposes only `u64`/`Option`/plain fields across the module
  boundary; HWNDs never leak as Win32 types; unsafe confined to FFI
  wrappers with `#[expect(unsafe_code)]` + SAFETY comments.
- `real-dcomp-pixel` semantics unchanged — it remains the authoritative
  screen-pixel `Fail` under the known occlusion.
- Clippy pedantic `-D warnings`: `#[expect(clippy::too_many_lines)]` on the
  declarative `real_entries` table (matches existing precedent) and a
  stage-helper extraction for `foreground_probe_children` rather than an
  expect, keeping the probe body readable.

## Related Experiments

### Adopted

- [issue-532-foreground-probe-exp.jsonl](../exp/issue-532-foreground-probe-exp.jsonl)
  — 実装中の試行・観測イベントログ（osmut 境界設計、clippy 対応の
  分岐、fmt reflow、review 反映、interactive bisect データ）。
  対応する人間可読の記録は
  [issue-532-foreground-probe.md](../exp/issue-532-foreground-probe.md)。

### Rejected

None.

## Validation

Run on this worktree (Windows, interactive desktop):

- `cargo fmt --all -- --check` — pass
- `mise run check|lint|test ruviform-plugin-dock-test` (default) — pass
- `mise run check|lint|test ruviform-plugin-dock-test --features real-dock` — pass
- `mise run test-ui-plugin-dock`, `test-ui-plugin-dock-console`,
  `test-ui-scenario-contract`, `check-ui-driver-boundary` — pass
- `mise exec -- prek run --all-files` — pass
- `plugin-dock-real-scenario --blocked-probe` — 14 rows, all
  `NotRun` + `environment_blocked`, exit 0
- Interactive `test-ui-plugin-dock-real` baseline and
  `GPUI_DISABLE_DIRECT_COMPOSITION=1` runs — see the
  `evidence/rf631/plugin-dock-real-scenario*.jsonl` lanes plus
  `evidence/rf532/foreground-probe-bisect.md` for recorded results.

## Risks

- `WindowFromPoint` cannot observe DirectComposition visuals/planes — a
  "self HWND + no green" result is strong external-stacking evidence but
  not direct compositor observation; the bisect conclusion must state
  this limit.
- Z-order/foreground restore is best-effort by OS design; a foreign
  window may legitimately hold the foreground afterwards.
- The probe mutates real window state on the user's desktop for a
  window of seconds per stage (longer with `--pixel-artifacts`); runs
  are opt-in manual lanes only.
- Accepted residuals from review (non-blocking): the root HWND is not
  re-validated for owner identity between baseline snapshot and the
  mutation stages — a mid-probe window destroy plus handle reuse could
  direct mutations at a foreign window (bounded: restore targets the
  same handle, lifecycle checks fail honestly afterward); there is no
  unwind guard around mutate→restore, so a panic in that span would
  skip restore — currently unreachable (no fallible/panicking calls on
  the path) and any escaped panic tears down our own session window
  rather than leaving foreign windows mutated.

## Follow-ups

- Record the bisect conclusion in `evidence/rf532/foreground-probe-bisect.md`
  and append the lane results to `evidence/rf631/verification.md`.
- Issue #532 stays open; the final compositor fix is a later slice.
- `evidence/rf631/plugin-dock-console-list-include-real.jsonl` predates
  the `real-dcomp-pixel` slice (12 rows) — regenerate in a follow-up if
  the console-list evidence is needed at 22 rows.
