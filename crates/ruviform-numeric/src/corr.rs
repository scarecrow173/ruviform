//! `f64` 相互相関・NCC・GCC-PHAT・overlap 相関（AudioLens `al-math::correlation`
//! の移植。Issue #152）。
//!
//! [`CorrelationPlan`] は template-in-signal の有効ラグ域 `0..=S−L` 上で
//! 符号付き NCC（[`CorrelationPlan::ncc`]）・生相互相関
//! （[`CorrelationPlan::cross_correlation`]）・GCC-PHAT
//! （[`CorrelationPlan::gcc_phat`]、Full mode plan のみ）を提供する。
//! [`OverlapCorrPlan`] は等長窓の両側ラグ `−max_lag..=+max_lag` で
//! overlap 区間の非中心化正規化相関（区間内 `corr0` と同一式）を返す。
//!
//! # 規約
//!
//! - plan（不変・`&` 共有可能）/ workspace（`&mut` 排他・全バッファ事前確保）/
//!   one-shot free fn の三層。カーブ系 API（`ncc`/`gcc_phat`/
//!   `cross_correlation`/`curve`）は事前確保済み workspace のみで動く
//!   **RT-suitable**。plan・workspace 構築と free fn は **non-RT**。
//! - カーブは workspace 内バッファへの借用として返し、借用中は
//!   `&mut workspace` が排他になる（再呼出しは型で禁止）。
//! - `max_lag` は呼出時に plan の無汚染ラグ上限へクランプされる
//!   （エラーではない）。戻り長は `min(max_lag, bound) + 1`。
//! - 正規化は `e > 0` 判定のみで行い、ゼロ/負/NaN エネルギーは `0.0` を返す。
//!   「無相関で 0」と「無音窓で 0」を区別するため、NCC は
//!   [`NccCurve`] で窓エネルギー・template エネルギーを、overlap 相関は
//!   [`OverlapCurve`] でラグごとの `Ex·Ey` 積を証拠として同時に返す。
//! - 退化 shape（`template_len == 0` または `signal_len < template_len`、
//!   `window_len == 0`）は `n = 0` の退化 plan となり、エラーではなく
//!   空カーブ（overlap は `2·max_lag+1` の全 `0.0`）を返す。
//! - NaN/Inf は IEEE どおり伝播する。証拠フィールドは NaN を `max` で
//!   隠さない（`window_energy[k]` が NaN なら当該窓に NaN 入力が存在する）。
//!
//! # 例
//!
//! ```
//! use ruviform_numeric::corr::{CorrelationPlan, CorrelationWorkspace};
//!
//! let template = [0.5, -1.0, 0.5];
//! let mut signal = [0.0; 8];
//! signal[3..6].copy_from_slice(&template);
//! let plan = CorrelationPlan::new_ncc(3, 8)?;
//! let mut ws = CorrelationWorkspace::new(&plan);
//! let curve = plan.ncc(&mut ws, &template, &signal, usize::MAX)?;
//! assert!((curve.values[3] - 1.0).abs() < 1e-12);
//! assert_eq!(curve.values.len(), 8 - 3 + 1);
//! # Ok::<(), ruviform_numeric::Error>(())
//! ```

use crate::fft::{RealFftPlan, RealFftWorkspace};
use crate::vec::{axpy_seq, corr0_parts_seq, sum_sq_seq};
use crate::{Error, Result};

/// plan が保証する演算範囲。`Full` は線形相関全域をカバーする FFT 長
/// `nextpow2(S+L−1)` で NCC/PHAT/生相互相関のすべてを実行できる。
/// `NccValid` は有効ラグ域 `0..=S−L` だけを対象に FFT 長を `nextpow2(S)`
/// へ短縮する（返却 index `k ≤ S−L` は `c[k−n]`・`c[k+n]` の wraparound 項が
/// いずれも定義域外のため `n ≥ S` で無汚染）。`NccDirect` は同じ有効ラグ域を
/// FFT なしの直接計算で求める。PHAT の bin 正規化は FFT 長依存なので
/// `NccValid`/`NccDirect` plan での `gcc_phat` は契約違反。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlanMode {
    Full,
    NccValid,
    NccDirect,
}

/// `run` の内部出力種別。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kernel {
    /// 窓エネルギーで正規化した符号付き NCC。
    Ncc,
    /// bin ごとの相互スペクトル大きさで白色化する GCC-PHAT。
    Phat,
    /// 正規化なしの線形相互相関分子 `c[k] = Σᵢ template[i]·signal[k+i]`。
    Raw,
}

/// `NccDirect` を選ぶ形状判定パラメータ。有効ラグ数 `K = S−L+1` が
/// 小さいほど分子の直接計算 `L·K` MAC が 3 回 FFT の `n·log2(n)` より安い。
/// `K ≤ 2048` の上限は acc ベクトルを ~16KB（L1）に留め、大 K 形状での
/// キャッシュ劣化を構造的に排除する。係数は al-math 校正値を継承し、
/// scalar 実装での再測定を #152 比較記録で確認する。
const DIRECT_NCC_MAX_LAGS: usize = 2048;
const DIRECT_NCC_COST_RATIO: f64 = 8.0;

/// `OverlapCorrPlan` の direct/FFT ルーティング係数。直接法は
/// `~3·out_len·L` flop（`corr0` の 3 累積）、FFT 法は
/// `~15·n·log2(n)` flop（3 変換 + 合成）として仕事量を比較する。
const OVERLAP_FFT_COST_FACTOR: usize = 15;

/// `NccDirect` を選ぶ形状判定。
#[expect(clippy::cast_precision_loss)]
fn prefers_direct(template_len: usize, signal_len: usize, n: usize) -> bool {
    let k = (signal_len - template_len + 1) as f64;
    k <= DIRECT_NCC_MAX_LAGS as f64
        && (template_len as f64 * k) <= DIRECT_NCC_COST_RATIO * (n as f64) * (n as f64).log2()
}

/// `conj(a)·b` を `a` へ書き戻す。`phat` なら bin ごとの
/// `|S[k]|² = re²+im²` で re/im 両成分を同一規則で白色化する
/// （`energy` は `a_re` と等長の scratch）。
fn cross_spectrum(
    a_re: &mut [f64],
    a_im: &mut [f64],
    b_re: &[f64],
    b_im: &[f64],
    phat: bool,
    energy: &mut [f64],
) {
    for (((ar, ai), &br), &bi) in a_re.iter_mut().zip(a_im.iter_mut()).zip(b_re).zip(b_im) {
        let re = *ar * br + *ai * bi;
        let im = *ar * bi - *ai * br;
        *ar = re;
        *ai = im;
    }
    if phat {
        for ((&ar, &ai), sq) in a_re.iter().zip(a_im.iter()).zip(energy.iter_mut()) {
            *sq = ar * ar + ai * ai;
        }
        normalize(a_re, energy, 1.0);
        normalize(a_im, energy, 1.0);
    }
}

/// direct 経路の 1 ラグ値 `(value, Ex·Ey)`。`|lag| ≥ len` は `(0.0, 0.0)`。
fn overlap_direct(x: &[f64], y: &[f64], lag: isize, len: usize) -> (f64, f64) {
    let a = lag.unsigned_abs();
    if a >= len {
        return (0.0, 0.0);
    }
    let (xs, ys) = if lag >= 0 {
        (&x[..len - a], &y[a..])
    } else {
        (&x[a..], &y[..len - a])
    };
    let (sxy, ex, ey) = corr0_parts_seq(xs, ys);
    (
        if ex > 0.0 && ey > 0.0 {
            sxy / (ex * ey).sqrt()
        } else {
            0.0
        },
        ex * ey,
    )
}

/// FFT 経路の 1 ラグ値 `(value, Ex·Ey)`。`corr` は線形相互相関
/// （`real` の逆変換結果。負 lag は巡回 index `n−a`）、
/// `xp`/`yp` は prefix 二乗和。`|lag| ≥ len` は `(0.0, 0.0)`。
fn overlap_fft(
    corr: &[f64],
    xp: &[f64],
    yp: &[f64],
    n: usize,
    lag: isize,
    len: usize,
) -> (f64, f64) {
    let a = lag.unsigned_abs();
    if a >= len {
        return (0.0, 0.0);
    }
    let (num, ex, ey) = if lag >= 0 {
        // x[..len−a] vs y[a..len]
        (corr[a], xp[len - a], yp[len] - yp[a])
    } else {
        // x[a..len] vs y[..len−a]、分子は巡回 index n−a
        (corr[n - a], xp[len] - xp[a], yp[len - a])
    };
    (
        if ex > 0.0 && ey > 0.0 {
            num / (ex * ey).sqrt()
        } else {
            0.0
        },
        ex * ey,
    )
}

