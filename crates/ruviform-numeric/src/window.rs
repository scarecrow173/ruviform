//! `f64` 窓関数の係数生成。
//!
//! **規約: periodic 窓（分母 `N`）**。STFT / スペクトル解析用途の規約であり、
//! `AudioLens` `hann(n)` と同じ定義（`w[n] = 0.5 − 0.5·cos(2πn/N)`）である。
//! `ruviform-dsp::Window`（f32・RT オーディオ向け）の symmetric 窓
//! （分母 `N−1`）とは規約が異なる点に注意。両者は置き換え関係にない。
//!
//! 係数は有限値のみを生成する（`N=0` は空、`N=1` は Hann/Hamming/Blackman で
//! `w[0] = 0`・Rectangular で `1` — 周期公式の素直な評価。NumPy の
//! `M==1 → ones` 特例は設けない）。

/// 窓関数の形状。
///
/// `#[non_exhaustive]` — 新しい形状を non-breaking に追加できる。
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Window {
    /// `w[n] = 1`。
    Rectangular,
    /// Hann（periodic）: `w[n] = 0.5 − 0.5·cos(2πn/N)`。
    Hann,
    /// Hamming（periodic）: `w[n] = 0.54 − 0.46·cos(2πn/N)`。
    Hamming,
    /// Blackman（periodic）: `w[n] = 0.42 − 0.5·cos(2πn/N) + 0.08·cos(4πn/N)`。
    Blackman,
}

impl Window {
    /// `dst.len()` 個の窓係数を `dst` に書き込む。
    ///
    /// **RT 契約:** RT-suitable — 純粋算術・alloc なし（生成は通常準備時に
    /// 1 回行う）。
    #[expect(clippy::cast_precision_loss)]
    pub fn fill(&self, dst: &mut [f64]) {
        if matches!(self, Self::Rectangular) {
            dst.fill(1.0);
            return;
        }
        let denom = dst.len().max(1) as f64;
        for (i, w) in dst.iter_mut().enumerate() {
            let x = i as f64 / denom;
            *w = match self {
                Self::Hann => 0.5 - 0.5 * (2.0 * core::f64::consts::PI * x).cos(),
                Self::Hamming => 0.54 - 0.46 * (2.0 * core::f64::consts::PI * x).cos(),
                Self::Blackman => {
                    0.42 - 0.5 * (2.0 * core::f64::consts::PI * x).cos()
                        + 0.08 * (4.0 * core::f64::consts::PI * x).cos()
                }
                Self::Rectangular => unreachable!("handled above"),
            };
        }
    }

    /// `len` 個の窓係数ベクトルを確保して返す。
    ///
    /// **RT 契約:** non-RT only — 確保する。
    #[must_use]
    pub fn coeffs(&self, len: usize) -> Vec<f64> {
        let mut v = vec![0.0; len];
        self.fill(&mut v);
        v
    }
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    #[test]
    fn coefficients_match_periodic_definitions() {
        let n = 8;
        let hann = Window::Hann.coeffs(n);
        let hamming = Window::Hamming.coeffs(n);
        let blackman = Window::Blackman.coeffs(n);
        let rect = Window::Rectangular.coeffs(n);
        for i in 0..n {
            let x = i as f64 / n as f64;
            assert_eq!(rect[i], 1.0);
            assert!(
                (hann[i] - (0.5 - 0.5 * (2.0 * core::f64::consts::PI * x).cos())).abs() < 1e-16,
                "hann[{i}]"
            );
            assert!(
                (hamming[i] - (0.54 - 0.46 * (2.0 * core::f64::consts::PI * x).cos())).abs()
                    < 1e-16,
                "hamming[{i}]"
            );
            let b = 0.42 - 0.5 * (2.0 * core::f64::consts::PI * x).cos()
                + 0.08 * (4.0 * core::f64::consts::PI * x).cos();
            assert!((blackman[i] - b).abs() < 1e-16, "blackman[{i}]");
        }
        // periodic 窓の性質: Hann 係数和は N/2、両端は 0 でない対称性
        // （w[N] = w[0] となる周期延長）。
        let sum: f64 = hann.iter().sum();
        assert!((sum - n as f64 / 2.0).abs() < 1e-14);
        // Hann は w[0] = 0 かつ w[N/2] = 1。
        assert_eq!(hann[0], 0.0);
        assert!((hann[n / 2] - 1.0).abs() < 1e-16);
    }

    #[test]
    fn edge_lengths() {
        // N=0 は空。
        assert_eq!(Window::Hann.coeffs(0), Vec::<f64>::new());
        // N=1 は周期公式の素直な評価（NumPy の ones 特例は設けない）。
        // periodic: Hann[0] = 0、Hamming[0] = 0.54−0.46 = 0.08、
        // Blackman[0] = 0.42−0.5+0.08 ≈ 0（丸め誤差のみ）。
        assert_eq!(Window::Hann.coeffs(1), vec![0.0]);
        assert_eq!(Window::Hamming.coeffs(1), vec![0.54 - 0.46]);
        assert!(Window::Blackman.coeffs(1)[0].abs() < 1e-16);
        assert_eq!(Window::Rectangular.coeffs(1), vec![1.0]);
        // fill と coeffs は一致。
        let mut dst = [0.0; 33];
        Window::Hann.fill(&mut dst);
        assert_eq!(dst.to_vec(), Window::Hann.coeffs(33));
    }
}
