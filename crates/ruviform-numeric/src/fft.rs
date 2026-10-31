//! 2 のべき乗長の `f64` 複素 FFT と実数 FFT。
//!
//! アルゴリズムは `AudioLens` `al-math::fft` の移植（radix-4 DIF 融合段 +
//! 終端 radix-2）。scalar 版は全要素を逐次評価し、al-math の `pulp::Scalar`
//! 経路と同じ演算順（非融合 `a*b ± c*d`）で全プラットフォーム bit-exact。
//!
//! # 規約
//!
//! - 複素数は実部・虚部を分離した等長スライス（split re/im）で受け渡す。
//! - forward は負符号回転・正規化なし、inverse は正符号回転・`1/N` 正規化。
//! - 実数 FFT の出力は非負周波数 `N/2+1` bin（k=0 が DC、k=N/2 が Nyquist）。
//! - 長さ調整・ゼロ埋めは呼び出し側の責務。入出力長は plan の `n` に完全一致。
//! - plan（不変・`&self` 共有可）と workspace（`&mut` 排他）を分離する。
//!   al-math が大変換で使う `thread_local` 暗黙確保は、呼び出し側所有の
//!   [`FftWorkspace`] / [`RealFftWorkspace`] に置き換えてある。
//!
//! # RT 契約
//!
//! plan・workspace の構築は確保を伴うため non-RT。変換 API（`transform`・
//! `forward`・`inverse`）は事前確保済み workspace で alloc なし・有界に動作する
//! **RT-suitable**。workspace を変換中の確保・借用なしで再利用できる。

use crate::{Error, Result};
use std::f64::consts::TAU;

// 副表間のパディング要素数。64B ずらしで別の L1 セットに乗せる
// （al-math と同じ twiddle レイアウトを維持し、将来の SIMD 移植と
// bit-exact 比較を可能にする）。
const TW_PAD: usize = 8;
// 1 stage の twiddle セグメント長。副表は q = len/4 要素 + pad。
#[inline]
fn tw_segment(q: usize) -> usize {
    4 * (q + TW_PAD)
}
// セグメント内の副表オフセット。[w1r|w1i|w2r|w2i] が pad 区切りで並ぶ。
#[inline]
fn tw_offset(idx: usize, q: usize) -> usize {
    idx * (q + TW_PAD)
}

// workspace 経路を使う最小長。小さな変換は L1/L2 に収まり skew の効果より
// 確保・複写コストが支配的になるため in-place のままとする。
const WS_MIN: usize = 1 << 14;
// workspace の im 部を re からずらす要素数。512B で 4K 境界の一致を解く。
const WS_SKEW: usize = 64;

/// `exp(−2πi·k/n)` の前半周期を `count` 個生成する。
#[expect(clippy::cast_precision_loss)]
fn roots(n: usize, count: usize) -> (Vec<f64>, Vec<f64>) {
    let angle = -TAU / n as f64;
    let step_re = angle.cos();
    let step_im = angle.sin();
    let mut re = Vec::with_capacity(count);
    let mut im = Vec::with_capacity(count);
    let (mut w_re, mut w_im) = (1.0, 0.0);
    for k in 0..count {
        // 長い逐次再帰の振幅・位相ドリフトを抑える。
        if k % 256 == 0 {
            let a = angle * k as f64;
            w_re = a.cos();
            w_im = a.sin();
        }
        re.push(w_re);
        im.push(w_im);
        // w *= step。複素乗算と同じ乗算・加算順を保つ。
        let next_re = w_re * step_re - w_im * step_im;
        w_im = w_re * step_im + w_im * step_re;
        w_re = next_re;
    }
    (re, im)
}

/// 複素 FFT の不変係数（plan）。複数の変換・スレッドで `&` 共有できる。
///
/// stage ごとに `[w1 re | pad | w1 im | pad | w2 re | pad | w2 im | pad]` の
/// twiddle 副表を保持する。
pub struct FftPlan {
    n: usize,
    twiddles: Vec<f64>,
}

/// 複素 FFT の排他的作業領域。
///
/// `n < WS_MIN` では in-place 変換のため `stage` は空。`n >= WS_MIN` では
/// `2n + WS_SKEW` 要素の skew 付き領域を持ち、呼び出し側配列と独立した
/// 4K 境界回避配置で stage を回す。構築は確保を伴うため **non-RT**。
pub struct FftWorkspace {
    n: usize,
    stage: Vec<f64>,
}

/// 不変 plan と排他的 workspace を所有する複素 FFT の便宜型。
pub struct Fft {
    plan: FftPlan,
    workspace: FftWorkspace,
}

impl FftPlan {
    /// `n` 点複素 FFT の plan を構築する。`n` は非ゼロの 2 のべき乗
    /// （`n = 1` を含む）。
    ///
    /// **RT 契約:** non-RT only — 確保する。
    ///
    /// # Errors
    ///
    /// `n` が 2 のべき乗でなければ `Error::InvalidValue("n")`。
    pub fn new(n: usize) -> Result<Self> {
        if !n.is_power_of_two() {
            return Err(Error::InvalidValue("n"));
        }
        let mut twiddles = Vec::new();
        let mut len = n;
        while len > 2 {
            let q = len / 4;
            // w1 = roots(len)[0..q]、w2 = roots(len/2)[0..q] (= w_len^{2k})。
            let (w1r, w1i) = roots(len, q);
            let (w2r, w2i) = roots(len / 2, q);
            for part in [&w1r, &w1i, &w2r, &w2i] {
                twiddles.extend(part.iter().copied());
                twiddles.resize(twiddles.len() + TW_PAD, 0.0);
            }
            len /= 4;
        }
        Ok(Self { n, twiddles })
    }

    /// 変換長 `n`。
    #[must_use]
    pub fn n(&self) -> usize {
        self.n
    }

