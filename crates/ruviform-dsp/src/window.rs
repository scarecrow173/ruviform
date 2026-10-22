//! Window functions: coefficient generation for analysis/processing
//! use (`Window`).

/// A window function shape.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new shapes may be added without breaking downstream
/// `match` arms.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Window {
    /// `w[n] = 1`.
    Rectangular,
    /// Hann: `w[n] = 0.5 · (1 − cos(2πn/(N−1)))`.
    Hann,
    /// Hamming: `w[n] = 0.54 − 0.46 · cos(2πn/(N−1))`.
    Hamming,
    /// Blackman: `w[n] = 0.42 − 0.5·cos(2πn/(N−1)) + 0.08·cos(4πn/(N−1))`.
    Blackman,
}

impl Window {
    /// Writes `dst.len()` window coefficients into `dst`.
    ///
    /// **RT contract:** RT-suitable — pure arithmetic, allocation-free
    /// (generation is normally done once at setup time).
    #[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn fill(&self, dst: &mut [f32]) {
        let n = dst.len();
        if matches!(self, Self::Rectangular) {
            dst.fill(1.0);
            return;
        }
        let denom = (n.max(2) - 1) as f64;
        for (i, w) in dst.iter_mut().enumerate() {
            let x = i as f64 / denom;
            let v = match self {
                Self::Hann => 0.5 * (1.0 - (2.0 * core::f64::consts::PI * x).cos()),
                Self::Hamming => 0.54 - 0.46 * (2.0 * core::f64::consts::PI * x).cos(),
                Self::Blackman => {
                    0.42 - 0.5 * (2.0 * core::f64::consts::PI * x).cos()
                        + 0.08 * (4.0 * core::f64::consts::PI * x).cos()
                }
                Self::Rectangular => unreachable!("handled above"),
            };
            *w = v as f32;
        }
    }

    /// Allocates a `len`-coefficient window vector.
    ///
    /// **RT contract:** non-RT only — allocates.
    #[must_use]
    pub fn coeffs(&self, len: usize) -> Vec<f32> {
        let mut v = vec![0.0; len];
        self.fill(&mut v);
        v
    }
}
