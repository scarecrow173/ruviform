---
id: "DESIGN-034R153StftBand5Xq9wPzLm"
type: "design"
status: "approved"
title: "Issue #153 汎用 f64 帯域演算・STFT・スペクトル統計（ruviform-numeric 拡張）"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/153"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  changes:
    added: []
    modified:
      - "crates/ruviform-numeric"
      - "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "ruviform-architecture-design-baseline.md"
    - "issue-150-f64-offline-numeric.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/simd-ulp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "offline-execution-driver.md"
  refines: []
  refined-by: []
  derives-from:
    - "ruviform-architecture-design-baseline.md"
    - "issue-150-f64-offline-numeric.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/blob/main/crates/al-analysis/src/stft.rs"
    - "https://github.com/scarecrow173/audio-lens/blob/main/crates/al-math/src/filter.rs"
  defers: []
  deferred-by: []
---

# Issue #153 汎用 f64 帯域演算・STFT・スペクトル統計

## Overview

AudioLens の `al-analysis::stft`（複素 STFT・PSD/Power/Magnitude・CSD・
coherence・帯域積分・1/3-octave 帯域・移動平均エンベロープ）と
`al-math::filter`（spectral mask 帯域フィルタ・`spectral_envelope`）のうち、
**用途非依存の数値処理** を `ruviform-numeric` に移植する。
`issue-150-f64-offline-numeric.md` の Deferred Design Concerns で予告された
`stft`/`帯域`/`CSD`/`coherence`/`filter` の拡張にあたり、`RealFftPlan`・
`FftPlan`・`Window`・`vec` をそのまま利用する。

AL 固有の FeatureRecord・Validity/Confidence・stage 意味論・alignment 選択・
Dataset 形式は移さない（Issue #153「Ruviform に入れない内容」）。

## Context

- #150 で `ruviform-numeric`（Primitives 最下位・std のみ・f64 専用・
  panic-free `Result`・split re/im・plan/workspace 分離・scalar bit-exact）
  が確立済み。本 Issue の全カーネルは同じ契約ファミリーに従う。
- al-analysis の呼出し構造: `cheap`/`reference`/`band_gain`/`expensive::*` が
  `stft()` を複数窓・複数信号に呼び、`bandpass_many`/`BandFilter` で
  1/3-octave 帯域波形を逐次生成し、Welch 蓄積（Pxx/Pyy/Pxy のフレーム和）を
  inline ループで書いている。共有 FFT plan・帯域中間情報の再利用が
  ボトルネック（audio-lens#71）だったため、`bandpass_many`/`BandFilter` と
  STFT plan は「1 回の forward を帯域/フレーム全員で共有」する構造を継承する。
- `band_ncc`（`CorrelationPlan::new_ncc`）は Issue #152（並行実施）の範囲。
  本 Issue が提供するのは帯域波形生成（`BandFilter`）までであり、
  `band → ncc` の接合は「#153 が波形を、`#152` が相関カーブを供給する」
  境界で設計上協調する。重複実装は作らない。

## Considered Options（配置）

| 配置 | 評価 |
| --- | --- |
| A. `ruviform-numeric` 新モジュール | 全カーネルが f64・用途非依存・plan/workspace ファミリー。#150 設計が想定した拡張先そのもの。**採用** |
| B. `ruviform-audio-analysis` | 同クレートは「audio ドメイン型上の f32・RT アナライザ」責務（`SpectrumAnalyzer` 等）。f64 オフライン・用途非依存の数値処理を置くと責務混在 + numeric 利用者へ audio-core 依存を強いる。不採用 |
| C. `ruviform-dsp` | f32 audio ドメイン型付きカーネル層。不採用（B と同理由） |
| D. `ruviform-math` | 「全公開関数 alloc なし」契約と衝突（#150 と同じ却下理由）。不採用 |

## カーネル責務表（採否・配置の記録 — 受け入れ条件）

