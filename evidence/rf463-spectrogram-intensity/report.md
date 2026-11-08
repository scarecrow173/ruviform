# RF463 — Spectrogram cell intensity(alpha) 実窓検証証拠レポート

Issue #463「Spectrogram cell intensity(alpha)が実窓で反映されない」の
修正について、実 GPUI ウィンドウでの before/after pixel 検証を記録する。

## 判定

| 対象 | 判定 | 根拠 |
| --- | --- | --- |
| intensity 勾配が実窓 pixel に現れる（中間 intensity → 中間色） | **Pass** | `after.png` scanline が 7 段階の合成色グラデーション。期待色 `over(viz_waveform.opacity(i), surface_bg)` と一致（後述） |
| 列 frame 区間が重なる spec（`cols > frames`、AL#179 と同形状）で alpha 累積飽和が起きない | **Pass** | `before.png` では全 cell が `#4A90D9`/`#498FD8` 系に飽和（AL#179 と同じ症状を再現）、`after.png` では stripe 毎に異なる合成色 |
| RF#461 `paint_layer` batch の性能契約維持 | **Pass** | `rvf_view_max_payload_canvas_repaint_budget` 通過（test-ui-gpui 全 61+11 件 pass） |

## 対象・環境

| key | value |
| --- | --- |
| fixture | `cargo run -p ruviform-ui-gpui --features gpui --example spectrogram_intensity`（本 Issue で追加。補助証拠: canonical = mise、example 用 mise task は未整備のため直接 cargo） |
| spec | `SpectrogramSpec`: viewport `0..7` frames, rows 128, columns 512, magnitude_range `0.0..1.0`, column `i` の intensity = `i/512`。各 frame slot を約 73 列が共有（`ruviform-viz` `Tile::columns` の `cols > frames` 契約と同形状、計 65,536 cells = `MAX_SPECTROGRAM_CELLS`） |
| 環境 | Windows 11 x64 / GPUI 0.2.2 / rustc（mise pin）/ display 実窓 |
| palette | `ViewPalette::fallback()`（`viz_waveform` = `#4A90D9`、`surface_bg` = `#14181D`） |

## 観測値（実測）

画面キャプチャは window 領域 `(40,20)` 原点 1200×640 の PNG。
canvas 内の有色 pixel を Pillow で集計した。再現手順は
`analyze.py`（`uv run --no-project --with pillow python
analyze.py <png>`）を参照。

### before（修正前 `render.rs` = HEAD `863a7a9d` 相当）

- canvas 代表色: `#4A90D9`（26,899 sample）、`#498FD8`（13,377）、
  `#478DD6`（6,757）— **AL#179 の観測値 `#4A90D9`/`#498FD8` と一致**。
- 中央 scanline: `#478DD6` → `#498FD8` → `#4A90D9` のみで、
  intensity 0.14〜1.0 の 7 stripe が全てほぼ同色に飽和。
  cell 数 73 重なりの `1-(1-a)^n` alpha 累積により、低 intensity 側も
  実効 alpha ≈ 0.99 以上となりグラデーションが消失。

### after（修正後）

- canvas 代表色（各 stripe 均等）: `#1C2938` / `#233A53` /
  `#2B4B6D` / `#335C88` / `#3A6EA3` / `#427FBE` / `#4A90D9`。
- 中央 scanline: 上記 7 色が左→右に並ぶ明確なグラデーション。
- 期待色照合: 各 stripe の勝者列 intensity は約
  `0.143 / 0.285 / 0.428 / 0.570 / 0.713 / 0.855 / 0.998`。
  `over(viz_waveform.opacity(i), surface_bg)` を sRGB 空間で計算した
  期待値と pixel が一致（例: i=0.143 → 期待 `#1C2938` = 実測 `#1C2938`）。
  中間 intensity で中間色が出ることを確認。

## 実行した検証

| コマンド | exit | 結果 |
| --- | --- | --- |
| `cargo build -p ruviform-ui-gpui --features gpui --example spectrogram_intensity` | 0 | before/after 両 binary を build（補助証拠） |
| example 実行 + `CopyFromScreen` キャプチャ | 0 | `before.png` / `after.png` 採取 |
| Pillow pixel 集計（上記スクリプト相当） | 0 | 上記観測値 |

## NotRun / Unsupported

- AL#179 実アプリ（audio-lens）上での再確認: **NotRun** — 別リポジトリの
  consumer 実行は本タスクの編集範囲外。ruviform 側 fixture で同形状 spec
  の pixel 証拠を取得済み。consumer 側の pin 追従後に AL 側で最終確認
  するのが自然な後続検証。
- `component_showcase` での確認: **NotRun** — 同 example は Spectrogram
  を含まないため、本 Issue 専用の `spectrogram_intensity` example を
  追加して代替した。

## 画面キャプチャ

- `before.png` — 修正前: canvas が `#4A90D9`/`#498FD8` 系に飽和し
  グラデーション消失（AL#179 症状の再現）。
- `after.png` — 修正後: 7 段階の intensity グラデーションが視認できる。