/// `v /= template_norm · √e`（`e > 0` の要素のみ。それ以外は `0.0`）。
fn normalize(values: &mut [f64], energy: &[f64], template_norm: f64) {
    for (v, &e) in values.iter_mut().zip(energy) {
        *v = if e > 0.0 {
            *v / (template_norm * e.sqrt())
        } else {
            0.0
        };
    }
}

/// 固定入力形状の不変相関 plan。NCC・GCC-PHAT・生相互相関は実行時に選ぶ。
///
/// - [`CorrelationPlan::new`]: Full mode。FFT 長 `nextpow2(S+L−1)` で
///   全 API（`ncc`/`gcc_phat`/`cross_correlation`）を提供する。
/// - [`CorrelationPlan::new_ncc`]: `NccValid`（FFT 長 `nextpow2(S)`）または
///   `NccDirect`（小さい有効ラグ域で FFT を持たない直接計算）。
///   `ncc`/`cross_correlation` のみ（`gcc_phat` は `Err`）。
///
/// `template_len == 0` または `signal_len < template_len` は `n = 0` の
/// 退化 plan で、全カーブ API が空カーブを返す。
pub struct CorrelationPlan {
    template_len: usize,
    signal_len: usize,
    fft: Option<RealFftPlan>,
    n: usize,
    mode: PlanMode,
}

/// 相関の排他的作業領域。すべての確保は constructor が所有する。
///
/// workspace は構築時の plan mode に紐づく: 同じ `(template_len, signal_len)`
/// でも mode が異なる plan とは互換でない（`Err("workspace")`）。
pub struct CorrelationWorkspace {
    template_len: usize,
    signal_len: usize,
    mode: PlanMode,
    fft: Option<RealFftWorkspace>,
    real: Vec<f64>,
    a_re: Vec<f64>,
    a_im: Vec<f64>,
    b_re: Vec<f64>,
    b_im: Vec<f64>,
    cumulative: Vec<f64>,
    norms: Vec<f64>,
    phat_energy: Vec<f64>,
}

/// [`CorrelationPlan::ncc`] の返却。カーブ値と正規化で使ったエネルギー証拠。
/// いずれも workspace 内バッファへの借用で、`'w` 生存中は workspace を
/// 再使用できない。
#[non_exhaustive]
pub struct NccCurve<'w> {
    /// `values[k]` = lag `k`（`signal[k..k+L]`）の符号付き NCC。
    /// 範囲は `0..=min(max_lag, S−L)`。`window_energy[k] <= 0` または
    /// `template_energy == 0` の要素は `0.0`。
    pub values: &'w [f64],
    /// `window_energy[k]` = 正規化で実際に使った
    /// `max(0, Σⱼ signal[k+j]²)`。`values[k]` が `0.0` の原因判別
    /// （無相関 vs 無音窓）に使う。NaN は伝播（当該窓に NaN 入力が存在）。
    pub window_energy: &'w [f64],
    /// `Σ template²`。`0` なら `values` は全 `0.0`。
    pub template_energy: f64,
}

/// 等長窓の両側 lag 相関の不変 plan。shape キーは窓長のみ。
///
/// [`CorrelationPlan`]（片側スライドの固定長正規化）とは別の演算契約:
/// `lag ∈ [−max_lag, +max_lag]` の各点で overlap 長 `L−|lag|` の
/// 非中心化正規化相関（overlap 区間の `corr0`）を返す。
pub struct OverlapCorrPlan {
    window_len: usize,
    fft: Option<RealFftPlan>,
    n: usize,
}

/// [`OverlapCorrPlan::curve`] の返却。カーブ値とラグごとのエネルギー証拠。
#[non_exhaustive]
pub struct OverlapCurve<'w> {
    /// `values[i]` = `lag = i − max_lag` の非中心化正規化相関
    /// （overlap 区間の `corr0`）。`|lag| ≥ window_len` または
    /// `overlap_energy[i] <= 0` の要素は `0.0`。
    pub values: &'w [f64],
    /// `overlap_energy[i]` = その lag の overlap 区間の `Ex·Ey` 積。
    /// `values[i]` が実商となるのは各因子が正のときのみ（両因子が正なら
    /// 積も正）。`|lag| ≥ window_len` では `0.0`。NaN は伝播。
    /// 有効 support（overlap 標本数）は `window_len − |lag|`。
    pub overlap_energy: &'w [f64],
}

/// `OverlapCorrPlan` の排他的作業領域。すべての確保は constructor が所有する。
pub struct OverlapCorrWorkspace {
    window_len: usize,
    fft: Option<RealFftWorkspace>,
    real: Vec<f64>,
    a_re: Vec<f64>,
    a_im: Vec<f64>,
    b_re: Vec<f64>,
    b_im: Vec<f64>,
    x_prefix: Vec<f64>,
    y_prefix: Vec<f64>,
    curve: Vec<f64>,
    overlap_energy: Vec<f64>,
}

impl CorrelationPlan {
    /// `template_len`/`signal_len` の固定形状 plan（Full mode）。
    /// NCC・GCC-PHAT・生相互相関のすべてを実行できる（FFT 長
    /// `nextpow2(S+L−1)` は線形相関全域をカバーする）。NCC・生相互相関
    /// しか使わない経路は [`Self::new_ncc`] の方が FFT 長を短縮できる。
    ///
    /// `template_len == 0` または `signal_len < template_len` では
    /// `n = 0` の退化 plan になり、全カーブ API は空カーブを返す。
    ///
    /// **RT 契約:** non-RT only — FFT plan を確保する。
    ///
    /// # Errors
    ///
    /// FFT 長の算術が overflow すれば `Error::InvalidValue("length")`。
    pub fn new(template_len: usize, signal_len: usize) -> Result<Self> {
        let n = if template_len == 0 || signal_len < template_len {
            0
        } else {
            signal_len
                .checked_add(template_len - 1)
                .and_then(usize::checked_next_power_of_two)
                .ok_or(Error::InvalidValue("length"))?
        };
        Ok(Self {
            template_len,
            signal_len,
            fft: (n != 0).then(|| RealFftPlan::new(n)).transpose()?,
            n,
            mode: PlanMode::Full,
        })
    }

    /// 有効ラグ域 `0..=signal_len − template_len` の NCC・生相互相関専用 plan。
    /// FFT 長は `nextpow2(signal_len)` で、返却される有効ラグ値は
    /// [`Self::new`] と同一契約。有効ラグ数 `K = S−L+1` が小さい形状では
    /// FFT を持たない直接計算 plan（`NccDirect`）を選ぶ
    /// （係数は al-math 校正値、選び損ねても出力契約は不変）。
    /// この plan で `gcc_phat` を呼ぶと `Err(Error::InvalidValue("mode"))`
    /// （PHAT の位相正規化は FFT 長依存のため）。
    ///
    /// **RT 契約:** non-RT only — 直接計算を選んだ場合を除き FFT plan を確保する。
    ///
    /// # Errors
    ///
    /// FFT 長の算術が overflow すれば `Error::InvalidValue("length")`。
    pub fn new_ncc(template_len: usize, signal_len: usize) -> Result<Self> {
        let (n, mode) = if template_len == 0 || signal_len < template_len {
            (0, PlanMode::NccValid)
        } else {
            let n = signal_len
                .checked_next_power_of_two()
                .ok_or(Error::InvalidValue("length"))?;
            let mode = if prefers_direct(template_len, signal_len, n) {
                PlanMode::NccDirect
            } else {
                PlanMode::NccValid
            };
            (n, mode)
        };
        Ok(Self {
            template_len,
            signal_len,
            fft: (n != 0 && mode != PlanMode::NccDirect)
                .then(|| RealFftPlan::new(n))
                .transpose()?,
            n,
            mode,
        })
    }

    /// plan の template 長。
    #[must_use]
    pub fn template_len(&self) -> usize {
        self.template_len
    }

    /// plan の signal 長。
    #[must_use]
    pub fn signal_len(&self) -> usize {
        self.signal_len
    }

