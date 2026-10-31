//! 単信号スペクトル形状統計プリミティブ（Issue #169）。
//!
//! `AudioLens` `al-analysis::cheap` の `measure_spectral` /
//! `measure_transient` に inline で書かれていたスペクトル形状統計
//! （重心・帯域幅・ロールオフ・平坦度・エントロピー・フラックス）と、
//! 汎用統計（OLS 傾き・中央値・argmax 選択）を、用途非依存の `f64`
//! カーネルとして提供する。
//!
//! # 規約
//!
//! - 入力 `mag` は非負周波数 bin の線形振幅（パワー・マグニチュード等の
//!   単位は問わない）。周波数軸は一様線形 `f(k) = k·bin_hz`（Hz）とし、
//!   `k` は bin index。
//! - 縮退規則（AL verbatim）: `total = Σ mag[k]` を基準とし、
//!   [`centroid`] / [`flatness`] / [`entropy`] は `total > 0.0` のときのみ
//!   本計算、それ以外（空・全零・NaN total を含む）は `0.0`。
//!   [`bandwidth`] のみ `total <= 0.0 → 0.0` の否定形で判定するため、
//!   NaN total は `0.0` に写像せず IEEE 伝播する（AL の非対称を維持）。
//! - [`rolloff`] は累積和が `fraction · total` に初めて達する bin を
//!   走査する。`total` が NaN、または累積和が target に達しない場合は
//!   bin 0（`0.0` Hz）となる。
//! - [`entropy`] は `ln(n)`（`n = mag.len()`）で正規化する。`n = 1` で
//!   `total > 0.0` なら `0/0 = NaN` を返す（AL verbatim）。
//! - [`argmax`] は `Iterator::max_by` + `f64::total_cmp` 規則: 同率最大が
//!   複数ある場合は最後（最大 index）を返し、NaN は total order で
//!   最大として扱われる。
//! - 全関数は逐次評価順を固定した非 FMA scalar 実装で、全プラットフォーム
//!   bit-exact。[`median`] 以外は **RT-suitable**（alloc なし・有界）。

use crate::{Error, Result};

/// 振幅重み付きスペクトル重心 `Σ f(k)·mag[k] / Σ mag[k]`（Hz）。
/// `f(k) = k·bin_hz`。
///
/// `total = Σ mag[k] > 0.0` でなければ `0.0`（空・全零・NaN total を含む、
/// AL verbatim）。各項は `(k·bin_hz)·mag[k]` の順で評価する。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
#[must_use]
#[expect(clippy::cast_precision_loss)]
pub fn centroid(mag: &[f64], bin_hz: f64) -> f64 {
    let total: f64 = mag.iter().sum();
    if total > 0.0 {
        mag.iter()
            .enumerate()
            .map(|(b, &v)| b as f64 * bin_hz * v)
            .sum::<f64>()
            / total
    } else {
        0.0
    }
}

/// 重心まわりの振幅重み付き標準偏差
/// `sqrt(Σ mag[k]·(f(k) − c)² / Σ mag[k])`（Hz）。`c` は [`centroid`]。
///
/// `total = Σ mag[k] <= 0.0` なら `0.0`。`total` が NaN の場合はこの
/// 判定を通過して IEEE 伝播し、`NaN` を返す（AL の `total <= 0.0` 否定形
/// 規則 verbatim — [`centroid`] 等の `total > 0.0` 肯定形とは NaN 応答が
/// 異なることに注意）。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
#[must_use]
#[expect(clippy::cast_precision_loss)]
pub fn bandwidth(mag: &[f64], bin_hz: f64) -> f64 {
    let total: f64 = mag.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let c = mag
        .iter()
        .enumerate()
        .map(|(b, &v)| b as f64 * bin_hz * v)
        .sum::<f64>()
        / total;
    (mag.iter()
        .enumerate()
        .map(|(b, &v)| {
            let d = b as f64 * bin_hz - c;
            d * d * v
        })
        .sum::<f64>()
        / total)
        .sqrt()
}

