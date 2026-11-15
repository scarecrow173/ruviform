---
id: "TASK-RF628sBscripted0000004"
type: "task"
status: "done"
title: "Issue #628 タスク — scripted port-trait backend 実装"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/plans/issue-628-scripted-backend.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-628-scripted-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-scripted-backend.md"
    - "docs/plans/issue-628-scripted-backend.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-628-scripted-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-628-scripted-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-628-scripted-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 タスク — scripted port-trait backend 実装

## Task List

- [x] T1: `src/backend.rs` script 型 + `ScriptedBackend`/`ScriptedDevice`/
  `ScriptedDeviceHandle` 骨格（enumerate/default/vanish/fault 注入 API）。
- [x] T2: `src/backend/stream.rs` — `OutputStream`/`InputStream` impl、
  open 検証、manual pump、timestamp/frame_position/capture/playback、
  PCM fill、fault 適用、gap→Xrun、Drop 解放。
- [x] T3: `src/backend/midi.rs` — `MidiFeed`（`TimedMidiInput` 変換 +
  `drain_due`/`pump_into_port`）。
- [x] T4: `src/lib.rs`・crate `README.md` 更新（公開 API・正直な制限・
  test-only 境界）。
- [x] T5: `tests/backend.rs`（正ケース: enumerate/format/buffer/clock/
  PCM 再生・capture/timestamp/MIDI feed）。
- [x] T6: `tests/backend_faults.rs`（負ケース: 全 fault class・
  Expected/Observed 一致）。
- [x] T7: `tests/rt_contract.rs` 拡張（`rt_` alloc-free 証明）。
- [x] T8: `docs/impl/ir`・`docs/impl/exp`（+jsonl）・`evidence/rf628/`
  作成、docs index 再生成。
- [x] T9: 正規検証（fmt-check/check/lint/test/test-rt/prek）・
  `review-protocol`・commit/push/PR（`Refs #628`）。

## Notes

- port 契約正本: `crates/ruviform-audio-core/src/device.rs`。
  `close` API は存在せず `stop()`+`Drop` が delivery 停止境界。
- callback 発火は manual pump — `start()` は delivery を開始済みに
  するだけで firing しない。
- 依存追加なし。`unsafe` なし。公開 enum は `#[non_exhaustive]`。
