---
id: "TSPEC-01M7WJTWYS1655HDBKFJ31SN92"
type: "test-spec"
status: "approved"
title: "Issue #631 editor input/focus verbs テスト仕様 — boundary 語彙・L0-only guard・fixture event log・title-suffix oracle の判定"
created: "2026-11-20"
updated: "2026-11-20"
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
  verifies:
    - "docs/specs/issue-631-editor-input.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 editor input/focus verbs テスト仕様 — boundary 語彙・L0-only guard・fixture event log・title-suffix oracle の判定

## Purpose

Pin down what the `real-editor-input`/`real-editor-focus` cases —
and the verb chain beneath them — must prove, so the
`Unsupported`-shaped gap becomes a `Pass` only on OS-observed
delivery, never on inference.

## Feature

`send_key`/`set_focus` through the production
`PluginEditorSession` → `InProcessEditorOps` → `HostCommand` →
`Vst3Plugin` → `IPlugView` chain, with the fixture recording
`onFocus`/`onKeyDown`/`onKeyUp` as title-stamped event tokens the
lane reads via `GetWindowTextW`.

## Rules

- **Driver-neutral boundary**: `GuiKeyEvent`/`GuiKeyDirection`/
  `GuiModifiers` carry no OS handle or toolkit type; the
  `editor_session.rs` boundary scan stays green.
- **Default refusal**: a `Plugin` that never implements the hooks
  answers `Err(HostError::Unsupported)` — `ErasedPlugin`
  included.
- **L0-only guard**: `HostCommand::{SetGuiFocus,SendGuiKey}` on an
  `OutOfProcess` slot → defined `Unsupported`; `to_wire` is never
  reached; wire version unchanged; `command_handle`/`retarget`
  rewrite `handle` correctly.
- **Session state gate**: `set_focus`/`send_key` before attach →
  `InvalidState`; in `Attached`/`Hidden` they call through.
- **Non-fatal semantics**: `Ops(Unsupported)` (or any
  `EditorOpError`) leaves `Attached`/`Hidden` untouched — no
  synthetic state transition.
- **L1 honesty**: `L1EditorOps` answers `Unsupported` for
  `set_focus`/`send_key` exactly like every other op.
- **VST3 honesty**: no view → `Unsupported`; supplementary-plane
  `key` → `InvalidValue`; non-`kResultOk` → `PluginFailed`;
  `GuiModifiers` bits map onto `KeyModifier` bits per flag.
- **Fixture log order**: tokens append in receive order —
  `f:{0|1}`, `kd:{key:04x}:{code:04x}:{mods:04x}`, `ku:{…}` —
  after `rvf-test-editor-attach-{n}|`.
- **Effect oracle**: `Pass` requires every child HWND title to
  `ends_with` the case's expected suffix — input
  `|kd:0061:0041:0000,ku:0061:0041:0000`, focus `|f:1,f:0`.
- **Session convergence**: events are exactly
  `[Attached{Docked}, Closed]`; `session_closed_concluded`;
  every recorded child reports `IsWindow(child)==0` after close.
- **Verdict honesty**: accepted+logged → `Pass`; defined refusal
  → `Unsupported`; accepted but unobservable (off-Windows) →
  `NotRun`; accepted but missing/mismatched suffix → `Fail`;
  missing artifact/headless → `NotRun`/`environment_blocked`.
- **Headless non-regression**: 9 headless + 26 real rows in
  `--list --include-real`; `blocked_probe` produces 26 `NotRun`
  rows; headless lane asserts unchanged.

## Examples

- **Example (Pass path, input)**: interactive Windows session +
  `test_editor` artifact → `real-editor-input` reports `Pass`
  with `observed.steps` showing `editor_input`,
  `editor_input_effect` (title carries
  `…|kd:0061:0041:0000,ku:0061:0041:0000`),
  `session_state_after_op=Attached`, and `session_events` rows
  all `pass`.
- **Example (Pass path, focus)**: same environment →
  `real-editor-focus` reports `Pass` with title suffix
  `|f:1,f:0`.
- **Example (Unsupported)**: a plugin whose `set_gui_focus`/
  `send_gui_key` stays defaulted → `Ops(Unsupported)` → verdict
  `Unsupported` with `Attached` preserved — not `Fail`.
- **Example (Fail)**: verbs accepted (`kResultOk`) but the child
  title lacks the expected suffix → `Fail` naming the
  `*_effect` step.
- **Example (fixture unit)**: `onFocus(1)` → `onKeyDown('a',
  0x41, 0)` → `onKeyUp('a', 0x41, 0)` → `onFocus(0)` yields
  `rvf-test-editor-attach-{n}|f:1,kd:0061:0041:0000,ku:0061:0041:0000,f:0`.
- **Example (headless)**: `--blocked-probe` writes 26 `NotRun` +
  `observed.environment_blocked` rows; exit 0.

## Guarantees

- A `Pass` requires the fixture to have actually received the
  verbs — the evidence row carries the title text the OS
  returned, not an inference.
- No wire-protocol change: the new commands are unreachable from
  the L1 path and the wire version is untouched.
- `GuiModifiers` is a `u16` bitmask — no
  `struct_excessive_bools` lint debt, no per-bool API drift.
- The session surface stays driver-neutral: no GPUI/HWND/NSView/
  X11 type in `editor_session.rs`.

## Non-goals

- Wire/`L1EditorOps` support for the verbs, CLAP input/focus,
  IME composition, OS key injection into the child HWND,
  pixel/compositing checks (Issue #532), macOS/Linux GUI runs.

## Risk

If the effect oracle is weakened (e.g. substring instead of
suffix, or skipping the title check on an accepted verb), the
lane could report `Pass` for verbs the fixture never received —
the fake-pass failure mode `POLICIES.md` exists to prevent.
