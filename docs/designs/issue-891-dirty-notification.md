---
id: "DESIGN-891dirtyNoticePull9x"
type: "design"
status: "approved"
title: "Issue #891 plugin→host state dirty 通知経路設計（DirtyNotice + 専用 pull verb、wire v7）"
created: "2026-11-23"
updated: "2026-11-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/891"
    - "docs/specs/issue-891-dirty-notification.md"
    - "docs/discovery/issue-868-host-dirty-notification.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-891-dirty-notification.md"
  implements:
    - "docs/specs/issue-891-dirty-notification.md"
  implemented-by:
    - "docs/test-specs/issue-891-dirty-notification.md"
  depends-on:
    - "docs/designs/issue-873-plugin-gui-request.md"
    - "docs/specs/issue-630-adapter-state-bridge.md"
  blocks: []
---

# Issue #891 plugin→host state dirty 通知経路 — design

## Context

Issue #868 の discovery が残した確定論点: (a) 中立語彙の形、
(b) `performEdit` の id+value を param report 配線するか dirty 扱いに
留めるか、(c) `restartComponent` flag 別応答、(d) gesture 扱い、
(e) `IComponentHandler2` 兄弟 verb の応答方針、(f) ack-and-drop の解消法。
本設計は #873 の pull seam（中立 `#[non_exhaustive]` enum + trait pull
verb + sink/drain 責務分割）と `drain_output_reports` family（
`Command::DrainOutputReports`/`Reply::Reports` → `L1Instance` chunked
pull loop → 子側 backlog dispatch → `PluginHost` composite 照会面）を
雛形として確定する。

## Decisions

### D1 — 中立語彙は `EventKind` 拡張ではなく専用 `DirtyNotice` + 専用 pull verb

`Event` は `frame_offset` を持つ sample-accurate 契約。dirty 通知は
「再 save が必要」の lifecycle 隣接フラグで frame offset を持たず、
`Event` に載せると frame_offset の意味を嘘にする。`Reply::Reports`
payload 拡張（`Event` とは別フィールドを足す）も却下 — Reports は
process-path 出力の配送契約であり、restore/init 由来の通知を混ぜると
「process 由来」という読み手の前提を壊す。ゆえに:

- `boundary.rs` に `DirtyNotice`（`#[non_exhaustive]` `Copy` —
  `StateDirty`/`StateClean`/`ParamEdited { param_id, value }`）と
  `DirtyNoticeQueue`（bounded・coalesced 格納の正本）を置く。
- `plugin.rs` に `Plugin::drain_dirty_notices`（default 空実装）を
  追加し、`ErasedPlugin` trait member として中継する。
- wire には `Command::DrainDirtyNotices`（tag 28）と
  `Reply::DirtyNotices`（tag 9）を追加し `WIRE_VERSION` を `7` に
  bump（verb・reply payload 追加は bump 必須 — `wire.rs` module doc
  の契約; `4`=`#253` report 改訂、`6`=`#872/#882` GUI floating 改訂の
  先例）。
- `command.rs` の `HostCommand`/`HostReply` には触れない —
  `drain_output_reports`/`drain_gui_requests` と同じく、drain は
  `PluginHost`/`InProcessHost` の pub method として立つ。

### D2 — `DirtyNoticeQueue` の格納契約

「re-save me」は状態であって発生回数ではないため、flag は
`Option<bool>` latch で最終宣言勝ち。`ParamEdited` は payload を持つため
coalesce せず `MAX_DIRTY_EDITS = 64` の FIFO backlog（超過は最古を
drop、新しい値が最も新鮮な parameter 状態であるため — drop 数は
`dropped_edits` で観測可能）。`ParamEdited` enqueue は dirty を latch
する（bare `performEdit` でも flag-only consumer が dirty を拾える）。
`drain` 順は edits（古い順）→ latched flag edge。

RT 契約: enqueue/drain とも non-RT 面（adapter callback と host pull
verb）。RT `process` からは呼ばない。

