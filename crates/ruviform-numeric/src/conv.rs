//! 実数 FFT による overlap-save 畳込み（AudioLens `al-locate::dsp::conv`
//! の移植を plan/workspace 構造に分離したもの）。
//!
//! 入力の先頭 `signal.len()` サンプル分の線形畳込みを返す。ブロック長は
//! `b = max(1024, next_pow2(2·m))`（m はカーネル長）で、各ブロックが
//! `l = b−m+1` の新規出力を生成する。
//!
//! # 規約
//!
//! - [`ConvPlan`] はカーネルスペクトルを構築時に 1 回変換して保持する。
//! - [`ConvWorkspace`] は畳込みに必要な全バッファを事前確保する。
//! - [`ConvPlan::convolve`] は事前確保済みバッファのみで動作する
//!   **RT-suitable**。one-shot の [`conv`] は確保を伴う **non-RT**。
//! - 空カーネルの線形畳込みは `signal` 長のゼロ列、空 `signal` は空列
//!   （[`conv`] の契約）。[`ConvPlan::new`] は空カーネルを拒否する
//!   （早期 return が `m−1` の underflow を防ぐ契約を型で表す）。

use crate::fft::{RealFftPlan, RealFftWorkspace};
use crate::{Error, Result};

/// overlap-save 畳込みの不変設定とカーネルスペクトル。
///
/// 複数の信号・スレッドで `&` 共有できる（排他状態は [`ConvWorkspace`]）。
pub struct ConvPlan {
    /// カーネル長。
    m: usize,
    /// ブロック長（`max(1024, next_pow2(2·m))`、[`RealFftPlan`] の変換長）。
    b: usize,
    /// 1 ブロックあたりの新規出力数（`b−m+1`）。
    l: usize,
    /// ブロック長の実数 FFT plan。
    fft: RealFftPlan,
    /// カーネルスペクトルの実部（`b/2+1` bin）。
    hre: Vec<f64>,
    /// カーネルスペクトルの虚部（`b/2+1` bin）。
    him: Vec<f64>,
}

/// overlap-save 畳込みの排他的作業領域。全バッファを事前確保する。
pub struct ConvWorkspace {
    /// 対応する [`ConvPlan::b`]。
    b: usize,
    fft_ws: RealFftWorkspace,
    /// 移動窓の入力ブロック（`b` 要素）。
    xbuf: Vec<f64>,
    /// ブロックスペクトル（`b/2+1` bin）。
    xre: Vec<f64>,
    xim: Vec<f64>,
    /// 逆変換出力ブロック（`b` 要素）。
    obuf: Vec<f64>,
}

impl ConvPlan {
    /// `kernel` との線形畳込み plan を構築する。
    ///
    /// **RT 契約:** non-RT only — カーネルスペクトルと plan を確保する。
    ///
    /// # Errors
    ///
    /// `kernel` が空なら `Error::InvalidValue("kernel")`。
    pub fn new(kernel: &[f64]) -> Result<Self> {
        if kernel.is_empty() {
            return Err(Error::InvalidValue("kernel"));
        }
        let m = kernel.len();
        // 2 冪のブロック長。小カーネルでは 1024 floor が呼出しオーバーヘッド
        // を償却する（m <= 4096 なら 8192 以下に収まる）。
        let b = (2 * m).next_power_of_two().max(1024);
        let l = b - m + 1;
        let fft = RealFftPlan::new(b)?;
        let bins = b / 2 + 1;
        // カーネルスペクトルはゼロ埋めして 1 回だけ変換する。
        let mut hbuf = vec![0.0; b];
        hbuf[..m].copy_from_slice(kernel);
        let mut hre = vec![0.0; bins];
        let mut him = vec![0.0; bins];
        fft.forward(&mut RealFftWorkspace::new(&fft), &hbuf, &mut hre, &mut him)?;
        Ok(Self {
            m,
            b,
            l,
            fft,
            hre,
            him,
        })
    }

    /// カーネル長。
    #[must_use]
    pub fn kernel_len(&self) -> usize {
        self.m
    }

    /// ブロック長（内部で使う FFT 変換長）。
    #[must_use]
    pub fn block_len(&self) -> usize {
        self.b
    }

