---
id: "DESIGN-x7UlzjoyvZa1pu2ARRLOZZ"
type: "design"
status: "approved"
title: "Issue #389 Driver-neutral PluginEditorSession 設計（state machine / typed HostPort / L0-L1 写像・wire negotiation 方針）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/editor_session.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-core/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/specs/issue-389-plugin-editor-session.md"
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/119"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 389
---

# Issue #389 Driver-neutral PluginEditorSession 設計

## Context

Issue #389 は規格別 Gui/View を Ruviform Window/Dock Presentation へ
接続する型付き Plugin Editor Session / Host Port を要求する。監査
（discovery F2/F3/F4）が確定した現実:

- L1 `OpenGui`/`CloseGui` は defined `Unsupported` rejection（wire 到達なし）。
- L0 `InProcessHost::open_gui` 経路は live だが全 adapter が既定
  `Unsupported`。
- wire GUI 動詞は keep-for-future で存在するが lifecycle 実動詞なし。

本設計は**新 Session 面を追加しつつ上記現実を変えない** — L1 は
Unsupported のまま、L0 は ops 経由で既存 `open_gui` 経路を再利用、
wire は変更しない（negotiation は設計のみ）。plugin ↔ ui 相互依存
禁止のため、session は `ParentSlotLease`（opaque u64）を運び、
`SurfaceSlotId`/OS handle 写像は消費側 composition の責務。

## 決定

### D1: `ruviform-plugin-core::boundary` への 2 型追加

```rust
pub struct PluginEditorSessionId { slot: u32, generation: u16 }
// PluginHandle と同形。pub fn index()/generation()、#[doc(hidden)] new。

pub struct ParentSlotLease(u64);
// session 面の opaque parent 語彙。#[doc(hidden)] new(u64)、pub raw()。
// u64 payload の構造は発行側（SurfaceSlotRegistry 等）の opaque 実装選択。
```

`GuiParentToken`/`GuiHandle`/`PluginHandle` は不変で共存。
`ParentSlotLease` は `GuiParentToken` を**置き換えない** — 発行・
検証機構を持たない raw token は OS adapter 内限定で残る。

### D2: `PluginEditorSessions` registry（id mint・instance 帳簿）

`crates/ruviform-plugin-host/src/editor_session.rs` — `SlotAllocator`
と同型の `{slot:u32,generation:u16}` 帳簿型。`PluginEditorSessionId`
を mint/recycle し、instance ごと「高々 1 live session」を強制する:

```text
begin_session(instance: PluginHandle) -> Result<PluginEditorSessionId, ...>
  instance に live session 既存なら Err(SessionBusy)
end_session / session_for_instance / active_session
```

### D3: `PluginEditorSession` 状態機械（純粋モデル）

```rust
#[non_exhaustive]
pub enum EditorSessionState {
    Closed, Creating { request: u64 },
    Attached { mode: AttachMode }, Hidden { mode: AttachMode },
    Detaching, Closing, Failed { cause: SessionFailCause },
    Recovering { request: u64 },
}

#[non_exhaustive]
pub enum AttachMode { Docked, Floating }
```

規範遷移（issue-389 状態表を固定）:

```text
Closed --open--> Creating{req} --attach_ok(mode)--> Attached{mode}
                         \--fail--> Failed{cause}
Attached --hide--> Hidden --show--> Attached
Attached/Hidden --begin_detach--> Detaching
  --detach_ok--> Creating{req'}(re-attach)  または --detach_fail--> Failed
Attached/Hidden/Detaching --close--> Closing --closed--> Closed
                              \--close_fail--> Failed
任意非終端 --ops error/instance gone/window lost--> Failed{cause}
Failed --recover--> Recovering{req} --attach_ok--> Attached
                                     \--fail--> Failed
Failed --close--> Closing --closed--> Closed
Closed --open--> Creating  （再 open 可・同一 session id）
```

**Terminal event 一度切り**: `notify_terminated()`（instance destroy・
host crash・child exit・window lost）は `EditorSessionEvent::
Terminated{cause}` を 1 回だけ発し state を `Closed` へ収束。
`Failed` 経由でも同 event は重複しない — event sink は後続
`notify_*` を無視する。これにより「二重 Open/Close・stale callback・
request 世代不一致・Window 破棄・Host crash が単一 terminal event
に収束する」規範を型で強制。

