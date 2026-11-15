---
id: "TASK-RF628pwpipeline0000004"
type: "task"
status: "done"
title: "Issue #628 タスク — pipeline wiring 実装"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-pipeline-wiring.md"
    - "docs/plans/issue-628-pipeline-wiring.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-628-pipeline-wiring.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-pipeline-wiring.md"
    - "docs/designs/issue-628-pipeline-wiring.md"
    - "docs/plans/issue-628-pipeline-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-628-pipeline-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-628-pipeline-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-628-pipeline-wiring.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 タスク — pipeline wiring 実装

## Task List

- [x] T1: `Cargo.toml` に path 依存追加（`ruviform-audio`,
      `ruviform-audio-plugin-bridge`, `ruviform-graph`,
      `ruviform-plugin-core`）— 理由コメント付き
- [x] T2: `src/pipeline.rs` + `src/pipeline/common.rs` —
      `PipelineError`、expected PCM block 組立、PcmBlock 比較 helper
- [x] T3: `src/pipeline/output.rs` — `PlaybackPipeline`
      （`Session::open` → pump → capture → oracle 照合）
- [x] T4: `src/pipeline/input.rs` — `InputCapture`（bounded handoff）+
      `InputPipeline`（typed runtime 駆動）+ `GainNode`
- [x] T5: `src/pipeline/midi.rs` — `MidiLeg` + pipeline-local
      `InputPump`（`InputTx::pump` 契約 mirror）
- [x] T6: `src/lib.rs` に `pub mod pipeline` + doc 追記
- [x] T7: `tests/pipeline_{output,input,midi,faults}.rs` —
      Expected/Observed・fault 伝播テスト
- [x] T8: `tests/rt_contract.rs` に `rt_pipeline_*` 追加
- [x] T9: `README.md` pipeline API boundary + honest limits 明記
- [x] T10: 検証実行（fmt/check/lint/test/test-rt/prek）
- [x] T11: impl/ir + impl/exp + evidence/rf628 記録
- [x] T12: review-protocol レビュー → blocker 解消
- [x] T13: docs index 再生成・origin/main merge・最終検証・commit/push/PR