    /// plan が保持する係数メモリ量（バイト、比較記録用の構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.twiddles.len() * size_of::<f64>()
    }

    /// 自然順序の in-place 変換。`re`/`im` は長さ `n` の等長スライス。
    ///
    /// **RT 契約:** RT-suitable — 事前確保済み workspace のみ使用し、
    /// alloc なし・有界。`workspace` は [`FftWorkspace::new`] で本 plan 用に
    /// 構築したものを渡す。
    ///
    /// # Errors
    ///
    /// `re`/`im` の長さが `n` と異なれば `Error::InvalidValue("length")`。
    /// `workspace` が本 plan 用でなければ `Error::InvalidValue("workspace")`。
    #[expect(clippy::cast_precision_loss)]
    pub fn transform(
        &self,
        workspace: &mut FftWorkspace,
        re: &mut [f64],
        im: &mut [f64],
        inverse: bool,
    ) -> Result<()> {
        if re.len() != self.n || im.len() != self.n {
            return Err(Error::InvalidValue("length"));
        }
        if workspace.n != self.n {
            return Err(Error::InvalidValue("workspace"));
        }
        let n = self.n;
        let tw = &self.twiddles;
        if n < WS_MIN {
            if inverse {
                transform_inplace::<true>(tw, re, im);
            } else {
                transform_inplace::<false>(tw, re, im);
            }
            return Ok(());
        }
        // 呼び出し側の re/im が同一ページオフセットに揃うと、大きな len の
        // stage で quarter stride が 4K の倍数となりデータストリームが同一
        // L1/L2 セットへ集中する。skew した workspace で stage を回し、
        // DIF 出力の digit-reversed 置換を gather 書き戻しに畳み込む。
        debug_assert_eq!(workspace.stage.len(), 2 * n + WS_SKEW);
        let (wre, rest) = workspace.stage.split_at_mut(n + WS_SKEW);
        let wre = &mut wre[..n];
        let wim = &mut rest[..n];
        if inverse {
            stage_out::<true>(re, im, wre, wim, tw, n);
            butterfly_from::<true>(wre, wim, &tw[tw_segment(n / 4)..], n / 4);
        } else {
            stage_out::<false>(re, im, wre, wim, tw, n);
            butterfly_from::<false>(wre, wim, &tw[tw_segment(n / 4)..], n / 4);
        }
        let scale = if inverse { 1.0 / n as f64 } else { 1.0 };
        gather_permute(wre, wim, re, im, scale);
        Ok(())
    }
}

impl FftWorkspace {
    /// `plan` に対応する作業領域を全確保する。
    ///
    /// **RT 契約:** non-RT only — `n >= WS_MIN` で `2n + WS_SKEW` 要素を確保
    /// （小さな `n` では確保しない）。
    #[must_use]
    pub fn new(plan: &FftPlan) -> Self {
        Self {
            n: plan.n,
            stage: vec![
                0.0;
                if plan.n >= WS_MIN {
                    2 * plan.n + WS_SKEW
                } else {
                    0
                }
            ],
        }
    }

    /// 保持する作業領域のメモリ量（バイト）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.stage.len() * size_of::<f64>()
    }
}

impl Fft {
    /// `n` 点複素 FFT を構築する（plan + workspace の内蔵）。`n` は非ゼロの
    /// 2 のべき乗。
    ///
    /// **RT 契約:** non-RT only — 確保する。
    ///
    /// # Errors
    ///
    /// [`FftPlan::new`] と同じ。
    pub fn new(n: usize) -> Result<Self> {
        let plan = FftPlan::new(n)?;
        let workspace = FftWorkspace::new(&plan);
        Ok(Self { plan, workspace })
    }

    /// 変換長 `n`。
    #[must_use]
    pub fn n(&self) -> usize {
        self.plan.n
    }

    /// [`FftPlan::transform`] と同じ変換を内蔵 workspace で行う。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// [`FftPlan::transform`] と同じ（workspace は常に整合済み）。
    pub fn transform(&mut self, re: &mut [f64], im: &mut [f64], inverse: bool) -> Result<()> {
        self.plan.transform(&mut self.workspace, re, im, inverse)
    }
}

// (ar·br − ai·bi, ar·bi + ai·br) の複素乗算。非融合の乗算・加算順で、
// al-math の scalar 端数経路（cmul1）とビット一致する。
#[inline]
fn cmul1(ar: f64, ai: f64, br: f64, bi: f64) -> (f64, f64) {
    (ar * br - ai * bi, ar * bi + ai * br)
}

// radix-2 の 2 段を融合する radix-4 DIF 段の in-place 版。
// 出力の組順は 0,2,1,3 で binary bit reversal を保つ。
#[expect(clippy::similar_names)]
fn stage_inplace<const INVERSE: bool>(re: &mut [f64], im: &mut [f64], tw: &[f64], len: usize) {
    if len == 2 {
        for (pair_re, pair_im) in re
            .as_chunks_mut::<2>()
            .0
            .iter_mut()
            .zip(im.as_chunks_mut::<2>().0.iter_mut())
        {
            let ar = pair_re[0];
            let ai = pair_im[0];
            pair_re[0] = ar + pair_re[1];
            pair_im[0] = ai + pair_im[1];
            pair_re[1] = ar - pair_re[1];
            pair_im[1] = ai - pair_im[1];
        }
        return;
    }
    let q = len / 4;
    let w1r = &tw[tw_offset(0, q)..tw_offset(0, q) + q];
    let w1i = &tw[tw_offset(1, q)..tw_offset(1, q) + q];
    let w2r = &tw[tw_offset(2, q)..tw_offset(2, q) + q];
    let w2i = &tw[tw_offset(3, q)..tw_offset(3, q) + q];
    // 全要素が scalar 端数経路と同じ演算順で処理される。
    let sign = if INVERSE { 1.0 } else { -1.0 };
    for (block_re, block_im) in re.chunks_exact_mut(len).zip(im.chunks_exact_mut(len)) {
        let (a_re, tail) = block_re.split_at_mut(q);
        let (b_re, tail) = tail.split_at_mut(q);
        let (c_re, d_re) = tail.split_at_mut(q);
        let (a_im, tail) = block_im.split_at_mut(q);
        let (b_im, tail) = tail.split_at_mut(q);
        let (c_im, d_im) = tail.split_at_mut(q);
        for k in 0..q {
            let (w1rk, w1ik) = if INVERSE {
                (w1r[k], -w1i[k])
            } else {
                (w1r[k], w1i[k])
            };
            let (w2rk, w2ik) = if INVERSE {
                (w2r[k], -w2i[k])
            } else {
                (w2r[k], w2i[k])
            };
            let s0r = a_re[k] + c_re[k];
            let s0i = a_im[k] + c_im[k];
            let s1r = b_re[k] + d_re[k];
            let s1i = b_im[k] + d_im[k];
            let d0r = a_re[k] - c_re[k];
            let d0i = a_im[k] - c_im[k];
            let e1r = b_re[k] - d_re[k];
            let e1i = b_im[k] - d_im[k];
            // ±i 回転: forward は (e1i, −e1r)、inverse は (−e1i, e1r)。
            let d1r = -sign * e1i;
            let d1i = sign * e1r;
            a_re[k] = s0r + s1r;
            a_im[k] = s0i + s1i;
            let (vr, vi) = cmul1(s0r - s1r, s0i - s1i, w2rk, w2ik);
            b_re[k] = vr;
            b_im[k] = vi;
            let (vr, vi) = cmul1(d0r + d1r, d0i + d1i, w1rk, w1ik);
            c_re[k] = vr;
            c_im[k] = vi;
            // d 項は w1 適用後に w2 を掛ける（w12 を先に積算しない — al-math
            // scalar 端数と同じ演算順を保つ）。
            let (vr, vi) = cmul1(d0r - d1r, d0i - d1i, w1rk, w1ik);
            let (vr, vi) = cmul1(vr, vi, w2rk, w2ik);
            d_re[k] = vr;
            d_im[k] = vi;
        }
    }
}

