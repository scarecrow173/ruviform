---
id: "SPEC-891dirtyNoteCtrctPul1"
type: "spec"
status: "approved"
title: "Issue #891 plugin→host state dirty 通知経路 — 中立語彙・pull 配送・形式別応答契約"
created: "2026-11-23"
updated: "2026-11-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/891"
    - "docs/discovery/issue-868-host-dirty-notification.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-891-dirty-notification.md"
  implements:
    - "docs/discovery/issue-868-host-dirty-notification.md"
  implemented-by:
    - "docs/designs/issue-891-dirty-notification.md"
  depends-on:
    - "docs/specs/issue-873-plugin-gui-request.md"
    - "docs/designs/issue-873-plugin-gui-request.md"
  blocks: []
---

# Issue #891 plugin→host state dirty 通知経路 — spec

## 背景

plugin 発の state 変更通知（CLAP `clap_host_state` `mark_dirty`、VST3
`IComponentHandler2::setDirty` / `IComponentHandler::performEdit` /
`restartComponent`）が host の consumer 照会面まで届く経路が存在しない。
consumer の document-dirty/autosave 動作と supervisor の checkpoint 更新が
plugin 発端の変更を観測できない。本 spec は中立の通知語彙、pull 型の配送
seam、形式別応答方針を要件として定める。実装対応の正本は
`docs/designs/issue-891-dirty-notification.md`。

## 要件

### R1: 中立の dirty 通知語彙（AC1）

- `ruviform-plugin-core` に plugin→host の中立通知 enum `DirtyNotice` を
  追加する。`#[non_exhaustive]` 付き `Copy` 値型で、少なくとも
  `StateDirty`（再 save が必要）、`StateClean`（再同期済み — VST3
  `setDirty(false)` が生起する唯一の clean edge）、
  `ParamEdited { param_id, value }`（VST3 `performEdit` の id+normalized
  value）の 3 variant を持つ。
- dirty 通知は `Event`/`EventKind` に抱き合わせない。`Event` は
  `frame_offset` を持つ sample-accurate 契約であり、dirty 通知は
  frame offset を持たない lifecycle 隣接のフラグである。
- dirty 通知は `HostCommand`/`HostReply` に抱き合わせない（命令語彙は
  host→plugin 方向のみ）。`GuiRequest`（Issue #873）と同じ責務分割:
  中立 enum は `boundary.rs`、trait pull verb は `plugin.rs`。

### R2: pull 型の配送 seam（AC2）

- 配送は pull 型（since-last-drain）とする — `drain_output_reports` /
  `drain_gui_requests` と同型。`Plugin::drain_dirty_notices(&mut Vec<DirtyNotice>)`
  を追加し（default 空実装）、`ErasedPlugin` が中継する。
- L0 は `InProcessHost::drain_dirty_notices`（`Plain` 直接呼び +
  `Shared` pinned lock）と `PluginHost::drain_dirty_notices` composite
  照会面で届く。`OutOfProcessVideo` slot は `ReportDrain::Unavailable`
  を返す（video family に dirty surface は存在しない）。
- L1 は既存の request/reply 型 wire transport に新 verb を追加する:
  `Command::DrainDirtyNotices`（tag 28）→ `Reply::DirtyNotices`
  （tag 9、`Vec<DirtyNotice>` payload）。`WIRE_VERSION` は `6`→`7`
  に bump する。1 reply の上限 `WIRE_MAX_DIRTY_NOTICES` = 1024 で、
  超過 backlog は子側で chunk 化して連続 drain に分散する。
- 新しい asynchronous child→host transport は追加しない。

### R3: bounded・coalesced storage（AC1/R2 の補助契約）

- adapter 側の pending 格納は `DirtyNoticeQueue`（plugin-core 所蔵）を
  正本とする。dirty flag は latch（最終宣言勝ち — dirty→clean→dirty
  toggle は最終 edge のみ配送）、`ParamEdited` は上限 `MAX_DIRTY_EDITS`
  = 64 の FIFO backlog（超過は最古を drop し `dropped_edits` を
  カウント）。`ParamEdited` の enqueue は flag を dirty latch する。
- `drain` は edit backlog（古い順）→ latched flag edge の順に
  `out` へ移送する。drain 間で enqueue がなければ二度目の drain は
  空を返す。

### R4: CLAP adapter（AC1）

- `clack-host` の `HostState` extension を `BridgeHandlers::declare_extensions`
  に登録する。`MainThread<'a>` を `HostStateImpl` を満たす実 handler 型
  `BridgeMainThread` とし、`mark_dirty` を捕捉して instance 固有の
  shared queue に `DirtyNotice::StateDirty` を enqueue する。
