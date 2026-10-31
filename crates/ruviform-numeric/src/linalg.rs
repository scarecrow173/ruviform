//! 小列数の Householder QR による ridge 付き最小二乗
//! （`AudioLens` `al-math::linalg` の移植）。求解と正規行列による診断は
//! 独立している。
//!
//! # エラー契約
//!
//! 「妥当な入力だが解が存在しない」（ランク落ち・非有限・形状違反）は
//! `Option::None` で返す — 本モジュールのみ `Result` ではなく `Option`
//! を使う（`condition_number` は `f64::INFINITY` センチネル）。
//!
//! **RT 契約:** 全関数は確保を伴うため **non-RT only**。

use crate::vec::{dot_seq, sub_scaled_seq, sum_sq_seq};

// `v` と `target` は同じ suffix 長（m+extra-j）を持ち、等長契約は呼出し側
// の構造で保証される。
fn reflect(v: &[f64], target: &mut [f64]) {
    let factor = 2.0 * dot_seq(v, target);
    sub_scaled_seq(target, factor, v);
}

/// 元の観測列と反射を所有し、複数 rhs に再利用する分解。
///
/// **RT 契約:** 構築（[`LeastSquares::from_columns`]）も求解
/// （[`LeastSquares::solve`]）も確保を伴うため **non-RT only**。
pub struct LeastSquares {
    // ridge 適用前の入力列（1 コピーのみ、RSS 用）。
    obs_columns: Vec<Vec<f64>>,
    // ridge augmentation 付き分解後の列（R 情報）。
    columns: Vec<Vec<f64>>,
    reflectors: Vec<Vec<f64>>,
}

impl LeastSquares {
    /// 有限・等長の列、非負有限の ridge、観測数（列長）>= 列数を要求する。
    ///
    /// 契約違反・ランク落ち・非有限は `None`。
    #[must_use]
    pub fn from_columns(columns: &[&[f64]], lambda: f64) -> Option<Self> {
        let m = columns.first()?.len();
        if m < columns.len()
            || !lambda.is_finite()
            || lambda < 0.0
            || columns
                .iter()
                .any(|c| c.len() != m || c.iter().any(|x| !x.is_finite()))
        {
            return None;
        }
        let n = columns.len();
        let extra = if lambda > 0.0 { n } else { 0 };
        // 分解用は入力列のコピーに √λ 対角の extra 要素を末尾へ追加した
        // augmented 列（長さ m+extra）。augmentation は分解内部のみに存在する。
        let mut factored: Vec<Vec<f64>> = columns
            .iter()
            .enumerate()
            .map(|(j, c)| {
                let mut c = c.to_vec();
                c.extend((0..extra).map(|i| if i == j { lambda.sqrt() } else { 0.0 }));
                c
            })
            .collect();
        let scale = factored
            .iter()
            .flat_map(|c| c.iter())
            .fold(0.0_f64, |s, x| s.max(x.abs()));
        #[expect(clippy::cast_precision_loss)]
        let tolerance = f64::EPSILON * (m + extra).max(n) as f64 * scale;
        let mut reflectors = Vec::with_capacity(n);
        for j in 0..n {
            // hypot でノルム計算時の二乗による overflow/underflow を避ける。
            let norm = factored[j][j..].iter().fold(0.0_f64, |s, &x| s.hypot(x));
            if !norm.is_finite() || norm <= tolerance {
                return None;
            }
            let sign = if factored[j][j] >= 0.0 { 1.0 } else { -1.0 };
            let mut v: Vec<f64> = factored[j][j..].iter().map(|x| x / norm).collect();
            v[0] += sign;
            let vnorm = dot_seq(&v, &v).sqrt();
            for x in &mut v {
                *x /= vnorm;
            }
            for c in &mut factored[j..] {
                reflect(&v, &mut c[j..]);
            }
            factored[j][j] = -sign * norm;
            factored[j][j + 1..].fill(0.0);
            reflectors.push(v);
        }
        Some(Self {
            obs_columns: columns.iter().map(|c| c.to_vec()).collect(),
            columns: factored,
            reflectors,
        })
    }

