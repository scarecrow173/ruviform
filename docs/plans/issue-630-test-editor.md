---
id: "PLAN-034Y13aQ1WvmdD4BN426PS"
type: "plan"
status: "approved"
title: "Issue #630 test_editor slice — 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-test-editor.md"
  implements:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
  implemented-by:
    - "docs/tasks/issue-630-test-editor.md"
  depends-on:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
    - "docs/plans/issue-630-test-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-630-test-editor.md"
    - "docs/test-specs/issue-630-test-editor.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-630-test-editor.md"
  verified-by:
    - "docs/test-specs/issue-630-test-editor.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_editor slice — 実装計画 Implementation Plan

## Goal

`ruviform-plugin-test-vst3` cdylib に `test_editor` class pair
（pass-through component + `createView("editor")` を持つ
`IEditController`）と plugin 側 `IPlugView`（Windows real child
HWND）を追加し、production host `open_gui` 経路の plugin 側前提を
満たす。

## Scope

単一 implementation stream。VST3 crate 内に閉じ、production
crate・CLAP・dock-test・mise lane への変更なし。

## Upstream Documents

- `docs/specs/issue-630-test-editor.md`

## Design Inputs

- `docs/designs/issue-630-test-editor.md`
- `docs/designs/issue-630-test-plugin-suite.md`（suite 規約）
- `docs/designs/issue-630-test-fault.md`（multi-class factory 先例）
- `docs/designs/issue-389-vst3-l0-editor-attach.md`（host attach 契約）

## File Map

- `crates/ruviform-plugin-test-vst3/src/editor_view.rs` — 新規:
  `TestEditorView`（`IPlugViewTrait` 全 method）+ `#[cfg(windows)]
  mod win32`（class 登録・child HWND create/destroy・paint）。
- `crates/ruviform-plugin-test-vst3/src/editor.rs` — 新規:
  `TestEditorProcessor`（pass-through、event 無視）+
  `TestEditorComponent`（`TestComponent<P>` alias + CID）。
- `crates/ruviform-plugin-test-vst3/src/controller.rs` —
  `TestEditorController` + `EDITOR_CONTROLLER_CID` +
  `createView("editor")` → owned `IPlugView`。
- `crates/ruviform-plugin-test-vst3/src/factory.rs` —
  `countClasses` 6、index 4/5 の class-info/createInstance arms。
- `crates/ruviform-plugin-test-vst3/src/strings.rs` —
  `cstring_eq` bounded helper。
- `crates/ruviform-plugin-test-vst3/src/lib.rs` — `mod editor` +
  `mod editor_view` + doc 更新。
- `crates/ruviform-plugin-test-vst3/src/fault.rs` —
  `pass_through` を `pub(crate)` 化（editor と共有）。
- `crates/ruviform-plugin-test-vst3/Cargo.toml` — 承認済み
  `windows-sys` edge。
- `crates/ruviform-plugin-test-vst3/tests/host_path.rs` —
  descriptor 2→3 更新 + editor case。
- `crates/ruviform-plugin-test-vst3/README.md` — class 一覧更新。

## Implementation Sequence

- [ ] `strings.rs` に `cstring_eq` 追加
- [ ] `fault.rs` `pass_through` を `pub(crate)` 化
- [ ] `editor.rs`（processor + component + CID + unit test）
- [ ] `editor_view.rs`（`IPlugView` + win32 block + unit test）
- [ ] `controller.rs`（`TestEditorController` + CID）
- [ ] `factory.rs`（6 class arms）
- [ ] `lib.rs`/`Cargo.toml`/`README.md`
- [ ] `tests/host_path.rs` 更新
- [ ] `cargo test`/`clippy`/`fmt` で in-crate green 確認

## Dependencies and Risks

- `windows-sys 0.61.2` は workspace lock に既存 — `Cargo.lock`
  差分は本 crate dep list の edge 追加のみの想定。
- `open_gui` の `attached` は `IsWindow` pre-check で null/非 HWND
  を落とす — `ParentLease::new(0,0)` は headless-safe に
  `Err(PluginFailed)`（Windows）/ `Err(Unsupported)`（非 Windows）。
- COM-level `IPlugView` test は in-crate unit test（同一
  `ComWrapper`/`ComPtr` vtbl machinery — cdylib 経由ではないが
  ABI 面は同一実装）。real cdylib 経由の attach 成功は #631
  real-dock slice の証拠軸として分離記録。

## Task Breakdown

- `docs/tasks/issue-630-test-editor.md` に集約（単一 task）。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check ruviform-plugin-test-vst3`
- [ ] `mise run lint ruviform-plugin-test-vst3`
- [ ] `mise run test ruviform-plugin-test-vst3`
- [ ] `mise run check-l1-real-plugin`
- [ ] `mise run lint-l1-real-plugin`
- [ ] `mise run test-l1-real-plugin`（既存 gain/fault e2e 緑維持）
- [ ] `mise run test-rt`
- [ ] `mise exec -- prek run --all-files`
- [ ] `git diff` forbidden path 空
- [ ] COM-level/headless editor case が実際に収集・実行される

## Open Questions / Gaps

- real parent HWND での `attached` 成功証拠 — 本 slice 範囲外、
  #631 real-dock slice が取得（default `cargo test` には載せない）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 worker task）
- Independent work streams: none
- User approval required before dispatch: no（dispatch 済み task spec が承認相当）

## Review Handoff

- `windows-sys` edge の feature 集合が承認範囲と一致すること、
  `unsafe` が window-management block に限定されること、
  fake-lane/production crate への zero-diff を reviewer が確認。
