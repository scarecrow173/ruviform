//! 周波数帯域の分割・積分・spectral mask フィルタ（Issue #153・#183）。
//!
//! `AudioLens` `al-analysis::stft` の `band_power`/`third_octave_bands` と
//! `al-math::filter` の `bandpass`/`bandpass_many`/`BandFilter`/
//! `spectral_envelope` の移植。1 回の forward FFT を全帯域で共有する
//! 構造（audio-lens#71 の構造最適化）を API として固定する。
//!
//! # 帯域 bin 写像規約（2 種を個別に維持）
//!
//! - [`band_power`]/[`band_power_into`]: `[floor(lo/B), min(ceil(hi/B), len))`
//!   の bin を積分（端 bin を含む寛い規則。AL `band_power` verbatim）。
//! - [`BandSpectrum`]/[`BandFilter`]/[`bandpass`]/[`bandpass_many`]:
//!   `{k : lo ≤ k·B < hi}` の bin のみ通す spectral mask
//!   （`lo=NaN → 0`、`hi=NaN → 末尾まで`）。AL `FilterCore` verbatim。
//!   両者は端の丸めが異なる別規約であり、相互に置き換えない。
//!
//! # 共有スペクトルと並列化（Issue #183）
//!
//! - [`BandSpectrum`] は forward FFT 済みスペクトルの不変共有体であり、
//!   `Send + Sync` を自動実装する（全フィールドが値型）。`&`/`Arc` で
//!   複数スレッドから共有できる。
//! - [`BandSpectrum::band_into`] は `&self` と呼出ごとの
//!   [`BandScratch`]（`&mut` 排他）を取る。scratch をスレッド/タスク
//!   ごとに分離すれば、同一スペクトルへの帯域抽出をデータ競合なく
//!   並列実行できる。`fft_size` が同じ `BandSpectrum` 同士では
//!   scratch を使い回せる。
//! - [`BandFilter`] は `BandSpectrum` + `BandScratch` の逐次実行向け
//!   便宜型であり、出力規則は `BandSpectrum` と同一。
//!
//! # その他の規約
//!
//! - `BandSpectrum`/`BandFilter` は信号全体を `next_pow2(len)` へ
//!   ゼロ埋めして forward、帯域ごとに mask→IFFT→元長切戻し。
//!   spectral mask の端点影響（巡回畳込み由来）は呼出し側の guard 責務。
//! - [`band_into`](BandFilter::band_into)・
//!   [`BandSpectrum::band_into`]/[`band_power_into`] は
//!   **RT-suitable**（alloc なし・有界）。所有版・構築・
//!   [`third_octave_bands`]/[`spectral_envelope`] は non-RT。

use crate::fft::{RealFftPlan, RealFftWorkspace};
use crate::{Error, Result};

/// IEC 系 1/3-octave 帯域の `(lower, upper)` エッジ列を返す
/// （AL `third_octave_bands` の移植）。中心は `f_c = 1000·2^(k/3)`、
/// 帯域は `f_c/√r .. f_c·√r`（`r = 2^(1/3)`）。`f_min` 直下の中心まで
/// 下げてから `f_c·√r ≤ f_max` を満たす帯域を列挙する。
///
/// **RT 契約:** non-RT only — 出力を確保する。
///
/// # Errors
///
/// `f_min` が非正・非有限なら `Error::InvalidValue("f_min")`。
/// `f_max` が非有限または `f_max <= f_min` なら
/// `Error::InvalidValue("f_max")`（AL はこれらの入力で無限ループし得る
/// ため、検証境界で拒否する）。
pub fn third_octave_bands(f_min: f64, f_max: f64) -> Result<Vec<(f64, f64)>> {
    if !(f_min > 0.0 && f_min.is_finite()) {
        return Err(Error::InvalidValue("f_min"));
    }
    if !(f_max.is_finite() && f_max > f_min) {
        return Err(Error::InvalidValue("f_max"));
    }
    let mut bands = Vec::new();
    let ratio = 2f64.powf(1.0 / 3.0);
    let mut center = 1000.0;
    // f_min 以下になるまで中心を下げ、上へ歩く。
    while center / ratio.sqrt() > f_min {
        center /= ratio;
    }
    while center * ratio.sqrt() <= f_max {
        bands.push((center / ratio.sqrt(), center * ratio.sqrt()));
        center *= ratio;
    }
    Ok(bands)
}

