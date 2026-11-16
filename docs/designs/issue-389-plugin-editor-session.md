---
id: "DESIGN-x7UlzjoyvZa1pu2ARRLOZZ"
type: "design"
status: "approved"
title: "Issue #389 Driver-neutral PluginEditorSession 設計（state machine / typed HostPort / L0-L1 写像・wire negotiation 方針）"
created: "2026-11-07"
updated: "2026-11-15"
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
  `Unsupported`。（改訂 2 時点: VST3・CLAP adapter は `open_gui`/
  `close_gui` を実装済み — R5・S2 参照）
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

`GuiHandle`/`PluginHandle` は不変で共存。`GuiParentToken` は
改訂 2 時点で**完全削除済み**（`crates/**/*.rs` 0 hit —
`ParentLease`/`ParentSlotLease` へ移行）。session 面の opaque 語彙は
`ParentSlotLease`、OS adapter redemption の出力は `ParentLease`
（`generation: u32` + `payload: u64`）。

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

- **L0**: `InProcessEditorOps<'a>(&'a mut InProcessHost, PluginHandle,
  redeemer)` — `open` → `redeemer(lease)` が `ParentSlotLease` を
  issuer 側で検証・redeem し `ParentLease` を mint → `host.execute(
  HostCommand::OpenGui{handle,parent})`（`OpenGui.parent` の型は
  `ParentLease` — `GuiParentToken` から移行済み・改訂 2）。
  非 `Docked` mode は L0 command surface が plain open のみ運ぶ
  ため `Unsupported` を明示拒否（黙った docked 着地を fake
  success にしない）。`HostError::Unsupported` →
  `EditorOpError::Unsupported` 等の定義済み写像。plugin 既定
  `Unsupported` は `Unsupported` へ落ち、PASS に繰り上がらない。
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
| `GuiParentToken(u64)` | **削除済み**（改訂 2 で reconcile）— `ParentLease`/`ParentSlotLease` へ移行。`crates/**/*.rs` に残存参照なし |
| `GuiHandle`/`HostCommand::OpenGui`/`CloseGui`/`HostReply::Opened` | **保持** — L0 経路として session ops が内部利用 |
| `wire::Command::OpenGui`/`CloseGui`/`Reply::Opened` | **保持** — keep-for-future、L1 では依然 short-circuit |
| `L1Instance::execute` の GUI reject | **保持・不変** — session `L1EditorOps` と意味一致 |
| `Plugin::open_gui`/`close_gui`（既定 Unsupported） | **保持** — signature 不変。VST3・CLAP adapter 実装は済（改訂 2・R5 参照）。AU は初期対象外のまま |
| `PluginEditorSessionId`/`ParentSlotLease` | **追加**（plugin-core boundary） |
| `PluginEditorSession`/`Sessions`/`Ops`/`State`/`Event`/`Error`/`Capabilities`/`AttachMode` | **追加**（plugin-host editor_session） |
| `HostError` | **保持** — 新 variant 追加なし（ops 側で `EditorOpError` へ写像） |
| `WIRE_VERSION`=5/`Hello.capabilities` | **保持** — 現行値 5（改訂 2 時点）。`capabilities` は reserved u32・非 0 は `Malformed` hardening のまま。negotiation は設計のみ |

## Data and Control Flow