    /// 内部で使う FFT 変換長（退化 shape・`NccDirect` 直接計算では `0`）。
    /// `NccDirect` の判定はコストモデルに依存するため、plan mode を
    /// 利用側が型で確定できない代わりに本値で観測できる。
    #[must_use]
    pub fn fft_len(&self) -> usize {
        if self.mode == PlanMode::NccDirect {
            0
        } else {
            self.n
        }
    }

    /// `ncc`/`gcc_phat` が返せる最大ラグ（`signal_len − template_len`、
    /// 退化 shape では `0`）。
    #[must_use]
    pub fn valid_lag_max(&self) -> usize {
        self.signal_len.saturating_sub(self.template_len)
    }

    /// `cross_correlation` が返せる無汚染最大ラグ。Full mode は
    /// `S+L−2`（部分 overlap を含む線形相関全域）、
    /// `NccValid`/`NccDirect` は `S−L`。
    #[must_use]
    pub fn linear_lag_max(&self) -> usize {
        match self.mode {
            PlanMode::Full => self
                .signal_len
                .saturating_add(self.template_len)
                .saturating_sub(2),
            _ => self.valid_lag_max(),
        }
    }

    /// plan が保持する FFT 係数のメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.fft.as_ref().map_or(0, RealFftPlan::memory_bytes)
    }

    /// 有効ラグ域 `0..=min(max_lag, S−L)` の符号付き NCC カーブを
    /// workspace から借用する。`values[k]` は `signal[k..k+L]` と
    /// `template` の非中心化・符号付き正規化相関。
    ///
    /// 借用結果を保持したまま workspace を再使用できない。
    ///
    /// **RT 契約:** RT-suitable — 事前確保済み workspace のみ使用し、
    /// alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `template.len() != template_len` なら `Error::InvalidValue("template")`、
    /// `signal.len() != signal_len` なら `Error::InvalidValue("signal")`、
    /// `workspace` が本 plan の mode・shape と異なれば
    /// `Error::InvalidValue("workspace")`。
    pub fn ncc<'w>(
        &self,
        workspace: &'w mut CorrelationWorkspace,
        template: &[f64],
        signal: &[f64],
        max_lag: usize,
    ) -> Result<NccCurve<'w>> {
        let (out_len, template_energy) =
            self.run(workspace, template, signal, max_lag, Kernel::Ncc)?;
        Ok(NccCurve {
            values: &workspace.real[..out_len],
            window_energy: &workspace.norms[..out_len],
            template_energy,
        })
    }

    /// 同じ FFT plan で GCC-PHAT カーブを生成する。相互スペクトル
    /// `S[k] = conj(A[k])·B[k]` の各 bin を `|S[k]|` で白色化
    /// （ゼロ cross bin は `0.0`）して逆変換した `0..=min(max_lag, S−L)`
    /// のカーブ。`new_ncc` で作った plan では
    /// `Err(Error::InvalidValue("mode"))`（短縮 FFT 長では位相正規化結果が
    /// 変わるため）。
    ///
    /// 借用結果を保持したまま workspace を再使用できない。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// 形状・workspace 検証は [`Self::ncc`] と同じ。
    pub fn gcc_phat<'w>(
        &self,
        workspace: &'w mut CorrelationWorkspace,
        template: &[f64],
        signal: &[f64],
        max_lag: usize,
    ) -> Result<&'w [f64]> {
        if self.mode != PlanMode::Full {
            return Err(Error::InvalidValue("mode"));
        }
        let (out_len, _) = self.run(workspace, template, signal, max_lag, Kernel::Phat)?;
        Ok(&workspace.real[..out_len])
    }

    /// 正規化なしの線形相互相関 `c[k] = Σᵢ template[i]·signal[k+i]` の
    /// `0..=min(max_lag, linear_lag_max)` のカーブを workspace から借用する。
    /// Full mode plan では `S−L < k ≤ S+L−2` の部分 overlap ラグも
    /// 無汚染（`n ≥ S+L−1` で wraparound が混入しない）。
    ///
    /// 借用結果を保持したまま workspace を再使用できない。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// 形状・workspace 検証は [`Self::ncc`] と同じ。
    pub fn cross_correlation<'w>(
        &self,
        workspace: &'w mut CorrelationWorkspace,
        template: &[f64],
        signal: &[f64],
        max_lag: usize,
    ) -> Result<&'w [f64]> {
        let (out_len, _) = self.run(workspace, template, signal, max_lag, Kernel::Raw)?;
        Ok(&workspace.real[..out_len])
    }

    /// 共通処理。`real[..out_len]` にカーブを書き、Ncc なら
    /// `norms[..out_len]` に窓エネルギーを書いて `(out_len, template_energy)`
    /// を返す。Raw/Phat は `template_energy` を計算しない（`0.0`）。
    fn run(
        &self,
        workspace: &mut CorrelationWorkspace,
        template: &[f64],
        signal: &[f64],
        max_lag: usize,
        kind: Kernel,
    ) -> Result<(usize, f64)> {
        if template.len() != self.template_len {
            return Err(Error::InvalidValue("template"));
        }
        if signal.len() != self.signal_len {
            return Err(Error::InvalidValue("signal"));
        }
        if (workspace.template_len, workspace.signal_len, workspace.mode)
            != (self.template_len, self.signal_len, self.mode)
        {
            return Err(Error::InvalidValue("workspace"));
        }
        let len = self.template_len;
        if self.n == 0 {
            // 退化 shape: 有効ラグ域なし。Ncc の template_energy は
            // 空カーブでも縮退判別ができるよう実値を返す。
            let template_energy = if kind == Kernel::Ncc {
                sum_sq_seq(template)
            } else {
                0.0
            };
            return Ok((0, template_energy));
        }
        let bound = match kind {
            Kernel::Raw => self.linear_lag_max(),
            _ => self.valid_lag_max(),
        };
        let max_lag = max_lag.min(bound);
        let CorrelationWorkspace {
            fft,
            real,
            a_re,
            a_im,
            b_re,
            b_im,
            cumulative,
            norms,
            phat_energy,
            ..
        } = workspace;
        // Ncc: 正規化に使うエネルギー証拠（窓側・template 側）を先に計算する。
        // template_energy == 0 でも window_energy は実値を返すため、
        // 正規化経路とは独立にここで確定させる。
        let template_energy = if kind == Kernel::Ncc {
            cumulative[0] = 0.0;
            for (i, &v) in signal.iter().enumerate() {
                cumulative[i + 1] = cumulative[i] + v * v;
            }
            for (lag, norm) in norms[..=max_lag].iter_mut().enumerate() {
                let e = cumulative[lag + len] - cumulative[lag];
                // 負の丸めは 0 に張り付けるが、NaN は max で隠さない（証拠契約）。
                *norm = if e.is_nan() { f64::NAN } else { e.max(0.0) };
            }
            let energy = sum_sq_seq(template);
            if energy == 0.0 {
                real[..=max_lag].fill(0.0);
                return Ok((max_lag + 1, 0.0));
            }
            energy
        } else {
            0.0
        };
        // 分子（相互相関）。Ncc/Phat/Raw とも同じ経路で `real` に置く。
        if self.mode == PlanMode::NccDirect {
            // 分子を直接計算: real[k] = Σᵢ template[i]·signal[k+i]。
            // signal の参照窓は i に対し max_lag+1 要素のスライドなので
            // キャッシュ局所性が高い。
            real[..=max_lag].fill(0.0);
            for (i, &ai) in template.iter().enumerate() {
                axpy_seq(&mut real[..=max_lag], ai, &signal[i..=i + max_lag]);
            }
        } else {
            let (Some(fft_plan), Some(fft_ws)) = (self.fft.as_ref(), fft.as_mut()) else {
                // Full/NccValid plan では fft が構築保証される。ws は mode
                // で検証済みのため未到達だが、panic-free 契約として Err を返す。
                return Err(Error::InvalidValue("workspace"));
            };
            real.fill(0.0);
            real[..len].copy_from_slice(template);
            fft_plan.forward(fft_ws, real, a_re, a_im)?;
            real.fill(0.0);
            real[..self.signal_len].copy_from_slice(signal);
            fft_plan.forward(fft_ws, real, b_re, b_im)?;
            cross_spectrum(a_re, a_im, b_re, b_im, kind == Kernel::Phat, phat_energy);
            fft_plan.inverse(fft_ws, a_re, a_im, real)?;
        }
        if kind == Kernel::Ncc {
            normalize(
                &mut real[..=max_lag],
                &norms[..=max_lag],
                template_energy.sqrt(),
            );
        }
        Ok((max_lag + 1, template_energy))
    }
}