    /// plan が保持する係数・スペクトルのメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        (self.hre.len() + self.him.len()) * size_of::<f64>() + self.fft.memory_bytes()
    }

    /// `signal` とカーネルの線形畳込みの先頭 `signal.len()` サンプルを
    /// `out` に書き込む。`out` の長さは `signal.len()` に一致しなければ
    /// ならない。
    ///
    /// **RT 契約:** RT-suitable — 事前確保済み workspace のみ使用し、
    /// alloc なし・有界。`signal` が空なら `out` も空で no-op。
    ///
    /// # Errors
    ///
    /// `out.len()` が `signal.len()` と異なれば
    /// `Error::InvalidValue("length")`。`workspace` が本 plan 用でなければ
    /// `Error::InvalidValue("workspace")`。
    pub fn convolve(
        &self,
        workspace: &mut ConvWorkspace,
        signal: &[f64],
        out: &mut [f64],
    ) -> Result<()> {
        if workspace.b != self.b {
            return Err(Error::InvalidValue("workspace"));
        }
        if out.len() != signal.len() {
            return Err(Error::InvalidValue("length"));
        }
        let n = signal.len();
        let m = self.m;
        let b = self.b;
        let l = self.l;
        let bins = b / 2 + 1;
        let ConvWorkspace {
            fft_ws,
            xbuf,
            xre,
            xim,
            obuf,
            ..
        } = workspace;
        // one-shot 契約: 前回呼出しの移動窓状態は持ち越さない。
        xbuf.fill(0.0);
        for off in (0..n).step_by(l) {
            let take = l.min(n - off);
            // 前ブロック末尾 m−1 入力を先頭へ運ぶ（初回はゼロ）、
            // `take` 個の新規標本を継ぎ、尾部をゼロ埋めする。
            xbuf.copy_within(b + 1 - m.., 0);
            xbuf[m - 1..m - 1 + take].copy_from_slice(&signal[off..off + take]);
            xbuf[m - 1 + take..].fill(0.0);
            self.fft.forward(fft_ws, xbuf, xre, xim)?;
            for k in 0..bins {
                let zre = xre[k] * self.hre[k] - xim[k] * self.him[k];
                let zim = xre[k] * self.him[k] + xim[k] * self.hre[k];
                xre[k] = zre;
                xim[k] = zim;
            }
            // `RealFftPlan::inverse` は 1/N 正規化込み。
            self.fft.inverse(fft_ws, xre, xim, obuf)?;
            // 先頭 m−1 出力は巡回畳込みの回り込みを含むため捨てる。
            out[off..off + take].copy_from_slice(&obuf[m - 1..m - 1 + take]);
        }
        Ok(())
    }
}

impl ConvWorkspace {
    /// `plan` に必要な全バッファを全確保する。
    ///
    /// **RT 契約:** non-RT only — `3b + 2(b/2+1)` 要素を確保する。
    #[must_use]
    pub fn new(plan: &ConvPlan) -> Self {
        let bins = plan.b / 2 + 1;
        Self {
            b: plan.b,
            fft_ws: RealFftWorkspace::new(&plan.fft),
            xbuf: vec![0.0; plan.b],
            xre: vec![0.0; bins],
            xim: vec![0.0; bins],
            obuf: vec![0.0; plan.b],
        }
    }

    /// 保持する作業領域のメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        (self.xbuf.len() + self.xre.len() + self.xim.len() + self.obuf.len()) * size_of::<f64>()
            + self.fft_ws.memory_bytes()
    }
}

