---
id: "TASK-GxQyjrINNr7NrJtRcsxIBf"
type: "task"
status: "in-progress"
title: "Issue #629 AC3 Test Runtime wiring tasks"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/plans/issue-629-test-runtime-wiring.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-629-test-runtime-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-test-runtime-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-test-runtime-wiring.md"
    - "docs/specs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-629-test-runtime-wiring.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-test-runtime-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 AC3 Test Runtime wiring tasks

- [x] T1: 依存方向裁定を design に記録（逆方向 dev-dep 採用）。
- [ ] T2: `ruviform-audio-test/Cargo.toml` dev-dep 追加 +
  `Cargo.lock` 再生成。
- [ ] T3: `metadata.rs` に `RunMetadata::for_environment` 追加。
- [ ] T4: `tests/os_endpoint.rs` 実装。
- [ ] T5: device test module doc 同期（`virtual_endpoint.rs`・
  `loopback.rs`）。
- [ ] T6: 検証 matrix 実行。
- [ ] T7: self-run evidence を `evidence/rf629/` に記録。
- [ ] T8: stale design 文書 3 件 + survey Evidence 追記。
- [ ] T9: impl/ir・impl/exp(+jsonl) + review-protocol。
- [ ] T10: commit・push・PR・worker_done。
