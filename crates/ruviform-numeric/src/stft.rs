//! 複素 STFT と派生スペクトル（Issue #153）。
//!
//! `AudioLens` `al-analysis::stft` の `stft`/`StftMatrix`/`envelope` の移植。
//! [`FftPlan`](crate::fft::FftPlan)・[`Window`](crate::window::Window) の
//! 上に plan/workspace 構造で構築する。
//!
//! # 規約
//!
//! - フレームは `start ∈ {0, hop, 2·hop, …}` で `start + window_len ≤ len`
//!   を満たす先頭側のみ。末尾の余りは捨て、部分フレーム・centering・
//!   overlap-add 正規化は行わない。
//! - 各フレームは `x[i]·w[i]`（`i < window_len`）を `fft_size` まで
//!   ゼロ埋めして forward FFT（無正規化・負符号回転 — `FftPlan` 規約）。
//!   `fft_size` は既定 `next_pow2(window_len)`。
//! - 出力は全 `fft_size` bin の複素スペクトル（負周波数を含む）。
//!   [`StftMatrix`] は frame-major の連続 `frames × fft_size` レイアウト。
//! - PSD は two-sided `|X[k]|² / (fs·Σw²)`。`Σw²` は構築時に窓係数から
//!   実計算するため任意窓で正しい（Hann では `0.375·window_len`）。
//! - フレーム 0 は「信号が窓より短い」を意味する。妥当性の判断は
//!   呼出し側の責務であり、本モジュールは空の [`StftMatrix`] を返す。
//! - [`StftPlan::frame_into`]/[`StftPlan::stft_into`]/
//!   [`StftMatrix`] の `*_into` は **RT-suitable**（alloc なし・有界）。
//!   確保を伴う構築・所有版・[`envelope`] は non-RT。

use crate::fft::{FftPlan, FftWorkspace};
use crate::vec;
use crate::window::Window;
use crate::{Error, Result};

/// 複素 STFT の不変設定（窓係数・hop・FFT plan・正規化係数）。
///
/// 複数の信号・スレッドで `&` 共有できる（排他状態は [`StftWorkspace`]）。
/// メモリ量は [`memory_bytes`](Self::memory_bytes) で構造見積もりを得られる。
pub struct StftPlan {
    /// 解析窓長（サンプル、ゼロ埋め前）。
    window_len: usize,
    /// フレーム間隔（サンプル）。
    hop: usize,
    /// FFT 変換長（`>= window_len` の 2 冪）。
    fft_size: usize,
    /// 信号のサンプルレート（bin 周波数・PSD 正規化に使用）。
    sample_rate: u32,
    /// 窓係数（`window_len` 要素）。
    coeffs: Vec<f64>,
    /// `Σw²`（PSD 正規化係数）。
    window_sum_sq: f64,
    /// フレーム変換用の複素 FFT plan。
    fft: FftPlan,
}

/// 複素 STFT の排他的作業領域。[`FftWorkspace`] を内包する。
pub struct StftWorkspace {
    /// 対応する [`StftPlan::fft_size`]。workspace 照合用。
    fft_size: usize,
    fft_ws: FftWorkspace,
}

impl StftPlan {
    /// `window_len`・`hop`・窓形状 `shape`・`sample_rate` の plan を構築する。
    /// `fft_size` は `next_pow2(window_len)`（AL 規約）。
    ///
    /// **RT 契約:** non-RT only — 係数・plan を確保する。
    ///
    /// # Errors
    ///
    /// `window_len = 0` → `Error::InvalidValue("window_len")`、
    /// `hop = 0` → `Error::InvalidValue("hop")`、
    /// `sample_rate = 0` → `Error::InvalidValue("sample_rate")`。
    /// `window_len.next_power_of_two()` が `usize` を溢れる場合は
    /// `Error::InvalidValue("window_len")`。
    pub fn new(window_len: usize, hop: usize, shape: Window, sample_rate: u32) -> Result<Self> {
        let fft_size = window_len
            .checked_next_power_of_two()
            .ok_or(Error::InvalidValue("window_len"))?;
        Self::with_fft_size(window_len, hop, fft_size, shape, sample_rate)
    }

