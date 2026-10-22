//! `Fft` — iterative radix-2 complex FFT over planar re/im slices
//! with prepare-time bit-reversal and twiddle tables.

use ruviform_media_core::{Error, Result};

/// Radix-2 FFT: `new` allocates the bit-reversal table, twiddles and
/// scratch once; `process` runs in-place-permuted butterflies with
/// no allocation.
///
/// Layout is planar: `re[k]`/`im[k]` form the complex sample at bin
/// `k`. `inverse = true` conjugates the twiddles and scales by `1/n`.
///
/// **RT contract:** `process` is RT-suitable — pre-allocated tables
/// and scratch only. Precision is scalar `f32` per butterfly (twiddle
/// factors are computed in `f64` at prepare time and stored as `f32`).
pub struct Fft {
    n: usize,
    /// Bit-reversed source index for each output position.
    rev: Vec<u32>,
    /// `tw[m] = exp(−2πi·m/n)` for `m < n/2`.
    tw: Vec<(f32, f32)>,
    re_scratch: Vec<f32>,
    im_scratch: Vec<f32>,
}

impl Fft {
    /// Prepares a transform of `n` complex samples; `n` must be a
    /// power of two ≥ 2.
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `n` is not a power of two ≥ 2.
    pub fn new(n: usize) -> Result<Self> {
        if n < 2 || !n.is_power_of_two() {
            return Err(Error::InvalidValue("n"));
        }
        let bits = n.trailing_zeros();
        let rev = (0..n)
            .map(|i| u32::try_from(i).unwrap_or(u32::MAX).reverse_bits() >> (32 - bits))
            .collect();
        #[expect(clippy::cast_precision_loss)]
        let tw = (0..n / 2)
            .map(|m| {
                let angle = -2.0 * core::f64::consts::PI * m as f64 / n as f64;
                #[expect(clippy::cast_possible_truncation)]
                let v = (angle.cos() as f32, angle.sin() as f32);
                v
            })
            .collect();
        Ok(Self {
            n,
            rev,
            tw,
            re_scratch: vec![0.0; n],
            im_scratch: vec![0.0; n],
        })
    }

    /// The transform length.
    ///
    /// **RT contract:** RT-suitable — `const`-eligible read.
    #[must_use]
    pub fn len(&self) -> usize {
        self.n
    }

    /// Always `false` — the constructor rejects `n < 2`.
    ///
    /// **RT contract:** RT-suitable — `const`-eligible read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Runs the transform on planar `re`/`im` (both `n` long).
    ///
    /// **RT contract:** RT-suitable — bounded permute plus `n log n`
    /// butterflies; nothing allocates or blocks.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `re`/`im` lengths differ from `n`.
    pub fn process(&mut self, re: &mut [f32], im: &mut [f32], inverse: bool) -> Result<()> {
        if re.len() != self.n || im.len() != self.n {
            return Err(Error::InvalidValue("length"));
        }
        // Bit-reversal permutation into scratch.
        for (i, &r) in self.rev.iter().enumerate() {
            self.re_scratch[i] = re[r as usize];
            self.im_scratch[i] = im[r as usize];
        }
        let mut size = 2;
        while size <= self.n {
            let half = size / 2;
            let step = self.n / size;
            for block in (0..self.n).step_by(size) {
                for j in 0..half {
                    let (wr, mut wi) = self.tw[j * step];
                    if inverse {
                        wi = -wi;
                    }
                    let a = block + j;
                    let b = a + half;
                    let tr = wr * self.re_scratch[b] - wi * self.im_scratch[b];
                    let ti = wr * self.im_scratch[b] + wi * self.re_scratch[b];
                    self.re_scratch[b] = self.re_scratch[a] - tr;
                    self.im_scratch[b] = self.im_scratch[a] - ti;
                    self.re_scratch[a] += tr;
                    self.im_scratch[a] += ti;
                }
            }
            size *= 2;
        }
        if inverse {
            #[expect(clippy::cast_precision_loss)]
            let scale = 1.0 / self.n as f32;
            for i in 0..self.n {
                re[i] = self.re_scratch[i] * scale;
                im[i] = self.im_scratch[i] * scale;
            }
        } else {
            re.copy_from_slice(&self.re_scratch);
            im.copy_from_slice(&self.im_scratch);
        }
        Ok(())
    }
}
