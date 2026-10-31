---
id: "DESIGN-034R150f64Num3r1cKxVb2zQ"
type: "design"
status: "approved"
title: "Issue #150 f64 高精度オフライン数値・DSP API（ruviform-numeric 新設）"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/150"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  changes:
    added:
      - "crates/ruviform-numeric"
    modified:
      - "Cargo.toml"
      - "crates/README.md"
      - "docs/designs/ruviform-architecture-design-baseline.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/simd-ulp.md"
    - "simd-dispatch-layer.md"
    - "ruviform-dsp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "offline-execution-driver.md"
    - "../adr/dsp-simd-backend-adoption.md"
  refines: []
  refined-by: []
  derives-from:
    - "ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/tree/main/crates/al-math"
  defers: []
  deferred-by: []
---

# Issue #150 f64 高精度オフライン数値・DSP API

## Overview

AudioLens の `al-math`（f64 FFT / Real FFT / リサンプラ / 数値ベクトル / linalg）を
代替できる、用途非依存の高精度オフライン数値・DSP API を Ruviform に追加する
（Issue #150、AudioLens 移行追跡 audio-lens#95 の方針 1）。

配置は **新クレート `ruviform-numeric`**（Primitives 層・std のみ・ruviform 内部
依存なし）とする。`ruviform-math` の「全公開関数 alloc なし・有界」契約と
`ruviform-dsp` の「audio ドメイン型付き f32 カーネル」責務のどちらも変えずに、
f64 オフライン API 群を一箇所へ集約する。実装は scalar 参照実装を先に確立し、
逐次評価順により全プラットフォームで bit-exact な結果を保証する。

## Context

- 既存 `ruviform-math::dsp` は f32 スライス + `SimdToken` の SIMD dispatch 層で、
  「全公開関数を alloc なし・有界の RT 適合設計とする」（baseline A.2）契約を持つ。
  f64 FFT の plan 構築・リサンプラ・linalg のような確保を伴う API をここに置くと
  契約を弱めることになる。public API の f32/f64 generic 化は simd-dispatch-layer
  設計で却下済み（backend 型の公開 API 露出・engine 層への型汚染）。
- `ruviform-dsp` は `ruviform-audio-core` / `ruviform-media-core` のドメイン型上の
  f32 オーディオカーネル層。f64 数値ベクトルや linalg は「audio ドメイン型付き
  DSP」という責務に収まらず、用途非依存の数値利用者へ audio-core/media-core の
  依存を強いる。
- baseline の f64 方針は「f32 正準・f64 はオフライン精度用途に許容・本格対応は
  オフライン高精度需要をトリガに保留」。本 Issue がその着手トリガである。
- AudioLens 移行では後続 Issue（#152 相関・#153 STFT/帯域）が同じ f64 オフライン
  ファミリーを拡張するため、単一の配置先が求められる。

## Considered Options

| 配置 | 評価 |
| --- | --- |
| A. `ruviform-math` 内モジュール | 「全公開関数 alloc なし」契約と衝突。契約を API 単位へ緩めると RT アンカーとしての単純な不変条件が失われる。不採用 |
| B. `ruviform-dsp` 内 f64 モジュール | FFT/リサンプル/窓/畳込みは責務に近いが、vec/linalg は DSP 外。非オーディオの数値利用に audio-core・media-core 依存を強いる。f32 RT 系と f64 オフライン系の混在で RT ストーリーも複雑化。不採用 |
| C. 新クレート（Primitives・無依存） | 責務境界が明確。どの既存契約も弱めない。#150/#152/#153 の f64 ファミリー全体の単一の受け皿になる。**採用** |
| D. `ruviform-numeric` → `ruviform-math` 依存 | 現時点で必要な共有物がなく YAGNI。f64 カーネルの SIMD 化が必要になった時点で dispatch 層経由を再検討する（Deferred） |

命名: `ruviform-math`（f32 SIMD dispatch・RT）/ `ruviform-dsp`（f32 audio ドメイン
型付き・RT process）に対し、`ruviform-numeric` は「f64 高精度オフライン数値・
DSP プリミティブ」という第三の責務を名指す。

## Dependency Direction

```text
Primitives:  ruviform-math / ruviform-rt / ruviform-media-core / ruviform-exec
             / ruviform-observability / ruviform-numeric  ★新設（層内順序は
             ruviform-math の直後。依存可能レイヤー: なし — std のみ）
Domain foundations:  ruviform-audio-core / ruviform-video-core / ruviform-dsp
                     / ruviform-audio-analysis / ruviform-video-proc / ruviform-media-io
```

