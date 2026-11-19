---
id: "TSPEC-rf389-ime-verbs-01"
type: "test-spec"
status: "approved"
title: "Issue #389 adapter editor verbs テスト仕様"
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
    - "docs/designs/issue-389-ime-verbs.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-ime-verbs.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-ime-verbs.md"
    - "docs/designs/issue-389-ime-verbs.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-389-ime-verbs.md"
  verified-by: []
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 adapter editor verbs テスト仕様

## Purpose

3 verb が「production chain を通る」「plugin refusal は
`EditorOpError::Unsupported` へ honest mapping」「session 状態
遷移・event 系列・DSP 非干渉の契約を守る」ことを headless と
real-window lane の両面で保証する。

## Feature / Rule / Example

### Feature: headless session verb 契約

- **Rule H1**: `set_size`/`set_visibility`/`reattach` は
  `InProcessEditorOps` 経由で `HostCommand` を発行し、plugin の
  受理で `Ok(())`、decline で `EditorOpError::Unsupported` または
  `Plugin` を返す。session は `Attached`/`Hidden` の適切な状態に
  留まる。
- **Example**: `l0_set_size_reaches_plugin_through_setguisize`、
  `l0_set_visibility_cycles_hidden_shown_through_host`、
  `l0_reattach_redeems_lease_and_reaches_set_gui_parent`。
- **Rule H2**: `set_size` refusal は非致命 — session は
  `Attached`/`Hidden` を維持する。
- **Example**: `resize_rejection_is_non_fatal_and_observable`、
  `l0_plugin_verb_refusal_maps_to_unsupported_non_fatally`。
- **Rule H3**: verb 実行を挟んでも DSP lifecycle/`process` は
  無干渉。
- **Example**: `l0_session_open_close_leaves_dsp_processing_
  untouched`（hide→close 後も `Processing`+`Continue`）。
- **Rule H4**: `reattach` は `LeaseRedeemer` を通る — forged/
  stale lease は `SetGuiParent` 発行前に拒否される。
- **Example**: `l0_redeemer_refusal_blocks_reattach_before_
  setguiparent`。
- **Rule H5**: plugin が verb を実装しない（default hook）場合は
  defined `Unsupported` — fake success しない。
- **Example**: `verbs_supported == false` fixture 経路。

### Feature: real lane effect oracle

- **Rule R1** (`real-editor-resize`): accepted `set_size` の後、
  再列挙した全 child HWND の `GetWindowRect` が要求
  `800×600` と一致し、session は `Attached`、events は
  `[Attached{Docked}, Closed]`。plugin-declined resize →
  `Unsupported`。accepted だが geometry 不一致 → `Fail`。
- **Rule R2** (`real-editor-redock`): accepted `reattach` の後、
  pre-op child HWND が全て `IsWindow==0`、新 attach generation
  （title 前進）が container 配下に出現、events は
  `[Attached{Docked}, Detached, Reattached{Docked}, Closed]`。
  refused → `Failed{Unsupported}` + `[Attached{Docked},
  Detached, Failed{..}, Closed]` → `Unsupported`。
- **Rule R3** (`real-editor-visibility`): `set_visibility(false)`
  で `Hidden{Docked}` + pre-hide child 全破棄、
  `set_visibility(true)` で `Attached{Docked}` + 新 generation
  再成立、events は `[Attached{Docked}, Hidden, Shown{Docked},
  Closed]`。hide decline → `Unsupported`（`Attached` 維持）。
- **Rule R4**: accepted + lifecycle verified + effect confirmed
  のみ `Pass`。accepted + 効果未確認 → `NotRun`。契約不一致 →
  `Fail`。platform/observation 非対応 → `Unsupported`。
- **Rule R5**: headless/fake evidence を real-window `Pass` に
  昇格しない。off-Windows では `unsupported` observation を
  記録して verdict を正直に降格させる。

## 検証コマンド

- `cargo test -p ruviform-plugin-host --test editor_session`
- `cargo test -p ruviform-plugin-test-vst3`
- `cargo test -p ruviform-plugin-dock-test --features real-dock`
- `mise run test-ui-plugin-dock-real`（interactive Windows）
- `mise run test-ui-plugin-dock-real-headless`
