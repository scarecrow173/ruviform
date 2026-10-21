---
id: "ADR-034JQvAkxQaMfRbMHDRLoM"
type: "adr"
status: "accepted"
title: "本番 SIMD カーネルの精度契約形（ulp 固定 + 有限入力前提）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/53"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-math/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-math/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "dsp-simd-backend-adoption.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "dsp-simd-backend-adoption.md"
    - "public-enum-non-exhaustive-policy.md"
  refines:
    - "dsp-simd-backend-adoption.md"
  refined-by: []
  derives-from:
    - "../specs/simd-dispatch-layer.md"
    - "dsp-simd-backend-adoption.md"

  derived-by:
    - "../designs/simd-dispatch-layer.md"
    - "../plans/implement-issue-53-simd-dispatch-layer.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/ruviform-architecture-design-baseline.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# 本番 SIMD カーネルの精度契約形（ulp 固定 + 有限入力前提）

## Context and Problem Statement

採否 ADR（`dsp-simd-backend-adoption.md`）決定 3 は PoC 面の等価基準を確定した（`dot` ulp ≤256・`fir`/`biquad` ulp ≤8・elementwise/再帰一致系は bit 完全一致）。Issue #53 はこれを本番公開 API の精度契約へ具体化する。PoC の ulp 上限は「PoC 入力クラス」に基づく観測上限であり、本番 API では (a) 契約の**形**（固定 ulp 上限 / f64 真値基準 / スケール相対誤差）と (b) 契約の**前提条件**（入力域）を公開 doc で確定する必要がある。契約の形を誤ると、利用側が契約を読み違えたり、強打ち消し入力で契約自体が成立しなくなる。

## Decision Drivers

* 契約は利用側が検証可能で実装側が確実に守れる形であること（テスト可能・一義的）
* `dot` の実測が示す構造的事実: レーン分割 reduction の中間スケール差が結果スケールの数十 ulp に増幅されるが、**絶対誤差自体は ~1e-7 と小さいまま**（f64 真値照合済み・契約不適合でなく実装バグでもない）
* 強打ち消し入力では任意の固定 ulp 上限を原理的に超え得る — 契約はこの限界を前提条件で明示する必要がある
* 契約形をカーネル間で揃える単純さ vs カーネル固有の誤差構造への忠実さ
* RT 契約: 精度保証は実行時コストを持つ検証ではなく doc 契約として提供される（検証は等価性テストで静的に行う）

## Considered Options

* Option A — **固定 ulp 上限 + 有限入力前提**（PoC 確定値のそのままの本番化）: elementwise/再帰一致系は bit 完全一致、`dot` ulp ≤256・`fir`/`biquad` ulp ≤8。前提条件として「有限入力（finite f32）・NaN/Inf 取扱いは契約外」を公開 doc に明記
* Option B — **f64 真値基準**: 「f64 参照との絶対誤差 ≤ ε」形。実装上の真値との距離を直接保証する
* Option C — **スケール相対誤差**: 結果スケール対入力スケールの相対誤差形（強打ち消しに対して固定 ulp が破れる問題の形式的解決）

## Decision Outcome

Chosen option: "**Option A — 固定 ulp 上限 + 有限入力前提**", because 実測で f64 真値との絶対誤差が小さいことが検証済みであり契約の実質内容は変わらない、カーネル間で契約形が揃い利用側・テスト側ともに単純、PoC matrix の検証資産をそのまま本番面へ移行できる。

### Consequences

* Good, because 契約が `approx` ベースの既存テスト機構で直接検証可能。契約形がカーネル間で統一され、公開 doc の記述が一様。実測 max 46 ulp に対し 256 は ~5.5x マージンを持ち、未計測レーン幅のヘッドルームを含む
* Bad, because 強打ち消し入力では固定 ulp 上限を超え得る限界が残る（Option C の形式的解決を捨てた）。この限界は前提条件で明示し、超過時は契約緩和ではなく不具合/契約不適合として調査するプロトコルを維持する

### Confirmation

* 各公開カーネルの API doc に契約（bit 完全一致 or ulp 上限 + 入力前提）が明記されていること
* 本番公開 API 経路の等価性テストがスカラー参照対照でグリーンであること
* 前提条件を超える入力（NaN/Inf・強打ち消し域）への挙動は契約外と明記し、「保証が必要になった場合は契約形を切り替える」エスカレーション先を doc に残すこと

## Implementation Plan

* Affected paths: `crates/ruviform-math/src/`（公開カーネル doc）、`crates/ruviform-math/tests/`（等価性検証の本番面適用）
* Patterns to follow: PoC matrix の `contract_for` 対応表（`dot` 256・`fir`/`biquad` 8・その他 bit 完全一致）を本番 API doc の契約値として転記。`simd_poc::equiv` の NaN 両側 `is_nan` 一致規則を契約 doc に反映
* Patterns to avoid: 観測値への事後緩和、カーネルごとの異なる契約形の混在、精度検証の実行時実装（doc 契約 + テストで担保）
* Dependencies or configuration: なし（契約の文面確定のみ。検証機構は既存資産を再利用）

## Verification

* [x] 契約形が選択肢評価のうえ記録されている（本書 Considered Options / Decision Outcome）
* [ ] 8 カーネルすべての公開 API doc に精度契約と入力前提が明記されている（実装タスクで確認）
* [ ] 本番公開 API 経路の等価性テスト（スカラー参照対照・bit 完全一致/ulp 上限）がグリーン（実装タスクで確認）
* [ ] 強打ち消し・NaN/Inf を契約外として明記する doc 文言がある（実装タスクで確認）

## Pros and Cons of the Options

### Option B — f64 真値基準

* Good, because 実装上の真値との距離を直接保証するため直感的で、強打ち消しでも破れにくい
* Bad, because 「f64 参照」の定義自体が実装依存（逐次 vs 分割累積）で一意化の仕様コストが高い。実測で f64 との絶対誤差 ~1e-7 が確認済みのため追加の保証価値は小さい。テスト側は f64 参照実装を全ケースに対して持つ必要があり matrix 資産の再利用が効かない

### Option C — スケール相対誤差

* Good, because 打ち消しに対して形式上一貫し、固定 ulp の限界を原理的に回避する
* Bad, because 契約の読み取りと検証が複雑（「スケール」の定義・打ち消し近傍での挙動を利用側が理解する必要）。fir/biquad のような実測 max 0 ulp のカーネルには過剰。カーネル間で契約形が分かれると公開 doc の一貫性を損なう