```text
consumer(GUI Driver / app):
  slot_id = ui_driver.declare_slot(spec)              (ruviform-ui 側)
  lease   = ui_driver.lease_parent(slot_id)           (SlotLease, opaque)
  session = host_sessions.begin_session(plugin_handle)
  session_state.open(ops, ParentSlotLease::new(lease.raw()), size, Docked)
    -> ops.open → (L0) redeemer(lease) → ParentLease → HostCommand::OpenGui
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

## 改訂 1: DoD 再検証（Issue #389 PR・2026-11-08）

Issue #389 の DoD（移行表・状態遷移表・所有責務・fake host 検証・
Capability Matrix）を現行コードに照らして再検証し、発見した欠陥を
修正した。D1〜D8 の決定は維持し、以下を追記・修正する。

### R1: 欠陥修正（session 状態機械の不変条件）

- **Refused open が L0 instance を永久に塞ぐ欠陥を修正**: 従来は
  open 拒否で `Failed` になった session を `close` すると adapter の
  `CloseGui` が呼ばれ（plugin 既定 `Unsupported`）、再び `Failed` に
  戻るため `concluded` にならず `begin_session` が常に `SessionBusy`
  を返した。修正: `PluginEditorSession` に `editor_live`（adapter 側
  editor が生存しうるか）を追加。成功 attach で `true`、conclusion で
  `false`。`close` は `editor_live == false` なら adapter を呼ばず
  `Closed` へ収束する。`reattach` 拒否のように editor が生存しうる
  `Failed` は従来どおり adapter `close` を呼ぶ（ES-19/ES-22）。
- **L0 pass-through 廃止**: `InProcessEditorOps::new`（lease payload を
  無検証に `GuiParentToken::new(lease.raw())` として `OpenGui` へ渡す
  経路）を削除。L0 の `OpenGui` は必ず `LeaseRedeemer` を経由する。
  `with_redeemer` が唯一の constructor。
- **L0 capability の過大申告を修正**: 従来 `capabilities()` は open 前に
  `dock`/`close` を `Supported` と返していた。L0 surface は plugin の
  GUI 対応を問い合わせる手段がないため、open 前は全 `Unsupported`、
  open 成功の戻り値でのみ `dock`/`close` を主張する（ES-24）。
- **terminal once の規律を簡素化**: `terminated` フラグを削除。
  termination は必ず `concluded` を立てるため、`Closed` 判定だけで
  重複 notify を無視できる（clippy `struct_excessive_bools` 対応を
  bool 追加の抑制ではなく冗長状態の削除で解消）。

### R2: 移行表（D8 の増分）

| 現行 Public API / 内部 | 新設計での扱い | 根拠 |
| --- | --- | --- |
| `InProcessEditorOps::new(host, instance)`（pass-through） | **廃止** | lease を無検証で token 化する経路 |
| `InProcessEditorOps::with_redeemer` | **保持（唯一の constructor）** | poc-native-dock・GPUI driver の redemption 接続点 |
| `LeaseRedeemer<'a>`（出力 `ParentLease`） | **保持・seam として明示** | `GuiParentToken` は改訂 2 で完全削除済み — redemption 出力は `ParentLease{generation:u32,payload:u64}`（`plugin-core boundary`）。公開 API から raw token の除去は達成 |
| `InProcessEditorOps::capabilities`（open 前 dock/close=Supported） | **挙動変更**: 全 Unsupported | 未検証の GUI 対応を主張しない |
| `PluginEditorSession` 内部 `terminated: bool` | **廃止** | `concluded` と `Closed` 判定で代替 |
| `PluginEditorSession` 内部 `editor_live: bool` | **追加**（非公開フィールド） | refused open の close 規律 |
| `PluginEditorSession::close` | **挙動変更**（非 live Failed は adapter 非呼出し） | R1 |
| 既存 editor_session contract 22 件（fake/L0/L1） | **保持**（test 名・期待値不変） | 非回帰 |

### R3: 状態遷移表（現行実装）