    /// `fft_size` を明示する版。`fft_size` は `window_len` 以上の 2 冪で
    /// なければならない（大きいほど窓後のゼロ埋めが増える）。
    ///
    /// **RT 契約:** non-RT only — 係数・plan を確保する。
    ///
    /// # Errors
    ///
    /// `new` と同じ条件に加え、`fft_size` が 2 冪でない・
    /// `window_len` 未満なら `Error::InvalidValue("fft_size")`。
    pub fn with_fft_size(
        window_len: usize,
        hop: usize,
        fft_size: usize,
        shape: Window,
        sample_rate: u32,
    ) -> Result<Self> {
        if window_len == 0 {
            return Err(Error::InvalidValue("window_len"));
        }
        if hop == 0 {
            return Err(Error::InvalidValue("hop"));
        }
        if sample_rate == 0 {
            return Err(Error::InvalidValue("sample_rate"));
        }
        if !fft_size.is_power_of_two() || fft_size < window_len {
            return Err(Error::InvalidValue("fft_size"));
        }
        let coeffs = shape.coeffs(window_len);
        let window_sum_sq = vec::sum_sq(&coeffs);
        let fft = FftPlan::new(fft_size)?;
        Ok(Self {
            window_len,
            hop,
            fft_size,
            sample_rate,
            coeffs,
            window_sum_sq,
            fft,
        })
    }

    /// 解析窓長（サンプル）。
    #[must_use]
    pub fn window_len(&self) -> usize {
        self.window_len
    }

    /// フレーム間隔（サンプル）。
    #[must_use]
    pub fn hop(&self) -> usize {
        self.hop
    }

    /// FFT 変換長（出力 bin 数）。
    #[must_use]
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// 信号のサンプルレート。
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 窓係数の二乗和 `Σw²`。
    #[must_use]
    pub fn window_sum_sq(&self) -> f64 {
        self.window_sum_sq
    }

    /// bin 幅（Hz）`sample_rate / fft_size`。
    #[expect(clippy::cast_precision_loss)]
    #[must_use]
    pub fn bin_hz(&self) -> f64 {
        f64::from(self.sample_rate) / self.fft_size as f64
    }

    /// `bin` の中心周波数（Hz）`bin·bin_hz`。
    #[expect(clippy::cast_precision_loss)]
    #[must_use]
    pub fn bin_freq(&self, bin: usize) -> f64 {
        bin as f64 * self.bin_hz()
    }

    /// two-sided PSD の正規化分母 `Σw²·fs`。
    /// 呼出し側が保持する和系列（`CrossSpectrum` 等）を mean PSD に
    /// スケールする際にもこの係数を使う。
    #[must_use]
    pub fn psd_norm(&self) -> f64 {
        self.window_sum_sq * f64::from(self.sample_rate)
    }

    /// `signal_len` サンプルから生成されるフレーム数。
    /// `signal_len ≥ window_len` なら `(signal_len − window_len)/hop + 1`、
    /// それ以外は 0。
    ///
    /// **RT 契約:** RT-suitable — 純粋算術・alloc なし。
    #[must_use]
    pub fn frame_count(&self, signal_len: usize) -> usize {
        if signal_len >= self.window_len {
            (signal_len - self.window_len) / self.hop + 1
        } else {
            0
        }
    }

    /// `frame` 番目のフレームの複素スペクトルを `re`/`im`
    /// （各 `fft_size`）に書き込む。`frame` は
    /// `frame_count(signal.len())` 未満でなければならない。
    ///
    /// **RT 契約:** RT-suitable — 出力バッファを transform の in-place
    /// 作業領域として使い、alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `re`/`im` が `fft_size` でなければ `Error::InvalidValue("length")`、
    /// `frame` が範囲外なら `Error::InvalidValue("frame")`、
    /// workspace が本 plan の `fft_size` 用でなければ
    /// `Error::InvalidValue("workspace")`。
    pub fn frame_into(
        &self,
        workspace: &mut StftWorkspace,
        signal: &[f64],
        frame: usize,
        re: &mut [f64],
        im: &mut [f64],
    ) -> Result<()> {
        if workspace.fft_size != self.fft_size {
            return Err(Error::InvalidValue("workspace"));
        }
        if re.len() != self.fft_size || im.len() != self.fft_size {
            return Err(Error::InvalidValue("length"));
        }
        if frame >= self.frame_count(signal.len()) {
            return Err(Error::InvalidValue("frame"));
        }
        let start = frame * self.hop;
        im.fill(0.0);
        for (i, &w) in self.coeffs.iter().enumerate() {
            re[i] = signal[start + i] * w;
        }
        re[self.window_len..].fill(0.0);
        self.fft.transform(&mut workspace.fft_ws, re, im, false)
    }