    /// 元の観測のみから残差 RMS を求める（ridge 項を含めない）。
    ///
    /// `rhs` の長さが観測数と異なる・非有限を含む・解が非有限なら `None`。
    #[must_use]
    #[expect(clippy::cast_precision_loss)]
    pub fn solve(&self, rhs: &[f64]) -> Option<(Vec<f64>, f64)> {
        if rhs.len() != self.obs_columns[0].len() || rhs.iter().any(|x| !x.is_finite()) {
            return None;
        }
        let f = self;
        let n = f.columns.len();
        let mut y = rhs.to_vec();
        y.resize(f.columns[0].len(), 0.0);
        for (j, v) in f.reflectors.iter().enumerate() {
            reflect(v, &mut y[j..]);
        }
        let mut x = vec![0.0; n];
        for j in (0..n).rev() {
            let sum: f64 = (j + 1..n).map(|k| f.columns[k][j] * x[k]).sum();
            x[j] = (y[j] - sum) / f.columns[j][j];
        }
        // RSS は連続バッファ上の per-column `sub_scaled` パスで計算する。
        // ridge 項は含めない。
        let mut resid = rhs.to_vec();
        for (j, &xj) in x.iter().enumerate() {
            sub_scaled_seq(&mut resid, xj, &f.obs_columns[j]);
        }
        let rss = sum_sq_seq(&resid);
        let rms = (rss / rhs.len() as f64).sqrt();
        if x.iter().any(|v| !v.is_finite()) || !rms.is_finite() {
            return None;
        }
        Some((x, rms))
    }
}

/// 単一 rhs の ridge 最小二乗求解。
///
/// [`LeastSquares::from_columns`] / [`LeastSquares::solve`] の one-shot 版。
#[must_use]
pub fn least_squares(columns: &[&[f64]], rhs: &[f64], lambda: f64) -> Option<(Vec<f64>, f64)> {
    LeastSquares::from_columns(columns, lambda)?.solve(rhs)
}

/// 正方行列のスペクトル条件数推定。`AᵀA` への power iteration と
/// `(AᵀA)⁻¹` への power iteration の比から最大/最小特異値比を近似する。
/// 特異・非正方・空入力は `f64::INFINITY`。求解のランク判定には
/// 使用しない（対称 `AᵀA` 入力を想定するが任意の正方行列を受ける）。
#[must_use]
#[expect(clippy::cast_precision_loss)]
pub fn condition_number(a: &[Vec<f64>]) -> f64 {
    let n = a.len();
    if n == 0 || a.iter().any(|r| r.len() != n) {
        return f64::INFINITY;
    }
    let ata: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| (0..n).map(|k| a[k][i] * a[k][j]).sum())
                .collect()
        })
        .collect();
    let power = |m: &[Vec<f64>]| -> f64 {
        let mut v = vec![1.0 / (n as f64).sqrt(); n];
        let mut lambda = 0.0;
        for _ in 0..64 {
            let mut w = vec![0.0; n];
            for (i, row) in m.iter().enumerate() {
                w[i] = row.iter().zip(&v).map(|(a, b)| a * b).sum();
            }
            let norm = w.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm == 0.0 {
                return 0.0;
            }
            for x in &mut w {
                *x /= norm;
            }
            lambda = norm;
            v = w;
        }
        lambda
    };
    let max_sv2 = power(&ata);
    if max_sv2 <= 0.0 {
        return f64::INFINITY;
    }
    let Some(inv) = invert(&ata) else {
        return f64::INFINITY;
    };
    let min_sv2_inv = power(&inv);
    if min_sv2_inv <= 0.0 {
        return f64::INFINITY;
    }
    (max_sv2 * min_sv2_inv).sqrt()
}

