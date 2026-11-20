---
id: "SPEC-01M7WJTWYSWBE3NTHK5GMTCT74"
type: "spec"
status: "approved"
title: "Issue #631 editor input/focus verbs — EditorSessionOps set_focus/send_key、L0-only HostCommand、fixture event log による real-editor-input/focus Pass 化"
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
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 editor input/focus verbs — EditorSessionOps set_focus/send_key、L0-only HostCommand、fixture event log による real-editor-input/focus Pass 化

## What Should Be Built

The Issue #631 AC3 editor-verb surface gains its two remaining
verbs — `send_key` (keyboard input) and `set_focus` (GUI focus
notification) — end to end, plus two new real-lane catalog rows
`real-editor-input`/`real-editor-focus` that prove the verbs land
on a real `test_editor` `IPlugView` through the production chain:

```
session.set_focus(bool) / session.send_key(GuiKeyEvent)
→ InProcessEditorOps → HostCommand::{SetGuiFocus, SendGuiKey}
→ Vst3Plugin::{set_gui_focus, send_gui_key}
→ IPlugView::onFocus / onKeyDown / onKeyUp
→ fixture stamps event tokens on the child HWND window text
→ lane reads GetWindowTextW → effect oracle → verdict
```

- **Boundary vocabulary** (`ruviform-plugin-core`) —
  `GuiKeyDirection::{Down,Up}` (`#[non_exhaustive]`),
  `GuiModifiers` (u16 bitmask wrapper: `NONE`/`SHIFT`/`ALTERNATE`/
  `COMMAND`/`CONTROL` + `contains`/`union`), and
  `GuiKeyEvent{direction,key:char,key_code:i16,modifiers}` —
  driver-neutral, no OS handle or toolkit type.
- **`Plugin`/`ErasedPlugin` hooks** — `set_gui_focus(bool)` and
  `send_gui_key(GuiKeyEvent)` default to
  `Err(HostError::Unsupported)` so existing plugins keep their
  honest refusal with zero churn.
- **L0-only host commands** — `HostCommand::SetGuiFocus{handle,
  focused}` and `SendGuiKey{handle,event}` join the existing
  L0-only GUI verbs: `command_handle`/`retarget`/L0-only detection
  handle them exhaustively, out-of-process/composite paths answer
  the defined `Unsupported`, and `to_wire` is never reached — no
  wire encoding, no wire-version bump.
- **Session surface** — `EditorSessionOps::set_focus`/`send_key`;
  `InProcessEditorOps` forwards through the host;
  `L1EditorOps` stays `Unsupported` on every op; session
  convenience methods are non-fatal (like `set_size`): they require
  `Attached`|`Hidden` (else `InvalidState`) and leave the state
  untouched on a plugin refusal (`Ops(Unsupported)`).
  `EditorCapabilities` gains unclaimed `input`/`focus` flags.
- **VST3 forwarding** — `set_gui_focus` → `IPlugView::onFocus`;
  `send_gui_key` → `onKeyDown`/`onKeyUp` by direction;
  `GuiModifiers` bits map onto VST3 `KeyModifier` bits; the `key`
  char must fit one UTF-16 `char16` (a supplementary-plane key is
  the defined `InvalidValue`, never a truncated surrogate); no
  view open → `Unsupported`; a non-`kResultOk` view call →
  `PluginFailed`.
- **Fixture event log** — `test_editor`'s view records every
  `onFocus`/`onKeyDown`/`onKeyUp` as ordered tokens
  (`f:{0|1}`, `kd:{key:04x}:{code:04x}:{mods:04x}`, `ku:{…}`)
  appended to the child HWND's window text after
  `rvf-test-editor-attach-{n}|` — the same `GetWindowTextW`
  channel the attach generation already uses.