    /// 全フレームの複素スペクトルを frame-major の連続 `re`/`im`
    /// （各 `frame_count(signal.len()) × fft_size`）に書き込む。
    /// 大きな信号でも呼出し側の確保済みバッファに直接書くため、
    /// [`stft`](Self::stft) の `StftMatrix` 全保持を回避できる。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `re`/`im` の長さが `frame_count × fft_size` と一致しなければ
    /// `Error::InvalidValue("length")`。workspace 不一致は
    /// `Error::InvalidValue("workspace")`。
    pub fn stft_into(
        &self,
        workspace: &mut StftWorkspace,
        signal: &[f64],
        re: &mut [f64],
        im: &mut [f64],
    ) -> Result<()> {
        if workspace.fft_size != self.fft_size {
            return Err(Error::InvalidValue("workspace"));
        }
        let frames = self.frame_count(signal.len());
        let need = frames * self.fft_size;
        if re.len() != need || im.len() != need {
            return Err(Error::InvalidValue("length"));
        }
        for (f, (re_f, im_f)) in re
            .chunks_exact_mut(self.fft_size)
            .zip(im.chunks_exact_mut(self.fft_size))
            .enumerate()
        {
            // frame_into 相当を in-place で行う（workspace 検査は済み）。
            let start = f * self.hop;
            im_f.fill(0.0);
            for (i, &w) in self.coeffs.iter().enumerate() {
                re_f[i] = signal[start + i] * w;
            }
            re_f[self.window_len..].fill(0.0);
            self.fft
                .transform(&mut workspace.fft_ws, re_f, im_f, false)?;
        }
        Ok(())
    }

    /// 全フレームの [`StftMatrix`] を確保して返す。
    ///
    /// **RT 契約:** non-RT only — `2·frames·fft_size` 要素を確保する。
    ///
    /// # Errors
    ///
    /// workspace が本 plan の `fft_size` 用でなければ
    /// `Error::InvalidValue("workspace")`。
    pub fn stft(&self, workspace: &mut StftWorkspace, signal: &[f64]) -> Result<StftMatrix> {
        let frames = self.frame_count(signal.len());
        let mut re = vec![0.0; frames * self.fft_size];
        let mut im = vec![0.0; frames * self.fft_size];
        self.stft_into(workspace, signal, &mut re, &mut im)?;
        Ok(StftMatrix {
            re,
            im,
            window_len: self.window_len,
            hop: self.hop,
            fft_size: self.fft_size,
            sample_rate: self.sample_rate,
            window_sum_sq: self.window_sum_sq,
        })
    }

    /// plan が保持する係数・twiddle のメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.coeffs.len() * size_of::<f64>() + self.fft.memory_bytes()
    }
}

impl StftWorkspace {
    /// `plan` 用の作業領域を確保する（`fft_size >= 16384` のみ
    /// `FftWorkspace` 内部で大きな stage バッファを持つ）。
    ///
    /// **RT 契約:** non-RT only — `FftWorkspace` を確保する。
    #[must_use]
    pub fn new(plan: &StftPlan) -> Self {
        Self {
            fft_size: plan.fft_size,
            fft_ws: FftWorkspace::new(&plan.fft),
        }
    }

    /// 保持する作業領域のメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.fft_ws.memory_bytes()
    }
}

/// plan・workspace・出力を内蔵する one-shot 用の所有型
/// （[`crate::fft::Fft`] と同じ便宜層）。
pub struct Stft {
    plan: StftPlan,
    ws: StftWorkspace,
}

impl Stft {
    /// [`StftPlan::new`] + [`StftWorkspace::new`]。
    ///
    /// **RT 契約:** non-RT only — 係数・plan・workspace を確保する。
    ///
    /// # Errors
    ///
    /// [`StftPlan::new`] と同じ。
    pub fn new(window_len: usize, hop: usize, shape: Window, sample_rate: u32) -> Result<Self> {
        let plan = StftPlan::new(window_len, hop, shape, sample_rate)?;
        let ws = StftWorkspace::new(&plan);
        Ok(Self { plan, ws })
    }

