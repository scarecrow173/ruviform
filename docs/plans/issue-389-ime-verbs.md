---
id: "PLAN-rf389-ime-verbs-01"
type: "plan"
status: "approved"
title: "Issue #389 adapter editor verbs 実装計画"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "docs/specs/issue-389-ime-verbs.md"
    - "docs/designs/issue-389-ime-verbs.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-ime-verbs.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-ime-verbs.md"
  blocks:
    - "docs/tasks/issue-389-ime-verbs.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-389-ime-verbs.md"
    - "docs/test-specs/issue-389-ime-verbs.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-ime-verbs.md"
    - "docs/designs/issue-389-ime-verbs.md"
  derived-by:
    - "docs/tasks/issue-389-ime-verbs.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-ime-verbs.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 adapter editor verbs 実装計画

## Goal

`InProcessEditorOps` の 3 verb の defined refusal を production
chain 実装へ置き換え、rf631 lane の 3 row を実実行 +
effect oracle 検証へ flip する。

## Task 分解

1. **core seam**: `GuiSize`（`boundary.rs`）+ `Plugin`/
   `ErasedPlugin` の 3 verb hook（default `Unsupported`）+
   `lib.rs` re-export。
2. **host seam**: `HostCommand` 3 variant + `command_handle`/
   `retarget` 機械追従、`InProcessHost` dispatch
   （`set_gui_size`/`set_gui_visibility`/`set_gui_parent` —
   Plain/Shared slot）、`InProcessEditorOps` 実装
   （`reattach` は redeemer 経由）。
3. **composite guard**: `l0_only_gui_verb` で `OutOfProcess`
   slot の defined `Unsupported` を `to_wire` 到達前に返す。
4. **adapters**: VST3（`canResize`/`checkSizeConstraint`/`onSize`・
   `removed()`/`setFrame`+`attached`・remove+reattach+restore）と
   CLAP（`can_resize`/`set_size`・`hide`/`show`・`set_parent`）。
5. **fixture**: VST3 `test_editor` の `canResize=kResultTrue`、
   `onSize` の child HWND resize、attach generation title 刻印。
6. **headless tests**: accepted verb・refusal・DSP 非干渉の更新
   （`verbs_supported` flag で refusal 経路も維持検証）。
7. **lane**: `ChildWindowFacts` に `width`/`height`/`title`、
   oschild Windows impl に `GetWindowRect`/`GetWindowTextW`/
   `is_alive`、`ObserveOpenFn` 系に `&RealOsGuiTest` を通し、
   3 observe fn を `real_editor_ops` + effect oracle で書き換え、
   `EditorOpVerification.effect_confirmed` +
   `EditorOpIdentity` + `editor_op_body` の `Pass` arm を追加、
   doc/README/catalog 記述を追従。
8. **docs/evidence**: task/impl-ir/test-spec/discovery/exp chain、
   `evidence/rf389/verification.md` 追記、canonical lane JSONL
   再生成、design doc の capability matrix/verification-status
   reconcile。

## 検証行列

| 層 | 検証 |
| --- | --- |
| core/host | `cargo test -p ruviform-plugin-host --test editor_session [--test editor_session_driver]` |
| adapter | `cargo test -p ruviform-plugin-{vst3,clap,test-vst3}` |
| lane compile | `cargo test -p ruviform-plugin-dock-test [--features real-dock]` |
| real lane | `mise run test-ui-plugin-dock-real`（interactive Windows） |
| fail-closed | `mise run test-ui-plugin-dock-real-headless` |
| lint/fmt | `cargo clippy --all-targets`（dock-test は `--features real-dock`）+ `cargo fmt --check` |

## リスクと rollback

- VST3 hide=detach の意味差（plugin UI state 喪失）は format
  制約 — design doc に documented、CLAP の真の hide/show との差は
  adapter 実装コメントに明記。
- `set_gui_parent` の new-attach 失敗時 restore が失敗しうる —
  best-effort であり結果は `HostError` で正直に返る。
- lane の effect oracle は fixture の window title/geometry 契約に
  依存 — fixture が title を変えた場合は oracle 側の追従が必要。
- rollback は verb 実装を defined refusal へ戻す commit revert で
  可能（additive hook のため既存 Plugin 実装への影響なし）。
