---
id: "IMPL-034ZCLAPED7ir7ceM3kQ2x"
type: "impl"
status: "completed"
title: "Issue #630 CLAP test_editor — 実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/tasks/issue-630-clap-test-editor.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/editor.rs"
      - type: "file"
        path: "docs/impl/ir/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-test-editor-exp.jsonl"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/fault.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-630-clap-test-editor.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-clap-test-editor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-630-test-editor-slice.md"
    - "docs/impl/ir/issue-630-clap-test-plugin.md"
    - "docs/test-specs/issue-630-clap-test-editor.md"
    - "evidence/rf630/clap-test-editor-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-630-clap-test-editor.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-clap-test-editor-exp.jsonl"
    rejected: []
---

# Issue #630 CLAP test_editor — 実装記録

## Summary

Added `test_editor` — the fifth plugin class of
`crates/ruviform-plugin-test-clap` — a real CLAP `cdylib` fixture
exposing `clap.ext.gui` on the plugin side: a
`Shared`/`MainThread`/`AudioProcessor` 3-way split whose
MainThread object implements `PluginGuiImpl`, with a Windows
win32 module that validates the host-provided parent with
`IsWindow` and creates a real `WS_CHILD` `HWND`. DSP is a
parameterless pass-through sharing `fault.rs`'s `pass_through`.
The production host bridge (`ruviform-plugin-clap/src/host.rs`)
is unchanged; the fixture is verified through the production
`ClapLibrary` path plus in-crate unit tests and a `#[ignore]`d
Windows real-HWND test.

## Context