/// パワー/PSD 行を `[lo, hi)` 帯域へ積分する（AL `band_power`）。
/// 帯域 `b` の値は `Σ row[k]·bin_hz`（`k ∈ [floor(lo/bin_hz),
/// min(ceil(hi/bin_hz), row.len()))`）。bin 範囲外の帯域は 0。
/// `out.len()` は `bands.len()` に一致しなければならない。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// `out.len()` が `bands.len()` と異なれば `Error::InvalidValue("length")`。
// bin 写像は AL 規則の verbatim 移植。帯域エッジを bin index へ
// 落とす `f64 as usize` の丸め（負・NaN → 0、inf → saturating）は
// 意図どおり。
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn band_power_into(
    row: &[f64],
    bin_hz: f64,
    bands: &[(f64, f64)],
    out: &mut [f64],
) -> Result<()> {
    if out.len() != bands.len() {
        return Err(Error::InvalidValue("length"));
    }
    for (o, &(lo, hi)) in out.iter_mut().zip(bands) {
        let b0 = (lo / bin_hz).floor() as usize;
        let b1 = (hi / bin_hz).ceil() as usize;
        *o = (b0..b1.min(row.len())).map(|b| row[b]).sum::<f64>() * bin_hz;
    }
    Ok(())
}

/// [`band_power_into`] の所有版。
///
/// **RT 契約:** non-RT only — 出力を確保する。
#[must_use]
pub fn band_power(row: &[f64], bin_hz: f64, bands: &[(f64, f64)]) -> Vec<f64> {
    let mut out = vec![0.0; bands.len()];
    // out は bands と同数に構築済みのため失敗しない。
    let _ = band_power_into(row, bin_hz, bands, &mut out);
    out
}

/// 単一帯域の spectral mask フィルタ（`bandpass_many` の 1 帯域版）。
///
/// **RT 契約:** non-RT only — plan・workspace・出力を確保する。
///
/// # Errors
///
/// [`BandFilter::new`] と同じ。
pub fn bandpass(samples: &[f64], sample_rate: u32, lo_hz: f64, hi_hz: f64) -> Result<Vec<f64>> {
    Ok(BandFilter::new(samples, sample_rate)?.band(lo_hz, hi_hz))
}

/// 全帯域の波形を 1 回の forward FFT から生成する（AL `bandpass_many`）。
/// 出力順は `bands` と同じ。空入力は帯域ごとの空ベクトル。
///
/// **RT 契約:** non-RT only — plan・workspace・出力を確保する。
///
/// # Errors
///
/// [`BandFilter::new`] と同じ。
///
/// # Examples
///
/// ```
/// use ruviform_numeric::bands::bandpass_many;
///
/// // 低域のみ通す mask: 10 Hz 正弦は残り、100 Hz 成分は落ちる。
/// let rate = 256;
/// let n = 256;
/// let t = |i: usize| i as f64 / rate as f64;
/// let sig: Vec<f64> = (0..n)
///     .map(|i| {
///         (2.0 * std::f64::consts::PI * 10.0 * t(i)).sin()
///             + (2.0 * std::f64::consts::PI * 100.0 * t(i)).sin()
///     })
///     .collect();
/// let bands = bandpass_many(&sig, rate, &[(5.0, 20.0)]).unwrap();
/// let e_lo: f64 = bands[0].iter().map(|v| v * v).sum();
/// let e_all: f64 = sig.iter().map(|v| v * v).sum();
/// assert!(e_lo > 0.4 * e_all && e_lo < 0.6 * e_all);
/// ```
pub fn bandpass_many(
    samples: &[f64],
    sample_rate: u32,
    bands: &[(f64, f64)],
) -> Result<Vec<Vec<f64>>> {
    let mut filter = BandFilter::new(samples, sample_rate)?;
    Ok(bands.iter().map(|&(lo, hi)| filter.band(lo, hi)).collect())
}