impl CorrelationWorkspace {
    /// shape ごとの最大領域を確保し、実行時の resize を不要にする。
    /// `NccDirect` plan は FFT バッファを持たず、`real` は有効ラグ域
    /// `S−L+1` の出力領域として使う。
    ///
    /// **RT 契約:** non-RT only — plan の mode・shape に応じた全バッファを確保する。
    #[must_use]
    pub fn new(plan: &CorrelationPlan) -> Self {
        let direct = plan.mode == PlanMode::NccDirect;
        let bins = if plan.n == 0 || direct {
            0
        } else {
            plan.n / 2 + 1
        };
        let signal_len = if plan.n == 0 { 0 } else { plan.signal_len };
        let count = if plan.n == 0 {
            0
        } else {
            plan.signal_len - plan.template_len + 1
        };
        let real_len = if plan.n == 0 {
            0
        } else if direct {
            count
        } else {
            plan.n
        };
        Self {
            template_len: plan.template_len,
            signal_len: plan.signal_len,
            mode: plan.mode,
            fft: plan.fft.as_ref().map(RealFftWorkspace::new),
            real: vec![0.0; real_len],
            a_re: vec![0.0; bins],
            a_im: vec![0.0; bins],
            b_re: vec![0.0; bins],
            b_im: vec![0.0; bins],
            cumulative: vec![0.0; if plan.n == 0 { 0 } else { signal_len + 1 }],
            norms: vec![0.0; count],
            phat_energy: vec![0.0; if plan.mode == PlanMode::Full { bins } else { 0 }],
        }
    }

    /// 保持する作業領域のメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        (self.real.len()
            + self.a_re.len()
            + self.a_im.len()
            + self.b_re.len()
            + self.b_im.len()
            + self.cumulative.len()
            + self.norms.len()
            + self.phat_energy.len())
            * size_of::<f64>()
            + self.fft.as_ref().map_or(0, RealFftWorkspace::memory_bytes)
    }
}

impl OverlapCorrPlan {
    /// 窓長 `window_len` を shape に取る。内部 FFT 長は `nextpow2(2L−1)`。
    /// `window_len == 0` は `n = 0` の退化 plan で、`curve` は全 `0.0`。
    ///
    /// **RT 契約:** non-RT only — FFT plan を確保する。
    ///
    /// # Errors
    ///
    /// FFT 長の算術が overflow すれば `Error::InvalidValue("length")`。
    pub fn new(window_len: usize) -> Result<Self> {
        let n = if window_len == 0 {
            0
        } else {
            window_len
                .checked_mul(2)
                .and_then(|v| v.checked_sub(1))
                .and_then(usize::checked_next_power_of_two)
                .ok_or(Error::InvalidValue("length"))?
        };
        Ok(Self {
            window_len,
            fft: (n != 0).then(|| RealFftPlan::new(n)).transpose()?,
            n,
        })
    }

    /// plan の窓長。
    #[must_use]
    pub fn window_len(&self) -> usize {
        self.window_len
    }

    /// 内部で使う FFT 変換長（退化 shape では `0`）。
    #[must_use]
    pub fn fft_len(&self) -> usize {
        self.n
    }

    /// plan が保持する FFT 係数のメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.fft.as_ref().map_or(0, RealFftPlan::memory_bytes)
    }

    /// `lag = i − max_lag` の相関値を持つ長さ `2·max_lag+1` のカーブを
    /// workspace から借用する。`values[i]` は overlap 区間
    /// `L−|lag|` 標本での非中心化正規化相関（`|lag| ≥ L` は `0.0`）。
    ///
    /// 借用結果を保持したまま workspace を再使用できない。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `x`/`y` の長さが `window_len` と異なれば
    /// `Error::InvalidValue("x")`/`("y")`。`workspace` が本 plan 用でなければ
    /// `Error::InvalidValue("workspace")`。`2·max_lag+1` が workspace の
    /// 確保容量を超えれば `Error::InvalidValue("max_lag")`
    /// （[`OverlapCorrWorkspace::new`] は `2L−1`、
    /// [`OverlapCorrWorkspace::with_max_lag`] は `2·max_lag+1` を確保する）。
    pub fn curve<'w>(
        &self,
        workspace: &'w mut OverlapCorrWorkspace,
        x: &[f64],
        y: &[f64],
        max_lag: usize,
    ) -> Result<OverlapCurve<'w>> {
        if x.len() != self.window_len {
            return Err(Error::InvalidValue("x"));
        }
        if y.len() != self.window_len {
            return Err(Error::InvalidValue("y"));
        }
        if workspace.window_len != self.window_len {
            return Err(Error::InvalidValue("workspace"));
        }
        let out_len = max_lag
            .checked_mul(2)
            .and_then(|v| v.checked_add(1))
            .ok_or(Error::InvalidValue("max_lag"))?;
        if out_len > workspace.curve.len() {
            return Err(Error::InvalidValue("max_lag"));
        }
        if self.n == 0 {
            workspace.curve[..out_len].fill(0.0);
            workspace.overlap_energy[..out_len].fill(0.0);
            return Ok(OverlapCurve {
                values: &workspace.curve[..out_len],
                overlap_energy: &workspace.overlap_energy[..out_len],
            });
        }
        let len = self.window_len;
        let n = self.n;
        let OverlapCorrWorkspace {
            fft,
            real,
            a_re,
            a_im,
            b_re,
            b_im,
            x_prefix,
            y_prefix,
            curve,
            overlap_energy,
            ..
        } = workspace;
        // 直接法は ~3·(2·max_lag+1)·L flop（corr0 の 3 累積）、
        // FFT 法は ~15·n·log2(n) flop（3 変換 + 合成）。実測の交差に合わせた
        // 仕事量比較（al-math 校正値を継承、#152 比較記録で再確認）。
        let direct_ops = out_len.saturating_mul(len).saturating_mul(3);
        let fft_ops = OVERLAP_FFT_COST_FACTOR
            .saturating_mul(n)
            .saturating_mul(n.trailing_zeros() as usize);
        if direct_ops < fft_ops {
            for (i, out) in curve.iter_mut().enumerate().take(out_len) {
                let lag = i.cast_signed() - max_lag.cast_signed();
                let (value, energy) = overlap_direct(x, y, lag, len);
                *out = value;
                overlap_energy[i] = energy;
            }
            return Ok(OverlapCurve {
                values: &curve[..out_len],
                overlap_energy: &overlap_energy[..out_len],
            });
        }
        // 分子: 線形相互相関 c[k] = Σᵢ x[i]·y[i+k]。n ≥ 2L−1 の
        // ゼロ埋めで巡回 wraparound は混入しない。負 lag −a は
        // c[n−a] = Σᵢ x[i]·y[i−a] に対応する。
        let (Some(fft_plan), Some(fft_ws)) = (self.fft.as_ref(), fft.as_mut()) else {
            // n > 0 では fft が構築保証される。panic-free 契約として Err を返す。
            return Err(Error::InvalidValue("workspace"));
        };
        real.fill(0.0);
        real[..len].copy_from_slice(x);
        fft_plan.forward(fft_ws, real, a_re, a_im)?;
        real.fill(0.0);
        real[..len].copy_from_slice(y);
        fft_plan.forward(fft_ws, real, b_re, b_im)?;
        cross_spectrum(a_re, a_im, b_re, b_im, false, &mut []);
        fft_plan.inverse(fft_ws, a_re, a_im, real)?;
        // 分母: overlap 範囲のエネルギーを prefix 二乗和から得る。
        x_prefix[0] = 0.0;
        y_prefix[0] = 0.0;
        for i in 0..len {
            x_prefix[i + 1] = x_prefix[i] + x[i] * x[i];
            y_prefix[i + 1] = y_prefix[i] + y[i] * y[i];
        }
        for (i, out) in curve.iter_mut().enumerate().take(out_len) {
            let lag = i.cast_signed() - max_lag.cast_signed();
            let (value, energy) = overlap_fft(real, x_prefix, y_prefix, n, lag, len);
            *out = value;
            overlap_energy[i] = energy;
        }
        Ok(OverlapCurve {
            values: &curve[..out_len],
            overlap_energy: &overlap_energy[..out_len],
        })
    }
}