| 状態 | `open`/`begin_open` | `recover` | `set_visibility` | `set_size` | `reattach` | `close` | `notify_terminated` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `Closed`（未 open） | `Creating` → ok: `Attached`(`Attached`) / err: `Failed`(`Failed`) | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | 既存 session を `concluded` 化、イベントなし |
| `Closed`（concluded） | `InvalidState`（再 open 不可・instance 解放は registry） | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | イベントなし（無視） |
| `Creating{r}` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState`（in-flight 完了を待つ） | `Closed` + `Terminated`、遅延 `complete_*` は `StaleRequest` |
| `Attached{m}` | `InvalidState` | `InvalidState` | `Hidden{m}` (`Hidden`) / 不変 | 不変（拒否は非致命） | detach→re-attach: `Attached`(`Detached`,`Reattached`) / 拒否: `Failed` | `Closing` → `Closed`(`Closed`) / 拒否: `Failed` | `Closed` + `Terminated` |
| `Hidden{m}` | `InvalidState` | `InvalidState` | `Attached{m}` (`Shown`) | 不変 | 同 `Attached` | 同 `Attached` | `Closed` + `Terminated` |
| `Detaching{r}` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | atomic: `Attached`(`Reattached`) / 拒否: `Failed` | `Closing` | `Closed` + `Terminated` |
| `Closing{r}` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | `Closed` + `Terminated`、遅延 `complete_close` は `StaleRequest` |
| `Failed{c}`（editor 生存しうる） | `InvalidState` | `Recovering{r}` → ok: `Attached`(`Recovered`) / err: `Failed` | `InvalidState` | `InvalidState` | `InvalidState` | `Closing` → `Closed` / 拒否: `Failed` | `Closed` + `Terminated` |
| `Failed{c}`（refused open・`editor_live=false`） | `InvalidState` | `Recovering{r}`（同上） | `InvalidState` | `InvalidState` | `InvalidState` | **adapter 非呼出しで `Closed`**（`Closed` イベント） | `Closed` + `Terminated` |
| `Recovering{r}` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | `InvalidState` | `Closed` + `Terminated`、遅延 `complete_attach` は `StaleRequest` |

共通規則: 完了は必ず request id で照合し、不一致は `StaleRequest`
（状態不変・イベントなし）。`Terminated` は non-`Closed` から
`Closed` へ落ちる遷移でのみ発火し、以後は無視される。

### R4: 所有責務

| 概念 | 所有者（生成・破棄） | 本契約での役割 |
| --- | --- | --- |
| `PluginInstanceId`（= `PluginHandle`） | `InProcessHost`（plugin-host） | DSP instance の寿命。session はこれを握らず、destroy 時は `notify_terminated` を受けるだけ |
| `PluginEditorSessionId` | `PluginEditorSessions`（plugin-host registry） | slot+generation で mint/recycle。stale id は `UnknownSession` |
| `ParentSlotLease` | 発行側（UI 側 `SurfaceSlotRegistry` / `GpuiSurfaceAdapter`） | opaque 値。session は運ぶだけで解釈しない |
| `SurfaceSlotId` / `WindowId` / `ViewInstanceId` | `ruviform-ui`（dock/window 所有） | plugin-host の型には現れない。composition root が `lease` へ写像する |
| raw parent（`ParentLease` / HWND・NSView 等） | OS adapter（`LeaseRedeemer` の出力側） | redemption 直後に `OpenGui` へ渡すのみ。session 面に現れない。`GuiParentToken` は削除済み（改訂 2） |
| widget teardown | UI driver（Dock/Window 側） | `Terminated` を受けて後始末。DSP/RT の停止を伴わない |
| editor 生存判定（`editor_live`） | `PluginEditorSession` | 成功 attach で真、conclusion で偽。close 規律のみに使う |

### R5: Capability Matrix（現行コード根拠・改訂 2 で reconcile）

凡例: **Supported** = コード上で成功経路あり（実プラグイン実行の
有無は行の根拠欄に記載）。**Unsupported** = 定義済み拒否。**NotRun**
= 実機・実 binary 未実施。**対象外** = 初期スコープ外。

| 経路 | Dock（Docked open） | Floating | Resize | Hide/Show | Reparent | Close | 根拠・状態 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| L0 / VST3（Windows） | Supported（`open_gui` 実装・`kPlatformTypeHWND`・`createView("editor")`） | Unsupported（`InProcessEditorOps` が非 Docked を明示拒否） | Unsupported（verb なし） | Unsupported | Unsupported | Supported（`close_gui` 実装） | `ruviform-plugin-vst3/src/host.rs` L2614/L2675。実外部 VST3 module の HWND attach 証拠は `evidence/rf389-vst3-editor/`（task `issue-389-vst3-l0-editor-attach` completed）。`createView` null の plugin（test_gain 等）は honest `Unsupported`（dock-test `editor-attach` 実測） |
| L0 / VST3（非 Windows） | Unsupported（`EDITOR_PLATFORM_TYPE=None`） | Unsupported | Unsupported | Unsupported | Unsupported | Unsupported | 非 Windows attach は未実装 |
| L0 / CLAP | Supported（`clack-extensions` `gui` ext `PluginGui` 経由の `open_gui`/`close_gui`）※ ext 非対応 plugin は honest `Unsupported` | Unsupported（同上・非 Docked 明示拒否） | Unsupported | Unsupported | Unsupported | Supported（`close_gui` 実装） | `ruviform-plugin-clap/src/host.rs` L1092/L1150。`clap.gui` ext を持たない plugin/fixture は `Unsupported`（`open_gui_without_gui_extension_is_unsupported`）。実外部 CLAP 証拠は `evidence/rf389-clap-editor/`（task `issue-389-clap-l0-editor-attach` done） |
| L0 / AU（macOS） | 対象外 | 対象外 | 対象外 | 対象外 | 対象外 | 対象外 | ADR `plugin-architecture-clap-vst3`: AU は初期対象外。adapter crate なし |
| L0 / Ruviform 製 GPUI Plugin | NotRun | NotRun | NotRun | NotRun | NotRun | NotRun | plugin 側 GPUI editor の adapter・実装なし |
| L1 out-of-process（全規格） | Unsupported | Unsupported | Unsupported | Unsupported | Unsupported | Unsupported | `L1EditorOps` 全 op 拒否・`l1/mod.rs` の `OpenGui`/`CloseGui` 拒否 |
| Fake ops（contract・既定 `FakeOps::ok`） | Supported（fake） | 成功する（fake は mode/caps を強制しない） | 成功する（fake） | 成功する（fake） | 成功する（fake） | Supported（fake） | 状態機械の契約検証のみ。fake の既定 caps は dock/close のみで、op は caps を検査しない。**実 Dock PASS ではない** |

Ruviform 製 GPUI Plugin 対 外部 DAW の埋込み方向は、本 PR では
capability として宣言しない（実装 Issue が必要）。

### R6: DoD 達成状況（この PR 時点）

| DoD 項目 | 状態 | 根拠・未達理由 |
| --- | --- | --- |
| L0/L1 移行表（追加/廃止/保持）・状態遷移表・所有責務 | **達成（文書）** | R2・R3・R4 |
| fake driver の Open/Attach/Reparent/Detach/Close・stale generation・double close・timeout・OS unsupported・crash の terminal-event | **達成（契約層）** | `tests/editor_session.rs` 28 件。Reparent は `reattach` で検証。crash は `HostLost` で代用（GPU host 固有 cause は未定義・未達） |
| 代表実 Plugin/Format の L0/L1 capability・GUI hang で DAW/RT を巻き込まない実機受入 | **部分達成** | L0 VST3/CLAP の `open_gui`/`close_gui` は実外部 module で検証済み（`evidence/rf389-vst3-editor/`・`evidence/rf389-clap-editor/`）。dock-test は test_gain 実 binary で session 契約を実走検証（`evidence/rf389-dock/`）。GUI hang・DAW/RT 巻き込みの実機受入は依然 NotRun |
| Floating / Recovery の安全 fallback（Floating へ明示的に degrade） | **部分達成** | session 面に `open_or_degrade`（`OpenOutcome::Degraded` + `Degraded` event）と mode 指定の `recover` が実装済み（session-hardening）。dock-test `open-or-degrade` case が実 binary で double-refusal → `Ops(Unsupported)`・degrade event 0 件を実測（`evidence/rf389-dock/`）。L0 Floating attach 自体は依然 `Unsupported`、実 editor への degrade attach は NotRun |
| GPUI 型・raw native handle の公開 API 非露出 | **達成（session 面）** | `GuiParentToken` は完全削除済み（`crates/**/*.rs` 0 hit — 改訂 2）。session 面は `ParentSlotLease` opaque のみを運び、`ParentLease` は adapter-internal seam（`#[doc(hidden)] new`）。`editor_session.rs` 名前検査（ES-23）と `check-ui-driver-boundary` で確認 |
| `SurfaceSlotId`/`WindowId`/`ViewInstanceId` の分離 | **設計文書で分離（型は未追加）** | R4 のとおり UI 側所有。plugin-host に写像用の型を増やさない（YAGNI）。consumer の composition 実装は別 Issue |
| 同一 Plugin Instance の Dock/Float/Hidden 同一意味論 | **部分達成** | 状態機械上は同一（R3）。実装は L0 Docked のみ |
| Native Dock の可否（#388 Driver capability への委譲） | **委譲のまま未解決** | #388 gate は OPEN。本 PR は Native Dock PASS を主張しない |
| 両 Consumer 非回帰 | **NotRun** | Consumer repo は本 worktree 外（scope 外） |

