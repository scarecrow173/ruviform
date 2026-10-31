//! `f64` スライスのベクトル演算プリミティブ。
//!
//! すべての関数は **RT-suitable**（alloc なし・有界）。積和は逐次評価順を
//! 固定した非融合 `a*b+c` 形で、全プラットフォームで bit-exact。複数
//! スライスの等長契約違反は `Err(Error::InvalidValue("length"))` を返す
//! （zip 黙認しない）。`&mut` 出力は借用規則により入力とエイリアスしない。
//!
//! `*_seq` サフィックスの `pub(crate)` カーネルは検証済み呼び出し専用で、
//! 公開 API の検証と linalg 等の内部利用で演算順を共有する。

use crate::{Error, Result};

// 以下の `pub(crate)` カーネルは等長契約を呼び出し側が保証する前提の
// 無検証版。検証は公開 API の境界で行い、二重検証しない。

/// `a·b` の逐次内積（無検証）。
#[inline]
pub(crate) fn dot_seq(a: &[f64], b: &[f64]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(&x, &y)| x * y).sum()
}

/// `Σx²` の逐次和（無検証）。
#[inline]
pub(crate) fn sum_sq_seq(x: &[f64]) -> f64 {
    x.iter().map(|&v| v * v).sum()
}

/// `y[i] += a·x[i]`（無検証）。
#[inline]
pub(crate) fn axpy_seq(y: &mut [f64], a: f64, x: &[f64]) {
    debug_assert_eq!(y.len(), x.len());
    for (y, &x) in y.iter_mut().zip(x) {
        *y += a * x;
    }
}

/// `y[i] -= a·x[i]`（無検証）。
#[inline]
pub(crate) fn sub_scaled_seq(y: &mut [f64], a: f64, x: &[f64]) {
    debug_assert_eq!(y.len(), x.len());
    for (y, &x) in y.iter_mut().zip(x) {
        *y -= a * x;
    }
}

/// `corr0` の分解形（無検証）。`(Σxy, Σx², Σy²)` を同じ一走査で返す。
/// correlation カーネルが分子・分母を別々に使う際に演算順を共有する。
#[inline]
pub(crate) fn corr0_parts_seq(x: &[f64], y: &[f64]) -> (f64, f64, f64) {
    debug_assert_eq!(x.len(), y.len());
    let mut sxy = 0.0;
    let mut ex = 0.0;
    let mut ey = 0.0;
    for (&x, &y) in x.iter().zip(y) {
        sxy += x * y;
        ex += x * x;
        ey += y * y;
    }
    (sxy, ex, ey)
}

/// `a·b` の内積。逐次和で `Σ a[i]·b[i]`。2 スライスは等長であること。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// 長さが異なれば `Error::InvalidValue("length")`。
#[inline]
pub fn dot(a: &[f64], b: &[f64]) -> Result<f64> {
    if a.len() != b.len() {
        return Err(Error::InvalidValue("length"));
    }
    Ok(dot_seq(a, b))
}

/// `Σx²`。空入力は `0.0`。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
#[inline]
#[must_use]
pub fn sum_sq(x: &[f64]) -> f64 {
    sum_sq_seq(x)
}

/// 中心化しない `Σxy / √(Σx²·Σy²)`。
///
/// `Σx² > 0 かつ Σy² > 0` でなければ `0.0`（NaN・縮退エネルギーも `0.0`）。
/// 2 スライスは等長であること。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// 長さが異なれば `Error::InvalidValue("length")`。
#[inline]
pub fn corr0(x: &[f64], y: &[f64]) -> Result<f64> {
    if x.len() != y.len() {
        return Err(Error::InvalidValue("length"));
    }
    let (sxy, ex, ey) = corr0_parts_seq(x, y);
    Ok(if ex > 0.0 && ey > 0.0 {
        sxy / (ex * ey).sqrt()
    } else {
        0.0
    })
}

/// `sqrt(Σx² / n)`。空入力は `0.0`。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
#[inline]
#[must_use]
#[expect(clippy::cast_precision_loss)]
pub fn rms(x: &[f64]) -> f64 {
    if x.is_empty() {
        0.0
    } else {
        (sum_sq_seq(x) / x.len() as f64).sqrt()
    }
}

/// `out[i] = a·x[i]`。`x`/`out` は等長であること。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// 長さが異なれば `Error::InvalidValue("length")`。
#[inline]
pub fn scale(x: &[f64], a: f64, out: &mut [f64]) -> Result<()> {
    if x.len() != out.len() {
        return Err(Error::InvalidValue("length"));
    }
    for (&x, o) in x.iter().zip(out) {
        *o = x * a;
    }
    Ok(())
}

/// `y[i] += a·x[i]`。`y`/`x` は等長であること。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// 長さが異なれば `Error::InvalidValue("length")`。
#[inline]
pub fn axpy(y: &mut [f64], a: f64, x: &[f64]) -> Result<()> {
    if y.len() != x.len() {
        return Err(Error::InvalidValue("length"));
    }
    axpy_seq(y, a, x);
    Ok(())
}

/// `y[i] -= a·x[i]`。`y`/`x` は等長であること。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// 長さが異なれば `Error::InvalidValue("length")`。
#[inline]
pub fn sub_scaled(y: &mut [f64], a: f64, x: &[f64]) -> Result<()> {
    if y.len() != x.len() {
        return Err(Error::InvalidValue("length"));
    }
    sub_scaled_seq(y, a, x);
    Ok(())
}