### D3 — CLAP: `HostState` 登録 + `BridgeMainThread`

`mark_dirty` は `[main-thread]` 契約のため `Shared` 側ではなく
`MainThread<'a>` handler を実装する。`BridgeMainThread<'a>` は
`&'a BridgeShared` を保持し、`HostStateImpl::mark_dirty` で
`BridgeShared.dirty_notices`（`Arc<Mutex<DirtyNoticeQueue>>`）へ
`StateDirty` を enqueue する。`BridgeHandlers::declare_extensions` で
`HostGui` に並んで `HostState` を `register` する。

queue は `ClapPlugin` の `dirty_notices` field と instance 生成時に
`Arc::clone` で共有し、`Plugin::drain_dirty_notices` が空にする。
poisoned lock は defined drop（delivery の虚偽 ack を返さない
契約と整合）。

### D4 — VST3: `IComponentHandler2` 登録と応答方針

`BridgeComponentHandler` の `Interfaces` を
`(IComponentHandler, IComponentHandler2)` とし、両面を実装する:

- `performEdit(id, value_normalized)` → 有限 value は
  `ParamEdited{ParamId(id), value}`、非 finite は `StateDirty` に
  degrade（`ParamValue` 有限域を中立面に持ち込まない — ack-and-drop
  の解消は「配線」側で取る）。`kResultOk`。
- `setDirty(0)` → `StateClean`、`setDirty(非0)` → `StateDirty`。
  `kResultOk`。
- `restartComponent(flags)` → `kParamValuesChanged` のみ受理
  （bit の exact 一致 — 他 bit を含む複合は拒否）して
  `StateDirty` を enqueue、`kResultOk`。`kParamValuesChanged` の意味は
  「param 値が変わったので再送してほしい」であり dirty edge として
  意味を持つ。他の flag（`kReloadComponent`/`kIoChanged`/
  `kLatencyChanged`/`kMidiCCMappingChanged`/`kKeyswitchChanged` 等）は
  bridge が対応する再送・reload を行わないため `kNotImplemented` —
  `kResultOk` は約束を立てる嘘になる。
- `beginEdit`/`endEdit` → `kResultOk` のまま（gesture edge 自体は
  通知を持たない; gesture 内の `performEdit` が実通知を運ぶ。
  gesture を別 notices にすると noise でしかない）。
- `requestOpenEditor`/`startGroupEdit`/`finishGroupEdit` →
  `kNotImplemented`（QI 面に載る以上、正直な拒否を明示する）。

queue は `Rc<RefCell<DirtyNoticeQueue>>`（bridge は単一スレッド
main-thread 面のみ — `Rc` で十分）を `Vst3Plugin` と handler で
共有し、`drain_dirty_notices` は `try_borrow_mut` で空にする
（再入 borrow は空 drain — callback 中の reentrant drain を
deadlock させない）。

### D5 — L0/L1 配送

- L0: `InProcessHost::drain_dirty_notices`（`Plain` 直接呼び +
  `Shared` pinned three-tier lock — `drain_gui_requests` と同じ
  振り分け）→ `PluginHost::drain_dirty_notices`（`Result<ReportDrain,
  HostError>` — L0 delegate / L1 wire pull / video `Unavailable` /
  stale `InvalidHandle`）。
- L1: `L1Instance::drain_dirty_notices` が
  `Command::DrainDirtyNotices` を `WIRE_MAX_DIRTY_NOTICES` 単位で
  chunked pull（short reply まで反復 — `drain_output_reports` と
  同型）。export runner は `InstanceEntry.dirty_backlog` に erase 済
  plugin の drain を積み、reply で先頭 `WIRE_MAX` 件を返す
  （残りは次 drain）。
- 認証不要の version 担保: handshake が version 一致を要求するため
  capability bit は追加しない — v7 peer は verb を知っている
  （GUI floating verb が negotiated bit を持つ先例は capability
  差分が旧 child で動く必要があったため; dirty verb 新設は
  version bump 自体で担保）。