Issue #389 は本 PR で close しない（上記未達項目を follow-up として残す）。

## 改訂 2: verification-status reconcile（2026-11-15・dock-test session-lifecycle slice）

`ruviform-plugin-dock-test` へ実バイナリ session lifecycle case を
追加した slice（`docs/tasks/issue-389-session-lifecycle.md`）に伴い、
本書の stale 記述を現行コードの事実へ reconcile した。D1〜D8・
R1〜R6 の決定履歴は維持し、以下が現行の真実である。

### S1: parent 語彙（`GuiParentToken` → `ParentLease`）

- `GuiParentToken` は **完全削除済み**（`crates/**/*.rs` で 0 hit）。
  移行 commit: `66fcce0b feat(plugin): migrate GUI parent vocabulary
  to typed ParentLease`。
- 現行 seam: session 面 `ParentSlotLease(u64)`（opaque・発行側のみ
  mint）→ issuer 側 `LeaseRedeemer` が検証・redeem → adapter 内部で
  `ParentLease{generation: u32, payload: u64}` を mint
  （`ParentLease::new` は `#[doc(hidden)]`）→ `HostCommand::OpenGui
  { handle, parent: ParentLease }`。
- `WIRE_VERSION` 現行値は **5**（`crates/ruviform-plugin-core/src/
  wire.rs`）。`Hello.capabilities` は reserved u32 で非 0 は
  `Malformed` — negotiation は依然設計のみ。
