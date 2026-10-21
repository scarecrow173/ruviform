---
id: "TASK-034JRcWTeJbPQAh5fUwPZs"
type: "task"
status: "done"
title: "オフライン駆動プリミティブ（CancellationToken/Progress/OfflineError/逐次ドライバ）"
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

# オフライン駆動プリミティブ（CancellationToken/Progress/OfflineError/逐次ドライバ）

## Work

- `crates/ruviform-exec/Cargo.toml`: `ruviform-rt = { path = "../ruviform-rt" }` 追加
- `src/offline.rs`: `CancellationToken`・`Progress`・`OfflineConfig`・`OfflineError`（`#[non_exhaustive]`）・`OfflineDriver::run`（逐次、Send 境界なし）
- `src/lib.rs`: `mod offline` + 公開再エクスポート + crate doc に責務追記

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- <!-- prerequisite, blocker, or related task -->

## Affected Files

- <!-- file, directory, module, or interface -->

## Done When

- [x] `OfflineDriver::run` が `Processor`/`Setup`/`Vec<Io>` を受け逐次駆動し `Vec<Io>` を返す
- [x] `CancellationToken`/`Progress`/`OfflineError` が公開 API としてコンパイル・doc 済み（`**RT contract:** non-RT only`）
- [x] 全公開 enum が `#[non_exhaustive]`

## Verification

- [x] `mise run check`/`lint`/`fmt-check` パス
- [x] 逐次経路の基本テスト（順序保存・cancel 即時停止）がグリーン