各遷移は `request: u64`（採番世代）を運び、stale callback（完了
request != 現行 request）は `StaleRequest` として無視 — 終端へ
誤収束させない。

### D4: `EditorSessionOps` port（adapter が実装する typed host port）

```rust
pub trait EditorSessionOps {
    fn capabilities(&self, session: PluginEditorSessionId) -> Result<EditorCapabilities, EditorOpError>;
    fn open(&mut self, session: PluginEditorSessionId, lease: &ParentSlotLease,
            size: EditorSize, mode: AttachMode) -> Result<EditorCapabilities, EditorOpError>;
    fn close(&mut self, session: PluginEditorSessionId) -> Result<(), EditorOpError>;
    fn set_size(&mut self, session: PluginEditorSessionId, size: EditorSize) -> Result<(), EditorOpError>;
    fn set_visibility(&mut self, session: PluginEditorSessionId, visible: bool) -> Result<(), EditorOpError>;
    fn reattach(&mut self, session: PluginEditorSessionId, lease: &ParentSlotLease) -> Result<(), EditorOpError>;
}
```

`EditorCapabilities`: `dock`/`float`/`resize`/`hide_show`/`reparent`/
`ime`/`popup` 等の旗艦。`EditorOpError`（`#[non_exhaustive]`）:
`Unsupported`/`LeaseRejected`/`Timeout`/`HostLost`/`Plugin`/
`Closed`/`InvalidState`/`Io`…。`Timeout` は request 別 timeout
規範（fake clock 検証）を持つ。

### D5: L0/L1 写像（実装本 dispatch）

- **L0**: `InProcessEditorOps<'a>(&'a mut InProcessHost, PluginHandle)`
  — `open` → `host.execute(HostCommand::OpenGui{handle,parent:
  GuiParentToken::new(lease.raw())})`。**ここが raw
  `GuiParentToken::new` の唯一の production 呼出点**（session 面で
  隔離）。`HostError::Unsupported` → `EditorOpError::Unsupported`
  等の定義済み写像。plugin 既定 `Unsupported` は `Unsupported` へ
  落ち、PASS に繰り上がらない。
- **L1**: `L1EditorOps` — 全 ops が `EditorOpError::Unsupported` を
  返す定義済み拒否（現行 `L1Instance::execute` の short-circuit と
  同じ意味を session 面で表現）。**wire に出さない・子 proc に
  届かない・L0 へ退化させない**。`capabilities` も同様に拒否。

これにより「L1 対応を黙って L0 へ退化させない」確定仕様を、実装
語彙として強制する。

### D6: wire negotiation 方針（設計のみ・実装しない）

issue-115 解除条件 (a) は確定済み（wire 版枠組み）。本設計の
negotiation 案（将来 bump 前提・実装しない）:

- `Message::Hello.capabilities`（u32）で GUI 能力 bit を宣言:
  `GUI_OPEN`=1,`GUI_LIFECYCLE`=2,`GUI_REPARENT`=4 等。受信側は
  未知 bit を許容せず既知 GUI bit の合意交差を算出 — 非対応側は
  `0` 合意＝ session open を `Unsupported` 拒否。
- bump 政策（issue-119）: 新 lifecycle 動詞追加時は `WIRE_VERSION`
  bump。`capabilities` の非 0 許容は bump と同時に行う（現行 v3 の
  `!=0`→`Malformed` hardening を v3 のまま変更しない）。
- L1 GUI 実現は issue-115 解除条件 (b)–(d)（OS 機構・crash isolation・
  実機証拠）充足後の別 Issue — 本 dispatch では `L1EditorOps` の
  Unsupported 表現で境界を固定するに留める。

### D7: RT・DSP 独立性規約

- `PluginEditorSession`/`ops` は全て非 RT 専用（GUI thread/イベント
  ループ層）。RT 呼出は API 層で不可能（port に RT コンテキストを
  渡す方法がない）。
- instance destroy 時、live session は `notify_terminated` で
  `Terminated{HostLost}` 収束 → `Closed`。DSP instance の破棄と
  widget teardown は独立 — session 側は instance の post-drop
  callback として後始末する（session が instance の生存を握らない）。
