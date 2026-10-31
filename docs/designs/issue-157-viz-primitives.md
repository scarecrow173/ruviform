---
id: "DESIGN-034R157VizPrimitivesWk4"
type: "design"
status: "draft"
title: "Issue #157 波形・スペクトログラム・タイムラインの汎用インタラクティブ表示基盤"
created: "2026-10-31"
updated: "2026-10-31"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/157"
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/150"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  changes:
    added:
      - "crates/ruviform-viz"
    modified:
      - "Cargo.toml"
      - "crates/README.md"
      - "mise.toml"
      - "AGENTS.md"
      - "docs/designs/ruviform-architecture-design-baseline.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/designs/issue-150-f64-offline-numeric.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/audio-analysis.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/tree/main/crates/al-viz"
    - "https://github.com/scarecrow173/audio-lens/blob/main/docs/designs/dataset-schema.md"
  defers: []
  deferred-by: []
---

# Issue #157 波形・スペクトログラム・タイムラインの汎用インタラクティブ表示基盤

## Context

AudioLens は現在 `al-viz` の VisualizationStage で静的 HTML レポートと
派生 Visualization Dataset を生成しているが、AudioLens #95 の方針に従い
これを廃止し、Ruviform GUI 上で解析結果を**必要な範囲だけ読み出して
インタラクティブに描画**する構成へ移行する。Issue #157 はそのために
再利用可能な可視化プリミティブを Ruviform 側に整備する Issue であり、
本書はその設計である。

責務分割の確定契約（Issue #157・AudioLens 移行責務マップ）:

- **Ruviform**: 波形・スペクトログラム・タイムラインの**汎用処理/描画
  データ生成**。診断の意味付け・Validity/Confidence・イベント原因推定・
  画面構成は持たない。
- **AudioLens（利用側）**: データソース実装、キャッシュのキー/寿命/無効化、
  アノテーション意味論、画面構成。

`al-viz` の機械移植ではない。参照した要求は LOD ピラミッド
（min/max/RMS/個数・隣接ペア併合）、時間次元でチャンク化された
タイル、dB 変換と表示の分離、共有 timebase 宣言（SeriesDecl）、
範囲読出しにより全量をメモリに持たない構成である。

前提基盤:

- Issue #156 の `ruviform-ui` — `Service`/`Job` モデル、
  `DataRegistry`/`DataSource` の bounded binary pull、GUI/CLI 同一駆動。
  **本クレートは `ruviform-ui` に依存しない**（分離可能な port 要件）。
- Issue #150 の `ruviform-numeric` — `f64` Real FFT / 窓関数
  （STFT の数値カーネル）。Issue #153（STFT/帯域演算）は並行実施中の
  ため、その完成形に依存しない。
- `ruviform-exec` の `CancellationToken`/`Progress` — キャンセル可能な
  データロード契約。

## Scope

### In Scope

- `ruviform-viz` クレート新設（利用側層・engine と同順位）。
- 波形プリミティブ: 事前計算 LOD ピラミッドとオンデマンド decimation の
  両経路。`min`/`max`/`rms`/有限サンプル数を持つ bucket、欠測（NaN）
  の表現、zoom/pan/channel クエリ、極端 zoom-in 時のソース直読み。
- スペクトログラムプリミティブ: `ruviform-numeric` の Real FFT による
  汎用 STFT（窓・hop・fft_size 変更可能）、アプリ提供の保存済み
  （複素）スペクトルを再解析なしに消費する `SpectrumSource` 経路、
  時間×周波数タイルの decimate 取得。
- 表示写像の分離: dB レンジ・カラーマップ・周波数軸（linear/log）・
  転送量子化は、数値データ（線形 magnitude/power・f64）とは別 API。
- タイムラインプリミティブ: sample/frame/seconds の座標変換
  （`SampleTimebase`・`FrameGrid`）、複数レーンへの貪欲割当て、
  範囲選択・ヒットテスト、payload 非依存のイベント primitive。
- データアクセス port: アプリが実装する chunked/random-access な
  `WaveformSource`/`SpectrumSource` trait。AudioLens の
  `DatasetReader`/`FeatureRecord` は持ち込まない。
- 負荷管理: `CancellationToken` によるキャンセル、`Progress` 報告、
  バイト予算付き LRU キャッシュ、bounded クエリ（結果はビューポート
  サイズで上限化）、転送量子化選択（`f64`/`f32`/`u8`）。
- 検証: 極端 zoom・長尺・マルチチャンネル・欠測・不規則時刻・
  周波数分解能変更の単体/結合テスト、部分読出しの実証テスト、
  headless CLI example、criterion ベンチ（応答性・スループット・
  キャンセル・転送サイズ）。

### Out of Scope

- AudioLens 独自の `VisualizationStage`、al-viz JSON/テンソル形式、
  静的 `report.html`、ダッシュボードレイアウト、診断意味論
  （変化点/ドロップアウト/Validity/Confidence/原因推定）。
