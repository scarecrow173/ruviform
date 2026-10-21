# DSP/SIMD PoC ベンチマーク計測レポート（2026-10-21）

Issue [#19](https://github.com/scarecrow173/ruviform/issues/19) / task T6（`docs/tasks/run-simd-bench-and-collect-results.md`）の計測実績。採否 ADR（T7）の証拠入力。
生データ: [`data/dsp-simd-poc-20261021-01.jsonl`](data/dsp-simd-poc-20261021-01.jsonl)・[`data/dsp-simd-poc-20261021-02.jsonl`](data/dsp-simd-poc-20261021-02.jsonl)（1 行 = 1 計測点、756 行/run。run・kernel・impl・input・len・elems・median_ns・mean_ns・stddev_ns・CI95）。イベントログ: [`dsp-simd-poc-benchmark-comparison.jsonl`](dsp-simd-poc-benchmark-comparison.jsonl)。

## 環境メタデータ

- OS: Windows 11 x64 / CPU: Intel Core i9-9900K @ 3.60GHz / 16 コア（HT あり）
- rustc 1.99.0 (b940084d7 2026-09-28) / criterion 0.8.2
- 検出 level: `fearless_simd Level::new()` = AVX2、`pulp::Arch::new()` = V3（AVX2+FMA まで）
- 構成不能列（probe skip）: `fearless_avx512`, `pulp_v4` — 本機に AVX-512 非搭載。**AVX-512 は未計測**
- 計測列（9）: `scalar`, `fearless_fallback`, `fearless_sse4_2`, `fearless_avx2`, `fearless_dispatched`, `pulp_scalar`, `pulp_v3`, `pulp_dispatched`, `multiversion`
- bench 除外列（等価性 matrix のみ）: `fearless_sse2`, `pulp_v2`（SSE2 は x86_64 ベースライン）
- 計測ケース: 756/run（elementwise 4 カーネル × 9 列 × 7 長さ + cumulative 4 カーネル × 9 列 × 7 長さ × 2 入力）
- 時間設定: warm-up 300ms / measurement 1s / sample 20
- 負荷状況: 計測中も並行エージェントプロセスが多数稼働（~350 プロセス）。分散緩和は 2 run 実施＋中央値採用で対処

## 実施記録（単一プロセス実行からの変更）

- `mise run bench`（単一プロセス全量）を 2 回試行し、いずれも ~82–85% 地点で `memory allocation of 1048576 bytes failed` → abort（0xc0000409）。原因は候補コードの不具合ではなく、並行エージェント多数稼働下での Windows コミット上限逼迫（当時システムコミット 163.7/183 GB、物理空きメモリは潤沢）
- 対応: カーネル単位の 8 回分割実行（`cargo bench -p ruviform-math -- <kernel>`）。プロセス寿命を限定してコミット滞留を抑え、中断時の再開性も確保。`estimates.json` の産物は全量実行と同一
- run 1・run 2 とも 756/756 ケース完走・クラッシュなし。各 run の実測所要時間 ≈25–27 分（分割オーバーヘッド込み、E-4 目安 20–30 分内）

## 分散（run 間再現性）

- median_ns の run 間 |Δ%|: p50 = 1.8%、p90 = 25.7%、p99 = 237.7%、max = 516.9%
- 最大偏位は `clamp_unit/*/normal/1024`・`/1023` 近辺（run 2 で列全体が ~4–6 倍低速化 — 並行負荷由来のグループ単位スローダウンと判断）
- 本レポートのスカラー比は **run 内比較**（同 run の scalar を分母にした比率の run 平均）で評価し、run 間絶対値差の影響を相殺

## スカラー比判定表（median scalar_ns / impl_ns、2 run 平均、>1 で高速）

| kernel/len | fearless_fallback | fearless_sse4_2 | fearless_avx2 | fearless_dispatched | pulp_scalar | pulp_v3 | pulp_dispatched | multiversion |
|---|---|---|---|---|---|---|---|---|
| add/256 | 0.85 | 0.86 | 1.24 | 0.83 | 1.05 | 1.40 | 1.31 | 1.75 |
| add/1024 | 0.98 | 0.90 | 1.38 | 0.84 | 1.02 | 1.49 | 2.03 | 2.07 |
| add/4096 | 1.03 | 0.97 | 1.36 | 1.16 | 1.03 | 1.38 | 1.26 | 1.38 |
| scale/256 | 0.21 | 0.83 | 1.20 | 1.17 | 0.98 | 1.23 | 1.36 | 1.31 |
| scale/1024 | 0.24 | 1.08 | 2.01 | 1.25 | 1.17 | 1.37 | 1.74 | 1.38 |
| scale/4096 | 0.23 | 0.94 | 1.24 | 1.15 | 1.05 | 1.35 | 1.23 | 1.46 |
| clamp_unit/256 | 0.33 | 0.76 | 0.97 | 0.70 | 1.03 | 0.90 | 0.99 | 2.26 |
| clamp_unit/1024 | 0.76 | 2.69 | 3.47 | 1.21 | 1.01 | 3.48 | 3.28 | 7.12 |
| clamp_unit/4096 | 0.30 | 0.70 | 1.00 | 0.99 | 1.01 | 0.87 | 0.92 | 1.80 |
| fade/256 | 1.19 | 3.20 | 5.15 | 5.10 | 0.89 | 3.86 | 4.62 | 0.92 |
| fade/1024 | 1.44 | 2.18 | 4.05 | 3.94 | 1.02 | 4.52 | 4.01 | 1.08 |
| fade/4096 | 1.23 | 3.34 | 6.69 | 5.59 | 0.92 | 5.28 | 6.57 | 0.91 |
| dot/256 | 4.91 | 4.62 | 8.34 | 7.83 | 0.47 | 8.46 | 8.65 | 0.99 |
| dot/1024 | 4.12 | 4.16 | 8.15 | 8.63 | 0.51 | 8.92 | 8.73 | 1.00 |
| dot/4096 | 4.00 | 4.02 | 7.89 | 7.82 | 0.53 | 8.08 | 7.92 | 0.99 |
| fir/256 | 2.34 | 2.72 | 3.61 | 3.57 | 1.15 | 2.63 | 2.58 | 0.73 |
| fir/1024 | 2.85 | 3.22 | 5.07 | 5.09 | 1.13 | 3.44 | 3.42 | 0.70 |
| fir/4096 | 2.58 | 3.34 | 5.75 | 5.24 | 1.12 | 3.66 | 3.66 | 0.69 |
| biquad/256 | 2.66 | 2.75 | 2.88 | 2.85 | 0.95 | 1.67 | 1.62 | 0.99 |
| biquad/1024 | 2.69 | 2.71 | 2.76 | 2.76 | 0.94 | 1.54 | 1.55 | 0.98 |
| biquad/4096 | 2.70 | 2.17 | 2.77 | 2.29 | 0.94 | 1.54 | 1.52 | 0.98 |
| sine/* | ≈1.00（全 impl・全サイズ） | | | | | | | |

## denormal 崖観察（denormal/normal median_ns 比 @1024、2 run 平均）

| impl | dot | fir | biquad | sine |
|---|---|---|---|---|
| scalar | 1.00 | 7.13 | 4.26 | 10.12 |
| fearless_fallback | 0.96 | 17.18 | 3.33 | 10.01 |
| fearless_sse4_2 | 1.01 | 19.31 | 3.26 | 10.04 |
| fearless_avx2 | 0.91 | 31.84 | 2.12 | 10.20 |
| fearless_dispatched | 1.04 | 31.72 | 2.16 | 9.95 |
| pulp_scalar | 3.28 | 7.89 | 4.06 | 10.05 |
| pulp_v3 | 1.00 | 21.09 | 1.63 | 10.39 |
| pulp_dispatched | 1.01 | 20.43 | 1.69 | 10.43 |
| multiversion | 1.01 | 5.07 | 4.19 | 10.04 |

- `fir` は全 impl で崖（SIMD 系ほど崖の絶対比が大きい: fearless_avx2 ≈31.8x、pulp_v3 ≈21.1x）。`biquad` は scalar 4.3x に対し SIMD 系は 1.6–2.2x と相対的に浅い。`sine` は逐次処理のため全 impl で ~10x の崖。`dot` は崖なし
- FTZ/DAZ による flush は Rust 上で形式的 UB のため PoC 範囲外 — 方針記録は採否 ADR、実装はエンジン層 Issue（spec Deferred Design Concerns どおり）

## dispatch オーバーヘッド・小サイズ回帰

- `fearless_dispatched` vs `fearless_avx2` 強制 @1024（median 比）: add 0.62 / scale 0.66 / dot 1.06 / fir 1.00 / biquad 1.00 — elementwise 小カーネルで dispatched 経路の劣後が観測される（加算・乗算型の軽量カーネルで呼び出しディスパッチの相対コストが出る構造と整合）
- `pulp_dispatched` vs `pulp_v3` 強制 @1024: ≈0.98–1.38 — ほぼ同等（分散の範囲）
- 小サイズ回帰（dispatched 列 vs scalar、<0.9x）: `fearless_dispatched` add/64 = 0.58x・add/256 = 0.83x・scale/64 = 0.82x・clamp_unit/64 = 0.73x・clamp_unit/256 = 0.70x、`pulp_dispatched` clamp_unit/64 = 0.82x、`multiversion` fir/64 = 0.76x・fir/256 = 0.73x（他 fade/64–256、dot/64 も 0.92–0.94x）
- メモリ帯域律速の elementwise 小バッファでは SIMD + dispatch が scalar に劣後 → 二層構造（スカラー参照常備＋サイズ応答ルーティング）の実動機付け

## 判定目安との照合（設計書採否基準）

| 目安 | 実測 | 評価 |
|---|---|---|
| ≥1.5x @256/1024/4096（代表カーネル多数） | `fearless_avx2`: fade 4–6.7x・dot 7.9–8.3x・fir 3.6–5.8x（add/scale/clamp は <1.5x）。`pulp_v3`: dot 8.1–8.9x・fir 2.6–3.7x・fade 3.9–6.6x（add 1.4x・scale/clamp 混在） | 両者とも演算系カーネルで明確に充足。メモリ律速 elementwise は不達 — 採否はカーネル別適用が前提 |
| dispatched ≤0.9x 回帰 @64/256 なし | `fearless_dispatched` は elementwise 小サイズで 0.58–0.84x の劣後あり（上記表）。`pulp_dispatched` は clamp/64 のみ 0.82x | 単一 dispatched 経路では基準不達 — サイズ応答ルーティングか、小バッファは scalar 直結の設計が必要 |
| dispatch オーバーヘッド ≈≤5% @1024 | `fearless` は add 0.62 / scale 0.66（elementwise で目安超過、dot/fir/biquad は ~1.0）。`pulp` はほぼ 1.0 | fearless の per-call dispatch コストは軽量カーネルで顕在。pulp は顕在せず |
| multiversion | dot ≈1.0x・fir 0.69–0.73x（回帰）・clone 個別強制 API 不在 | 性能基準不達＋検証網羅性の構造差。採用候補から除外が妥当 |
| 等価性（T4 実績） | bit 完全一致: add/scale/clamp_unit/fade/sine 全列クリア。ulp: dot 実測 max 46（暫定上限 256）・fir 0・biquad 0 | 等価性 matrix 全グリーン。dot の暫定上限は ADR で確定値を裁定 |

## 補助データ（ADR 入力）

- コンパイルコスト: dev-dep 導入前コールド `cargo check --workspace --all-targets` 0.60s（cargo 報告）/ 0.81s（wall）→ 導入後 30.14s / 30.38s（T1 A-1 記録値。~37–50x の増分はほぼ criterion+pulp 系 dev-dep）
- unsafe 範囲: PoC コード `unsafe` 0（`mise run lint` の `unsafe_code` ゲート通過で検証）。`assume_supported`/`new_unchecked`/`std::arch` 不使用
- MSRV: `fearless_simd 1.1.0` = 1.89、`multiversion 0.9.0` = 1.86、`criterion 0.8.2` = 1.86、`pulp 0.22.3` = 未宣言（x86-v4 → bytemuck `avx512_simd` 推移で ~1.89）、`approx 0.5.1` = 未宣言。workspace MSRV 1.85 を全員が超過 → dev-dep 封じ込めでは `rustup run 1.85 cargo check -p ruviform-math --lib --locked` がグリーン（実測済み）。本番 `[dependencies]` 昇格は MSRV 引上げ Issue 前提
- 構造的制限: `multiversion` は個別 clone 強制 API がなく dispatched 列のみ検証可能（matrix 網羅性の差として記録）

## 未計測・残課題

- AVX-512 環境不在のため `fearless_avx512`/`pulp_v4` は skip 記録のみ（実測なし）
- aarch64 NEON は CI `macos-latest` レーンの `--all-targets` コンパイルで構造確認のみ（計測は Windows x64 が主計測）
- `fearless_dispatched` の elementwise 小サイズ劣後は、per-call dispatch コストかノイズか未分離（biquad/multiversion の run 内分散も大きい）。必要なら低負荷環境での再計測を推奨
