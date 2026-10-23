# ruviform-audio-analysis

`ruviform-dsp` カーネルの上に置く音声解析層（Issue #92）。
PCM ブロックを計測値へ変える解析器を、各々 `prepare`（確保あり・非 RT）/
`process`（事前確保・RT-suitable）分離で提供する。

## 責務

- `LevelMeter` — 1 ブロックのチャネルごと peak/RMS スナップショット
- `SpectrumAnalyzer` — 窓掛け FFT 振幅スペクトル（`n/2+1` ビン、非正規化）
- `WaveformPeaks` — min/max 間引きによる描画用データ生成

変換 DSP は `ruviform-dsp` に残す — 本クレートは解析器の状態とスナップショット
のみを所有する。I/O・キャッシュ・RT→非 RT 受け渡しチャネルは置かない —
スナップショットの移送は利用側が既存の `ruviform-rt` 規約で行う。

## 層・許可依存

- 層: Domain foundations（設計ベースライン クレート表 A.2 — audio-core・dsp の上位段）
- 許可依存: `ruviform-audio-core`（バッファビュー）、`ruviform-dsp`
  （`Fft`/`Window` カーネル）、`ruviform-media-core`（`Error`/`Result`）。
  同階層 foundations（`ruviform-media-io`, `ruviform-video-proc` 等）への
  依存は禁止。

## RT 契約

`process` 系メソッドは RT-suitable: `prepare` 時に確保したバッファへのみ書き込み、
確保・ブロック・I/O を行わない。コンストラクタ/`prepare` と
`WaveformPeaks::peaks` は非 RT 専用（アロケーションあり）。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2 `ruviform-audio-analysis` 行）
- 設計: docs/designs/audio-analysis.md
- Issue #92（クレート新設）
