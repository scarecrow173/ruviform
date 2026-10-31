# ruviform-viz

Ruviform の汎用インタラクティブ可視化プリミティブを担うクレート（Issue #157）。
波形 LOD・STFT スペクトログラム・タイムライン座標変換・表示側写像
（dB/周波数軸/カラーマップ/quantization）・binary transfer エンコードを
アプリケーション非依存の形で提供する。GUI/WebView/スレッドを持たず、
同じ解析結果が headless の CLI からも使える（`examples/viz_cli`）。

## 責務

- `source` — 入力 port。`WaveformSource`（時間範囲の bounded PCM 読出し）と
  `SpectrumSource`（時刻×周波数の bounded tile 読出し、複素/振幅）。
  アプリは自前ストレージの上に実装し、`viz` はビューが必要とする範囲だけを
  引く（全データの一括読込を要求しない）。`SliceWaveform`/`VecSpectrum` は
  インメモリ参照実装。
- `waveform` — `WaveformLod`（min/max/RMS/count の LOD ピラミッドを
  chunked read で構築）と `WaveformSeries::view`（極端な zoom-out は
  ピラミッド、zoom-in はソース直読みに自動切替する decimation）。
  `WaveformSeries::on_demand` はピラミッド無しのオンデマンド経路。
- `spectrogram` — `Stft::analyze`（`ruviform-numeric` f64 real FFT による
  汎用 STFT、複素 bin を `SpectrumData::Complex` として保持）と
  `Spectrogram::tile`（`TileRequest`: フレーム範囲→cols、bin 範囲→rows の
  `RowSpec` 集約で f64 magnitude tile を返す）。蓄積済み中間複素スペクトルは
  `SpectrumSource` 経由で再解析なしに同じタイル経路を使う。
- `timeline` — `SampleTimebase`（sample↔秒、`media-core` 有理数時刻で正確）、
  `FrameGrid`（window/hop↔sample）、`Viewport`（軸↔pixel、zoom/pan）、
  `LaneItem`/`assign_lanes`（複数レーン配置）、`Selection`/`items_in`/`hit_test`。
  診断意味論（ドロップアウト・変化点等）は一切持たない。
- `display` — 表示境界の写像。`DbRange`・`LinearRange`・`FreqAxis`
  （linear/log → `RowSpec`）・`ColorMap`・`Quantization`。
  保存値は常に `f64`、丸め・量子化はこの境界でのみ行う。
- `cache` — `BoundedCache`（byte 予算付き LRU）でビュー応答を再利用。
- `encode` — `PagedBytes` port（`ruviform_ui::DataSource` と同形 — `rvf://data`
  への接着はアプリ側 adapter）と `RVBW`（波形 bin）/`RVST`（タイル）の
  リトルエンディアン転送形式（`encode_buckets`/`encode_tile`）。
- `demo` — `DemoWaveform`（合成 2-tone 源、読出し時生成 — port が
  事前生成データを要求しない実例。example/bench/テストで使用）。

## 層・許可依存

- 層: 利用側層（engine/adapter と同順位）。GUI 基盤よりは下の、
  描画を持たないデータ意味論レイヤー。
- 依存: `ruviform-media-core`（正確な時刻）、`ruviform-numeric`（f64 FFT/窓）、
  `ruviform-exec`（`CancellationToken`）。`ruviform-ui` には依存しない
  （binary transfer 同形 port で疎結合）。
- dev-dependency: criterion（`benches/viz.rs`。`mise run bench-smoke` 対象）。

## 精度・RT 契約

- 内部値は全経路 `f64`。`f32`/`u8`/dB/色への変換は `display`/`encode`
  境界でのみ、呼出し側が明示する。ソースの解析精度は改変しない。
- `NaN` は統一された「欠測/無効」マーカー。統計・スペクトル・タイルを
  透過的に伝播し、補間で埋めない。
- **全 API non-RT only**。スレッド・イベントループを持たない。
  GUI 非ブロッキング要件は `ruviform-ui` の Job ワーカー等で
  これら API を呼ぶことで満たす。重い構築（LOD build・STFT）は
  chunked・progress 通知・`CancellationToken` 観測付き。

## 関連

- 設計: docs/designs/issue-157-viz-primitives.md
- 実装記録・ベンチマーク: docs/impl/ir/issue-157-viz-primitives.md
- 依存先: `ruviform-numeric`（Issue #150）、`ruviform-media-core`、
  `ruviform-exec`（Issue #21）、`ruviform-ui`（Issue #156 — adapter 側が接続）