/// スペクトル平坦度 `exp(Σ ln(mag[k] + 1e-12)/n) / (Σ mag[k]/n)`。
/// `n = mag.len()`。幾何平均は `1e-12` の下限床付き。
///
/// `total = Σ mag[k] > 0.0` でなければ `0.0`（空・全零・NaN total を含む、
/// AL verbatim）。`mag[k] <= -1e-12` を含むと `ln` が NaN を伝播する。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
#[must_use]
#[expect(clippy::cast_precision_loss)]
pub fn flatness(mag: &[f64]) -> f64 {
    let total: f64 = mag.iter().sum();
    let n = mag.len() as f64;
    let geo = (mag.iter().map(|v| (v + 1e-12).ln()).sum::<f64>() / n).exp();
    if total > 0.0 { geo / (total / n) } else { 0.0 }
}

/// 累積振幅が `fraction · total` に初めて達する bin の周波数（Hz）。
/// `total = Σ mag[k]`。
///
/// `acc += mag[k]` が `target = fraction · total` 以上となる最初の bin
/// index を `ro_bin` とし `ro_bin·bin_hz` を返す。全零入力では
/// `acc(0) >= target(0)` が bin 0 で成立するため `0.0`。`total` が NaN
/// の場合は `acc >= target` が一度も成立せず bin 0（`0.0`）となる
/// （AL verbatim。AL 側コメントは "energy rolloff" だが累積するのは
/// 振幅値であり、その契約を維持する）。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
#[must_use]
#[expect(clippy::cast_precision_loss)]
pub fn rolloff(mag: &[f64], bin_hz: f64, fraction: f64) -> f64 {
    let total: f64 = mag.iter().sum();
    let mut acc = 0.0;
    let target = total * fraction;
    let mut ro_bin = 0;
    for (b, &v) in mag.iter().enumerate() {
        acc += v;
        if acc >= target {
            ro_bin = b;
            break;
        }
    }
    ro_bin as f64 * bin_hz
}

/// `ln(n)` 正規化スペクトルエントロピー
/// `−Σ_{mag[k] > 0} p[k]·ln(p[k]) / ln(n)`。`p[k] = mag[k]/total`、
/// `n = mag.len()`。
///
/// `total = Σ mag[k] > 0.0` でなければ `0.0`（空・全零・NaN total を含む）。
/// `mag[k] <= 0.0` の bin は和から除外される（NaN 要素も除外されるが、
/// その場合 `total` は NaN となるため先に `0.0` に写像される）。
/// `n = 1` かつ `total > 0.0` では `ln(1) = 0` 除算により `NaN` を返す
/// （AL verbatim）。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
#[must_use]
#[expect(clippy::cast_precision_loss)]
pub fn entropy(mag: &[f64]) -> f64 {
    let total: f64 = mag.iter().sum();
    if total > 0.0 {
        let h: f64 = mag
            .iter()
            .filter(|&&v| v > 0.0)
            .map(|&v| (v / total) * (v / total).ln())
            .sum::<f64>();
        -h / (mag.len() as f64).ln()
    } else {
        0.0
    }
}

/// 正方向スペクトルフラックス `Σ max(0, curr[k] − prev[k])`。
/// 2 スライスは等長であること。
///
/// `(a − b)` が NaN の bin は `f64::max(0.0)` により `0.0` に写像される
/// （IEEE-754 maxNum 規則、AL verbatim）。「先頭フレームは `0.0`」の
/// 軌跡ポリシーは呼び出し側の責務。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// 長さが異なれば `Error::InvalidValue("length")`。
pub fn flux(curr: &[f64], prev: &[f64]) -> Result<f64> {
    if curr.len() != prev.len() {
        return Err(Error::InvalidValue("length"));
    }
    Ok(curr.iter().zip(prev).map(|(&a, &b)| (a - b).max(0.0)).sum())
}