// `stage_inplace` と同じ演算を src → dst の out-of-place で行う。
// workspace 経路の先頭 stage で使い、入力側配列からの複写 pass を省く。
// radix-4 に限定する（len == 2 の終端は `stage_inplace` 側が担う）。
#[expect(clippy::similar_names)]
fn stage_out<const INVERSE: bool>(
    src_re: &[f64],
    src_im: &[f64],
    dst_re: &mut [f64],
    dst_im: &mut [f64],
    tw: &[f64],
    len: usize,
) {
    debug_assert!(len >= 4);
    let q = len / 4;
    let w1r = &tw[tw_offset(0, q)..tw_offset(0, q) + q];
    let w1i = &tw[tw_offset(1, q)..tw_offset(1, q) + q];
    let w2r = &tw[tw_offset(2, q)..tw_offset(2, q) + q];
    let w2i = &tw[tw_offset(3, q)..tw_offset(3, q) + q];
    let sign = if INVERSE { 1.0 } else { -1.0 };
    for (((block_src_re, block_src_im), block_dst_re), block_dst_im) in src_re
        .chunks_exact(len)
        .zip(src_im.chunks_exact(len))
        .zip(dst_re.chunks_exact_mut(len))
        .zip(dst_im.chunks_exact_mut(len))
    {
        let (s_a_re, tail) = block_src_re.split_at(q);
        let (s_b_re, tail) = tail.split_at(q);
        let (s_c_re, s_d_re) = tail.split_at(q);
        let (s_a_im, tail) = block_src_im.split_at(q);
        let (s_b_im, tail) = tail.split_at(q);
        let (s_c_im, s_d_im) = tail.split_at(q);
        let (d_a_re, tail) = block_dst_re.split_at_mut(q);
        let (d_b_re, tail) = tail.split_at_mut(q);
        let (d_c_re, d_d_re) = tail.split_at_mut(q);
        let (d_a_im, tail) = block_dst_im.split_at_mut(q);
        let (d_b_im, tail) = tail.split_at_mut(q);
        let (d_c_im, d_d_im) = tail.split_at_mut(q);
        for k in 0..q {
            let (w1rk, w1ik) = if INVERSE {
                (w1r[k], -w1i[k])
            } else {
                (w1r[k], w1i[k])
            };
            let (w2rk, w2ik) = if INVERSE {
                (w2r[k], -w2i[k])
            } else {
                (w2r[k], w2i[k])
            };
            let s0r = s_a_re[k] + s_c_re[k];
            let s0i = s_a_im[k] + s_c_im[k];
            let s1r = s_b_re[k] + s_d_re[k];
            let s1i = s_b_im[k] + s_d_im[k];
            let d0r = s_a_re[k] - s_c_re[k];
            let d0i = s_a_im[k] - s_c_im[k];
            let e1r = s_b_re[k] - s_d_re[k];
            let e1i = s_b_im[k] - s_d_im[k];
            let d1r = -sign * e1i;
            let d1i = sign * e1r;
            d_a_re[k] = s0r + s1r;
            d_a_im[k] = s0i + s1i;
            let (vr, vi) = cmul1(s0r - s1r, s0i - s1i, w2rk, w2ik);
            d_b_re[k] = vr;
            d_b_im[k] = vi;
            let (vr, vi) = cmul1(d0r + d1r, d0i + d1i, w1rk, w1ik);
            d_c_re[k] = vr;
            d_c_im[k] = vi;
            let (vr, vi) = cmul1(d0r - d1r, d0i - d1i, w1rk, w1ik);
            let (vr, vi) = cmul1(vr, vi, w2rk, w2ik);
            d_d_re[k] = vr;
            d_d_im[k] = vi;
        }
    }
}

// len から開始する stage 列。workspace 経路では先頭 stage が out-of-place
// のため、残りを len = n/4 から始める。
fn butterfly_from<const INVERSE: bool>(
    re: &mut [f64],
    im: &mut [f64],
    mut tw: &[f64],
    mut len: usize,
) {
    while len > 1024 {
        stage_inplace::<INVERSE>(re, im, tw, len);
        tw = &tw[tw_segment(len / 4)..];
        len /= 4;
    }
    for (block_re, block_im) in re.chunks_exact_mut(len).zip(im.chunks_exact_mut(len)) {
        let mut rest = tw;
        let mut size = len;
        while size > 1 {
            stage_inplace::<INVERSE>(block_re, block_im, rest, size);
            let used = if size == 2 { 0 } else { tw_segment(size / 4) };
            rest = &rest[used..];
            size = if size == 2 { 1 } else { size / 4 };
        }
    }
}

// re/im の同一 index スワップ。
#[inline]
fn swap_pair(re: &mut [f64], im: &mut [f64], i: usize, j: usize) {
    let ar = re[i];
    let br = re[j];
    let ai = im[i];
    let bi = im[j];
    re[i] = br;
    im[i] = bi;
    re[j] = ar;
    im[j] = ai;
}