- `ruviform-numeric` は ruviform 内部クレートに一切依存しない（`ruviform-math`
  と同じ「なし（std のみ）」）。安全 SIMD バックエンドを将来導入する場合も
  `ruviform-math` と同じく実装詳細として private dep で閉じ込める。
- 音声専用処理を本クレートへ漏らさない:`SampleRate`・`Error` 等のドメイン型を
  受け取らず、サンプルレートは `u32`・時刻は持たない・入出力は `&[f64]`。
- AudioLens 固有概念（Job/Stage/Validity/Confidence/FeatureRecord）を API に
  持ち込まない。`ruviform-numeric` から AudioLens への逆依存は存在しない。

## API Surface

全 API は `f64` 専用（generic 化しない）。複素数は実部・虚部を分離した等長
スライス（split re/im planar）で受け渡す。検証エラーは `Result<_, Error>`
（`Error::InvalidValue(&'static str)`、Copy・`#[non_exhaustive]`）で返し、
公開 API は panic-free とする。「数値的に解が存在しない」は `Option` で表す
（`linalg` のみ）。

| モジュール | API | RT 契約 |
| --- | --- | --- |
| `vec` | `dot`/`sum_sq`/`corr0`/`rms`/`scale`/`axpy`/`sub_scaled`/`mag_sq` | **RT-suitable** — alloc なし・有界・逐次和で bit-exact |
| `fft` | `FftPlan::new`/`FftWorkspace::new`（確保） | non-RT only |
| `fft` | `FftPlan::transform(&mut FftWorkspace, re, im, inverse)` | **RT-suitable** — 事前確保済み state のみ |
| `fft` | `Fft`（plan+workspace 所有の便宜型）`new`/`transform` | new: non-RT / transform: RT-suitable |
| `fft` | `RealFftPlan::new`/`RealFftWorkspace::new`（確保） | non-RT only |
| `fft` | `RealFftPlan::forward`/`inverse`（`&mut RealFftWorkspace` 経由） | **RT-suitable** |
| `fft` | `RealFft`（所有型）`new`/`forward`/`inverse` | new: non-RT / 変換: RT-suitable |
| `window` | `Window::{Rectangular,Hann,Hamming,Blackman}` `coeffs`（確保）/`fill` | coeffs: non-RT / fill: RT-suitable |
| `resample` | `ResamplePlan::new`・`output_len`・`resample`（Vec 返し） | non-RT only |
| `resample` | `ResamplePlan::resample_into(input, out)` | **RT-suitable** — 位相係数表を plan 時に全確保 |
| `conv` | `ConvPlan::new`/`ConvWorkspace::new`（確保）・`conv`（Vec 返し） | non-RT only |
| `conv` | `ConvPlan::convolve(&mut ConvWorkspace, signal, out)` | **RT-suitable** — overlap-save の全 buffer を事前確保 |
| `linalg` | `LeastSquares`（Householder QR + ridge）・`least_squares`・`condition_number` | non-RT only（分解・求解とも確保を伴う） |

`f64` → `f32` の暗黙ダウングレードは存在しない（入力も出力も `f64` のみ）。
精度が落ちる API は存在しないため降级 API は提供しない。

## Numeric Contract（明文化項目）

- **スケーリング**: forward は正規化なし（X[k] = Σ x[n]·e^(−2πi·kn/N)）、
  inverse は 1/N 正規化（x[n] = (1/N)·Σ X[k]·e^(+2πi·kn/N)）。forward→inverse
  の round-trip は元信号を復元する。
- **符号規約**: forward は負符号回転（−i）、inverse は正符号回転（+i）。
- **実数スペクトル配置**: `RealFftPlan::forward` は非負周波数 `N/2+1` bin を
  `out_re[k]`/`out_im[k]` に出力（k=0 が DC、k=N/2 が Nyquist）。負周波数は
  共役対称で省略する。`inverse` は同じ `N/2+1` 配置を入力に取り、DC/Nyquist の
  虚部は読まない（実数列の共役対称から常に 0 となるため）。
- **パディング**: 変換長の調整・ゼロ埋めは呼び出し側の責務。API は入力長を
  plan の `n` と完全一致で要求し、内部で暗黙に延長・切詰めしない。
- **複素数規約**: 複素配列は split re/im。`re[k] + i·im[k]` が bin k の複素値。
  インターリーブ形式は提供しない。