- View close/hang で DSP/Graph/Project/RT を暗黙停止しない — ops
  呼出は同期 Result だが「呼出側が GUI スレッドの async 文脈で
  実行し timeout 監視する」規約を `EditorOpError::Timeout` と
  request 世代で表現する。

### D8: 移行表（追加/廃止/保持）

| 項目 | 扱い |
| --- | --- |
| `GuiParentToken(u64)` | **保持** — OS adapter 内限定の raw 語彙として残存 |
| `GuiHandle`/`HostCommand::OpenGui`/`CloseGui`/`HostReply::Opened` | **保持** — L0 経路として session ops が内部利用 |
| `wire::Command::OpenGui`/`CloseGui`/`Reply::Opened` | **保持** — keep-for-future、L1 では依然 short-circuit |
| `L1Instance::execute` の GUI reject | **保持・不変** — session `L1EditorOps` と意味一致 |
| `Plugin::open_gui`/`close_gui`（既定 Unsupported） | **保持** — signature 不変、adapter 実装は別 Issue |
| `PluginEditorSessionId`/`ParentSlotLease` | **追加**（plugin-core boundary） |
| `PluginEditorSession`/`Sessions`/`Ops`/`State`/`Event`/`Error`/`Capabilities`/`AttachMode` | **追加**（plugin-host editor_session） |
| `HostError` | **保持** — 新 variant 追加なし（ops 側で `EditorOpError` へ写像） |
| `WIRE_VERSION`=3/`Hello.capabilities` | **保持** — negotiation は設計のみ |

## Data and Control Flow

```text
consumer(GUI Driver / app):
  slot_id = ui_driver.declare_slot(spec)              (ruviform-ui 側)
  lease   = ui_driver.lease_parent(slot_id)           (SlotLease, opaque)
  session = host_sessions.begin_session(plugin_handle)
  session_state.open(ops, ParentSlotLease::new(lease.encode()), size, Docked)
    -> ops.open → (L0) GuiParentToken::new(lease.raw) → HostCommand::OpenGui
       → plugin.open_gui → Ok/Unsupported
    -> Ok  → state=Attached{Docked} + event Attached
    -> Err → state=Failed{cause} + event Failed
  ... hide/detach/resize/close 各 request 世代付き遷移
  instance destroy → sessions.notify_terminated(instance)
    → 全 live session: Terminated{HostLost} 1回 → Closed
```

## Non-Functional Constraints

- 非 RT 専用・`unsafe` 不使用・外部依存追加なし。
- 全公開 enum `#[non_exhaustive]`。
- `ruviform-plugin-host` は `ruviform-ui` に依存しない（lease は
  opaque u64）— plugin↔ui 相互依存禁止を維持。
- wire・`Plugin` trait・`HostError` の公開面は不変。

## Verification

- `crates/ruviform-plugin-host/tests/editor_session.rs`（fake ops）:
  full lifecycle・state 表・terminal 一度切り・stale request/
  double-open・double-close・instance destroy converge・
  unsupported・timeout・crash/host lost。
- L0 ops 写像: `InProcessHost` + stub `ErasedPlugin`（`open_gui`
  が `Ok(GuiHandle)` を返す fake）で live 経路を通す。
- `mise run fmt-check`/`check`/`lint`/`test` green。

## Deferred Design Concerns

- `PluginEditorSession` を GPUI Dock の View へ実接続する
  composition root（`ruviform-ui-gpui`/`ruviform-engine` 利用側）。
- L1 wire 動詞（`open`/`close`/`set_size`/`set_visibility`/
  `reattach`）と `Hello.capabilities` GUI bit の bump 実施（issue-115
  解除条件充足後の別 Issue）。
- 各 OS adapter の `open_gui` 実装本体（CLAP `clap_plugin_gui`/
  VST3 `IPlugView` の HWND/NSView/XID attach）— 別 Issue 級。
- 実 Dock PASS の必須ケース実行（hide/tab/resize/float/re-dock/
  2Window/DPI/crash/hang/IME/popup/a11y/RT 非停止）。
- timeout の wall-clock 監視基盤（GUI runtime 側の timer/cancel —
  現行 port は request 世代で「完了対照」を強制するに留める）。
