# ruviform-dsp

`ruviform-math::dsp` の上に置く型付き DSP カーネル層（Issue #75）。
`ruviform-audio-core` / `ruviform-media-core` 型（`SampleRate`, `Result`）上で
表現したオーディオドメイン DSP カーネルを、各々 `prepare`（確保あり・非 RT）/
`process`（事前確保・RT-suitable）分離で提供する。

## 責務

- `Window` — Hann/Hamming/Blackman 係数生成
- `BiquadDesign` / `BiquadFilter` — RBJ cookbook 設計 +
  ディスパッチ済み `math::dsp::biquad` カーネル上のプレーナ多チャネルフィルタ
- `FirFilter` — `math::dsp::fir` 上のストリーミング FIR（遅延線履歴を保持）
- `Fft` — プレーナ re/im スライス上の radix-2 複素 FFT
- `LinearResampler` — ストリーミング線形補間リサンプラ

低レベル SIMD ディスパッチは `ruviform-math::dsp` に残す
（raw スライス型・呼び出し側所有 state）。本クレートはドメイン便宜性 —
係数設計・ストリーミング state・スクラッチバッファ — を所有する。
I/O・デバイスアクセス・グラフ実行はここに置かない。

## 層・許可依存

- 層: Domain foundations（設計ベースライン クレート表 A.2 — 処理・境界の段）
- 許可依存: `ruviform-audio-core`, `ruviform-media-core`（ドメイン型）+
  `ruviform-math`（Primitives カーネル）。同階層 foundations
  （`ruviform-video-proc`, `ruviform-media-io` 等）への依存は禁止。

## RT 契約

`process` 系メソッドは RT-suitable: `prepare` 時に確保したバッファと状態のみを
操作し、確保・ブロック・I/O を行わない。`prepare` 系メソッドと係数デザイナは
非 RT 専用。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2 `ruviform-dsp` 行）
- 設計（層分担規則 — dsp→math 一方向・再 dispatch なし）: docs/designs/ruviform-dsp.md
- 下位カーネル層: docs/designs/simd-dispatch-layer.md（`ruviform-math::dsp`）
- Issue #75（クレート新設）