    /// [`StftPlan::with_fft_size`] + [`StftWorkspace::new`]。
    ///
    /// **RT 契約:** non-RT only — 係数・plan・workspace を確保する。
    ///
    /// # Errors
    ///
    /// [`StftPlan::with_fft_size`] と同じ。
    pub fn with_fft_size(
        window_len: usize,
        hop: usize,
        fft_size: usize,
        shape: Window,
        sample_rate: u32,
    ) -> Result<Self> {
        let plan = StftPlan::with_fft_size(window_len, hop, fft_size, shape, sample_rate)?;
        let ws = StftWorkspace::new(&plan);
        Ok(Self { plan, ws })
    }

    /// 内蔵 plan。
    #[must_use]
    pub fn plan(&self) -> &StftPlan {
        &self.plan
    }

    /// [`StftPlan::frame_count`]。
    ///
    /// **RT 契約:** RT-suitable — 純粋算術・alloc なし。
    #[must_use]
    pub fn frame_count(&self, signal_len: usize) -> usize {
        self.plan.frame_count(signal_len)
    }

    /// [`StftPlan::frame_into`]。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// [`StftPlan::frame_into`] と同じ（workspace 照合は内蔵で必ず一致）。
    pub fn frame_into(
        &mut self,
        signal: &[f64],
        frame: usize,
        re: &mut [f64],
        im: &mut [f64],
    ) -> Result<()> {
        self.plan.frame_into(&mut self.ws, signal, frame, re, im)
    }

    /// [`StftPlan::stft_into`]。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// [`StftPlan::stft_into`] と同じ。
    pub fn transform_into(&mut self, signal: &[f64], re: &mut [f64], im: &mut [f64]) -> Result<()> {
        self.plan.stft_into(&mut self.ws, signal, re, im)
    }

    /// [`StftPlan::stft`]。
    ///
    /// **RT 契約:** non-RT only — `2·frames·fft_size` 要素を確保する。
    pub fn transform(&mut self, signal: &[f64]) -> Result<StftMatrix> {
        self.plan.stft(&mut self.ws, signal)
    }
}

/// 全フレームの複素スペクトル行列（frame-major `frames × fft_size`）。
///
/// `re[f·fft_size + k]` がフレーム `f`・bin `k` の実部。
/// AL の `StftMatrix { re: Vec<Vec<f64>> }` を連続レイアウトに置き換えた
/// もので、bin 規約・正規化は同一。
#[derive(Debug, Clone)]
pub struct StftMatrix {
    /// 実部（frame-major `frames × fft_size`）。
    re: Vec<f64>,
    /// 虚部（同レイアウト）。
    im: Vec<f64>,
    /// 解析窓長（サンプル、ゼロ埋め前）。
    window_len: usize,
    /// フレーム間隔（サンプル）。
    hop: usize,
    /// FFT 変換長（bin 数）。
    fft_size: usize,
    /// 信号のサンプルレート。
    sample_rate: u32,
    /// `Σw²`。
    window_sum_sq: f64,
}

impl StftMatrix {
    /// フレーム数。
    #[must_use]
    pub fn frames(&self) -> usize {
        self.re.len() / self.fft_size
    }

    /// bin 数（=`fft_size`）。
    #[must_use]
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// 解析窓長（サンプル）。
    #[must_use]
    pub fn window_len(&self) -> usize {
        self.window_len
    }

    /// フレーム間隔（サンプル）。
    #[must_use]
    pub fn hop(&self) -> usize {
        self.hop
    }

    /// 信号のサンプルレート。
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// `Σw²`。
    #[must_use]
    pub fn window_sum_sq(&self) -> f64 {
        self.window_sum_sq
    }

    /// bin 幅（Hz）`sample_rate / fft_size`。
    #[expect(clippy::cast_precision_loss)]
    #[must_use]
    pub fn bin_hz(&self) -> f64 {
        f64::from(self.sample_rate) / self.fft_size as f64
    }

    /// `bin` の中心周波数（Hz）。
    #[expect(clippy::cast_precision_loss)]
    #[must_use]
    pub fn bin_freq(&self, bin: usize) -> f64 {
        bin as f64 * self.bin_hz()
    }