/// 1 回の forward FFT で保持した全スペクトルの不変共有体（Issue #183）。
/// 帯域ごとの出力規則は [`bandpass_many`] と同一。
///
/// 全フィールドが値型のため `Send + Sync` を自動実装し、`&`/`Arc` で
/// 複数スレッドから共有できる。帯域抽出は [`band_into`](Self::band_into)
/// が呼出ごとの [`BandScratch`] を `&mut` で排他借用するため、
/// スレッド/タスクごとの scratch で同一スペクトルへの並列抽出が
/// データ競合なく行える（audio-lens#71 の (channel, band) タスク並列化
/// で入力 1 本あたり forward FFT を 1 回に抑える用途）。
///
/// メモリ量は [`memory_bytes`](Self::memory_bytes)（plan + スペクトルの
/// 構造見積もり。scratch は [`BandScratch::memory_bytes`]）。
///
/// # Examples
///
/// ```
/// use ruviform_numeric::bands::{BandScratch, BandSpectrum};
///
/// let rate = 256;
/// let sig: Vec<f64> = (0..256).map(|i| (i as f64 * 0.713).sin()).collect();
/// let spectrum = BandSpectrum::new(&sig, rate).unwrap();
///
/// // 不変スペクトルを複数スレッドで共有し、scratch はスレッドごとに分離。
/// let bands = [(5.0, 20.0), (80.0, 120.0)];
/// let spectrum = &spectrum;
/// let out = std::thread::scope(|s| {
///     let handles: Vec<_> = bands
///         .iter()
///         .map(|&(lo, hi)| {
///             s.spawn(move || {
///                 let mut scratch = BandScratch::new(spectrum);
///                 let mut out = vec![0.0; spectrum.len()];
///                 spectrum
///                     .band_into(&mut scratch, lo, hi, &mut out)
///                     .unwrap();
///                 out
///             })
///         })
///         .collect();
///     handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()
/// });
/// assert_eq!(out.len(), 2);
/// assert_eq!(out[0].len(), sig.len());
/// ```
pub struct BandSpectrum {
    /// 変換長 `n` の実数 FFT plan。
    plan: RealFftPlan,
    /// bin 幅（Hz）`sample_rate / n`。
    bin_hz: f64,
    /// 元信号長。
    len: usize,
    /// 全信号スペクトル実部（`n/2+1`）。
    spectrum_re: Vec<f64>,
    /// 全信号スペクトル虚部（`n/2+1`）。
    spectrum_im: Vec<f64>,
}

impl BandSpectrum {
    /// `samples` の forward FFT を 1 回だけ行い、スペクトルを保持する。
    ///
    /// **RT 契約:** non-RT only — plan・スペクトルを確保する。
    ///
    /// # Errors
    ///
    /// `sample_rate = 0` → `Error::InvalidValue("sample_rate")`。
    /// `samples.len().next_power_of_two()` が `usize` を溢れる場合は
    /// `Error::InvalidValue("length")`。
    #[expect(clippy::cast_precision_loss)]
    pub fn new(samples: &[f64], sample_rate: u32) -> Result<Self> {
        if sample_rate == 0 {
            return Err(Error::InvalidValue("sample_rate"));
        }
        let n = samples
            .len()
            .checked_next_power_of_two()
            .ok_or(Error::InvalidValue("length"))?;
        let plan = RealFftPlan::new(n)?;
        let mut ws = RealFftWorkspace::new(&plan);
        let bins = plan.spectrum_len();
        let mut real = vec![0.0; n];
        real[..samples.len()].copy_from_slice(samples);
        let mut spectrum_re = vec![0.0; bins];
        let mut spectrum_im = vec![0.0; bins];
        plan.forward(&mut ws, &real, &mut spectrum_re, &mut spectrum_im)?;
        Ok(Self {
            plan,
            bin_hz: f64::from(sample_rate) / n as f64,
            len: samples.len(),
            spectrum_re,
            spectrum_im,
        })
    }

    /// 元信号長。
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// 元信号が空か。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// 内部 FFT 変換長（`next_pow2(len)`、ゼロ埋め後）。
    #[must_use]
    pub fn fft_size(&self) -> usize {
        self.plan.n()
    }

    /// bin 幅（Hz）。
    #[must_use]
    pub fn bin_hz(&self) -> f64 {
        self.bin_hz
    }

    /// `[lo_hz, hi_hz)` の帯域波形を `out`（長さ `len`）に書き込む。
    /// passband bin 規則: `{k : lo ≤ k·bin_hz < hi}`。`hi = NaN` は
    /// Nyquist まで通し、`lo = NaN` は 0 から通す（AL verbatim）。
    /// 出力は元長に切り戻される zero-phase 波形で、
    /// [`BandFilter::band_into`] と同一実装・同一結果（bit-exact）。
    ///
    /// `scratch` は [`BandScratch::new`] で構築した呼出ごとの排他領域。
    /// `fft_size` が同じ `BandSpectrum` 同士では使い回せる。
    ///
    /// **RT 契約:** RT-suitable — 事前確保済みスクラッチのみ使用し
    /// alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `out.len()` が `len` と異なれば `Error::InvalidValue("length")`。
    /// `scratch` が異なる変換長の `BandSpectrum` 用に構築されていれば
    /// `Error::InvalidValue("scratch")`。
    #[expect(clippy::cast_precision_loss)]
    pub fn band_into(
        &self,
        scratch: &mut BandScratch,
        lo_hz: f64,
        hi_hz: f64,
        out: &mut [f64],
    ) -> Result<()> {
        if out.len() != self.len {
            return Err(Error::InvalidValue("length"));
        }
        if scratch.n != self.plan.n() {
            return Err(Error::InvalidValue("scratch"));
        }
        if self.len == 0 {
            return Ok(());
        }
        // 連続した passband だけコピーし、範囲外はゼロにする。
        let lo_bin = lower_bin(self.spectrum_re.len(), |k| self.bin_hz * (k as f64) < lo_hz);
        let hi_bin = lower_bin(self.spectrum_re.len(), |k| {
            hi_hz.is_nan() || self.bin_hz * (k as f64) < hi_hz
        });
        scratch.masked_re.fill(0.0);
        scratch.masked_im.fill(0.0);
        if lo_bin < hi_bin {
            scratch.masked_re[lo_bin..hi_bin].copy_from_slice(&self.spectrum_re[lo_bin..hi_bin]);
            scratch.masked_im[lo_bin..hi_bin].copy_from_slice(&self.spectrum_im[lo_bin..hi_bin]);
        }
        self.plan.inverse(
            &mut scratch.ws,
            &scratch.masked_re,
            &scratch.masked_im,
            &mut scratch.real,
        )?;
        out.copy_from_slice(&scratch.real[..self.len]);
        Ok(())
    }