impl OverlapCorrWorkspace {
    /// 全有効ラグ域（`|lag| < L` の全域 `2L−1`）を収める容量で確保する。
    /// `max_lag ≥ L` の `curve` 呼出は容量超過のため
    /// [`Self::with_max_lag`] で確保する。
    ///
    /// **RT 契約:** non-RT only — 全バッファを確保する。
    #[must_use]
    pub fn new(plan: &OverlapCorrPlan) -> Self {
        let capacity = (2usize.saturating_mul(plan.window_len))
            .saturating_sub(1)
            .max(1);
        Self::with_capacity(plan, capacity)
    }

    /// 出力長 `2·max_lag+1` までを収める容量で確保する。
    /// `lag_max << window_len` の用途（al-analysis の reference lag は
    /// `lag_max=512` に対し `lag_window=8192`）では [`Self::new`] より
    /// 小さいバッファになる。容量を超える `max_lag` の `curve` 呼出は
    /// `Err(Error::InvalidValue("max_lag"))` を返す（al-math は `Vec::resize`
    /// で呼出中に確保していたが、本 API は処理呼出しの alloc を排除するため
    /// 容量を workspace 構築時へ移す）。
    ///
    /// **RT 契約:** non-RT only — 全バッファを確保する。
    #[must_use]
    pub fn with_max_lag(plan: &OverlapCorrPlan, max_lag: usize) -> Self {
        let capacity = (2usize.saturating_mul(max_lag)).saturating_add(1).max(1);
        Self::with_capacity(plan, capacity)
    }

    fn with_capacity(plan: &OverlapCorrPlan, capacity: usize) -> Self {
        let bins = if plan.n == 0 { 0 } else { plan.n / 2 + 1 };
        let l = plan.window_len;
        Self {
            window_len: l,
            fft: plan.fft.as_ref().map(RealFftWorkspace::new),
            real: vec![0.0; plan.n],
            a_re: vec![0.0; bins],
            a_im: vec![0.0; bins],
            b_re: vec![0.0; bins],
            b_im: vec![0.0; bins],
            x_prefix: vec![0.0; l + 1],
            y_prefix: vec![0.0; l + 1],
            curve: vec![0.0; capacity],
            overlap_energy: vec![0.0; capacity],
        }
    }

    /// 保持する作業領域のメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        (self.real.len()
            + self.a_re.len()
            + self.a_im.len()
            + self.b_re.len()
            + self.b_im.len()
            + self.x_prefix.len()
            + self.y_prefix.len()
            + self.curve.len()
            + self.overlap_energy.len())
            * size_of::<f64>()
            + self.fft.as_ref().map_or(0, RealFftWorkspace::memory_bytes)
    }
}

/// `signal[lag..lag + template.len()]` との符号付き NCC。one-shot 版。
///
/// `max_lag` は `signal.len() − template.len()` へクランプされる。
/// 戻り値の長さは `min(max_lag, S−L) + 1`（退化 shape では `0`）。
///
/// **RT 契約:** non-RT only — plan・workspace・出力を確保する。
///
/// # Errors
///
/// plan 構築の長さ算術が overflow すれば `Error::InvalidValue("length")`。
pub fn ncc(template: &[f64], signal: &[f64], max_lag: usize) -> Result<Vec<f64>> {
    let plan = CorrelationPlan::new_ncc(template.len(), signal.len())?;
    let mut workspace = CorrelationWorkspace::new(&plan);
    Ok(plan
        .ncc(&mut workspace, template, signal, max_lag)?
        .values
        .to_vec())
}

/// 窓 NCC と同じ正規化・lag 方向を持つ GCC-PHAT の one-shot 版。
/// 戻り値の長さは `min(max_lag, S−L) + 1`（退化 shape では `0`）。
///
/// **RT 契約:** non-RT only — plan・workspace・出力を確保する。
///
/// # Errors
///
/// plan 構築の長さ算術が overflow すれば `Error::InvalidValue("length")`。
pub fn gcc_phat(template: &[f64], signal: &[f64], max_lag: usize) -> Result<Vec<f64>> {
    let plan = CorrelationPlan::new(template.len(), signal.len())?;
    let mut workspace = CorrelationWorkspace::new(&plan);
    Ok(plan
        .gcc_phat(&mut workspace, template, signal, max_lag)?
        .to_vec())
}

/// 正規化なしの線形相互相関 `c[k] = Σᵢ template[i]·signal[k+i]` の
/// one-shot 版。`max_lag` は `S+L−2`（Full mode の無汚染ラグ上限）へ
/// クランプされる。戻り値の長さは `min(max_lag, S+L−2) + 1`
/// （退化 shape では `0`）。
///
/// **RT 契約:** non-RT only — plan・workspace・出力を確保する。
///
/// # Errors
///
/// plan 構築の長さ算術が overflow すれば `Error::InvalidValue("length")`。
pub fn cross_correlation(template: &[f64], signal: &[f64], max_lag: usize) -> Result<Vec<f64>> {
    let plan = CorrelationPlan::new(template.len(), signal.len())?;
    let mut workspace = CorrelationWorkspace::new(&plan);
    Ok(plan
        .cross_correlation(&mut workspace, template, signal, max_lag)?
        .to_vec())
}

