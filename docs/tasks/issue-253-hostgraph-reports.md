---
id: "TASK-034Vj6McJYc0Zg8Cae9FZg"
type: "task"
status: "completed"
title: "Issue #253 host-graph report consumer タスク（host drain API + fixture E2E）"
created: "2026-11-10"
updated: "2026-11-10"
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
    - "docs/plans/implement-issue-253-hostgraph-reports.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-253-hostgraph-reports.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-253-hostgraph-reports.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-253-hostgraph-reports.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-253-hostgraph-reports.md"
  verified-by:
    - "docs/test-specs/issue-253-hostgraph-reports.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# タスク: Issue #253 host-graph report consumer

- [x] T1 `host.rs`: `ReportDrain` enum + `InProcessHost::drain_output_reports`
      （`Plain` 直接 / `Shared` は `shared_verb`）+ reporting fixture の
      unit テスト（DR-01..05, AV-02..04）
- [x] T2 `composite.rs`: `PluginHost::drain_output_reports`（L0 委譲 /
      L1 audio・video → `Unavailable`）+ unit テスト（AV-01, AV-02）
- [x] T3 `lib.rs`: `ReportDrain` export + crate doc RT contract 追記
- [x] T4 `plugin_graph_e2e.rs`: report fixture + endpoint attach 中の
      Shared drain E2E + L1 `Unavailable` E2E（DR-04, AV-01）
- [x] T5 `docs/impl/ir/issue-253-host-graph-report-consumer-wiring.md` — API 形状・
      capability（`Declared` 維持）判断・検証・残 NotRun を記録
- [x] T6 canonical 検証 lane 実行・`git merge origin/main`・
      review-protocol・commit/push/PR

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-253-hostgraph-reports.md`
- `docs/specs/issue-253-hostgraph-reports.md`
- `docs/designs/issue-253-hostgraph-reports.md`
- `docs/test-specs/issue-253-hostgraph-reports.md`

## Dependencies

- PR #555（`641f49dd`）の `drain_output_reports` 中立契約と VST3
  emit 経路 — merge 済み。

## Affected Files

- `crates/ruviform-plugin-host/src/host.rs`
- `crates/ruviform-plugin-host/src/composite.rs`
- `crates/ruviform-plugin-host/src/lib.rs`
- `crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs`
- `docs/impl/ir/issue-253-host-graph-report-consumer-wiring.md`（新規）

## Done When

- [x] L0 `Plain`/`Shared` スロットで canonical `Event` を caller の
      `Vec` に append できる（endpoint attach 中を含む）。
- [x] L1 audio/video は `Unavailable`、stale handle は
      `InvalidHandle`、`Failed` は `Err(Failed)`。
- [x] drain は非 RT 契約で doc 明記、RT path 無変更。
- [x] fixture E2E（endpoint→graph→mock session→host drain）が pass。

## Verification

- [x] `mise run fmt-check` exits 0
- [x] `mise run check` exits 0
- [x] `mise run lint` exits 0
- [x] `mise run test` exits 0
- [x] `mise run test-rt` exits 0
- [x] `mise run lint-l1-real-plugin` exits 0
- [x] `mise run test-l1-real-plugin` exits 0