| AudioLens カーネル（出所） | 汎用性判定 | Ruviform 配置 | 採否・契約差分 |
| --- | --- | --- | --- |
| `stft` / `StftMatrix`（al-analysis::stft） | 汎用 — 窓掛け・ゼロ埋め・複素 FFT の枠組みは用途非依存 | `numeric::stft`（`StftPlan`/`StftWorkspace`/`Stft`/`StftMatrix`/`stft`） | **採用**。`Vec<Vec<f64>>` → 連続 `frames×fft_size` レイアウト。`window=0`/`hop=0` の暗黙空結果は構築時 `Err` に変更（panic-free 検証契約） |
| `StftMatrix::{power, magnitude, psd}`（al-analysis::stft） | 汎用 | `StftMatrix` の同名メソッド + `*_into` | **採用**。PSD 分母 `Σw²·fs` を窓係数から実計算し任意窓へ一般化（AL の `0.375·window` ハードコードを Hann で再現する特別値として吸収。1 ulp 級の差は比較記録で測定） |
| `hann`（al-analysis::stft） | 汎用 | `numeric::window::Window::Hann`（#150 既存） | 既存を流用。同一 periodic 定義で重複実装しない |
| `band_power`（al-analysis::stft） | 汎用 — パワー行の `[lo,hi)` 積分 | `numeric::bands::{band_power, band_power_into}` | **採用**。`(sample_rate, fft_size)` 引数を `bin_hz` に一般化（呼出し側は `rate/fft_size` を渡す）。bin 写像規則 `[floor(lo/B), min(ceil(hi/B), len))` は verbatim 維持 |
| `third_octave_bands`（al-analysis::stft） | 汎用 — IEC 系 1/3-octave 帯域エッジ | `numeric::bands::third_octave_bands` | **採用**。`f_min≤0`・非有限・`f_max≤f_min` で AL 側は無限ループし得るため `Result` 化し拒否（`Err(InvalidValue)`）。正常入力の帯域列は AL と同一 |
| `csd`（al-analysis::stft） | 汎用 — per-bin `conj(x)·y` | `numeric::xspec::{csd, csd_into}` | **採用**。`Vec<(re,im)>` → split re/im（クレート規約） |
| `complex_coherence`（al-analysis::stft） | 汎用 — `Pxy/√(Pxx·Pyy)` | `numeric::xspec::{complex_coherence, complex_coherence_into}` | **採用**。`denom≤0 → (0,0)` 規則 verbatim。NaN は除算を通り伝播 |
| Welch 蓄積 `Pxx/Pyy/Pxy`（reference.rs・band_gain.rs の inline ループ） | 汎用原始統計 — 3 か所で同一のフレーム和パターン | `numeric::xspec::CrossSpectrum` | **採用（新規抽象）**。和のみ保持し、coherence/H1 は `1/frames` が約分されるため和のまま計算（AL と値域一致）。mean 系列が必要な呼出しは `vec::scale` で合成 |
| H1 伝達関数 `Pxy/Pxx`（reference::fir_ir inline） | 汎用 — Welch H1 推定量 | `numeric::xspec::{transfer_h1, transfer_h1_into}` | **採用**。`Pxx≤0 → (0,0)` 規則 verbatim。逆変換による時間領域 IR 化は呼出し側が `FftPlan` で行う（AL 同様） |
| `envelope`（al-analysis::stft） | 汎用 — 振幅行の中心移動平均 | `numeric::stft::envelope` | **採用**。`bins` 偶数時の窓幅 `bins+1`・端部短縮は verbatim |
| `bandpass`/`bandpass_many`/`BandFilter`（al-math::filter） | 汎用 — 1 回の forward を共有する zero-phase spectral mask | `numeric::bands::{BandFilter, bandpass, bandpass_many}` | **採用**。`band()`（確保・non-RT）に加え `band_into`（RT-suitable）を追加。mask bin 規則 `{k: lo ≤ k·B < hi}`・`hi=NaN → Nyquist まで通す`・元長切戻しは verbatim。`sample_rate=0 → Err` |
| `spectral_envelope`（al-math::filter） | 汎用 — 非負周波数マグニチュードの線形 bin 分割平均 `ln(mean+1e-12)` | `numeric::bands::spectral_envelope` | **採用**。`fft_size` 非ゼロ 2 冪の assert を `Err(InvalidValue)` に変更（責務表の引継ぎ項目）。`n = max(fft_size, next_pow2(len))`・`bins=0 → 空` 維持 |
| `fir_ir`/`fir_model`（reference.rs） | 部分汎用 — 推定量は transfer_h1、モデル畳込みは conv で合成可能 | — | 演算部は `transfer_h1`+`conv` で合成。**編成ロジック（モデル残差・cap 4096）は AL 残留** |
| phase unwrap / group delay（reference.rs） | 汎用寄りだが薄い（~15 行の後処理） | — | **不採用**。record 意味論と結合した後処理であり、AL#101 の利用側切替で必要になれば追加候補（Deferred） |
| iSTFT / overlap-add 再合成 | AL に実装が存在しない | — | **不採用・Deferred**。`fir_ir` は単一逆 FFT で足りる。必要要件が出た時点で別途設計 |
| `band_gain`/`band_ncc` の stage 意味論（reason 分類・極性群・Validity・閾値） | AudioLens 固有 | — | **移さない**。接合は `BandFilter`（#153）→ `CorrelationPlan`（#152）→ `dot`/`sum_sq`（#150）の合成 |
| `matrix_to_tensor`・intermediate Dataset 形式 | AudioLens 固有 | — | **移さない**（Issue 明記の非対象） |

