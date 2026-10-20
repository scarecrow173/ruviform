---
id: "TASK-034Jf41t2KGVmA9hbvyDVz"
type: "task"
status: "todo"
title: "等価性 matrix テスト"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-math/tests/simd_equiv.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/dsp-simd-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/dsp-simd-poc.md"
    - "implement-simd-poc-scalar-kernels.md"
    - "implement-simd-poc-candidate-backends.md"
  blocks:
    - "write-simd-adoption-adr.md"
  supersedes: []
  superseded-by: []
  related:
    - "../specs/dsp-simd-poc-adr.md"
    - "../designs/dsp-simd-poc.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/dsp-simd-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/simd-equivalence-matrix-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# 等価性 matrix テスト

## Work

plan `PLAN-034JdOWI3vblJW11kIueY0`（[../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md)）の Phase D / Task T4。`tests/simd_equiv.rs` でカーネル × level × 入力の等価性 matrix を `mise run test` 経由で検証する。等価性契約の正本は [../test-specs/simd-equivalence-matrix-contract.md](../test-specs/simd-equivalence-matrix-contract.md)。

- [ ] D-1. `tests/simd_equiv.rs` を作成し、カーネル × 実装列（`scalar` / `fearless_fallback` / `fearless_neon`（aarch64 cfg ゲート列）/ `fearless_sse2` / `fearless_sse4_2` / `fearless_avx2` / `fearless_avx512` / `fearless_dispatched` / `pulp_scalar` / `pulp_v2` / `pulp_v3` / `pulp_v4` / `pulp_dispatched` / `multiversion`）× 入力セット（`normal` / `edge` / `denormal`）× 長さ（`{7,15,16,17,31,33}` + `{100,255,1023,1024,4096}`）× 先頭オフセット（`{0,1,3}`）の matrix をカーネルごと 1 テスト関数で集約検証（x86 限定列は cfg ゲート。aarch64 では `fearless_neon` と portable 列のみ — 設計書 matrix 節どおり）
- [ ] D-2. 構成不能な level（トークン `None`）は panic ではなく skip として記録する振る舞いを実装。カーネル契約上 NaN/Inf を扱わない edge セル（`dot` の有限値契約等）は除外条件をコードコメントで明記
- [ ] D-3. `mise run test` で matrix がグリーンであることを確認し、観測 max ulp を集計（観測値 > 8 は実装バグか契約不適合として調査し、数値を合わせ込む事後緩和はしない。`biquad` で固定上限超過が正当と判明した場合のみ設計書の代替契約形〈先頭 N フレーム ulp + 以降相対誤差 / フレーム長依存上限〉へ切替）

コミット粒度は plan 規約どおり 1 タスク 1 コミット。共有 mod 完成後は [implement-criterion-benchmarks.md](implement-criterion-benchmarks.md)（T5）と並行実施可能。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md) — `PLAN-034JdOWI3vblJW11kIueY0`（approved）。Phase D の正本
- [../test-specs/simd-equivalence-matrix-contract.md](../test-specs/simd-equivalence-matrix-contract.md) — `TSPEC-034JenWCuRngWvxWm00Ls3`。等価性 matrix の契約保証の正本（検証意図はこちらを参照）
- [../specs/dsp-simd-poc-adr.md](../specs/dsp-simd-poc-adr.md) — `SPEC-034Jci19siJZexLs53Xh5d`。等価性契約 + matrix グリーンの受け入れ基準
- [../designs/dsp-simd-poc.md](../designs/dsp-simd-poc.md) — `DESIGN-034JcqP2KiWsdes88fgvtt`（approved）。matrix 軸・等価性契約（bit 完全一致 / ulp 上限 8・代替契約形）の正本

## Dependencies

- 前提タスク: [implement-simd-poc-scalar-kernels.md](implement-simd-poc-scalar-kernels.md)（T2 — スカラー参照が比較基準）、[implement-simd-poc-candidate-backends.md](implement-simd-poc-candidate-backends.md)（T3 — 実装列・等価性判定 helper が前提）
- 後続をブロック: [write-simd-adoption-adr.md](write-simd-adoption-adr.md)（T7 — 等価性結果が採否 ADR の証拠入力）
- 観測 max ulp の集計値は採否 ADR の等価基準確定値の実測根拠となる（ADR で実測値に余裕を持たせた確定値を記録）

## Affected Files

- `crates/ruviform-math/tests/simd_equiv.rs` — 新規（等価性テストターゲット本体。`tests/simd_poc/` 共有 mod を利用）

## Done When

- [ ] `tests/simd_equiv.rs` の matrix が `mise run test` でグリーン（denormal / ±0 / NaN / Inf / 非アライン先頭 / 端数長入力を含む）
- [ ] 構成不能 level が panic ではなく skip として記録される
- [ ] 観測 max ulp が集計され、8 超過時の扱い（調査→実装バグ/契約不適合の峻別、`biquad` 代替契約形の切替判断）が記録される
- [ ] カーネル契約外の edge セル除外条件がコードコメントで明記されている

## Verification

- [ ] `mise run test`（`cargo test --workspace`）がグリーン
- [ ] 観測 max ulp の集計結果が記録されている（テスト出力または実施記録）
