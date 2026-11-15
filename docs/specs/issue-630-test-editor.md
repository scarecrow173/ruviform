---
id: "SPEC-034Y10F0GbD81g2dJSmyyY"
type: "spec"
status: "approved"
title: "Issue #630 test_editor slice — VST3 plugin-side IPlugView (AC-4 real child HWND fixture)"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/editor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/editor_view.rs"
      - type: "file"
        path: "docs/specs/issue-630-test-editor.md"
      - type: "file"
        path: "docs/designs/issue-630-test-editor.md"
      - type: "file"
        path: "docs/plans/issue-630-test-editor.md"
      - type: "file"
        path: "docs/tasks/issue-630-test-editor.md"
      - type: "file"
        path: "docs/test-specs/issue-630-test-editor.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-test-editor-slice.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-test-editor-slice.jsonl"
      - type: "file"
        path: "evidence/rf630/test-editor-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/strings.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/controller.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/factory.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/fault.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/host_path.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-fault.md"
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-fault.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-plugin-suite.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "evidence/rf630/test-fault-verification.md"
  defers: []
  deferred-by: []
---

# Issue #630 test_editor slice — VST3 plugin-side IPlugView (AC-4 real child HWND fixture)

## What Should Be Built

A `test_editor` plugin class inside the existing Issue #630 VST3
test fixture cdylib (`ruviform-plugin-test-vst3`), providing the
plugin-side `IPlugView` implementation that the production host's
`open_gui` path (`Vst3Plugin::open_gui`, Issue #389 substrate)
requires — the plugin-fixture precondition of Issue #630 AC-4:

- **`test_editor` class pair** — a component
  (`IComponent`+`IAudioProcessor`+`IProcessContextRequirements`,
  trivial pass-through DSP, no parameters) and a dedicated
  `IEditController` whose `createView("editor")` returns an owned
  `IPlugView`. VST3 factory grows 4→6 classes (index 4/5 =
  `test_editor` component + `test_editor Controller`).
- **`IPlugView` implementation** — `isPlatformTypeSupported` →
  `kResultTrue` only for `"HWND"`; `attached` validates the parent
  HWND and creates a real `WS_CHILD` window under it on Windows
  (`CreateWindowExW`, solid-brush paint, `DefWindowProcW`,
  once-per-module class registration, no own message pump or
  thread); `removed` destroys the child; `getSize`/`onSize` carry an
  honest fixed size; `setFrame` retains/releases `IPlugFrame`
  (null always accepted); `canResize` → `kResultFalse` (honest —
  the host bridge's `resizeView` refuses); remaining verbs ack or
  reject honestly.
- **Host-path coverage** — `tests/host_path.rs` descriptor pins
  (3 components), COM-level `IPlugView` contract cases (through the
  real `ComWrapper`/`ComPtr` vtbl dispatch), and headless-safe
  `open_gui` cases through the production `Vst3Library` bridge
  (`ParentLease::new(0,0)` → `Err(PluginFailed)` on Windows,
  `Err(Unsupported)` elsewhere).
- **Doc/evidence chain** for this slice.

## Why Now

Issue #630's AC-4 (Native Editor producing a real child HWND) is
blocked on a plugin fixture that actually implements `IPlugView`:
the production host adapter (`open_gui`/`close_gui`,
`BridgePlugFrame`, neutral `PluginEditorSession`/`ParentLease`
seam) is already implemented and verified against Issue #389, but
every existing fixture answers `createView` with null, so the host
converges to `Err(Unsupported)` before `attached` is ever reached.
This slice supplies the missing plugin-side fixture so the real-HWND
attach pass can be exercised by the follow-up #631 real-dock lane
slice.

## Users and Value

Ruviform test infrastructure: the L1/GUI verification lanes and
Issue #631 real-dock scenarios need a deterministic test plugin
whose editor is a real native window, so native-editor acceptance
is judged against a controlled fixture rather than an external
commercial plugin whose behavior cannot be pinned.

## Scope

### In Scope

- `crates/ruviform-plugin-test-vst3/` only: `editor.rs` (processor +
  component pair), `editor_view.rs` (`IPlugView` + Windows child
  HWND), `controller.rs` (`TestEditorController`), `factory.rs`
  (6 classes), `lib.rs`, `strings.rs` (cstring compare helper),
  `Cargo.toml` (approved `windows-sys` edge), `README.md`.
- `tests/host_path.rs` updates + COM-level editor cases.
- Doc chain (`spec`/`design`/`plan`/`task`/`test-spec`/`ir`/`exp`)
  and `evidence/rf630/test-editor-verification.md`.

### Out of Scope

- CLAP `test_editor` (separate slice; own dep approval).
- Real parent-HWND `open_gui` **success** evidence — requires a
  real HWND + message pump; belongs to the #631 real-dock slice.
- Fake-lane editor-attach verdict changes (`ruviform-plugin-dock-test`
  untouched — its lease payload is not a real HWND).
- L1 editor attach (Unsupported by design, Issue #115 scope).
- `test_stateful`/`test_instrument`, pre-Hello faults, external SDK
  validators, non-Windows GUI runs, `*-msrv`/`*-full` lanes.
- Any production crate change (`ruviform-plugin-vst3`/`clap`/`host`/
  `export`/`core` stay zero-diff).

## Requirements

- `createView("editor")` returns a non-null owned `IPlugView`;
  any other view name returns null.
- `isPlatformTypeSupported("HWND")` → `kResultTrue` (the host
  requires exactly this value); other platform types →
  `kResultFalse`.
- `attached` is an honest gate: null/non-HWND/non-Windows parents
  and window-creation failure → `kResultFalse`; success creates a
  real child HWND owned by this view. Never fake success.
- `removed` always returns `kResultOk` (host debug-asserts it) and
  destroys the child window.
- `getSize` reports a deterministic fixed size; null out-pointer →
  `kInvalidArgument`. `canResize` → `kResultFalse`.
- `setFrame` accepts null and non-null `IPlugFrame` with correct
  COM retain/release.
- The component stays a trivial pass-through plugin with no
  parameters; lifecycle/protocol violations remain defined rejects.
- Editor state is interior-mutable; no own message pump/thread;
  `unsafe` confined to the Windows window-management block with
  `#[expect(unsafe_code)]` + SAFETY comments.
- Windows-only Win32 calls live behind `#[cfg(target_os = "windows")]`
  so the same cdylib still builds on macOS/Linux.

## Acceptance Criteria (this slice)

- [ ] `test_editor` discoverable as a third component descriptor
  through `Vst3Library::load` → `descriptors()`/`factories()`;
  `countClasses == 6`.
- [ ] COM-level `IPlugView` contract verified in-crate: view-type
  dispatch, platform support, size/frame/resize verbs, defined
  rejections, attach lifecycle with a real HWND where the OS
  permits.
- [ ] Headless `open_gui` through the production bridge returns
  `Err(PluginFailed)` on Windows (null lease payload fails the
  `IsWindow` gate) and `Err(Unsupported)` on non-Windows.
- [ ] Existing `test_gain`/`test_fault` lanes stay green:
  `fmt-check`, `check`/`lint`/`test ruviform-plugin-test-vst3`,
  `check-l1-real-plugin`, `lint-l1-real-plugin`,
  `test-l1-real-plugin`, `test-rt`, `prek run --all-files`.
- [ ] Honest boundaries recorded: real-HWND attach success,
  composited pixels, focus/input/resize round-trips, CLAP editor,
  L1 attach, external validators are NotRun/Unsupported with
  reasons — never claimed as Pass.
- [ ] Production crates zero-diff; PR uses `Refs #630`.

## Deferred Design Concerns

- Real-HWND attach Pass + dock-pixels — `scarecrow173/ruviform#631`
  real-dock lane slice consumes this fixture.
- CLAP `test_editor` — later slice with its own dependency approval.
- L1 editor attach — Issue #115 scope, Unsupported by design.
- Editor input/focus/IME forwarding — separate engineering once a
  real attach exists.

## Sources and Relations

Suite conventions: `docs/designs/issue-630-test-plugin-suite.md`.
Multi-class factory precedent: `docs/designs/issue-630-test-fault.md`.
Host attach contract: `docs/designs/issue-389-vst3-l0-editor-attach.md`
(D1 call sequence, D2 `BridgePlugFrame` honest refusal, D4 platform
gate, D6 error taxonomy). Verification vocabulary: `POLICIES.md`
(OS-L1/GUI-L2/EXT-L3, Pass/Fail/NotRun/Unsupported).