// DIF 出力の digit-reversed 置換（in-place）。
fn permute(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    if n < 1024 {
        for i in 1..n {
            let j = i.reverse_bits() >> (usize::BITS - n.trailing_zeros());
            if i < j {
                swap_pair(re, im, i, j);
            }
        }
        return;
    }
    // 高位/低位 5 bit をタイル化し、相互の行をキャッシュに留める。
    let middle_bits = n.trailing_zeros() - 10;
    for middle in 0..(n >> 10) {
        let reversed = if middle_bits == 0 {
            0
        } else {
            middle.reverse_bits() >> (usize::BITS - middle_bits)
        };
        if middle > reversed {
            continue;
        }
        for high in 0usize..32 {
            let low_out = high.reverse_bits() >> (usize::BITS - 5);
            for low in 0usize..32 {
                let high_out = low.reverse_bits() >> (usize::BITS - 5);
                let i = (high << (middle_bits + 5)) | (middle << 5) | low;
                let j = (high_out << (middle_bits + 5)) | (reversed << 5) | low_out;
                if middle < reversed || i < j {
                    swap_pair(re, im, i, j);
                }
            }
        }
    }
}

// workspace 上の digit-reversed 出力を呼び出し側配列へ書き戻す。
// in-place 置換の代わりに sequential write + gather read で済ませ、
// inverse の 1/N スケールもここへ畳み込む。
fn gather_permute(wre: &[f64], wim: &[f64], re: &mut [f64], im: &mut [f64], scale: f64) {
    let n = re.len();
    // workspace 経路 (n >= WS_MIN) からのみ呼ばれるため、タイル化の
    // 前提は n >= 1024。小さな変換は in-place `transform` が置換を担う。
    debug_assert!(n >= 1024);
    let middle_bits = n.trailing_zeros() - 10;
    for middle in 0..(n >> 10) {
        let reversed = if middle_bits == 0 {
            0
        } else {
            middle.reverse_bits() >> (usize::BITS - middle_bits)
        };
        for high in 0usize..32 {
            let low_out = high.reverse_bits() >> (usize::BITS - 5);
            for low in 0usize..32 {
                let high_out = low.reverse_bits() >> (usize::BITS - 5);
                let i = (high << (middle_bits + 5)) | (middle << 5) | low;
                let j = (high_out << (middle_bits + 5)) | (reversed << 5) | low_out;
                re[i] = wre[j] * scale;
                im[i] = wim[j] * scale;
            }
        }
    }
}

// 小変換（n < WS_MIN）の in-place 経路: butterfly + 置換 + inverse スケール。
#[expect(clippy::cast_precision_loss)]
fn transform_inplace<const INVERSE: bool>(tw: &[f64], re: &mut [f64], im: &mut [f64]) {
    butterfly_from::<INVERSE>(re, im, tw, re.len());
    permute(re, im);
    if INVERSE {
        // n は 2 のべき乗のため 1/n は厳密に表現でき、*= と /= は同じ結果。
        let scale = 1.0 / re.len() as f64;
        for v in re.iter_mut() {
            *v *= scale;
        }
        for v in im.iter_mut() {
            *v *= scale;
        }
    }
}

/// 実数 FFT の不変係数（plan）。複数スレッドで `&` 共有できる。
///
/// 非負周波数 `N/2+1` bin（`N=1` は 1 bin）を split re/im で扱う。
/// 内部に `m = max(N/2, 1)` 点の [`FftPlan`] と組立て係数を持つ。
pub struct RealFftPlan {
    n: usize,
    fft: FftPlan,
    roots: Vec<f64>,
}

/// 実数 FFT の排他的作業領域。
///
/// packing 用 `scratch`（`2m + WS_SKEW` 要素）と、内部 `m` 点複素 FFT の
/// [`FftWorkspace`] を保持する。構築は確保を伴うため **non-RT**。
pub struct RealFftWorkspace {
    n: usize,
    scratch: Vec<f64>,
    fft_ws: FftWorkspace,
}

/// 不変 plan と排他的 workspace を所有する実数 FFT の便宜型。
pub struct RealFft {
    plan: RealFftPlan,
    workspace: RealFftWorkspace,
}

impl RealFftPlan {
    /// `n` 点実数 FFT の plan を構築する。`n` は非ゼロの 2 のべき乗
    /// （`n = 1` を含む）。
    ///
    /// **RT 契約:** non-RT only — 確保する。
    ///
    /// # Errors
    ///
    /// `n` が 2 のべき乗でなければ `Error::InvalidValue("n")`。
    pub fn new(n: usize) -> Result<Self> {
        if !n.is_power_of_two() {
            return Err(Error::InvalidValue("n"));
        }
        let (re, im) = roots(n, n / 2 + 1);
        let m = (n / 2).max(1);
        let mut roots = Vec::with_capacity(2 * (n / 2 + 1) + TW_PAD);
        // 組立て係数 (0.5·im, −0.5·re) を split 格納する。
        roots.extend(im.iter().map(|&w| 0.5 * w));
        roots.resize(roots.len() + TW_PAD, 0.0);
        roots.extend(re.iter().map(|&w| -0.5 * w));
        Ok(Self {
            n,
            fft: FftPlan::new(m)?,
            roots,
        })
    }

    /// 時間域の変換長 `n`。
    #[must_use]
    pub fn n(&self) -> usize {
        self.n
    }

    /// 周波数域の bin 数（`n/2 + 1`）。
    #[must_use]
    pub fn spectrum_len(&self) -> usize {
        self.n / 2 + 1
    }