## API Surface

全 API は `f64` 専用・split re/im・`Result<_, Error>`（`InvalidValue`）・
panic-free。`sample_rate` は `u32`（`ResamplePlan` と同じ規約）。

| モジュール | API | RT 契約 |
| --- | --- | --- |
| `stft` | `StftPlan::new(window_len, hop, shape, sample_rate)`/`with_fft_size`（確保） | non-RT only |
| `stft` | `StftPlan::{frame_count, frame_into, stft_into}` | **RT-suitable** — 事前確保済み状態のみ |
| `stft` | `StftPlan::stft`（`StftMatrix` 確保）/`Stft`（所有型）/`stft` one-shot | non-RT only |
| `stft` | `StftWorkspace::new` | non-RT only |
| `stft` | `StftMatrix::{power_into, magnitude_into, psd_into}` | **RT-suitable** |
| `stft` | `StftMatrix::{power, magnitude, psd}` | non-RT only |
| `stft` | `envelope(row, bins)` | non-RT only（確保） |
| `bands` | `third_octave_bands`/`band_power`/`bandpass`/`bandpass_many`/`spectral_envelope`/`BandFilter::new`/`BandFilter::band` | non-RT only（確保） |
| `bands` | `band_power_into`/`BandFilter::band_into` | **RT-suitable** |
| `xspec` | `CrossSpectrum::new`（確保） | non-RT only |
| `xspec` | `CrossSpectrum::accumulate`/`coherence_into`/`transfer_h1_into`・`csd_into`/`complex_coherence_into`/`transfer_h1_into` free fn | **RT-suitable** |
| `xspec` | `csd`/`complex_coherence`/`transfer_h1` free fn（確保） | non-RT only |

## Numeric Contract（本 Issue 追加分）

- **STFT フレーム規約**: `start ∈ {0, hop, 2·hop, …}` で `start+window_len ≤ len`
  を満たす先頭側フレームのみ（末尾の余りは捨てる・部分フレーム・
  centering・OLA 正規化なし）。`frame_count(len) = len ≥ window_len ?
  (len−window_len)/hop + 1 : 0`。