/// 最小二乗（OLS）傾き `(n·Σxy − Σx·Σy) / (n·Σx² − (Σx)²)`。
/// 2 スライスは等長であること。
///
/// 和は `Σx`・`Σy`・`Σxy`・`Σx²` をこの順に同一走査で逐次累積する
/// （AL verbatim）。分母 `n·Σx² − (Σx)² == 0.0`（零分散・空入力を含む）
/// なら `0.0`。NaN 入力は分母判定を NaN として通過し IEEE 伝播する。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
///
/// # Errors
///
/// 長さが異なれば `Error::InvalidValue("length")`。
#[expect(clippy::cast_precision_loss)]
pub fn ols_slope(xs: &[f64], ys: &[f64]) -> Result<f64> {
    if xs.len() != ys.len() {
        return Err(Error::InvalidValue("length"));
    }
    let n = xs.len() as f64;
    let (sx, sy, sxy, sxx) = xs
        .iter()
        .zip(ys)
        .fold((0.0, 0.0, 0.0, 0.0), |(ax, ay, axy, axx), (&x, &y)| {
            (ax + x, ay + y, axy + x * y, axx + x * x)
        });
    Ok(if sxx * n - sx * sx == 0.0 {
        0.0
    } else {
        (sxy * n - sx * sy) / (sxx * n - sx * sx)
    })
}

/// 上位中央値: `x` を `f64::total_cmp` で整列した `sorted[n/2]`
/// （`n = x.len()`。偶数長では中央 2 要素の大きい方）。空入力は `None`。
///
/// AL の `nb.sort_by(total_cmp); nb[nb.len()/2]` パターン
/// （近傍中央値・onset 判定）と同じ要素を返す。入力は変更しない。
///
/// **RT 契約:** non-RT only — 整列用に `n` 要素を確保する。
#[must_use]
pub fn median(x: &[f64]) -> Option<f64> {
    if x.is_empty() {
        return None;
    }
    let mut sorted = x.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(sorted[sorted.len() / 2])
}

