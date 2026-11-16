---
id: "TSPEC-034ZCLAPED6ts6ceM3kQ2x"
type: "test-spec"
status: "approved"
title: "Issue #630 CLAP test_editor slice — test specification"
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
        path: "docs/test-specs/issue-630-clap-test-editor.md"
  implements:
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/designs/issue-630-clap-test-editor.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-editor.md"
    - "docs/test-specs/issue-630-clap-test-plugin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-630-clap-test-editor.md"
    - "docs/tasks/issue-630-test-editor.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/designs/issue-630-clap-test-editor.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/designs/issue-630-clap-test-editor.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 CLAP test_editor slice — test specification

## Purpose

`test_editor` class と plugin 側 `PluginGuiImpl` が production
host `open_gui`/`close_gui` の要求契約（extension negotiation・
create・set_parent・show/hide・destroy）を満たすことを、実 ABI
（clack `clap_entry` vtbl dispatch）と production `ClapLibrary`
bridge の両経路で固定する。real parent HWND への attach 成功は
本 spec の対象外 — #631 real-dock slice が別途検証する。

## Feature: test_editor fixture discovery (in-process, `tests/host_path.rs`)

- **Rule**: `test_editor` は既存 4 class と同一 cdylib/factory
  に第 5 component として発見される。
  **Example**: `ClapLibrary::load` → `descriptors().len() == 5`、
  index 4 の id が `org.ruviform-test.clap-test-editor`、
  name `"test_editor"`、vendor `"Ruviform Test"`、version
  `"0.0.0"`。`factories().len() == 5`。
- **Rule**: `test_editor` は param を持たない
  （`params()` が空）。
- **Rule**: component は pass-through — stereo block で bitwise
  copy を返し、foreign `ParamValue` event は drop する。

## Feature: `clap.ext.gui` extension declaration (in-crate unit test)

- **Rule**: `declare_extensions` が `PluginGui` を登録し、
  `is_api_supported` は `GuiConfiguration{api_type: WIN32,
  is_floating: false}` のみ true を返す。floating・非 WIN32・
  非 Windows build は全て false（honest capability gate）。
- **Rule**: `get_preferred_api` は `GuiApiType::WIN32` を返す。

## Feature: `PluginGuiImpl` lifecycle (in-crate unit test)

- **Rule**: `create` → `GuiState` 初期化、size 640×480 固定。
  `destroy` は child HWND を破棄して state をクリアし冪等。
- **Rule**: `get_size` は `GuiSize{width:640,height:480}`、
  `can_resize` は false、`set_size` は defined rejection。
  `set_scale` は state に保持して Ok。
- **Rule**: `set_parent` は wrong API type・create 前・二重
  embed・null/非 window/opaque payload を全て defined
  rejection。Windows では `IsWindow` pre-check 後に実
  `WS_CHILD|WS_VISIBLE|WS_CLIPSIBLINGS` child HWND を生成。
- **Rule**: `show`/`hide` は attach 済み child に対して
  `ShowWindow`/`SW_SHOW`/`SW_HIDE`。未 attach・未 create は
  defined rejection。
- **Rule**: `set_transient` は defined rejection（floating 非
  対応の正直な拒否）。`suggest_title` は default に委譲。

## Feature: real child HWND creation (Windows-only, `#[ignore]`d)

- **Rule**: Windows 実機で `set_parent` が実 `WS_CHILD` HWND
  を生成し、`IsWindow`/`GetParent`/`GetWindowLong(GWL_STYLE)`
  で `WS_CHILD` 関係・parent 関係・live window status を観測
  できる。`destroy` 後は `IsWindow` が false に収束。
- **Rule**: default `cargo test` に real-HWND success を要求
  しない — headless 環境でも defined rejection 側のみを必須
  とする（`#[ignore]` で Windows 実機手動実走のみ）。

## Feature: headless `open_gui` through production bridge (`tests/host_path.rs`)

- **Rule**: `open_gui(ParentLease::new(0,0))` は payload 0 が
  real HWND でないため成功を偽装しない。Windows では
  `set_parent` の `IsWindow` gate → `PluginFailed`、非 Windows
  では `EDITOR_GUI_API=None` → `Unsupported`。
- **Rule**: `open_gui` 失敗後の instance は破損しない —
  `stop_processing`/`deactivate`/`terminate` が通常どおり
  収束する。
- **Rule**: 非 editor class（`test_gain`/`test_fault`/
  `test_stateful`/`test_instrument`）の `open_gui` は
  `clap.ext.gui` 無し → `Unsupported` に収束。

## Guarantees

- `clap.ext.gui` は `declare_extensions` 経由でのみ宣言され、
  plugin 側実装は `PluginGuiImpl` trait のみ — host 側
  `ruviform-plugin-clap` は zero-diff。
- Win32 呼出は `#[cfg(target_os = "windows")]` win32 module +
  `#[expect(unsafe_code)]` + SAFETY コメントに局所化。同一
  cdylib が macOS/Linux でも build できる。
- fake lane の lease payload は real HWND でないため
  `set_parent` → `IsWindow` gate で defined rejection — fake
  attach が Pass を偽装しない。
- `process` は `fault.rs` の `pass_through` を共有し、
  `rt_` `assert_no_alloc` 契約を維持。

## Non-goals

- real parent HWND への attach 成功 + composited pixel 判定
  （#631 real-dock slice の証拠軸）。
- `set_transient`/`suggest_title` の実 attach、floating
  window、resize round-trip、focus/input 転送。
- L1 editor attach、外部 SDK validator、非 Windows GUI 実走、
  consumer oracle 照合。

## Risk

- `IsWindow` pre-check を省略すると opaque payload が
  `CreateWindowExW` に流れ、fake attach が誤って成功し得る —
  本 spec はこの gate を必須要件として固定する。
- `PluginGuiImpl` `&self` verb + `RefCell` borrow — host 再入で
  `try_borrow_mut` が `Err` となり panic ではなく defined
  rejection に収束することを必須とする。
