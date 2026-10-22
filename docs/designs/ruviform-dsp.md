---
id: "DESIGN-lzqhLAShvqF3efUlehuaUS"
type: "design"
status: "approved"
title: "ruviform-dsp 設計（上位 DSP カーネル・math::dsp 層分担・prepare/process 分離）"
created: "2026-11-03"
updated: "2026-11-03"
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
    - "docs/specs/ruviform-dsp.md"
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/simd-ulp.md"
    - "docs/designs/simd-dispatch-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ruviform-dsp.md"
    - "docs/adr/dsp-simd-backend-adoption.md"
    - "docs/designs/simd-dispatch-layer.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/plans/implement-issue-75-ruviform-dsp.md"
    - "docs/test-specs/ruviform-dsp-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-dsp-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-dsp 設計

## Overview

`ruviform-math::dsp`（Primitives・型非依存 f32 スライス + `SimdToken`）と、その上に置く型付き上位カーネル層 `ruviform-dsp`（Domain foundations）の層分担を確立する。**math は「どう計算するか」（dispatch・精度契約）、dsp は「何を計算するか」（係数設計・状態管理・サンプルレート意味論）**を担い、dsp は SIMD を再 dispatch せず `SimdToken` を保持して数学カーネルへ渡すだけ。

## API 形状（prepare/process 分離）

全カーネル共通: 構築・再構成は `new`/`prepare`/`set_*`（確保あり・**非 RT**）、`process` 系は事前確保済み状態のみ使用（確保なし・**RT 適合**）。

| 型 | prepare（非 RT） | process（RT） |
| --- | --- | --- |
| `Window`（enum: Rectangular/Hann/Hamming/Blackman） | `Window::coeffs(len) -> Vec<f32>`（確保） | `Window::fill(dst)`（確保なし書込）・`Window::apply(dst)`（係数乗算 — `math::dsp` 経由） |
| `BiquadFilter` | `BiquadFilter::prepare(coeffs, channels)`（state `Vec` 2ch 分確保）・`set_coeffs`（再構成）・`BiquadDesign::{lowpass,highpass,peaking}(sample_rate, f0, q\|gain_db) -> BiquadCoeffs`（RBJ） | `process(planar: &mut [f32])` — `math::dsp::biquad` DF-II 転置・planar `[ch0][ch1]…` |
| `FirFilter` | `FirFilter::prepare(taps, max_frames)`（taps + 履歴 + スクラッチ確保） | `process(dst, src)` — スクラッチ `[delay ‖ src]` を `math::dsp::fir` に通し、末尾 `taps.len()-1` を履歴へキャリ |
| `Fft` | `Fft::new(n)`（n=2 冪・bit-rev 表 + twiddle `Vec` 確保） | `process(re, im, inverse)` — 反復 radix-2・スクラッチは prepare 時確保 |
| `LinearResampler` | `LinearResampler::new(step)`（入力/出力サンプル比 f64）・`reset` | `process(input, output) -> usize`（消費/生成を返す線形補間・`last` 履歴 1 サンプル） |

## 層分担の規則

- dsp は `SimdToken::detect()` を `prepare` 時に取得・保持し、数学カーネル呼出しに再利用（prepare 固定の dispatch 契約 — `docs/designs/simd-dispatch-layer.md` に整合）
- 精度契約は `math::dsp` 側に帰属（ulp ≤ 8 等）。dsp 層は係数設計の f64→f32 変換誤差のみ責務
- `BiquadCoeffs`/`SineCoeffs` 等の数学側値型は `math` から再 export せず、dsp の `prepare` 引数としてそのまま受け取る（依存は下向き一方向で透過）

## RT 制約

- `process` 系は `Vec` の確保/伸長を行わない: `FirFilter` は `max_frames` 上限のスクラッチを prepare 時に確保・超過入力は `Error::InvalidValue`（呼出し側がブロック分割する契約）
- `Fft::process` の bit-rev permutation は prepare 済み `rev` 表 + 確保済みスクラッチで実施
- `LinearResampler::process` は状態（分数位置・last 入力）のみ更新

## Verification

- [x] prepare/process 分離が全 5 型で一貫（spec 要件 2・3）
- [x] 層分担: dsp→math 一方向・再 dispatch なし（spec 要件 5）
- [x] RT 契約: `process` 確保なし（`rt_` テスト・`assert_no_alloc` 実測対象を test-spec に記載）
