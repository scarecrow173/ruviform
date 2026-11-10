---
id: "TASK-rf253osmidibackend001"
type: "task"
status: "in-progress"
title: "Issue #253 OS MIDI input backend 実装タスク"
created: "2026-11-11"
updated: "2026-11-11"
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
    - "docs/plans/implement-issue-253-os-midi-backend.md"
    - "docs/designs/issue-253-os-midi-backend.md"
  implemented-by:
    - "docs/impl/ir/issue-253-os-midi-input-backend-adapter-windows-winmm-os-unsupported.md"
  depends-on:
    - "docs/specs/issue-253-os-midi-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/select-midi-input-backend.md"
    - "docs/test-specs/issue-253-os-midi-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-253-os-midi-backend.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-253-os-midi-backend.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Tasks

- [x] spec/design/ADR/plan/test-spec 作成
- [x] audio-core additive 変更（`MidiPortOrigin::Unknown`、
      `MidiReadStats.unsupported`）
- [x] `ruviform-midi-device` crate 骨格 + pure module（midi/queue）
- [x] WinMM backend 実装（enumerate/open/read/close/callback）
- [x] 非対応 OS Unsupported stub
- [x] workspace member + crates/README + crate README
- [x] 単体テスト（decode/queue/stats/stamp/結線/open 失敗）
- [x] `examples/list_midi_ports` + `#[ignore]` 実機 smoke
- [x] `mise run fmt-check check lint test test-rt` 全 Pass
- [x] review-protocol + IR doc
- [x] `git fetch origin main && git merge origin/main` +
      Cargo.lock 再生成 + 最終検証
- [ ] push + PR 作成 + worker_done