- **ゼロ埋め**: 各フレームは `x[i]·w[i]`（`i < window_len`）を `fft_size` まで
  ゼロ埋め。`fft_size` は既定 `next_pow2(window_len)`（`with_fft_size` で
  それ以上の 2 冪を許容）。信号長のパディングは行わない。
- **スペクトル規約**: 複素 FFT の全 `fft_size` bin（負周波数を含む）を保持
  （AL が `fir_ir` で全 bin 逆変換するため）。forward 無正規化は `FftPlan`
  規約そのまま。
- **PSD**: two-sided `|X[k]|²/(fs·Σw²)`。全 bin 和 × bin_hz がおおよそ信号
  パワー（窓・パディング由来の誤差を含む）。`psd_norm() = Σw²·fs` を公開し、
  呼出し側が和系列をそのままスケールする合成を可能にする。
- **帯域 bin 写像（2 規約を個別維持）**: `band_power` は
  `[floor(lo/B), min(ceil(hi/B), row.len()))`（端 bin を含む寛い規則）。
  `BandFilter`/`bandpass_many` は `{k: lo ≤ k·B < hi}`（`lower_bin` 二分探索、
  `hi=NaN → 末尾まで`、`lo=NaN → 0`）。両者は端の丸めが異なる別規約であり、
  AL の振る舞いをそれぞれ verbatim 維持する。
- **欠損・縮退表現**: `stft` のフレーム 0 は「短すぎ」を意味し妥当性判断は
  呼出し側（AL 契約継承）。coherence `denom≤0 → (0,0)`、transfer_h1
  `Pxx≤0 → (0,0)`（無音・低 support を 0 の計測値として返し、`frames()` で
  蓄積数を観測可能にする）。`CrossSpectrum` の `frames=0` は全系列 0。
  NaN/Inf は IEEE 伝播（`h1` のみ `Pxx>0` 判定で NaN → 0 写像、verbatim）。
- **メモリ上限**: 確保は plan/workspace/係数・スペクトル・呼出し側出力のみ。
  `memory_bytes()` で構造見積もりを公開。大入力は `frame_into`/`stft_into`/
  `band_into` で呼出し側バッファに直接書き、`StftMatrix` 全保持を回避できる。

## Error Model

`StftPlan::new` は `window_len=0`/`hop=0`/`sample_rate=0` を
`Err(InvalidValue)`（AL の暗黙空結果を検証エラー化）。`with_fft_size` は
`fft_size` 非 2 冪・`fft_size < window_len` を拒否。`frame_into`/`stft_into`/
`band_into`/`*_into` 系は出力長・workspace 不一致を `Err`。
`spectral_envelope` は `fft_size` 非ゼロ 2 冪違反を `Err`（AL の assert を
引き継ぎつつ panic-free 化）。`third_octave_bands` は非正・非有限・逆順入力を
`Err`（AL の潜在的無限ループを排除）。`BandFilter::new`/`spectral_envelope`/
`StftPlan` は `len.next_power_of_two()` の usize オーバーフローを
`checked_next_power_of_two` で検査して `Err`。

## Testing Strategy

- **oracle 検証**: STFT は naive O(n²) DFT（窓掛け込み）と `<1e-10` で比較。
  csd/coherence/H1/帯域積分は同一定義の直接評価と比較。
- **AL 契約回帰**: AL のユニットテスト相当（sine の bin 位置・非 2 冪窓の
  ゼロ埋め・PSD Parseval・1/3-octave 網羅・coherence=1・BandFilter≡
  bandpass_many の帯域一致・NaN hi・`envelope` 端部）を移植。
- **境界行列**: 空/1 要素/window ちょうど/window+hop−1/非 2 冪 window・
  空帯域・全帯域・`bins=0`・frames=0。
- **RT レーン**: `rt_` テストで `frame_into`/`stft_into`/`band_into`/
  `csd_into`/`complex_coherence_into`/`transfer_h1_into`/`accumulate`/
  `power_into`/`magnitude_into`/`psd_into`/`band_power_into` を
  `assert_no_alloc` 窓内で検証。
