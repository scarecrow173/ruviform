//! 小列数の Householder QR による ridge 付き最小二乗と、部分 pivot
//! 選択付き Gaussian 消去による複素連立一次方程式ソルバ（`AudioLens`
//! `al-math::linalg`・`al-analysis::expensive::channel::csolve` の
//! 移植）。求解と正規行列による診断は独立している。
//!
//! # エラー契約
//!
//! 「妥当な入力だが解が存在しない」（ランク落ち・非有限・形状違反）は
//! `Option::None` で返す — 本モジュールのみ `Result` ではなく `Option`
//! を使う（`condition_number` は `f64::INFINITY` センチネル）。
//! `complex_solve` のみ例外で、移植元 `csolve` の失敗判定をそのまま
//! 維持するため非有限入力を拒否せず IEEE どおり伝播させる
//! （形状違反・特異のみ `None`）。
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

/// 複素連立一次方程式 `A·w = b` を部分 pivot 選択付き Gaussian 消去で
/// 解く（Issue #167）。`AudioLens`
/// `al-analysis::expensive::channel::csolve` の移植で、pivot 選択・
/// 消去順序・失敗判定を同一に保つ — 同じ入力に同じ解と条件数 proxy・
/// 失敗を返し、AL 側のローカル実装を bit-exact に置換できる。
///
/// 複素値は `(re, im)` タプルで受け渡す（本クレートは std のみに依存
/// し、`num-complex` は使わない）。`mat` は正方係数行列（row-major、
/// `mat.len() == n`・全行 `n` 要素）、`rhs` は右辺 `n` 要素を要求する。
///
/// 返却タプルの第 2 要素は条件数 proxy — 消去で現れた pivot 絶対値の
/// 最大/最小比。真のスペクトル条件数（[`condition_number`]）ではなく
/// 診断用の概算値。
///
/// # 失敗判定（`None`）
///
/// - `n == 0`・非正方または ragged な `mat`・`rhs.len() != n` の形状違反
/// - 消去中に pivot 絶対値の最大候補 `< 1e-30`（特異・数値的縮退）
///
/// 上記以外の検証は行わない。非有限（NaN / Inf）を含む入力は拒否せず
/// IEEE どおり解へ伝播させる — `csolve` と同じ失敗判定を維持するため
/// 有限性検査は入れない（`least_squares` 等とはこの点で契約が異なる）。
///
/// **RT 契約:** 確保を伴うため **non-RT only**。
///
/// # Examples
///
/// ```
/// use ruviform_numeric::linalg::complex_solve;
///
/// // 2·w0 = 2、(0+i)·w1 = (0+i) → w = [1, 1]。
/// let mat = vec![
///     vec![(2.0, 0.0), (0.0, 0.0)],
///     vec![(0.0, 0.0), (0.0, 1.0)],
/// ];
/// let rhs = vec![(2.0, 0.0), (0.0, 1.0)];
/// let (w, cond) = complex_solve(&mat, &rhs).unwrap();
/// assert_eq!(w, vec![(1.0, 0.0), (1.0, 0.0)]);
/// assert_eq!(cond, 2.0);
/// ```
#[must_use]
pub fn complex_solve(
    mat: &[Vec<(f64, f64)>],
    rhs: &[(f64, f64)],
) -> Option<(Vec<(f64, f64)>, f64)> {
    let dim = mat.len();
    if dim == 0 || rhs.len() != dim || mat.iter().any(|row| row.len() != dim) {
        return None;
    }
    // 拡大係数行列 [A | b]（行ごとに dim+1 要素）。
    let mut aug: Vec<Vec<(f64, f64)>> = mat
        .iter()
        .zip(rhs)
        .map(|(row, &rhs_i)| row.iter().copied().chain([rhs_i]).collect())
        .collect();
    let cabs = |(re, im): (f64, f64)| (re * re + im * im).sqrt();
    // x/y = x·conj(y)/|y|²。`fft::cmul1` と同じ非融合 a*b±c*d 形で
    // 評価順を固定し、csolve とビット一致させる。
    let cdiv = |(x_re, x_im): (f64, f64), (y_re, y_im): (f64, f64)| {
        let denom = y_re * y_re + y_im * y_im;
        (
            (x_re * y_re + x_im * y_im) / denom,
            (x_im * y_re - x_re * y_im) / denom,
        )
    };
    let cmul = |(x_re, x_im): (f64, f64), (y_re, y_im): (f64, f64)| {
        (x_re * y_re - x_im * y_im, x_re * y_im + x_im * y_re)
    };
    let (mut piv_max, mut piv_min) = (0.0f64, f64::MAX);
    for col in 0..dim {
        // 部分 pivot: |aug[行][col]| が最大の行。同値・NaN の順序は
        // `max_by` + `total_cmp` そのもの（csolve と同じ tie-break で
        // 同値最大は最後の index を選ぶ）。
        let pivot =
            (col..dim).max_by(|&ri, &rj| cabs(aug[ri][col]).total_cmp(&cabs(aug[rj][col])))?;
        let pmag = cabs(aug[pivot][col]);
        if pmag < 1e-30 {
            return None;
        }
        piv_max = piv_max.max(pmag);
        piv_min = piv_min.min(pmag);
        aug.swap(col, pivot);
        for row in col + 1..dim {
            let factor = cdiv(aug[row][col], aug[col][col]);
            let (head, tail) = aug.split_at_mut(row);
            let pivot_row = &head[col];
            for (ci, cell) in tail[0].iter_mut().enumerate().take(dim + 1).skip(col) {
                let prod = cmul(factor, pivot_row[ci]);
                *cell = (cell.0 - prod.0, cell.1 - prod.1);
            }
        }
    }
    // 後退代入。pivot 行を正規化しない LU 形で、aug[row][row] に残る
    // pivot 要素で最後に割る（csolve と同じ代入順序）。
    let mut sol = vec![(0.0, 0.0); dim];
    for row in (0..dim).rev() {
        let mut sum = (0.0, 0.0);
        for (&mat_ij, &sol_j) in aug[row][row + 1..dim].iter().zip(&sol[row + 1..dim]) {
            let prod = cmul(mat_ij, sol_j);
            sum = (sum.0 + prod.0, sum.1 + prod.1);
        }
        sol[row] = cdiv(
            (aug[row][dim].0 - sum.0, aug[row][dim].1 - sum.1),
            aug[row][row],
        );
    }
    Some((
        sol,
        if piv_min > 0.0 {
            piv_max / piv_min
        } else {
            f64::INFINITY
        },
    ))
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

    // テスト側の独立な複素乗算（残差検証を実装と別経路で行う）。
    fn cmul_t((x_re, x_im): (f64, f64), (y_re, y_im): (f64, f64)) -> (f64, f64) {
        (x_re * y_re - x_im * y_im, x_re * y_im + x_im * y_re)
    }

    fn cmat_vec(mat: &[Vec<(f64, f64)>], vec_in: &[(f64, f64)]) -> Vec<(f64, f64)> {
        mat.iter()
            .map(|row| {
                row.iter().zip(vec_in).fold((0.0, 0.0), |sum, (&ma, &wi)| {
                    let prod = cmul_t(ma, wi);
                    (sum.0 + prod.0, sum.1 + prod.1)
                })
            })
            .collect()
    }

    fn assert_complex_close(actual: &[(f64, f64)], expect: &[(f64, f64)], context: &str) {
        assert_eq!(actual.len(), expect.len(), "{context}: length");
        for (idx, (&ac, &ex)) in actual.iter().zip(expect).enumerate() {
            let tol = 1e-10 * (1.0 + ac.0.abs() + ac.1.abs() + ex.0.abs() + ex.1.abs());
            assert!(
                (ac.0 - ex.0).abs() <= tol && (ac.1 - ex.1).abs() <= tol,
                "{context}[{idx}]: {ac:?} vs {ex:?}"
            );
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

    #[test]
    fn complex_solve_recovers_known_solution_with_small_residual() {
        // 良条件の 3x3 複素系。既知 w から b = A·w を独立に算定し、
        // 解が一致し残差 ||b - A·x|| が小さいこと（= csolve と同じ
        // 数学的結果を返すこと）を確認する。
        let mat = vec![
            vec![(2.0, 0.5), (0.0, -1.0), (1.0, 0.25)],
            vec![(0.5, 0.0), (3.0, 1.0), (0.0, 0.5)],
            vec![(1.0, -0.25), (0.0, 2.0), (2.0, 0.0)],
        ];
        let want = vec![(1.0, 0.5), (2.0, -1.0), (0.5, 0.25)];
        let rhs = cmat_vec(&mat, &want);
        let (sol, cond) = complex_solve(&mat, &rhs).unwrap();
        assert_complex_close(&sol, &want, "known solution");
        let resid = cmat_vec(&mat, &sol);
        let rss: f64 = resid
            .iter()
            .zip(&rhs)
            .map(|(res, &bi)| (res.0 - bi.0) * (res.0 - bi.0) + (res.1 - bi.1) * (res.1 - bi.1))
            .sum();
        assert!(rss.sqrt() < 1e-12, "residual {}", rss.sqrt());
        assert!(cond.is_finite() && cond >= 1.0, "cond {cond}");
    }

    #[test]
    fn complex_solve_partial_pivot_avoids_tiny_column_entry() {
        // 部分 pivot: 列 0 の先頭候補が閾値未満でも |1| の行が pivot に
        // 選ばれて特異判定を回避する（csolve と同じ pivot 規則）。
        let mat = vec![vec![(1e-40, 0.0), (1.0, 0.0)], vec![(1.0, 0.0), (1.0, 0.0)]];
        let (sol, cond) = complex_solve(&mat, &[(1.0, 0.0), (2.0, 0.0)]).unwrap();
        assert_complex_close(&sol, &[(1.0, 0.0), (1.0, 0.0)], "pivot swap");
        assert_eq!(cond, 1.0);
        // 列の全候補が 1e-30 未満なら特異として None。
        let mat = vec![
            vec![(1e-40, 0.0), (1.0, 0.0)],
            vec![(1e-40, 0.0), (1.0, 0.0)],
        ];
        assert!(complex_solve(&mat, &[(1.0, 0.0), (2.0, 0.0)]).is_none());
    }

    #[test]
    fn complex_solve_uses_complex_division_in_elimination() {
        // (1+i)·w = 2 → w = 1-i。複素除算・後退代入経路の確認。
        let (sol, cond) = complex_solve(&[vec![(1.0, 1.0)]], &[(2.0, 0.0)]).unwrap();
        assert_complex_close(&sol, &[(1.0, -1.0)], "1x1 complex");
        assert_eq!(cond, 1.0);
    }

    #[test]
    fn complex_solve_condition_proxy_is_pivot_magnitude_ratio() {
        // 対角系では proxy は pivot 絶対値の比そのもの。
        let mat = vec![
            vec![(1e10, 0.0), (0.0, 0.0)],
            vec![(0.0, 0.0), (1e-20, 0.0)],
        ];
        let (sol, cond) = complex_solve(&mat, &[(1e10, 0.0), (1e-20, 0.0)]).unwrap();
        assert_complex_close(&sol, &[(1.0, 0.0), (1.0, 0.0)], "diag cond");
        // 1e10 / 1e-20 = 1e30（浮動小数点商のため相対誤差で比較）。
        assert!((cond / 1e30 - 1.0).abs() < 1e-12, "cond {cond}");
    }

    #[test]
    fn complex_solve_singular_and_invalid_shapes_are_none() {
        // ランク落ち（2 行目 = 2·1 行目）は消去で pivot が消え None。
        let singular = vec![vec![(1.0, 0.0), (2.0, 1.0)], vec![(2.0, 0.0), (4.0, 2.0)]];
        assert!(complex_solve(&singular, &[(1.0, 0.0), (2.0, 0.0)]).is_none());
        // n == 0・非正方・ragged・b 長不一致はすべて None。
        assert!(complex_solve(&[], &[]).is_none());
        let wide = vec![vec![(1.0, 0.0); 3]; 2];
        assert!(complex_solve(&wide, &[(1.0, 0.0); 2]).is_none());
        let ragged = vec![vec![(1.0, 0.0); 2], vec![(1.0, 0.0)]];
        assert!(complex_solve(&ragged, &[(1.0, 0.0); 2]).is_none());
        let square = vec![vec![(1.0, 0.0); 2]; 2];
        assert!(complex_solve(&square, &[(1.0, 0.0); 3]).is_none());
        assert!(complex_solve(&square, &[(1.0, 0.0)]).is_none());
    }

    #[test]
    fn complex_solve_propagates_non_finite_like_csolve() {
        // 移植元 csolve の失敗判定を維持するため有限性検査は行わない。
        // NaN は `total_cmp` で最大扱いとなり pivot に選ばれ、解へ
        // 伝播する — None ではなく非有限値を含む Some が返る契約。
        let mat = vec![
            vec![(f64::NAN, 0.0), (0.0, 0.0)],
            vec![(0.0, 0.0), (1.0, 0.0)],
        ];
        let (sol, _cond) = complex_solve(&mat, &[(1.0, 0.0), (1.0, 0.0)]).unwrap();
        assert!(
            sol.iter().all(|&val| val.0.is_nan() && val.1.is_nan()),
            "sol {sol:?}"
        );
    }
}
