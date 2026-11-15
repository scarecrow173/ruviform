---
id: "TSPEC-034Y3opmTfAWO3534oKOoH"
type: "test-spec"
status: "approved"
title: "Issue #631 real-editor-attach テスト仕様（child HWND OS 観測・detach 収束・thread-affinity の判定）"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-631-real-editor-attach.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-631-real-editor-attach.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-editor-attach テスト仕様（child HWND OS 観測・detach 収束・thread-affinity の判定）

## Purpose

Pin down what the upgraded `real-editor-attach` case must prove —
and what it must never fake — so the `Unsupported → Pass` transition
rests on OS-observed facts, not inference. Retirable once the case's
evidence is consumed by Issue #631's remaining slices.

## Feature

`real-editor-attach` — real `test_editor` `IPlugView` attach/detach
under a real leased container HWND, observed by the OS.

## Rules

- **Descriptor honesty**: `load_plugin` resolves `test_editor` by
  name only; no fallback to `test_gain` or any other descriptor.
  Missing `test_editor` → `Fail`/`NotRun` per artifact taxonomy.
- **Attach is real**: `Pass` requires `session.open(Docked)` →
  `Ok(EditorCapabilities)` through the production
  `Vst3Plugin::open_gui` path (`createView` → `setFrame` →
  `attached`).
- **Child HWND observed by OS**: `EnumChildWindows(container)`
  finds ≥1 child; every observed child has
  `GetParent(child) == container`, `IsWindowVisible(child) != 0`,
  `IsWindow(child) != 0`.
- **Detach converges**: after `session.close`, every recorded
  child satisfies `IsWindow(child) == 0`;
  `session_closed_concluded` holds; drained events contain
  `Attached{mode: Docked}` and `Closed`.
- **Mismatch fails**: any observation contradicting the contract
  (no child, wrong parent, invisible, alive-after-close, missing
  events) → `Fail`, never `Pass`.
- **Defined refusal stays Unsupported**: `EditorOpError::
  Unsupported` from `session.open` → `Unsupported`.
- **Prerequisite honesty**: missing artifact / non-interactive
  session / non-Windows → `NotRun`/`environment_blocked`/
  `Unsupported` — never a fabricated `Pass`.
- **Thread-affinity recorded, not claimed**: caller/container/child
  owning thread ids land in `observed`; the D1-contract deviation
  is documented in `detail`/docs but is not a verdict input.
- **Headless non-regression**: `src/scenario.rs` `editor-attach`
  stays `Unsupported` on `test_gain`; `tests/console.rs` asserts
  unchanged and passing.
- **Evidence completeness**: `observed.steps` carries a pass/fail
  row per observation; `plugin_artifact` identifies the resolved
  `test_editor` descriptor (id/name/vendor/version/SHA-256).

## Examples

- **Example (Pass path)**: Given an interactive Windows session
  and the `test_editor` artifact present, when
  `plugin-dock-real-scenario --features real-dock` runs, then
  `real-editor-attach` reports `Pass` and its JSONL row's
  `observed.steps` shows container-token, open, child-enumerated,
  parent-match, visible, close, destroyed, concluded, events rows
  all `pass`, with concrete HWND/thread-id values.
- **Example (Fail on mismatch)**: If `session.open` returns `Ok`
  but `EnumChildWindows` finds no child (or a child with wrong
  parent / still alive after close), the verdict is `Fail` and the
  failing step row names the violated invariant.
- **Example (Unsupported preserved)**: If `test_editor`'s
  `isPlatformTypeSupported("HWND")`/`attached` declines with a
  defined refusal, verdict is `Unsupported` — not `Fail`, not
  `Pass`.
- **Example (headless)**: `plugin-dock-scenario` (default
  features) still reports `editor-attach` = `Unsupported` on
  `test_gain`; `run_artifact_then_editor_attach` console asserts
  keep passing.
- **Example (non-interactive)**: `test-ui-plugin-dock-real` in a
  headless session records `environment_blocked`/`NotRun` for all
  real cases; exit code stays 0 (no `Fail`).

## Guarantees

- A `Pass` verdict is only emitted when every OS observation
  agreed at run time — the evidence row contains the actual HWND
  values, parent/visibility/liveness booleans, thread ids, and
  drained events that produced it.
- `windows-sys` usage is confined to `#[cfg(windows)]` query-only
  code inside the crate; no Win32 type or raw handle escapes into
  the public API or evidence schema.
- The default feature set and headless lane dependency tree are
  unchanged by the optional dep.

## Non-goals

- Pixel/compositing verification of the child HWND (DComp #532
  unresolved — not claimed).
- Input/focus/resize/re-dock behavior on the child HWND.
- Strict D1 thread-affinity conformance (open on the pump-owning
  thread) — deviation is recorded, not fixed, in this slice.
- macOS/Linux GUI runs, `*-msrv`/`*-full` lanes.

## Risk

If these guarantees are dropped, the lane could report `Pass`
without a real child HWND ever existing — exactly the fake-pass
failure mode `POLICIES.md` exists to prevent, and the reason this
slice exists at all.
