---
id: "EXP-034ZCLAPED8ex8ceM3kQ2x"
type: "experiment"
status: "completed"
title: "Issue #630 CLAP test_editor — experiment log"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/impl/ir/issue-630-clap-test-editor.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-test-editor-exp.jsonl"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-630-clap-test-editor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/exp/issue-630-test-editor-slice.jsonl"
    - "docs/impl/exp/issue-630-clap-test-plugin.jsonl"
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

# Issue #630 CLAP test_editor — experiment log

Companion event stream:
`docs/impl/exp/issue-630-clap-test-editor-exp.jsonl`.

## Hypotheses

1. The production `ClapPlugin::open_gui`/`close_gui` path can drive
   a plugin-side `PluginGuiImpl` end-to-end with zero host-crate
   changes — the missing piece is only the fixture's
   `clap.ext.gui` declaration and implementation.
2. A `RefCell<GuiState>` on the MainThread object with
   `try_borrow_mut`/`try_borrow` satisfies `PluginGuiImpl`'s `&self`
   verbs without panic on reentrancy.
3. `IsWindow` pre-check in `set_parent` is the correct boundary
   for rejecting opaque `ParentLease` payloads before
   `CreateWindowExW`.
4. `DefWindowProcW` + once-per-module `RegisterClassW` avoids the
   dangling-wnd_proc hazard when the cdylib unloads (VST3
   precedent).
5. Sharing `fault.rs`'s `pass_through` via `pub(crate)` keeps one
   parameterless pass-through law across `test_fault` and
   `test_editor`.

## Observations

- `clack-extensions =0.2.0` `gui` feature exposes `PluginGuiImpl`
  with all required verbs and `Window::as_win32_hwnd()` returning
  the raw handle only for `GuiApiType::WIN32`.
- `clack-host =0.2.0` in-crate test harness instantiates the
  plugin through the real `clap_entry` vtbl — same ABI path as
  production `ClapLibrary`.
- `cargo check` initially flagged two orphaned `///` doc-comment
  blocks in `editor.rs` under clippy `empty_line_after_doc_comments`
  — converted to `//` line comments; lint then passed.
- `TestEditorMainThread` initially held an unread `Shared`
  reference — resolved via `_shared: PhantomData<&'a
  TestEditorShared>` matching existing fixture classes.
- `cargo test --locked -p ruviform-plugin-test-clap`: 57 passed,
  1 ignored, 0 failed. The ignored
  `set_parent_creates_real_child_hwnd` passed when run explicitly
  with `--ignored` on this Windows session.
- `tests/host_path.rs`: 15 passed — five descriptors/factories,
  editor fields, empty params, pass-through, foreign-event drop,
  opaque-lease rejection (`ParentLease::new(0,0)` → `PluginFailed`
  on Windows), non-editor `open_gui` → `Unsupported`.
- L1 lanes (`check`/`lint`/`test-l1-real-plugin`), `test-rt`
  (workspace `rt_` tests), and `prek run --all-files` all passed.

## Decisions

- Adopted: `IsWindow` gate in `set_parent` (hypothesis 3
  confirmed — opaque payload rejected before HWND creation).
- Adopted: `DefWindowProcW` wnd_proc (hypothesis 4 confirmed —
  class registration survives cdylib unload safely).
- Adopted: shared `pass_through` (hypothesis 5 confirmed — no DSP
  law duplication, `rt_` contract preserved).
- Adopted: `PluginGuiImpl` verb matrix returning defined
  rejections for unsupported API/floating/transient/resize —
  honest capability reporting over fake success.
- Deferred: real parent HWND + message pump `open_gui` success —
  NotRun, belongs to #631.

## Summary

All five hypotheses held. The fixture implements `clap.ext.gui`
plugin-side contract faithfully: negotiation is honest, defined
rejections cover every invalid ordering/payload, and the Windows
path creates/destroys a real `WS_CHILD` `HWND`. No production
crate was modified; the host bridge's existing `open_gui`/
`close_gui` observes the fixture without changes.