    /// two-sided PSD の正規化分母 `Σw²·fs`。
    #[must_use]
    pub fn psd_norm(&self) -> f64 {
        self.window_sum_sq * f64::from(self.sample_rate)
    }

    /// 実部の連続スライス（frame-major）。
    #[must_use]
    pub fn re(&self) -> &[f64] {
        &self.re
    }

    /// 虚部の連続スライス（frame-major）。
    #[must_use]
    pub fn im(&self) -> &[f64] {
        &self.im
    }

    /// `f` 番目フレームの実部 `fft_size` 要素。
    /// `f >= frames` なら `None`。
    #[must_use]
    pub fn frame_re(&self, f: usize) -> Option<&[f64]> {
        self.re.chunks_exact(self.fft_size).nth(f)
    }

    /// `f` 番目フレームの虚部 `fft_size` 要素。
    /// `f >= frames` なら `None`。
    #[must_use]
    pub fn frame_im(&self, f: usize) -> Option<&[f64]> {
        self.im.chunks_exact(self.fft_size).nth(f)
    }

    /// power 行列（`|z|²`）を frame-major `frames × fft_size` の
    /// `out` に書き込む。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `out.len()` が `frames × fft_size` と異なれば
    /// `Error::InvalidValue("length")`。
    pub fn power_into(&self, out: &mut [f64]) -> Result<()> {
        if out.len() != self.re.len() {
            return Err(Error::InvalidValue("length"));
        }
        vec::mag_sq(&self.re, &self.im, out)
    }

    /// [`power_into`] の所有版。
    ///
    /// **RT 契約:** non-RT only — 確保する。
    #[must_use]
    pub fn power(&self) -> Vec<f64> {
        let mut out = vec![0.0; self.re.len()];
        // バッファは同一長に構築済みのため失敗しない。
        let _ = self.power_into(&mut out);
        out
    }

    /// magnitude 行列（`|z|`）を `out` に書き込む。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `out.len()` が `frames × fft_size` と異なれば
    /// `Error::InvalidValue("length")`。
    pub fn magnitude_into(&self, out: &mut [f64]) -> Result<()> {
        self.power_into(out)?;
        for v in out.iter_mut() {
            *v = v.sqrt();
        }
        Ok(())
    }

    /// [`magnitude_into`] の所有版。
    ///
    /// **RT 契約:** non-RT only — 確保する。
    #[must_use]
    pub fn magnitude(&self) -> Vec<f64> {
        let mut out = vec![0.0; self.re.len()];
        let _ = self.magnitude_into(&mut out);
        out
    }

    /// two-sided PSD 行列 `|z|² / (Σw²·fs)` を `out` に書き込む。
    /// 全 bin の和 × `bin_hz` がおおよそ信号パワーとなる
    /// （AL `StftMatrix::psd` と同じ正規化）。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `out.len()` が `frames × fft_size` と異なれば
    /// `Error::InvalidValue("length")`。
    pub fn psd_into(&self, out: &mut [f64]) -> Result<()> {
        self.power_into(out)?;
        let inv = 1.0 / self.psd_norm();
        for v in out.iter_mut() {
            *v *= inv;
        }
        Ok(())
    }

    /// [`psd_into`] の所有版。
    ///
    /// **RT 契約:** non-RT only — 確保する。
    #[must_use]
    pub fn psd(&self) -> Vec<f64> {
        let mut out = vec![0.0; self.re.len()];
        let _ = self.psd_into(&mut out);
        out
    }

    /// 保持データのメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        (self.re.len() + self.im.len()) * size_of::<f64>()
    }
}

