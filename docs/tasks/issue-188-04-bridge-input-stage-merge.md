---
id: "TASK-54RLlHRS4QloXdmLEZEM8L"
type: "task"
status: "done"
title: "bridge: InputStage + merge + NoteLedger wiring + control/status/attach"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-188-timed-midi.md"
  implemented-by:
    - "../impl/ir/issue-188-timed-midi.md"
  depends-on:
    - "docs/tasks/issue-188-02-plugin-core-eventkind-midi-merge.md"
    - "docs/tasks/issue-188-03-audio-core-midi-input-port.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-188-timed-midi.md"
  references: []
  defers: []
  deferred-by: []
---

# bridge: InputStage + merge + NoteLedger wiring + control/status/attach

## Work

`ruviform-audio-plugin-bridge` に `input` モジュールを追加し、`PluginNode` の `CallSink` で plan path と live path を merge する。

- `input.rs`: `INPUT_RING_CAPACITY`/`INPUT_DRAIN_MAX` 定数、`InputStage`（bounded SPSC rx・`SnapshotReader<ClockAnchor>`・mapped scratch・counters）、`InputConfig`、`InputTx`（producer 側 `try_push`、full は `InputQueueFull` counter）。
- `control.rs`: `NodeControl::SetClockAnchor{anchor:ClockAnchor}`（`#[non_exhaustive]` additive）。
- `status.rs`: `NodeStatus` counters — `input_received`/`input_mapped`/`input_queue_full`/`input_unmapped`/`input_before_anchor`/`input_epoch_mismatch`/`input_ledger_overflow`/`input_merge_overflow`/`input_released`。
- `attach.rs`: `AttachConfig::input: Option<InputConfig>`、`Attached::input_tx: Option<InputTx>`。
- `node.rs`/`CallSink`: (1) `NodeControl::SetClockAnchor` 適用、(2) `InputStage::drain(block_start,frames)` → mapped live scratch（anchor 不在/domain 不一致/負 delta は counter+defined drop）、(3) transport 不連続検出→`NoteLedger::release_all` offset 0、(4) `merge_call_events(plan ++ release ++ live)` → `EventList` → plugin call、(5) `NoteLedger.feed(events)`、(6) counter/report flush。
- `lib.rs`: `pub mod input;`。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-188-timed-midi.md`
- `docs/designs/issue-188-timed-midi-detailed.md`（§5, §7）

## Dependencies

- `docs/tasks/issue-188-02-plugin-core-eventkind-midi-merge.md`（`EventKind::Midi`/`merge_call_events`）
- `docs/tasks/issue-188-03-audio-core-midi-input-port.md`（port trait・`TimedInputEvent`）

## Affected Files

- `crates/ruviform-audio-plugin-bridge/src/input.rs`（新設）
- `crates/ruviform-audio-plugin-bridge/src/control.rs`
- `crates/ruviform-audio-plugin-bridge/src/status.rs`
- `crates/ruviform-audio-plugin-bridge/src/attach.rs`
- `crates/ruviform-audio-plugin-bridge/src/node.rs`
- `crates/ruviform-audio-plugin-bridge/src/lib.rs`

## Done When

- [ ] `InputStage` が bounded ring drain→anchor map→mapped scratch を `INPUT_DRAIN_MAX` budget で処理する
- [ ] `NodeControl::SetClockAnchor` が install される
- [ ] `CallSink` が plan/live/release を `merge_call_events` で 1 つの `EventList` に統合する
- [ ] `NoteLedger` が merged events を feed し、transport 不連続で `release_all` を offset 0 で供給する
- [ ] 全 fault（queue full/unmapped/before anchor/epoch mismatch/ledger overflow/merge overflow）が counter+report で可観測
- [ ] `rt_` no-alloc テストが存在する

## Verification

- [ ] `mise run check` / `lint` / `test` / `test-rt` green