- **bit-exact 契約**: scalar 実装は逐次評価順を固定し、FMA 融合
  （`f64::mul_add`）を使わない `a*b+c` 形で統一する。結果は全プラットフォーム・
  全 ISA でビット完全一致（al-math の `pulp::Scalar` 経路と同じ演算順）。
  この契約は scalar 参照実装が「正」であり続ける限り維持する。将来 SIMD 化する
  場合は lane 分割による和順差が生じ得るため、カーネルごとに
  「bit-exact 維持」か「ulp 上限内」を simd-ulp ADR に従って個別契約する。
- **denormal**: flush しない（FTZ/DAZ はエンジン側責務 — baseline A.4 方針）。
- **NaN/Inf**: IEEE どおり伝播させる。`corr0` は NaN を含むエネルギー縮退を
  `0.0` へ写像する（`ex > 0 && ey > 0` の比較が false になる規則を契約化）。
  `linalg` は非有限入力を `None` で拒否する。`resample`/`fft`/`conv` は
  NaN/Inf をそのまま伝播し、ゼロ置換しない。
- **誤差特性**: FFT の round-trip 誤差は n ≤ 2^17 で max abs < 1e-10 を回帰
  テストで保証する（実測は 1e-13 台）。twiddle は 256 要素ごとの再三角関数で
  位相ドリフトを抑える（al-math 準拠）。

## FFT アルゴリズム（al-math 移植の骨子）

- radix-4 DIF 段（2 段分の radix-2 を融合した split-radix 形状）+ 終端 radix-2。
  twiddle は stage ごとに `[w1r|pad|w1i|pad|w2r|pad|w2i|pad]` の副表レイアウト
  （`TW_PAD = 8`）。pad は 4K 境界でのキャッシュセット衝突回避のためのもので、
  scalar 版でも同一レイアウトを維持する（将来の SIMD 移植と bit-exact 検証のため）。
- 小変換（n < `WS_MIN` = 16384）は in-place butterfly + タイル化 bit-reversal
  置換。大変換は呼び出し側提供の skew 付き workspace（`WS_SKEW` = 64 要素）へ
  out-of-place 第 1 stage を行い、残りを workspace 内で回し、digit-reversed
  gather 書き戻しに inverse 1/N スケールを畳み込む。
- al-math は大変換 workspace を thread_local で暗黙確保するが、本 API では
  **呼び出し側が `FftWorkspace`/`RealFftWorkspace` を明示所有**する。TLS 借用と
  初回呼出し時の暗黙 resize を排除し、`transform` を RT-suitable とするため。
- `RealFftPlan` は `m = n/2` の `FftPlan` + 組立て係数表を持ち、偶数/奇数標本の
  packing → m 点複素 FFT → 共役対 recombine で実数 FFT を構成する（内部で
  n=1 を特別扱い）。

## Resampler（polyphase Blackman-sinc）

- `ResamplePlan::new(source_rate, target_rate)`: 有理比を gcd で約分し
  `up`/`down`・`radius = ceil(32/min(rate_ratio,1))`・`cutoff = 0.94·ratio` を
  決定。全 `up` 位相の正規化済み係数表（各位相 `2·radius+1` tap）を構築時に
  確保する。メモリは `up × taps × 8B`（al-math の遅延 HashMap キャッシュとは
  異なり初回呼出の確保も無い）。
- 端点はゼロ拡張、出力長は `ceil(frames·up/down)`（`output_len`、u128 で
  オーバーフロー検査）。同一レートは入力をそのまま複製（波形を変えない）。
- `resample_into` は出力長が `output_len` と完全一致であることを要求し、
  alloc なしで位相畳込みのみを行う（RT-suitable）。`resample` は Vec を確保する
  non-RT 便宜版。

## Convolution（overlap-save）

- `conv` は `signal.len()` サンプル先頭分の線形畳込みを返す。空 kernel は
  `signal` 長のゼロ列、空 signal は空列（早期 return が `m-1` の underflow を
  防ぐ契約）。ブロック長 `b = max(1024, next_pow2(2·m))`、各ブロックが
  `l = b−m+1` の新規出力を生成する。
- `ConvPlan`（kernel スペクトルを plan 時に 1 回変換）+ `ConvWorkspace`
  （xbuf/obuf/スペクトル scratch を全確保）で `convolve` を RT-suitable にする。
  `conv` は plan+workspace を内蔵する one-shot 版（non-RT）。

## linalg

- `LeastSquares::from_columns(columns: &[&[f64]], lambda) -> Option<Self>`:
  Householder QR で ridge 付き最小二乗を分解。列は等長・有限・観測数 ≥ 列数を
  要求し、違反は `None`。分解は複数 rhs に再利用できる。
- `solve(rhs) -> Option<(Vec<f64>, f64)>`: 係数と残差 RMS（ridge 項を含めない
  元観測のみで評価）。非有限 rhs や非有限解は `None`。
