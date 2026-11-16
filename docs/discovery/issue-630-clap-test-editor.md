---
id: "DISC-034ZCLAPED1sl1ceM3kQ2x"
type: "discovery"
status: "completed"
title: "Issue #630 CLAP test_editor slice — exploration notes"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-630-clap-test-editor.md"
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
  related:
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
    - "crates/ruviform-plugin-test-vst3/src/editor_view.rs"
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

# Issue #630 CLAP test_editor slice — exploration notes

## Question

`ruviform-plugin-test-clap` に `test_editor`（`clap.ext.gui` plugin
側実装 + 実 child HWND）を追加する際、production host bridge・
clack-extensions 0.2.0・VST3 先例から確定した設計制約は何か。

## Findings

- **host 側は実装済み・変更不要**: `ruviform-plugin-clap/src/host.rs`
  の `open_gui` は `get_extension::<PluginGui>()` →
  `is_api_supported(GuiConfiguration{WIN32, floating:false})` →
  `create` → `set_parent(Window::from_win32_hwnd(parent.payload()))`
  → `show`、失敗時 `gui.destroy` rollback → `PluginFailed`。
  `close_gui` は `hide`(best-effort)+`destroy`。非 Windows は
  `EDITOR_GUI_API=None` で早期 `Unsupported`。
- **plugin 側に欠けているのは `clap.ext.gui` extension のみ** —
  `clack-extensions` の `gui` feature が `PluginGuiImpl`（必須 verb
  一式）と `Window::as_win32_hwnd()` を提供。現行 fixture の
  features（`audio-ports`/`note-ports`/`params`/`state`）に `gui`
  は無い → Spec 承認済み feature 追加。
- **`Window` は opaque**: `as_win32_hwnd()` は api_type==WIN32 の
  ときだけ raw pointer を返す。handle の生存は型が保証しない —
  `IsWindow` による pre-check が必須（opaque `ParentLease` payload
  を HWND と誤認しないための境界）。
- **3 分割構成の先例**: `fault.rs`/`stateful.rs` の
  Shared/MainThread/AudioProcessor。gui state は MainThread 側
  `RefCell` interior mutability（`PluginGuiImpl` は `&self`）。
- **unsafe 局所化**: crate は現状 unsafe 無し方針 → win32 module
  + `#[expect(unsafe_code)]` + SAFETY コメント。VST3
  `editor_view.rs` の once-per-module class 登録・
  `lpfnWndProc=DefWindowProcW`（cdylib unload 後も class が残存
  するため module-local wnd_proc は dangling）・`GetClassInfoW`
  確認・`GetModuleHandleExW(FROM_ADDRESS)` own-module HINSTANCE
  を同型で移植。class 名は `"RuviformClapTestEditor"`（VST3 の
  `"RuviformTestEditor"` と別名・色も別）。
- **windows-sys edge**: `cfg(windows)` target dep、
  `Win32_Foundation`/`Graphics_Gdi`/`System_LibraryLoader`/
  `UI_WindowsAndMessaging` — VST3 fixture と同一 feature 集合、
  workspace lock 既存の `windows-sys 0.61.2`。Coordinator 承認済み
  （2026-11-15）。
- **L1 e2e は追加しない**: L1 `OpenGui`/`CloseGui` は
  `HostError::Unsupported` by design（`plugin-host/src/l1/mod.rs`）。
  l1 lane は `--test` target 明示 enumerate のため新規 tests/*.rs
  はどの lane からも走らない → editor case は既存
  `tests/host_path.rs` に置く。

## Decision

VST3 `test_editor`（merged PR#668）を先例に、plugin 側のみ実装。
DSP は param 無し pass-through（`fault.rs` の `pass_through` を
`pub(crate)` 共有）。real-HWND success evidence は
`#[ignore]`d Windows test + #631 real-attach lane へ委譲。