    /// plan が保持する係数メモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.roots.len() * size_of::<f64>() + self.fft.memory_bytes()
    }

    /// 実数入力から非負周波数 `n/2+1` bin へ変換する。
    ///
    /// **RT 契約:** RT-suitable — 事前確保済み workspace のみ使用し、
    /// alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `input` が `n` と異なる長さ、`out_re`/`out_im` が `n/2+1` と異なる
    /// 長さなら `Error::InvalidValue("length")`。`workspace` が本 plan 用で
    /// なければ `Error::InvalidValue("workspace")`。
    pub fn forward(
        &self,
        workspace: &mut RealFftWorkspace,
        input: &[f64],
        out_re: &mut [f64],
        out_im: &mut [f64],
    ) -> Result<()> {
        let m = self.n / 2;
        if workspace.n != self.n {
            return Err(Error::InvalidValue("workspace"));
        }
        if input.len() != self.n || out_re.len() != m + 1 || out_im.len() != m + 1 {
            return Err(Error::InvalidValue("length"));
        }
        if self.n == 1 {
            out_re[0] = input[0];
            out_im[0] = 0.0;
            return Ok(());
        }
        let RealFftWorkspace {
            scratch, fft_ws, ..
        } = workspace;
        let (sre, sim_) = {
            let (re, rest) = scratch.split_at_mut(m);
            (re, &mut rest[WS_SKEW..WS_SKEW + m])
        };
        // 偶数標本を re、奇数標本を im へ詰め込む。
        for i in 0..m {
            sre[i] = input[2 * i];
            sim_[i] = input[2 * i + 1];
        }
        self.fft.transform(fft_ws, sre, sim_, false)?;
        let zr = sre[0];
        let zi = sim_[0];
        out_re[0] = zr + zi;
        out_im[0] = 0.0;
        out_re[m] = zr - zi;
        out_im[m] = 0.0;
        if m > 1 {
            let mid = m / 2;
            // 自己共役 bin は回転しない。
            out_re[mid] = sre[mid];
            out_im[mid] = -sim_[mid];
            // low bin (1..mid) に鏡像共役 Z[m−k] を先行充填し、
            // recombine_pair で上書きする。
            for k in 1..mid {
                out_re[k] = sre[m - k];
                out_im[k] = -sim_[m - k];
            }
            let roots_re = &self.roots[..=m];
            let roots_im = &self.roots[m + 1 + TW_PAD..=m + 1 + TW_PAD + m];
            let (low_re, high_re) = out_re[1..m].split_at_mut(mid);
            let (low_im, high_im) = out_im[1..m].split_at_mut(mid);
            let low_re = &mut low_re[..mid - 1];
            let low_im = &mut low_im[..mid - 1];
            recombine_pair(
                &sre[1..mid],
                &sim_[1..mid],
                &roots_re[1..mid],
                &roots_im[1..mid],
                low_re,
                low_im,
                high_re,
                high_im,
            );
            high_re.reverse();
            high_im.reverse();
        }
        Ok(())
    }

    /// 非負周波数 `n/2+1` bin から実数列へ `1/N` 正規化して復元する。
    ///
    /// DC / Nyquist の虚部は読まない（実数列では常に 0）。`in_re` と `in_im`
    /// は同一スライスを共有してはならない。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `output` が `n`、`in_re`/`in_im` が `n/2+1` と異なる長さなら
    /// `Error::InvalidValue("length")`。`in_re`/`in_im` が同一スライスなら
    /// `Error::InvalidValue("in_im")`。`workspace` が本 plan 用でなければ
    /// `Error::InvalidValue("workspace")`。
    pub fn inverse(
        &self,
        workspace: &mut RealFftWorkspace,
        in_re: &[f64],
        in_im: &[f64],
        output: &mut [f64],
    ) -> Result<()> {
        let m = self.n / 2;
        if workspace.n != self.n {
            return Err(Error::InvalidValue("workspace"));
        }
        if output.len() != self.n || in_re.len() != m + 1 || in_im.len() != m + 1 {
            return Err(Error::InvalidValue("length"));
        }
        if core::ptr::eq(in_re, in_im) {
            return Err(Error::InvalidValue("in_im"));
        }
        if self.n == 1 {
            output[0] = in_re[0];
            return Ok(());
        }
        let RealFftWorkspace {
            scratch, fft_ws, ..
        } = workspace;
        let (sre, sim_) = {
            let (re, rest) = scratch.split_at_mut(m);
            (re, &mut rest[WS_SKEW..WS_SKEW + m])
        };
        // DC/Nyquist から半復元し、残り bin に鏡像共役を充填する。
        sre[0] = in_re[0].midpoint(in_re[m]);
        sim_[0] = (in_re[0] - in_re[m]) * 0.5;
        for k in 1..m {
            sre[k] = in_re[m - k];
            sim_[k] = -in_im[m - k];
        }
        let roots_re = &self.roots[..=m];
        let roots_im = &self.roots[m + 1 + TW_PAD..=m + 1 + TW_PAD + m];
        recombine_inverse(
            &in_re[1..m],
            &in_im[1..m],
            &roots_re[1..m],
            &roots_im[1..m],
            &mut sre[1..m],
            &mut sim_[1..m],
        );
        self.fft.transform(fft_ws, sre, sim_, true)?;
        for i in 0..m {
            output[2 * i] = sre[i];
            output[2 * i + 1] = sim_[i];
        }
        Ok(())
    }
}

impl RealFftWorkspace {
    /// `plan` に必要な packing と大変換用 stage 領域を全確保する。
    ///
    /// **RT 契約:** non-RT only — `2m + WS_SKEW` 要素（および `m >= WS_MIN`
    /// で同等の stage 領域）を確保する。
    #[must_use]
    pub fn new(plan: &RealFftPlan) -> Self {
        let m = (plan.n / 2).max(1);
        Self {
            n: plan.n,
            scratch: vec![0.0; 2 * m + WS_SKEW],
            fft_ws: FftWorkspace::new(&plan.fft),
        }
    }

    /// 保持する作業領域のメモリ量（バイト）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        (self.scratch.len() + self.fft_ws.stage.len()) * size_of::<f64>()
    }
}

impl RealFft {
    /// `n` 点実数 FFT を構築する（plan + workspace の内蔵）。`n` は非ゼロの
    /// 2 のべき乗。
    ///
    /// **RT 契約:** non-RT only — 確保する。
    ///
    /// # Errors
    ///
    /// [`RealFftPlan::new`] と同じ。
    pub fn new(n: usize) -> Result<Self> {
        let plan = RealFftPlan::new(n)?;
        let workspace = RealFftWorkspace::new(&plan);
        Ok(Self { plan, workspace })
    }

    /// 時間域の変換長 `n`。
    #[must_use]
    pub fn n(&self) -> usize {
        self.plan.n
    }

    /// 周波数域の bin 数（`n/2 + 1`）。
    #[must_use]
    pub fn spectrum_len(&self) -> usize {
        self.plan.spectrum_len()
    }

    /// [`RealFftPlan::forward`] と同じ変換を内蔵 workspace で行う。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// [`RealFftPlan::forward`] と同じ（workspace は常に整合済み）。
    pub fn forward(&mut self, input: &[f64], out_re: &mut [f64], out_im: &mut [f64]) -> Result<()> {
        self.plan
            .forward(&mut self.workspace, input, out_re, out_im)
    }

