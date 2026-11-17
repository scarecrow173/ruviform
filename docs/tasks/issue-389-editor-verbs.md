---
id: "TASK-rf389-editor-verbs-01"
type: "task"
status: "in-progress"
title: "Issue #389 adapter editor verbs 実装タスク（set_size/set_visibility/reattach production chain + rf631 lane flip）"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-plugin-editor-session.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-plugin-editor-session.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-389-real-session-lifecycle.md"
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-plugin-editor-session.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  verified-by:
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 adapter editor verbs 実装タスク

## Work

`InProcessEditorOps` の `set_size`/`set_visibility`/`reattach` が
返していた定義済み `EditorOpError::Unsupported` refusal を除去し、
plugin-core GUI trait hook → `HostCommand`/`HostReply` → host
dispatch → composite L0 routing → VST3/CLAP adapter 実装の
production chain を通す。VST3 test fixture を実 resize 可能にし
（`canResize == kResultTrue`、`onSize` が child HWND を resize、
attach generation を window title `rvf-test-editor-attach-{n}` に
刻印）、`rf631.plugin-dock-real` lane の
`real-editor-resize`/`real-editor-redock`/`real-editor-visibility`
を refusal/deferred から実実行 + OS-level effect oracle 検証へ
flip する。L1/out-of-process は refusal-only を維持する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-389-plugin-editor-session.md`
- `docs/designs/issue-389-plugin-editor-session.md`
- `docs/tasks/issue-389-real-session-lifecycle.md`（lane 先例）

## Dependencies

- `crates/ruviform-plugin-host` session/ops API
- `crates/ruviform-plugin-test-vst3` `test_editor` fixture
  （`mise run build ruviform-plugin-test-vst3` で cdylib 生成）
- `RealOsGuiTest`/`oschild` Win32 観測面（read-only 参照 + 拡張）

## Affected Files

- `crates/ruviform-plugin-core/src/{boundary,lib,plugin}.rs`
- `crates/ruviform-plugin-host/src/{command,composite,editor_session,host,lib}.rs`
- `crates/ruviform-plugin-host/tests/editor_session.rs`
- `crates/ruviform-plugin-vst3/src/host.rs`
- `crates/ruviform-plugin-clap/src/host.rs`
- `crates/ruviform-plugin-test-vst3/src/editor_view.rs`
- `crates/ruviform-plugin-dock-test/src/{real_dock,real_catalog,lib}.rs`
- `crates/ruviform-plugin-dock-test/README.md`
- `docs/tasks/issue-389-editor-verbs.md`・`docs/impl/ir/`
- `evidence/rf389/`・`evidence/rf631/`

## Done When

- [x] `InProcessEditorOps::{set_size,set_visibility,reattach}` が
  defined `Unsupported` を返さず production chain を通る
  （plugin refusal は `HostError::Unsupported` →
  `EditorOpError::Unsupported` の honest mapping を維持）。
- [x] VST3 adapter が `canResize`/`checkSizeConstraint`/`onSize`・
  `removed()`+`setFrame`/`attached`・remove+reattach
  （失敗時 old parent best-effort restore）を実装し、CLAP adapter
  が `can_resize`/`set_size`・`hide`/`show`・`set_parent` を実装。
- [x] headless `editor_session` test が accepted verbs・DSP 非干渉・
  refusal semantics を検証して green（54/54）。
- [x] `real-editor-resize` が `GetWindowRect == 800×600` の effect
  oracle で `pass`、`real-editor-redock` が旧 child HWND 破棄 +
  新 attach generation で `pass`、`real-editor-visibility` が
  hide→destroy/show→recreate で `pass`（実走 JSONL 記録）。
- [x] L1 `L1EditorOps` は全 op refusal-only のまま無変更。
- [x] `docs/impl/ir` が実績で更新され、review-protocol 1 cycle・
  blocker 0。

## Verification

- `cargo fmt --all -- --check` / `cargo clippy`（touch crate 全て
  `--all-targets`、dock-test は `--features real-dock`）— warning 0。
- `cargo test -p ruviform-plugin-host --test editor_session` —
  54/54。
- `cargo test -p ruviform-plugin-test-vst3` — 全 green。
- `cargo test -p ruviform-plugin-dock-test [--features real-dock]`
  — 全 green。
- `mise run test-ui-plugin-dock-real-headless` — 22 row 全て
  `not_run` + `environment_blocked`、violation 0、exit 0。
- `mise run test-ui-plugin-dock-real` — 実走。3 row が `pass`
  （effect oracle 確認済み）、`real-dcomp-pixel` の既知 `fail`
  （#532・対象外）で設計通り exit 1。
- 記録: `evidence/rf389/verification-editor-verbs.md`。
