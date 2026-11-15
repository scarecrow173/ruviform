---
id: "TASK-034Y3oeCjY3yeLsdZlZF32"
type: "task"
status: "in-progress"
title: "Issue #631 real-editor-attach — test_editor 実 IPlugView child HWND attach/detach 実走"
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
    - "docs/plans/issue-631-real-editor-attach.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-real-editor-attach.md"
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
---

# Issue #631 real-editor-attach — test_editor 実 IPlugView child HWND attach/detach 実走

## Work

Upgrade the `real-editor-attach` case in
`crates/ruviform-plugin-dock-test/src/real_dock.rs` from `test_gain`
(`createView` null → `Unsupported`) to the `test_editor` fixture
(merged `a6d5735`), and add query-only Win32 observation of the
plugin's real child HWND: existence under the leased container,
`GetParent` match, visibility, post-detach `IsWindow(child) == 0`,
drained `Attached{Docked}`/`Closed` events, and thread-affinity
recording. Verdict mapping: `Pass` only on full agreement;
mismatch → `Fail`; defined refusal → `Unsupported`; missing
prereq → `NotRun`.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-631-real-editor-attach.md`
- `docs/specs/issue-631-real-editor-attach.md`
- `docs/designs/issue-631-real-editor-attach.md`
- `docs/specs/issue-631-real-window-dock.md` (predecessor lane)
- `docs/specs/issue-630-test-editor.md` (fixture contract)

## Dependencies

- `test_editor` fixture at `a6d5735` (present in tree).
- Predecessor real-window lane (`real_dock.rs` skeleton, slot ops,
  `native_container_token`, `lease_real_slot`, `attempt_real_attach`).
- `windows-sys` 0.61 optional dep — approved by user (2026-11-15).

## Affected Files

- `crates/ruviform-plugin-dock-test/Cargo.toml`
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
- `crates/ruviform-plugin-dock-test/src/lib.rs` (doc row only)
- `crates/ruviform-plugin-dock-test/README.md` (real-lane row)
- `crates/ruviform-plugin-dock-test/tests/` (only if needed)
- `Cargo.lock` (regeneration only, if it diffs)
- `docs/*/issue-631-real-editor-attach.*`, `docs/*/README.md` indexes
- `evidence/rf631/plugin-dock-real-scenario.jsonl`,
  `evidence/rf631/verification.md`

## Done When

- [ ] `load_plugin` selects `test_editor` by name; silent fallback
  removed; missing descriptor → honest `Fail`/`NotRun`.
- [ ] `real-editor-attach` = `Pass` on interactive Windows with
  child HWND observed (parent match, visible, live), destroyed
  after `session.close` (`IsWindow == 0`), events
  `[Attached{Docked}, Closed]` drained, thread ids recorded.
- [ ] Headless lane unchanged: `editor-attach` stays `Unsupported`,
  `tests/console.rs` non-regression asserts pass.
- [ ] `windows-sys` optional/Windows-targeted; default features and
  headless dep tree unchanged.
- [ ] Evidence + doc chain complete; indexes regenerated.

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check/lint/test ruviform-plugin-dock-test` (default)
- [ ] `mise run check/lint/test ruviform-plugin-dock-test
  --features real-dock`
- [ ] `mise run test-ui-plugin-dock` (headless non-regression)
- [ ] `mise run check-ui-driver-boundary`
- [ ] `mise run test-ui-plugin-dock-real` (interactive Windows)
- [ ] `mise exec -- prek run --all-files`