    /// [`RealFftPlan::inverse`] と同じ変換を内蔵 workspace で行う。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// [`RealFftPlan::inverse`] と同じ（workspace は常に整合済み）。
    pub fn inverse(&mut self, in_re: &[f64], in_im: &[f64], output: &mut [f64]) -> Result<()> {
        self.plan.inverse(&mut self.workspace, in_re, in_im, output)
    }
}

// forward の共役対組立て。和/差と twiddle 積を共有して low/high を同時に
// 求める。low 側には鏡像共役が先行充填済みであること。
#[expect(clippy::too_many_arguments)]
fn recombine_pair(
    in_re: &[f64],
    in_im: &[f64],
    roots_re: &[f64],
    roots_im: &[f64],
    low_re: &mut [f64],
    low_im: &mut [f64],
    high_re: &mut [f64],
    high_im: &mut [f64],
) {
    for i in 0..in_re.len() {
        let ar = in_re[i];
        let ai = in_im[i];
        let wr = roots_re[i];
        let wi = roots_im[i];
        let br = low_re[i];
        let bi = low_im[i];
        let sum_r = ar.midpoint(br);
        let sum_i = ai.midpoint(bi);
        let (dfr, dfi) = cmul1(wr, wi, ar - br, ai - bi);
        low_re[i] = sum_r + dfr;
        low_im[i] = sum_i + dfi;
        // high 側は conj(sum − diff)。
        high_re[i] = sum_r - dfr;
        high_im[i] = dfi - sum_i;
    }
}

