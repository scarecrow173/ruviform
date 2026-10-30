# crates/

Ruviform ワークスペースのメンバークレート索引（計23クレート）。
各クレートの責務・許可依存層・RT 制約・実装 Issue の詳細は各クレートの
`README.md` と crate-level doc（`src/lib.rs`）を参照。
層構造と `path` 依存方向・クレート DAG の正本は
[docs/designs/ruviform-architecture-design-baseline.md](../docs/designs/ruviform-architecture-design-baseline.md)
（Issue #1 由来・approved）。

## Primitives（最下位・相互依存なし）

| クレート | 責務 |
| --- | --- |
| [`ruviform-math`](ruviform-math/README.md) | SIMD dispatch カーネル層（`math::dsp`） |
| [`ruviform-numeric`](ruviform-numeric/README.md) | 高精度 `f64` オフライン数値・DSP プリミティブ（`fft`/`window`/`vec`/`resample`/`conv`/`linalg`、Issue #150） |
| [`ruviform-rt`](ruviform-rt/README.md) | RT 基盤（`Processor`/`RtProcessor`・`RtContext`・`MasterClock`・SPSC・retire channel・スナップショット交換） |
| [`ruviform-media-core`](ruviform-media-core/README.md) | 規格中立メディアデータモデル（時間・transport・音楽時間・`Error`） |
| [`ruviform-exec`](ruviform-exec/README.md) | 非 RT 実行基盤（スレッド優先度機構・`OfflineDriver`） |
| [`ruviform-observability`](ruviform-observability/README.md) | 共有観測プリミティブ（`Counter`/`CounterReader` counter pair — Issue #101 候補・#121 で L1 diagnostics が初消費者） |

## Domain foundations

| クレート | 責務 |
| --- | --- |
| [`ruviform-audio-core`](ruviform-audio-core/README.md) | オーディオデータモデル（`SampleRate`・planar buffer view・device ポート trait） |
| [`ruviform-video-core`](ruviform-video-core/README.md) | ビデオデータモデル（フレーム・ピクセル形式） |
| [`ruviform-video-proc`](ruviform-video-proc/README.md) | ビデオ処理カーネル（色変換・`RgbaF32` linear working space 色管理・コンポジット/遷移・feature `icc` で ICC 変換） |
| [`ruviform-dsp`](ruviform-dsp/README.md) | ドメイン型付き audio DSP カーネル（`Window`・`Biquad`・`FirFilter`・`Fft`・`LinearResampler`） |
| [`ruviform-audio-analysis`](ruviform-audio-analysis/README.md) | 解析層（`LevelMeter`・`SpectrumAnalyzer`・`WaveformPeaks`） |

## メディア I/O（非 RT 専用）

| クレート | 責務 |
| --- | --- |
| [`ruviform-media-io`](ruviform-media-io/README.md) | メディア I/O 抽象境界（demux/mux・decode/encode の4系統 trait） |
| [`ruviform-io-symphonia`](ruviform-io-symphonia/README.md) | symphonia バックエンド PoC（demux + decode のみ） |

## Optional platform adapters

| クレート | 責務 |
| --- | --- |
| [`ruviform-audio-device`](ruviform-audio-device/README.md) | cpal アダプタ（`audio-core` device ポートの `Box<dyn>` 実装） |

## 拡張層（Plugin）

| クレート | 責務 |
| --- | --- |
| [`ruviform-plugin-core`](ruviform-plugin-core/README.md) | 規格中立プラグイン基盤（音声 RT・動画 NONRT 契約、typed plain 制御） |
| [`ruviform-plugin-ofx`](ruviform-plugin-ofx/README.md) | OpenFX Image Effect ホストアダプタ（実 foreign ABI・NONRT L0） |
| [`ruviform-plugin-clap`](ruviform-plugin-clap/README.md) | CLAP 薄アダプタ + feature `host` で規格 host-side bridge・L1 runner bin（Issue #114・opt-in lane `mise run test-l1-real-plugin`） |
| [`ruviform-plugin-vst3`](ruviform-plugin-vst3/README.md) | VST3 アダプタ（ABI-only）+ feature `host` で規格 host-side bridge・L1 runner bin（Issue #114・opt-in lane `mise run test-l1-real-plugin`） |
| [`ruviform-plugin-export`](ruviform-plugin-export/README.md) | プラグイン側 L1 薄ランナー（子プロセスエントリ `run(factories)`/`run_video(factories)` + 参照 runner bin `ruviform-plugin-l1-runner`/`-l1-video-runner`） |

## Host 層

| クレート | 責務 |
| --- | --- |
| [`ruviform-plugin-host`](ruviform-plugin-host/README.md) | 規格中立プラグインホスト境界（L0 インプロセス + L1 アウトプロセス — `PluginHost` コンポジットが登録時ルーティング・`PluginHost::video()` が NONRT video L1 別面を貸す — Issue #118） |

## Engines

| クレート | 責務 |
| --- | --- |
| [`ruviform-audio`](ruviform-audio/README.md) | オーディオエンジン（pull 型グラフ・`Mixer`・`GraphPlan`/`PlanSwap`・`Session`） |
| [`ruviform-video`](ruviform-video/README.md) | ビデオエンジン（`FramePipeline`・A/V sync・seek 仲介） |

## 利用側層（engine と同順位・engine ではない）

| クレート | 責務 |
| --- | --- |
| [`ruviform-edit`](ruviform-edit/README.md) | NLE 非破壊編集データモデル + 評価射影（全 API 非 RT） |
