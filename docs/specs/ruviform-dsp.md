---
id: "SPEC-X9YOosDAuB8PX2Qv31thuB"
type: "spec"
status: "approved"
title: "ruviform-dsp 新設（上位 DSP カーネル層 — prepare/process 分離・RT 適合）"
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
  implements: []
  implemented-by:
    - "docs/designs/ruviform-dsp.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/ruviform-dsp.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform-dsp spec

Issue #75 — ベースライン責務表の未実装クレート。Domain foundations 層の計算カーネルクレートとして `ruviform-dsp` を新設する。

## 背景

`ruviform-math::dsp`（#53・PR #66）は型非依存の低級 SIMD カーネル（`add`/`scale`/`fir`/`biquad`/`sine` 等・`SimdToken` dispatch）を提供するが、サンプルレート・チャンネル数・係数設計を扱う上位カーネル層がない。ベースライン責務表は `ruviform-dsp` にフィルタ・リサンプリング・FFT・窓関数・畳込みを割り当て、API 形状は「初期化（確保あり）と処理（確保なし）を分離」と規定する。

## 要件

1. `crates/ruviform-dsp` を新設（Domain foundations 層。許可依存: `ruviform-math`・`ruviform-audio-core`・`ruviform-media-core`。`ruviform-video-proc`/`ruviform-media-io` とは非依存）
2. **初期実装スライス**: `Window`（hann/hamming/blackman/rectangular 窓係数生成）・`BiquadFilter`（RBJ 設計式 + `math::dsp::biquad` 駆動の planar IIR）・`FirFilter`（`math::dsp::fir` 駆動のストリーミング畳込み・履歴キャリ）・`Fft`（radix-2 複素 FFT・twiddle 事前計算）・`LinearResampler`（線形補間リサンプラ）
3. **API 形状**: `prepare`/`new`（確保あり・非 RT）と `process`（事前確保済み状態のみ使用・確保なし・RT 適合）を全カーネルで分離
4. **RT 制約**: `process` 系は allocation・ブロッキングなし（`rt_` テスト + `assert_no_alloc` で実測）
5. SIMD は `math::dsp` の `SimdToken` dispatch に委譲し、dsp 層は再 dispatch しない
6. I/O・デバイス・グラフ実行は持たない（カーネルのみ）

## Acceptance Criteria

- [x] 5 カーネルが prepare/process 分離で実装され、`process` が 0 alloc（`rt_` テスト実測）
- [x] `BiquadFilter`/`FirFilter` が `math::dsp` カーネル経由で `SimdToken` dispatch を利用
- [x] `FirFilter` がコール境界を跨いだ履歴を正しく畳込む（連続入力 == 分割入力の結果一致）
- [x] `Fft` が既知入力（インパルス・正弦）で正しいスペクトルを返し逆変換で復元
- [x] `mise run` 検証列パス・doc 監査チェーン起因 findings 0・graph complete
