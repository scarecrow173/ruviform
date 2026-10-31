//! 2 信号間のスペクトル統計プリミティブ（Issue #153）。
//!
//! `AudioLens` `al-analysis::stft` の `csd`/`complex_coherence` と、
//! `reference.rs`/`band_gain.rs` に inline で書かれていた Welch 型
//! フレーム蓄積（`Pxx`/`Pyy`/`Pxy` の per-bin 和）と H1 伝達関数
//! 推定量を、用途非依存の `f64` カーネルとして提供する。
//!
//! # 規約
//!
//! - 複素数は split re/im（`re[k] + i·im[k]`）。`csd` は `conj(x)·y`。
//! - [`CrossSpectrum`] は per-bin の **和** を保持する。coherence・
//!   `transfer_h1` は `1/frames` 係数が分子・分母で約分されるため、
//!   平均化せず和のまま計算する（AL の mean 化と値は一致する）。
//!   平均スペクトル自体が必要なら `sum_*` アクセサを [`crate::vec::scale`]
//!   で `1/frames` に合成する。
//! - 縮退規則（AL verbatim）: coherence は `sqrt(xx·yy) <= 0.0` の bin を
//!   `(0.0, 0.0)` に、`transfer_h1` は `xx <= 0.0` の bin を `(0.0, 0.0)` に
//!   写像する（無音・低 support を 0 の計測値として返し、蓄積フレーム数は
//!   [`CrossSpectrum::frames`] で観測可能）。NaN/Inf は IEEE どおり伝播
//!   （h1 の `xx > 0.0` 判定で NaN 入力のみ 0 写像）。
//! - `*_into` 系列は **RT-suitable**（alloc なし・有界）。所有 Vec を返す
//!   同名関数は non-RT。

use crate::{Error, Result};

/// `x`・`y` の per-bin cross spectral density（`conj(x)·y`）を
/// `out_re`/`out_im` に書き込む。
///
/// `out[k] = (x_re·y_re + x_im·y_im) + i·(x_re·y_im − x_im·y_re)`。
/// 全入力・出力は等長であること。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// 入力・出力のいずれかの長さが揃わなければ `Error::InvalidValue("length")`。
pub fn csd_into(
    x_re: &[f64],
    x_im: &[f64],
    y_re: &[f64],
    y_im: &[f64],
    out_re: &mut [f64],
    out_im: &mut [f64],
) -> Result<()> {
    let n = x_re.len();
    if x_im.len() != n
        || y_re.len() != n
        || y_im.len() != n
        || out_re.len() != n
        || out_im.len() != n
    {
        return Err(Error::InvalidValue("length"));
    }
    for k in 0..n {
        out_re[k] = x_re[k] * y_re[k] + x_im[k] * y_im[k];
        out_im[k] = x_re[k] * y_im[k] - x_im[k] * y_re[k];
    }
    Ok(())
}

/// [`csd_into`] の所有版（`(re, im)` のペアを確保して返す）。
///
/// **RT 契約:** non-RT only — 確保する。
///
/// # Errors
///
/// 入力の長さが揃わなければ `Error::InvalidValue("length")`。
pub fn csd(x_re: &[f64], x_im: &[f64], y_re: &[f64], y_im: &[f64]) -> Result<(Vec<f64>, Vec<f64>)> {
    let mut out_re = vec![0.0; x_re.len()];
    let mut out_im = vec![0.0; x_re.len()];
    csd_into(x_re, x_im, y_re, y_im, &mut out_re, &mut out_im)?;
    Ok((out_re, out_im))
}