/// `samples` の複素 STFT を返す one-shot 版。
/// [`Stft::new`] + [`Stft::transform`] と同等（plan・workspace・出力を確保）。
///
/// **RT 契約:** non-RT only — 確保する。
///
/// # Errors
///
/// [`StftPlan::new`] と同じ。
///
/// # Examples
///
/// ```
/// use ruviform_numeric::stft::stft;
/// use ruviform_numeric::window::Window;
///
/// // 8 Hz の正弦を 64 Hz で標本化: 窓 32・hop 16 → fft_size 32。
/// let rate = 64;
/// let samples: Vec<f64> = (0..96)
///     .map(|i| (2.0 * std::f64::consts::PI * 8.0 * i as f64 / rate as f64).sin())
///     .collect();
/// let m = stft(&samples, 32, 16, Window::Hann, rate).unwrap();
/// assert_eq!(m.frames(), 5);
/// let mag = m.magnitude();
/// let peak = (0..m.fft_size() / 2 + 1)
///     .max_by(|&a, &b| mag[a].total_cmp(&mag[b]))
///     .unwrap();
/// assert_eq!(peak, 4); // 8 Hz / 2 Hz-per-bin
/// ```
pub fn stft(
    samples: &[f64],
    window_len: usize,
    hop: usize,
    shape: Window,
    sample_rate: u32,
) -> Result<StftMatrix> {
    Stft::new(window_len, hop, shape, sample_rate)?.transform(samples)
}

