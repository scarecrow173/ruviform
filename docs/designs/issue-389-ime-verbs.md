---
id: "DESIGN-rf389-ime-verbs-01"
type: "design"
status: "approved"
title: "Issue #389 adapter editor verbs 設計"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "docs/specs/issue-389-ime-verbs.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-389-ime-verbs.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-ime-verbs.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-389-ime-verbs.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
    - "docs/designs/issue-389-clap-l0-editor-attach.md"
  blocks:
    - "docs/plans/issue-389-ime-verbs.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-389-ime-verbs.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-ime-verbs.md"
  derived-by:
    - "docs/plans/issue-389-ime-verbs.md"
    - "docs/test-specs/issue-389-ime-verbs.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-ime-verbs.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 adapter editor verbs 設計

## 概要

`EditorSessionOps` の 3 verb を L0 production chain へ通す。
verb の語彙は driver-neutral（`GuiSize`/`bool`/`ParentSlotLease`）、
format 固有の意味は adapter が解釈する。session 状態機械・
lease/generation boundary・honest refusal semantics は
`issue-389-plugin-editor-session` 設計の確定契約を維持する。

## seam 構成

```
PluginEditorSession::set_size/set_visibility/reattach
  → EditorSessionOps (InProcessEditorOps)
    → HostCommand::SetGuiSize/SetGuiVisibility/SetGuiParent
      → PluginHost::execute → composite dispatch
        → Slot::InProcess → InProcessHost::execute
          → InstanceSlot::{Plain,Shared} → ErasedPlugin verb
            → Plugin::set_gui_size/set_gui_visibility/set_gui_parent
              → VST3 / CLAP adapter 実装
        → Slot::OutOfProcess → defined HostError::Unsupported
          （wire::Command 未定義 — l0_only_gui_verb guard が
           to_wire の unreachable 到達を防ぐ）
```

`reattach` のみ lease redeem を伴う: `InProcessEditorOps` が保持
する `LeaseRedeemer`（`RealOsGuiTest::redeem_native_lease` /
headless fixture の ledger redeem）で `ParentSlotLease` を
`ParentLease` へ変換してから `SetGuiParent` を発行する。
redeem 失敗は op 発行前に `EditorOpError` へ落ちる — stale/
forged lease は plugin に到達しない。

## verb 別の adapter 意味

### set_size

- VST3: `IPlugView::canResize` が `kResultTrue` でなければ
  `Unsupported`。`checkSizeConstraint` で plugin が rect を調整
  しうるため、その結果を `onSize` へ渡す。`GuiSize` logical
  units → `ViewRect` pixels は 1:1（attach path が既に持つ
  documented unit deviation、DPI scaling は後続課題）。
- CLAP: `PluginGui::can_resize == false` → `Unsupported`、
  `set_size` decline → `PluginFailed`。`GuiSize` f64 → `u32`
  pixels truncate（同じ unit deviation）。

### set_visibility

- VST3: `IPlugView` に hide/show verb が無いため、hide は
  `removed()`（detach）、show は `setFrame`+`attached`
  （記憶した parent/frame で再構成）。plugin UI state は hide で
  失われる — format 制約として設計書に documented。
- CLAP: `PluginGui::hide`/`show` — window は生存したまま
  非表示化（detach ではない）。

### reattach (set_gui_parent)

- VST3: 現在の attachment を `removed()` で外し、新しい
  `ParentLease` の parent へ `setFrame`+`attached` で attach。
  新 attach 失敗時は old parent への best-effort restore を試み、
  その成否に関わらず結果を `HostError` として返す。
- CLAP: `PluginGui::set_parent` — live GUI の再 embed。
  plugin が既 parented GUI の re-embed を拒否すれば
  `PluginFailed` として正直に返る。

## lane oracle 設計

3 row はいずれも非画素 oracle（session state/event/HWND 契約）
に加え、verb 固有の OS-level effect oracle を持つ:

| row | verb | effect oracle |
| --- | --- | --- |
| `real-editor-resize` | `set_size(800×600)` | 再列挙した全 child の `GetWindowRect` が要求サイズと一致 |
| `real-editor-redock` | `reattach`（同一 live lease） | pre-op child HWND 全て `IsWindow==0` + 新 attach generation（`rvf-test-editor-attach-{n}` title 前進）が container 配下に出現 |
| `real-editor-visibility` | `set_visibility(false→true)` | hide で全 child 破棄・`Hidden{Docked}`、show で新 generation 再成立・`Attached{Docked}` |

`EditorOpVerification` に `effect_confirmed` を追加し、
`editor_op_body` の verdict 写像を
「accepted + lifecycle verified + effect confirmed → `Pass`、
accepted + 未確認 → `NotRun`、refused → `Unsupported`、
契約不一致 → `Fail`」へ拡張した。`ObserveOpenFn` 系 callback
signature に `&RealOsGuiTest` を通し、observe fn が
`real_editor_ops`（実 lease redeem）を構築できるようにした。

## 非目標・境界

- L1 wire verb・`Hello.capabilities` GUI bit・plugin→host GUI
  request は issue #389 後続 slice。
- `AttachMode::Floating` は `InProcessEditorOps` が非 Docked を
  明示拒否する既存契約を維持。
- lane は interactive Windows 前提。off-Windows では
  `child_hwnd_observation` が `unsupported` を記録し verdict は
  正直に落ちる。
