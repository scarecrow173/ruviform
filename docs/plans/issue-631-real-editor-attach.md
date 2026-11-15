---
id: "PLAN-034Y3odXzCYwJRpyjtWZgK"
type: "plan"
status: "approved"
title: "Issue #631 real-editor-attach 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
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
    - "docs/specs/issue-631-real-editor-attach.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-real-editor-attach.md"
    - "docs/designs/issue-631-real-editor-attach.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-editor-attach 実装計画 Implementation Plan

## Goal

Upgrade the `real-editor-attach` case of the `rf631.plugin-dock-real`
scenario to load the `test_editor` fixture, observe the plugin's
real child HWND under the leased container HWND via query-only Win32,
and record detach convergence (`IsWindow(child) == 0`) — turning the
case from `Unsupported` into an honest `Pass` on interactive Windows.

## Scope

Single implementation stream, one slice of Issue #631. All changes
close inside `crates/ruviform-plugin-dock-test` plus the doc chain and
`evidence/rf631/`. Headless lane (`src/scenario.rs`,
`tests/console.rs`) is explicitly untouched.

## Upstream Documents

- `docs/specs/issue-631-real-editor-attach.md`
- `docs/designs/issue-631-real-editor-attach.md`
- `docs/specs/issue-631-real-window-dock.md` (predecessor lane)
- `docs/specs/issue-630-test-editor.md` (fixture contract)

## Design Inputs

- `docs/designs/issue-631-real-editor-attach.md`

## File Map

- `crates/ruviform-plugin-dock-test/Cargo.toml` — optional
  `windows-sys` under `[target.'cfg(windows)'.dependencies]` +
  `dep:windows-sys` in `real-dock`; rationale comment.
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` — main change:
  `load_plugin` descriptor selection, container-token step,
  `oschild` observation module (`#[cfg(windows)]` + stub), extended
  `attempt_real_attach`/`editor_attach_body` steps + verdict mapping,
  module-doc table update.
- `crates/ruviform-plugin-dock-test/src/lib.rs` — doc-table row for
  `real-editor-attach` if the table lives there.
- `crates/ruviform-plugin-dock-test/README.md` — real-lane row update.
- `crates/ruviform-plugin-dock-test/tests/` — only if report shape
  requires; console asserts must keep passing unchanged.
- `Cargo.lock` — regeneration only if the optional dep produces a
  diff.
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/
  issue-631-real-editor-attach.*` — this chain.
- `docs/*/README.md` indexes — regenerated via doc-maintenance only.
- `evidence/rf631/plugin-dock-real-scenario.jsonl`,
  `evidence/rf631/verification.md` — real-run record.

## Implementation Sequence

- [ ] `Cargo.toml`: add `windows-sys` optional dep (features:
  `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`,
  `Win32_System_Threading`) + `dep:windows-sys` in `real-dock`,
  with the rationale comment.
- [ ] `real_dock.rs` `load_plugin`: `d.name == "test_editor"`,
  remove `.or_else(first())`, missing-descriptor → honest
  `unavailable(.., prerequisite=false)` Fail.
- [ ] `real_dock.rs`: add `oschild` module — Windows impl
  (`EnumChildWindows` callback collecting child HWNDs as `u64`,
  `GetParent`/`IsWindow`/`IsWindowVisible`/`GetWindowThreadProcessId`/
  `GetCurrentThreadId`) + non-Windows stub; all `#[expect(unsafe_code)]`
  + SAFETY comments, query-only.
- [ ] `attempt_real_attach`: add `container_token` step after
  `lease_real_slot`; on `open Ok` run child observation steps
  (children enumerated, parent match, visible, live); after
  `session.close` record close result, `IsWindow(child)==0` per
  child, `session_closed_concluded`, drained events.
- [ ] `editor_attach_body` verdict mapping: `Open(Ok)` + all
  observations pass → `Pass`; `Open(Ok)` + mismatch → `Fail`;
  `Open(Err(Unsupported))` → `Unsupported`; setup/prereq → existing
  `Fail`/`NotRun` paths. Update module doc table + evidence tags.
- [ ] `src/lib.rs`/`README.md` real-lane rows; regenerate docs
  indexes via doc-maintenance.
- [ ] Verification matrix below; real run on interactive Windows;
  update `evidence/rf631/`; review-protocol review; commits; PR.

## Dependencies and Risks

- Depends on `test_editor` fixture merged at `a6d5735` (present in
  tree) and the predecessor lane at `f03390d0`/`95cf4e3b`.
- `windows-sys` 0.61 optional dep → possible `Cargo.lock` diff
  (regeneration only; noted in PR body).
- Interactive Windows session required for the real run; headless
  stays `environment_blocked`/`NotRun` — recorded honestly.
- Risk: `IsWindowVisible` ancestor-chain false negatives — recorded
  as `Fail` honestly, no masking retries.
- Risk: thread-affinity deviation could be mistaken for a defect —
  mitigated by explicit `observed.thread_affinity` + detail note.

## Task Breakdown

- T1 (this doc's task): the whole slice — single coherent change.

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check ruviform-plugin-dock-test` /
  `lint` / `test` (default features — headless non-regression)
- [ ] `mise run check ruviform-plugin-dock-test --features real-dock`
  / `lint` / `test` (`--locked` consistency)
- [ ] `mise run test-ui-plugin-dock` — headless verdict table
  unchanged
- [ ] `mise run check-ui-driver-boundary`
- [ ] `mise run test-ui-plugin-dock-real` — interactive Windows real
  run; `real-editor-attach` = `Pass` with OS observation in JSONL
- [ ] `mise exec -- prek run --all-files`

## Open Questions / Gaps

- Whether `Cargo.lock` actually diffs — resolved at implementation;
  either way the outcome is recorded.
- Exact step-row naming — finalized during implementation; the
  contract is "every observation lands as a pass/fail row in
  `observed.steps`".

## Self-Review

- [x] Every spec requirement maps to a task step.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
  consistent with the doc chain.

## Implementation Handoff

- Recommended mode: inline execution (single worker slice).
- Independent work streams: none.
- User approval required before dispatch: yes (orchestrated worker
  dispatch already approved via task spec).
- If approved: execute task-by-task with review checkpoints.

## Review Handoff

- Reviewer focus: verdict honesty (no fake Pass), headless non-
  regression, `windows-sys` scope minimality, unsafe confinement,
  thread-affinity recorded-not-claimed.
