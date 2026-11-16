---
id: "SPEC-034ZCLAPED2sl2ceM3kQ2x"
type: "spec"
status: "approved"
title: "Issue #630 CLAP slice — test_editor plugin（clap.ext.gui plugin 側 fixture）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-editor.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/editor.rs"
      - type: "file"
        path: "docs/specs/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/designs/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/plans/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/tasks/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/test-specs/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/discovery/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-test-editor-exp.jsonl"
      - type: "file"
        path: "evidence/rf630/clap-test-editor-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/fault.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-test-editor.md"
    - "docs/specs/issue-630-clap-test-plugin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-630-test-editor.md"
    - "docs/plans/issue-630-test-editor.md"
    - "docs/tasks/issue-630-test-editor.md"
    - "docs/test-specs/issue-630-test-editor.md"
    - "crates/ruviform-plugin-test-vst3/README.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-test-editor.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 CLAP slice — test_editor plugin（clap.ext.gui plugin 側 fixture）

## What Should Be Built

`crates/ruviform-plugin-test-clap` に 5 番目の plugin class
`test_editor`（CLAP id `org.ruviform-test.clap-test-editor`、
vendor `"Ruviform Test"`、version `"0.0.0"`）を追加する。DSP は
parameterless pass-through、GUI は `clap.ext.gui` の plugin 側
実装 — `Shared`/`MainThread`/`AudioProcessor` 3 分割構成の
MainThread object に `PluginGuiImpl` を実装し、Windows では
`set_parent` が host 提供 parent HWND 配下に実 `WS_CHILD` HWND を
生成する。merged VST3 `test_editor`（PR#668）の CLAP 対応物で、
production host bridge（`open_gui`/`close_gui`）は無変更のまま
本番 path から Discover/Instantiate/Process/GUI contract 観測が
通ることを検証する。

## Why Now

Issue #630 受入条件の AC-1 残り分（CLAP 側 `test_editor` の実
ファイルを製品 Host の本番 Path から Discover/Instantiate/
Process/Uninstantiate）と AC-4 の headless 半分（gui 宣言・
open/close・サイズ・親 attach の契約観測）を対象とする、suite
最後の未実装 fixture class の slice。VST3 側は merged 済みで、
CLAP 側 host substrate（`open_gui` の `clap.ext.gui` negotiation）
は実装済み — plugin 側 fixture のみが欠けている。

## Users and Value

Consumers are the Ruviform test infrastructure and the
`scarecrow173/ruviform#631` real-attach lane（CLAP leg 拡張待ち）。
Value: `clap.ext.gui` の plugin 側契約（API negotiation・defined
rejection・実 child HWND 生成）を実 ABI で固定し、host↔plugin の
GUI path を fake 無しで検証できる。

## Scope

### In Scope

- `src/editor.rs`: `TestEditorShared`/`TestEditorMainThread`/
  `TestEditorAudioProcessor`/`TestEditorClapPlugin` の 3 分割構成、
  `declare_extensions` で `PluginAudioPorts` + `PluginGui` 登録、
  `PluginGuiImpl` 実装（is_api_supported/get_preferred_api/create/
  destroy/set_scale/get_size/set_size/set_parent/set_transient/
  show/hide）。
- Windows `win32` module: once-per-module class 登録
  （`DefWindowProcW` + `GetClassInfoW` 確認 + own-module
  `HINSTANCE`）、`IsWindow` pre-check → `CreateWindowExW
  (WS_CHILD|WS_VISIBLE|WS_CLIPSIBLINGS)` → `ShowWindow`/
  `DestroyWindow`。非 Windows は同 module の shim が defined
  rejection に収束。
- `src/lib.rs`: `mod editor` + descriptor index 4 + `plugin_count()`
  4→5 + `create_plugin` dispatch。既存 4 class の descriptor/id/
  挙動は不変。
- `Cargo.toml`: `clack-extensions` に `gui` feature 追加（Spec
  承認済み）+ `[target.'cfg(windows)'.dependencies]` に
  `windows-sys 0.61`（VST3 fixture と同一 feature 集合、
  Coordinator 承認済み）。
- `src/fault.rs`: `pass_through` を `pub(crate)` 化して
  `test_editor` と共有（DSP law の重複排除）。
- `tests/host_path.rs`: descriptor/factory pin 4→5、editor の
  params 無し・pass-through・foreign event drop・opaque-lease
  `open_gui` defined rejection・非 editor class の `Unsupported`
  を追加。
- in-process unit tests: API negotiation 行列・size/resize の
  honest 応答・create/destroy・set_parent defined rejection・
  show/hide・pass-through・`#[ignore]`d real-child-HWND test。
