---
id: "IMPL-034R157VizVerif1cat10nXyZ"
type: "impl"
status: "completed"
title: "Issue #157 viz primitives implementation + verification"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/157"
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/150"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-viz"
      - type: "file"
        path: "docs/designs/issue-157-viz-primitives.md"
      - type: "file"
        path: "docs/impl/exp/issue-157-viz-bench-20261031.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/designs/ruviform-architecture-design-baseline.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-157-gate-fmt-check.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-157-gate-check.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-157-gate-lint.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-157-gate-test.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-157-gate-bench-smoke.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-157-viz-bench.log"
  related:
    - "docs/designs/issue-157-viz-primitives.md"
    - "docs/impl/exp/issue-157-viz-bench-20261031.md"
  derives-from:
    - "docs/designs/issue-157-viz-primitives.md"
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #157 可視化プリミティブ — 実装と検証記録

## Summary

`ruviform-viz`（GUI/WebView 非依存の汎用インタラクティブ可視化
プリミティブ）を新設し、波形 LOD・STFT スペクトログラム・
タイムライン座標変換・表示量子化・RVBW/RVST バイナリ転送・
バイト予算 LRU キャッシュを実装した。アプリが実装する
`WaveformSource`/`SpectrumSource` ポート経由でビューポート必要分のみ
部分読出しし、内部値は f64・表示丸めは `display`/`encode` 境界に
限定する。headless CLI example（`viz_cli`）で GUI なしの同一経路を
実証し、全ゲート pass・criterion ベンチを記録した。

## ゲート結果（全て実実行・exit 0）

| コマンド | 結果 | log |
| --- | --- | --- |
| `mise run fmt-check` | pass | `../exp/data/issue-157-gate-fmt-check.log` |
| `mise run check` | pass（headless・GUI 依存なし） | `../exp/data/issue-157-gate-check.log` |
| `mise run lint` | pass（`-D warnings`） | `../exp/data/issue-157-gate-lint.log` |
| `mise run test` | pass（`ruviform-viz` 単体 35 件・結合 `viz_acceptance` 13 件を含む全ワークスペース） | `../exp/data/issue-157-gate-test.log` |
| `mise run bench-smoke` | pass（`ruviform-math` + `ruviform-rt` + `ruviform-viz`） | `../exp/data/issue-157-gate-bench-smoke.log` |

## 受け入れ条件との対応

| 要件（Issue #157 作業内容 1–7） | 実装・検証 |
| --- | --- |
| 1. 異なるデータソースから描画できる再利用 API | `WaveformSource`/`SpectrumSource` trait + 参照実装 `SliceWaveform`/`VecSpectrum`/`DemoWaveform`/`Spectrogram`（保存済み複素スペクトル）。`viz_acceptance.rs` でカスタムソース経由の描画を検証 |
| 2. 極端 zoom・長尺・マルチch・不規則時刻・欠測・周波数分解能変更 | 結合テスト: 1M フレーム全期間→256 サンプル zoom、2ch、NaN ギャップ、不規則間隔タイル、fft_size 変更（`stft_resolution_change_keeps_f64_bins`） |
| 3. f64 保存と表示丸めの分離 | 全内部値 f64・NaN=欠測。`Quantization`/`DbRange`/`ColorMap`/`encode` のみが f32/u8 へ変換（`quantization_rounds_at_display_only`） |
| 4. ビューポート部分読出し | `view`/`tile` の結果は要求 bin/行/列数のみに比例。読出し範囲を記録する `RecordingSource` で全量ロード不要を実証 |
| 5. 大規模データの応答性・メモリ・キャンセル記録 | `docs/impl/exp/issue-157-viz-bench-20261031.md`。LOD 構築 4M フレーム 192 ms・ビュー応答 10–20 µs・`BoundedCache` 予算超過拒否・`CancellationToken` で `Cancelled` 返却 |
| 6. Rust データ意味論とフロントエンド描画の分離 | クレートは typed 表示データ + バイト列のみ出力。`ruviform-ui`・描画バックエンド・イベントループ非依存。`PagedBytes` は `ruviform_ui::DataSource` 形状（`len()+read(offset,buf)`）と一致し、利用側の薄い adapter で配線可能 |
| 7. GUI なし CLI から同一解析結果の利用 | `examples/viz_cli.rs` — LOD/zoom/STFT/タイル/dB 写像/RVST エンコード/タイムラインを headless で実行（出力はベンチ記録に添記） |

## 性能記録（要点・詳細は exp レポート）

- 波形 LOD 構築: 約 21.7 M frames/s（4M フレーム 192 ms・全階層）
- ビューポート応答: zoom-out 14 µs / zoom-in 8 s 窓 20 µs / 極端 zoom 10 µs
- STFT: 1M フレーム 210 ms（fft=1024・hop 256・約 100× 実時間）
- タイル: 512×96 log 軸タイル 18 ms（複素スペクトル再解析なし）
- エンコード 512×96: f64 284 µs/f32 96 µs/u8 392 µs、payload 1/0.5/0.125 比

## 依存分離の確認

- `ruviform-viz` の依存は `ruviform-media-core`（`TimeBase`/`MediaTime`）・
  `ruviform-numeric`（f64 Real FFT/Window）・`ruviform-exec`
  （`CancellationToken`）のみ — 全て Primitives 層。
- `ruviform-ui`・`ruviform-ui-webview`・`ruviform-audio-analysis`・
  wry/tao/wgpu への依存なし。`mise run check` 既定経路は headless のまま。
- RT 契約: 全 API 非 RT（UI スレッド/job ワーカー専用）。既存 RT API
  の契約変更なし。
- AudioLens の `DatasetReader`/`FeatureRecord`/診断意味論は持ち込まない。
- `unsafe` なし・公開 enum は `#[non_exhaustive]`。

## 未解決・既知制限（Issue 候補として切り分け）

1. **描画実装は利用側** — 本クレートは表示データとバイト列まで。
   Canvas2D/WebGL2/ネイティブへの描画と `rvf://data` 配線は
   `ruviform-ui`/利用側の責務（設計どおり・実装 Issue 別途）。
2. **GPU 計測対象なし** — 描画バックエンド非保有のため CPU/メモリのみ
   記録。AudioLens 規模の実アプリ負荷は移行時に別途計測。
3. **`SpectrumSource` のタイル粒度キャッシュ未結合** — `BoundedCache` は
   汎用提供のみで `tile()` との自動連携は利用側判断（YAGNI）。
4. **`WaveformLod` はオンデマンド再構築前提** — 永続化・増分更新は
   持たない（実行時キャッシュのみの設計合意）。
5. **macOS/Linux 未実機** — 本記録は Windows 11 のみ。実装は
   プラットフォーム非依存（std のみ）だが他 OS でのベンチ再計測は
   未実施。