- **Two real catalog rows** — `real-editor-input` sends key
  down+up of `'a'` and requires title suffix
  `|kd:0061:0041:0000,ku:0061:0041:0000`; `real-editor-focus`
  sends `set_focus(true)` then `(false)` and requires `|f:1,f:0`.
  Both share `observe_editor_verb_lifecycle`, keep the session
  `Attached`, and close-converge on
  `[Attached{Docked}, Closed]` — never pixels.

## Why Now

AC3's remaining gap was exactly this verb pair: resize/re-dock/
visibility already judge real effects, but `EditorSessionOps` had
no input/focus verbs and the fixture acknowledged the VST3 calls
with a bare `kResultOk` — no lane could prove delivery. With the
attach/redock machinery (`real-editor-attach`,
`real-editor-redock`) and the window-text channel precedent in
place, the missing pieces are a driver-neutral boundary
vocabulary, an L0-only host command pair, the adapter forward,
and a fixture-side observable record — all additive, none
wire-visible.

## Users and Value

Ruviform GUI test infrastructure: `send_key`/`set_focus` are the
last AC3 verbs, and their real-lane rows are the first evidence
that a plugin editor receives keyboard/focus notifications through
the production host chain — the prerequisite for any later
interactive-editor acceptance (IME composition stays out of
scope).

## Scope

### In Scope

- `crates/ruviform-plugin-core/` — `boundary.rs` vocabulary,
  `plugin.rs` hooks (trait + erased impls + default
  `Unsupported`), `lib.rs` re-exports.
- `crates/ruviform-plugin-host/` — `command.rs` variants,
  `composite.rs` guard/`command_handle`/`retarget`, `host.rs`
  handlers (plain + shared plugin slots), `editor_session.rs`
  (`EditorCapabilities` flags, `EditorSessionOps`, convenience
  methods, `InProcessEditorOps`/`L1EditorOps`), `lib.rs`
  re-exports, contract tests.
- `crates/ruviform-plugin-vst3/` — `host.rs` `set_gui_focus`/
  `send_gui_key` impls + `KeyModifier` mapping.
- `crates/ruviform-plugin-test-vst3/` — `editor_view.rs` event
  log + title restamp.
- `crates/ruviform-plugin-dock-test/` — `real_dock.rs` two cases
  + `observe_editor_verb_lifecycle` + entries/pins (24→26),
  `real_catalog.rs` rows + docs, `src/lib.rs`, `README.md`,
  `tests/console.rs`, `tests/plugin_dock.rs`.
- `mise.toml` — `test-ui-plugin-dock-real` description lists the
  new op rows.
- Doc chain `docs/{discovery,specs,designs,test-specs}/
  issue-631-editor-input.md` + index rows.
- `evidence/rf631/` regeneration + `verification.md`.

### Out of Scope

- Wire/L1 encoding for the new verbs — `SendGuiKey`/`SetGuiFocus`
  are L0-only by design; out-of-process editor verbs stay a
  separate slice.
- CLAP adapter input/focus forwarding (no `IPlugView` analog
  wired yet — `Unsupported` stands).
