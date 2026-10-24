---
id: "TASK-034LFHuE6Ii7Wqar445Znn"
type: "task"
status: "todo"
title: "Issue #99 色変換カーネル final verification"
created: "2026-10-25"
updated: "2026-10-25"
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
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
    - "docs/tasks/working-space-kernels.md"
    - "docs/tasks/tone-map-sdr-kernel.md"
    - "docs/tasks/icc-transform-feature.md"
    - "docs/tasks/rgba-f32-composite-affine-arms.md"
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
    - "docs/test-specs/color-working-space-kernel-tests.md"
    - "docs/test-specs/icc-transform-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #99 色変換カーネル final verification

## Work

**Full Gate（1 回のみ・plan 最終段・action 15）**: `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke`・`mise run test-video-proc-icc`・`mise run lint-video-proc-icc`・`mise exec -- prek run --all-files`・doc audit — 全 log を `docs/impl/exp/data/` に保存。`test-rt` は実一致件数で証明（0 件スキップを成功と偽らない — 0 件一致は skip-equivalent で前向き証拠にならない）。feature `icc` 列は `cargo test --workspace` が default feature のみのため `test-video-proc-icc`/`lint-video-proc-icc` の別 lane で担保する。Windows ローカル + Rust 実証のみ・mac/Linux/CI は実施時のみ記録

## Verification

- [ ] `mise run fmt-check` — exit 0
- [ ] `mise run check` — exit 0
- [ ] `mise run lint` — exit 0
- [ ] `mise run test` — exit 0
- [ ] `mise run test-rt` — exit 0 かつ **実一致件数 >0**（0 failed）
- [ ] `mise run test-video-proc-icc` — exit 0（feature-on テスト全 pass）
- [ ] `mise run lint-video-proc-icc` — exit 0
- [ ] `mise run bench-smoke` — exit 0
- [ ] `mise exec -- prek run --all-files` — exit 0
- [ ] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` — exit 0・0 findings
- **expected**: 全コマンド実実行・log 保存済み（`docs/impl/exp/data/`）。full gate は plan 最終段で 1 回のみ実施 — 実施時のみチェックを付け log パスを `changes.generated` へ追記する
