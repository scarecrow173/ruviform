---
id: "TASK-034Y13NY1bznQbU0TgCsEn"
type: "task"
status: "completed"
title: "Issue #630 test_editor slice — task"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-630-test-editor.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-test-editor.md"
  implements:
    - "docs/plans/issue-630-test-editor.md"
  implemented-by:
    - "docs/impl/ir/issue-630-test-editor-slice.md"
  depends-on:
    - "docs/plans/issue-630-test-editor.md"
    - "docs/tasks/issue-630-test-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-editor.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-630-test-editor.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-test-editor.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_editor slice — task

## Work

`ruviform-plugin-test-vst3` に `test_editor` class pair と
plugin 側 `IPlugView`（Windows real child HWND）を追加する。
factory を 4→6 class に拡張し、production host `open_gui` が
要求する `createView("editor")`/`isPlatformTypeSupported`/
`setFrame`/`attached`/`removed` の契約を満たす。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-630-test-editor.md`
- `docs/specs/issue-630-test-editor.md`
- `docs/designs/issue-630-test-editor.md`
- `docs/test-specs/issue-630-test-editor.md`

## Dependencies

- `docs/tasks/issue-630-test-fault.md`（multi-class factory・
  generic `TestComponent<P>`/`ParamController` shell が前提）
- Issue #389 host attach substrate（`open_gui`/`BridgePlugFrame`/
  `ParentLease` — 実装済み・変更なし）

## Affected Files

- `crates/ruviform-plugin-test-vst3/src/editor_view.rs`（新規）
- `crates/ruviform-plugin-test-vst3/src/editor.rs`（新規）
- `crates/ruviform-plugin-test-vst3/src/controller.rs`
- `crates/ruviform-plugin-test-vst3/src/factory.rs`
- `crates/ruviform-plugin-test-vst3/src/strings.rs`
- `crates/ruviform-plugin-test-vst3/src/lib.rs`
- `crates/ruviform-plugin-test-vst3/src/fault.rs`
- `crates/ruviform-plugin-test-vst3/Cargo.toml`
- `crates/ruviform-plugin-test-vst3/README.md`
- `crates/ruviform-plugin-test-vst3/tests/host_path.rs`
- `docs/*` doc chain + `docs/*/README.md` index 再生成
- `evidence/rf630/test-editor-verification.md`

## Done When

- [x] `test_editor` descriptor が `Vst3Library` scan で 3 件目の
      component として発見される（`countClasses == 6`）
- [x] `createView("editor")` → owned `IPlugView`、他 name → null
- [x] `IPlugViewTrait` 全 method の COM-level 契約 test が green
- [x] headless `open_gui` が Windows で `Err(PluginFailed)`、
      非 Windows で `Err(Unsupported)`
- [x] 既存 gain/fault lane が全て green のまま
- [x] doc chain + evidence + index 再生成完了

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check ruviform-plugin-test-vst3`
- [x] `mise run lint ruviform-plugin-test-vst3`
- [x] `mise run test ruviform-plugin-test-vst3`
- [x] `mise run check-l1-real-plugin` / `lint-l1-real-plugin` /
      `test-l1-real-plugin`
- [x] `mise run test-rt`
- [ ] `mise exec -- prek run --all-files`
- [ ] `git diff` forbidden path 空
- [ ] review-protocol review で blocker なし