/// `signal` と `kernel` の線形畳込みの先頭 `signal.len()` サンプルを返す
/// one-shot 版。
///
/// 空 `kernel` は `signal` 長のゼロ列、空 `signal` は空列を返す
/// （ブロック長計算より先に確定する契約）。
///
/// **RT 契約:** non-RT only — plan・workspace・出力を確保する。
#[must_use]
pub fn conv(signal: &[f64], kernel: &[f64]) -> Vec<f64> {
    if kernel.is_empty() {
        return vec![0.0; signal.len()];
    }
    if signal.is_empty() {
        return Vec::new();
    }
    // 非空 kernel では b >= 1024 が常に 2 のべき乗となり構築は失敗しない。
    let Ok(plan) = ConvPlan::new(kernel) else {
        return Vec::new();
    };
    let mut workspace = ConvWorkspace::new(&plan);
    let mut out = vec![0.0; signal.len()];
    // plan・workspace・out は構築直後で必ず整合するため失敗しない。
    let _ = plan.convolve(&mut workspace, signal, &mut out);
    out
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    /// O(n·m) の直接線形畳込み oracle。実装を共有しない。
    /// `out[i] = Σ_k signal[i−k]·kernel[k]`（i−k < 0 はゼロ拡張）。
    fn naive_conv(signal: &[f64], kernel: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; signal.len()];
        for (i, o) in out.iter_mut().enumerate() {
            let mut acc = 0.0;
            for (k, &h) in kernel.iter().enumerate() {
                if i >= k {
                    acc += signal[i - k] * h;
                }
            }
            *o = acc;
        }
        out
    }

    fn assert_close(a: &[f64], b: &[f64], context: &str) {
        assert_eq!(a.len(), b.len(), "{context}: length");
        for (i, (&a, &b)) in a.iter().zip(b).enumerate() {
            let tol = 1e-10 * (1.0 + a.abs() + b.abs());
            assert!((a - b).abs() <= tol, "{context}[{i}]: {a} vs {b}");
        }
    }

    #[test]
    fn matches_naive_convolution_across_shapes() {
        // ブロック境界を跨ぐ（n > l の多ブロック）・n < m・m = 1・
        // 部分ブロックを網羅する。
        let kernel: Vec<f64> = (0..257).map(|i| (f64::from(i) * 0.05).sin()).collect();
        for &n in &[1usize, 7, 767, 768, 1023, 2048, 3000] {
            // m=257 → b=1024、l=768。n=3000 で 4 ブロック + 端部を通る。
            let signal: Vec<f64> = (0..n).map(|i| (i as f64 * 0.31).cos()).collect();
            let got = conv(&signal, &kernel);
            assert_close(&got, &naive_conv(&signal, &kernel), &format!("n={n}"));
        }
        // カーネルがブロック長を超える場合: m=3000 → b=8192。
        let big_kernel: Vec<f64> = (0..3000).map(|i| (f64::from(i) * 0.011).sin()).collect();
        let signal: Vec<f64> = (0..1500).map(|i| (f64::from(i) * 0.17).cos()).collect();
        assert_close(
            &conv(&signal, &big_kernel),
            &naive_conv(&signal, &big_kernel),
            "m=3000",
        );
    }

    #[test]
    fn delta_and_scalar_kernels() {
        let signal: Vec<f64> = (0..40).map(|i| f64::from(i) * 0.5 - 10.0).collect();
        // 単位インパルスは信号をそのまま返す。
        assert_close(&conv(&signal, &[1.0]), &signal, "delta");
        // 長さ 1 のカーネルはスカラー乗算（l = b の 1 ブロック経路）。
        let scaled: Vec<f64> = signal.iter().map(|v| v * 2.5).collect();
        assert_close(&conv(&signal, &[2.5]), &scaled, "scalar");
        // 中央配置デルタは遅延。
        let delayed = conv(&signal, &[0.0, 0.0, 1.0]);
        for i in 0..38 {
            assert!((delayed[i + 2] - signal[i]).abs() < 1e-12, "delay[{i}]");
        }
    }

    #[test]
    fn empty_operands_have_defined_results() {
        // 空 kernel → signal 長のゼロ列。空 signal → 空列。
        assert_eq!(conv(&[1.0, 2.0], &[]), vec![0.0; 2]);
        assert_eq!(conv(&[], &[1.0]), Vec::<f64>::new());
        assert_eq!(conv(&[], &[]), Vec::<f64>::new());
        // 空 kernel の plan 構築は拒否される。
        assert_eq!(
            ConvPlan::new(&[]).err(),
            Some(Error::InvalidValue("kernel"))
        );
        // 空 signal + 有効 plan は no-op（out も空で受理）。
        let plan = ConvPlan::new(&[1.0]).unwrap();
        let mut ws = ConvWorkspace::new(&plan);
        plan.convolve(&mut ws, &[], &mut []).unwrap();
    }

    #[test]
    fn prepared_path_matches_one_shot_and_reuses_workspace() {
        let kernel: Vec<f64> = (0..100).map(|i| (f64::from(i) * 0.2).cos()).collect();
        let plan = ConvPlan::new(&kernel).unwrap();
        let mut ws = ConvWorkspace::new(&plan);
        // 異なる信号長で 2 回連続しても結果が安定する（one-shot 契約）。
        for &n in &[512usize, 1500] {
            let signal: Vec<f64> = (0..n).map(|i| (i as f64 * 0.13).sin()).collect();
            let mut out = vec![0.0; n];
            plan.convolve(&mut ws, &signal, &mut out).unwrap();
            assert_close(&out, &conv(&signal, &kernel), &format!("prepared n={n}"));
        }
        // 長さ不一致・別 plan の workspace はエラー。
        let signal = vec![0.0; 32];
        let mut short = vec![0.0; 31];
        assert_eq!(
            plan.convolve(&mut ws, &signal, &mut short),
            Err(Error::InvalidValue("length"))
        );
        let other = ConvPlan::new(&[1.0, 2.0]).unwrap();
        let mut other_ws = ConvWorkspace::new(&other);
        // 同じ b の plan 同士は入れ替えても動作する（workspace は b に紐づく）。
        assert_eq!(other.b, plan.b);
        plan.convolve(&mut other_ws, &signal, vec![0.0; 32].as_mut_slice())
            .unwrap();
        // 異なるカーネル長で b が変わる plan なら拒否される。
        let big = ConvPlan::new(&vec![1.0; 6000]).unwrap();
        let mut big_ws = ConvWorkspace::new(&big);
        assert_ne!(big.b, plan.b);
        assert_eq!(
            plan.convolve(&mut big_ws, &signal, vec![0.0; 32].as_mut_slice()),
            Err(Error::InvalidValue("workspace"))
        );
        let _ = other_ws;
    }

    #[test]
    fn block_shape_and_memory_reporting() {
        let plan = ConvPlan::new(&[0.5; 100]).unwrap();
        assert_eq!(plan.kernel_len(), 100);
        // m=100 → next_pow2(200)=256 → b = max(1024, 256) = 1024。
        assert_eq!(plan.block_len(), 1024);
        assert!(plan.memory_bytes() > 0);
        let ws = ConvWorkspace::new(&plan);
        assert!(ws.memory_bytes() > 0);
        let big = ConvPlan::new(&vec![1.0; 3000]).unwrap();
        assert_eq!(big.block_len(), 8192);
    }
}
