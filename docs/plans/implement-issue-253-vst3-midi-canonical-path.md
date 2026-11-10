---
id: "PLAN-rf253vst3midicanonpath001"
type: "plan"
status: "approved"
title: "Issue #253 VST3 MIDI 正規経路 slice 実装計画"
created: "2026-11-12"
updated: "2026-11-12"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
    - "docs/test-specs/issue-253-vst3-midi-canonical-path.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-253-vst3-midi-canonical-path.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-vst3-midi-canonical-path.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [253]
---

# 実装計画: Issue #253 VST3 MIDI 正規経路 slice

設計: `docs/designs/issue-253-vst3-midi-canonical-path.md`。
テスト仕様: `docs/test-specs/issue-253-vst3-midi-canonical-path.md`。
タスク: `docs/tasks/issue-253-vst3-midi-canonical-path.md`。

## ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-plugin-core/src/processor.rs` | `PluginProcessor::drain_output_reports` 追加（default no-op） |
| `crates/ruviform-plugin-core/src/plugin.rs` | `ErasedPlugin` + `Erased` impl に同名メソッド追加 |
| `crates/ruviform-plugin-vst3/src/midi.rs` | `outgoing_event`/`ppq_position`/helper の ungate、`incoming_event` pub(crate) 化、`MidiAssignmentTable`/`control_assignment`/`routed_assignment` 追加、`fill_event_list` に `midi_map`、`MidiBridgeStats.param_routed` |
| `crates/ruviform-plugin-vst3/src/controller.rs` | `GainController` に `IMidiMapping` 実装（CC7→gain） |
| `crates/ruviform-plugin-vst3/src/gain.rs` | `pending_reports` + report 発行 + `drain_output_reports` |
| `crates/ruviform-plugin-vst3/src/component.rs` | `report_scratch`/`report_stats`、`emit_reports`（outputParameterChanges/outputEvents 書込） |
| `crates/ruviform-plugin-vst3/src/host.rs` | `IMidiMapping` 解決・snapshot、writable output queue/changes、output event list、`collect_output_reports`、`drain_output_reports`、`fill_param_queues` MIDI 分岐 |
| `crates/ruviform-audio-plugin-bridge/src/cap.rs` | Vst3 `param_reporting` → `Declared`、notes/test 更新 |
| `docs/impl/ir/issue-253-vst3-midi-mapping-and-reporting.md` | 新規 IR（neutral contract 判断・検証・残 NotRun） |
| `docs/impl/ir/issue-253-midi-backend-fixture-and-vst3-translation.md` | 後続 slice への参照を最小更新 |

## 順序

1. plugin-core: `drain_output_reports`（default で全既存実装を維持）。
2. midi.rs: ungate + mapping table + routed predicate + fill_event_list。
3. host.rs: mapping capture/snapshot + output channel 配線 + report drain。
4. controller.rs/gain.rs/component.rs: fixture 側 IMidiMapping + report 発行。
5. cap.rs + docs/IR。
6. 検証: `mise run fmt-check check lint test test-rt lint-l1-real-plugin test-l1-real-plugin`。

## リスク

- `ProcessData` output ptr の方向取り違え — VST3 spec 上 output 側は
  host が object を供給し plugin が書き込む向き。component の
  `data` は `&ProcessData` だが field は `*mut` で interior mutability
  経由のため `&mut data` は不要。
- RT no-alloc: 全 buffer は prepare/component 生成時に capacity 確保、
  process 内は len<capacity guard のみ。
- `ParamValue` 名衝突（vst3 f64 alias vs plugin-core 型）— host.rs は
  plugin-core 側を完全修飾で使う。