Follows `docs/tasks/issue-630-clap-test-editor.md` →
`docs/plans/issue-630-clap-test-editor.md` →
`docs/specs|designs/issue-630-clap-test-editor.md`, derived from
the approved suite design `docs/designs/issue-630-test-plugin-suite.md`,
the merged VST3 editor slice
(`docs/impl/ir/issue-630-test-editor-slice.md`, PR#668), and the
CLAP fixture base (`docs/impl/ir/issue-630-clap-test-plugin.md`).

## Implementation

### Changes

- `src/editor.rs` (new) — `TestEditorShared` (paramless descriptor
  state), `TestEditorMainThread<'a>` (owns `RefCell<GuiState>` +
  `PluginGuiImpl`), `TestEditorAudioProcessor<'a>` (`PluginProcessor`
  pass-through via shared `pass_through`), `TestEditorClapPlugin`
  (`Plugin`/`PluginShared`/`PluginMainThread`/`PluginAudioProcessor`/
  `PluginAudioPorts`/`PluginGui` impls, `declare_extensions` with
  `PluginAudioPorts` + `PluginGui`). `PluginGuiImpl` covers all
  required verbs: `is_api_supported` (WIN32 + embedded +
  `cfg!(windows)` only), `get_preferred_api`, `create`/`destroy`
  (idempotent), `set_scale`, `get_size` (640×480 fixed),
  `can_resize`/`set_size` (honest fixed-size rejection),
  `set_parent` (API-type + create-ordering + double-attach +
  `IsWindow` gates → real `WS_CHILD` `HWND`), `set_transient`
  (defined rejection), `show`/`hide` (`ShowWindow`/`SW_SHOW`/
  `SW_HIDE`, ordering-gated). `#[cfg(test)]` module: negotiation
  matrix, size/resize, lifecycle ordering, defined rejections,
  pass-through, and `#[ignore]`d
  `set_parent_creates_real_child_hwnd` Windows test.
- `src/editor.rs` win32 module (`#[cfg(target_os = "windows")]` +
  `#[expect(unsafe_code)]`) — once-per-module `RegisterClassW`
  (`DefWindowProcW` wnd_proc so the registered class never
  dangles after cdylib unload; `GetClassInfoW` confirm;
  `GetModuleHandleExW(FROM_ADDRESS)` own-module `HINSTANCE`;
  process-lifetime solid brush), `IsWindow` pre-check →
  `CreateWindowExW(WS_CHILD|WS_VISIBLE|WS_CLIPSIBLINGS)` →
  child handle stored in `GuiState`; `DestroyWindow` on
  `destroy`. Non-Windows shim returns defined rejections from the
  same call sites.
- `src/lib.rs` — `mod editor;`, `editor: PluginDescriptor` field,
  `plugin_count()` 4→5, editor descriptor at index 4
  (`org.ruviform-test.clap-test-editor`), `create_plugin`
  dispatch to `TestEditorClapPlugin::{new_shared,new_main_thread}`.
- `src/fault.rs` — `pass_through` widened to `pub(crate)` so
  `test_editor` shares the identical pass-through law (single
  implementation, no DSP-law duplication).
- `Cargo.toml` — `clack-extensions` gains the `gui` feature;
  `[target.'cfg(windows)'.dependencies] windows-sys = "0.61"` with
  `Win32_Foundation`/`Win32_Graphics_Gdi`/
  `Win32_System_LibraryLoader`/`Win32_UI_WindowsAndMessaging`
  (same feature set as the VST3 fixture; workspace lock already
  carries `windows-sys 0.61.2`).
- `tests/host_path.rs` — descriptor/factory pins updated 4→5,
  editor index-4 descriptor fields, `params()` empty,
  pass-through audio, foreign `ParamValue` event drop,
  `open_gui(ParentLease::new(0,0))` defined rejection (Windows
  `PluginFailed` via `IsWindow` gate; non-Windows `Unsupported`),
  post-failure teardown convergence, non-editor `open_gui` →
  `Unsupported`.
- `Cargo.lock` — only the `windows-sys` target edge added.
- `README.md` — five-plugin factory description + editor class
  notes.
- Doc chain (`docs/{discovery,specs,designs,plans,tasks,
  test-specs}/issue-630-clap-test-editor.*`), this record,
  `docs/impl/exp/issue-630-clap-test-editor.md` + `-exp.jsonl`,
  `evidence/rf630/clap-test-editor-verification.md`.

### Decisions During Implementation

- **`IsWindow` pre-check is mandatory in `set_parent`** — the
  host bridge converts `ParentLease::payload()` into a Win32
  `Window` without proving the handle is live; an opaque payload
  (e.g. `ParentLease::new(0,0)`) must be rejected before
  `CreateWindowExW`, otherwise fake attach could falsely succeed.
- **`DefWindowProcW` as `lpfnWndProc`** — a module-local wnd_proc
  would dangle if the cdylib unloads while the registered class
  persists; `DefWindowProcW` lives in user32 and is always valid
  (same judgment as VST3 `editor_view.rs`).
- **GUI state in `RefCell<GuiState>` on MainThread with
  `try_borrow_mut`/`try_borrow`** — `PluginGuiImpl` verbs take
  `&self`; borrow failure converges to a defined rejection rather
  than panic, matching the fixture's reentrancy policy.
- **`pass_through` shared via `pub(crate)`** — the parameterless
  pass-through law is identical across `test_fault` and
  `test_editor`; one implementation avoids drift.
- **`set_transient`/`suggest_title` stay honest** — floating and
  transient attach are #389 D7 deferred; the fixture returns
  defined rejections/defaults instead of faking support.
- **`windows-sys` target-only dep, VST3-identical feature set** —
  approved by coordinator; non-Windows builds compile the shim
  with zero Win32 surface.

## Related Experiments

### Adopted

- `docs/impl/exp/issue-630-clap-test-editor-exp.jsonl` —
  hypothesis → implementation → validation → decision chain for
  this slice.
- `docs/impl/ir/issue-630-test-editor-slice.md` (VST3 precedent) —
  win32 class-registration/lifetime pattern and host-path test
  shape.

### Rejected

- Module-local `wnd_proc` for the registered class — rejected for
  the dangling-pointer risk after cdylib unload.
- Treating `ParentLease::payload()` as a proven HWND — rejected;
  `IsWindow` gate required.
- Separate `test_editor` DSP implementation — rejected; shared
  `pass_through` keeps one law.

## Validation

All lanes run and passed on this Windows session
(rust `1.99.x` via mise, see evidence for exact versions):

- `cargo check --locked --all-targets -p ruviform-plugin-test-clap`
  — pass.
- `mise run lint ruviform-plugin-test-clap` — pass (`-D warnings`).
- `cargo test --locked -p ruviform-plugin-test-clap` — 57 passed,
  1 ignored (`set_parent_creates_real_child_hwnd`), 0 failed.
- `cargo test --locked -p ruviform-plugin-test-clap -- --ignored`
  — real-HWND test passed (real `WS_CHILD` creation, `GetParent`
  relation, `IsWindow` liveness, duplicate-attach rejection,
  `DestroyWindow` teardown).
- `mise run check-l1-real-plugin` / `lint-l1-real-plugin` /
  `test-l1-real-plugin` — all pass (adapter e2e 3+3,
  test-plugin e2e, fault e2e 8+8, stateful e2e 2+2).
- `mise run test-rt` — all workspace `rt_` tests pass.
- `mise exec -- prek run --all-files` — all hooks pass.
- `cargo fmt --check` via fmt lane — pass.

## Risks

- L0 in-process + plugin-side HWND verification only; the
  real-parent `open_gui` success path through a message pump is
  NotRun — deferred to `scarecrow173/ruviform#631` (CLAP leg
  pending lane extension).
- L1 `OpenGui`/`CloseGui` remains `HostError::Unsupported` by
  design — no L1 editor e2e in this slice.
- Non-Windows OS runtime untested — `cfg!(windows)` gate and shim
  keep build green, runtime GUI behavior is Windows-only evidence.
- `set_transient`/`suggest_title` real attach deferred (#389 D7).

## Follow-ups

- `scarecrow173/ruviform#631` — real-dock lane: real parent HWND +
  message pump `open_gui` success for both VST3 and CLAP legs.
- External CLAP SDK validator run — separate slice once License/
  environment is settled.
- `test_stateful`/`test_instrument` editor variants — only if a
  future requirement adds GUI to those classes.
- Consumer-oracle cross-checks and non-Windows OS lanes — Deferred.