- doc chain + `evidence/rf630/` 検証記録。

### Out of Scope

- production crate（`ruviform-plugin-clap`/`plugin-host`/`plugin-core`
  等）への一切の変更 — host 側 `open_gui`/`close_gui` は実装済み。
- L1 `OpenGui`/`CloseGui` e2e — `HostError::Unsupported` by design
  の本番側 deferral。l1 lane は `--test` target 明示 enumerate の
  ため新規 tests target は走らない。
- real parent HWND + message pump 経由の `open_gui` **success**
  path・composited pixel・focus/input/resize round-trip —
  `scarecrow173/ruviform#631` real-attach lane の責務（現行 lane は
  VST3 cdylib のみ、CLAP leg は lane 拡張待ち）。
- floating window（`is_floating=true`）・`set_transient`/
  `suggest_title` の実 attach — #389 D7 系 deferred、正直な拒否/
  defined 応答に留める。
- Preset 面（AC-2 残解釈）・外部 SDK validator（AC-3）・artifact
  digest 記録（AC-5 残り）・consumer oracle 照合・非 Windows OS
  実走・GUI-L2/EXT-L3 — 後続 slice/Deferred。
- 既存 class（`test_gain`/`test_fault`/`test_stateful`/
  `test_instrument`）の descriptor/id/挙動変更、runner bin 変更、
  `mise.toml`/`docs/**/README.md`/`AGENTS.md`/`POLICIES.md` 変更。

## Requirements

- `clap.ext.gui` を `declare_extensions` で宣言し、
  `PluginGuiImpl` の必須 verb 全てに defined 応答を返す — 成功を
  捏造しない。
- `is_api_supported` は `GuiApiType::WIN32` + `is_floating=false`
  + `cfg!(windows)` のみ true。他（floating・非 win32・非 Windows
  build）は正直に false。
- `set_parent` は wrong API type・null/非 window/opaque payload・
  create 前・二重 embed を全て defined rejection とし、Windows
  では `IsWindow` pre-check 後に実 child HWND を生成する。
- `show`/`hide`/`destroy` は対称・冪等で安全。GUI verb は RT
  process 経路に接続しない。
- `unsafe` は `#[cfg(target_os = "windows")]` win32 module +
  `#[expect(unsafe_code)]` + SAFETY コメントに局所化。
- `test_editor` の process は `rt_` assert_no_alloc 契約を壊さない
  pass-through。

## Acceptance Criteria

- [x] `plugin_count()` 5・descriptor index 4 = `test_editor` が
  production `ClapLibrary` path で Discover/Instantiate される。
- [x] `clap.ext.gui` extension が宣言され、`is_api_supported`
  negotiation が host `open_gui` の要求（WIN32 embedded）に一致。
- [x] `open_gui(ParentLease::new(0,0))` が defined rejection
  （Windows: `PluginFailed` — `IsWindow` gate、非 Windows:
  `Unsupported`）を返し、失敗後も instance が通常 teardown 可能。
- [x] 非 editor class の `open_gui` が `Unsupported` に収束。
- [x] editor が params 無し pass-through（foreign `ParamValue`
  event も drop）で、`rt_` 契約を維持。
- [x] Windows 実機で `set_parent` が実 `WS_CHILD` HWND を生成・
  `destroy` で破棄（`#[ignore]`d test の手動実走 evidence）。

## Deferred Design Concerns

- real parent HWND への `open_gui` success + composited pixel —
  `scarecrow173/ruviform#631` lane 拡張待ち。
- L1 out-of-process editor attach — `HostError::Unsupported` by
  design の本番側 deferral。
- floating window・`set_transient`/`suggest_title` 実 attach —
  #389 D7 系 deferred。
- 外部 SDK validator（clap-validator 等）— License・環境確保後の
  別 slice。
- CLAP `test_editor` 以外の consumer oracle 照合・非 Windows OS
  実走 — Deferred。

## Sources and Relations

- Issue #630 受入条件・slice scoping（Coordinator task spec）。
- approved suite design `docs/designs/issue-630-test-plugin-suite.md`
  と VST3 editor slice chain
  `docs/{specs,designs,plans,tasks,test-specs}/issue-630-test-editor.md`
  + `docs/impl/ir/issue-630-test-editor-slice.md` が直接の先例。
- `crates/ruviform-plugin-clap/src/host.rs`（`open_gui`/`close_gui`、
  参照のみ・zero-diff）と `crates/ruviform-plugin-test-vst3/src/
  editor_view.rs`（win32 module 先例）が substrate。
- `docs/discovery/issue-630-clap-test-editor.md` に探索結果を記録。
