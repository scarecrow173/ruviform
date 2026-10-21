---
id: "TASK-034JPwYHzq8qU5fxLnMLlO"
type: "task"
status: "todo"
title: "低負荷再計測と生データ収集"
created: "2026-10-21"
updated: "2026-10-21"
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
    - "../plans/implement-issue-56-dispatch-analysis.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-56-dispatch-analysis.md"
    - "add-dispatch-probe-bench.md"
  blocks:
    - "write-dispatch-analysis-report.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-56-dispatch-analysis.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 低負荷再計測と生データ収集

## Work

1. 計測前に環境負荷を記録（プロセス数・CPU 負荷）
2. `cargo bench -p ruviform-math -- dispatch_probe` で per-call 素コストを計測
3. elementwise 軽量カーネルの filtered 再計測（`cargo bench -p ruviform-math -- '<kernel>'` で add/scale/clamp_unit/fade の該当列）を複数 run（≥3 または時間許容内）実施
4. 各 run の estimates/stdout を `docs/impl/exp/data/dispatched-overhead-<date>-NN.jsonl` に記録（PoC 形式踏襲）
5. run 間分散（median |Δ%| の p50/p90）を計算

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- 先行タスク: [add-dispatch-probe-bench.md](add-dispatch-probe-bench.md)

## Affected Files

- `docs/impl/exp/data/dispatched-overhead-*.jsonl`（新規・計測産物）

## Done When

- [x] プローブ値が定量されている
- [x] 軽量カーネルが複数 run で再計測されている
- [x] 生データが `docs/impl/exp/data/` に記録されている

## Verification

- [x] 各 run 完走・計測ケース数記録
