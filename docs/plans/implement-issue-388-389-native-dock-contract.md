---
id: "PLAN-kHnSXCzzcxyo5rlkMiqQWx"
type: "plan"
status: "approved"
title: "Issue #388/#389 Native Dock 契約の実装計画（neutral slot/lease/session + fake 検証 + L0-L1 写像）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
    - "docs/designs/issue-115-l1-cross-process-parenting.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/386"
  defers: []
  deferred-by: []
metadata:
  issues: [388, 389]
---

# Issue #388/#389 Native Dock 契約の実装計画

## 目的

`docs/designs/issue-388-native-surface-slot.md`（D1–D7）と
`docs/designs/issue-389-plugin-editor-session.md`（D1–D8）を、
**実 GPUI fixture 未到着でも検証可能な中立契約・純粋モデル・fake
検証・L0/L1 写像**として実装する。実 Native Dock PoC 行は
Blocked/NotRun で記録するに留める。

## スコープ境界（所有範囲）

- 本 dispatch 所有: `ruviform-ui` の `native_slot`・
  `ruviform-plugin-core` の `boundary` 追加 2 型・
  `ruviform-plugin-host` の `editor_session`・これらのテスト・
  doc chain・マトリクス。
- 非所有（触らない）: `ruviform-ui-gpui` 実装（RF386）・`Cargo.lock`
  の他者変更・`ruviform-ui-webview`・既存 `render.rs`/`dock.rs`/
  `window.rs` の改変・`Plugin` trait/`HostError`/wire/l1 本体。

## 段階

### T-1: `ruviform-plugin-core` boundary 追加

`boundary.rs` に `PluginEditorSessionId{slot:u32,generation:u16}` と
`ParentSlotLease(u64)` を追加（`PluginHandle`/`GuiParentToken` 先例、
`#[doc(hidden)] new`・public accessor・serde なし・`unsafe` なし）。
既存型は不変。README・lib.rs re-export を更新。

### T-2: `ruviform-ui` native_slot モジュール

`src/native_slot.rs` 新設 + `lib.rs` に `mod`/`pub use` 追加:

- `SurfaceSlotId{index:u32,generation:u16}`（`#[doc(hidden)] new`）。
- `SlotLease`（`u64` pack `index<<32|generation<<16|seq`、
  `#[doc(hidden)] new`・`encode`/`decode`・`raw`）。
- `SlotGeometry{LogicalRect,scale_factor}`/`SlotVisibility`/
  `SlotSpec{window,view,geometry,visibility}`/`SlotCapabilities`/
  `SlotError`/`SlotEvent`/`LeaseInfo`/`SlotState` — 全 enum
  `#[non_exhaustive]`。
- `SurfaceSlotRegistry`（`DockWorkspace` と同型純粋モデル:
  `declare_slot`/`release_slot`/`set_geometry`/`set_visibility`/
  `set_clip`/`set_zorder`/`lease_parent`/`release_lease`/
  `validate_lease`/`slot`/`capabilities`/`drain_events`/
  `mark_busy`/`clear_busy`）。
- `NativeSurfaceSlot` trait（D4 の port）。

`ruviform-ui` は gpui/wry/tao/OS handle 型を一切持ち込まない。

### T-3: `ruviform-plugin-host` editor_session モジュール

`src/editor_session.rs` 新設 + `lib.rs` に `mod`/`pub use` 追加:

- `EditorSessionState`/`AttachMode`/`SessionFailCause`/
  `EditorSessionEvent`/`EditorSessionError`/`EditorOpError`/
  `EditorCapabilities`/`EditorSize` — 全 enum `#[non_exhaustive]`。
- `PluginEditorSessions` registry（instance→≤1 session、id mint/
  recycle、`notify_terminated`）。
- `PluginEditorSession`（状態機械・request 世代・terminal 一度切り・
  event sink）。
- `EditorSessionOps` trait。
- `InProcessEditorOps<'a>`（`InProcessHost` ラップ — `GuiParentToken::
  new` の唯一 production 呼出点・`HostError`→`EditorOpError` 写像）。
- `L1EditorOps`（全 ops `Unsupported` 定義済み拒否・wire 不変）。

### T-4: テスト

- `crates/ruviform-ui/tests/native_slot.rs`（headless 契約）。
- `crates/ruviform-plugin-host/tests/editor_session.rs`（fake ops +
  `InProcessEditorOps` + stub `ErasedPlugin` live 経路）。
- 既存テストに変更なし（l1 reject pin はそのまま green を維持）。

### T-5: 検証・証跡

- `mise run fmt-check`/`check`/`lint`/`test`（workspace）。
- `Cargo.lock` は差分を確認し自追加依存なしを確認。
- 証跡を本 worktree `docs/impl/exp/` または evidence report へ実値
  記録（コマンド/exit/OS/toolchain/範囲）。

## 検証

`docs/test-specs/issue-388-389-native-dock-contract.md` の全ケース +
workspace lint/fmt/test green。

## 未着手（本 dispatch 外・Blocked/NotRun）

- 実 GPUI Window での Native Dock PoC（RF386 最小 fixture 未到着 →
  Blocked）。
- 実プラグイン binary の open/dock（adapter `open_gui` 未実装・
  binary 未入手 → NotRun）。
- L1 wire 動詞追加・capabilities negotiation 実装（設計のみ）。
- macOS/Linux 実機（環境なし → NotRun）。
