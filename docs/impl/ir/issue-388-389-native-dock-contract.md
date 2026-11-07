---
id: "IMPL-OW0F0pEruOX61ZMf3ODHQD"
type: "impl"
status: "completed"
title: "Issue #388/#389 Native Plugin Dock 早期 Gate — NativeSurfaceSlot / PluginEditorSession 契約実装"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/src/native_slot.rs"
      - type: "file"
        path: "crates/ruviform-ui/tests/native_slot.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/editor_session.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session.rs"
      - type: "file"
        path: "docs/discovery/issue-388-389-native-dock-current-state.md"
      - type: "file"
        path: "docs/specs/issue-388-native-surface-slot.md"
      - type: "file"
        path: "docs/specs/issue-389-plugin-editor-session.md"
      - type: "file"
        path: "docs/designs/issue-388-native-surface-slot.md"
      - type: "file"
        path: "docs/designs/issue-389-plugin-editor-session.md"
      - type: "file"
        path: "docs/plans/implement-issue-388-389-native-dock-contract.md"
      - type: "file"
        path: "docs/test-specs/issue-388-389-native-dock-contract.md"
      - type: "file"
        path: "docs/tasks/issue-388-389-native-dock-contract.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/README.md"
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/command.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-388-389-native-dock-contract.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-388-389-native-dock-contract.md"
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-182-dock-tree.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #388/#389 Native Plugin Dock 早期 Gate — 契約実装

## Summary

Driver-neutral な `NativeSurfaceSlot`（`ruviform-ui::native_slot`）と
`PluginEditorSession`（`ruviform-plugin-host::editor_session`）の契約を実装した。
共有 opaque 語彙（`PluginEditorSessionId`/`ParentSlotLease`）は
`ruviform-plugin-core::boundary` に置き、plugin 系 ↔ ui 系の相互依存を持たせない。
旧 API（`HostCommand::OpenGui`/`CloseGui`・`GuiParentToken`・
`render::NativeSurfaceHost`）は全て保持。L1 は全 op `Unsupported` の
defined refusal であり、L0 への暗黙退化はしない。

## Context

- 実施タスク: docs/tasks/issue-388-389-native-dock-contract.md
- 上流: 2 spec・2 design・plan・test-spec（本 worktree の doc chain）
- 判定正本: docs/discovery/issue-388-389-native-dock-current-state.md
  （現行 `GuiParentToken`・L1 `OpenGui`→`Unsupported` short-circuit・
  `render::NativeSurfaceHost`・dock tree・adapter `open_gui` 未実装の実監査）
- RF386 の GPUI 最小 Window fixture は未到着（同時刻 worktree は
  `gpui 0.2.2` pin + feature-gate の skeleton のみ）。実 Native Dock PoC は
  Blocked とし、headless 契約検証を先行させた

## Implementation

### Changes

- `ruviform-plugin-core::boundary`: `PluginEditorSessionId{slot:u32,
  generation:u16}`・`ParentSlotLease(u64)` を追加。`Copy`・const ctor/accessor・
  `Debug`/`Display` は非零 payload のみ表示。`GuiParentToken` は据え置き
- `ruviform-ui::native_slot`: `{index,generation}` の `SurfaceSlotId`・
  `SlotLease`（registry 所有 mint、借り手は opaque 保持）・`SlotSpec`・
  `SlotGeometry`/`LogicalRect`・`SlotCapabilities`・`SlotError`・`SlotEvent`・
  `SlotState`/`SlotVisibility`・`NativeSurfaceSlot` port・
  `SurfaceSlotRegistry`（declare/lease/release/geometry/visibility/busy/
  destroy・単一 live lease・stale generation 拒否・index 再利用時 generation
  bump）。`Default` capability は全 `Unsupported`（honest default）。
  OS handle・GPUI/wry/tao 型は非露出・全操作 NONRT
- `ruviform-plugin-host::editor_session`: `EditorSessionState`
  （`Closed→Creating→Attached{Docked|Floating}→Hidden|Detaching|Closing`
  ・`Failed→Recovering|Closed`）、request 世代管理（stale completion 拒否）、
  terminal event 一度切り、`concluded` flag（新生 `Closed` と lifecycle 収束
  `Closed` の区別 — instance 毎 ≤1 live session を正しく強制）、
  `EditorSessionOps` port、`InProcessEditorOps`（L0 既存
  `OpenGui`/`CloseGui` 経路 — `GuiParentToken::new` の唯一の本番呼出点）、
  `L1EditorOps`（全 op `EditorOpError::Unsupported`・wire 非送信）、
  `PluginEditorSessions` registry（instance 結び付け・retire は concluded
  session のみ・host 側 instance 終了通知での session 収束）
- `command.rs`/`lib.rs`: 新境界型の re-export。既存 `HostCommand`/
  `HostReply`/`GuiParentToken`/`GuiHandle` の公開形は不変

### Decisions During Implementation