/// 等長窓の両側 lag 相関曲線。`curve[i]` は `lag = i − max_lag` の値の
/// one-shot 版。`2·max_lag+1` 分だけ確保する。
///
/// **RT 契約:** non-RT only — plan・workspace・出力を確保する。
///
/// # Errors
///
/// `x`/`y` が等長でなければ `Error::InvalidValue("y")`。plan 構築の長さ
/// 算術が overflow すれば `Error::InvalidValue("length")`。
pub fn overlap_corr_curve(x: &[f64], y: &[f64], max_lag: usize) -> Result<Vec<f64>> {
    if x.len() != y.len() {
        return Err(Error::InvalidValue("y"));
    }
    let plan = OverlapCorrPlan::new(x.len())?;
    let mut workspace = OverlapCorrWorkspace::with_max_lag(&plan, max_lag);
    Ok(plan.curve(&mut workspace, x, y, max_lag)?.values.to_vec())
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    /// 決定的ノイズ（al-math correlation テストと同じ LCG）。
    fn noise(len: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        (0..len)
            .map(|_| {
                s = s
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                ((s >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
            })
            .collect()
    }

    /// overlap 区間の `corr0` を lag ごとに直接評価する oracle
    /// （al-math テストの `corr_at` と同じ定義）。
    fn corr_at(x: &[f64], y: &[f64], lag: isize) -> f64 {
        let (xs, ys) = if lag >= 0 {
            (
                &x[..x.len() - lag.cast_unsigned().min(x.len())],
                &y[lag.cast_unsigned()..],
            )
        } else {
            (
                &x[lag.unsigned_abs()..],
                &y[..y.len() - lag.unsigned_abs().min(y.len())],
            )
        };
        let n = xs.len().min(ys.len());
        if n == 0 {
            return 0.0;
        }
        crate::vec::corr0(&xs[..n], &ys[..n]).unwrap()
    }

    /// lag k の NCC を内積・二乗和で直接評価する oracle。
    fn ncc_at(template: &[f64], signal: &[f64], lag: usize) -> f64 {
        let window = &signal[lag..lag + template.len()];
        let sxy = crate::vec::dot(template, window).unwrap();
        let te = crate::vec::sum_sq(template);
        let se = crate::vec::sum_sq(window);
        if te > 0.0 && se > 0.0 {
            sxy / (te * se).sqrt()
        } else {
            0.0
        }
    }

    /// カーブの |値| argmax。
    fn argmax_abs(curve: &[f64]) -> usize {
        curve
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .map(|(i, _)| i)
            .unwrap()
    }

    // --- CorrelationPlan: plan モードと形状 ---

    /// 形状ベンチの 6 代表形状で、コストモデルが al-math と同じ mode を
    /// 選ぶことを固定する（係数回帰で FFT/direct が逆転しないよう pin）。
    #[test]
    fn ncc_plan_mode_pins_benchmarked_shapes() {
        for (template_len, signal_len, expected) in [
            (2_205_000usize, 2_205_236usize, PlanMode::NccDirect), // band_ncc/full_ncc
            (551_250, 551_309, PlanMode::NccDirect),               // coarse_ncc
            (4_410, 7_938, PlanMode::NccValid),                    // band_gain window
            (4_410, 13_230, PlanMode::NccValid),                   // narrow anchor 小
            (4_410, 66_150, PlanMode::NccValid),                   // narrow anchor 中
            (800, 1_438, PlanMode::NccValid),                      // tiny 8kHz
        ] {
            let plan = CorrelationPlan::new_ncc(template_len, signal_len).unwrap();
            assert_eq!(plan.mode, expected, "shape=({template_len},{signal_len})");
            assert_eq!(
                plan.fft.is_none(),
                expected == PlanMode::NccDirect,
                "shape=({template_len},{signal_len})"
            );
        }
    }

    /// direct plan は FFT plan/バッファを持たず、`real` は有効ラグ域長の
    /// 出力領域になる。共有の正規化バッファのみ残る。
    #[test]
    fn ncc_direct_plan_skips_fft_buffers() {
        let plan = CorrelationPlan::new_ncc(256, 300).unwrap();
        assert_eq!(plan.mode, PlanMode::NccDirect);
        assert!(plan.fft.is_none());
        let ws = CorrelationWorkspace::new(&plan);
        assert!(ws.fft.is_none());
        assert_eq!(ws.a_re, []);
        assert_eq!(ws.a_im, []);
        assert_eq!(ws.b_re, []);
        assert_eq!(ws.b_im, []);
        assert_eq!(ws.phat_energy, []);
        assert_eq!(ws.real.len(), 300 - 256 + 1);
        assert_eq!(ws.cumulative.len(), 301);
        assert_eq!(ws.norms.len(), 45);
    }

    /// NCC-valid plan（FFT `nextpow2(S)`）は返却される全有効ラグで
    /// Full plan（`nextpow2(S+L−1)`）と同一契約の値を返す。
    /// wraparound 汚染は index `> S−L` にのみ発生する。
    /// 形状はコストモデルが `NccValid` を選ぶものに限る
    /// （`NccDirect` ルートは別テストで検証する）。
    #[test]
    fn ncc_valid_plan_matches_full_plan_on_all_returned_lags() {
        for (template_len, signal_len, max_lag) in [
            (100usize, 1000usize, usize::MAX),
            (512, 4096, usize::MAX),
            (4410, 7938, usize::MAX), // 44.1k band_gain 形状
            (4800, 8640, usize::MAX), // 48k band_gain 形状
            (96_000, 100_800, usize::MAX),
            (2_000, 3_000, usize::MAX), // 非 2 冪・K ≤ 2048 でコスト超過
            (100, 1000, 10),            // max_lag < S−L
            (100, 1000, 0),             // lag 0 のみ
        ] {
            let template = noise(template_len, 11);
            let signal = noise(signal_len, 13);
            let full = CorrelationPlan::new(template_len, signal_len).unwrap();
            let valid = CorrelationPlan::new_ncc(template_len, signal_len).unwrap();
            assert_eq!(valid.mode, PlanMode::NccValid);
            let mut full_ws = CorrelationWorkspace::new(&full);
            let mut valid_ws = CorrelationWorkspace::new(&valid);
            let expected = full
                .ncc(&mut full_ws, &template, &signal, max_lag)
                .unwrap()
                .values
                .to_vec();
            let actual = valid
                .ncc(&mut valid_ws, &template, &signal, max_lag)
                .unwrap()
                .values
                .to_vec();
            assert_eq!(
                expected.len(),
                actual.len(),
                "shape=({template_len},{signal_len})"
            );
            for (i, (&a, &b)) in expected.iter().zip(&actual).enumerate() {
                let d = (a - b).abs();
                assert!(
                    d < 1e-10 || d / a.abs().max(1.0) < 1e-10,
                    "shape=({template_len},{signal_len}) lag={i}: full {a} vs valid {b}"
                );
            }
            assert_eq!(
                argmax_abs(&expected),
                argmax_abs(&actual),
                "shape=({template_len},{signal_len}) peak"
            );
        }
    }

    /// direct plan の NCC は Full FFT plan と有効ラグ全域で一致し、
    /// `argmax`・`max_lag` クランプ・戻り範囲も同一。
    /// 形状はコストモデルが `NccDirect` を選ぶものに限る。
    #[test]
    fn ncc_direct_matches_full_plan_on_all_returned_lags() {
        for (template_len, signal_len, max_lag) in [
            (4usize, 4usize, usize::MAX), // 等長（K=0）
            (1, 8, usize::MAX),
            (3, 33, usize::MAX),
            (4, 5, usize::MAX),
            (33, 64, usize::MAX),
            (64, 100, usize::MAX),
            (256, 300, usize::MAX),
            (16, 64, 10), // max_lag < S−L
            (16, 64, 0),  // lag 0 のみ
        ] {
            let template = noise(template_len, 11);
            let signal = noise(signal_len, 13);
            let full = CorrelationPlan::new(template_len, signal_len).unwrap();
            let direct = CorrelationPlan::new_ncc(template_len, signal_len).unwrap();
            assert_eq!(
                direct.mode,
                PlanMode::NccDirect,
                "shape=({template_len},{signal_len})"
            );
            let mut full_ws = CorrelationWorkspace::new(&full);
            let mut direct_ws = CorrelationWorkspace::new(&direct);
            let expected = full
                .ncc(&mut full_ws, &template, &signal, max_lag)
                .unwrap()
                .values
                .to_vec();
            let actual = direct
                .ncc(&mut direct_ws, &template, &signal, max_lag)
                .unwrap()
                .values
                .to_vec();
            assert_eq!(
                expected.len(),
                actual.len(),
                "shape=({template_len},{signal_len})"
            );
            for (i, (&a, &b)) in expected.iter().zip(&actual).enumerate() {
                let d = (a - b).abs();
                assert!(
                    d < 1e-10 || d / a.abs().max(1.0) < 1e-10,
                    "shape=({template_len},{signal_len}) lag={i}: full {a} vs direct {b}"
                );
            }
            assert_eq!(
                argmax_abs(&expected),
                argmax_abs(&actual),
                "shape=({template_len},{signal_len}) peak"
            );
        }
    }

    /// NCC は逐点 oracle（内積/二乗和の直接評価）と一致する。
    #[test]
    fn ncc_matches_pointwise_oracle() {
        for (template_len, signal_len) in [
            (3usize, 33usize),
            (16, 64),
            (64, 100),
            (4410, 7938), // FFT 経路・実形状
        ] {
            let template = noise(template_len, 17);
            let signal = noise(signal_len, 19);
            let curve = ncc(&template, &signal, usize::MAX).unwrap();
            assert_eq!(curve.len(), signal_len - template_len + 1);
            for (lag, &v) in curve.iter().enumerate() {
                let oracle = ncc_at(&template, &signal, lag);
                let d = (v - oracle).abs();
                assert!(
                    d < 1e-10 || d / oracle.abs().max(1.0) < 1e-10,
                    "shape=({template_len},{signal_len}) lag={lag}: {v} vs oracle {oracle}"
                );
            }
        }
    }

    /// 既知オフセットを埋め込んだ信号で、埋込み位置 `len` のラグが
    /// 最大 |値| を返す（符号・極性契約: 正の埋込みは正ピーク、
    /// 反転埋込みは負ピーク）。`len=1` では全有効窓が |NCC|=1 に
    /// なるため一意 argmax は要求しない。
    #[test]
    fn ncc_finds_known_offset_with_signed_polarity() {
        for len in [1usize, 2, 3, 7, 16, 33, 127] {
            let signal: Vec<_> = (0..=(len * 3)).map(|i| (i as f64 * 1.713).sin()).collect();
            for polarity in [-1.0, 1.0] {
                let template: Vec<_> = signal[len..len * 2].iter().map(|x| x * polarity).collect();
                for phat in [false, true] {
                    let plan = CorrelationPlan::new(template.len(), signal.len()).unwrap();
                    let mut ws = CorrelationWorkspace::new(&plan);
                    let curve: Vec<f64> = if phat {
                        plan.gcc_phat(&mut ws, &template, &signal, usize::MAX)
                            .unwrap()
                            .to_vec()
                    } else {
                        plan.ncc(&mut ws, &template, &signal, usize::MAX)
                            .unwrap()
                            .values
                            .to_vec()
                    };
                    assert_eq!(curve.len(), signal.len() - template.len() + 1);
                    // PHAT の白色化は振幅情報を捨てるため、小さい合成入力では
                    // 真の lag に一意ピークを立てない。PHAT の argmax は
                    // `gcc_phat_mode_gate_and_known_delay` の実ノイズ入力で
                    // 検証し、ここでは有限性のみ確認する。
                    if phat {
                        assert!(
                            curve.iter().all(|v| v.is_finite()),
                            "len={len} polarity={polarity} phat={phat}: non-finite value"
                        );
                    } else {
                        let max_abs = curve.iter().fold(0.0f64, |m, v| m.max(v.abs()));
                        // 埋込み位置 len が（同率を含めて）最大 |値| のひとつ。
                        assert!(
                            curve[len].abs() >= max_abs * (1.0 - 1e-9),
                            "len={len} polarity={polarity} phat={phat}: |curve[len]|={} vs max {max_abs}",
                            curve[len].abs()
                        );
                        // 十分な窓長では一意 argmax。
                        if len >= 7 {
                            assert_eq!(
                                argmax_abs(&curve),
                                len,
                                "len={len} polarity={polarity} phat={phat}"
                            );
                        }
                    }
                    // NCC の符号契約: 反転埋込みは負値が最大 |値|。
                    if !phat && curve[len] != 0.0 {
                        assert!(
                            (curve[len].signum() - polarity.signum()).abs() < f64::EPSILON,
                            "len={len} polarity={polarity}: sign {}",
                            curve[len]
                        );
                    }
                }
            }
        }
    }

    /// ゼロ template・無音窓・NaN 混入の縮退意味論と証拠フィールド。
    #[test]
    fn ncc_degenerate_energy_and_evidence_fields() {
        // ゼロ template → 全値 0.0 だが window_energy は実値を返す。
        let template = [0.0; 8];
        let signal = noise(32, 5);
        let plan = CorrelationPlan::new_ncc(8, 32).unwrap();
        let mut ws = CorrelationWorkspace::new(&plan);
        let curve = plan.ncc(&mut ws, &template, &signal, usize::MAX).unwrap();
        assert_eq!(curve.template_energy, 0.0);
        assert!(curve.values.iter().all(|&v| v == 0.0));
        for (lag, &e) in curve.window_energy.iter().enumerate() {
            let expected = crate::vec::sum_sq(&signal[lag..lag + 8]);
            assert!(
                (e - expected).abs() < 1e-10 * expected.max(1.0),
                "lag={lag}: {e} vs {expected}"
            );
        }
        // 窓内が無音の lag は値 0.0 とエネルギー 0.0 が対応する。
        let template = noise(8, 7);
        let mut signal = noise(32, 9);
        signal[10..18].fill(0.0); // lag 10 の窓を無音化
        let plan = CorrelationPlan::new_ncc(8, 32).unwrap();
        let mut ws = CorrelationWorkspace::new(&plan);
        let c = plan.ncc(&mut ws, &template, &signal, usize::MAX).unwrap();
        assert_eq!(c.values[10], 0.0);
        assert_eq!(c.window_energy[10], 0.0);
        // NaN 混入: 窓内 NaN の lag は値 0.0・エネルギー NaN（証拠を隠さない）。
        let mut signal = noise(32, 9);
        signal[12] = f64::NAN; // lag 5..=12 の窓に混入
        let c = plan.ncc(&mut ws, &template, &signal, usize::MAX).unwrap();
        for lag in 5..=12usize {
            assert!(
                c.window_energy[lag].is_nan() || c.values[lag] == 0.0,
                "lag={lag}: energy {} value {}",
                c.window_energy[lag],
                c.values[lag]
            );
        }
    }

    /// `gcc_phat` は `new_ncc` plan を `Err("mode")` で拒否する。
    /// `new` plan では既知遅延のデルタで正しい lag にピークを立てる。
    #[test]
    fn gcc_phat_mode_gate_and_known_delay() {
        let template = noise(64, 21);
        let mut signal = noise(300, 23);
        for (i, &v) in template.iter().enumerate() {
            signal[100 + i] = v;
        }
        let plan = CorrelationPlan::new(64, 300).unwrap();
        let mut ws = CorrelationWorkspace::new(&plan);
        let curve = plan
            .gcc_phat(&mut ws, &template, &signal, usize::MAX)
            .unwrap();
        assert_eq!(curve.len(), 300 - 64 + 1);
        assert_eq!(argmax_abs(curve), 100);
        // NccValid/NccDirect plan では mode エラー。
        for (l, s) in [(64usize, 300usize), (256, 300)] {
            let plan = CorrelationPlan::new_ncc(l, s).unwrap();
            let mut ws = CorrelationWorkspace::new(&plan);
            let template = noise(l, 25);
            let signal = noise(s, 27);
            assert_eq!(
                plan.gcc_phat(&mut ws, &template, &signal, 10).err(),
                Some(Error::InvalidValue("mode"))
            );
        }
    }

    /// `cross_correlation` は正規化なしの分子を返し、Full plan では
    /// `S−L < k ≤ S+L−2` の部分 overlap ラグも無汚染で返す。
    #[test]
    fn cross_correlation_raw_and_partial_overlap_lags() {
        let template = noise(4, 31);
        let signal = noise(8, 33);
        let plan = CorrelationPlan::new(4, 8).unwrap();
        assert_eq!(plan.linear_lag_max(), 4 + 8 - 2);
        let mut ws = CorrelationWorkspace::new(&plan);
        let curve = plan
            .cross_correlation(&mut ws, &template, &signal, usize::MAX)
            .unwrap()
            .to_vec();
        assert_eq!(curve.len(), 4 + 8 - 1);
        for (k, &v) in curve.iter().enumerate() {
            // oracle: c[k] = Σᵢ template[i]·signal[k+i]（範囲外はゼロ拡張）。
            let oracle: f64 = template
                .iter()
                .enumerate()
                .map(|(i, &t)| {
                    if k + i < signal.len() {
                        t * signal[k + i]
                    } else {
                        0.0
                    }
                })
                .sum();
            assert!(
                (v - oracle).abs() < 1e-12,
                "lag={k}: {v} vs oracle {oracle}"
            );
        }
        // NccValid plan では有効ラグ域（S−L）でクランプされる。
        let plan = CorrelationPlan::new_ncc(4, 8).unwrap();
        assert_eq!(plan.linear_lag_max(), 4);
        let mut ws = CorrelationWorkspace::new(&plan);
        let curve = plan
            .cross_correlation(&mut ws, &template, &signal, usize::MAX)
            .unwrap()
            .to_vec();
        assert_eq!(curve.len(), 8 - 4 + 1);
    }

    /// 形状・workspace・mode の検証は panic ではなく Err。
    #[test]
    fn shape_mismatches_are_errors_not_panics() {
        let plan = CorrelationPlan::new(4, 8).unwrap();
        let mut ws = CorrelationWorkspace::new(&plan);
        let template = noise(4, 35);
        let signal = noise(8, 37);
        assert_eq!(
            plan.ncc(&mut ws, &template[..3], &signal, 0).err(),
            Some(Error::InvalidValue("template"))
        );
        assert_eq!(
            plan.ncc(&mut ws, &template, &signal[..7], 0).err(),
            Some(Error::InvalidValue("signal"))
        );
        // 異なる shape・異なる mode の workspace は拒否される。
        let other_plan = CorrelationPlan::new(4, 9).unwrap();
        let mut other_ws = CorrelationWorkspace::new(&other_plan);
        assert_eq!(
            plan.ncc(&mut other_ws, &template, &signal, 0).err(),
            Some(Error::InvalidValue("workspace"))
        );
        // 同じ (L,S) でも mode 違い（Full ws を NccValid plan へ）は拒否。
        let ncc_plan = CorrelationPlan::new_ncc(4, 8).unwrap();
        assert_eq!(
            ncc_plan.ncc(&mut ws, &template, &signal, 0).err(),
            Some(Error::InvalidValue("workspace"))
        );
    }

    /// 退化 shape（L=0・S<L）は空カーブ。one-shot も空 Vec。
    #[test]
    fn degenerate_shapes_return_empty_curves() {
        for (l, s) in [(0usize, 8usize), (8, 4), (0, 0)] {
            let plan = CorrelationPlan::new(l, s).unwrap();
            assert_eq!(plan.fft_len(), 0);
            let mut ws = CorrelationWorkspace::new(&plan);
            let template = noise(l, 39);
            let signal = noise(s, 41);
            assert_eq!(
                plan.ncc(&mut ws, &template, &signal, usize::MAX)
                    .unwrap()
                    .values,
                []
            );
            assert_eq!(
                plan.gcc_phat(&mut ws, &template, &signal, usize::MAX)
                    .unwrap(),
                []
            );
            assert_eq!(
                plan.cross_correlation(&mut ws, &template, &signal, usize::MAX)
                    .unwrap(),
                []
            );
            assert_eq!(ncc(&template, &signal, usize::MAX).unwrap(), []);
            assert_eq!(gcc_phat(&template, &signal, usize::MAX).unwrap(), []);
            assert_eq!(
                cross_correlation(&template, &signal, usize::MAX).unwrap(),
                Vec::<f64>::new()
            );
        }
    }

    // --- OverlapCorrPlan ---

    /// 両経路（direct/FFT）とも逐点 `corr0` oracle と一致する。
    #[test]
    fn overlap_corr_matches_corr_at_pointwise() {
        // 既知オフセット埋め込み + 複数窓サイズ・max_lag で網羅。
        for (len, max_lag, shift) in [
            (64usize, 16usize, 5isize),
            (256, 40, -17),
            (1024, 96, 200), // FFT 経路（direct_ops > fft_ops）
            (300, 130, 41),  // 非 2 冪の窓長
            (16, 4, 3),      // 小窓（direct 経路）
        ] {
            let x = noise(len, 7);
            let mut y = noise(len, 11);
            let a = shift.unsigned_abs().min(len);
            for i in 0..len - a {
                if shift >= 0 {
                    y[i + a] = 0.8 * x[i] + 0.2 * y[i + a];
                } else {
                    y[i] = 0.8 * x[i + a] + 0.2 * y[i];
                }
            }
            let curve = overlap_corr_curve(&x, &y, max_lag).unwrap();
            assert_eq!(curve.len(), 2 * max_lag + 1);
            let oracle: Vec<f64> = (-max_lag.cast_signed()..=max_lag.cast_signed())
                .map(|lag| corr_at(&x, &y, lag))
                .collect();
            let peak_new = curve
                .iter()
                .enumerate()
                .max_by(|p, q| p.1.total_cmp(q.1))
                .map(|(i, _)| i.cast_signed() - max_lag.cast_signed())
                .unwrap();
            let peak_old = oracle
                .iter()
                .enumerate()
                .max_by(|p, q| p.1.total_cmp(q.1))
                .map(|(i, _)| i.cast_signed() - max_lag.cast_signed())
                .unwrap();
            assert_eq!(peak_new, peak_old, "l={len} peak lag mismatch");
            for (i, (&new, &old)) in curve.iter().zip(&oracle).enumerate() {
                let rel = (new - old).abs() / old.abs().max(1e-12);
                assert!(
                    rel < 1e-9 || (new - old).abs() < 1e-12,
                    "l={len} lag={}: {new} vs {old}",
                    i.cast_signed() - max_lag.cast_signed()
                );
            }
        }
    }

    /// direct 経路と FFT 経路の両方を強制して一致を検証する
    /// （ルーティングはコストモデル依存のため、内部経路を直接比較）。
    #[test]
    fn overlap_corr_direct_and_fft_paths_match() {
        for len in [33usize, 100, 511] {
            let x = noise(len, 5);
            let y = noise(len, 9);
            let max_lag = len.min(40);
            let plan = OverlapCorrPlan::new(len).unwrap();
            // with_max_lag で小容量・全量の両 workspace を用意。
            let mut ws_small = OverlapCorrWorkspace::with_max_lag(&plan, max_lag);
            let mut ws_full = OverlapCorrWorkspace::new(&plan);
            let direct = plan
                .curve(&mut ws_small, &x, &y, max_lag)
                .unwrap()
                .values
                .to_vec();
            // len=511, out_len=81: direct_ops=81*511*3=124k < fft_ops=15*1024*10=153k
            // → direct。大きい max_lag では FFT に切り替わることを
            // out_len 差で間接確認するため、同一 plan で max_lag を変える。
            let fft_side = plan
                .curve(&mut ws_full, &x, &y, len - 1)
                .unwrap()
                .values
                .to_vec();
            for (i, &d) in direct.iter().enumerate() {
                let lag = i.cast_signed() - max_lag.cast_signed();
                // fft_side は lag 域 ±(len−1) のみ。域外 lag は比較対象外。
                let fi = lag + (len - 1).cast_signed();
                if fi < 0 || fi.cast_unsigned() >= fft_side.len() {
                    continue;
                }
                let fi = fi.cast_unsigned();
                let diff = (d - fft_side[fi]).abs();
                assert!(
                    diff < 1e-10 || diff / d.abs().max(1.0) < 1e-10,
                    "l={len} lag={lag}: direct {d} vs fft {}",
                    fft_side[fi]
                );
            }
        }
    }

    /// 片側無音・out-of-range lag・自己相関・空窓のガードと証拠。
    #[test]
    fn overlap_corr_guards_silence_and_out_of_range_lags() {
        let zero = vec![0.0; 128];
        let x = noise(128, 3);
        // 片側無音 → 全 lag が 0.0、energy も 0.0。
        let plan = OverlapCorrPlan::new(128).unwrap();
        let mut ws = OverlapCorrWorkspace::new(&plan);
        let c = plan.curve(&mut ws, &x, &zero, 30).unwrap();
        assert!(c.values.iter().all(|&v| v == 0.0));
        assert!(c.overlap_energy.iter().all(|&e| e == 0.0));
        // max_lag ≥ L: |lag| ≥ L の要素は 0.0（with_max_lag で容量確保）。
        let mut ws_big = OverlapCorrWorkspace::with_max_lag(&plan, 200);
        let c = plan.curve(&mut ws_big, &x, &x, 200).unwrap();
        assert_eq!(c.values.len(), 401);
        for (i, &v) in c.values.iter().enumerate() {
            let lag = i.cast_signed() - 200;
            if lag.unsigned_abs() >= 128 {
                assert_eq!(v, 0.0, "lag {lag} must be 0.0");
                assert_eq!(c.overlap_energy[i], 0.0);
            }
        }
        // 自己相関の lag=0 は 1.0。
        assert!((c.values[200] - 1.0).abs() < 1e-12);
        // 空窓は全 0（one-shot は with_max_lag で容量を確保する）。
        let curve = overlap_corr_curve(&[], &[], 4).unwrap();
        assert_eq!(curve, vec![0.0; 9]);
        // new の容量（2L−1）を超える max_lag は Err。
        let mut ws = OverlapCorrWorkspace::new(&plan);
        assert_eq!(
            plan.curve(&mut ws, &x, &x, 200).err(),
            Some(Error::InvalidValue("max_lag"))
        );
    }

    /// x/y・workspace・容量の検証は panic ではなく Err。
    #[test]
    fn overlap_corr_shape_mismatches_are_errors() {
        let plan = OverlapCorrPlan::new(64).unwrap();
        let mut ws = OverlapCorrWorkspace::new(&plan);
        let x = noise(64, 43);
        let y = noise(64, 45);
        assert_eq!(
            plan.curve(&mut ws, &x[..63], &y, 4).err(),
            Some(Error::InvalidValue("x"))
        );
        assert_eq!(
            plan.curve(&mut ws, &x, &y[..63], 4).err(),
            Some(Error::InvalidValue("y"))
        );
        let other = OverlapCorrPlan::new(32).unwrap();
        let mut other_ws = OverlapCorrWorkspace::new(&other);
        assert_eq!(
            plan.curve(&mut other_ws, &x, &y, 4).err(),
            Some(Error::InvalidValue("workspace"))
        );
        // one-shot は x/y 等長を要求。
        assert_eq!(
            overlap_corr_curve(&x[..63], &y, 4),
            Err(Error::InvalidValue("y"))
        );
    }

    /// 同率最大値: 決定的カーブでは tied maxima は値の一致として
    /// そのまま観測できる（証拠を隠さない契約）。
    #[test]
    fn ncc_tied_maxima_are_observable_in_values() {
        // 同一 template を 2 箇所に埋め込み、両 lag で同じピーク値。
        let template = noise(16, 51);
        let mut signal = vec![0.0; 96];
        for (i, &v) in template.iter().enumerate() {
            signal[10 + i] = v;
            signal[60 + i] = v;
        }
        let curve = ncc(&template, &signal, usize::MAX).unwrap();
        assert!((curve[10] - 1.0).abs() < 1e-12);
        assert!((curve[60] - 1.0).abs() < 1e-12);
        // 両方が最大値として並存する（同率最大の検証は利用側の tie-break）。
        assert_eq!(curve[10], curve[60]);
    }
}
