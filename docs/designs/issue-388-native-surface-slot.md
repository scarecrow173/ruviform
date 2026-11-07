---
id: "DESIGN-E2c3n3d0DhHGOrf8ZGHJrP"
type: "design"
status: "approved"
title: "Issue #388 Driver-neutral NativeSurfaceSlot 契約設計（SurfaceSlotRegistry / SlotLease / port）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/src/native_slot.rs"
      - type: "file"
        path: "crates/ruviform-ui/tests/native_slot.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/specs/issue-388-native-surface-slot.md"
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-182-dock-tree.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 388
---

# Issue #388 Driver-neutral NativeSurfaceSlot 契約設計

## Context

Issue #388 の早期 Gate は「GPUI Main Dock 内の実レイアウト・clip・
tab・resize・focus 管理下でプラグイン自身の実ネイティブ Editor が
動作する」ことを要求する。その前段として、**どの GUI Driver も
実装できる driver-neutral な native 埋込領域の契約**を
`ruviform-ui` に置く。`HWND`/`NSView`/X11/GPUI 型は Driver・OS
adapter 内に限定し、公開契約の意味論を gpui の `Element` 等へ
合わせない（issue-388 確定仕様）。

発見 F6（discovery）で配置が確定: port 定義元は `ruviform-ui`、
plugin 系 ↔ ui 系の相互依存は禁止のため lease → `GuiParentToken`
への写像は利用側（Driver composition）の責務。

## 決定

### D1: `SurfaceSlotId` は `{index: u32, generation: u16}` の
generation 付き opaque id

`PluginHandle` と同じ stale 検出形式に揃える（`ViewInstanceId`
系の consumer-minted `NonZeroU64` ではない）。slot は Driver 側の
registry が mint し、解放された index は generation++ で再利用
される — 旧 generation の id/lease を機械的に拒否できる。

```rust
pub struct SurfaceSlotId { index: u32, generation: u16 }
// #[doc(hidden)] const fn new — mint は registry 内部操作
// pub fn index() / generation()
```

### D2: `SlotLease` は `{index:u32, generation:u16, seq:u16}` を
u64 に pack した opaque token

「opaque, generation-bound, lifetime-checked」（issue-389 確定
仕様）を registry が強制する:

- `SurfaceSlotRegistry::lease_parent(slot)` は当該 slot の現在
  generation に紐づく一意 `seq` を採番して mint。1 slot に live
  lease は同時に 1 つ（`LeaseBusy` で二重発行を拒否 — 1 slot =
  高々 1 attach の不変条件と一致）。
- `validate_lease(lease)` は index/generation/seq を検査し
  `Ok(LeaseInfo)`/`StaleGeneration`/`LeaseRevoked`/`InvalidLease`
  を返す。`release_lease`・`release_slot`（slot 破棄）は live
  lease を revoke。
- `encode() -> u64`（`index<<32 | generation<<16 | seq`）は
  session 面語彙 `ParentSlotLease::new(u64)`（plugin-core）への
  composition 変換点。`decode(u64) -> Option<SlotLease>` は
  adapter 側 redeem でのみ使う。

caller は中身を解釈しない。`SlotLease::new` は `#[doc(hidden)]`
（`PluginHandle` 先例）。発行側構造は opaque — 別 Driver は独自の
u64 構造を選べ、本書の pack 形は Ruviform 標準 Driver の実装選択。

### D3: `SurfaceSlotRegistry` は `DockWorkspace` と同型の純粋モデル

slot の所有・世代・lease 帳簿を保持する唯一の真実層。`WindowHandle`/
`UiRuntime`/OS API を呼ばず headless で全契約を検証する。

```text
slot: index → { id(SurfaceSlotId), state, spec, lease: Option<seq> }
state: Declared → Leased → Declared(release_lease) → Released(release_slot)
       Released index は次回 declare で generation++ して再利用
```

公開語彙: `declare_slot(spec) -> SurfaceSlotId`、`release_slot`、
`set_geometry`/`set_visibility`/`set_clip`/`set_zorder`（意味記録 —
実 OS 適用は Driver）、`lease_parent`、`release_lease`、
`validate_lease`、`slot(slot_id)`/`state`/`capabilities` 参照、
`drain_events`（`SlotEvent::LeaseRevoked`/`GeometryChanged`/
`VisibilityChanged`/`Destroyed` — `Destroyed` は終端）。