- `mark_dirty` は `[main-thread]` 契約 — RT `process` からの発火は
  fixture 側でも行わない。

### R5: VST3 adapter（AC1/AC4）

- `BridgeComponentHandler` を `IComponentHandler` +
  `IComponentHandler2` の両 interface で実装する。
- `performEdit(id, value_normalized)` は `DirtyNotice::ParamEdited`
  として捕捉する。非有限 value は `ParamEdited` にせず
  `StateDirty` に degrade する（`ParamValue` 有限域の保存）。
- `setDirty(bool)` は `StateDirty`/`StateClean` に写像する。
- `restartComponent(kParamValuesChanged)` は `StateDirty` として受理し
  `kResultOk` を返す。それ以外の restart flag bit（`kReloadComponent`、
  `kIoChanged`、`kLatencyChanged` 等 — 単独・複合を問わず）は
  `kNotImplemented` で正直に拒否する。bridge は要求された
  param 再送・component reload を行わないため、`kResultOk` を返すと
  虚偽の約束になる。
- `beginEdit`/`endEdit` は従来どおり `kResultOk`（gesture edge は
  通知を持たない — gesture 内の `performEdit` が実通知を運ぶ）。
- `IComponentHandler2` の兄弟 verb（`requestOpenEditor`、
  `startGroupEdit`/`finishGroupEdit`）も `kNotImplemented` とする —
  `IComponentHandler2` の QI 面に載る以上、応答方針を明示する。
- `setComponentHandler` の注入は従来どおり `init` 時の一度とし、
  handler 参照は `Vst3Plugin` が保持する。

### R6: fixture 発火手段（AC3）

- test-clap `test_stateful`: state/preset restore（`apply_restored_gain`）
  が値の変更 edge で `clap_host_state.mark_dirty` を呼ぶ
  （`rescan(VALUES)` と同じ発火点）。
- test-vst3 `test_stateful`（distributed）: `ParamController` が
  `IComponentHandler` を保持し、`setComponentState` で採用した
  normalized 値を無条件に `performEdit(param_id, normalized)` で
  echo し、QI が取れた場合は `IComponentHandler2::setDirty(true)`
  を呼ぶ — `setComponentState` の呼出自体が component state
  再適用の change signal（controller の adopted mirror は
  process-path の `IParameterChanges` を見ないため、stored-value
  比較では component が実際に変化した restore を抑止する）。
  `setState`（controller-private state）は値変更 edge のみ
  `setDirty` で発火し `performEdit` echo はしない。
  `test_stateful_single` の once-contract（`handler_calls`
  counter）は変更しない — distributed 側で発火をカバーする。
- 発火 trigger は in-band（既存の state verb）とし、argv/env/
  out-of-band 機構を追加しない。

### R7: 検証 oracle（AC3/AC5）

- `DirtyNoticeQueue` の単体 test（latch coalescing・edit FIFO・
  edit→dirty latch・overflow drop+count・clean edge）。
- VST3 adapter の handler 単体 test（`performEdit`/`setDirty`/
  `restartComponent(kParamValuesChanged)` の enqueue 写像・非有限
  value degrade・非対応 flag/verb の `kNotImplemented`）。
- wire codec の round-trip・malformed/bound test（verb/reply/
  `DirtyNotice` field codec、`WIRE_MAX_DIRTY_NOTICES` 超過の
  `LimitExceeded`）。
- `PluginHost::drain_dirty_notices` の slot 別 routing test
  （L0 `Shared` slot 到達・L1 audio 未接続の `Ipc`・L1 video の
  `Unavailable`・stale handle の `InvalidHandle`）。
- CLAP fixture の `mark_dirty` 発火を観測する ABI 経由 test。
- 両 test suite の `l1_stateful_test_plugin_e2e` に
  `#[ignore]`-gated の dirty 通知 e2e を追加する — mutate →
  restore(change edge) → drain で `DirtyNotice` が consumer 照会面に
  届くことを oracle する。`mise run test-l1-real-plugin` lane が
  実行する。

## 対象外

- plugin→host の param 値報告の全面配線（VST3 `outputParameterChanges`、
  CLAP gesture event、params rescan 系）— 別 Issue 候補。
- consumer 側の document-dirty/autosave/checkpoint ポリシー実装
  （synvelis・audio-lens 側）。
- `clap_host_gui` 系 GUI 要求（Issue #873 スコープ）。
- latency/tail/note-name/track-info/audio-ports rescan 等のほかの
  host extension 通知。