/// 最大値の `(index, value)`。`Iterator::max_by` + `f64::total_cmp`
/// 規則: 同率最大が複数ある場合は最後（最大 index）を返し、NaN は
/// total order で最大として扱われる。空入力は `None`。
///
/// AL の `.enumerate().max_by(|a, b| a.1.total_cmp(b.1))` パターン
/// （tonal peak・AM bin 選択）と同一。選択対象の範囲（DC 除外・上限
/// 帯域）は呼び出し側がスライスで絞る。
///
/// **RT 契約:** RT-suitable — alloc なし・有界。
#[must_use]
pub fn argmax(x: &[f64]) -> Option<(usize, f64)> {
    x.iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(b, &v)| (b, v))
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!((a - b).abs() <= tolerance, "{a} != {b}");
    }

    /// AL `measure_spectral` の逐次式をそのまま再現した独立 oracle。
    /// bit-exact 置換を保証するため演算順・縮退規則を verbatim で写す。
    mod oracle {
        pub(super) fn centroid(row: &[f64], bin_hz: f64) -> f64 {
            let total: f64 = row.iter().sum();
            if total > 0.0 {
                row.iter()
                    .enumerate()
                    .map(|(b, &v)| b as f64 * bin_hz * v)
                    .sum::<f64>()
                    / total
            } else {
                0.0
            }
        }

        pub(super) fn bandwidth(row: &[f64], bin_hz: f64) -> f64 {
            let total: f64 = row.iter().sum();
            if total <= 0.0 {
                return 0.0;
            }
            let c = row
                .iter()
                .enumerate()
                .map(|(b, &v)| b as f64 * bin_hz * v)
                .sum::<f64>()
                / total;
            (row.iter()
                .enumerate()
                .map(|(b, &v)| {
                    let d = b as f64 * bin_hz - c;
                    d * d * v
                })
                .sum::<f64>()
                / total)
                .sqrt()
        }

        pub(super) fn flatness(row: &[f64]) -> f64 {
            let total: f64 = row.iter().sum();
            let half = row.len();
            let geo = (row.iter().map(|v| (v + 1e-12).ln()).sum::<f64>() / half as f64).exp();
            if total > 0.0 {
                geo / (total / half as f64)
            } else {
                0.0
            }
        }

        pub(super) fn rolloff(row: &[f64], bin_hz: f64, fraction: f64) -> f64 {
            let total: f64 = row.iter().sum();
            let mut acc = 0.0;
            let target = total * fraction;
            let mut ro_bin = 0;
            for (b, &v) in row.iter().enumerate() {
                acc += v;
                if acc >= target {
                    ro_bin = b;
                    break;
                }
            }
            ro_bin as f64 * bin_hz
        }

        pub(super) fn entropy(row: &[f64]) -> f64 {
            let total: f64 = row.iter().sum();
            let half = row.len();
            if total > 0.0 {
                let h: f64 = row
                    .iter()
                    .filter(|&&v| v > 0.0)
                    .map(|&v| (v / total) * (v / total).ln())
                    .sum::<f64>();
                -h / (half as f64).ln()
            } else {
                0.0
            }
        }

        pub(super) fn flux(row: &[f64], prev: &[f64]) -> f64 {
            row.iter()
                .zip(prev.iter())
                .map(|(&a, &b)| (a - b).max(0.0))
                .sum()
        }

        pub(super) fn slope(xs: &[f64], ys: &[f64]) -> f64 {
            let n = xs.len() as f64;
            let (sx, sy, sxy, sxx) = xs
                .iter()
                .zip(ys)
                .fold((0.0, 0.0, 0.0, 0.0), |(ax, ay, axy, axx), (&x, &y)| {
                    (ax + x, ay + y, axy + x * y, axx + x * x)
                });
            if sxx * n - sx * sx == 0.0 {
                0.0
            } else {
                (sxy * n - sx * sy) / (sxx * n - sx * sx)
            }
        }
    }

    /// 決定的な擬似スペクトル（零・微小・通常値を混ぜる）。
    fn spectra() -> Vec<Vec<f64>> {
        vec![
            (0..32)
                .map(|k: usize| (k as f64 * 0.61).cos() + 0.5)
                .collect(),
            (0..32)
                .map(|k: usize| ((k as f64 * 1.3).sin() * 4.0).max(0.0))
                .collect(),
            (0..32)
                .map(|k: usize| if k.is_multiple_of(5) { 3.0 } else { 0.0 })
                .collect(),
            vec![1.0],
            vec![0.0; 7],
            Vec::new(),
        ]
    }

    #[test]
    fn spectral_stats_match_al_oracle_bit_exact() {
        let bin_hz = 3.7;
        for row in spectra() {
            assert_eq!(
                centroid(&row, bin_hz).to_bits(),
                oracle::centroid(&row, bin_hz).to_bits(),
                "centroid {row:?}"
            );
            assert_eq!(
                bandwidth(&row, bin_hz).to_bits(),
                oracle::bandwidth(&row, bin_hz).to_bits(),
                "bandwidth {row:?}"
            );
            assert_eq!(
                flatness(&row).to_bits(),
                oracle::flatness(&row).to_bits(),
                "flatness {row:?}"
            );
            assert_eq!(
                rolloff(&row, bin_hz, 0.85).to_bits(),
                oracle::rolloff(&row, bin_hz, 0.85).to_bits(),
                "rolloff {row:?}"
            );
            assert_eq!(
                entropy(&row).to_bits(),
                oracle::entropy(&row).to_bits(),
                "entropy {row:?}"
            );
            let prev: Vec<f64> = row.iter().map(|v| v * 0.5).collect();
            assert_eq!(
                flux(&row, &prev).unwrap().to_bits(),
                oracle::flux(&row, &prev).to_bits(),
                "flux {row:?}"
            );
            if row.len() >= 2 {
                // AL の `1..(half*9/10).max(2)` 域選択に合わせる。
                let xs: Vec<f64> = (1..(row.len() * 9 / 10).max(2))
                    .map(|b| (b as f64 * bin_hz).ln())
                    .collect();
                let ys: Vec<f64> = (1..(row.len() * 9 / 10).max(2))
                    .map(|b| (row[b] + 1e-12).ln())
                    .collect();
                assert_eq!(
                    ols_slope(&xs, &ys).unwrap().to_bits(),
                    oracle::slope(&xs, &ys).to_bits(),
                    "ols_slope {row:?}"
                );
            }
        }
    }

    #[test]
    fn centroid_is_amplitude_weighted_mean_of_bin_frequencies() {
        // bin 3 に集中 → 3·bin_hz。
        let mut row = vec![0.0; 8];
        row[3] = 2.5;
        assert_eq!(centroid(&row, 10.0), 30.0);
        // bin 2/4 の等振幅 2 点 → 中点 3·bin_hz。
        let mut row = vec![0.0; 8];
        row[2] = 1.0;
        row[4] = 1.0;
        assert_eq!(centroid(&row, 10.0), 30.0);
        // 全零・空・NaN total は 0.0（肯定形ゲート）。
        assert_eq!(centroid(&[0.0; 4], 10.0), 0.0);
        assert_eq!(centroid(&[], 10.0), 0.0);
        assert_eq!(centroid(&[1.0, f64::NAN], 10.0), 0.0);
    }

    #[test]
    fn bandwidth_is_rms_spread_around_centroid() {
        // 単一 bin 集中 → 分散 0。
        let mut row = vec![0.0; 8];
        row[5] = 4.0;
        assert_eq!(bandwidth(&row, 10.0), 0.0);
        // bin 1/3 の等振幅 → |f3−f1|/2 = 10 Hz。
        let mut row = vec![0.0; 8];
        row[1] = 2.0;
        row[3] = 2.0;
        close(bandwidth(&row, 10.0), 10.0, 1e-12);
        // 全零・空 → 0.0。
        assert_eq!(bandwidth(&[0.0; 4], 10.0), 0.0);
        assert_eq!(bandwidth(&[], 10.0), 0.0);
        // NaN total は `total <= 0.0` 否定形を通過して IEEE 伝播
        // （centroid の `total > 0.0` 肯定形とは非対称、AL verbatim）。
        assert!(bandwidth(&[1.0, f64::NAN], 10.0).is_nan());
    }

    #[test]
    fn flatness_is_geometric_over_arithmetic_mean() {
        // 一様スペクトル → geo = v+eps ≈ amean → ≈1。
        close(flatness(&[1.0; 16]), 1.0, 1e-9);
        // 集中スペクトルは平坦度が落ちる。
        let mut peak = vec![0.0; 16];
        peak[0] = 1.0;
        assert!(flatness(&peak) < 0.1);
        // 全零・空 → 0.0（肯定形ゲート。NaN total も 0.0）。
        assert_eq!(flatness(&[0.0; 4]), 0.0);
        assert_eq!(flatness(&[]), 0.0);
        assert_eq!(flatness(&[1.0, f64::NAN]), 0.0);
    }

    #[test]
    fn rolloff_finds_first_cumulative_crossing_bin() {
        // 一様 10 bin、bin_hz=1 → target=8.5、acc が先に越えるのは bin 8。
        let row = vec![1.0; 10];
        assert_eq!(rolloff(&row, 1.0, 0.85), 8.0);
        // bin 3 集中 → 最初の非零到達で bin 3。
        let mut row = vec![0.0; 10];
        row[3] = 9.0;
        assert_eq!(rolloff(&row, 100.0, 0.85), 300.0);
        // fraction=1.0 → 最後の非零 bin。
        assert_eq!(rolloff(&row, 100.0, 1.0), 300.0);
        // 全零・空・NaN total → bin 0 → 0.0。
        assert_eq!(rolloff(&[0.0; 4], 10.0, 0.85), 0.0);
        assert_eq!(rolloff(&[], 10.0, 0.85), 0.0);
        assert_eq!(rolloff(&[f64::NAN; 4], 10.0, 0.85), 0.0);
    }

    #[test]
    fn entropy_is_ln_n_normalized() {
        // 一様 n bin → p=1/n、h=ln(1/n) → entropy = 1。
        close(entropy(&[1.0; 8]), 1.0, 1e-15);
        // 単一 bin 集中（n>1）→ h=0 → -0.0（0.0 と等値）。
        let mut row = vec![0.0; 8];
        row[2] = 3.0;
        assert_eq!(entropy(&row), 0.0);
        // 全零・空 → 0.0。NaN total → 0.0（肯定形ゲート）。
        assert_eq!(entropy(&[0.0; 4]), 0.0);
        assert_eq!(entropy(&[]), 0.0);
        assert_eq!(entropy(&[1.0, f64::NAN]), 0.0);
        // n=1 かつ total>0 → ln(1)=0 除算で NaN（AL verbatim）。
        assert!(entropy(&[5.0]).is_nan());
    }

    #[test]
    fn flux_sums_half_wave_rectified_differences() {
        // 同一 → 0。単調増分のみ加算（減分は 0）。
        assert_eq!(flux(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).unwrap(), 0.0);
        assert_eq!(
            flux(&[3.0, 1.0, 4.0], &[1.0, 2.0, 1.0]).unwrap(),
            2.0 + 0.0 + 3.0
        );
        // 空同士 → 0.0。長さ不一致 → Err（zip 黙認しない）。
        assert_eq!(flux(&[], &[]).unwrap(), 0.0);
        assert_eq!(
            flux(&[1.0; 3], &[1.0; 4]),
            Err(Error::InvalidValue("length"))
        );
        // NaN 差分は max(0.0) で 0.0 に写像（maxNum 規則）。
        assert_eq!(flux(&[f64::NAN, 1.0], &[0.0, 0.0]).unwrap(), 1.0);
    }

    #[test]
    fn ols_slope_recovers_exact_linear_relation() {
        // y = 2x + 1 → slope = 2。
        let xs: Vec<f64> = (0..10).map(|k: usize| k as f64 * 0.5).collect();
        let ys: Vec<f64> = xs.iter().map(|&x| 2.0 * x + 1.0).collect();
        close(ols_slope(&xs, &ys).unwrap(), 2.0, 1e-12);
        // 零分散（x 一定）・空 → 0.0。
        assert_eq!(ols_slope(&[1.0; 4], &[0.0, 1.0, 2.0, 3.0]).unwrap(), 0.0);
        assert_eq!(ols_slope(&[], &[]).unwrap(), 0.0);
        // 長さ不一致 → Err。
        assert_eq!(
            ols_slope(&[1.0; 3], &[1.0; 4]),
            Err(Error::InvalidValue("length"))
        );
    }

    #[test]
    fn median_is_upper_middle_of_total_cmp_sorted() {
        // 奇数長 → 中央。偶数長 → 中央 2 要素の大きい方（index n/2）。
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), Some(3.0));
        assert_eq!(median(&[7.0]), Some(7.0));
        // 空 → None。NaN は total order で末尾へ → 中央値は有限側。
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[1.0, f64::NAN, 2.0]), Some(2.0));
        // 入力を変更しない。
        let x = [2.0, 1.0];
        assert_eq!(median(&x), Some(2.0));
        assert_eq!(x, [2.0, 1.0]);
    }

    #[test]
    fn argmax_returns_last_max_on_ties() {
        // 一意最大 → (index, value)。
        assert_eq!(argmax(&[1.0, 5.0, 3.0]), Some((1, 5.0)));
        // 同率最大 → 最後の index（Iterator::max_by 規則）。
        assert_eq!(argmax(&[2.0, 5.0, 5.0, 1.0]), Some((2, 5.0)));
        assert_eq!(argmax(&[3.0, 3.0, 3.0]), Some((2, 3.0)));
        // 空 → None。NaN は total order 最大として選ばれる。
        assert_eq!(argmax(&[]), None);
        assert_eq!(argmax(&[1.0, f64::NAN, 3.0]).unwrap().0, 1);
    }
}