### D6 — fixture 発火

- test-clap: `apply_restored_gain`（state `load`/`preset-load`
  restore の共有 tail — main-thread 面）が値変更 edge で
  `HostParams::rescan(VALUES)` に並んで `HostState::mark_dirty` を
  呼ぶ。`HostState` が host に無い場合は no-op — 発火は
  host extension presence に依存する CLAP 本来の law。
- test-vst3: `ParamController`（distributed `TestStatefulController`
  が delegate する共有 shell）に `AtomicPtr<IComponentHandler>` で
  handler を保持（非所有 — VST3 契約上 host 所有の参照、寿命は
  オブジェクトの存続期間）。`setComponentState` は verb 自体が
  change signal（documented restore 列 `component.setState` →
  `component.getState` → `setComponentState` は状態再適用時のみ
  走る）として `adopt_and_announce` が無条件に
  `performEdit(param.id, normalized)` + QI 取れた場合
  `IComponentHandler2::setDirty(true)` を呼ぶ — controller の
  adopted mirror は process-path の param 適用
  （`IParameterChanges`）を見ず、fixture は `IConnectionPoint`
  messaging を持たないため stored-value 比較では component が
  実際に変化した restore を抑止してしまう。`setState`
  （controller-private state）は `store_and_mark` が値変更
  edge のみ `setDirty` で発火し `performEdit` echo はしない。
  stateless class は verb が `kNotImplemented` で発火点に
  到達しない。`TestStatefulSingle` は once-contract oracle の
  まま変更しない。

### D7 — video slot

`OutOfProcessVideo` slot は `ReportDrain::Unavailable` —
`drain_output_reports` と同じ「video family に surface が無い」
正直な回答。

## 不変条件・契約

- dirty 通知は `Event`/`HostCommand` 語彙に載らない。pull verb のみ
  drain する; drain なしに通知は配送されない。
- `DirtyNotice::ParamEdited` の `value` は `ParamValue`（有限
  normalized）— 非有限は adapter 境界で `StateDirty` に degrade。
- queue は bound 済み: flag latch（O(1)）+ edit backlog ≤ 64 —
  無制限成長しない。`dropped_edits` で drop 観測。
- wire: verb tag 28・reply tag 9・notice tag {0,1,2}・
  `WIRE_MAX_DIRTY_NOTICES` = 1024・`WIRE_VERSION` = 7。
- non-RT: enqueue/drain/adapter callback は main-thread/lifecycle 面。
  RT `process` は queue に触れない。

## Consumer 影響分類

plugin-core に `DirtyNotice`/`DirtyNoticeQueue`/`MAX_DIRTY_EDITS` と
`Plugin::drain_dirty_notices`（default 空実装 — downstream adapter の
compile は壊さない）、`ErasedPlugin::drain_dirty_notices`（trait
member 追加 — `ErasedPlugin` を direct impl する downstream crate は
member 追加で compile 影響あり; 実 workspace では erase() 経由）。
`wire.rs` の `WIRE_VERSION` bump は L1 handshake 契約 — 同一
workspace 内の parent/child は同期して上がるため version mismatch は
発生しない（別バイナリ混在は handshake で検出済み）。

## Deferred Design Concerns

- plugin→host param 値報告の全面配線（VST3 `outputParameterChanges`、
  CLAP gesture event、params rescan/`restartComponent` 系）— Issue
  #891 対象外。`ParamEdited` は dirty 通知面の payload であって
  param report 面の置き換えではない。追跡 Issue は現状未起票
  （PR 本文の残課題に記録）。
- `dropped_edits` の consumer 側露出面（現状は queue accessor のみ —
  host 照会面の公開 shape は consumer 需要が出てから）。
- gesture（`beginEdit`/`endEdit`）粒度の通知 — 現状 edge 集約で
  十分なユースケースが出てから。