/// per-bin 複素 coherence `Pxy / sqrt(Pxx·Pyy)` を `out_re`/`out_im` に
/// 書き込む。`sqrt(Pxx·Pyy) <= 0.0` の bin は `(0.0, 0.0)`（AL 規則）。
/// `|coh|²` が magnitude-squared coherence（`[0,1]`）になる。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// 入力・出力のいずれかの長さが揃わなければ `Error::InvalidValue("length")`。
pub fn complex_coherence_into(
    pxx: &[f64],
    pyy: &[f64],
    pxy_re: &[f64],
    pxy_im: &[f64],
    out_re: &mut [f64],
    out_im: &mut [f64],
) -> Result<()> {
    let n = pxx.len();
    if pyy.len() != n
        || pxy_re.len() != n
        || pxy_im.len() != n
        || out_re.len() != n
        || out_im.len() != n
    {
        return Err(Error::InvalidValue("length"));
    }
    for k in 0..n {
        let denom = (pxx[k] * pyy[k]).sqrt();
        if denom <= 0.0 {
            out_re[k] = 0.0;
            out_im[k] = 0.0;
        } else {
            out_re[k] = pxy_re[k] / denom;
            out_im[k] = pxy_im[k] / denom;
        }
    }
    Ok(())
}

/// [`complex_coherence_into`] の所有版。
///
/// **RT 契約:** non-RT only — 確保する。
///
/// # Errors
///
/// 入力の長さが揃わなければ `Error::InvalidValue("length")`。
pub fn complex_coherence(
    pxx: &[f64],
    pyy: &[f64],
    pxy_re: &[f64],
    pxy_im: &[f64],
) -> Result<(Vec<f64>, Vec<f64>)> {
    let mut out_re = vec![0.0; pxx.len()];
    let mut out_im = vec![0.0; pxx.len()];
    complex_coherence_into(pxx, pyy, pxy_re, pxy_im, &mut out_re, &mut out_im)?;
    Ok((out_re, out_im))
}

/// per-bin H1 伝達関数推定量 `Pxy / Pxx` を `out_re`/`out_im` に書き込む。
/// `Pxx <= 0.0` の bin は `(0.0, 0.0)`（AL `fir_ir` 規則 — 無音 bin を
/// 0 の計測値として返し、NaN は `xx > 0.0` 判定で 0 に写像される）。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// 入力・出力のいずれかの長さが揃わなければ `Error::InvalidValue("length")`。
pub fn transfer_h1_into(
    pxx: &[f64],
    pxy_re: &[f64],
    pxy_im: &[f64],
    out_re: &mut [f64],
    out_im: &mut [f64],
) -> Result<()> {
    let n = pxx.len();
    if pxy_re.len() != n || pxy_im.len() != n || out_re.len() != n || out_im.len() != n {
        return Err(Error::InvalidValue("length"));
    }
    for k in 0..n {
        if pxx[k] > 0.0 {
            out_re[k] = pxy_re[k] / pxx[k];
            out_im[k] = pxy_im[k] / pxx[k];
        } else {
            out_re[k] = 0.0;
            out_im[k] = 0.0;
        }
    }
    Ok(())
}

/// [`transfer_h1_into`] の所有版。
///
/// **RT 契約:** non-RT only — 確保する。
///
/// # Errors
///
/// 入力の長さが揃わなければ `Error::InvalidValue("length")`。
pub fn transfer_h1(pxx: &[f64], pxy_re: &[f64], pxy_im: &[f64]) -> Result<(Vec<f64>, Vec<f64>)> {
    let mut out_re = vec![0.0; pxx.len()];
    let mut out_im = vec![0.0; pxx.len()];
    transfer_h1_into(pxx, pxy_re, pxy_im, &mut out_re, &mut out_im)?;
    Ok((out_re, out_im))
}