- **doctest**: 公開 API の例は doctest（暫定規約どおり）。

## AudioLens 比較方針

- 比較対象は #98 凍結 baseline worktree（`al98-baseline`）の
  `al-analysis::stft` / `al-math::filter`。`stft.rs`/`filter.rs`/`fft.rs`/
  `vec.rs` は baseline と現行 main で byte-identical を `diff -q` 確認済み
  （#150 と同じ兼合い）。
- ハーネスは #150 と同じく `%TEMP%` の独立 scratch crate に `al-math` を
  path 依存させる。`al-analysis::stft` は依存木を引かないよう、
  `al_math::fft::Fft` + `al_math::vec` 上の verbatim 移植を harness 内に置く
  （`al-locate::dsp::conv` と同じ扱い — `stft.rs` の外部依存は
  `al_locate::dsp` 経由の `al-math` カーネルのみ）。
- `stft`/`psd`/`power`/`magnitude`/`band_power`/`csd`/`complex_coherence`/
  `envelope`/`bandpass_many`/`BandFilter`/`spectral_envelope`/Welch 蓄積・H1 の
  max|diff|・per-call 中央値・alloc 実測を `docs/impl/exp/` に記録。
- `band_gain`/`band_ncc` 関連: #153 側の構成費用は `BandFilter` forward ×2 +
  帯域ごとの IFFT（`band`/`band_into`）と `dot`/`sum_sq`（#150 比較済み）。
  NCC 本体は #152 範囲（この worktree に未存在）のため、帯域波形生成部と
  Welch/coherence 部を production 形状（48 kHz・数秒・1/3-octave 帯域）で
  実測し、構造回帰（forward 共有の有無）を記録する。

## Consequences

- 良い点: 帯域・STFT・CSD 系の f64 カーネルが #150 と同じ plan/workspace・
  panic-free・RT ラベル規約で単一クレートに集約される。`BandFilter`・
  `StftPlan` が #71 型の「forward 共有」構造を API として固定し、
  `*_into` 系列で確保なし経路を持つ。AL 側は AL#101 で薄い adapter に
  置き換えられる。
- 悪い点: scalar 実装のため al-math（pulp SIMD）より FFT 系が 1.2〜3.7 倍
  遅い見込み（#150 計測と同じ幅。比較記録で定量化）。`Vec<(f64,f64)>`→
  split re/im 等の形状差は AL#101 adapter が吸収する。
- トレードオフ: `band_power`/`BandFilter` の 2 つの帯域 bin 規約をどちらも
  verbatim 維持した（統一すると AL golden と乖離する）。

## Deferred Design Concerns

- iSTFT/overlap-add 再合成 | 要件発生時に別 Issue | AL に利用実績がない |
- phase unwrap/group delay カーネル | AL#101 で必要になれば | reference.rs 内
  15 行の後処理として当面 AL 残留 |
- 任意 `Window` 以外のカスタム窓係数（`StftPlan` への係数注入） | 利用要求時 |
  現行は `Window` enum で AL 全用途をカバー |
- f64 カーネルの SIMD 化 | #150 と同じ Deferred | 等価性は本実装を oracle と
  して simd-ulp ADR に従って個別契約 |

## Verification Notes

- [ ] `stft`/`bands`/`xspec` モジュールが `ruviform-numeric` に追加され、
      crates/README.md・crate README・lib.rs の責務記述が同期されている。
- [ ] 全公開 API が panic-free で RT/non-RT ラベル付き。
- [ ] 上記テスト戦略が `mise run test` / `mise run test-rt` で緑。
- [ ] `mise run fmt-check`/`check`/`lint`/`build` が pass。
- [ ] `docs/impl/exp/issue-153-*.md` に al-analysis/al-math との精度・性能・
      メモリ比較と band_gain/band_ncc 関連の実測が記録されている。