### D4: `NativeSurfaceSlot` trait は Driver 実装の port

```rust
pub trait NativeSurfaceSlot {
    fn capabilities(&self, slot: SurfaceSlotId) -> Result<SlotCapabilities, SlotError>;
    fn declare_slot(&mut self, spec: &SlotSpec) -> Result<SurfaceSlotId, SlotError>;
    fn release_slot(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError>;
    fn set_geometry(&mut self, slot: SurfaceSlotId, geometry: SlotGeometry) -> Result<(), SlotError>;
    fn set_visibility(&mut self, slot: SurfaceSlotId, v: SlotVisibility) -> Result<(), SlotError>;
    fn set_clip(&mut self, slot: SurfaceSlotId, clip: Option<LogicalRect>) -> Result<(), SlotError>;
    fn set_zorder(&mut self, slot: SurfaceSlotId, z: i32) -> Result<(), SlotError>;
    fn request_focus(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError>;
    fn lease_parent(&mut self, slot: SurfaceSlotId) -> Result<SlotLease, SlotError>;
    fn drain_events(&mut self) -> Vec<SlotEvent>;
}
```

`SlotSpec` は `window: WindowId`・`view: ViewInstanceId`・`geometry`・
`visibility` を持つ論理配置宣言 — OS/GPUI 型は含まない。
`SlotCapabilities` は能力旗艦（`attach_parent`/`reparent`/
`cross_window_move`/`resize_negotiate`/`hide_show`/`focus`/
`keyboard_input`/`ime`/`popup`/`clip`/`z_order`/
`out_of_process_parent`）。未対応 capability は `SlotError::Unsupported`
で明示し、fake 成功を実 Dock PASS にしない。

### D5: `DockWorkspace` 2-phase move との接続は契約規約

- slot の論理所有は `SlotSpec { window, view }` — `DockWorkspace` の
  `ViewInstanceId`/`WindowId` を参照するが dock 層へ逆依存しない。
- View の `begin_move` 中は対象 slot は `SlotBusy` 相当として新規
  `lease_parent` を拒否（`ViewBusy` の意味を slot 面へ写像）。
- `commit_move` で View が別 Window/tree へ移る場合、slot は
  `Detaching → re-declare`（新 `SlotSpec`）または同 slot の
  `set_geometry`/`set_window` 更新で表す — 失敗時は `abort_move` と
  対称に「lease revoke → 元 spec を保持して re-issue」で元 View/
  Plugin Instance/Project/RT を保持する（issue-388 必須ケースの
  失敗規約）。物理 reparent の可否は `capabilities.cross_window_move`
  に依存し、不可なら session 側は detach→reattach として扱う。

### D6: OS/format/L0-L1 事前評価マトリクス（行省略なし）

「observed outcomes」列は現行コード監査 + issue-115 機構調査の
承継値。**本 dispatch の全行は実機 PoC 未実施（RF386 最小 Window
未到着 + adapter `open_gui` 未実装 + 実 binary 未入手）のため
NotRun/Blocked。推測と実測を分離する。**