- `least_squares` は factorize+solve の one-shot。`condition_number` は
  `AᵀA` への power iteration と `(AᵀA)⁻¹` への power iteration の比からスペクトル
  条件数を推定し、特異・非正方・空は `f64::INFINITY`（行列は row-major 正方、
  対称 `AᵀA` 入力を想定するため呼び出し側で区別する必要はない）。

## Error Model

```rust
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    /// 契約違反の入力値（長さ不一致・範囲外・非 2 冪・レート 0 等）。
    InvalidValue(&'static str),
}
```

- すべての検証失敗は `Err(Error::InvalidValue("<field>"))`。公開 API は panic-free。
- 長さ・形状・レンジの検証は公開境界で行い、内部カーネルは契約成立を前提とする
  （二重検証しない — `LeastSquares::solve` が分解済み内部 shape を再検査しない等）。
- `linalg` のみ「妥当な入力だが解が存在しない」（ランク落ち・非有限）を `None`
  で表現し、`Result`（契約違反）と `Option`（解なし）を使い分ける。

## f32/f64 分離方式

- generic 化・型パラメータ共有は行わない（simd-dispatch-layer 設計の公開 API
  generic 化却下と同じ理由。内部のモノモーフ化による共有のみ将来許容）。
- f32 RT API（`ruviform-math::dsp`・`ruviform-dsp`）は変更しない。f64 側を
  別クレートに置くため、既存 f32 呼び出し側のコード・feature・検証レーンは
  一切影響を受けない。
- f64 API から f32 への暗黙変換は存在しない。`Window` は f64 係数を返す別型で、
  `ruviform-dsp::Window` とは独立の `#[non_exhaustive]` enum。

## Testing Strategy

- **scalar 参照実装を正とした oracle 検証**: 本実装が scalar であるため、テストでは
  アルゴリズム定義に忠実な独立 oracle（naive O(n²) DFT、正規方程式の直接構成、
  直接畳込み、逐次和）と比較する。`vec`/`linalg`/`conv`/`resample` は
  al-math と同じ逐次演算順を持つため bit-exact を期待し、FFT 系は丸め経路差を
  許す範囲で `< 1e-10`（round-trip）/`< 1e-8`（既知スペクトル）を要求する。
- **誤差・境界回帰**: サイズ行列 0..=33（空・1 要素・端数）＋ 1k・64k・1M。
  FFT は n = 1, 2, …, 2^10 全冪と n ≥ `WS_MIN` の workspace 経路（16384, 32768）
  を含む。非 2 冪・長さ不一致・レート 0・非有限入力のエラーパスを網羅する。
- **実サンプル回帰**: 決定的な疑似実信号（複数正弦 + 高調波 + DC + LCG 疑似
  ノイズの混合）でスペクトルピーク位置・エネルギー保存・round-trip を検証する。
  音声波形の解析的性質（Parseval・エイリアス抑制・遅延補償）を fixture ではなく
  性質として検証する。
- **RT レーン**: `rt_` プレフィックス付きテストで `assert_no_alloc` 測定窓内に
  `transform`/`forward`/`inverse`/`vec`/`resample_into`/`convolve` を置き、
  `mise run test-rt` で検出する（fixture・warm-up は窓外）。

## AudioLens 比較記録

比較対象は AudioLens Issue #98 凍結 baseline worktree
（`al98-baseline`）の `crates/al-math`（pulp x86-v3 dispatch）。凍結 baseline
の al-math ソースは現行 `F:/repositry/audio-products/audio-lens` main と
byte-identical を `diff -q` で確認済みのため、現行カーネル・#98 baseline の
双方との突合を兼ねる。

測定方法（実測・両リポジトリ非改変）:

- 両クレートを `path` 依存だけで参照する独立 scratch crate を `%TEMP%` に
  作成し、同一決定入力を両側へ与えて比較（ソースは比較レポートと共に保存）。
- **精度**: max |diff|（同一演算順の resample は bit-exact、vec/linalg/conv は
  丸め順差のみ、FFT は ~1e-12 台）。
- **性能**: per-call `Instant` 中央値。al-math は pulp x86-v3 SIMD、本実装は
  sequential scalar — SIMD 未導入の退行は想定内、幅を記録する。
- **メモリ**: counting global allocator で構築/呼出し中 alloc を実測。

記録の正本は [`docs/impl/exp/issue-150-al-math-comparison-20261031.md`](../impl/exp/issue-150-al-math-comparison-20261031.md)。要約は末尾「Comparison Results」節。

## Consequences

