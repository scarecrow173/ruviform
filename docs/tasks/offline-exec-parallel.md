---
id: "TASK-034JRcXokykm0XlpOqcor0"
type: "task"
status: "done"
title: "並列ワーカープール駆動（有界キュー・順序再構成）"
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
    - "docs/plans/implement-issue-24-offline-execution-driver.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-24-offline-execution-driver.md"
    - "docs/tasks/offline-exec-primitives.md"
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
    - "docs/test-specs/offline-execution-driver-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# 並列ワーカープール駆動（有界キュー・順序再構成）

## Work

- `src/offline.rs`: `OfflineDriver::run_parallel`（`workers>1`、固定 `std::thread` プール + `sync_channel` 有界キュー + `BTreeMap` 順序再構成、各ワーカーが `setup.clone()` で `prepare`）
- `prepare` 失敗集約・ワーカーパニックの `WorkerPanicked` 写像・投入ループの cancel 確認

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- <!-- prerequisite, blocker, or related task -->

## Affected Files

- <!-- file, directory, module, or interface -->

## Done When

- [x] `run_parallel` が `Setup: Clone+Send`・`Rt: Send`・`Io: Send` 境界で並列駆動
- [x] 出力順序が入力順序と一致（seq 再構成）
- [x] 有界キューがバックプレッシャーを提供（投入ブロック）

## Verification

- [x] workers ∈ {1,2,4,8} で出力が逐次と一致するテストがグリーン
- [x] prepare 失敗・ワーカーパニック・cancel のエラーパステストがグリーン