- **所有配置**: architecture baseline の「plugin 系と ui 系の相互依存禁止・
  consumer 側で結線」に従い、session/lease 語彙は plugin-core、slot 契約は
  ruviform-ui、session 状態機械は plugin-host に配置。lease→raw OS token
  の償還（`GuiParentToken`）は composition/adapter 側に限定
- **`concluded` flag 追加**: `Closed` 初期状態を「lifecycle 収束済み」と
  誤認し second live session を通してしまう回帰を full workspace test で
  検出。`Closed` を born/converged で分離し、retire は converged のみに限定
- **capability は bool 群ではなく `Support` enum**: `#[non_exhaustive]` 方針と
  「既定は全 Unsupported（能力未実証）」の honest default のため。
  clippy `struct_excessive_bools` も同時解消
- **resize 拒否は非致命**: `EditorSessionError::ResizeRejected` は session を
  終端へ送らず observable な拒否として返す（format 固有の制約を表現）
- **RF390 fixture は複製しない**: fake ops は本 crate tests 内の最小 stub に
  限定し、共通 semantic fixture は RF390 到着後に消費する方針を plan に記録

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64・Git Bash・`mise` 管理 toolchain
（`rust = "1.99.0"`、`CARGO_BUILD_JOBS=2`）。

| Check | コマンド | 結果 |
| --- | --- | --- |
| fmt | `mise run fmt` / `cargo fmt --all -- --check` | Pass |
| workspace lint | `cargo clippy --workspace --all-targets -- -D warnings`（`mise run lint` 相当） | Pass — 0 warnings |
| native_slot tests | `cargo test -p ruviform-ui --test native_slot` | Pass — 11/11 |
| editor_session tests | `cargo test -p ruviform-plugin-host --test editor_session` | Pass — 20/20 |
| plugin-core suites | `mise run test-plugin-core` 相当 | Pass |
| full workspace test | `mise run test` | Pass — 全 suite ok・0 failed（独立 review 修正後の再実行で green） |

独立 review（Reviewer A/B → Validator）は2件の P1（concluded
session の再 open 経路・live session の `end_session` 除去）を確認・
修正済み。副次修正: lease seq を registry-global `u32` 化・slot
generation tombstone・capability 未 enforcement・sync/async reattach
イベント不整合・L0 floating open の暗黙 docked 偽装・born-Closed
session への terminate による再 open 孔・`NativeSurfaceSlot` trait の
`release_lease`/`validate_lease` 欠落・`SlotError`/`EditorOpError`
`Display`/`Error` impl 欠落・`PluginEditorSessionId` generation
`u16`→`u32` 拡幅。残 residual: `PluginHandle.generation` は
wire 互換のため `u16` のまま（別 Issue 起票候補）。

### OS/format/L0-L1 要求行マトリクス（実機実行は別 lane）

| OS | 経路 | format | L0 | L1 | 状態 |
| --- | --- | --- | --- | --- | --- |
| Windows | GPUI main dock 内 plugin native editor | VST3/CLAP | contract headless Pass / 実機 Blocked | Unsupported | Blocked on RF386 fixture + adapter `open_gui` 未実装 |
| macOS | 同上 | VST3/CLAP/AU | NotRun | Unsupported | Blocked on RF386 + 実 macOS lane 未実行 |
| Linux X11/XWayland | 同上 | VST3/CLAP | NotRun | Unsupported | Blocked on RF386 + 実 OS lane 未実行 |
| Linux Wayland | 同上 | VST3/CLAP | NotRun | Unsupported | Blocked on RF386 + GPUI Wayland 親子表面契約未実証 |

注記: 「contract headless Pass」は driver-neutral 契約と状態機械の
headless 検証を指し、実 Native Dock PASS ではない。実 Dock PASS には
RF386 の実 GPUI window・adapter `open_gui` 実装・実 plugin binary が必要。

## Risks

- `ParentSlotLease`/`GuiParentToken` の償還は adapter 実装側の責務 — 契約は
  用意したが redemption 経路の実装は未存在（設計上 composition 層の仕事）
- GPUI の実 dock 制約（clip/z-order/tab/input/focus）は fixture 到着まで
  未実証。契約側の `SlotCapabilities`/`SlotGeometry` が GPUI 実制約と
  不整合になる可能性は残る（設計は dock tree の既知制約から導出済み）
- L1 wire 側の GUI op 到達経路（child runner dispatch）は tag 予約済みだが
  host 側で `Unsupported` short-circuit のまま — wire 変更は行っていない

## Follow-ups

- RF386 fixture 到着後: adapter `open_gui`/`close_gui` 実装と
  `NativeSurfaceSlot`→`ParentSlotLease` 償還の結線、Windows 実 plugin
  native dock PoC へ移行（clip/z-order/hide/tab/resize/dockmove/
  float-redock/close/hang/RT 隔離の測定）
- RF390 到着後: tests 内の最小 fake ops を共通 semantic fixture へ置換
- RF262: 本 IR の OS/format マトリクスを最終採否判断の入力として供給
- SV176: Synvelis 側 session 結線の語彙として `PluginEditorSessionId`/
  `ParentSlotLease` を供給
