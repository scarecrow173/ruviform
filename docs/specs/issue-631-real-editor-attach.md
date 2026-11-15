---
id: "SPEC-034Y3nW74RKYWUkLfv81MG"
type: "spec"
status: "approved"
title: "Issue #631 real-editor-attach（test_editor 実 IPlugView による real-editor-attach Pass 化 — 実 child HWND の OS 観測・detach 収束・thread-affinity 記録）"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-editor-attach（test_editor 実 IPlugView による real-editor-attach Pass 化 — 実 child HWND の OS 観測・detach 収束・thread-affinity 記録）

## What Should Be Built

The `real-editor-attach` case of the `rf631.plugin-dock-real`
scenario (`crates/ruviform-plugin-dock-test/src/real_dock.rs`,
opt-in `real-dock` feature) moves from an honest `Unsupported`
verdict to a real `Pass` by exercising the `test_editor` VST3
fixture — the plugin-side `IPlugView` merged in `a6d5735`
(Issue #630 test_editor slice, PR #668) — through the production
host path:

```
NativeSlot → lease_parent → ParentSlotLease → redeem_native_lease
→ session.open(Docked) → createView("editor") → setFrame
→ attached(container_hwnd, "HWND") → real child HWND
→ OS observation → session.close → removed() → DestroyWindow
→ IsWindow(child) == 0
```

- **`test_editor` descriptor selection** — `load_plugin` selects
  the `test_editor` descriptor explicitly; the silent
  `.or_else(first())` fallback to `test_gain` is removed. A
  missing `test_editor` descriptor is an honest `Fail`/`NotRun`,
  never a silent retreat to an editor-less fixture.
- **OS-level child-HWND observation** — after `session.open(Docked)`
  returns `Ok(EditorCapabilities)`, the lane enumerates the real
  container HWND's children (`EnumChildWindows`) and verifies the
  plugin-created child: `GetParent(child) == container`,
  `IsWindowVisible(child) != 0`, `IsWindow(child) != 0`.
- **Detach convergence** — `session.close` runs the production
  `close_gui` → `removed()` path; the lane then verifies
  `IsWindow(child) == 0`, `session_closed_concluded`, and that
  drained events contain `Attached{mode: Docked}` and `Closed`.
- **Thread-affinity recording** — `GetWindowThreadProcessId` records
  the owning thread ids of the container and child HWNDs plus the
  scenario caller thread; the deviation from the Issue #389 D1
  contract (attach on the parent HWND's pump-owning thread) is
  recorded as an observation fact, not used as a Pass basis.
- **Honest verdicts** — `Pass` only when open succeeds AND every
  OS observation agrees; observation mismatch → `Fail`; defined
  refusal (`EditorOpError::Unsupported`) → `Unsupported`; missing
  artifact/interactive session → `NotRun`/`environment_blocked`.

## Why Now

The predecessor slice (PR #660) built the real-window lane and
recorded `real-editor-attach` as `Unsupported` because `test_gain`
answers `createView` with null. The `test_editor` fixture — a real
`IPlugView` that creates a `WS_CHILD | WS_VISIBLE` HWND under the
supplied parent — is now merged (`a6d5735`, Issue #630). This slice
consumes that fixture so the production host's `open_gui`/`close_gui`
path is exercised end-to-end against a real native editor surface,
and the child HWND's existence, parenting, visibility, and
destruction are proven by OS observation rather than inferred.

## Users and Value

Ruviform GUI test infrastructure: the `rf631.plugin-dock-real`
scenario is the evidence lane for Issue #631's native-editor dock
acceptance. A `Pass` verdict here is the first real proof that the
production `PluginEditorSession` + `ParentLease` redeem +
`Vst3Plugin::open_gui` chain produces and tears down a genuine
plugin-owned child HWND under a genuine container HWND — the
substrate every later dock-pixels/input/resize/focus case builds
on.

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test/` only:
  - `Cargo.toml` — optional `windows-sys` 0.61 dependency under
    `[target.'cfg(windows)'.dependencies]`, wired into the
    `real-dock` feature via `dep:windows-sys`; features limited to
    the observed APIs (`Win32_Foundation`,
    `Win32_UI_WindowsAndMessaging`, `Win32_System_Threading`).
  - `src/real_dock.rs` — `test_editor` descriptor selection,
    container-token observation step, child-HWND OS observation,
    detach-convergence steps, thread-affinity recording, verdict
    mapping update.
  - `src/lib.rs` / `README.md` — doc-table updates for the real
    lane only; the headless lane table is unchanged.
  - `tests/` — non-regression asserts stay valid; updates only if
    the real lane's report shape requires it.
- Doc chain `docs/{specs,designs,plans,tasks,test-specs,impl/ir,
  impl/exp}/issue-631-real-editor-attach.*` and regenerated docs
  indexes.
- `evidence/rf631/plugin-dock-real-scenario.jsonl` +
  `verification.md` — the real-run record.

### Out of Scope

- Headless lane changes — `src/scenario.rs` keeps selecting
  `test_gain` and `editor-attach` stays `Unsupported` (the fake
  `ParentSlotLease` payload is not a real HWND and would fail the
  plugin's `IsWindow` gate → `Fail`, a regression).
- `mise.toml` — outside this worker's lease; the existing
  `test-ui-plugin-dock-real` task is used as-is (its description
  still says `test_gain`; hygiene update is a follow-up slice).
- dock-pixels / editor pixel capture and compositing judgment
  (Issue #532 DComp unresolved), child-HWND input/focus,
  resize negotiation, re-dock — all stay `NotRun`/`Unsupported`.
- `crates/ruviform-plugin-test-vst3/**` (fixture is consumed
  as-merged), all other production crates, `*.github/**`,
  `POLICIES.md`, `AGENTS.md`, `crates/README.md`, root `Cargo.toml`.
- 3OS GUI support (macOS/Linux stay `Deferred`/`Unsupported`),
  `*-msrv`/`*-full` lanes, new mise tasks, CI wiring, Issue
  auto-close.

## Requirements

- `load_plugin` resolves the `test_editor` descriptor by name;
  absence is `Fail` (present-but-broken, `prerequisite: false`)
  or `NotRun` per the existing artifact-resolution taxonomy —
  never a silent `test_gain` fallback.
- `session.open(Docked)` must reach `createView("editor")`,
  `isPlatformTypeSupported("HWND")`, `setFrame`, and
  `attached(container_hwnd, "HWND")` on the real fixture through
  the production `Vst3Plugin::open_gui` path.
- OS observation (Windows only, `#[cfg(windows)]`, query-only
  Win32 with `#[expect(unsafe_code)]` + SAFETY comments):
  `EnumChildWindows(container)` yields ≥1 child;
  `GetParent(child) == container`; `IsWindowVisible(child) != 0`;
  `IsWindow(child) != 0` after open; `IsWindow(child) == 0` after
  `session.close`.
- `session.drain_events()` after close contains
  `Attached{mode: Docked}` and `Closed`; `session_closed_concluded`
  holds.
- Thread-affinity: caller/container/child owning thread ids are
  recorded in `observed`; the D1-contract deviation note is
  recorded in `detail`/docs but never used as a Pass basis.
- Every step lands in `observed.steps` as a pass/fail row; an
  observation mismatch produces `Fail`, never `Pass`.
- Non-Windows: the OS-observation steps record `Unsupported`
  honestly; the case cannot claim `Pass` there (in practice the
  `NativePluginDock` capability pre-flight already reports
  `Unsupported` off-Windows).
- `EnvironmentKind::RealWindow` is emitted only on the real-run
  path; `plugin_artifact` evidence carries the resolved
  `test_editor` descriptor identity (plugin_id/name/vendor/
  version/SHA-256).
- `windows-sys` stays optional and Windows-targeted — the default
  feature set and headless lane dependency tree are unchanged.
- GPUI types and raw OS handles do not leak into this crate's
  public API or evidence schema (`check-ui-driver-boundary`
  stays green).

## Acceptance Criteria

- [ ] `real-editor-attach` reports `Pass` on an interactive
  Windows session: `session.open(Docked)` → `Ok(EditorCapabilities)`
  → child HWND observed under the container → `session.close` →
  `IsWindow(child) == 0` → events `[Attached{Docked}, Closed]`.
- [ ] `load_plugin` selects `test_editor` by name; no
  `.or_else(first())` fallback remains; a missing descriptor is
  `Fail`/`NotRun`, not silent `test_gain`.
- [ ] `observed` records container/child HWND identity, parent
  match, visibility, post-detach `IsWindow == 0`, drained events,
  and caller/container/child thread ids.
- [ ] Observation mismatch (no child, wrong parent, invisible,
  child alive after close, missing events) → `Fail`.
- [ ] Headless lane non-regression: `editor-attach` stays
  `Unsupported`, `tests/console.rs` asserts keep passing,
  `mise run test-ui-plugin-dock` verdict table unchanged.
- [ ] `windows-sys` optional dep + `real-dock` feature wiring
  documented in `Cargo.toml` comments and PR body; `Cargo.lock`
  diff (if any) is regeneration-only.
- [ ] `evidence/rf631/` updated with the real run; `verification.md`
  records verdicts, observation values, and remaining
  `NotRun`/`Unsupported` honestly.

## Deferred Design Concerns

- dock-pixels / editor pixel capture + compositing judgment —
  `scarecrow173/ruviform#532` DComp unresolved; pixel verdicts are
  not claimed in this slice.
- child-HWND input/focus/resize/re-dock `Pass` judgments —
  `BACKEND_CAPS` keeps those `Unsupported`; follow-up slices.
- `mise.toml` `test-ui-plugin-dock-real` description hygiene
  (`test_gain` wording) — outside this worker's lease.
- 3OS GUI support — Deferred; macOS/Linux stay `Unsupported`.

## Sources and Relations

- Issue `scarecrow173/ruviform#631` (tracker — no auto-close).
- Predecessor slice: `docs/specs/issue-631-real-window-dock.md`
  (PR #660 real-window lane), `docs/specs/issue-631-plugin-dock-scenario.md`
  (headless lane).
- Fixture source: `docs/specs/issue-630-test-editor.md` (PR #668,
  `a6d5735` — `TestEditorView` real `IPlugView`).
- Thread-affinity contract: `docs/designs/issue-389-vst3-l0-editor-attach.md`
  D1 (caller thread = parent HWND pump owner; deviation recorded
  as observation).
- Verification vocabulary: `POLICIES.md` (OS-L1/GUI-L2, Pass/Fail/
  NotRun/Unsupported, fail-closed).
- Win32 observation precedents: `crates/ruviform-poc-native-dock/src/poc.rs`
  (`EnumChildWindows` callback), `crates/ruviform-plugin-test-vst3/src/editor_view.rs`
  (`GetParent`/`IsWindow` asserts), `real_dock.rs`
  `native_container_token` observation.