| OS / format | L0/L1 | embedding API | parent ownership | process boundary | resize negotiation | popup/IME | rendering & z-order | observed outcomes | verdict | limitation |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Windows / VST3 | L0 | `IPlugView::attached(HWND)` → `WS_CHILD` | host 子 | 同一プロセス | `onSize`/`checkSizeConstraint` | 親 IME/子 popup 実機要検証 | HWND z-order・GPUI swapchain 遮蔽要検証 | `open_gui` adapter 未実装・機構は issue-115 調査で条件付き | **NotRun** | 実 binary・GPUI Window・`open_gui` 実装待ち |
| Windows / VST3 | L1 | 越境 `WS_CHILD`/`SetParent` | host 子（別 proc） | 別プロセス・入力キュー暗黙結合 | 同上（越境追加リスク） | 入力キュー結合で hang 波及 | 孤児窓/reparent 回復要 | 現行 `Unsupported` defined rejection | **Unsupported**（現行実装）/ 要再評価 | isolation 弱体化 — issue-115 defer |
| Windows / CLAP | L0 | `clap_window.win32` HWND → `WS_CHILD` | host 子 | 同一プロセス | `clap_plugin_gui.set_size`/`adjust_size` | 同上 | 同上 | `open_gui` adapter 未実装 | **NotRun** | 同上 |
| Windows / CLAP | L1 | 越境 `WS_CHILD` | host 子（別 proc） | 別プロセス | 同上 | 同上 | 同上 | 現行 `Unsupported` | **Unsupported**（現行）/ 要再評価 | 同上 |
| macOS / VST3 | L0 | `IPlugView::attached(NSView*)` 同一 proc | host view 子 | 同一プロセス | `onSize`/`checkSizeConstraint` | responder chain・popup | NSView layer・Metal swapchain 要検証 | 文書評価のみ（実機なし） | **NotRun** | macOS 実機未実施 |
| macOS / VST3 | L1 | 越境 NSView reparent 公認 API なし | — | 別プロセス | — | 入力転送なし | `CAContext`/`CALayerHost` 合成専用 | issue-115 で不可確定 | **Unsupported** | 公認越境 view 機構なし |
| macOS / CLAP | L0 | `clap_window.cocoa` NSView 同一 proc | host view 子 | 同一プロセス | `set_size`/`adjust_size` | 同上 | 同上 | 文書評価のみ | **NotRun** | 同上 |
| macOS / CLAP | L1 | 越境 NSView 不可 | — | 別プロセス | — | — | 同上 | 不可確定 | **Unsupported** | 同上 |
| macOS / AU | L0 | AU view 提供（`NSView`/AUVC 経由） | host view 子 | 同一プロセス（AUv3 は別 proc UI の場合あり） | AU 規格の view 制約 | AU 独自 | AU 独自 | 規格調査のみ | **NotRun** | AU 実装形態の調査・実機要 |
| macOS / AU | L1 | 越境 NSView 不可 + AUv3 XPC view | — | 別プロセス | — | — | 同上 | 不可確定 | **Unsupported** | 同上 |
| Linux X11 / VST3 | L0 | `IPlugView` `"X11EmbedWindowID"` → XEmbed | host XID 親 | 同一プロセス | `onSize` | X 入力/ICCCM | X 窓 z-order | wry `build_gtk` は in-process コンテナ前提・GPUI 経路未検証 | **NotRun** | X11 session 実機要 |
| Linux X11 / VST3 | L1 | XEmbed 越境（`GtkSocket`/`GtkPlug` GTK3 現存・deprecated） | host XID 親（別 proc） | 別プロセス（X server object） | 同上 | 同上 | 同上 | 旧式機構のみ理論上成立・現行 `Unsupported` | **Unsupported**（現行）/ 要再評価 | GTK4 除去・wry/GPUI 経路なし |
| Linux X11 / CLAP | L0 | `clap_window.x11` XID → XEmbed | host XID 親 | 同一プロセス | `set_size` | 同上 | 同上 | 未検証 | **NotRun** | 同上 |
| Linux X11 / CLAP | L1 | XEmbed 越境 | 同上 | 別プロセス | 同上 | 同上 | 同上 | 現行 `Unsupported` | **Unsupported**（現行）/ 要再評価 | 同上 |
| Linux XWayland / VST3 | L0 | XEmbed over XWayland | host XID 親 | 同一プロセス | 同上 | XWayland 経由の IME/clip 差 | compositor 依存 | 未検証 | **NotRun** | XWayland 実機要 |
| Linux XWayland / VST3 | L1 | XEmbed 越境 over XWayland | 同上 | 別プロセス | 同上 | 同上 | 同上 | 現行 `Unsupported` | **Unsupported**（現行） | 同上 |
| Linux XWayland / CLAP | L0 | `clap_window.x11` over XWayland | 同上 | 同一プロセス | 同上 | 同上 | 同上 | 未検証 | **NotRun** | 同上 |
| Linux XWayland / CLAP | L1 | 同上 | 同上 | 別プロセス | 同上 | 同上 | 同上 | 現行 `Unsupported` | **Unsupported**（現行） | 同上 |
| Linux Wayland / VST3 | 任意 | 規格上 wayland platform type なし | — | — | — | — | `xdg-foreign` は toplevel hint のみ | native embed 機構不存在 | **Unsupported** | 規格未対応 |
| Linux Wayland / CLAP | 任意 | `clap_window` に wayland 型なし | — | — | — | — | 同上 | native embed 機構不存在 | **Unsupported** | 規格未対応 |

