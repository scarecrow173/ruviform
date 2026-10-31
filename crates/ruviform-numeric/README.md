# ruviform-numeric

Ruviform の高精度 `f64` オフライン数値・DSP プリミティブを担うクレート（Issue #150・#152・#153）。
用途非依存の FFT / Real FFT / 数値ベクトル / 窓関数 / polyphase リサンプラ /
overlap-save 畳込み / 最小二乗 / 相関カーネル / 複素 STFT / 帯域フィルタ /
スペクトル統計（Issue #152・#153）を提供する。メディア固有概念・I/O・スレッドは持たない。

## 責務

- `vec` — `f64` スライスの逐次和ベクトル演算（`dot`/`sum_sq`/`corr0`/`rms`/
  `scale`/`axpy`/`sub_scaled`/`mag_sq`）。全関数 RT-suitable。
- `fft` — `f64` 複素 FFT（`FftPlan`/`FftWorkspace`/`Fft`）と実数 FFT
  （`RealFftPlan`/`RealFftWorkspace`/`RealFft`）。plan + 呼び出し側所有
  workspace 方式で変換 API を RT-suitable にする。
- `window` — `f64` 窓係数（`Window::{Rectangular,Hann,Hamming,Blackman}`）。
- `resample` — polyphase Blackman-sinc リサンプラ（`ResamplePlan`）。
- `conv` — 実数 FFT による overlap-save 畳込み（`ConvPlan`/`ConvWorkspace`/`conv`）。
- `linalg` — Householder QR による ridge 付き最小二乗（`LeastSquares`・
  `least_squares`・`condition_number`）。
- `corr` — template-in-signal の符号付き NCC・生相互相関・GCC-PHAT
  （`CorrelationPlan`/`CorrelationWorkspace`/`NccCurve`、Full・
  NccValid・NccDirect の 3 mode）と、等長窓の両側 lag overlap 相関
  （`OverlapCorrPlan`/`OverlapCorrWorkspace`/`OverlapCurve`、Issue #152）。
  有効ラグ域のクランプ・正規化エネルギー証拠の返却を契約化する。
- `stft` — 複素 STFT（`StftPlan`/`StftWorkspace`/`Stft`/`StftMatrix`）と
  派生行列（`power`/`magnitude`/two-sided `psd`）、移動平均 `envelope`（#153）。
- `bands` — 1/3-octave 帯域エッジ（`third_octave_bands`）、帯域積分
  （`band_power`）、spectral mask 帯域フィルタ（`BandFilter`/
  `bandpass`/`bandpass_many`）、`spectral_envelope`（#153）。
- `xspec` — per-bin スペクトル統計（`csd`/`complex_coherence`/
  `transfer_h1`）と Welch 型蓄積器 `CrossSpectrum`（#153）。
- `error` — 検証エラー型 `Error`（`InvalidValue(&'static str)`、Copy）。

## 層・許可依存

- 層: Primitives（最下位、設計ベースライン クレート表 A.2 — `ruviform-math` 直後）
- 許可依存: なし（std のみ）。ruviform 内部クレートに依存しない。

## 精度・RT 契約

- 全 API は `f64` 専用。`f64` → `f32` の暗黙ダウングレードは存在しない。
- scalar 参照実装は逐次評価順を固定し FMA 融合を使わないため、全
  プラットフォームで bit-exact。
- 公開 API は panic-free。検証失敗は `Result`（`Error::InvalidValue`）、
  「妥当な入力だが解が存在しない」は `Option`（`linalg` のみ）で返す。
- RT-suitable（alloc なし・有界・事前確保済み状態でのみ動作）な API と
  non-RT（確保を伴う）API は関数単位で rustdoc に明記する。詳細は設計書
  docs/designs/issue-150-f64-offline-numeric.md の API Surface 表を参照。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2）
- 設計書: docs/designs/issue-150-f64-offline-numeric.md、
  docs/designs/issue-152-f64-correlation.md（相関カーネルの lag 域・
  正規化・証拠返却・direct/FFT ルーティング契約）
- RT 契約・検証ポリシー: docs/adr/realtime-contract-and-verification-policy.md
- Issue #150 設計: docs/designs/issue-150-f64-offline-numeric.md
- Issue #153 設計: docs/designs/issue-153-f64-stft-bands-spectral.md
- 参照実装（移植元）: AudioLens `crates/al-math`・`crates/al-analysis`（別リポジトリ・読み取り専用）
