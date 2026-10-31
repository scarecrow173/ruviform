# Issue #150 ruviform-numeric ↔ AudioLens al-math 比較レポート（2026-10-31）

Issue [#150](https://github.com/scarecrow173/ruviform/issues/150) の受け入れ条件「AudioLens 現行カーネルとの精度・性能・メモリ比較を記録」の実測記録。
比較相手は **AudioLens Issue #98 凍結 baseline**（`al98-baseline` worktree、`baseline/` 配下に凍結済み成果物）の `crates/al-math`。凍結 baseline の al-math ソース（fft/resample/vec/linalg）は現行 `F:/repositry/audio-products/audio-lens` main と `diff -q` で byte-identical を確認済みのため、本記録は「現行カーネル」と「#98 baseline」の双方の突合を兼ねる。

## 環境メタデータ

- OS: Windows 11 x64 / CPU: Intel Core i9-9900K @ 3.60GHz / 16 コア（HT あり）
- rustc 1.99.0（mise pin）/ cargo release profile（`debug = true`）
- 測定ハーネス: 両クレートを `path` 依存だけで参照する独立 scratch crate（`%TEMP%/r150-compare`、各リポジトリ非改変）。ソースは [`data/issue-150-al-math-compare-main.rs`](data/issue-150-al-math-compare-main.rs)・[`data/issue-150-al-math-compare-Cargo.toml`](data/issue-150-al-math-compare-Cargo.toml) に保存。`al-locate::dsp::conv` は `al_math::fft::RealFft` だけを使う自己完結関数のため verbatim 移植して比較（`al-locate` の依存木は引かない）
- 生出力: [`data/issue-150-al-math-compare-output.txt`](data/issue-150-al-math-compare-output.txt)
- 計測方法: `std::time::Instant` の per-call 中央値（warmup 1 回 + 7〜200 rep）。alloc/bytes は counting global allocator で実測。数値差は `max |a − b|`
- 負荷注意: 単発の ad-hoc 計測であり Criterion 統計処理なし。±20% 程度の揺れはあり得るため桁の大小関係のみを読む（`x比率` は al時間/rf時間、>1 で ruviform 側が高速）

## 精度比較（同一入力・max |diff|）

| カーネル | 条件 | max |diff| |
| --- | --- | --- | --- |
| complex FFT forward | n=1024 / 8192 / 65536 | 5.7e-14 / 2.7e-13 / 3.6e-12 |
| complex FFT round-trip | n=1024 / 8192 / 65536 | 1.1e-15 / 1.1e-15 / 1.6e-15 |
| real FFT forward | n=1024 / 8192 / 65536 | 5.7e-14 / 2.3e-13 / 3.6e-12 |
| real FFT inverse | n=1024 / 8192 / 65536 | 6.7e-16 / 8.9e-16 / 1.0e-15 |
| vec `dot` / `sum_sq` | n=2²⁰（|値| ~1e5 規模） | 4.8e-12 / 1.2e-8 |
| vec `corr0` / `rms` | 同上 | 1.1e-17 / 7.0e-15 |
| vec `scale` / `axpy` / `sub_scaled` / `mag_sq` | 同上 | 0〜4.4e-16 |
| resample | 48000→96000 / 48000→44100（48kf） | **0.0（bit-exact）** |
| least_squares | m=64 n=2 λ=1e-3 | x: 1.1e-16 / rms: 8.7e-18 |
| conv（al-locate 移植） | n=100k m=256 | 1.9e-14 |

所見:

- **resample は bit-exact**。係数式・評価順・正規化を同一に移植したため。両実装が同一アルゴリズムの同一実装であることを実測で裏付け。
- FFT の forward 差は twiddle 表構築順と SIMD/scalar 評価順の差に由来（al-math は pulp V3 の SIMD カーネル、ruviform-numeric は sequential scalar）。forward は丸め順差として O(n·ε) で増大する値で、双方とも naive O(n²) DFT oracle との整合は ruviform 側テストで ≤1e-10 相対を確認済み。round-trip / inverse では ~1e-15 に収束。
- vec reduction 差は SIMD lanes 内の部分和順序差（pulp）と逐次加算（ruviform）の丸め差。`sum_sq` 1.2e-8 は総和 ~1e5 に対し相対 ~1e-13 で、f64 精度として問題ない。
- linalg / conv は ~1e-16〜1e-14 でほぼ一致。

## 性能比較（median µs/call・release）

| カーネル | 条件 | al-math | ruviform | 比率（al/rf） |
| --- | --- | --- | --- | --- |
| complex FFT forward | n=1024 | 5.8 | 9.3 | 0.62 |
| | n=16384 | 115.0 | 201.3 | 0.57 |
| | n=131072 | 1226.5 | 1867.1 | 0.66 |
| real FFT forward | n=8192 | 30.5 | 43.9 | 0.69 |
| | n=65536 | 331.7 | 459.4 | 0.72 |
| `dot` | n=2²⁰ | 682.5 | 1107.8 | 0.62 |
| `sum_sq` | n=2²⁰ | 250.5 | 940.1 | 0.27 |
| resample | 48000→96000・48kf | 10527.1 | 5762.5 | **1.83** |
| | 48000→44100・48kf | 5858.5 | 3045.6 | **1.92** |
| conv | n=1M m=256 | 11561.3 | 14399.3 | 0.80 |
| least_squares | m=256 n=4 λ=1e-3 | 28.0 | 16.7 | **1.68** |

所見:

- **SIMD 向きカーネル（FFT/rfft/dot/sum_sq）は al-math が 1.2〜3.7 倍高速**。pulp V3（AVX2+FMA）ベクトル化対 sequential scalar の順当な差であり、ruviform-numeric が scalar-first 参照実装である設計意図どおり。SIMD 化は将来の最適化余地として残す（最適化時は scalar 実装を oracle にした等価性検証が必須、と設計に記録済み）。
- **resample は ruviform が 1.8〜1.9 倍高速**。al-math は `resample()` 呼出し毎に位相係数を `HashMap` 遅延構築＋ `values`/`products` バッファを確保するのに対し、ruviform は plan 側に全位相の正規化係数表を事前確保して処理呼出しを alloc なしにした構造差が効いた（scalar でも逆転）。
- **lstsq も ruviform が 1.7 倍高速**。ruviform は分解・残差計算を `vec::*_seq` 連続バッファカーネルに統一して確保を削減した差と推定。
- conv は al-locate が 0.8 倍（= 高速）。al-locate 版は呼出し毎に plan+バッファを全確保しつつ SIMD FFT で相殺する形。ruviform は scalar FFT でも 0.8 倍まで食らいつく。

## メモリ / alloc 比較（counting allocator 実測）

| 項目 | al-math | ruviform-numeric |
| --- | --- | --- |
| FFT plan 構築（n=16384） | 32 allocs / 666,272 B | 32 allocs / 666,272 B（twiddle 構造同一） |
| 処理 workspace | TLS `RefCell<Vec>` に暗黙確保 | `FftWorkspace::new`: 1 alloc / 262,656 B（明示） |
| transform 呼出し（スレッド初回） | **+1 alloc / 262,656 B**（TLS lazy init） | **0 alloc / 0 B** |
| transform 呼出し（同スレッド 2 回目以降） | 0 / 0 | 0 / 0 |
| `resample` 呼出し（2 回目以降も毎回） | **+6 allocs / 770,228 B** | 0 / 0（`resample_into`） |
| resample plan 係数表 | 呼出し毎に HashMap 構築（plan 非保持） | 1,040 B（`ResamplePlan` 内に全位相） |

所見:

- al-math FFT はスレッド毎の初回 transform で TLS workspace を lazy 確保する設計（以降の呼出しは 0 alloc）。ruviform は workspace を `FftWorkspace` として明示化し、TLS 依存・初回 lazy 確保を構造上排除 — RT-suitable の記述を実測で裏付け。
- al-math `resample` は呼出し毎に ~770KB・6 allocs を伴う non-RT 実装。ruviform `resample_into` は同処理を 0 alloc で行う（`rt_processing_paths_do_not_allocate` テストでも検証済み）。
- plan 構築コストは FFT で byte-identical（同一 twiddle レイアウトを移植したため）。

## 構造差まとめ（API 契約レベル）

| 側面 | al-math | ruviform-numeric |
| --- | --- | --- |
| エラー契約 | `assert!` panic / `Result<_, &'static str>` | panic-free `Result<_, Error>`（linalg のみ `Option`） |
| workspace | TLS 暗黙 / `RealFftWorkspace` 明示が混在 | すべて明示 `*Workspace` 型（`&mut` 排他） |
| resample 係数 | 呼出し毎に HashMap 遅延構築 | plan 内全位相テーブル |
| conv | 呼出し毎に plan/buffer 全確保（one-shot のみ） | `ConvPlan`/`ConvWorkspace` 分離 + one-shot `conv` |
| 実行バックエンド | pulp x86-v3 SIMD | sequential scalar（FMA 非融合、ISA 非依存の bit-exact） |

## 未解決・後続

- SIMD 化は未実施（Issue #150 のスコープは scalar-first）。最適化実施時は本レポートの scalar 結果を oracle に等価性検証を行う。
- al-locate 由来の NCC/GCC-PHAT 等は Issue #152 範囲であり本比較対象外。
- 計測は単発 ad-hoc のため、継続計測が必要になれば Criterion ベンチ化を検討する。