- `ruviform-gpu` / wgpu 実装、フロントエンド実装（描画側）。
  本クレートが出力するのは typed 表示データとバイト列であり、
  Canvas2D/WebGL2/ネイティブへの描画は利用側の責務。
- `rvf://data` への登録・`Service`/`Job` 配線（`ruviform-ui` 側の責務。
  本書は配線方法を文書化するが実装は利用側）。
- Issue #153 完成形の STFT/帯域演算 API への追従（本設計は
  `ruviform-numeric` のみに依存し、#153 の到着を待たない）。
- 永続的可視化 dataset（実行時キャッシュのみ。移行責務マップの
  「永続物として保存しない」契約）。

## Components and Boundaries

### クレート構成と依存 DAG

```text
利用側アプリ（al-gui / CLI 等。本リポジトリ外）
    │ implements                  ┌─ 別クレート（依存なし・同層）──┐
    ▼                             │ ruviform-ui（GUI 配線のみ）  │
ruviform-viz（利用側層）           └──────────────────────────────┘
    │
    ├── ruviform-numeric（f64 Real FFT / Window。Primitives）
    ├── ruviform-exec（CancellationToken / Progress。Primitives）
    └── ruviform-media-core（Error / TimeBase。Primitives）
```

- `ruviform-viz` は **GUI・WebView・描画バックエンド・`ruviform-ui`
  系に一切依存しない**。headless CLI と GUI の両方から同じ API で
  使えることが「`ruviform-ui` 系と分離可能な port」要件の解釈である。
- `ruviform-ui` との統合は `DataSource` 形状（`len()` + `read(offset,
  buf)`）と `encode` モジュールの `PagedBytes` が一致していることで
  利用側の薄い adapter（`DataSource` impl 6 行程度）として成立する。
- `ruviform-audio-analysis`（f32・RT 向け解析）には依存しない。
  viz の値は f64 で、RT 解析とは別精度階層である。
- RT 契約: **全 API 非 RT**。UI スレッド/job ワーカー専用であり、
  RT スレッドからの呼出しは契約違反。

### モジュール責務

| モジュール | 責務 |
| --- | --- |
| `error` | `VizError`（`Invalid(&'static str)`/`Source(String)`/`Cancelled`/`Unavailable(&'static str)`）と `Result` alias。ソース由来失敗は文字列を畳める形にする（`ruviform-ui` の `DataSource::read -> Result<usize, String>` と同じ発想） |
| `source` | 入力 port。`WaveformSource`（`info()` + 範囲読 `read(start, frames) -> PcmBlock`、interleaved f64、NaN=欠測）、`SpectrumSource`（`info()` + `read_tile(frames, bins) -> SpectrumTile`、複素または magnitude）。参照実装 `SliceWaveform`/`VecSpectrum` |
| `timeline` | `SampleTimebase`（`u32` Hz の sample↔seconds。秒は表示用 f64、内部変換は整数有理数）、`FrameGrid`（window/hop/offset の frame↔sample）、`Viewport`（時間範囲×画素、px↔時間写像、samples_per_px）、`LaneItem<T>`/`assign_lanes`（重複時刻の貪欲レーン分割）、`Selection`/`items_in`/`hit_test` |
| `waveform` | `LodBucket { min, max, rms, count }`（count=有限サンプル数、空 bucket は NaN）、`WaveformLod`（chunked 範囲読から構築、隣接ペア併合で上位レベル）、`WaveformSeries::view(range, bins)`（最適レベル選択＋クエリ側併合、spp < base_bucket 時はソース直読み）、`LodConfig` |
| `spectrogram` | `StftConfig { fft_size, hop, window }`、`Stft`（WaveformSource → 複素スペクトル。NaN 含む frame は NaN 行）、`Spectrogram`（`from_stft`/`from_source` 二経路、bin index domain の `tile(TileRequest)` で時間×周波数 decimate）、`Aggregate::{Max,Mean}` |
| `display` | 表示写像のみ。`DbRange`/`to_db`（amp/power）、`FreqAxis::{Linear,Log}`（Hz↔bin/行写像）、`ColorMap`（LUT＋補間）、`Quantization::{F64,F32,U8}` |
| `cache` | `BoundedCache<K,V>`（バイト予算付き LRU。generation tag 付き VecDeque で O(1) 追い出し） |
| `encode` | バイナリ転送。`encode_lod`（`[f64le min,max,rms,u64le count]` AoS。量子化で f32/u8 版も）、`encode_tile`、`PagedBytes` port（`len()`+`read(offset,buf)` の DataSource 形状。lazy 範囲読ソースをそのまま登録できる） |
| `demo` | `DemoWaveform` — 決定的合成ソース（多チャンネル・ギャップ・長尺対応）。テストと example の共有 fixture |

### 主要 API スケッチ