補足:

- 「GPUI swapchain の遮蔽」は GPUI Driver 実機試験でのみ評価可能 —
  本表では全行要検証として残す。
- 方式候補（L0 view attach / L1 cross-process child attach / 別 proc
  composited surface / Host-owned window fallback）は PoC 比較対象で
  あり、全 OS・全 format で同一方式を強制しない（issue-388 確定仕様）。
- 実機行の証拠（OS/GPU/GPUI revision/plugin binary/version/
  process tree/command/exit/TestID/3repo SHA/証拠 path）は PoC 実施
  時に `docs/impl/exp/` へ記録する。本 dispatch では発生しない。

### D7: 旧 `NativeSurfaceHost` との移行写像

| 旧（render.rs・WebView 隣接前提） | 新（native_slot・driver-neutral） | 扱い |
| --- | --- | --- |
| `SurfaceId(u64)` | `SurfaceSlotId{index,generation}` | 新規（世代付き） |
| `SurfaceRegion`（物理 px・window content 相対） | `SlotGeometry{LogicalRect, scale_factor}` | 論理+scale で再定義 |
| `create_surface`/`move_surface`/`destroy_surface` | `declare_slot`/`set_geometry`/`release_slot` | 写像 |
| なし（親子/lease 概念なし） | `SlotLease`/`lease_parent`/`validate_lease` | 新規（本設計の核心） |
| なし | `SlotCapabilities`/`SlotEvent`/`SlotError` | 新規 |
| `NativeSurfaceHost` trait | `NativeSurfaceSlot` trait | 旧 trait は WebView 経路として存置・新 trait が正本 |

`NativeSurfaceHost` は `ruviform-ui` から削除しない（旧 WebView 経路
は縮退判断まで保持 — issue-384 移行戦略）。

## Data and Control Flow

```text
consumer(Driver):
  declare_slot(SlotSpec{window,view,geometry,visibility}) -> SurfaceSlotId
  lease_parent(slot)            -> SlotLease{index,gen,seq}
    ↓ composition: ParentSlotLease::new(lease.encode())
    → session(ruviform-plugin-host) へ渡す
    → session の EditorSessionOps impl が Driver の OS adapter 内で
      registry.validate_lease(decode(lease.raw())) -> Ok
      → OS parent handle(HWND/NSView/XID)を adapter 内部で取得
      → host.execute(OpenGui{parent: GuiParentToken::new(hwnd)})  (L0)
  release_lease(lease) / release_slot(slot) -> LeaseRevoked 終端化
```

- `GuiParentToken::new(u64)` の呼出は上記 OS adapter 内部のみ —
  session 公開 API は `ParentSlotLease` を受ける。
- `validate_lease` が拒否した場合、adapter は OS handle を出さず
  session 側は `EditorOpError::LeaseRejected` → session state を
  `Failed{LeaseInvalid}` へ収束。

## Non-Functional Constraints

- 非 RT 専用（UI スレッド）。`unsafe` 不使用。外部依存追加なし。
- 全公開 enum `#[non_exhaustive]`（policy ADR）。
- `ruviform-ui` は backend 非依存を維持 — `native_slot` に
  gpui/wry/tao/OS handle 型を持ち込まない。
- plugin 系型を参照しない（`ParentSlotLease` 写像は消費側）。

## Verification

- `crates/ruviform-ui/tests/native_slot.rs`（headless）:
  declare/lease/validate/release/stale/double-lease busy/
  generation recycle/destroyed terminal/events drain/
  capabilities unsupported。
- `mise run fmt-check`/`check`/`lint`/`test` green。

## Deferred Design Concerns

- GPUI Driver の `NativeSurfaceSlot` 実装と実 OS adapter（HWND/
  NSView/X11 取得・swapchain 遮蔽）— RF386/#388 実機段。
- `ruviform-ui-gpui` 実装時の `SlotSpec`→GPUI Element/clip 写像。
- 実プラグイン binary fixture 選定とライセンス（固定/可変・別
  toolkit・GPU/WebView-backed）。
- Wayland native plugin embed の規格動向（CLAP wayland 等）追跡。
- 実機 PoC の必須ケース列（hide/tab/resize/float/re-dock/2Window/
  DPI/crash/hang/IME/popup/a11y）の実行設計。
