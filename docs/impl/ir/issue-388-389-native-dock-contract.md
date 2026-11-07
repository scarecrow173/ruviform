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
| Windows | GPUI main dock 内 plugin native editor | VST3/CLAP | contract headless Pass / **実 GPUI window 内実 native attach Pass**(window 内 region。実 dock layout 未実装のため Dock PASS ではない) / 実 plugin binary NotRun | Unsupported | adapter+PoC 完了・残 Gate は dock layout widget + 実 binary |
| macOS | 同上 | VST3/CLAP/AU | NotRun | Unsupported | backend 未実装(container stub → 明示 `Unsupported`)/ 実 lane 未実行 |
| Linux X11/XWayland | 同上 | VST3/CLAP | NotRun | Unsupported | 同上(backend 未実装) |
| Linux Wayland | 同上 | VST3/CLAP | NotRun | Unsupported | 同上 + GPUI Wayland 親子表面契約未実証 |

注記: 「contract headless Pass」は driver-neutral 契約と状態機械の
headless 検証を指し、実 Native Dock PASS ではない。実 Dock PASS には
GPUI 実 dock layout(tab/clip/dock 管理 widget)+ 実 plugin binary が必要。
第二 increment(2026-11-07): `GpuiSurfaceAdapter` + consumer PoC で
Windows 実 GPUI window 内への実 native editor attach を実機観測済み
（`evidence/rf388-389-native-dock/`、実行 tuple 同封）。

### 第二 increment: GpuiSurfaceAdapter + consumer PoC(main `5c0f16b` merge 後)

追加したもの:

- `ruviform-plugin-host`: `InProcessEditorOps::with_redeemer` —
  consumer 側 `LeaseRedeemer` 注入 seam(opaque `ParentSlotLease` →
  `GuiParentToken` を adapter 検証経由で交換)。plugin↔ui 相互依存禁止の
  baseline に従い redeemer 本体は利用側層が供給
- `ruviform-ui`: `SlotError::Backend` 追加(Unsupported と OS 失敗を区別)
- `ruviform-ui-gpui`(gpui feature): `GpuiSurfaceAdapter` —
  `NativeSurfaceSlot` port 実装。`open_window` で `HasWindowHandle` 経由
  GPUI top-level raw handle を捕捉し、slot 毎に Win32 child container
  HWND を生成(`STATIC` class・`WS_CHILD|WS_CLIPCHILDREN|WS_CLIPSIBLINGS`)。
  geometry/visibility/clip(SetWindowRgn)/z-order/focus/破棄を実表面へ適用。
  `WindowCapabilities::native_surface_slot` は backend 存在 OS(Windows)のみ
  `Supported`、他 OS は `Unsupported` のまま
- `ruviform-poc-native-dock` 新設: 利用側層 composition root。
  in-process probe plugin の `open_gui` が redeemed token(container HWND)
  直下に実 `WS_CHILD` editor を生成。非 default `gpui` feature の手動
  real-window lane(`mise run poc-native-dock`)
- `mise.toml`: `poc-native-dock` task 追加(手動 real-OS 入口)
- RF390 boundary guard fixture: `ruviform-ui-gpui` allowedDeps に
  `raw-window-handle`、poc crate に `composition-root` grant を追加

実機証拠(Windows 11 x64 / RTX 4070 SUPER / gpui 0.2.2):
`evidence/rf388-389-native-dock/poc-run-win11.log` — 実 GPUI window 内で
container HWND 実生成→probe `open_gui` 実 child editor(`GetParent` で
実親子確認)→`set_geometry` で rect 実追従→hide/show で `IsWindowVisible`
実追従→`request_focus` で実 focus→session close で editor 破棄→
`release_lease` で container 破棄。3 回連続で同一証拠列を再現(exit 0)。

## Risks

- redemption 経路は実装・実証済みだが、`LeaseRedeemer` は composition 層の
  注入 seam — host/plugin 側が自前で償還を偽装しないことの強制は型上は
  弱い（guard は dep-graph で composition-root のみ adapter 依存を許可）
- GPUI の実 dock 制約（clip/z-order/tab/input/focus）は **window 内
  region では実証済み** だが、実 dock layout widget が未実装のため
  dock 管理下での同一性は未実証（container 方式は dock slot が geometry/
  visibility を同一 surface へ駆動する前提設計）
- L1 wire 側の GUI op 到達経路（child runner dispatch）は tag 予約済みだが
  host 側で `Unsupported` short-circuit のまま — wire 変更は行っていない

## Follow-ups

- 実 Dock PASS: GPUI 実 dock layout widget 到着後、container surface を
  dock slot 管理下へ接続し clip/tab/dockmove/float-redock を測定
- 実 plugin binary(VST3/CLAP/AU)attach: binary 入手後。probe と同経路
  (`open_gui`→redeemed container 直下 child)を踏む想定
- macOS(NSView)/Linux(X11/Wayland)container backend 実装は別工程
- `PluginHandle.generation` u16 wrap は wire 互換のため残留(別 Issue 候補)
- RF390 到着後: tests 内の最小 fake ops を共通 semantic fixture へ置換
- RF262: 本 IR の OS/format マトリクスを最終採否判断の入力として供給
- SV176: Synvelis 側 session 結線の語彙として `PluginEditorSessionId`/
  `ParentSlotLease` を供給
