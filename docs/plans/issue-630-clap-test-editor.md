---
id: "PLAN-034ZCLAPED4pl4ceM3kQ2x"
type: "plan"
status: "approved"
title: "Issue #630 CLAP test_editor slice — 実装計画 Implementation Plan"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/designs/issue-630-clap-test-editor.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-clap-test-editor.md"
  implements:
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/designs/issue-630-clap-test-editor.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-630-clap-test-editor.md"
    - "docs/specs/issue-630-clap-test-editor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-630-clap-test-editor.md"
    - "docs/test-specs/issue-630-clap-test-editor.md"
    - "docs/plans/issue-630-test-editor.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/designs/issue-630-clap-test-editor.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 CLAP test_editor slice — 実装計画 Implementation Plan

## Goal

`crates/ruviform-plugin-test-clap` に `test_editor`（`clap.ext.gui`
plugin 側実装 + Windows 実 child HWND）を追加し、production
`ClapLibrary` host path から Discover/Instantiate/Process/GUI
contract を観測可能にする。host crate は無変更。

## Scope

In Scope: `src/editor.rs` 新規、`src/lib.rs`/`Cargo.toml`/
`src/fault.rs`/`tests/host_path.rs`/`README.md` 更新、
`Cargo.lock`、doc chain、evidence。
Out of Scope: production crate 変更、real parent HWND success
（#631）、L1 OpenGui/CloseGui、floating window、外部 validator、
非 Windows 実走、consumer oracle。

## Upstream Documents

- `docs/specs/issue-630-clap-test-editor.md`
- `docs/designs/issue-630-clap-test-editor.md`
- `docs/discovery/issue-630-clap-test-editor.md`

## Design Inputs

- VST3 `test_editor` slice（merged PR#668）: `editor.rs`/
  `editor_view.rs`/`tests/host_path.rs` の先例。
- CLAP 既存 fixture: `fault.rs`/`stateful.rs`/`gain.rs`/`plugin.rs`
  の 3 分割構成・`declare_extensions`・`PluginProcessor`。
- production host: `crates/ruviform-plugin-clap/src/host.rs`
  `open_gui`/`close_gui`（参照のみ・zero-diff）。
- clack-extensions 0.2.0 `gui` feature: `PluginGuiImpl`/`Window`/
  `GuiConfiguration`/`GuiSize`/`GuiApiType`。

## File Map

| path | 変更 |
| --- | --- |
| `crates/ruviform-plugin-test-clap/src/editor.rs` | 新規: 3 分割 + `PluginGuiImpl` + win32 module + unit tests |
| `crates/ruviform-plugin-test-clap/src/lib.rs` | `mod editor`、descriptor index 4、`plugin_count()` 5、dispatch |
| `crates/ruviform-plugin-test-clap/src/fault.rs` | `pass_through` を `pub(crate)` 化 |
| `crates/ruviform-plugin-test-clap/Cargo.toml` | `clack-extensions` `gui` feature、`[target.'cfg(windows)'.dependencies]` `windows-sys` |
| `crates/ruviform-plugin-test-clap/tests/host_path.rs` | descriptor/factory pin 5、editor cases、opaque-lease rejection、非 editor `Unsupported` |
| `crates/ruviform-plugin-test-clap/README.md` | 5-plugin factory・editor 説明 |
| `Cargo.lock` | `windows-sys` target edge のみ |
| `docs/{discovery,specs,designs,plans,tasks,test-specs}/issue-630-clap-test-editor.*` | doc chain |
| `docs/impl/{ir,exp}/issue-630-clap-test-editor.*` | impl/ir + exp + companion jsonl |
| `evidence/rf630/clap-test-editor-verification.md` | 検証記録 |

## Implementation Sequence

1. `src/fault.rs`: `pass_through` を `pub(crate)` 化（共有 law）。
2. `src/editor.rs`: `TestEditorShared`/`TestEditorMainThread`/
   `TestEditorAudioProcessor`/`TestEditorClapPlugin` +
   `PluginGuiImpl` + win32 module + `#[cfg(test)]` tests。
3. `src/lib.rs`: `mod editor` + descriptor index 4 +
   `plugin_count()` 5 + dispatch。
4. `Cargo.toml`: `gui` feature + windows-sys target dep。
5. `tests/host_path.rs`: pin 5 + editor cases。
6. `README.md` 更新。
7. `cargo check`/`clippy`/`test`/`test --ignored`（Windows）/
   `check-l1-real-plugin`/`lint-l1-real-plugin`/
   `test-l1-real-plugin`/`test-rt`/`prek`。
8. doc chain + evidence + review-protocol。
9. commit/push/PR/`worker_done`。

## Dependencies and Risks

- `clack-extensions =0.2.0` `gui` feature（Spec 承認済み）。
- `windows-sys 0.61` target dep（Coordinator 承認済み、
  workspace lock 既存 `0.61.2`）。
- Risk: `IsWindow` pre-check 省略で opaque payload が HWND と
  誤認 → `set_parent` に `IsWindow` gate を必須化。
- Risk: module-local wnd_proc が cdylib unload 後に dangling →
  `DefWindowProcW` 採用。
- Risk: `PluginGuiImpl` `&self` + RefCell borrow → `try_borrow_mut`
  で defined rejection 収束。

## Task Breakdown

単一 slice（`test_editor` fixture 追加）— 分割不要。VST3 先例と
同じく 1 task/1 PR。

## Verification

- `cargo check --locked --all-targets -p ruviform-plugin-test-clap`
- `mise run lint ruviform-plugin-test-clap`（clippy `-D warnings`）
- `cargo test --locked -p ruviform-plugin-test-clap`
- `cargo test --locked -p ruviform-plugin-test-clap -- --ignored`
  （Windows 実機 real-HWND）
- `mise run check-l1-real-plugin`/`lint-l1-real-plugin`/
  `test-l1-real-plugin`
- `mise run test-rt`
- `mise exec -- prek run --all-files`

## Open Questions / Gaps

- real parent HWND + message pump 経由の `open_gui` success —
  #631 lane 拡張待ち（NotRun）。
- 非 Windows OS 実走 — Unsupported（Windows session のみ）。
- 外部 SDK validator — Deferred。

## Self-Review

- spec/design の全 Requirements が file map/sequence に対応。
- host crate zero-diff を確認（`ruviform-plugin-clap`/`plugin-host`
  は参照のみ）。
- `windows-sys` は target dep のみ、`gui` feature は Spec 承認済み。

## Implementation Handoff

実装済み・全検証 lane green。`#[ignore]`d real-HWND test は
Windows 実機で手動実走済み（evidence 記録）。

## Review Handoff

review-protocol で spec-conformance + defect-search の 2 pass を
実施し、blocker 無しを確認。
