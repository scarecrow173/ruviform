---
id: "TASK-034UoZXLONDKdThLicpKCD"
type: "task"
status: "todo"
title: "Implement Issue #290 OBS-08 allocation-probe — CountingAllocator/AllocationScope"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
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
    - "docs/plans/implement-issue-290-observability-allocation-probe.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-290-observability-allocation-probe.md"
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
    - "docs/test-specs/issue-290-observability-allocation-probe-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement Issue #290 OBS-08 allocation-probe — CountingAllocator/AllocationScope

## Work

`crates/ruviform-observability` に `allocation-probe` feature で gate
された `profiling` モジュールを実装する。`CountingAllocator<A>`
（`GlobalAlloc` 4 API を内側 `A` へ委譲 + 飽和計数）と
`AllocationScope`/`AllocationWindow`（TLS 計測ウィンドウ）を含む。
設計は `docs/designs/issue-290-observability-allocation-probe.md`、
検証意図は `docs/test-specs/issue-290-observability-allocation-probe-tests.md`
を正本とする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-290-observability-allocation-probe.md`
- `docs/specs/issue-290-observability-allocation-probe.md`
- `docs/designs/issue-290-observability-allocation-probe.md`
- `docs/test-specs/issue-290-observability-allocation-probe-tests.md`

## Dependencies

- `docs/plans/implement-issue-290-observability-allocation-probe.md`
  （approved plan）

## Affected Files

- `crates/ruviform-observability/Cargo.toml` — `allocation-probe` feature
- `crates/ruviform-observability/src/lib.rs` — module gate + doc bullet
- `crates/ruviform-observability/src/profiling/{mod,stats,allocator,scope}.rs` — 新規

## Done When

- [ ] `CountingAllocator` が 4 `GlobalAlloc` API を全て内側 `A` へ
  委譲し、成功/失敗/`alloc_zeroed`/realloc（拡大・縮小・失敗時旧
  ブロック生存）/align の試験が通る
- [ ] `AllocationStats`/`OpStats` が要求/成功/失敗/バイト数と
  `current_bytes`/`peak_bytes` を正しく報告する（飽和カウンタ）
- [ ] `AllocationScope` のネスト・同時スレッド・スレッド終了・
  非 LIFO drop・再 enter 拒否・`mem::forget` 耐性が試験される
- [ ] allocator 経路と window enter/exit が no-alloc/no-panic である
  ことが `assert_no_alloc`（mock inner 使用）で検証される
- [ ] `alloc_counter`/`play_sine` 共存判断・オーバーヘッド・
  CPU sampling/Tracy 制約が rustdoc/設計文書に記録される
- [ ] `cargo test -p ruviform-observability --features allocation-probe`
  が pass し、`mise run fmt-check`/`check`/`lint`/`test` が pass する

## Verification

- [ ] `cargo test -p ruviform-observability --features allocation-probe` exits 0
- [ ] `cargo clippy -p ruviform-observability --features allocation-probe --all-targets -- -D warnings` exits 0
- [ ] `mise run fmt-check && mise run check && mise run lint && mise run test` exits 0
- [ ] `cargo tree -p ruviform-observability --features allocation-probe -e normal` shows no new external dependencies