/// magnitude 行の中心移動平均スムーザ（AL `stft::envelope` の移植）。
/// `out[i] = mean(row[max(0, i−bins/2) .. min(len, i+bins/2+1)])`。
/// `bins = 0` は 1 として扱う（出力は入力のコピー）。`bins` が偶数のとき
/// 実効窓幅は `bins+1` になる（AL verbatim の丸め規則）。
///
/// **RT 契約:** non-RT only — 出力を確保する。
#[expect(clippy::cast_precision_loss)]
#[must_use]
pub fn envelope(magnitude_row: &[f64], bins: usize) -> Vec<f64> {
    let bins = bins.max(1);
    (0..magnitude_row.len())
        .map(|i| {
            let lo = i.saturating_sub(bins / 2);
            let hi = (i + bins / 2 + 1).min(magnitude_row.len());
            magnitude_row[lo..hi].iter().sum::<f64>() / (hi - lo) as f64
        })
        .collect()
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    /// naive O(n²) DFT oracle: `X[k] = Σ x[i]·w[i]·e^(−2πi·k·i/N)`。
    /// 実装を共有しない独立評価。
    fn naive_dft(
        samples: &[f64],
        start: usize,
        coeffs: &[f64],
        fft_size: usize,
    ) -> Vec<(f64, f64)> {
        (0..fft_size)
            .map(|k| {
                let mut re = 0.0;
                let mut im = 0.0;
                for (i, &c) in coeffs.iter().enumerate() {
                    let x = samples[start + i] * c;
                    let ang = -2.0 * core::f64::consts::PI * (k * i) as f64 / fft_size as f64;
                    re += x * ang.cos();
                    im += x * ang.sin();
                }
                (re, im)
            })
            .collect()
    }

    /// 決定的な疑似実信号（複数正弦 + LCG 疑似ノイズ）。
    fn fixture(n: usize, rate: u32) -> Vec<f64> {
        let mut s = 0x9e37_79b9_u64;
        (0..n)
            .map(|i| {
                s = s
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let noise = ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0;
                let t = i as f64 / f64::from(rate);
                (2.0 * core::f64::consts::PI * 1000.0 * t).sin()
                    + 0.3 * (2.0 * core::f64::consts::PI * 3000.0 * t).sin()
                    + 0.1 * noise
            })
            .collect()
    }

    #[test]
    fn stft_places_sine_in_the_right_bin() {
        let rate = 48000;
        let samples = fixture(2048, rate);
        let m = stft(&samples, 1024, 512, Window::Hann, rate).unwrap();
        assert_eq!(m.frames(), 3);
        assert_eq!(m.fft_size(), 1024);
        let mag = m.magnitude();
        let (bin, _) = mag[..=m.fft_size() / 2]
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        // 主成分は振幅 1.0 の 1000 Hz → 46.875 Hz/bin で bin 21/22。
        assert!((21..=22).contains(&bin), "peak bin {bin}");
    }

    #[test]
    fn frames_match_naive_dft_and_fft_size_padding() {
        // 非 2 冪窓は next_pow2 へゼロ埋め。全 bin を naive DFT と照合。
        let samples = fixture(1500, 48000);
        let plan = StftPlan::new(1500, 500, Window::Hann, 48000).unwrap();
        assert_eq!(plan.fft_size(), 2048);
        let mut ws = StftWorkspace::new(&plan);
        let m = plan.stft(&mut ws, &samples).unwrap();
        assert_eq!(m.frames(), 1);
        let coeffs = Window::Hann.coeffs(1500);
        let oracle = naive_dft(&samples, 0, &coeffs, 2048);
        for (k, &(ore, oim)) in oracle.iter().enumerate() {
            assert!(
                (m.re()[k] - ore).abs() < 1e-8,
                "re[{k}]: {} vs {ore}",
                m.re()[k]
            );
            assert!(
                (m.im()[k] - oim).abs() < 1e-8,
                "im[{k}]: {} vs {oim}",
                m.im()[k]
            );
        }
    }

    #[test]
    fn frame_count_and_edge_boundaries() {
        let plan = StftPlan::new(1024, 512, Window::Hann, 48000).unwrap();
        assert_eq!(plan.frame_count(0), 0);
        assert_eq!(plan.frame_count(1023), 0);
        assert_eq!(plan.frame_count(1024), 1);
        assert_eq!(plan.frame_count(1535), 1);
        assert_eq!(plan.frame_count(1536), 2);
        assert_eq!(plan.frame_count(2048), 3);
        // 0 フレームの stft_into は空出力を要求し、範囲外 frame は Err。
        let mut ws = StftWorkspace::new(&plan);
        plan.stft_into(&mut ws, &[0.0; 10], &mut [], &mut [])
            .unwrap();
        assert_eq!(
            plan.stft_into(&mut ws, &[0.0; 10], &mut [0.0; 1], &mut [0.0; 1]),
            Err(Error::InvalidValue("length"))
        );
        assert_eq!(
            plan.frame_into(&mut ws, &[0.0; 10], 0, &mut [0.0; 1024], &mut [0.0; 1024]),
            Err(Error::InvalidValue("frame"))
        );
        // stft() の空行列はメタデータを保持する。
        let m = plan.stft(&mut ws, &[0.0; 10]).unwrap();
        assert_eq!(m.frames(), 0);
        assert_eq!(m.fft_size(), 1024);
        assert_eq!(m.window_len(), 1024);
        assert!(m.frame_re(0).is_none());
    }

    #[test]
    fn frame_into_stft_into_and_owned_paths_agree() {
        let samples = fixture(3000, 48000);
        let plan = StftPlan::new(1024, 512, Window::Hann, 48000).unwrap();
        let mut ws = StftWorkspace::new(&plan);
        let m = plan.stft(&mut ws, &samples).unwrap();
        let frames = m.frames();
        let mut re = vec![0.0; frames * 1024];
        let mut im = vec![0.0; frames * 1024];
        plan.stft_into(&mut ws, &samples, &mut re, &mut im).unwrap();
        assert_eq!(re, m.re()[..]);
        assert_eq!(im, m.im()[..]);
        // frame_into も同一。
        let mut fre = [0.0; 1024];
        let mut fim = [0.0; 1024];
        for f in 0..frames {
            plan.frame_into(&mut ws, &samples, f, &mut fre, &mut fim)
                .unwrap();
            assert_eq!(fre[..], m.frame_re(f).unwrap()[..]);
            assert_eq!(fim[..], m.frame_im(f).unwrap()[..]);
        }
        // Stft 所有型・one-shot も同一。
        let mut owned = Stft::new(1024, 512, Window::Hann, 48000).unwrap();
        let m2 = owned.transform(&samples).unwrap();
        assert_eq!(m2.re()[..], m.re()[..]);
        let m3 = stft(&samples, 1024, 512, Window::Hann, 48000).unwrap();
        assert_eq!(m3.re()[..], m.re()[..]);
        // with_fft_size で大きな fft_size は長いゼロ埋めの別規約。
        let wide = StftPlan::with_fft_size(1024, 512, 2048, Window::Hann, 48000).unwrap();
        assert_eq!(wide.fft_size(), 2048);
        assert_eq!(
            StftPlan::with_fft_size(1024, 512, 512, Window::Hann, 48000).err(),
            Some(Error::InvalidValue("fft_size"))
        );
        assert_eq!(
            StftPlan::with_fft_size(1024, 512, 1500, Window::Hann, 48000).err(),
            Some(Error::InvalidValue("fft_size"))
        );
    }

    #[test]
    fn constructor_validation() {
        assert_eq!(
            StftPlan::new(0, 512, Window::Hann, 48000).err(),
            Some(Error::InvalidValue("window_len"))
        );
        assert_eq!(
            StftPlan::new(1024, 0, Window::Hann, 48000).err(),
            Some(Error::InvalidValue("hop"))
        );
        assert_eq!(
            StftPlan::new(1024, 512, Window::Hann, 0).err(),
            Some(Error::InvalidValue("sample_rate"))
        );
    }

    #[test]
    fn hann_sum_sq_and_psd_parseval() {
        // Σw²(Hann, N) = 0.375·N（解析解）。
        for n in [8usize, 1024, 4096] {
            let plan = StftPlan::new(n, n, Window::Hann, 48000).unwrap();
            assert!((plan.window_sum_sq() - 0.375 * n as f64).abs() < 1e-12 * n as f64);
        }
        // 単位正弦の PSD 総和 ≈ パワー 0.5（AL テストと同じ Parseval 検査）。
        let rate = 48000;
        let samples: Vec<f64> = (0..2048)
            .map(|i| (2.0 * core::f64::consts::PI * 1000.0 * f64::from(i) / f64::from(rate)).sin())
            .collect();
        let m = stft(&samples, 2048, 2048, Window::Hann, rate).unwrap();
        let psd = m.psd();
        let total: f64 = psd.iter().sum::<f64>() * m.bin_hz();
        assert!((total - 0.5).abs() < 0.15, "total power {total}");
    }

    #[test]
    fn power_magnitude_psd_into_match_owned() {
        let samples = fixture(2048, 48000);
        let mat = stft(&samples, 1024, 512, Window::Hann, 48000).unwrap();
        let total = mat.frames() * mat.fft_size();
        let mut pow = vec![0.0; total];
        let mut mag = vec![0.0; total];
        let mut psd = vec![0.0; total];
        mat.power_into(&mut pow).unwrap();
        mat.magnitude_into(&mut mag).unwrap();
        mat.psd_into(&mut psd).unwrap();
        assert_eq!(pow, mat.power());
        assert_eq!(mag, mat.magnitude());
        assert_eq!(psd, mat.psd());
        // magnitude² = power、psd = power / (Σw²·fs)。
        for k in 0..total.min(64) {
            assert!((mag[k] * mag[k] - pow[k]).abs() < 1e-12 * pow[k].max(1.0));
            assert!((psd[k] - pow[k] / mat.psd_norm()).abs() < 1e-20 * pow[k].max(1.0));
        }
        assert_eq!(
            mat.power_into(&mut vec![0.0; total + 1]),
            Err(Error::InvalidValue("length"))
        );
    }

    #[test]
    fn workspace_mismatch_and_shape_variants() {
        let a = StftPlan::new(1024, 512, Window::Hann, 48000).unwrap();
        let b = StftPlan::new(2048, 1024, Window::Hann, 48000).unwrap();
        let mut ws_b = StftWorkspace::new(&b);
        let signal = vec![0.0; 2048];
        assert_eq!(
            a.stft(&mut ws_b, &signal).err(),
            Some(Error::InvalidValue("workspace"))
        );
        // 同じ fft_size の plan 間では workspace を共用できる（conv と同規則）。
        let c = StftPlan::new(512, 256, Window::Rectangular, 48000).unwrap();
        let d = StftPlan::with_fft_size(256, 128, 512, Window::Hann, 48000).unwrap();
        let mut ws_d = StftWorkspace::new(&d);
        c.stft(&mut ws_d, &signal).unwrap();
        // Rectangular → 係数は全 1、Σw² = window_len。
        assert!((c.window_sum_sq() - 512.0).abs() < 1e-12);
    }

    #[test]
    fn envelope_centers_and_edges() {
        // bins=0/1 → コピー。
        let row = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(envelope(&row, 0), row.to_vec());
        assert_eq!(envelope(&row, 1), row.to_vec());
        // bins=2 → 実効幅 3（i−1 .. i+2 の切詰め）。
        let out = envelope(&row, 2);
        assert_eq!(out[0], 1.5);
        assert_eq!(out[1], 2.0);
        assert_eq!(out[3], 3.5);
        // 空入力 → 空。
        assert_eq!(envelope(&[], 8), Vec::<f64>::new());
        // 巨大 bins → 全域平均。
        let out = envelope(&row, 100);
        assert!(out.iter().all(|&v| (v - 2.5).abs() < 1e-15));
    }
}