```rust
// 入力 port（利用側が実装）
trait WaveformSource: Send + Sync {
    fn info(&self) -> WaveformInfo;            // rate, channels, frames
    fn read(&self, start: u64, frames: usize) -> Result<PcmBlock>;
}
trait SpectrumSource: Send + Sync {
    fn info(&self) -> SpectrumInfo;            // frames, bins, FrameGrid, bin_hz
    fn read_tile(&self, frames: Range<u64>, bins: Range<usize>)
        -> Result<SpectrumTile>;
}

// 波形: ビューポート駆動の部分読出し
let series = WaveformSeries::with_lod(&source, &LodConfig::default(), &cancel)?;
let bins = series.view(0, SampleRange::new(0, frames), /* px */ 1024)?;

// スペクトログラム: STFT 経路と保存スペクトル経路で同じ Spectrogram
let gram = Spectrogram::from_stft(&stft, &source, /* ch */ 0, &cancel, &prog)?;
let gram = Spectrogram::from_source(&spectrum_source)?;   // 再解析なし
let tile = gram.tile(TileRequest { frames, bins, cols, rows, agg })?;
let px = display::quantize(&tile, &DbRange::new(-90., 0.), Quantization::U8);
```

### 精度契約

- 内部表現は **`f64` 専用**。ソースが f64 で返した値を丸め・縮退しない。
- 表示丸めは `display`/`encode` の変換時にのみ発生し、選択は呼出側が
  明示する（`Quantization::{F64,F32,U8}`・`DbRange`・`ColorMap`）。
- NaN は「欠測/無効」の一貫した表現。bucket 集約・STFT・タイル・
  encode の各段で NaN を伝播し、0 や隣接値への補完は行わない。
- RMS は `sum_sq` を上位レベルへ正確に伝播して再計算する
  （レベル間で RMS の再近似をしない）。

### 負荷・転送契約

- `WaveformSeries::view` / `Spectrogram::tile` の結果サイズは要求した
  画素数で上限化される（`bins`/`cols`/`rows`）。ソースへの読出しも
  要求範囲のみで、全量読込を要求しない。
- 重い構築（LOD build・STFT）は `CancellationToken` をチャンク境界で
  観測し `VizError::Cancelled` で打ち切る。`Progress` で進捗報告。
- `BoundedCache` は利用側がビューポート結果・タイルを入れるための
  バイト予算制 LRU。キー設計・無効化・寿命は利用側の責務。
- バルク転送は `encode` + `PagedBytes` — `rvf://data` 配線時は JSON に
  乗せない（#156 契約を再利用）。量子化は転送選択であり内部値を変えない。

## Non-Functional Constraints

- `unsafe` 不使用。全公開 enum は `#[non_exhaustive]`。
- 依存追加理由を manifest コメントに記録。新規外部依存は原則追加しない
  （`criterion`/`assert_no_alloc` は dev-dep のみ・既存パターン踏襲）。
- headless 既定: 本クレートは既定 `cargo check`/`test` で GUI 依存を
  引き込まない（GUI 依存を持たないので当然維持される）。
- 公開 API は panic-free を目指し、検証失敗は `Result`/`Option` で返す
  （既存クレート契約に揃える）。

## Verification

- 単体: LOD 併合の参照実装一致・レベル選択・部分 bin・マルチ
  チャンネル・NaN bucket、STFT 既知トーンの bin・hop/window 変更・
  NaN 伝播、タイル Max/Mean decimate、dB/log 軸写像、時刻変換の
  整数厳密性、レーン割当て・選択・ヒット、キャッシュ追い出し順、
  encode レイアウト golden、PagedBytes 境界。
- 部分読出し実証: 読出し範囲を記録する計数ソースで、`view`/`tile` が
  要求範囲のみ読むこと（全量読込不要）を検証。長尺（疑似 1 時間級）
  でもクエリ時間がビューポートサイズに比例すること。
- 欠測・不規則時刻: NaN 区間の bucket/タイル、sample 非等間隔
  （frame grid offset）・イベントの不規則時刻。
- 極端 zoom: spp < base_bucket でソース直読み経路、zoom-out で最上位
  レベル、周波数分解能の変更（fft_size 差し替え）。
- 複素再利用: `SpectrumSource` 複素値 → `from_source` が
  `from_stft` と同値を返すこと（再解析なし経路）。
- キャンセル: 事前 cancel・途中 cancel の打ち切り。
- CLI 利用例: `examples/viz_cli` で合成ソース → LOD → STFT →
  タイル → encode の一連を headless 実行（GUI 非依存の証跡）。
- ベンチ: LOD build スループット、zoom 別クエリ応答、STFT
  スループット、タイル/encode、キャンセル応答。結果を impl 記録へ。

## Deferred Design Concerns

- `ruviform-ui` への `PagedBytes`/`VizService` ブリッジ（利用側
  adapter で足りるか実運用で確認。必要なら feature 分離で追加）。
- `SpectrumSource` の位相/複素値を活かす表示（群遅延・再合成）。
- wgpu ネイティブ面への直接タイル転送（`ruviform-gpu` 到着後）。
- #153 完成形との API 整合・乗換え検討。
- 対数軸の mel/ERB 等の知覚スケール追加（要求が出てから）。
