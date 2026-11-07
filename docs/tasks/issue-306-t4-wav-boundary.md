---
id: "TASK-rahQqS2k4zTTzmXeFud6L2"
type: "task"
status: "done"
title: "T4 WAV finalize→seal→readback→publish→reopen 境界試験 (Issue #306)"
created: "2026-11-07"
updated: "2026-11-07"
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
    - "docs/plans/issue-306-io-composition-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-306-t1-crate-scaffold.md"
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
    - "docs/test-specs/issue-306-io-composition-e2e.md"
  references: []
  defers: []
  deferred-by: []
---

# T4 WAV finalize→seal→readback→publish→reopen 境界試験 (Issue #306)

## Work

`tests/wav_staged.rs`（R7/R8: `WavMuxer<&mut StagedFile>` + WavEncoder
→ `finalize` receipt → `into_sink` 借用解除 → `seal` →
`open_readback` → symphonia 独立 decode PCM 照合 → publish →
公開 file 再 decode。`finalize` 未実施の best-effort のみでは
readback 検証が失敗し destination を作らない否定経路）を実装。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-306-io-composition-e2e.md`
- `docs/test-specs/issue-306-io-composition-e2e.md`（R7-R8）