- これに伴い D1・D5・D8・R2・R4・R6 の `GuiParentToken` 言及を
  上記現状へ更新済み。

### S2: L0 capability matrix 訂正

- **VST3**: `open_gui`/`close_gui` は実装済み
  （`ruviform-plugin-vst3/src/host.rs` L2614/L2675・Windows
  `kPlatformTypeHWND`・`createView("editor")`）。実外部 VST3 module
  の HWND attach 証拠は `evidence/rf389-vst3-editor/`（task
  `issue-389-vst3-l0-editor-attach` completed）。R5 の「実 attach
  の証拠は #388 記録（NotRun）」は stale だったため訂正。
- **CLAP**: `open_gui`/`close_gui` は `clack-extensions` `gui` ext
  （`PluginGui`）経由で実装済み（`ruviform-plugin-clap/src/host.rs`
  L1092/L1150・task `issue-389-clap-l0-editor-attach` done・
  `evidence/rf389-clap-editor/`）。`clap.gui` ext を持たない
  plugin/fixture への応答は honest `Unsupported`
  （`open_gui_without_gui_extension_is_unsupported`）。R5 の
  「`Plugin` 既定 Unsupported」は stale だったため訂正。
- 未実装 verb（resize/hide-show/reparent/IME/popup 等）は L0 ops が
  `EditorOpError::Unsupported` を返す現状のまま — Supported へは
  書き換えない。L0 `AttachMode::Floating` も `InProcessEditorOps`
  が明示拒否（L0 command surface は plain open のみ）。
- **dock-test 実バイナリ証拠（本 slice）**: `test_gain` VST3 binary
  に対し `session-lifecycle`/`host-terminate`/`open-or-degrade`
  が `Pass`（`environment_kind=real-abi-test-plugin`）。ただし
  test_gain は `createView` null で editor surface を持たないため
  open は honest `Unsupported` — session registry 契約
  （`SessionBusy`・refused-open converge・`Terminated{HostLost}`
  exactly-once・degrade 非発火）の実走証拠であり、実 editor attach
  や Native Dock の証拠ではない（`evidence/rf389-dock/`）。

### S3: DoD 状況の更新