    /// 保持バッファのメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        (self.spectrum_re.len() + self.spectrum_im.len()) * size_of::<f64>()
            + self.plan.memory_bytes()
    }
}

/// [`BandSpectrum::band_into`] が消費する呼出ごとの排他スクラッチ
/// （Issue #183）。IFFT 用 workspace・mask 適用後スペクトル・逆変換出力
/// を事前確保する。1 つの `BandScratch` を同時に使う呼出は 1 つ
/// （`&mut` 借用で静的に保証）なので、並列化する場合はスレッド/タスク
/// ごとに構築して使い回す。
///
/// `Send + Sync` は自動実装されるが、並列契約は「共有スペクトル
/// `&BandSpectrum` × 排他 `&mut BandScratch`」であり、scratch 自体の
/// 共有は想定しない。
///
/// メモリ量は [`memory_bytes`](Self::memory_bytes)（構造見積もり）。
pub struct BandScratch {
    /// IFFT 用 workspace。
    ws: RealFftWorkspace,
    /// mask 適用後スペクトルのスクラッチ（`n/2+1`）。
    masked_re: Vec<f64>,
    /// 同上・虚部。
    masked_im: Vec<f64>,
    /// 逆変換出力スクラッチ（`n`）。
    real: Vec<f64>,
    /// 構築元 plan の変換長（`band_into` の整合検証用）。
    n: usize,
}

impl BandScratch {
    /// `spectrum` と同じ変換長のスクラッチを確保する。
    /// この scratch は `fft_size` が `spectrum` と同じ全ての
    /// [`BandSpectrum`] で使い回せる。
    ///
    /// **RT 契約:** non-RT only — workspace・スクラッチを確保する。
    #[must_use]
    pub fn new(spectrum: &BandSpectrum) -> Self {
        Self {
            ws: RealFftWorkspace::new(&spectrum.plan),
            masked_re: vec![0.0; spectrum.plan.spectrum_len()],
            masked_im: vec![0.0; spectrum.plan.spectrum_len()],
            real: vec![0.0; spectrum.plan.n()],
            n: spectrum.plan.n(),
        }
    }

    /// 保持バッファのメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        (self.masked_re.len() + self.masked_im.len() + self.real.len()) * size_of::<f64>()
            + self.ws.memory_bytes()
    }
}

/// 1 回の forward FFT で保持した全スペクトルから帯域波形を逐次生成する
/// フィルタ（AL `BandFilter` の移植）。帯域を 1 本ずつ消費する呼出側で
/// 全帯域波形の同時保持を避ける。帯域ごとの出力規則は
/// [`bandpass_many`] と同一。
///
/// 内部は共有スペクトル [`BandSpectrum`] + スクラッチ [`BandScratch`]
/// の逐次実行向け便宜型（Issue #183）。同一スペクトルを複数スレッドで
/// 共有して帯域抽出を並列化する場合は `BandSpectrum` を直接使う。
///
/// メモリ量は [`memory_bytes`](Self::memory_bytes)（plan + スペクトル +
/// スクラッチの構造見積もり）。
pub struct BandFilter {
    /// 共有スペクトル（不変部）。
    spectrum: BandSpectrum,
    /// 帯域抽出スクラッチ（排他部）。
    scratch: BandScratch,
}

impl BandFilter {
    /// `samples` の forward FFT を 1 回だけ行い、スペクトルを保持する。
    ///
    /// **RT 契約:** non-RT only — plan・workspace・スペクトル・スクラッチを
    /// 確保する。
    ///
    /// # Errors
    ///
    /// [`BandSpectrum::new`] と同じ。
    pub fn new(samples: &[f64], sample_rate: u32) -> Result<Self> {
        let spectrum = BandSpectrum::new(samples, sample_rate)?;
        let scratch = BandScratch::new(&spectrum);
        Ok(Self { spectrum, scratch })
    }

