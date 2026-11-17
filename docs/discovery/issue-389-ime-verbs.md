---
id: "DISC-rf389-ime-verbs-01"
type: "discovery"
status: "resolved"
title: "Issue #389 adapter editor verbs 調査記録"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "docs/specs/issue-389-ime-verbs.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-389-ime-verbs.md"
    - "docs/designs/issue-389-ime-verbs.md"
    - "docs/plans/issue-389-ime-verbs.md"
    - "docs/tasks/issue-389-ime-verbs.md"
    - "docs/test-specs/issue-389-ime-verbs.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  derived-by:
    - "docs/specs/issue-389-ime-verbs.md"
  verifies: []
  verified-by: []
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 adapter editor verbs 調査記録

## Summary

Issue `scarecrow173/ruviform#389` の adapter editor verbs slice
について、`InProcessEditorOps` の 3 verb defined refusal の
解消に必要な seam を調査した記録。session 状態機械・lease/
generation boundary・rf631 lane 構造は先行 slice
（`issue-389-plugin-editor-session`・`issue-389-real-session-
lifecycle`）で確立済みであり、本 slice はその上に verb の
production chain と lane effect oracle を載せる。

## 調査結果

### verb seam の現状（着手時）

- `EditorSessionOps` trait は `set_size`/`set_visibility`/
  `reattach` を定義済みだが、`InProcessEditorOps` は 3 verb 全て
  `Err(EditorOpError::Unsupported)` を返す placeholder
  （「No `HostCommand` resize verb exists yet」コメント付き）。
- `HostCommand` には `OpenGui`/`CloseGui` のみ — verb 用
  command/reply が存在しない。
- `Plugin` trait は `open_gui`/`close_gui` のみ — verb hook が
  無く、全 Plugin 実装が default refusal を返す構造だった。
- composite の `to_wire` は handle-carrying command のみを
  wire へ写像 — verb を wire に載せると `unreachable!` に
  到達するため、L0-only guard が必要。

### adapter の verb 面

- VST3 `IPlugView`: `canResize`/`checkSizeConstraint`/`onSize`・
  `setFrame`/`attached`/`removed` が resize/visibility/parent の
  全てを賄う。hide/show verb は存在しないため hide=detach・
  show=再構成という意味差を採用（plugin UI state は hide で
  喪失 — format 制約）。
- CLAP `PluginGui`: `can_resize`/`set_size`・`hide`/`show`・
  `set_parent` が全 verb を直接賄う（真の hide/show）。
- test-vst3 fixture は verb 面を実装済みだったが
  `canResize == kResultFalse` で resize を decline しており、
  `onSize` は child HWND を resize しなかった — lane oracle が
  「受理されたのに効果が無い」を検出できるよう fixture を
  実 resize 可能に拡張する必要があった。

### lane oracle の設計判断

- 3 row はいずれも非画素 oracle（session state/event/HWND 契約）
  のみだった。verb が「本当に OS 効果を持つ」ことを証明するには
  verb 固有の effect oracle が必要:
  - resize → `GetWindowRect` geometry
  - redock → 旧 child HWND teardown + 新 attach generation
    （HWND 値は再利用されうるため window title の generation
    stamp で識別）
  - visibility → hide で child 破棄・show で新 generation 再成立
- `EditorOpVerification` に `effect_confirmed` を追加し、
  accepted op が `Pass` へ到達するには effect oracle の確認を
  必須化（accepted+未確認は `NotRun` のまま — fake success を
  防ぐ）。
- observe callback に `&RealOsGuiTest` を通す必要があった
  （`reattach` が実 lease redeem を要求するため）。共有
  signature の全 observe fn に `_gui`/`gui` param を追加し、
  pixel/foreground/present 系は `_gui` 未使用で互換維持。

### lease 境界の確認

- `reattach` は `InProcessEditorOps` の `LeaseRedeemer` を通じて
  `ParentSlotLease` → `ParentLease` を redeem — `open` と同一
  seam であり、stale/forged lease は op 発行前に拒否される。
  raw OS handle は session boundary を越えない契約を維持。

## 結論

seam は「core trait hook（additive default refusal）→
HostCommand（L0-only、composite guard で OutOfProcess は defined
Unsupported）→ adapter verb → lane effect oracle」の一続きで
閉じる。L1 wire・Floating・plugin→host GUI request は本 slice
対象外として明示的に残す。