- IME composition (`real-window-ime` stays `Unsupported`,
  Issue #389 scope), OS-level key injection into the child HWND
  (`real-os-input` covers window-level injection), pixel
  verification (Issue #532 DComp unresolved).
- macOS/Linux GUI, `*-msrv`/`*-full` lanes, CI wiring, Issue
  auto-close (`Refs #631` only).

## Requirements

- `GuiKeyEvent`/`GuiKeyDirection`/`GuiModifiers` are
  driver-neutral plain data; `#[non_exhaustive]` public enums per
  the repo ADR; no GPUI/HWND/NSView/X11 types in core/host public
  API (the `editor_session.rs` boundary test keeps asserting
  this).
- `Plugin`/`ErasedPlugin` default hooks return
  `Err(HostError::Unsupported)` — a plugin that never opts in
  answers honestly.
- `HostCommand::SetGuiFocus`/`SendGuiKey` never reach `to_wire`;
  composite `OutOfProcess` dispatch returns the defined
  `Unsupported`; `command_handle`/`retarget` rewrite `handle`
  correctly; wire protocol version unchanged.
- Session verbs require `Attached`|`Hidden` (`InvalidState`
  otherwise) and are non-fatal: `Ops(Unsupported)` leaves
  `Attached`/`Hidden` untouched — mirroring `set_size`.
- `L1EditorOps` answers `Unsupported` for `set_focus`/`send_key`
  like every other op.
- VST3: view closed → `Unsupported`; supplementary-plane `key` →
  `InvalidValue`; non-`kResultOk` → `PluginFailed`; `GuiModifiers`
  bits map onto `KeyModifier` bits per flag.
- Fixture: event tokens append in receive order after
  `rvf-test-editor-attach-{n}|`; attach-generation title prefix
  preserved; unit tests prove ordered tokens.
- Lane: `Pass` only when verbs accepted AND every child HWND
  title ends with the expected `|{tokens}` suffix AND session
  stays `Attached` AND close converges on
  `[Attached{Docked}, Closed]`; `Ops(Unsupported)` →
  `Unsupported`; accepted-but-unobservable → `NotRun`;
  accepted-but-mismatch → `Fail`; headless/missing artifact →
  `NotRun`/`environment_blocked`.
- Row count 24→26 everywhere (`real_entries`,
  `REAL_CATALOG`, blocked-probe, tests, README, `mise.toml`,
  lib docs).

## Acceptance Criteria

- [ ] `real-editor-input`/`real-editor-focus` appear in
  `plugin-dock-console --list --include-real` (9 headless + 26
  real rows) and in `REAL_CATALOG`/`real_entries` in identical
  order with `NativePluginDock` required.
- [ ] On an interactive Windows session both rows report `Pass`
  with `effect_confirmed` oracles (title suffix observed), or
  `Unsupported`/`NotRun`/`environment_blocked` honestly —
  never a fabricated `Pass`.
- [ ] `--blocked-probe` regenerates 26 `NotRun` +
  `observed.environment_blocked` rows with no real-window claim.
- [ ] Session contract: `send_key`/`set_focus` before open →
  `InvalidState`; plugin refusal → `Ops(Unsupported)` with state
  preserved; `L1EditorOps` → `Unsupported`.
- [ ] Fixture unit tests: `onFocus(1);onKeyDown('a',0x41,0);
  onKeyUp('a',0x41,0);onFocus(0)` produces
  `rvf-test-editor-attach-{n}|f:1,kd:0061:0041:0000,ku:0061:0041:0000,f:0`.
- [ ] `cargo clippy --all-targets -D warnings`, `cargo fmt
  --check`, `prek run`, scenario-contract and
  `check-ui-driver-boundary` all pass.
- [ ] Evidence + `verification.md` regenerated; no `Pass`
  claimed without the title suffix observed.

## Deferred Design Concerns

- Wire encoding + out-of-process `L1EditorOps` for the verbs —
  deferred; the boundary keeps them L0-only.
- CLAP input/focus verbs — deferred (adapter has no view-input
  analog yet).
- IME composition — `Ime` capability unclaimed, Issue #389
  scope.
- macOS/Linux — `Unsupported`/`Deferred` per platform-type
  policy.

## Sources and Relations

- Issue `scarecrow173/ruviform#631` (tracker — no auto-close).
- Predecessor slices: `docs/specs/issue-631-real-editor-attach.md`
  (child-HWND OS observation + `GetWindowTextW` channel),
  `docs/designs/issue-631-real-row.md` (catalog-row verdict
  mapping), `docs/specs/issue-630-test-editor.md` (fixture),
  `docs/designs/issue-389-vst3-l0-editor-attach.md` (L0-only verb
  precedent).
- VST3 bindings: `IPlugView::onFocus`/`onKeyDown`/`onKeyUp`,
  `KeyModifier` bits (`vst3-0.3.0` `bindings.rs`).
- Verification vocabulary: `POLICIES.md` (Pass/Fail/NotRun/
  Unsupported, fail-closed).