    /// 元信号長。
    #[must_use]
    pub fn len(&self) -> usize {
        self.spectrum.len()
    }

    /// 元信号が空か。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spectrum.is_empty()
    }

    /// 内部 FFT 変換長（`next_pow2(len)`、ゼロ埋め後）。
    #[must_use]
    pub fn fft_size(&self) -> usize {
        self.spectrum.fft_size()
    }

    /// bin 幅（Hz）。
    #[must_use]
    pub fn bin_hz(&self) -> f64 {
        self.spectrum.bin_hz()
    }

    /// `[lo_hz, hi_hz)` の帯域波形を `out`（長さ `len`）に書き込む。
    /// passband bin 規則: `{k : lo ≤ k·bin_hz < hi}`。`hi = NaN` は
    /// Nyquist まで通し、`lo = NaN` は 0 から通す（AL verbatim）。
    /// 出力は元長に切り戻される zero-phase 波形。
    ///
    /// **RT 契約:** RT-suitable — 事前確保済みスクラッチのみ使用し
    /// alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `out.len()` が `len` と異なれば `Error::InvalidValue("length")`。
    pub fn band_into(&mut self, lo_hz: f64, hi_hz: f64, out: &mut [f64]) -> Result<()> {
        self.spectrum
            .band_into(&mut self.scratch, lo_hz, hi_hz, out)
    }

    /// `[lo_hz, hi_hz)` の帯域波形を確保して返す。
    /// [`band_into`](Self::band_into) の所有版。
    ///
    /// **RT 契約:** non-RT only — 出力を確保する。
    #[must_use]
    pub fn band(&mut self, lo_hz: f64, hi_hz: f64) -> Vec<f64> {
        let mut out = vec![0.0; self.spectrum.len];
        // out は len に構築済みのため失敗しない。
        let _ = self.band_into(lo_hz, hi_hz, &mut out);
        out
    }

    /// 保持バッファのメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.spectrum.memory_bytes() + self.scratch.memory_bytes()
    }
}

