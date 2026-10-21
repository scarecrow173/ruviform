---
id: "TASK-034JQH8r1Mfi5CItnm9Ebf"
type: "task"
status: "todo"
title: "fearless_simd 昇格 + dsp モジュール新設（SimdToken + カーネル本体）"
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

# fearless_simd 昇格 + dsp モジュール新設（SimdToken + カーネル本体）

## Work

`fearless_simd` を `[dependencies]` へ昇格し、`crates/ruviform-math/src/dsp.rs` を新設する。`SimdToken`（内部 `Sel` enum + `detect()` + `#[doc(hidden)] force_*` + `level()`）と、PoC `tests/simd_poc/fearless_impl.rs` からの `#[simd]` 8 カーネル本体の移植（`*_impl<S: Simd>`）を含む。ルーティング・公開関数は次タスク — 本タスクは「昇格が解決し内部カーネルがコンパイルする」まで。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-issue-53-simd-dispatch-layer.md
- docs/specs/simd-dispatch-layer.md
- docs/designs/simd-dispatch-layer.md
- docs/adr/simd-ulp.md

## Dependencies

- 前タスクの完了（直列依存）

## Affected Files

- crates/ruviform-math/Cargo.toml（`fearless_simd = "1.1"` を [dependencies] に追加、dev-dep 側 `force_support_fallback` + `fearless_simd_macros` 維持）
- crates/ruviform-math/src/dsp.rs（新設: SimdToken/Sel/detect/force_*/level + *_impl 8 本）
- crates/ruviform-math/src/lib.rs（`pub mod dsp`）
- Cargo.lock

## Done When

- [ ] `fearless_simd` が `[dependencies]` にあり `Cargo.lock` が更新されている
- [ ] `SimdToken::detect()` / `force_*` / `level()` がコンパイルし、内部 `*_impl` が全レベルトークンで呼べる
- [ ] 公開 API に fearless_simd 型が露出しない

## Verification

- [ ] `mise run check` / `lint` / `fmt-check` パス
- [ ] `SimdToken::detect()` が本機で `"avx2"` を返す（単体テストまたは doctest で確認）