/// Welch 型の per-bin スペクトル蓄積器（`Pxx`/`Pyy`/`Pxy` のフレーム和）。
///
/// 各フレームの split re/im スペクトルを [`accumulate`](Self::accumulate) で
/// 加算し、[`coherence_into`](Self::coherence_into) /
/// [`transfer_h1_into`](Self::transfer_h1_into) で推定量を取り出す。
/// 和のまま評価するため `frames` が異なる系列同士を混ぜても coherence/H1 の
/// 値は平均化と一致する（`1/frames` が約分）。
///
/// # Examples
///
/// ```
/// use ruviform_numeric::xspec::CrossSpectrum;
///
/// let mut acc = CrossSpectrum::new(2);
/// acc.accumulate(&[1.0, 0.0], &[0.0, 0.0], &[2.0, 0.0], &[0.0, 0.0])
///     .unwrap();
/// acc.accumulate(&[1.0, 0.0], &[0.0, 0.0], &[2.0, 0.0], &[0.0, 0.0])
///     .unwrap();
/// // y = 2x → H1 の DC bin は (2, 0)。
/// let mut hre = [0.0; 2];
/// let mut him = [0.0; 2];
/// acc.transfer_h1_into(&mut hre, &mut him).unwrap();
/// assert_eq!(hre[0], 2.0);
/// ```
pub struct CrossSpectrum {
    /// bin 数（全バッファの長さ）。
    bins: usize,
    /// 蓄積フレーム数。
    frames: usize,
    /// `Σ|x|²`。
    xx: Vec<f64>,
    /// `Σ|y|²`。
    yy: Vec<f64>,
    /// `Σ conj(x)·y` 実部。
    xy_re: Vec<f64>,
    /// `Σ conj(x)·y` 虚部。
    xy_im: Vec<f64>,
}

impl CrossSpectrum {
    /// `bins` bin の蓄積バッファを全確保する。
    ///
    /// **RT 契約:** non-RT only — `5·bins` 要素を確保する。
    #[must_use]
    pub fn new(bins: usize) -> Self {
        Self {
            bins,
            frames: 0,
            xx: vec![0.0; bins],
            yy: vec![0.0; bins],
            xy_re: vec![0.0; bins],
            xy_im: vec![0.0; bins],
        }
    }

    /// 1 フレーム分の `conj(x)·y`・`|x|²`・`|y|²` を per-bin 加算する。
    /// 全入力の長さは `bins` に一致しなければならない。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// 入力長が `bins` と異なれば `Error::InvalidValue("length")`。
    pub fn accumulate(
        &mut self,
        x_re: &[f64],
        x_im: &[f64],
        y_re: &[f64],
        y_im: &[f64],
    ) -> Result<()> {
        let n = self.bins;
        if x_re.len() != n || x_im.len() != n || y_re.len() != n || y_im.len() != n {
            return Err(Error::InvalidValue("length"));
        }
        for k in 0..n {
            let (xr, xi) = (x_re[k], x_im[k]);
            let (yr, yi) = (y_re[k], y_im[k]);
            self.xx[k] += xr * xr + xi * xi;
            self.yy[k] += yr * yr + yi * yi;
            self.xy_re[k] += xr * yr + xi * yi;
            self.xy_im[k] += xr * yi - xi * yr;
        }
        self.frames += 1;
        Ok(())
    }

    /// 蓄積したフレーム数。
    #[must_use]
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// bin 数。
    #[must_use]
    pub fn bins(&self) -> usize {
        self.bins
    }

    /// `Σ|x|²`（和、正規化なし）。
    #[must_use]
    pub fn sum_xx(&self) -> &[f64] {
        &self.xx
    }

    /// `Σ|y|²`（和、正規化なし）。
    #[must_use]
    pub fn sum_yy(&self) -> &[f64] {
        &self.yy
    }

    /// `Σ conj(x)·y` 実部（和、正規化なし）。
    #[must_use]
    pub fn sum_xy_re(&self) -> &[f64] {
        &self.xy_re
    }

    /// `Σ conj(x)·y` 虚部（和、正規化なし）。
    #[must_use]
    pub fn sum_xy_im(&self) -> &[f64] {
        &self.xy_im
    }

    /// 蓄積和に対する [`complex_coherence_into`]。
    /// `out_re`/`out_im` の長さは `bins` に一致しなければならない。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// 出力長が `bins` と異なれば `Error::InvalidValue("length")`。
    pub fn coherence_into(&self, out_re: &mut [f64], out_im: &mut [f64]) -> Result<()> {
        complex_coherence_into(&self.xx, &self.yy, &self.xy_re, &self.xy_im, out_re, out_im)
    }

