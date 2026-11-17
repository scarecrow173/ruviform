---
id: "SPEC-rf389-ime-verbs-01"
type: "spec"
status: "approved"
title: "Issue #389 adapter editor verbs（InProcessEditorOps set_size/set_visibility/reattach production chain + rf631 lane verb 行 flip）"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "POLICIES.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-389-ime-verbs.md"
      - type: "doc"
        path: "docs/designs/issue-389-ime-verbs.md"
      - type: "doc"
        path: "docs/test-specs/issue-389-ime-verbs.md"
      - type: "doc"
        path: "docs/plans/issue-389-ime-verbs.md"
      - type: "doc"
        path: "docs/tasks/issue-389-ime-verbs.md"
      - type: "doc"
        path: "docs/impl/ir/issue-389-ime-verbs.md"
      - type: "doc"
        path: "docs/impl/exp/issue-389-ime-verbs.md"
      - type: "doc"
        path: "docs/impl/exp/issue-389-ime-verbs-exp.jsonl"
      - type: "doc"
        path: "docs/discovery/issue-389-ime-verbs.md"
      - type: "doc"
        path: "evidence/rf389/"
    modified:
      - type: "doc"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-core/src/plugin.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-host/src/command.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-host/src/editor_session.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-host/tests/editor_session.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-test-vst3/src/editor_view.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "doc"
        path: "docs/designs/issue-389-plugin-editor-session.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-389-ime-verbs.md"
    - "docs/plans/issue-389-ime-verbs.md"
  depends-on:
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/specs/issue-389-real-session-lifecycle.md"
    - "docs/specs/issue-631-real-editor-attach.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-389-ime-verbs.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-plugin-editor-session.md"
  derived-by:
    - "docs/designs/issue-389-ime-verbs.md"
    - "docs/test-specs/issue-389-ime-verbs.md"
    - "docs/plans/issue-389-ime-verbs.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-ime-verbs.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 adapter editor verbs 仕様

## 背景・目的

`PluginEditorSession` の `EditorSessionOps` verb 面のうち、
`set_size`/`set_visibility`/`reattach` は `InProcessEditorOps`
（L0 in-process adapter）が「No `HostCommand` resize verb exists
yet」の defined `EditorOpError::Unsupported` refusal を返す
placeholder だった（`editor_session.rs`）。`rf631.plugin-dock-real`
lane の `real-editor-resize`/`real-editor-redock`/
`real-editor-visibility` 3 row はこの refusal を観測する
deferred/refusal row として catalog に存在した。

本 spec はこれら 3 verb を L0 production chain
（plugin-core GUI trait → `HostCommand` → host dispatch →
composite L0 routing → VST3/CLAP adapter）へ実装し、lane の
3 row を「受理 + OS-level effect oracle 確認」で `Pass` 判定
可能にする slice を定義する。Issue `scarecrow173/ruviform#389`
（tracker — auto-close しない）の一部。

## スコープ

### 対象

- plugin-core: `GuiSize` vocabulary と `Plugin`/`ErasedPlugin`
  の `set_gui_size`/`set_gui_visibility`/`set_gui_parent` hook
  （default defined `Unsupported` — additive、既存 Plugin 実装は
  無変更でコンパイル可能）。
- plugin-host: `HostCommand::SetGuiSize`/`SetGuiVisibility`/
  `SetGuiParent`（L0 surface のみ）、`InProcessHost` dispatch、
  `InProcessEditorOps` の 3 verb 実装。`reattach` は `open` と
  同じ `LeaseRedeemer` seam で `ParentSlotLease` を検証してから
  `ParentLease` を plugin へ渡す。
- composite: L0 verb を `OutOfProcess` slot へ流さず defined
  `HostError::Unsupported` を返す。`wire::Command` 追加・
  `WIRE_VERSION` bump は対象外（L1 GUI verb は別 Issue 級）。
- VST3 adapter: `canResize`→`checkSizeConstraint`→`onSize`、
  hide=`removed()`/show=`setFrame`+`attached` 再構成、
  `set_gui_parent`=remove+新 parent attach（失敗時 old parent
  best-effort restore）。Windows のみ、非 Windows attach は
  honest `Unsupported` を維持。
- CLAP adapter: `PluginGui::can_resize`/`set_size`、`hide`/`show`、
  `set_parent`。
- test-vst3 fixture: `canResize == kResultTrue`、`onSize` が
  child HWND を resize、attach generation を window title
  `rvf-test-editor-attach-{n}` に刻印（redock/visibility の
  generation oracle 用）。
- headless contract test: accepted verb・refusal semantics・
  DSP 非干渉の更新。
- rf631 lane: 3 row の observe fn を `real_editor_ops`（実
  `RealOsGuiTest::redeem_native_lease` 経由）で実行し、
  OS-level effect oracle を追加。verdict 写像に
  `effect_confirmed` を導入し accepted+効果確認のみ `Pass`。

### 対象外

- L1/out-of-process GUI verb（`L1EditorOps` は全 op refusal-only
  のまま。`wire::Command`・`WIRE_VERSION` 変更なし）。
- `AttachMode::Floating`（依然 `Unsupported`）。
- plugin→host GUI request、crash/hang 受入、pixel gate、
  consumer gate（issue #389 後続 slice）。
- `docs/**/README.md` index 再生成（Coordinator が統合時に実施）。

## 要件

- R1: 3 verb は plugin/core/host/adapter の production chain を
  通り、adapter 成功時のみ session 状態が遷移する。
  `HostError::Unsupported` → `EditorOpError::Unsupported` の
  honest mapping を維持する。
- R2: `set_size` の plugin refusal は session 非致命
  （`Attached`/`Hidden` 維持）。`set_visibility` は adapter 成功
  後のみ `Attached↔Hidden` を遷移し、no-op は event を発火
  しない。`reattach` は documented の detach/reattach・failure
  event 系列を維持する。
- R3: raw OS handle は session boundary を越えない。
  `ParentSlotLease` → `ParentLease` の redeem は issuer 側
  seam（`LeaseRedeemer`）のみ。
- R4: lane verdict は「accepted + lifecycle verified + effect
  oracle confirmed」のみ `Pass`。defined refusal → `Unsupported`、
  accepted だが効果未確認 → `NotRun`、契約不一致 → `Fail`。
- R5: 既存の lease/generation boundary・DSP/RT 非干渉・
  fail-closed semantics を回帰させない。

## 受入条件

- [x] headless `editor_session` test が accepted verbs・refusal・
  DSP 非干渉を検証して green。
- [x] `real-editor-resize`/`real-editor-redock`/
  `real-editor-visibility` が interactive Windows 実走で `Pass`
  （effect oracle 実測）または環境不可時 `NotRun`+
  `environment_blocked` を正直に記録。
- [x] `test-ui-plugin-dock-real-headless` が全 row
  `not_run`+`environment_blocked`・violation 0。
- [x] L1 `L1EditorOps` の refusal-only が不変。
