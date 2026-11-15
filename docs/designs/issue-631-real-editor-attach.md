---
id: "DESIGN-034Y3o3I0foDX68pAnpDEx"
type: "design"
status: "approved"
title: "Issue #631 real-editor-attach 設計（test_editor descriptor 選択・child HWND OS 観測・thread-affinity 記録）"
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
    - "docs/specs/issue-631-real-editor-attach.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-editor-attach 設計（test_editor descriptor 選択・child HWND OS 観測・thread-affinity 記録）

## Context

The real-window dock lane (`src/real_dock.rs`, feature `real-dock`)
already drives the production chain `NativeSlot → lease_parent →
ParentSlotLease → redeem_native_lease → session.open(Docked)` on a
real GPUI window. `real-editor-attach` currently loads `test_gain`,
whose controller answers `createView` with null →
`HostError::Unsupported` → `EditorOpError::Unsupported` → honest
`Unsupported` verdict.

The `test_editor` fixture (Issue #630, merged `a6d5735`) ships a real
`IPlugView`: `createView("editor")` returns `TestEditorView`, whose
Windows `attached(parent, "HWND")` creates a `WS_CHILD | WS_VISIBLE |
WS_CLIPSIBLINGS` child under the supplied parent HWND and whose
`removed()` calls `DestroyWindow`.

This design upgrades the case to consume `test_editor` and adds
query-only Win32 observation of the child HWND lifecycle, all inside
`crates/ruviform-plugin-dock-test`.

## Scope

### In Scope

- `Cargo.toml`: optional `windows-sys` under
  `[target.'cfg(windows)'.dependencies]` + `dep:windows-sys` in the
  `real-dock` feature.
- `src/real_dock.rs`: descriptor selection, container-token step,
  child-HWND observation module (`#[cfg(windows)]` + non-Windows
  stub), detach-convergence steps, thread-affinity recording,
  verdict mapping.
- `src/lib.rs` / `README.md` real-lane doc rows.
- Evidence (`evidence/rf631/`) and doc chain.

### Out of Scope

- `src/scenario.rs` headless lane, `tests/console.rs` asserts,
  `mise.toml`, production crates, fixture crate, pixel/input/
  resize/focus/re-dock cases, non-Windows GUI.

## Components and Boundaries

- **`load_plugin`** — resolves the artifact once per lane run;
  descriptor selection becomes `d.name == "test_editor"` with no
  fallback. `PluginArtifactEvidence` then carries the
  `test_editor` identity.
- **`attempt_real_attach` / `editor_attach_body`** — the case body;
  gains observation steps around the existing
  lease → begin → open → close skeleton.
- **`oschild` module (new, inside `real_dock.rs`)** — `#[cfg(windows)]`
  Win32 query helpers + `#[cfg(not(windows))]` stub returning an
  `Unsupported` observation. Query-only: `EnumChildWindows`,
  `GetParent`, `IsWindow`, `IsWindowVisible`,
  `GetWindowThreadProcessId`, `GetCurrentThreadId`. No window
  creation/destruction/message posting — observation never mutates
  the plugin's state.
- **`RealOsGuiTest`** — unchanged; `native_container_token(slot)`
  supplies the container HWND token (u64) used as the observation
  root. The token stays opaque `u64` at the crate boundary; only the
  `#[cfg(windows)]` observation module casts it to `HWND`.

## Data and Control Flow

```
load_plugin(test_editor descriptor)
  → RealOsGuiTest::run(window)
    → declare/bind/settle (existing)
    → lease_real_slot → container_token step (NEW: record HWND u64)
    → begin_session → ParentSlotLease(payload=token)
    → session.open(Docked)          [caller = scenario thread]
        → redeem_native_lease → ParentLease(seq, token)
        → Vst3Plugin::open_gui → createView/setFrame/attached
        → plugin creates child HWND under container
    → OS OBSERVE (NEW, Windows):
        EnumChildWindows(container) → children[]
        per child: GetParent==container? IsWindowVisible? IsWindow?
        GetWindowThreadProcessId(container/child) + GetCurrentThreadId
    → session.close → close_gui → removed() → DestroyWindow
    → OS VERIFY (NEW): IsWindow(child)==0 for all observed children
    → session_closed_concluded + drain_events ⊇ {Attached{Docked}, Closed}
    → verdict: all steps pass → Pass; any mismatch → Fail;
               open Unsupported → Unsupported; prereq missing → NotRun
```

## Non-Functional Constraints

- Win32 calls are query-only and `#[cfg(windows)]`-gated;
  `#[expect(unsafe_code)]` + SAFETY comments per call site
  (`poc.rs` precedent). No `windows-sys` types escape the module —
  observations serialize to `u64`/`u32`/`bool`/`Vec<u64>` in
  `observed`.
- `windows-sys` is optional + Windows-targeted: default features and
  the headless lane's dependency tree are unchanged.
- Thread-affinity deviation (open runs on the scenario thread, not
  the container HWND's pump-owning GPUI app thread) is recorded as
  fact in `observed.thread_affinity` + a `detail` note; it is never
  a Pass criterion and never hidden.
- Evidence schema stays `ruviform-ui` vocabulary — HWND values are
  recorded as decimal `u64`, thread ids as `u32`.

## Risks and Trade-offs

- **Child enumeration races**: `attached` creates the child
  synchronously inside `session.open`, so by the time `open`
  returns `Ok` the child exists — no settle wait needed. If a
  plugin deferred creation, `EnumChildWindows` would honestly find
  none → `Fail` (correct: contract violation).
- **Multiple children**: the fixture creates exactly one; the
  observation records all children and requires every observed
  child to satisfy parent+visible, and every recorded child to be
  dead after close. Extra unexpected children fail honestly.
- **`IsWindowVisible` ancestor chain**: returns false if any
  ancestor is hidden. The container is shown via `SW_SHOW` at
  lease time; if the OS reports invisible the step fails honestly —
  no retry loop that could mask a real defect.
- **Off-Windows compile**: the stub returns an `Unsupported`
  observation; the case body is unreachable there anyway
  (`NativePluginDock` pre-flight), but the code must compile — the
  stub keeps types identical.

## Deferred Design Concerns

- dock-pixels / editor pixel capture — `scarecrow173/ruviform#532`
  DComp unresolved; not claimed.
- input/focus/resize/re-dock on the child HWND — `BACKEND_CAPS`
  Unsupported; follow-up slices.
- Running `open` on the pump-owning thread (true D1 conformance) —
  would require dispatching `session.open` onto the GPUI app
  thread; recorded as a known deviation, revisit if a later slice
  needs strict D1 evidence.

## Verification Notes

- [ ] `mise run check/lint/test ruviform-plugin-dock-test` (default
  features — headless non-regression).
- [ ] `mise run check/lint/test ruviform-plugin-dock-test
  --features real-dock` — feature-gated path compiles.
- [ ] `mise run test-ui-plugin-dock-real` on interactive Windows —
  `real-editor-attach` = `Pass` with OS observation values in JSONL.
- [ ] `mise run check-ui-driver-boundary` — no GPUI/Win32 type leak.
- [ ] `mise exec -- prek run --all-files`.