/// 周波数 grid の `lower_bound`（AL `lower_bin` 移植）。
/// `predicate` が `[0, length)` で `true…false` に単調と仮定し、
/// 最初の `false` 位置を返す。
fn lower_bin(length: usize, predicate: impl Fn(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (0, length);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if predicate(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// 非負周波数の区間平均振幅を log に変換したスペクトル包絡
/// （AL `spectral_envelope` の移植）。
///
/// `n = max(fft_size, next_pow2(len))` の実数 FFT を取り、非負周波数
/// `n/2+1` bin の `|z|` を線形等分割 `bins` 区間で平均して
/// `ln(mean + 1e-12)` を返す。bin 分割は `[0, n/2)` に限られるため
/// Nyquist bin（index `n/2`）はどの区間にも含まれない（AL verbatim）。
/// `bins = 0` は空を返す。
///
/// **RT 契約:** non-RT only — plan・スペクトル・出力を確保する。
///
/// # Errors
///
/// `fft_size` が非ゼロの 2 冪でなければ `Error::InvalidValue("fft_size")`
/// （AL の assert を `Err` 化して引き継ぐ）。
/// `samples.len().next_power_of_two()` が `usize` を溢れる場合は
/// `Error::InvalidValue("length")`。
#[expect(clippy::cast_precision_loss)]
pub fn spectral_envelope(samples: &[f64], fft_size: usize, bins: usize) -> Result<Vec<f64>> {
    if !fft_size.is_power_of_two() {
        return Err(Error::InvalidValue("fft_size"));
    }
    if bins == 0 {
        return Ok(Vec::new());
    }
    let padded = samples
        .len()
        .checked_next_power_of_two()
        .ok_or(Error::InvalidValue("length"))?;
    let n = fft_size.max(padded);
    let plan = RealFftPlan::new(n)?;
    let mut ws = RealFftWorkspace::new(&plan);
    let mut real = vec![0.0; n];
    real[..samples.len()].copy_from_slice(samples);
    let mut spectrum_re = vec![0.0; plan.spectrum_len()];
    let mut spectrum_im = vec![0.0; plan.spectrum_len()];
    plan.forward(&mut ws, &real, &mut spectrum_re, &mut spectrum_im)?;
    let half = n / 2;
    Ok((0..bins)
        .map(|b| {
            let lo = b * half / bins;
            let hi = ((b + 1) * half / bins).max(lo + 1).min(half);
            let mut sum = 0.0;
            for k in lo..hi {
                let m2 = spectrum_re[k] * spectrum_re[k] + spectrum_im[k] * spectrum_im[k];
                sum += m2.sqrt();
            }
            (sum / (hi - lo) as f64 + 1e-12).ln()
        })
        .collect())
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    fn noise(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn third_octave_bands_cover_range() {
        // AL テストの移植: 20–20000 Hz は 25 帯域超をカバーし、
        // 最初の帯域は 20 Hz を跨ぎ、最後は ~16 kHz を超える。
        let bands = third_octave_bands(20.0, 20000.0).unwrap();
        assert!(bands.len() > 25);
        assert!(bands[0].0 <= 20.0 && bands[0].0 > 10.0);
        assert!(bands.last().unwrap().1 > 16_000.0);
        // 非正・非有限・逆順は Err（AL の潜在的無限ループを排除）。
        assert_eq!(
            third_octave_bands(0.0, 100.0).err(),
            Some(Error::InvalidValue("f_min"))
        );
        assert_eq!(
            third_octave_bands(-5.0, 100.0).err(),
            Some(Error::InvalidValue("f_min"))
        );
        assert_eq!(
            third_octave_bands(f64::NAN, 100.0).err(),
            Some(Error::InvalidValue("f_min"))
        );
        assert_eq!(
            third_octave_bands(100.0, f64::INFINITY).err(),
            Some(Error::InvalidValue("f_max"))
        );
        assert_eq!(
            third_octave_bands(100.0, 50.0).err(),
            Some(Error::InvalidValue("f_max"))
        );
    }

    #[test]
    fn band_power_integrates_rows() {
        // bin_hz=1、行 [1,1,1,1]、帯域 [0.5, 2.5) → bins 0(floor .5)..3(ceil 2.5)。
        let row = [1.0, 1.0, 1.0, 1.0];
        let out = band_power(&row, 1.0, &[(0.5, 2.5), (0.0, 4.0), (10.0, 20.0)]);
        assert_eq!(out, vec![3.0, 4.0, 0.0]);
        // band_power_into と一致・長さ不一致は Err。
        let mut buf = [0.0; 3];
        band_power_into(&row, 1.0, &[(0.5, 2.5), (0.0, 4.0), (10.0, 20.0)], &mut buf).unwrap();
        assert_eq!(buf, [3.0, 4.0, 0.0]);
        assert_eq!(
            band_power_into(&row, 1.0, &[(0.0, 1.0)], &mut buf),
            Err(Error::InvalidValue("length"))
        );
        // bin_hz スケール: row[0..2] × 0.5。
        let out = band_power(&row, 0.5, &[(0.0, 1.5)]);
        assert_eq!(out, vec![1.5]);
    }

    #[test]
    fn bandpass_isolates_band() {
        // AL dsp.rs テストの移植: 10 Hz + 100 Hz の混合を帯域分離する。
        let n = 256;
        let t = |i: usize| i as f64 / 256.0;
        let sig: Vec<f64> = (0..n)
            .map(|i| {
                (2.0 * core::f64::consts::PI * 10.0 * t(i)).sin()
                    + (2.0 * core::f64::consts::PI * 100.0 * t(i)).sin()
            })
            .collect();
        let lo = bandpass(&sig, 256, 5.0, 20.0).unwrap();
        let e_lo: f64 = lo.iter().map(|v| v * v).sum();
        let hi = bandpass(&sig, 256, 80.0, 120.0).unwrap();
        let e_hi: f64 = hi.iter().map(|v| v * v).sum();
        assert!(e_lo > e_hi * 0.5 && e_hi > e_lo * 0.5);
        // band() と band_into() は同一出力。
        let mut filter = BandFilter::new(&sig, 256).unwrap();
        let mut out = vec![0.0; n];
        filter.band_into(5.0, 20.0, &mut out).unwrap();
        assert_eq!(out, lo);
        assert_eq!(filter.len(), n);
        assert_eq!(filter.fft_size(), 256);
        assert_eq!(filter.bin_hz(), 1.0);
        // out 長不一致は Err。
        assert_eq!(
            filter.band_into(5.0, 20.0, &mut vec![0.0; n - 1]),
            Err(Error::InvalidValue("length"))
        );
    }

    /// AL `band_filter_matches_bandpass_many_per_band` の移植:
    /// `BandFilter::band` の逐次生成は `bandpass_many` と帯域ごと一致。
    #[test]
    fn band_filter_matches_bandpass_many_per_band() {
        for len in [0usize, 1, 3, 33, 256, 1027] {
            let input: Vec<_> = (0..len).map(|i| (i as f64 * 0.713).sin()).collect();
            let bands = [
                (0.0, 100.0),
                (100.0, 250.0),
                (0.0, 1025.0),
                (250.0, 250.0),
                (500.0, f64::NAN),
                (0.0, f64::INFINITY),
            ];
            let mut filter = BandFilter::new(&input, 2048).unwrap();
            let expected = bandpass_many(&input, 2048, &bands).unwrap();
            for ((lo, hi), expected) in bands.iter().zip(expected) {
                assert_eq!(
                    filter.band(*lo, *hi),
                    expected,
                    "len={len} band=({lo},{hi})"
                );
            }
        }
    }

    #[test]
    fn band_filter_edge_cases() {
        // 空入力 → 全帯域空。
        let mut filter = BandFilter::new(&[], 48000).unwrap();
        assert!(filter.is_empty());
        assert_eq!(filter.band(0.0, 1000.0), Vec::<f64>::new());
        filter.band_into(0.0, 1000.0, &mut []).unwrap();
        // 空帯域（lo == hi）→ ゼロ波形。
        let input = noise(512, 7);
        let mut filter = BandFilter::new(&input, 2048).unwrap();
        assert!(filter.band(250.0, 250.0).iter().all(|&v| v == 0.0));
        // sample_rate = 0 → Err。
        assert_eq!(
            BandFilter::new(&input, 0).err(),
            Some(Error::InvalidValue("sample_rate"))
        );
        // NaN hi は Nyquist まで通す: 全帯域 mask は元信号を
        //（ゼロ埋め・巡回端の差を除き）ほぼ復元する。
        let mut filter = BandFilter::new(&input, 2048).unwrap();
        let full = filter.band(0.0, f64::NAN);
        for (a, b) in full.iter().zip(&input) {
            assert!((a - b).abs() < 1e-9);
        }
        // 負 lo は 0 から通す。lo = +inf / hi = -inf は空帯域。
        let neg = filter.band(-100.0, f64::NAN);
        for (a, b) in neg.iter().zip(&input) {
            assert!((a - b).abs() < 1e-9);
        }
        assert!(filter.band(f64::INFINITY, 1.0).iter().all(|&v| v == 0.0));
        assert!(
            filter
                .band(0.0, f64::NEG_INFINITY)
                .iter()
                .all(|&v| v == 0.0)
        );
        assert!(BandFilter::new(&input, 2048).unwrap().memory_bytes() > 0);
    }

    #[test]
    fn spectral_envelope_groups_log_bands() {
        // bins=0 → 空。非 2 冪 fft_size → Err。
        let input = noise(512, 3);
        assert_eq!(
            spectral_envelope(&input, 256, 0).unwrap(),
            Vec::<f64>::new()
        );
        assert_eq!(
            spectral_envelope(&input, 0, 8).err(),
            Some(Error::InvalidValue("fft_size"))
        );
        assert_eq!(
            spectral_envelope(&input, 1000, 8).err(),
            Some(Error::InvalidValue("fft_size"))
        );
        // fft_size < len は len.next_pow2 に拡大。
        let out = spectral_envelope(&input, 64, 8).unwrap();
        assert_eq!(out.len(), 8);
        // 単一周波数の包絡: エネルギーのある区間だけ ln 値が大きい。
        let t = |i: usize| i as f64 / 256.0;
        let sine: Vec<f64> = (0..256)
            .map(|i| (2.0 * core::f64::consts::PI * 8.0 * t(i)).sin())
            .collect();
        let out = spectral_envelope(&sine, 256, 8);
        assert!(out.is_ok());
        let out = out.unwrap();
        let (peak, _) = out
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        // 8 Hz 正弦は非負周波数 bin 8 → 8 区画のうち index 0。
        assert_eq!(peak, 0);
        assert!(out[0] > out[7]);
        // 空入力 → 全区間 ln(1e-12)（AL verbatim）。
        let out = spectral_envelope(&[], 256, 4).unwrap();
        assert!(out.iter().all(|&v| (v - 1e-12_f64.ln()).abs() < 1e-20));
    }

    /// Issue #183: `BandSpectrum::band_into` は同一入力・同一帯域で
    /// `BandFilter::band`/`band_into` の出力と bit-exact に一致する
    ///（同一実装を共有する）。
    #[test]
    fn band_spectrum_matches_band_filter_bit_exact() {
        for len in [0usize, 1, 3, 33, 256, 1027] {
            let input: Vec<_> = (0..len).map(|i| (i as f64 * 0.713).sin()).collect();
            let bands = [
                (0.0, 100.0),
                (100.0, 250.0),
                (0.0, 1025.0),
                (250.0, 250.0),
                (500.0, f64::NAN),
                (0.0, f64::INFINITY),
                (f64::NAN, 300.0),
            ];
            let spectrum = BandSpectrum::new(&input, 2048).unwrap();
            let mut scratch = BandScratch::new(&spectrum);
            let mut filter = BandFilter::new(&input, 2048).unwrap();
            for &(lo, hi) in &bands {
                let mut out_a = vec![1.0; len];
                let mut out_b = vec![2.0; len];
                spectrum
                    .band_into(&mut scratch, lo, hi, &mut out_a)
                    .unwrap();
                filter.band_into(lo, hi, &mut out_b).unwrap();
                assert_eq!(out_a, out_b, "len={len} band=({lo},{hi})");
                assert_eq!(out_a, filter.band(lo, hi), "len={len} band=({lo},{hi})");
            }
        }
    }

    /// Issue #183: 不変 `BandSpectrum` を `&` で複数スレッドから共有し、
    /// スレッドごとの `BandScratch` で帯域抽出しても逐次実行と一致する。
    /// `Send + Sync` 契約はコンパイル自体が保証する（`&spectrum` を
    /// scoped thread に渡せる）。
    #[test]
    fn band_spectrum_shared_across_threads() {
        // Send + Sync の静的保証（auto impl の退行防止）。
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BandSpectrum>();
        assert_send_sync::<BandScratch>();
        let input = noise(1024, 11);
        let spectrum = BandSpectrum::new(&input, 48000).unwrap();
        let bands = [
            (0.0, 100.0),
            (100.0, 250.0),
            (500.0, f64::NAN),
            (20.0, 5000.0),
            (f64::NAN, 40.0),
        ];
        let mut seq_scratch = BandScratch::new(&spectrum);
        let expected: Vec<Vec<f64>> = bands
            .iter()
            .map(|&(lo, hi)| {
                let mut out = vec![0.0; spectrum.len()];
                spectrum
                    .band_into(&mut seq_scratch, lo, hi, &mut out)
                    .unwrap();
                out
            })
            .collect();
        let spectrum = &spectrum;
        let got = std::thread::scope(|s| {
            let handles: Vec<_> = bands
                .iter()
                .map(|&(lo, hi)| {
                    s.spawn(move || {
                        let mut scratch = BandScratch::new(spectrum);
                        let mut out = vec![0.0; spectrum.len()];
                        spectrum.band_into(&mut scratch, lo, hi, &mut out).unwrap();
                        out
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(got, expected);
    }

    /// Issue #183: scratch の整合契約 — 異なる変換長の scratch は
    /// `Error::InvalidValue("scratch")` で拒否し、同じ変換長の
    /// `BandSpectrum` 同士では使い回せる。
    #[test]
    fn band_scratch_contract() {
        let input_a = noise(512, 5);
        let input_b = noise(700, 9);
        let spec_a = BandSpectrum::new(&input_a, 48000).unwrap();
        // 512 → n=512、700 → n=1024。変換長の異なる scratch は拒否。
        let spec_b = BandSpectrum::new(&input_b, 48000).unwrap();
        let mut scratch_a = BandScratch::new(&spec_a);
        let mut out_b = vec![0.0; spec_b.len()];
        assert_eq!(
            spec_b.band_into(&mut scratch_a, 0.0, 1000.0, &mut out_b),
            Err(Error::InvalidValue("scratch"))
        );
        // 同じ n=512 だが len が異なる別スペクトルでは使い回せる。
        let input_c = noise(400, 13);
        let spec_c = BandSpectrum::new(&input_c, 48000).unwrap();
        assert_eq!(spec_c.fft_size(), spec_a.fft_size());
        let mut out_c = vec![0.0; spec_c.len()];
        spec_c
            .band_into(&mut scratch_a, 0.0, 1000.0, &mut out_c)
            .unwrap();
        let mut own = BandScratch::new(&spec_c);
        let mut expected = vec![0.0; spec_c.len()];
        spec_c
            .band_into(&mut own, 0.0, 1000.0, &mut expected)
            .unwrap();
        assert_eq!(out_c, expected);
        // 空スペクトル（n=1）と非空 scratch の組合せも拒否。
        let spec_empty = BandSpectrum::new(&[], 48000).unwrap();
        assert_eq!(
            spec_empty.band_into(&mut scratch_a, 0.0, 1.0, &mut []),
            Err(Error::InvalidValue("scratch"))
        );
        // out 長不一致は scratch 検証より先に "length" を返す。
        assert_eq!(
            spec_b.band_into(&mut scratch_a, 0.0, 1000.0, &mut [0.0; 1]),
            Err(Error::InvalidValue("length"))
        );
        assert!(spec_a.memory_bytes() > 0 && scratch_a.memory_bytes() > 0);
        // BandFilter::memory_bytes は spectrum + scratch の合計と一致。
        let filter = BandFilter::new(&input_a, 48000).unwrap();
        assert_eq!(
            filter.memory_bytes(),
            spec_a.memory_bytes() + BandScratch::new(&spec_a).memory_bytes()
        );
    }
}
