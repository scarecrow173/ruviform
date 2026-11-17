---
id: "IMPL-rf389-editor-verbs-01"
type: "impl"
status: "completed"
title: "Issue #389 adapter editor verbs 実装記録（set_size/set_visibility/reattach production chain + rf631 lane flip）"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-389-editor-verbs.md"
      - type: "doc"
        path: "docs/impl/ir/issue-389-editor-verbs.md"
      - type: "file"
        path: "evidence/rf389/verification-editor-verbs.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/command.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/editor_session.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/editor_view.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-headless.jsonl"
      - type: "file"
        path: "evidence/rf389/plugin-dock-real-scenario.jsonl"
      - type: "file"
        path: "evidence/rf389/plugin-dock-real-headless.jsonl"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-389-editor-verbs.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/tasks/issue-389-real-session-lifecycle.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "evidence/rf389/verification-editor-verbs.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #389 adapter editor verbs 実装記録

## Summary

`InProcessEditorOps` の `set_size`/`set_visibility`/`reattach` が
返していた定義済み `EditorOpError::Unsupported` refusal を除去し、
editor verb を plugin-core trait → `HostCommand`/`HostReply` →
host dispatch → composite L0 routing → VST3/CLAP adapter の
production chain へ通した。VST3 test fixture を実 resize 可能に
拡張し、`rf631.plugin-dock-real` lane の 3 row
（`real-editor-resize`/`real-editor-redock`/
`real-editor-visibility`）を refusal/deferred から実実行 +
OS-level effect oracle 検証へ flip した。実走 lane で 3 row とも
`pass` / `environment_kind=real-window` を記録した。L1
`L1EditorOps` は全 op refusal-only のまま無変更。

## Context

`docs/specs|designs/issue-389-plugin-editor-session.md` が定義する
`EditorSessionOps` verb 面の L0 実装 slice。headless contract
（`tests/editor_session.rs`）と real-window lane
（`rf631.plugin-dock-real`）の双方で、refusal ではなく実 effect
（child HWND geometry / teardown / recreation / attach
generation）を oracle として検証する。lease/generation boundary
と raw OS handle 非通過の契約は維持。

## Implementation

### Changes

- `ruviform-plugin-core`: `GuiSize` vocabulary と plugin GUI trait
  の `set_gui_size`/`set_gui_visibility`/`set_gui_parent` hook を
  追加（`boundary.rs`/`plugin.rs`/`lib.rs`）。
- `ruviform-plugin-host`: `HostCommand::SetGuiSize`/
  `SetGuiVisibility`/`SetGuiParent`（+ reply）を追加し、host
  dispatch・composite L0 routing・`InProcessEditorOps` の 3 verb
  実装を接続。`HostError::Unsupported` → `EditorOpError::
  Unsupported` mapping を維持。composite は L0 のみ verb を
  dispatch し、L1 は refusal-only。
- `ruviform-plugin-vst3`: `set_gui_size` = `canResize` gate →
  `checkSizeConstraint` → `onSize`（`GuiSize` → `ViewRect`、
  logical:pixel 1:1 の既存 documented deviation を踏襲）。
  `set_gui_visibility(false)` = `removed()`、`(true)` =
  `setFrame`+`attached` 再構成（`IPlugView` に hide/show verb が
  無いため hide は detach と同義 — 設計通り）。`set_gui_parent` =
  remove → 新 parent へ attach、失敗時 old parent best-effort
  restore。`gui_attached`/`gui_parent` を track。
- `ruviform-plugin-clap`: `PluginGui::can_resize`/`set_size`、
  `hide`/`show`、`set_parent` を配線。
- `ruviform-plugin-test-vst3` fixture: `canResize` を
  `kResultTrue` 化、`onSize` が child HWND を `SetWindowPos` で
  resize、attach generation を window title
  `rvf-test-editor-attach-{n}` に刻印。fixture test を
  `kResultTrue`/`checkSizeConstraint` 契約へ追従。
- `ruviform-plugin-host/tests/editor_session.rs`: 旧 refusal 期待
  を accepted 動作へ更新（DSP 非干渉 test は `set_visibility(false)`
  → `Hidden` → close → DSP `Processing`/`Continue` 維持を検証）。
  `verbs_supported` flag で honest refusal 経路も維持検証。
- `ruviform-plugin-dock-test`: `ChildWindowFacts` に
  `width`/`height`/`title` を追加し oschild Windows impl が
  `GetWindowRect`/`GetWindowTextW` を観測。`is_alive` helper 追加。
  `ObserveOpenFn` 系 callback signature に `&RealOsGuiTest` を
  通し、resize/redock/visibility の observe fn を `real_editor_ops`
  （実 lease redeem 経由）で実行するよう書き換え。各 verb の
  effect oracle: resize = 全 child `GetWindowRect == 800×600`、
  redock = 旧 child HWND 全 `IsWindow==0` + 新 generation title
  前進、visibility = hide で全 child 破棄 + show で新 generation
  再成立。`EditorOpVerification.effect_confirmed` +
  `EditorOpIdentity` を追加し `editor_op_body` に `Pass` arm
  （accepted + lifecycle verified + effect confirmed）を追加。
  module doc・`real_catalog.rs`・`lib.rs`・`README.md` を新 verdict
  semantics へ追従。

## Verification

`evidence/rf389/verification-editor-verbs.md` 参照。要点:

- headless: `editor_session` 54/54、fixture test 全 green、
  dock-test（`--features real-dock` 含む）全 green、
  clippy warning 0、fmt clean。
- real lane 実走（interactive Windows）: `real-editor-resize` /
  `real-editor-redock` / `real-editor-visibility` が全て `pass`
  （effect oracle 確認済み）。`real-dcomp-pixel` の既知 `fail`
  （#532・対象外）で lane exit 1 は設計通り。
- headless fail-closed: `test-ui-plugin-dock-real-headless` が
  22 row 全て `not_run` + `environment_blocked`、violation 0。

## Notes / Boundaries

- `AttachMode::Floating` は依然 `Unsupported`（非 Docked は
  `InProcessEditorOps` が明示拒否）。
- VST3 `set_gui_visibility` の hide は plugin UI state を失う
  detach 同義動作 — `IPlugView` に hide verb が無い format 制約で
  あり、設計書に documented。
- `set_size` の plugin refusal は session 非致命（`Attached`/
  `Hidden` 維持）を維持。`set_visibility` は adapter 成功後のみ
  `Attached↔Hidden` を遷移し no-op は event を発火しない。
- L1 wire 動詞・`Hello.capabilities` GUI bit・plugin→host GUI
  request・crash/hang 受入・pixel gate・consumer gate は issue
  #389 の後続 slice に残る。