/// Gauss-Jordan による正方行列の逆行列。特異行列は `None`。
#[expect(clippy::needless_range_loop)]
fn invert(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    if n == 0 || a.iter().any(|r| r.len() != n) {
        return None;
    }
    let mut m: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            a[i].iter()
                .copied()
                .chain((0..n).map(|j| f64::from(u8::from(i == j))))
                .collect()
        })
        .collect();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
        if m[pivot][col].abs() < 1e-12 {
            return None;
        }
        m.swap(col, pivot);
        let inv = 1.0 / m[col][col];
        for c in 0..2 * n {
            m[col][c] *= inv;
        }
        for row in 0..n {
            if row == col {
                continue;
            }
            let f = m[row][col];
            for c in 0..2 * n {
                m[row][c] -= f * m[col][c];
            }
        }
    }
    Some(m.into_iter().map(|row| row[n..].to_vec()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: &[f64], b: &[f64], context: &str) {
        assert_eq!(a.len(), b.len(), "{context}: length");
        for (i, (&a, &b)) in a.iter().zip(b).enumerate() {
            let tol = 1e-10 * (1.0 + a.abs() + b.abs());
            assert!((a - b).abs() <= tol, "{context}[{i}]: {a} vs {b}");
        }
    }

    #[test]
    fn exact_fit_recovers_coefficients_with_zero_residual() {
        let c1 = [1.0, 2.0, 3.0, 4.0, 5.0];
        let c2 = [0.0, 1.0, 0.0, 1.0, 0.0];
        let c3 = [1.0, -1.0, 2.0, -2.0, 1.0];
        // y = 2·c1 + 3·c2 − 0.5·c3。
        let y: Vec<f64> = (0..5)
            .map(|i| 2.0 * c1[i] + 3.0 * c2[i] - 0.5 * c3[i])
            .collect();
        let (x, rms) = least_squares(&[&c1, &c2, &c3], &y, 0.0).unwrap();
        assert_close(&x, &[2.0, 3.0, -0.5], "exact fit");
        assert!(rms < 1e-12, "rms {rms}");
    }

    #[test]
    fn noisy_fit_reports_observation_residual_only() {
        // 1 列モデル: x·[1,1,1,1] ≈ y。λ=1 → x = 4/(4+1) = 0.8。
        let col = [1.0; 4];
        let y = [1.0; 4];
        let (x, rms) = least_squares(&[&col], &y, 1.0).unwrap();
        assert!((x[0] - 0.8).abs() < 1e-12, "x = {}", x[0]);
        // 残差 RMS は元の観測のみから: resid = 0.2 各 → rms = 0.2。
        // ridge augmentation 行を含めると 0.4 になるため、この値は
        // 「ridge を残差に含めない」契約を識別する。
        assert!((rms - 0.2).abs() < 1e-12, "rms {rms}");
        // λ=0 では x = 1、rms ≈ 0。
        let (x0, rms0) = least_squares(&[&col], &y, 0.0).unwrap();
        assert!((x0[0] - 1.0).abs() < 1e-12);
        assert!(rms0 < 1e-12);
    }

    #[test]
    fn factorization_is_reusable_across_rhs() {
        let c1 = [1.0, 0.0, 1.0, 0.0];
        let c2 = [0.0, 1.0, 1.0, 1.0];
        let ls = LeastSquares::from_columns(&[&c1, &c2], 0.0).unwrap();
        let (xa, _) = ls.solve(&[1.0, 1.0, 2.0, 1.0]).unwrap();
        let (xb, _) = ls.solve(&[2.0, -1.0, 1.0, -1.0]).unwrap();
        // y1 = [1,1,2,1] = c1 + c2 → x ≈ [1,1]。
        assert_close(&xa, &[1.0, 1.0], "rhs a");
        // y2 = [2,-1,1,-1] = 2·c1 − 1·c2 → x ≈ [2,-1]。
        assert_close(&xb, &[2.0, -1.0], "rhs b");
        // 長さ不一致・非有限 rhs は None。
        assert!(ls.solve(&[1.0, 2.0]).is_none());
        assert!(ls.solve(&[1.0, 2.0, f64::NAN, 4.0]).is_none());
    }

    #[test]
    fn invalid_and_degenerate_inputs_are_none() {
        let col = [1.0, 2.0, 3.0];
        let short = [1.0, 2.0];
        // 空列・m < n・ragged・非有限・負/非有限 λ はすべて None。
        assert!(LeastSquares::from_columns(&[], 0.0).is_none());
        assert!(LeastSquares::from_columns(&[&short, &short, &short], 0.0).is_none());
        assert!(LeastSquares::from_columns(&[&col, &short], 0.0).is_none());
        let nan_col = [1.0, f64::NAN, 3.0];
        assert!(LeastSquares::from_columns(&[&nan_col], 0.0).is_none());
        let inf_col = [1.0, f64::INFINITY, 3.0];
        assert!(LeastSquares::from_columns(&[&inf_col], 0.0).is_none());
        assert!(LeastSquares::from_columns(&[&col], -1.0).is_none());
        assert!(LeastSquares::from_columns(&[&col], f64::NAN).is_none());
        // ランク落ち（同じ列を複製）は λ=0 で None、λ>0 で解ける。
        let dup = [2.0, 4.0, 6.0];
        assert!(LeastSquares::from_columns(&[&col, &dup], 0.0).is_none());
        assert!(LeastSquares::from_columns(&[&col, &dup], 1e-3).is_some());
        // 全ゼロ列もランク落ち。
        let zero = [0.0; 3];
        assert!(LeastSquares::from_columns(&[&zero], 0.0).is_none());
    }

    #[test]
    fn condition_number_estimates_spectral_ratio() {
        let ident = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        assert!((condition_number(&ident) - 1.0).abs() < 1e-6);
        // 対角 [3, 0.25] → σ比 = 12。
        let diag = vec![vec![3.0, 0.0], vec![0.0, 0.25]];
        assert!((condition_number(&diag) - 12.0).abs() < 1e-6);
        // 特異・非正方・空は INFINITY センチネル。
        let singular = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
        assert_eq!(condition_number(&singular), f64::INFINITY);
        let ragged = vec![vec![1.0, 2.0], vec![1.0]];
        assert_eq!(condition_number(&ragged), f64::INFINITY);
        assert_eq!(condition_number(&[]), f64::INFINITY);
    }

    #[test]
    fn tall_rectangular_system_uses_only_m_observations() {
        // m=8 観測・n=2 列。y = 0.5·c1 + 1.5·c2 + 小ノイズ。
        let c1: Vec<f64> = (0..8).map(f64::from).collect();
        let c2 = [1.0; 8];
        let noise = [0.01, -0.01, 0.02, -0.02, 0.01, 0.0, -0.01, 0.0];
        let y: Vec<f64> = (0..8)
            .map(|i| 0.5 * c1[i] + 1.5 * c2[i] + noise[i])
            .collect();
        let (x, rms) = least_squares(&[&c1, &c2], &y, 0.0).unwrap();
        // 最小二乗解は係数 [0.5, 1.5] の近傍。
        assert!((x[0] - 0.5).abs() < 0.01, "x0 {}", x[0]);
        assert!((x[1] - 1.5).abs() < 0.05, "x1 {}", x[1]);
        // 残差はノイズの射影成分、RMS はそれ以下に収まる。
        let noise_rms = (noise.iter().map(|v| v * v).sum::<f64>() / 8.0).sqrt();
        assert!(rms <= noise_rms + 1e-12, "rms {rms} vs {noise_rms}");
        assert!(rms > 0.0);
    }
}