- Floating/Recovery fallback: session 面に `open_or_degrade`
  （`OpenOutcome::Degraded` + `Degraded` event）と mode 指定の
  `recover` が実装済み（session-hardening）。L0 Floating attach
  自体は依然 `Unsupported`。実 editor への degrade attach は NotRun。
- raw native handle 公開 API 非露出: `GuiParentToken` 除去により
  session 面の gap は解消（`ParentLease` は adapter-internal seam
  に限定）。
- Issue #389 は依然 OPEN — L2/L3 実 GUI（`test_editor` fixture・
  実 Window/Dock）・consumer gates は後続 slice の scope。

## 改訂 3: real-window session lifecycle slice（2026-11-17・rf389-session-lifecycle）

`rf631.plugin-dock-real` lane を 15 → 21 row へ拡張し、Issue #389
session lifecycle の real-window 受入を 6 row 追加した slice
（`docs/tasks/issue-389-real-session-lifecycle.md`）。実装は
`crates/ruviform-plugin-dock-test`（feature `real-dock`）に閉じ、
plugin-host/plugin-vst3/plugin-clap は無変更。NativeSlot →
container HWND → `ParentSlotLease` → `InProcessEditorOps` redeem
の実 chain を全 case が通る。

### S4: 追加 row と verdict（interactive Windows 実走）

| row | verdict | 内容 |
| --- | --- | --- |
| `real-session-degraded` | **pass** | `open_or_degrade(Floating, Docked)` — Floating は `InProcessEditorOps` が `Unsupported` で拒否、Docked fallback が attach。`OpenOutcome::Degraded{Docked}` + event 列 `[Degraded{Docked}, Closed]`（`Attached` 非発火）+ child HWND 契約を実測 |
| `real-session-recovery` | **pass** | `open(Floating)` refused（`editor_live=false`）→ `recover(Docked)` で attach。event 列 `[Failed{Unsupported}, Recovered{Docked}, Closed]` + child HWND 契約を実測 |
| `real-session-timeout` | **pass** | attach 後 ZERO `SessionDeadlinePolicy` + `begin_detach` → `tick_all()==1` で `Failed{Timeout}`。event 列 `[Attached{Docked}, Failed{Timeout}, Closed]` + child HWND teardown を実測 |
| `real-editor-visibility` | **unsupported** | `set_visibility(false)` は定義済み `EditorOpError::Unsupported` — session は `Attached{Docked}` 維持・`Hidden`/`Shown` event 0 件を確認した上で refused boundary として正直に記録 |
| `real-session-crash` | **unsupported** | in-process lane は fixture の process-killing crash fault を生存できず、out-of-process plugin host は存在しない — 実行せず boundary row として記録 |
| `real-session-hang` | **unsupported** | 同上 — wedging/hang fault は lane を永久に塞ぐため実行しない。architectural boundary を明示 |

### S5: capability/DoD reconcile

- R5 matrix の L0 VST3 行「Floating: Unsupported（`InProcessEditorOps`
  が非 Docked を明示拒否）」は本 slice の degraded/recovery 実測で
  **裏付けられた**（拒否は fake ではなく実 lease chain 上で観測）。
  `set_visibility`/`reattach`/`set_size` の Unsupported も同様に
  実 window 上で確認。
- R6「Floating / Recovery の安全 fallback」: 実 editor への degrade
  attach が **達成**（`real-session-degraded`/`real-session-recovery`
  Pass・`evidence/rf389/`）。L0 Floating attach 自体は依然
  `Unsupported` のまま — degrade は Docked への安全 fallback であり
  Floating 成功ではない。
- crash/hang の実機受入は **依然 NotRun 相当** — in-process lane の
  architectural boundary として `Unsupported` を維持。process 分離
  host（L1 実装）が前提の後続 slice。
- `real-dcomp-pixel` の `fail`（GPUI DComp topmost occlusion・
  #532 決裁待ち）は本 slice でも不変 — 既知・対象外。
- headless fail-closed: `test-ui-plugin-dock-real-headless` が
  21 row 全て `not_run` + `observed.environment_blocked` を記録
  （violation 0）。catalog と executable entry の同期は
  `real_catalog_matches_entries` test が gate。
