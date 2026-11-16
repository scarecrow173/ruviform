---
id: "TASK-034ZCLAPED5tk5ceM3kQ2x"
type: "task"
status: "completed"
title: "Issue #630 CLAP slice — test_editor plugin（ruviform-plugin-test-clap）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/plans/issue-630-clap-test-editor.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-test-editor.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-test-editor-exp.jsonl"
      - type: "file"
        path: "evidence/rf630/clap-test-editor-verification.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-630-clap-test-editor.md"
  implemented-by:
    - "docs/impl/ir/issue-630-clap-test-editor.md"
  depends-on:
    - "docs/plans/issue-630-clap-test-editor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-clap-test-editor.md"
    - "docs/tasks/issue-630-test-editor.md"
    - "docs/tasks/issue-630-clap-test-plugin.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-630-clap-test-editor.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-clap-test-editor.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 CLAP slice — test_editor plugin（ruviform-plugin-test-clap）

## Work

`crates/ruviform-plugin-test-clap` に 5 番目の plugin class
`test_editor`（`clap.ext.gui` plugin 側実装 + Windows 実 child
HWND）を追加する。`Shared`/`MainThread`/`AudioProcessor` 3 分割
構成の MainThread object に `PluginGuiImpl` を実装し、
`set_parent` が host 提供 parent HWND 配下に実 `WS_CHILD` HWND
を生成する。DSP は paramless pass-through（`fault.rs` の
`pass_through` を `pub(crate)` 共有）。production host bridge
（`open_gui`/`close_gui`）は無変更のまま本番 path から
Discover/Instantiate/Process/GUI contract を観測する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-630-clap-test-editor.md`
- `docs/specs/issue-630-clap-test-editor.md`
- `docs/designs/issue-630-clap-test-editor.md`

## Dependencies

- `docs/tasks/issue-630-test-editor.md`（VST3 editor slice — merged
  先例;no blocking dependency remains）。
- `docs/tasks/issue-630-clap-test-plugin.md`（CLAP fixture 既存
  4 class — merged）。

## Affected Files

- `crates/ruviform-plugin-test-clap/src/editor.rs`（new）
- `crates/ruviform-plugin-test-clap/src/lib.rs`
- `crates/ruviform-plugin-test-clap/src/fault.rs`
- `crates/ruviform-plugin-test-clap/Cargo.toml`
- `crates/ruviform-plugin-test-clap/tests/host_path.rs`
- `crates/ruviform-plugin-test-clap/README.md`
- `Cargo.lock`
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-630-clap-test-editor.*`
- `evidence/rf630/clap-test-editor-verification.md`

## Done When

- [x] `plugin_count()` 5・descriptor index 4 = `test_editor`
      （`org.ruviform-test.clap-test-editor`、vendor
      `"Ruviform Test"`）が production `ClapLibrary` path で
      Discover/Instantiate される。
- [x] `clap.ext.gui` extension が宣言され、`PluginGuiImpl` の
      必須 verb 全てに defined 応答（WIN32 embedded のみ
      supported、floating/非 Windows は正直に拒否）。
- [x] `set_parent` が `IsWindow` pre-check 後に実 `WS_CHILD`
      HWND を生成し、`destroy` で破棄（`#[ignore]`d Windows
      test で実走確認）。
- [x] `open_gui(ParentLease::new(0,0))` が defined rejection
      （Windows `PluginFailed`、非 Windows `Unsupported`）を
      返し、失敗後も instance が通常 teardown 可能。
- [x] 非 editor class の `open_gui` が `Unsupported` に収束。
- [x] editor が params 無し pass-through（foreign `ParamValue`
      event も drop）で、`rt_` 契約を維持。
- [x] `mise run test ruviform-plugin-test-clap`・
      `check/lint/test-l1-real-plugin`・`test-rt`・`prek` 全て
      pass。
- [x] Doc chain + evidence records written;review-protocol clean。

## Verification

- `cargo check --locked --all-targets -p ruviform-plugin-test-clap`
- `mise run lint ruviform-plugin-test-clap`（clippy `-D warnings`）
- `cargo test --locked -p ruviform-plugin-test-clap`（57 pass +
  1 ignored）
- `cargo test --locked -p ruviform-plugin-test-clap -- --ignored`
  （Windows 実機 real-HWND:1 pass）
- `mise run check-l1-real-plugin`/`lint-l1-real-plugin`/
  `test-l1-real-plugin`（全 lane pass）
- `mise run test-rt`（workspace `rt_` tests pass）
- `mise exec -- prek run --all-files`（全 hook pass）
