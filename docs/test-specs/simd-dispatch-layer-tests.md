---
id: "TSPEC-034JQFftbYwM1JhPvmCvI1"
type: "test-spec"
status: "approved"
title: "SIMD dispatch 層の等価性・RT 検証"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/simd-dispatch-layer.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# SIMD dispatch 層の等価性・RT 検証

## Purpose

<!-- Why this test spec exists: the intent it preserves and when it may be retired. -->
Issue #53 の本番 dispatch 層が「スカラー参照を正とする等価性」「prepare 固定トークン」「サイズ別ルーティング」「allocation-free」を恒久的に検証するための仕様。カーネル API の契約が残る限り維持する。

## Feature

本番 `ruviform_math::dsp` API が、有限 f32 入力・契約準拠の長さのもとで、スカラー参照実装と等価な結果を allocation なしに返す。

## Rules

- Rule: dispatch トークンは一度構築すれば全カーネル・全サイズで再利用可能であり、per-call の `Level::new()`/`dispatch!` を実行しない
- Rule: forced ISA 経路（fallback/sse2/sse4_2/avx2/neon — 構築可能なもの）の全カーネルは、ルーティング閾値を超えるサイズ帯を含む全サイズでスカラー参照と契約適合する（`*_impl` 直接呼出しの単体テストでルーティングのマスクを回避）
- Rule: elementwise カーネル（add/scale/clamp_unit/fade）は参照と bit 完全一致。dot ulp ≤256・fir/biquad ulp ≤8・sine bit 完全一致（`simd-ulp.md` ADR）
- Rule: ルーティング閾値未満のサイズでは公開関数がスカラー経路を選択する（閾値境界 ±1 で直接検証可能な観測点を持つ）
- Rule: 契約違反（長さ不一致・channels==0・state 不足）は debug で assert、release では panic せず共通範囲のみ処理する
- Rule: 全カーネル呼出しは allocation-free（`assert_no_alloc` 測定窓内で 0）

## Examples

- Example: `SimdToken::detect()` を AVX2 搭載機で呼ぶ → `level() == "avx2"`
- Example: `add(tok, dst, a, b)` を n=127（閾値未満）と n=128（閾値）で呼ぶ → 両方スカラー参照と bit 一致（経路選択の違いが結果に影響しない）
- Example: `biquad` を `channels=3`・`state.len()=4`（不足）で呼ぶ → debug で assert; release では `state.len()/2` チャンネル分のみ処理し panic しない
- Example: `dot(tok, a, b)` の AVX2 経路 vs スカラー参照 → ulp ≤256 内

## Guarantees

- 公開シグネチャに `fearless_simd` 型が現れない（`Level`/`Simd`/`Avx2` 等）
- 契約形がカーネル間で統一（bit 完全一致 or 固定 ulp + 有限入力前提）で公開 doc に明記
- matrix の本番列（`prod_*`）が既存 PoC 列と同じ matrix 入力・契約で評価される

## Non-goals

- 性能目標のテスト化（ベンチは smoke のみ。性能回帰検出は計測運用の責務）
- AVX-512/NEON 実機経路の検証（#55）
- denormal 入力での性能（#54。等価性のみ既存 matrix が担保）

## Risk

- ルーティング追加後に強制経路テストを怠ると、小サイズ帯で ISA カーネルの不具合が永続的に隠れる（`*_impl` 直接テストで回避）
- panic-free 化の検証漏れは RT パスでの panic リスクとして残る（release 契約テストで回避）
