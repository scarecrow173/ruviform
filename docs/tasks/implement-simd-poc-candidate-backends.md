---
id: "TASK-034Jf3MvCqPUi8pRT6jjES"
type: "task"
status: "done"
title: "候補バックエンド実装と共有サポート"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/mod.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/inputs.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/equiv.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/fearless_impl.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/pulp_impl.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/multiversion_impl.rs"
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
    - "add-bench-harness-and-dev-deps.md"
    - "implement-simd-poc-scalar-kernels.md"
  blocks:
    - "add-simd-equivalence-matrix-tests.md"
    - "implement-criterion-benchmarks.md"
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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 候補バックエンド実装と共有サポート

## Work

plan `PLAN-034JdOWI3vblJW11kIueY0`（[../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md)）の Phase C / Task T3。`tests/simd_poc/` 共有モジュールとして、境界入力生成・等価性判定 helper・候補 3 実装（`fearless_simd` / `pulp` / `multiversion`+autovec）を cfg ゲート方針どおりに実装する。

- [x] C-1. `tests/simd_poc/mod.rs` を作成（共有サポートの再エクスポート + matrix ランナー。`tests/simd_poc/` は cargo のテストターゲットにならない構造を利用）
- [x] C-2. `tests/simd_poc/inputs.rs` に入力生成を実装（`normal` / `edge` = denormal・±0・NaN・±Inf 混合 / `denormal` 混入。長さ `{7,15,16,17,31,33}` + `{100,255,1023,1024,4096}`、先頭オフセット `{0,1,3}`。`biquad` は `channels = 8` 固定で frames 側を同系列に変える — 設計書 methodology 節どおり）
- [x] C-3. `tests/simd_poc/equiv.rs` に等価性判定を実装（bit 完全一致 = `to_bits()` 比較、ulp 上限 = `approx::UlpsEq`、失敗報告に `(kernel, impl, level, input, len, offset, index, expected_bits, actual_bits, ulps)` を含める）
- [x] C-4. `tests/simd_poc/fearless_impl.rs` を実装（`#[simd]` 付き portable 層カーネル + `dispatch!` 経路 + `Fallback::new()` / `Level::new().as_sse2()` 等の `Option` ベース強制ラッパ。x86 限定 API 参照は `#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]` でゲート。clamp は `*_precise` 系で NaN 伝播を揃える）
- [x] C-5. `tests/simd_poc/pulp_impl.rs` を実装（`WithSimd` trait 直接 impl を基本形とするカーネル + `pulp::Scalar::new()` / `pulp::x86::V2,V3,V4::try_new()` の `Option` ベース強制 + `Arch::new().dispatch` 経路。`V2`/`V3`/`V4` 参照は cfg ゲート）
- [x] C-6. `tests/simd_poc/multiversion_impl.rs` を実装（`#[multiversion(targets("x86_64+avx2+fma","x86_64+sse4.2","aarch64+neon"))]`。本文はスカラー参照と同一ループ。個別 clone 強制 API は存在しない — matrix 制限として記録）
- [x] C-7. `mise run check`（`--all-targets` で dev ターゲット込みコンパイル）と `mise run lint` がグリーンであることを確認（`unsafe` 0・候補 API の unsafe 要求経路不使用・公開 API に候補クレートの型が露出しないことをレビュー）。pedantic lint 衝突（`cast_precision_loss` / `suboptimal_flops` / `imprecise_flops` / `wildcard_imports`）と共有 mod の dead_code は `#[expect]` / `#[allow]` + 理由コメントで対象 item に局所化する（B-2 と同方針）

コミット粒度は plan 規約どおり 1 タスク 1 コミット。C-4 / C-5 / C-6 は候補別独立ファイルのため並列実施可能。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md) — `PLAN-034JdOWI3vblJW11kIueY0`（approved）。Phase C の正本
- [../specs/dsp-simd-poc-adr.md](../specs/dsp-simd-poc-adr.md) — `SPEC-034Jci19siJZexLs53Xh5d`。成果物・受け入れ基準の正本
- [../designs/dsp-simd-poc.md](../designs/dsp-simd-poc.md) — `DESIGN-034JcqP2KiWsdes88fgvtt`（approved）。matrix 軸・cfg ゲート・等価性契約の正本

## Dependencies

- 前提タスク: [add-bench-harness-and-dev-deps.md](add-bench-harness-and-dev-deps.md)（T1 — dev-dep 解決が前提）、[implement-simd-poc-scalar-kernels.md](implement-simd-poc-scalar-kernels.md)（T2 — 共通型・参照 API が前提）
- 後続をブロック: [add-simd-equivalence-matrix-tests.md](add-simd-equivalence-matrix-tests.md)（T4）、[implement-criterion-benchmarks.md](implement-criterion-benchmarks.md)（T5）
- `unsafe` 0 方針: `assume_supported` / `new_unchecked` / `std::arch` は使わず、`Option` ベース強制のみ。`unsafe_code` lint ゲート（`mise run lint` の `-D warnings`）で担保する
- `#[path]` 共有 mod が `mise run lint` を通らない場合の fallback は plan Open Questions の独立 PoC support クレート案（設計書 Deferred Concerns と同じ据え置き）

## Affected Files

- `crates/ruviform-math/tests/simd_poc/mod.rs` — 新規（再エクスポート + matrix ランナー）
- `crates/ruviform-math/tests/simd_poc/inputs.rs` — 新規（境界入力生成）
- `crates/ruviform-math/tests/simd_poc/equiv.rs` — 新規（等価性判定）
- `crates/ruviform-math/tests/simd_poc/fearless_impl.rs` — 新規（`fearless_simd` カーネル群 + 強制ラッパ）
- `crates/ruviform-math/tests/simd_poc/pulp_impl.rs` — 新規（`pulp` カーネル群 + 強制ラッパ）
- `crates/ruviform-math/tests/simd_poc/multiversion_impl.rs` — 新規（`multiversion` カーネル群）

## Done When

- [x] 共有 mod（inputs / equiv / matrix ランナー）と候補 3 実装が `tests/simd_poc/` に存在し、全候補依存で dev ターゲットがコンパイルされる
- [x] PoC コードの `unsafe` が 0（`mise run lint` の `unsafe_code` lint ゲート通過で検証 — `grep` 全文検索は偽陽性のため採用しない）
- [x] 候補クレートの型が `#[doc(hidden)]` 含む公開 API に露出しない
- [x] x86 限定 API が cfg ゲートされ、aarch64 で portable 経路のみ参照される構造になっている

## Verification

- [x] `mise run check` がグリーン（`--all-targets` で dev ターゲット込みコンパイル）
- [x] `mise run lint` がグリーン（`unsafe_code` ゲート・pedantic lint の局所化のみ）
- [x] 公開 API レビュー: 候補クレートの型が公開 API（`#[doc(hidden)]` 含む）に露出していないこと