// inverse の鏡像組立て。mir_* は反転済み共役 spectrum が先行充填済みで、
// in-place に復元規則を適用する。twiddle の虚部は符号反転して使う。
fn recombine_inverse(
    in_re: &[f64],
    in_im: &[f64],
    roots_re: &[f64],
    roots_im: &[f64],
    mir_re: &mut [f64],
    mir_im: &mut [f64],
) {
    for i in 0..in_re.len() {
        let ar = in_re[i];
        let ai = in_im[i];
        let wr = roots_re[i];
        let wi = -roots_im[i];
        let br = mir_re[i];
        let bi = mir_im[i];
        let (dfr, dfi) = cmul1(wr, wi, ar - br, ai - bi);
        mir_re[i] = ar.midpoint(br) + dfr;
        mir_im[i] = ai.midpoint(bi) + dfi;
    }
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    // 定義どおりの O(n²) DFT oracle。アルゴリズムに忠実で実装を共有しない。
    fn naive_dft(re: &[f64], im: &[f64], inverse: bool) -> (Vec<f64>, Vec<f64>) {
        let n = re.len();
        let sign = if inverse { 1.0 } else { -1.0 };
        let scale = if inverse { 1.0 / n as f64 } else { 1.0 };
        let mut ore = vec![0.0; n];
        let mut oim = vec![0.0; n];
        for k in 0..n {
            let mut sr = 0.0;
            let mut si = 0.0;
            for (j, (&ar, &ai)) in re.iter().zip(im).enumerate() {
                let ang = sign * 2.0 * PI * (j * k) as f64 / n as f64;
                let (c, s) = (ang.cos(), ang.sin());
                sr += ar * c - ai * s;
                si += ar * s + ai * c;
            }
            ore[k] = sr * scale;
            oim[k] = si * scale;
        }
        (ore, oim)
    }

    // 実数 DFT oracle（非負周波数 n/2+1 bin）。
    fn naive_rdft(input: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let n = input.len();
        let bins = n / 2 + 1;
        let mut re = vec![0.0; bins];
        let mut im = vec![0.0; bins];
        for k in 0..bins {
            let (mut sr, mut si) = (0.0, 0.0);
            for (j, &x) in input.iter().enumerate() {
                let ang = 2.0 * PI * (j * k) as f64 / n as f64;
                sr += x * ang.cos();
                si -= x * ang.sin();
            }
            re[k] = sr;
            im[k] = si;
        }
        (re, im)
    }

    // 等価性 predicate: |a - b| <= 1e-10 * (1 + |a| + |b|)（al-math 準拠）。
    fn assert_equiv(a: f64, b: f64, context: &str) {
        let tol = 1e-10 * (1.0 + a.abs() + b.abs());
        assert!(
            (a - b).abs() <= tol,
            "{context}: {a} vs {b} (diff {} > tol {tol})",
            (a - b).abs()
        );
    }

    fn assert_equiv_slices(a: &[f64], b: &[f64], context: &str) {
        assert_eq!(a.len(), b.len(), "{context}: length mismatch");
        for (i, (&a, &b)) in a.iter().zip(b).enumerate() {
            assert_equiv(a, b, &format!("{context}[{i}]"));
        }
    }

    fn test_signal(n: usize) -> (Vec<f64>, Vec<f64>) {
        let re: Vec<f64> = (0..n).map(|i| (i as f64 * 0.7).sin() - 0.2).collect();
        let im: Vec<f64> = (0..n).map(|i| (i as f64 * 0.3).cos() + 0.1).collect();
        (re, im)
    }

    #[test]
    fn complex_forward_matches_naive_dft() {
        // n <= 512 までは O(n²) oracle と突き合わせる（n=1, 2 を含む全冪）。
        for power in 0..=9 {
            let n = 1 << power;
            let (mut re, mut im) = test_signal(n);
            let (ere, eim) = naive_dft(&re, &im, false);
            let plan = FftPlan::new(n).unwrap();
            let mut ws = FftWorkspace::new(&plan);
            plan.transform(&mut ws, &mut re, &mut im, false).unwrap();
            assert_equiv_slices(&re, &ere, &format!("forward re n={n}"));
            assert_equiv_slices(&im, &eim, &format!("forward im n={n}"));
        }
    }

    #[test]
    fn complex_round_trip_recovers_input_across_all_sizes() {
        // n = 1..=2^17。n >= WS_MIN は workspace 経路を通る。
        for power in 0..=17 {
            let n = 1 << power;
            let (orig_re, orig_im) = test_signal(n);
            let (mut re, mut im) = (orig_re.clone(), orig_im.clone());
            let mut fft = Fft::new(n).unwrap();
            fft.transform(&mut re, &mut im, false).unwrap();
            fft.transform(&mut re, &mut im, true).unwrap();
            for i in 0..n {
                assert_equiv(re[i], orig_re[i], &format!("roundtrip re n={n}[{i}]"));
                assert_equiv(im[i], orig_im[i], &format!("roundtrip im n={n}[{i}]"));
            }
        }
    }

    #[test]
    fn workspace_path_matches_known_transforms() {
        // n >= WS_MIN の解析的な既知変換: impulse → 全 bin 1+0i、
        // DC → bin 0 のみ非零、単位冪 exp → 単一 bin。
        for power in [14, 15, 17] {
            let n = 1 << power;
            let mut fft = Fft::new(n).unwrap();

            // impulse: X[k] = 1。
            let mut re = vec![0.0; n];
            let mut im = vec![0.0; n];
            re[0] = 1.0;
            fft.transform(&mut re, &mut im, false).unwrap();
            assert_equiv_slices(&re, &vec![1.0; n], &format!("impulse re n={n}"));
            assert_equiv_slices(&im, &vec![0.0; n], &format!("impulse im n={n}"));

            // DC: X[0] = n、他は 0。
            let mut re = vec![1.0; n];
            let mut im = vec![0.0; n];
            fft.transform(&mut re, &mut im, false).unwrap();
            assert_equiv(re[0], n as f64, &format!("dc n={n}"));
            assert_eq!(im[0], 0.0);
            assert_equiv_slices(&re[1..], &vec![0.0; n - 1], &format!("dc re rest n={n}"));

            // x[j] = exp(+2πi·5j/n) → X[5] = n、他は 0。
            // （exp(−2πi·5j/n) は負周波数で bin n−5 に落ちる）。
            let mut re: Vec<f64> = (0..n)
                .map(|j| (2.0 * PI * 5.0 * j as f64 / n as f64).cos())
                .collect();
            let mut im: Vec<f64> = (0..n)
                .map(|j| (2.0 * PI * 5.0 * j as f64 / n as f64).sin())
                .collect();
            fft.transform(&mut re, &mut im, false).unwrap();
            assert_equiv(re[5], n as f64, &format!("tone re n={n}"));
            for k in 0..n {
                if k == 5 {
                    continue;
                }
                // 大きい n では FFT 丸め誤差のノイズフロアが n 比例で
                // 増えるため n 相対の許容を取る。
                assert!(
                    re[k].abs() < n as f64 * 1e-12,
                    "tone re n={n}[{k}]: {}",
                    re[k]
                );
                assert!(
                    im[k].abs() < n as f64 * 1e-12,
                    "tone im n={n}[{k}]: {}",
                    im[k]
                );
            }
        }
    }

    #[test]
    fn inverse_scales_by_one_over_n_and_conjugates() {
        // forward の出力を反転入力にしても inverse は 1/N・共役規約で
        // 元に戻る（forward→inverse の round-trip と別経路の検証として、
        // naive IDFT oracle と突き合わせる）。
        let n = 64;
        let (mut re, mut im) = test_signal(n);
        let (ere, eim) = naive_dft(&re, &im, true);
        let mut fft = Fft::new(n).unwrap();
        fft.transform(&mut re, &mut im, true).unwrap();
        assert_equiv_slices(&re, &ere, "inverse re");
        assert_equiv_slices(&im, &eim, "inverse im");
    }

    #[test]
    fn real_forward_matches_naive_rdft() {
        for power in 0..=9 {
            let n = 1 << power;
            let (input, _) = test_signal(n);
            let (ere, eim) = naive_rdft(&input);
            let plan = RealFftPlan::new(n).unwrap();
            let mut ws = RealFftWorkspace::new(&plan);
            let mut re = vec![0.0; plan.spectrum_len()];
            let mut im = vec![0.0; plan.spectrum_len()];
            plan.forward(&mut ws, &input, &mut re, &mut im).unwrap();
            assert_equiv_slices(&re, &ere, &format!("rforward re n={n}"));
            assert_equiv_slices(&im, &eim, &format!("rforward im n={n}"));
        }
    }

    #[test]
    fn real_round_trip_and_dc_nyquist_placement() {
        for power in 0..=17 {
            let n = 1 << power;
            let input: Vec<f64> = (0..n).map(|i| (i as f64 * 0.37).sin() + 0.4).collect();
            let mut fft = RealFft::new(n).unwrap();
            let bins = n / 2 + 1;
            let mut re = vec![0.0; bins];
            let mut im = vec![0.0; bins];
            fft.forward(&input, &mut re, &mut im).unwrap();
            // DC/Nyquist の虚部は常に 0（実数列の共役対称）。
            assert_eq!(im[0], 0.0);
            assert_eq!(im[bins - 1], 0.0);
            // DC bin は入力の総和に等しい。
            let dc: f64 = input.iter().sum();
            assert_equiv(re[0], dc, &format!("dc n={n}"));
            let mut output = vec![0.0; n];
            fft.inverse(&re, &im, &mut output).unwrap();
            for i in 0..n {
                assert_equiv(output[i], input[i], &format!("iroundtrip n={n}[{i}]"));
            }
        }
    }

    #[test]
    fn real_fft_workspace_path_matches_naive_on_impulse_and_tone() {
        // n >= WS_MIN の実数 FFT。内部 m = n/2 も WS_MIN 超えで複素
        // workspace 経路を通る。
        for power in [15, 17] {
            let n = 1 << power;
            let mut fft = RealFft::new(n).unwrap();
            let bins = n / 2 + 1;

            // impulse: 全 bin 1+0i。
            let mut input = vec![0.0; n];
            input[0] = 1.0;
            let mut re = vec![0.0; bins];
            let mut im = vec![0.0; bins];
            fft.forward(&input, &mut re, &mut im).unwrap();
            assert_equiv_slices(&re, &vec![1.0; bins], &format!("rimpulse re n={n}"));
            assert_equiv_slices(&im, &vec![0.0; bins], &format!("rimpulse im n={n}"));

            // 単一正弦 bin: cos(2π·7j/n) → X[7] = X[n−7] = n/2。
            let input: Vec<f64> = (0..n)
                .map(|j| (2.0 * PI * 7.0 * j as f64 / n as f64).cos())
                .collect();
            fft.forward(&input, &mut re, &mut im).unwrap();
            assert_equiv(re[7], n as f64 / 2.0, &format!("rtone n={n}"));
            assert_equiv(im[7], 0.0, &format!("rtone im n={n}"));
            for k in 0..bins {
                if k == 7 {
                    continue;
                }
                // n 比例の丸め誤差フロアを許容する。
                assert!(
                    re[k].abs() < n as f64 * 1e-12,
                    "rtone re n={n}[{k}]: {}",
                    re[k]
                );
                assert!(
                    im[k].abs() < n as f64 * 1e-12,
                    "rtone im n={n}[{k}]: {}",
                    im[k]
                );
            }
        }
    }

    #[test]
    fn real_sample_regression_spectral_peaks() {
        // 決定的な疑似実信号（複数正弦 + DC）で、既知スペクトル配置と
        // Parseval エネルギー保存を検証する。
        let n = 4096;
        let input: Vec<f64> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64;
                0.25 + (2.0 * PI * 3.0 * t).sin()
                    + 0.5 * (2.0 * PI * 11.0 * t).cos()
                    + 0.25 * (2.0 * PI * 37.0 * t).sin()
            })
            .collect();
        let mut fft = RealFft::new(n).unwrap();
        let bins = n / 2 + 1;
        let mut re = vec![0.0; bins];
        let mut im = vec![0.0; bins];
        fft.forward(&input, &mut re, &mut im).unwrap();
        // bin 配置: DC=0.25n、|X[3]|=n/2、|X[11]|=n/4、|X[37]|=n/8。
        assert_equiv(re[0], 0.25 * n as f64, "dc");
        let mag = |k: usize| re[k].hypot(im[k]);
        assert_equiv(mag(3), n as f64 / 2.0, "bin3");
        assert_equiv(mag(11), n as f64 / 4.0, "bin11");
        assert_equiv(mag(37), n as f64 / 8.0, "bin37");
        // ピーク以外はほぼ 0。
        for k in [1, 2, 4, 10, 12, 36, 38, 500, bins - 2] {
            assert!(mag(k) < n as f64 * 1e-10, "bin {k} leaks: {}", mag(k));
        }
        // Parseval: Σx² = (|X[0]|² + |X[N/2]|² + 2·Σ_{0<k<N/2}|X[k]|²)/n。
        let time_energy: f64 = input.iter().map(|v| v * v).sum();
        let freq_energy: f64 = (re[0] * re[0]
            + re[bins - 1] * re[bins - 1]
            + 2.0
                * (1..bins - 1)
                    .map(|k| re[k] * re[k] + im[k] * im[k])
                    .sum::<f64>())
            / n as f64;
        assert_equiv(time_energy, freq_energy, "parseval");
    }

    #[test]
    fn n1_degenerates_to_identity() {
        // n=1: forward は [x] → re=x、im=0。inverse は x を復元。
        let mut fft = RealFft::new(1).unwrap();
        let mut re = [0.0; 1];
        let mut im = [0.0; 1];
        fft.forward(&[3.5], &mut re, &mut im).unwrap();
        assert_eq!(re[0], 3.5);
        assert_eq!(im[0], 0.0);
        let mut out = [0.0; 1];
        fft.inverse(&re, &im, &mut out).unwrap();
        assert_eq!(out[0], 3.5);

        let mut cfft = Fft::new(1).unwrap();
        let mut re = [2.0];
        let mut im = [-1.0];
        cfft.transform(&mut re, &mut im, false).unwrap();
        assert_eq!((re[0], im[0]), (2.0, -1.0));
        cfft.transform(&mut re, &mut im, true).unwrap();
        assert_eq!((re[0], im[0]), (2.0, -1.0));
    }

    #[test]
    fn invalid_lengths_and_shapes_are_errors() {
        for n in [0, 3, 6, 100] {
            assert_eq!(FftPlan::new(n).err(), Some(Error::InvalidValue("n")));
            assert_eq!(RealFftPlan::new(n).err(), Some(Error::InvalidValue("n")));
        }
        let plan = FftPlan::new(64).unwrap();
        let mut ws = FftWorkspace::new(&plan);
        let mut re = vec![0.0; 64];
        let mut im = vec![0.0; 64];
        assert_eq!(
            plan.transform(&mut ws, &mut re[..32], &mut im, false),
            Err(Error::InvalidValue("length"))
        );
        assert_eq!(
            plan.transform(&mut ws, &mut re, &mut im[..32], false),
            Err(Error::InvalidValue("length"))
        );
        // 別 plan の workspace は拒否される。
        let other_plan = FftPlan::new(128).unwrap();
        let mut wrong_ws = FftWorkspace::new(&other_plan);
        assert_eq!(
            plan.transform(&mut wrong_ws, &mut re, &mut im, false),
            Err(Error::InvalidValue("workspace"))
        );

        let rplan = RealFftPlan::new(64).unwrap();
        let mut rws = RealFftWorkspace::new(&rplan);
        let input = vec![0.0; 64];
        let mut sre = vec![0.0; 33];
        let mut sim = vec![0.0; 33];
        assert_eq!(
            rplan.forward(&mut rws, &input[..32], &mut sre, &mut sim),
            Err(Error::InvalidValue("length"))
        );
        assert_eq!(
            rplan.forward(&mut rws, &input, &mut sre[..16], &mut sim),
            Err(Error::InvalidValue("length"))
        );
        let shared = vec![0.0; 33];
        let mut out = vec![0.0; 64];
        // 同一スライス共有は in_im として拒否される。
        let shared2 = shared.clone();
        let in_re: &[f64] = &shared;
        let in_im: &[f64] = &shared;
        assert_eq!(
            rplan.inverse(&mut rws, in_re, in_im, &mut out),
            Err(Error::InvalidValue("in_im"))
        );
        let _ = shared2;
    }

    #[test]
    fn plans_are_shared_and_workspaces_exclusive() {
        // 不変 plan は複数 workspace で共有できる。
        let plan = FftPlan::new(128).unwrap();
        let mut ws_a = FftWorkspace::new(&plan);
        let mut ws_b = FftWorkspace::new(&plan);
        let (mut re1, mut im1) = test_signal(128);
        let (mut re2, mut im2) = test_signal(128);
        plan.transform(&mut ws_a, &mut re1, &mut im1, false)
            .unwrap();
        plan.transform(&mut ws_b, &mut re2, &mut im2, false)
            .unwrap();
        assert_eq!(re1, re2);
        assert_eq!(im1, im2);
    }
}