    /// 蓄積和に対する [`transfer_h1_into`]。
    /// `out_re`/`out_im` の長さは `bins` に一致しなければならない。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// 出力長が `bins` と異なれば `Error::InvalidValue("length")`。
    pub fn transfer_h1_into(&self, out_re: &mut [f64], out_im: &mut [f64]) -> Result<()> {
        transfer_h1_into(&self.xx, &self.xy_re, &self.xy_im, out_re, out_im)
    }

    /// 保持バッファのメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        (self.xx.len() + self.yy.len() + self.xy_re.len() + self.xy_im.len()) * size_of::<f64>()
    }
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    fn split(v: &[(f64, f64)]) -> (Vec<f64>, Vec<f64>) {
        (
            v.iter().map(|v| v.0).collect(),
            v.iter().map(|v| v.1).collect(),
        )
    }

    #[test]
    fn csd_computes_conj_x_times_y() {
        // X = 3+4i, Y = 2+1i → conj(X)·Y = (6+4) + i(3−8) = 10 − 5i。
        let (re, im) = csd(&[3.0], &[4.0], &[2.0], &[1.0]).unwrap();
        assert_eq!(re, vec![10.0]);
        assert_eq!(im, vec![-5.0]);
        // 長さ不一致。
        assert_eq!(
            csd_into(&[1.0], &[0.0], &[1.0, 2.0], &[0.0, 0.0], &mut [], &mut []),
            Err(Error::InvalidValue("length"))
        );
    }

    #[test]
    fn coherence_is_one_for_identical_spectra_and_zero_for_dead_bins() {
        // 同一スペクトル → coherence = (1,0)。エネルギー 0 → (0,0)。
        let pxx = [4.0, 0.0];
        let pyy = [4.0, 0.0];
        let (xyr, xyi) = split(&[(4.0, 0.0), (0.0, 0.0)]);
        let (re, im) = complex_coherence(&pxx, &pyy, &xyr, &xyi).unwrap();
        assert_eq!(re, vec![1.0, 0.0]);
        assert_eq!(im, vec![0.0, 0.0]);
        // 片側ゼロ → denom=0 → (0,0)。
        let (re, im) = complex_coherence(&[0.0], &[4.0], &[1.0], &[1.0]).unwrap();
        assert_eq!((re, im), (vec![0.0], vec![0.0]));
        // NaN は denom の `<= 0.0` 判定を通らず除算へ進み NaN を伝播。
        let (re, _) = complex_coherence(&[f64::NAN], &[1.0], &[1.0], &[0.0]).unwrap();
        assert!(re[0].is_nan());
    }

    #[test]
    fn transfer_h1_recovers_gain_and_maps_dead_bins_to_zero() {
        // y = 2x の定常系列 → 全 bin で H1 = 2。
        let pxx = [4.0, 0.0, 9.0];
        let pxy_re = [8.0, 0.0, 18.0];
        let pxy_im = [0.0, 0.0, 0.0];
        let (re, im) = transfer_h1(&pxx, &pxy_re, &pxy_im).unwrap();
        assert_eq!(re, vec![2.0, 0.0, 2.0]);
        assert_eq!(im, vec![0.0; 3]);
        // NaN Pxx は `xx > 0.0` 判定で 0 写像（IEEE 伝播でなく縮退規則）。
        let (re, _) = transfer_h1(&[f64::NAN], &[1.0], &[0.0]).unwrap();
        assert_eq!(re[0], 0.0);
        // 位相差だけの回転因子: y = i·x → H1 = i。
        let (re, im) = transfer_h1(&[1.0], &[0.0], &[1.0]).unwrap();
        assert!(re[0].abs() < 1e-15 && (im[0] - 1.0).abs() < 1e-15);
    }

    #[test]
    fn cross_spectrum_accumulates_and_derives() {
        let mut acc = CrossSpectrum::new(3);
        assert_eq!(acc.frames(), 0);
        assert_eq!(acc.bins(), 3);
        // x = [1, i, 0], y = [2, 2i, 0]（y = 2x）。
        let x_re = [1.0, 0.0, 0.0];
        let x_im = [0.0, 1.0, 0.0];
        let y_re = [2.0, 0.0, 0.0];
        let y_im = [0.0, 2.0, 0.0];
        acc.accumulate(&x_re, &x_im, &y_re, &y_im).unwrap();
        acc.accumulate(&x_re, &x_im, &y_re, &y_im).unwrap();
        assert_eq!(acc.frames(), 2);
        assert_eq!(acc.sum_xx(), &[2.0, 2.0, 0.0]);
        assert_eq!(acc.sum_yy(), &[8.0, 8.0, 0.0]);
        assert_eq!(acc.sum_xy_re(), &[4.0, 4.0, 0.0]);
        assert_eq!(acc.sum_xy_im(), &[0.0; 3]);
        let mut re = [0.0; 3];
        let mut im = [0.0; 3];
        acc.coherence_into(&mut re, &mut im).unwrap();
        assert_eq!(re, [1.0, 1.0, 0.0]);
        acc.transfer_h1_into(&mut re, &mut im).unwrap();
        assert_eq!(re, [2.0, 2.0, 0.0]);
        // 長さ不一致 → Err。frames=0 の coherence は全 (0,0)。
        assert_eq!(
            acc.accumulate(&x_re[..2], &x_im, &y_re, &y_im),
            Err(Error::InvalidValue("length"))
        );
        let empty = CrossSpectrum::new(1);
        empty.coherence_into(&mut [0.0], &mut [0.0]).unwrap();
        assert!(empty.memory_bytes() > 0);
    }

    /// `CrossSpectrum` の蓄積は inline 手書きループ（AL の
    /// `reference.rs`/`band_gain.rs` パターン）と bit 一致しなければ
    /// ならない — 同じ加算順を維持していることの回帰。
    #[test]
    fn accumulation_matches_inline_reference_bit_exact() {
        let bins = 8;
        let x_re: Vec<f64> = (0..bins).map(|k| (k as f64 * 0.7).sin()).collect();
        let x_im: Vec<f64> = (0..bins).map(|k| (k as f64 * 0.3).cos()).collect();
        let y_re: Vec<f64> = (0..bins).map(|k| (k as f64 * 0.5).cos()).collect();
        let y_im: Vec<f64> = (0..bins).map(|k| (k as f64 * 0.9).sin()).collect();
        let mut xx = [0.0f64; 8];
        let mut yy = [0.0f64; 8];
        let mut xyr = [0.0f64; 8];
        let mut xyi = [0.0f64; 8];
        let mut acc = CrossSpectrum::new(bins);
        for _ in 0..3 {
            for k in 0..bins {
                xx[k] += x_re[k].powi(2) + x_im[k].powi(2);
                yy[k] += y_re[k].powi(2) + y_im[k].powi(2);
                xyr[k] += x_re[k] * y_re[k] + x_im[k] * y_im[k];
                xyi[k] += x_re[k] * y_im[k] - x_im[k] * y_re[k];
            }
            acc.accumulate(&x_re, &x_im, &y_re, &y_im).unwrap();
        }
        for k in 0..bins {
            assert_eq!(acc.sum_xx()[k].to_bits(), xx[k].to_bits(), "xx[{k}]");
            assert_eq!(acc.sum_yy()[k].to_bits(), yy[k].to_bits(), "yy[{k}]");
            assert_eq!(acc.sum_xy_re()[k].to_bits(), xyr[k].to_bits(), "xyr[{k}]");
            assert_eq!(acc.sum_xy_im()[k].to_bits(), xyi[k].to_bits(), "xyi[{k}]");
        }
    }
}