/// `out[i] = re[i]² + im[i]²`。3 スライスはすべて等長であること。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// いずれかの長さが異なれば `Error::InvalidValue("length")`。
#[inline]
pub fn mag_sq(re: &[f64], im: &[f64], out: &mut [f64]) -> Result<()> {
    if re.len() != im.len() || re.len() != out.len() {
        return Err(Error::InvalidValue("length"));
    }
    for ((&r, &i), o) in re.iter().zip(im).zip(out) {
        *o = r * r + i * i;
    }
    Ok(())
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!((a - b).abs() <= tolerance, "{a} != {b}");
    }

    // 0..=33（空・短入力）・1k・64k の決定的入力（al-math vec テスト由来）。
    fn inputs(n: usize) -> (Vec<f64>, Vec<f64>) {
        let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.71).sin() - 0.25).collect();
        let y: Vec<f64> = (0..n).map(|i| (i as f64 * 1.37).cos() + 0.5).collect();
        (x, y)
    }

    // 独立 oracle: 畳み込み fold と異なる評価形で丸め経路差を許す比較。
    fn oracle_dot(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b).fold(0.0, |acc, (&x, &y)| acc + x * y)
    }
    fn oracle_sum_sq(x: &[f64]) -> f64 {
        x.iter().fold(0.0, |acc, &v| acc + v * v)
    }

    #[test]
    fn reductions_match_independent_oracle_across_sizes() {
        for n in (0..=33).chain([1_024, 65_536]) {
            let (x, y) = inputs(n);
            // fold と map+sum は同じ左から右の評価順なので bit-exact。
            assert_eq!(dot(&x, &y).unwrap(), oracle_dot(&x, &y), "dot n={n}");
            assert_eq!(sum_sq(&x), oracle_sum_sq(&x), "sum_sq n={n}");
        }
    }

    #[test]
    fn corr0_returns_ratio_only_when_both_energies_are_positive() {
        let (x, y) = inputs(33);
        let sxy = oracle_dot(&x, &y);
        let expected = sxy / (oracle_sum_sq(&x) * oracle_sum_sq(&y)).sqrt();
        close(corr0(&x, &y).unwrap(), expected, 1e-15);
        // corr0(x, x) = Σx²/|Σx²| = 1（正のエネルギーのとき）。
        assert_eq!(corr0(&x, &x).unwrap(), 1.0);
        // 直交入力は sxy = 0 → 0.0。
        assert_eq!(corr0(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]).unwrap(), 0.0);
    }

    #[test]
    fn corr0_maps_zero_or_nan_energy_to_zero() {
        let x = [1.0, -2.0, 3.0];
        let zeros = [0.0; 3];
        assert_eq!(corr0(&zeros, &x).unwrap(), 0.0);
        assert_eq!(corr0(&x, &zeros).unwrap(), 0.0);
        assert_eq!(corr0(&zeros, &zeros).unwrap(), 0.0);
        assert_eq!(corr0(&[], &[]).unwrap(), 0.0);
        assert_eq!(corr0(&[f64::NAN, 1.0], &[1.0, 1.0]).unwrap(), 0.0);
        assert_eq!(corr0(&[1.0, 1.0], &[1.0, f64::NAN]).unwrap(), 0.0);
    }

    #[test]
    fn rms_is_zero_for_empty_and_abs_for_constant_input() {
        assert_eq!(rms(&[]), 0.0);
        for n in 1..=16 {
            assert_eq!(rms(&vec![-0.5; n]), 0.5);
        }
    }

    #[test]
    fn elementwise_ops_match_formulas() {
        for n in (0..=33).chain([1_024]) {
            let (x, y) = inputs(n);
            let a = -2.5;

            let mut out = vec![0.0; n];
            scale(&x, a, &mut out).unwrap();
            let expected: Vec<f64> = x.iter().map(|&x| x * a).collect();
            assert_eq!(out, expected, "scale n={n}");

            let mut acc = y.clone();
            axpy(&mut acc, a, &x).unwrap();
            let expected: Vec<f64> = y.iter().zip(&x).map(|(&y, &x)| y + a * x).collect();
            assert_eq!(acc, expected, "axpy n={n}");

            let mut acc = y.clone();
            sub_scaled(&mut acc, a, &x).unwrap();
            let expected: Vec<f64> = y.iter().zip(&x).map(|(&y, &x)| y - a * x).collect();
            assert_eq!(acc, expected, "sub_scaled n={n}");

            let mut out = vec![0.0; n];
            mag_sq(&x, &y, &mut out).unwrap();
            let expected: Vec<f64> = x.iter().zip(&y).map(|(&r, &i)| r * r + i * i).collect();
            assert_eq!(out, expected, "mag_sq n={n}");
        }
    }

    #[test]
    fn length_mismatches_are_errors_not_panics() {
        assert_eq!(
            dot(&[1.0; 3], &[1.0; 4]),
            Err(Error::InvalidValue("length"))
        );
        assert_eq!(
            corr0(&[1.0; 3], &[1.0; 4]),
            Err(Error::InvalidValue("length"))
        );
        assert_eq!(
            scale(&[1.0; 3], 2.0, &mut [0.0; 4]),
            Err(Error::InvalidValue("length"))
        );
        assert_eq!(
            axpy(&mut [0.0; 3], 2.0, &[1.0; 4]),
            Err(Error::InvalidValue("length"))
        );
        assert_eq!(
            sub_scaled(&mut [0.0; 3], 2.0, &[1.0; 4]),
            Err(Error::InvalidValue("length"))
        );
        assert_eq!(
            mag_sq(&[1.0; 3], &[1.0; 4], &mut [0.0; 3]),
            Err(Error::InvalidValue("length"))
        );
        assert_eq!(
            mag_sq(&[1.0; 3], &[1.0; 3], &mut [0.0; 4]),
            Err(Error::InvalidValue("length"))
        );
    }
}
