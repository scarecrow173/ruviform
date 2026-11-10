---
id: "TASK-rf253vst3midicanonpath001"
type: "task"
status: "completed"
title: "Issue #253 VST3 MIDI 正規経路 slice タスク（IMidiMapping + plugin→host reporting）"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-253-vst3-midi-canonical-path.md"
    - "docs/test-specs/issue-253-vst3-midi-canonical-path.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-253-vst3-midi-canonical-path.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-253-vst3-midi-canonical-path.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# タスク: Issue #253 VST3 MIDI 正規経路 slice

- [x] T1 plugin-core `drain_output_reports` contract（processor.rs/plugin.rs）
- [x] T2 midi.rs: ungate + `MidiAssignmentTable`/`control_assignment`/`routed_assignment` + `fill_event_list` midi_map + stats
- [x] T3 host.rs: IMidiMapping 解決/snapshot、writable output queue/changes、output event list、report drain、fill_param_queues MIDI 分岐
- [x] T4 fixture: controller IMidiMapping、gain.rs reports、component.rs emit_reports
- [x] T5 cap.rs `param_reporting` → Declared + test 更新
- [x] T6 E2E/unit tests（MM-01..05, PR-01..05）
- [x] T7 IR + 既存 IR 更新（neutral contract 判断記録）
- [x] T8 canonical 検証 lane 実行・merge origin/main・review・commit/push/PR
