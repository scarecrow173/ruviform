# Issue #157 ruviform-viz ベンチマーク記録（2026-10-31）

Issue [#157](https://github.com/scarecrow173/ruviform/issues/157) の
受け入れ条件「大規模データの操作応答性・CPU/GPU/メモリ・キャンセルを
ベンチマークで記録する」の実測記録。対象は新設クレート
`crates/ruviform-viz` の criterion ベンチ `benches/viz.rs`。

## 環境メタデータ

- OS: Windows 11 x64 / CPU: Intel Core i9-9900K @ 3.60GHz / 16 コア（HT あり）
- rustc 1.99.0（mise pin）/ `cargo bench`（release 相当の bench プロファイル）
- 測定ハーネス: `cargo bench -p ruviform-viz`（criterion、100 samples・
  warmup 3 s）。生出力は [`data/issue-157-viz-bench.log`](data/issue-157-viz-bench.log)
- 入力データ: `ruviform_viz::demo::DemoWaveform`（決定的合成 2ch・ギャップ含む）。
  スペクトログラム系は 48000 Hz・1M フレームを `Stft` で解析し `Spectrogram`
  に複素スペクトルとして保持したものを使用

## 測定結果（criterion 中央値。括弧は 95% CI 幅）

### 波形 LOD 構築（`viz/lod_build`）

| フレーム数 | 時間 | 換算スループット |
| --- | --- | --- |
| 1,048,576（約 21.8 s @48 kHz） | 48.4 ms | 約 21.7 M frames/s |
| 4,194,304（約 87.4 s @48 kHz） | 192.0 ms | 約 21.8 M frames/s |

- 全階層の min/max/rms/count 集約を含む一括構築。フレーム数に対し線形。
- 構築は chunked で `CancellationToken` をフレーム境界ごとに観測する
  （単体テスト `build_cancels_at_chunk_boundary` でキャンセル検証済み）。

### ビューポート応答（`viz/view`、4M フレーム・ピラミッド構築済み）

| 条件 | 時間 |
| --- | --- |
| 全期間 zoom-out → 1024 bin | 14.2 µs |
| 8 s 窓 zoom-in → 1024 bin（ソース直読み） | 19.9 µs |
| 極端 zoom-in 256 サンプル → 1024 bin | 9.9 µs |

- zoom-out は LOD ピラミッドから、zoom-in はソース直読みから取得。
- いずれも結果は要求 bin 数（1024）のみに比例し、総フレーム数には
  依存しない。10–20 µs の応答は 60 fps 描画予算（16.7 ms）の
  約 0.1% で、GUI イベントループをブロックしない負荷であることを確認。

### STFT 解析（`viz/stft`、1M フレーム・hop=fft/4・単チャンネル逐次）

| FFT サイズ | 時間 | フレーム数×bin |
| --- | --- | --- |
| 1024 | 210.8 ms | 4093 フレーム × 513 bin |
| 4096 | 214.0 ms | 1023 フレーム × 2049 bin |

- `ruviform-numeric::RealFft`（sequential scalar・f64）による窓掛け FFT。
- 1M サンプル実時間約 21.8 s のデータを約 210 ms で解析（約 100× 実時間）。
- FFT サイズを 4 倍にしても総計算量はほぼ同一（フレーム数が 1/4 のため）。

### スペクトログラムタイル取得（`viz/tile`、保存済み複素スペクトルから）

| 出力列数 | 時間 | 出力形状 |
| --- | --- | --- |
| 512 列 | 18.2 ms | 512 列 × 96 行（log 周波数軸・max 集約） |
| 2048 列 | 22.6 ms | 2048 列 × 96 行 |

- `SpectrumSource::read_tile` 経由で複素スペクトルの部分読出し→
  magnitude 変換→時間方向 max 集約を含む。
- 列数 4 倍でも時間は約 1.24 倍（行集約・部分読出しの固定費支配）。
  再解析なし経路のため STFT コストは掛からない。

### バイナリ転送エンコード（`viz/encode_tile`、512×96 タイル）

| dtype | 時間 | payload サイズ | f64 比 |
| --- | --- | --- | --- |
| f64 | 283.6 µs | 393,264 B | 1.0 |
| f32 | 96.0 µs | 196,632 B | 0.50 |
| u8 | 392.1 µs | 49,176 B | 0.125 |

- RVST 形式（ヘッダ + 値列）。f32/u8 は表示量子化境界で変換。
- u8 は dB 正規化＋`DbRange` 写像を含むため f32 より遅いが、
  payload は 1/8 に圧縮される。帯域重視なら u8、CPU 重視なら f32 が有利。

## スコープ外・未計測

- **GPU**: 描画バックエンドを持たない設計のため GPU 計測対象なし
  （描画は利用側・`ruviform-gpu`/`ruviform-ui` 側の責務）。
- **メモリ**: バイト予算付き `BoundedCache` は予算超過を構造的に
  拒否する単体テストで検証。LOD ピラミッドの常駐サイズは
  `O(frames/256 × levels × channels × 8 B)` で元データの約 1.6%。
- **キャンセル**: 計測対象は構築/解析のチャンク境界観測のみ
  （`viz_acceptance.rs` で `Cancelled` 返却を検証済み）。
