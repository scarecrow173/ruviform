# Issue #152 ruviform-numeric::corr ↔ AudioLens al-math::correlation 比較レポート（2026-10-31）

Issue [#152](https://github.com/scarecrow173/ruviform/issues/152) の受け入れ条件「`al-math` と同じ形状の実入力に対し correctness・busy time・wall time・memory を比較する記録」の実測記録。
比較相手は **AudioLens Issue #98 凍結 baseline**（`al98-baseline` worktree）の `crates/al-math`。`correlation.rs` および `vec.rs` は現行 `F:/repositry/audio-products/audio-lens` main と `diff -q` で byte-identical を確認済みのため、本記録は「現行カーネル」と「#98 baseline」の双方の突合を兼ねる。

## 環境メタデータ

- OS: Windows 11 x64 / CPU: Intel Core i9-9900K @ 3.60GHz / 16 コア（HT あり）
- rustc 1.99.0（mise pin）/ cargo release profile（`debug = true`）
- 測定ハーネス: 両クレートを `path` 依存だけで参照する独立 scratch crate（`%TEMP%/r152-compare`、各リポジトリ非改変）。ソースは [`data/issue-152-al-math-compare-main.rs`](data/issue-152-al-math-compare-main.rs)・[`data/issue-152-al-math-compare-Cargo.toml`](data/issue-152-al-math-compare-Cargo.toml) に保存
- 生出力: [`data/issue-152-al-math-compare-output.txt`](data/issue-152-al-math-compare-output.txt)
- 計測方法: `std::time::Instant` の per-call 中央値（warmup 1 回 + 5〜200 rep。大形状は 5 rep）。alloc/bytes は counting global allocator で実測。数値差は `max |a − b|`
- 負荷注意: 単発の ad-hoc 計測であり Criterion 統計処理なし。±20% 程度の揺れはあり得るため桁の大小関係のみを読む（`ratio` は rf時間/al時間、>1 で al-math 側が高速）
- al-math 側は pulp V3 SIMD dispatch 込みの現行実装、ruviform 側は sequential scalar 実装

## 精度比較（同一入力・max |diff|）

形状は al-locate/al-analysis の実運用形状（`band_gain` window・narrow anchor・band_ncc/full_ncc・coarse_ncc・reference lag）。

| カーネル | 形状 (L, S / 条件) | mode | max |diff| | argmax 一致 |
| --- | --- | --- | --- | --- | --- |
| NCC | 4410, 7938（band_gain 44.1k） | NccValid | 3.1e-17 | ✓（2865） |
| NCC | 4800, 8640（band_gain 48k） | NccValid | 2.8e-17 | ✓（3454） |
| NCC | 4410, 66150（narrow anchor mid） | NccValid | 2.9e-17 | ✓（39281） |
| NCC | 96000, 100800（narrow anchor big） | NccValid | 6.9e-18 | ✓（2326） |
| NCC | 2205000, 2205236（band/full_ncc） | NccDirect | 6.5e-18 | ✓（185） |
| NCC | 551250, 551309（coarse_ncc） | NccDirect | 5.6e-18 | ✓（37） |
| NCC | 800, 1438（tiny 8kHz） | NccValid | 4.5e-17 | ✓（89） |
| GCC-PHAT | 4410, 7938（埋込み 1764） | Full | 3.8e-17 | ✓（真値どちらも 1764） |
| GCC-PHAT | 4800, 8640（埋込み 1920） | Full | 1.1e-16 | ✓（真値どちらも 1920） |
| GCC-PHAT | 4410, 66150（埋込み 30870） | Full | 1.9e-17 | ✓（真値どちらも 30870） |
| overlap | 8192, max_lag=512（reference lag） | FFT | 1.4e-17 | ✓（808） |
| overlap | 1024, max_lag=128 | FFT | 3.1e-17 | ✓（182） |
| overlap | 300, max_lag=130 | direct | 6.9e-17 | ✓（6） |

所見:

- 全形状で max |diff| ≤ 1.1e-16。al-math は pulp V3 SIMD カーネル（lanes 内部分和順序差）、ruviform は sequential scalar 評価順固定で、差は丸め評価順の違いのみに由来する。#150 の fft/vec と同じ誤差クラス。
- argmax（|値| 最大ラグ）は全ケースで一致し、GCC-PHAT は既知埋込み lag で両実装とも真値にピークを立てた。`NccValid`（短縮 FFT `nextpow2(S)`）と `NccDirect`（直接計算）の両経路が Full mode 相当の値を返すことは ruviform 側単体テストでも oracle 突合済み（`ncc_valid_plan_matches_full_plan_on_all_returned_lags`・`ncc_direct_matches_full_plan_on_all_returned_lags`・`ncc_matches_pointwise_oracle`・`overlap_corr_matches_corr_at_pointwise`）。
- `rf_fft_len=0` は `NccDirect`（FFT 非保持の直接計算 plan）を示す。al-math も同一コストモデルで同じ mode を選択（形状ピン留めテストで確認）。

## 性能比較（median µs/call・release・plan+ws 再利用）

| カーネル | 形状 | al-math (SIMD) | ruviform (scalar) | ratio (rf/al) |
| --- | --- | --- | --- | --- |
| NCC | 4410, 7938 | 125.1 | 165.6 | 1.32 |
| NCC | 4800, 8640 | 274.3 | 357.3 | 1.30 |
| NCC | 4410, 66150 | 2869.8 | 3694.4 | 1.29 |
| NCC | 96000, 100800 | 3050.8 | 4025.6 | 1.32 |
| NCC | 2205000, 2205236（direct） | 74128.1 | 154565.9 | 2.09 |
| NCC | 551250, 551309（direct） | 7253.7 | 10975.1 | 1.51 |
| NCC | 800, 1438 | 28.5 | 38.9 | 1.36 |
| GCC-PHAT | 4410, 7938 | 259.5 | 380.9 | 1.47 |
| GCC-PHAT | 4800, 8640 | 263.2 | 380.5 | 1.45 |
| GCC-PHAT | 4410, 66150 | 2704.4 | 3748.1 | 1.39 |
| overlap | 8192, max_lag=512 | 284.9 | 352.8 | 1.24 |
| overlap | 1024, max_lag=128 | 26.3 | 38.7 | 1.47 |
| overlap | 300, max_lag=130 | 12.2 | 17.0 | 1.39 |

所見:

- **scalar 実装でも al-math SIMD の 1.2〜2.1 倍以内**に収まった。#150 で FFT 単体は ~1.5〜1.8 倍遅かったが、相関呼出しは FFT 以外の逐次処理（prefix 二乗和・正規化・白化）が支配的で、そこは両実装とも scalar 同等のため差が縮む。
- FFT 経路の NCC/PHAT は 1.3〜1.5 倍差で概ね一定。direct 経路（大 L・小 K）は 1.5〜2.1 倍差で、SIMD axpy が scalar MAC ループに最も効く箇所（`2205000` 形状は呼出し ~74ms → ~155ms）。
- 大形状の direct NCC が最も差が開くのは al-math の pulp SIMD MAC vs sequential 加算の差であり、将来の SIMD 化余地として残る（scalar を oracle にした等価性検証前提、#150 と同じ方針）。

## メモリ / alloc 比較（counting allocator 実測）

| 項目 | al-math | ruviform-numeric |
| --- | --- | --- |
| CorrelationWorkspace 構築（4410,7938 NccValid） | 42 allocs / 783,720 B | 39 allocs / 652,160 B |
| 同（96000,100800 NccValid） | 51 allocs / 12,949,816 B | 48 allocs / 10,851,120 B（`memory_bytes()`=6,088,752 + FFT ws） |
| 同（2205000,2205236 NccDirect） | 5 allocs / 52,927,576 B | 3 allocs / 17,645,688 B |
| 処理呼出し（ncc/gcc_phat/curve・warmup 後） | 0 alloc | 0 alloc |

所見:

- **両実装とも処理呼出しは 0 alloc**（rt_ テスト `rt_processing_paths_do_not_allocate` でも assert_no_alloc 実測済み）。al-math は TLS FFT workspace の初回確保が入りうる点だけが暗黙的で、ruviform は workspace 明示化で構造的に排除。
- ruviform の ws は全形状で al-math より小さい。差の内訳は（a）al-math の `template_squares`/`signal_squares` scratch（L+S 要素）を `sum_sq_seq`/prefix 二乗和で代替して削除、（b）`phat_energy` を Full mode plan のみに限定（al-math は常に bins 分確保）。`(4410,7938)` での差 131,560 B ≈ (L+S)×8 + bins×8 と一致。
- direct mode では `real` を有効ラグ域 `S−L+1` に縮退させる形状は両者同じだが、上記 (a)(b) で 52.9MB → 17.6MB（約 1/3）。

## ルーティング検証（direct/FFT コストモデル）

`NccDirect` 判定係数 `C=8`（`L·K ≤ 8·n·log2(n)` かつ `K ≤ 2048`）は al-math 校正値を継承。scalar 実装で交差位置が変わりうるため、境界形状で direct 分子ループ（verbatim 複写）と FFT 経路（同 n の Full plan 呼出）を実測比較した。

| L, S | K | n | C_eff = L·K/(n·log2n) | t_direct | t_fft | 選択 |
| --- | --- | --- | --- | --- | --- | --- |
| 800, 840 | 41 | 1024 | 3.2 | 7.2 µs | 53.1 µs | direct（fft_len=0）✓ |
| 800, 1025 | 226 | 2048 | 8.0 | 35.6 µs | 36.9 µs | FFT（境界直下）✓ |
| 800, 1438 | 639 | 2048 | 22.7 | 105.1 µs | 72.8 µs | FFT ✓ |
| 2000, 2040 | 41 | 2048 | 3.6 | 17.9 µs | 72.2 µs | direct（fft_len=0）✓ |

所見:

- **C_eff=8.0 の境界形状で t_direct ≈ t_fft（35.6 vs 36.9 µs、差 ~4%）** — al-math 校正値 C=8 が scalar 実装でもほぼ正確に測定交差と一致する（scalar 化で direct/FFT 両経路が同程度に遅くなり比は保たれた）。係数は変更せず、選択ミスしても両経路の出力契約は同一。
- `K ≤ 2048` 上限の内側（K=226）で境界があり、上限は L1 に acc ベクトルを収める保険としてそのまま維持。
- overlap 相関の `3·out_len·L` vs `15·n·log2(n)` ルーティングも同一形状で両経路の一致を単体テスト済み（`overlap_corr_direct_and_fft_paths_match`、L=100・511 で FFT 経路を強制通過）。

## 構造差まとめ（API 契約レベル）

| 側面 | al-math | ruviform-numeric::corr |
| --- | --- | --- |
| エラー契約 | `assert!` panic（shape/mode/capacity） | `Result` + `Error::InvalidValue("<tag>")`（panic-free） |
| 出力返却 | `&'w [f64]`（値のみ） | `NccCurve`/`OverlapCurve`（値 + `window_energy`/`overlap_energy`/`template_energy` 証拠）— `band_gain` の `sum_sq` 再計算不要化 |
| overlap 容量 | `Vec::resize` で呼出中に確保しうる | workspace 構築時容量（`new`=2L−1 / `with_max_lag`）で処理呼出し alloc を構造排除 |
| `gcc_phat` 非対応 plan | panic | `Err(InvalidValue("mode"))` |
| FFT スクラッチ | TLS lazy init | `RealFftWorkspace` 明示所有 |
| SIMD | pulp V3 runtime dispatch | sequential scalar（bit-exact・#150 と同じ移植規則） |
| 生相互相関 | 公開 API なし（`cross_correlation` free fn 相当の内側のみ） | `cross_correlation`（`linear_lag_max` まで Full mode で無汚染公開、「必要なら full/partial lag」要件） |

## 結論

- `ruviform-numeric::corr` は al-math::correlation と全実運用形状で ≤1.1e-16 の数値一致（argmax 完全一致・PHAT 真値ピーク一致）を確認。用途非依存の f64 相関カーネルとして移植正本と互換の出力契約を持つ。
- 性能は scalar で al-math SIMD の 1.24〜2.09 倍。#98 baseline で計測された al-math 最適化水準に対し、SIMD 化は将来の改善余地として残る（scalar を oracle にした検証方針は設計書に記録済み）。
- 処理呼出し 0 alloc・workspace メモリは al-math より小さい構造改善を確認。direct/FFT ルーティング係数 C=8 は scalar 実測でも交差位置と一致し継承を維持。

## 未解決・留意点

- ad-hoc `Instant` 中央値の計測であり ±20% 程度の揺れ（#150 と同じ caveat）。継続計測が必要になれば Criterion ベンチの導入を検討する。
- `pulp` を使った f64 SIMD 化は #150 と同じく将来 Issue。比較数値が SIMD 化の効果見積もりとして使える。
- `cross_correlation` の Full-mode 部分 overlap ラグ（`S−L < k ≤ S+L−2`）は ruviform 側の新規公開面であり、al-math には対応公開 API がないため本比較対象外（ruviform 単体テストの oracle で検証済み）。