- 良い点: f64 オフライン API が単一クレートに集約され、#152/#153 が同じ
  構造で拡張できる。math の「全公開関数 alloc なし」契約と dsp の audio 型
  契約が無傷で残る。呼び出し側所有 workspace により TLS 暗黙確保を排し、
  FFT/リサンプル/畳込みを API 単位で RT-suitable と明記できる。scalar 逐次
  評価で全環境 bit-exact を保証し、al-math の Scalar 経路と結果が一致する。
- 悪い点: SIMD 未導入のため大規模変換で al-math 現行実装より遅い（比較記録で
  定量化）。新クレート分の索引・表の更新コスト。AudioLens 側の呼出しは
  panic→Result/Option への移行アダプトが必要（AL#101 で吸収）。
- トレードオフ: scalar 先行で正しさを確立し、SIMD は「dispatch 層を
  ruviform-math に集約する仮説」の延長で後日導入する — f64 カーネルを
  math::dsp に追加するか、fearless_simd を private dep として本クレートに
  導入するかは別途設計する（Deferred）。

## Deferred Design Concerns

- f64 カーネルの SIMD 化（fearless_simd or math::dsp 集約）| 別 Issue |
  #150 実装後の性能比較で優先度評価 | 現状 scalar で要件を満たす。
- `correlation`（NCC/GCC-PHAT/相互相関） | Ruviform #152 | #152 着手 |
  本クレートの fft/vec プリミティブ上に構築する想定。
- `stft`/帯域/CSD/coherence、`filter`（bandpass/BandFilter/spectral_envelope） |
  Ruviform #153 | #153 着手 | `RealFftPlan`/`Window` をそのまま利用。
- `conv` の配置確定 | #150 の境界判定として本クレートに配置 | — | 責務マップの
  「#150 系または #152」の判定を、overlap-save が `RealFftPlan` に直接依存し
  #152 相関より汎用カーネルであることから #150 側とした。
- raw PCM 位深度/尺度の保持 | Issue #151 | #151 着手 | 本クレートは内部演算
  精度（f64）のみを扱い、raw 入力精度の観測契約とは混同しない。

## Verification Notes

- [x] `ruviform-numeric` が crates/README.md・workspace members・baseline A.2
      表に Primitives（std のみ）として記録されている。
- [x] 全公開 API が panic-free で、RT-suitable / non-RT のラベルが API 単位で
      付与されている（`Result`/`Option` による検証境界、panic 経路なし）。
- [x] scalar 参照・oracle・境界・実サンプル回帰テストが `mise run test` で
      緑になる（ruviform-numeric: 38 passed / 0 failed）。
- [x] `rt_` テストが `mise run test-rt` で検出され、RT-suitable API が測定窓内
      で確保しない（`rt_processing_paths_do_not_allocate` が assert_no_alloc
      下で vec・window・FFT・RealFFT・resample_into・convolve を実行し pass）。
- [x] 既存 f32 API・`mise run fmt/check/lint/test` が退行しない
      （全タスク pass、lint 0 警告・全クレートテスト緑）。
- [x] AudioLens 現行カーネルとの精度・性能・メモリ比較が「Comparison Results」
      節に記録されている（#98 凍結 baseline の al-math と現行 main が
      byte-identical のため同一測定で双方をカバー）。

## Comparison Results

実測記録の正本は
[`docs/impl/exp/issue-150-al-math-comparison-20261031.md`](../impl/exp/issue-150-al-math-comparison-20261031.md)
（ハーネスソース・生出力は同ディレクトリ `data/`）。要点:

- **精度**: resample は両実装で **bit-exact**（max|diff| = 0）。FFT forward は
  n=65536 で 3.6e-12 まで（twiddle 構築・SIMD/scalar 評価順差の丸め差、
  round-trip では ~1e-15 に収束）。vec/linalg/conv は ~1e-16〜1e-14 でほぼ一致。
- **性能**: SIMD 向きカーネル（FFT/rfft/dot/sum_sq）は al-math（pulp V3）が
  1.2〜3.7 倍高速（scalar-first の想定内退行）。resample は ruviform が
  1.8〜1.9 倍高速（plan 事前係数表 vs 呼出し毎 HashMap 構築の構造差）、
  lstsq も 1.7 倍高速。
- **メモリ/alloc**: transform 系は両側とも処理呼出し中 0 alloc（al-math は
  スレッド初回のみ TLS +262KB）。al-math `resample` は呼出し毎に 6 allocs・
  ~770KB を要するのに対し `resample_into` は 0 alloc — RT-suitable 契約の
  差を実測で裏付け。
