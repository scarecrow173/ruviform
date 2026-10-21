---
id: "TASK-034JQH9rC2GoOQC8WTqmpY"
type: "task"
status: "done"
title: "本番経路の検証（forced 単体・matrix 列・RT no-alloc）"
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
    - "docs/plans/implement-issue-53-simd-dispatch-layer.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-53-simd-dispatch-layer.md"
    - "docs/tasks/simd-dispatch-api-routing.md"
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
    - "docs/test-specs/simd-dispatch-layer-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# 本番経路の検証（forced 単体・matrix 列・RT no-alloc）

## Work

本番経路の検証一式: (a) `dsp` 内部単体テスト — 全 forced token × 全 8 カーネル × 複数サイズでスカラー参照対照（ルーティングを迂回して `*_impl` 直接検証）。(b) `tests/simd_equiv.rs` matrix に `prod_detected`/`prod_avx2`/`prod_sse4_2`/`prod_fallback`/`prod_neon`(probe-gated) 列追加。(c) `rt_dsp_kernels_no_alloc` — `assert_no_alloc` 測定窓で公開 API 呼出し 0 allocation（`ruviform-math` に dev-dep `assert_no_alloc` 追加）。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-issue-53-simd-dispatch-layer.md
- docs/specs/simd-dispatch-layer.md
- docs/designs/simd-dispatch-layer.md
- docs/adr/simd-ulp.md
- docs/tasks/simd-dispatch-api-routing.md

## Dependencies

- 前タスクの完了（直列依存）

## Affected Files

- crates/ruviform-math/src/dsp.rs（`#[cfg(test)] mod tests`）
- crates/ruviform-math/tests/simd_equiv.rs + tests/simd_poc/*（matrix 列追加）
- crates/ruviform-math/Cargo.toml（dev-dep `assert_no_alloc`）

## Done When

- [x] forced 全経路の契約適合テストがグリーン
- [x] matrix 本番列がグリーン
- [x] `rt_` テストが `assert_no_alloc` 窓内 0 alloc でグリーン

## Verification

- [x] `mise run test` / `test-rt` / `lint` / `fmt-check` / `bench-smoke` パス
