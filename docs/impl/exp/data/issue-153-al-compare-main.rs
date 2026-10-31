//! Issue #153: ruviform-numeric (stft/bands/xspec) vs al-analysis::stft +
//! al-math::filter（#98 frozen baseline 相当）の精度・性能・メモリ比較
//! ハーネス。どちらのリポジトリも変更しない。
//!
//! al-analysis::stft は依存木を引かないよう verbatim 移植する
//! （`al_locate::dsp::{Fft, mag_sq, scale}` → `al_math::fft::Fft`・
//! `al_math::vec::{mag_sq, scale}` で再構成。#150 の conv 移植と同じ扱い）。
//! al-math::filter（bandpass_many/BandFilter/spectral_envelope）は
//! path 依存で直接呼ぶ。
//!
//! 実行: cargo run --release (from %TEMP%/r153-compare)

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use ruviform_numeric::{bands, stft as r_stft, xspec};
use ruviform_numeric::window::Window;

// === 割付計測 allocator ===
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn alloc_stats() -> (usize, usize) {
    (
        ALLOC_COUNT.load(Ordering::Relaxed),
        ALLOC_BYTES.load(Ordering::Relaxed),
    )
}

// === al-analysis/src/stft.rs verbatim 移植 ===
// 変更点は use 行のみ（al_locate::dsp → al_math 直参照）。
mod al_stft {
    use al_math::fft::Fft;
    use al_math::vec::{mag_sq, scale};

    /// One frame×bin complex matrix produced by [`stft`].
    #[derive(Debug, Clone)]
    pub struct StftMatrix {
        /// Real part, `frames[frame][bin]` (bins = `fft_size`).
        pub re: Vec<Vec<f64>>,
        /// Imaginary part, same layout.
        pub im: Vec<Vec<f64>>,
        /// Analysis window length (samples, before padding).
        pub window: u64,
        /// Hop between successive frames (samples).
        pub hop: u64,
        /// FFT size (power of two, `>= window`).
        pub fft_size: usize,
        /// Sample rate of the source signal.
        pub sample_rate: u32,
    }

    impl StftMatrix {
        /// Number of frames.
        pub fn frames(&self) -> usize {
            self.re.len()
        }

        /// Bin center frequency (Hz) of `bin`.
        pub fn bin_freq(&self, bin: usize) -> f64 {
            bin as f64 * self.sample_rate as f64 / self.fft_size as f64
        }

        /// Squared window energy used for PSD normalization.
        fn window_power(&self) -> f64 {
            // Hann window energy = 3/8 * window; matches `hann()` below.
            0.375 * self.window as f64
        }

        /// Power spectral density, `frames[frame][bin]` in |x|²/Hz.
        /// One-sided interpretation is left to the caller — this is the
        /// full `fft_size`-bin transform.
        pub fn psd(&self) -> Vec<Vec<f64>> {
            // Two-sided PSD normalization: |X[k]|² / (fs · Σw²). Summing
            // all bins × bin width recovers the signal power.
            let norm = self.window_power() * self.sample_rate as f64;
            self.re
                .iter()
                .zip(&self.im)
                .map(|(re, im)| {
                    // Two passes (mag_sq then scale): the primitives do
                    // not allow logical in-place aliasing.
                    let mut power = vec![0.0; re.len()];
                    mag_sq(re, im, &mut power);
                    let mut out = vec![0.0; re.len()];
                    scale(&power, 1.0 / norm, &mut out);
                    out
                })
                .collect()
        }

        /// Magnitude matrix.
        pub fn magnitude(&self) -> Vec<Vec<f64>> {
            self.re
                .iter()
                .zip(&self.im)
                .map(|(re, im)| {
                    let mut m2 = vec![0.0; re.len()];
                    mag_sq(re, im, &mut m2);
                    m2.iter().map(|&v| v.sqrt()).collect()
                })
                .collect()
        }

        /// Power (squared magnitude) matrix.
        pub fn power(&self) -> Vec<Vec<f64>> {
            self.re
                .iter()
                .zip(&self.im)
                .map(|(re, im)| {
                    let mut out = vec![0.0; re.len()];
                    mag_sq(re, im, &mut out);
                    out
                })
                .collect()
        }
    }

    /// Hann window coefficients.
    pub fn hann(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos())
            .collect()
    }

    /// Compute the complex STFT of `samples`.
    ///
    /// Frames are Hann-windowed `window`-sample segments stepped by
    /// `hop`, zero-padded to the next power of two. A signal shorter
    /// than one window yields zero frames — callers decide the validity
    /// (`InsufficientSupport`), not this function.
    pub fn stft(samples: &[f64], window: usize, hop: usize, sample_rate: u32) -> StftMatrix {
        let fft_size = window.next_power_of_two();
        let win = hann(window);
        let mut re = Vec::new();
        let mut im = Vec::new();
        if samples.len() >= window && window > 0 && hop > 0 {
            let plan = Fft::new(fft_size);
            for start in (0..=samples.len() - window).step_by(hop) {
                let mut fre = vec![0.0; fft_size];
                let mut fim = vec![0.0; fft_size];
                for (i, &v) in samples[start..start + window].iter().enumerate() {
                    fre[i] = v * win[i];
                }
                plan.transform(&mut fre, &mut fim, false);
                re.push(fre);
                im.push(fim);
            }
        }
        StftMatrix {
            re,
            im,
            window: window as u64,
            hop: hop as u64,
            fft_size,
            sample_rate,
        }
    }

    /// Integrate a power/PSD row into `[lo_hz, hi_hz)` bands. Returns one
    /// value per band; bands outside the bin range contribute zero.
    pub fn band_power(
        row: &[f64],
        sample_rate: u32,
        fft_size: usize,
        bands: &[(f64, f64)],
    ) -> Vec<f64> {
        let bin_hz = sample_rate as f64 / fft_size as f64;
        bands
            .iter()
            .map(|&(lo, hi)| {
                let b0 = (lo / bin_hz).floor() as usize;
                let b1 = (hi / bin_hz).ceil() as usize;
                (b0..b1.min(row.len())).map(|b| row[b]).sum::<f64>() * bin_hz
            })
            .collect()
    }

    /// Standard IEC 1/3-octave bands inside `[f_min, f_max]` as
    /// `(lower, upper)` edges around `f_c = 1000·2^(k/3)` centers.
    pub fn third_octave_bands(f_min: f64, f_max: f64) -> Vec<(f64, f64)> {
        let mut bands = Vec::new();
        let ratio = 2f64.powf(1.0 / 3.0);
        let mut center = 1000.0;
        // Step down below f_min, then walk upward.
        while center / ratio.sqrt() > f_min {
            center /= ratio;
        }
        while center * ratio.sqrt() <= f_max {
            bands.push((center / ratio.sqrt(), center * ratio.sqrt()));
            center *= ratio;
        }
        bands
    }

    /// Cross-spectral density of two aligned frames: `conj(x) * y`
    /// per bin (returns `(re, im)` pairs).
    pub fn csd(x_re: &[f64], x_im: &[f64], y_re: &[f64], y_im: &[f64]) -> Vec<(f64, f64)> {
        x_re.iter()
            .zip(x_im)
            .zip(y_re.iter().zip(y_im))
            .map(|((&xr, &xi), (&yr, &yi))| (xr * yr + xi * yi, xr * yi - xi * yr))
            .collect()
    }

    /// Complex coherence `Pxy / sqrt(Pxx * Pyy)` per bin. Bins with zero
    /// auto-power in either signal return `0`.
    pub fn complex_coherence(pxx: &[f64], pyy: &[f64], pxy: &[(f64, f64)]) -> Vec<(f64, f64)> {
        pxx.iter()
            .zip(pyy)
            .zip(pxy)
            .map(|((&xx, &yy), &(pr, pi))| {
                let denom = (xx * yy).sqrt();
                if denom <= 0.0 {
                    (0.0, 0.0)
                } else {
                    (pr / denom, pi / denom)
                }
            })
            .collect()
    }

    /// Spectral envelope of a magnitude row: a smoothed log-frequency
    /// curve (moving average in linear magnitude over `bins`-wide
    /// windows), used for coarse shape comparison.
    pub fn envelope(magnitude_row: &[f64], bins: usize) -> Vec<f64> {
        let bins = bins.max(1);
        (0..magnitude_row.len())
            .map(|i| {
                let lo = i.saturating_sub(bins / 2);
                let hi = (i + bins / 2 + 1).min(magnitude_row.len());
                magnitude_row[lo..hi].iter().sum::<f64>() / (hi - lo) as f64
            })
            .collect()
    }
}

// === al-analysis::reference の Welch 蓄積+H1 を verbatim 移植 ===
// （reference.rs L520-546: mean Pxx/Pyy/Pxy → complex_coherence → H1）
fn al_welch(
    xs_re: &[Vec<f64>],
    xs_im: &[Vec<f64>],
    ys_re: &[Vec<f64>],
    ys_im: &[Vec<f64>],
    frames: usize,
    bins: usize,
) -> (Vec<f64>, Vec<f64>, Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let mut pxx = vec![0.0; bins];
    let mut pyy = vec![0.0; bins];
    let mut pxy = vec![(0.0, 0.0); bins];
    for f in 0..frames {
        for k in 0..bins {
            let xr = xs_re[f][k];
            let xi = xs_im[f][k];
            let yr = ys_re[f][k];
            let yi = ys_im[f][k];
            pxx[k] += xr * xr + xi * xi;
            pyy[k] += yr * yr + yi * yi;
            pxy[k].0 += xr * yr + xi * yi;
            pxy[k].1 += xr * yi - xi * yr;
        }
    }
    for k in 0..bins {
        pxx[k] /= frames as f64;
        pyy[k] /= frames as f64;
        pxy[k].0 /= frames as f64;
        pxy[k].1 /= frames as f64;
    }
    let coherence = al_stft::complex_coherence(&pxx, &pyy, &pxy);
    let h: Vec<(f64, f64)> = pxx
        .iter()
        .zip(&pxy)
        .map(|(&xx, &(pr, pi))| {
            if xx > 0.0 {
                (pr / xx, pi / xx)
            } else {
                (0.0, 0.0)
            }
        })
        .collect();
    (pxx, pyy, pxy, coherence, h)
}

// === 決定的入力（al-analysis fixture_samples 同一式）===
fn fixture_samples(n: usize, rate: u32) -> Vec<f64> {
    let mut s = 0x9e3779b9u64;
    (0..n)
        .map(|i| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let noise = ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0;
            let t = i as f64 / rate as f64;
            (2.0 * std::f64::consts::PI * 1000.0 * t).sin()
                + 0.3 * (2.0 * std::f64::consts::PI * 3000.0 * t).sin()
                + 0.1 * noise
        })
        .collect()
}

fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(&x, &y)| (x - y).abs())
        .fold(0.0, f64::max)
}

fn max_diff_pairs(a: &[(f64, f64)], b_re: &[f64], b_im: &[f64]) -> f64 {
    assert_eq!(a.len(), b_re.len());
    a.iter()
        .zip(b_re.iter().zip(b_im))
        .map(|(&(x, y), (&r, &i))| ((x - r).abs()).max((y - i).abs()))
        .fold(0.0, f64::max)
}

fn bench_us<T, F: FnMut() -> T>(reps: usize, mut f: F) -> f64 {
    black_box(f()); // warmup
    let mut ts = Vec::with_capacity(reps);
    for _ in 0..reps {
        let t = Instant::now();
        black_box(f());
        ts.push(t.elapsed().as_secs_f64() * 1e6);
    }
    ts.sort_by(|a, b| a.total_cmp(b));
    ts[reps / 2]
}

fn main() {
    println!("=== 精度比較 (max |diff|) ===");

    // -- STFT: production shapes (reference 4096/2048, cheap 2048/1024) --
    for &(win, hop) in &[(4096usize, 2048usize), (2048, 1024), (1500, 500)] {
        let samples = fixture_samples(192_000, 48_000);
        let a = al_stft::stft(&samples, win, hop, 48_000);
        let b = r_stft::stft(&samples, win, hop, Window::Hann, 48_000).unwrap();
        assert_eq!(a.frames(), b.frames());
        assert_eq!(a.fft_size, b.fft_size());
        let mut dre: f64 = 0.0;
        let mut dim: f64 = 0.0;
        for f in 0..a.frames() {
            dre = dre.max(max_abs_diff(&a.re[f], b.frame_re(f).unwrap()));
            dim = dim.max(max_abs_diff(&a.im[f], b.frame_im(f).unwrap()));
        }
        println!("stft w={win} h={hop}: re {dre:.3e} im {dim:.3e}");

        // 派生行列（先頭 8 フレームで評価 — 全フレームは冗長）。
        let ap = a.power();
        let am = a.magnitude();
        let asd = a.psd();
        let bp = b.power();
        let bm = b.magnitude();
        let bsd = b.psd();
        let (mut dp, mut dm, mut ds): (f64, f64, f64) = (0.0, 0.0, 0.0);
        for f in 0..a.frames().min(8) {
            for k in 0..a.fft_size {
                let i = f * a.fft_size + k;
                dp = dp.max((ap[f][k] - bp[i]).abs());
                dm = dm.max((am[f][k] - bm[i]).abs());
                ds = ds.max((asd[f][k] - bsd[i]).abs());
            }
        }
        println!("stft w={win} derived: power {dp:.3e} mag {dm:.3e} psd {ds:.3e}");
    }

    // -- band 分割・積分 --
    {
        let a_bands = al_stft::third_octave_bands(40.0, 16_000.0);
        let b_bands = bands::third_octave_bands(40.0, 16_000.0).unwrap();
        assert_eq!(a_bands.len(), b_bands.len());
        let d = a_bands
            .iter()
            .zip(&b_bands)
            .map(|(&(l0, h0), &(l1, h1))| (l0 - l1).abs().max((h0 - h1).abs()))
            .fold(0.0, f64::max);
        println!("third_octave_bands: {d:.3e} ({} bands)", a_bands.len());

        // band_power: psd 行を積分。AL は (rate, fft_size)、rf は bin_hz。
        let samples = fixture_samples(96_000, 48_000);
        let a = al_stft::stft(&samples, 4096, 2048, 48_000);
        let apsd = a.psd();
        let bin_hz = 48_000.0 / 4096.0;
        let ao = al_stft::band_power(&apsd[0], 48_000, 4096, &a_bands);
        let bo = bands::band_power(&apsd[0], bin_hz, &a_bands);
        println!("band_power: {:.3e}", max_abs_diff(&ao, &bo));
    }

    // -- csd / complex_coherence / Welch / H1 --
    {
        let rate = 48_000;
        let xs = fixture_samples(96_000, rate);
        // y は x の遅延+ゲイン版（相関のある信号）。
        let ys: Vec<f64> = xs
            .iter()
            .enumerate()
            .map(|(i, &v)| 0.8 * v + if i >= 64 { 0.2 * xs[i - 64] } else { 0.0 })
            .collect();
        let a = al_stft::stft(&xs, 4096, 2048, rate);
        let c = al_stft::stft(&ys, 4096, 2048, rate);
        let frames = a.frames().min(c.frames());
        let bins_n = a.fft_size;

        // csd（frame 0）。
        let ac = al_stft::csd(&a.re[0], &a.im[0], &c.re[0], &c.im[0]);
        let (bc_re, bc_im) = xspec::csd(&a.re[0], &a.im[0], &c.re[0], &c.im[0]).unwrap();
        println!("csd: {:.3e}", max_diff_pairs(&ac, &bc_re, &bc_im));

        // Welch 蓄積: AL inline ループ vs CrossSpectrum（和）。
        let (pxx, pyy, pxy, a_coh, a_h) =
            al_welch(&a.re, &a.im, &c.re, &c.im, frames, bins_n);
        let mut acc = xspec::CrossSpectrum::new(bins_n);
        for f in 0..frames {
            acc.accumulate(&a.re[f], &a.im[f], &c.re[f], &c.im[f]).unwrap();
        }
        // 和系列の一致: AL mean × frames。
        let mut d: f64 = 0.0;
        for k in 0..bins_n {
            d = d.max((acc.sum_xx()[k] - pxx[k] * frames as f64).abs());
            d = d.max((acc.sum_yy()[k] - pyy[k] * frames as f64).abs());
            d = d.max((acc.sum_xy_re()[k] - pxy[k].0 * frames as f64).abs());
            d = d.max((acc.sum_xy_im()[k] - pxy[k].1 * frames as f64).abs());
        }
        println!("welch sums (vs mean*frames): {d:.3e}");

        // coherence / h1: AL は mean 系列から、rf は和のまま。
        let (mut b_coh_re, mut b_coh_im) = (vec![0.0; bins_n], vec![0.0; bins_n]);
        acc.coherence_into(&mut b_coh_re, &mut b_coh_im).unwrap();
        println!(
            "coherence (mean vs sum): {:.3e}",
            max_diff_pairs(&a_coh, &b_coh_re, &b_coh_im)
        );
        let (mut b_h_re, mut b_h_im) = (vec![0.0; bins_n], vec![0.0; bins_n]);
        acc.transfer_h1_into(&mut b_h_re, &mut b_h_im).unwrap();
        println!(
            "transfer_h1 (mean vs sum): {:.3e}",
            max_diff_pairs(&a_h, &b_h_re, &b_h_im)
        );

        // free-fn complex_coherence も mean 系列で一致確認。
        let pxy_re: Vec<f64> = pxy.iter().map(|v| v.0).collect();
        let pxy_im: Vec<f64> = pxy.iter().map(|v| v.1).collect();
        let (bf_re, bf_im) =
            xspec::complex_coherence(&pxx, &pyy, &pxy_re, &pxy_im).unwrap();
        println!(
            "complex_coherence fn (mean): {:.3e}",
            max_diff_pairs(&a_coh, &bf_re, &bf_im)
        );

        // envelope（magnitude 行、bins=32）。
        let am = a.magnitude();
        let ae = al_stft::envelope(&am[0], 32);
        let be = r_stft::envelope(&am[0], 32);
        println!("envelope: {:.3e}", max_abs_diff(&ae, &be));
    }

    // -- bandpass_many / BandFilter（al-math::filter 直接呼出し）--
    {
        let rate = 48_000;
        let input = fixture_samples(192_000, rate);
        let band_edges = al_stft::third_octave_bands(40.0, 16_000.0);

        let a = al_math::filter::bandpass_many(&input, rate, &band_edges);
        let b = bands::bandpass_many(&input, rate, &band_edges).unwrap();
        assert_eq!(a.len(), b.len());
        let mut d: f64 = 0.0;
        for (av, bv) in a.iter().zip(&b) {
            d = d.max(max_abs_diff(av, bv));
        }
        println!("bandpass_many ({} bands): {d:.3e}", a.len());

        // BandFilter 逐次生成 vs bandpass_many。
        let mut af = al_math::filter::BandFilter::new(&input, rate);
        let mut bf = bands::BandFilter::new(&input, rate).unwrap();
        let mut bout = vec![0.0; input.len()];
        d = 0.0;
        for &(lo, hi) in &band_edges {
            let av = af.band(lo, hi);
            bf.band_into(lo, hi, &mut bout).unwrap();
            d = d.max(max_abs_diff(&av, &bout));
        }
        println!("BandFilter.band vs band_into: {d:.3e}");

        // spectral_envelope。
        for &(len, fft_size, bins_n) in &[(8_192usize, 1_024usize, 32usize), (4_096, 1024, 32)] {
            let seg = &input[..len];
            let ae = al_math::filter::spectral_envelope(seg, fft_size, bins_n);
            let be = bands::spectral_envelope(seg, fft_size, bins_n).unwrap();
            println!(
                "spectral_envelope len={len} fft={fft_size}: {:.3e}",
                max_abs_diff(&ae, &be)
            );
        }
    }

    println!("\n=== 性能比較 (median us/call, release) ===");

    // -- stft perf: production reference shape (4096/2048 @ 4s) --
    {
        let samples = fixture_samples(192_000, 48_000);
        let a_t = bench_us(10, || {
            black_box(al_stft::stft(&samples, 4096, 2048, 48_000))
        });
        let b_t = bench_us(10, || {
            black_box(r_stft::stft(&samples, 4096, 2048, Window::Hann, 48_000).unwrap())
        });
        println!("stft 4096/2048 4s one-shot: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
        // plan 再利用 + stft_into。
        let plan = r_stft::StftPlan::new(4096, 2048, Window::Hann, 48_000).unwrap();
        let mut ws = r_stft::StftWorkspace::new(&plan);
        let frames = plan.frame_count(samples.len());
        let mut re = vec![0.0; frames * plan.fft_size()];
        let mut im = vec![0.0; frames * plan.fft_size()];
        let b2_t = bench_us(10, || {
            black_box(plan.stft_into(&mut ws, &samples, &mut re, &mut im).unwrap())
        });
        println!("stft_into (plan reuse): rf {b2_t:.1} (vs al x{:.2})", a_t / b2_t);
    }

    // -- bandpass perf: band_ncc shape (2ch × 1/3-octave @ 4s) --
    {
        let rate = 48_000;
        let src = fixture_samples(192_000, rate);
        let cap: Vec<f64> = src.iter().map(|&v| 0.9 * v).collect();
        let edges = al_stft::third_octave_bands(40.0, 16_000.0);
        let a_t = bench_us(5, || {
            black_box(al_math::filter::bandpass_many(&src, rate, &edges));
            black_box(al_math::filter::bandpass_many(&cap, rate, &edges));
        });
        let b_t = bench_us(5, || {
            black_box(bands::bandpass_many(&src, rate, &edges).unwrap());
            black_box(bands::bandpass_many(&cap, rate, &edges).unwrap());
        });
        println!(
            "bandpass_many 2ch {} bands 4s: al {a_t:.1} vs rf {b_t:.1} (x{:.2})",
            edges.len(),
            a_t / b_t
        );
        // BandFilter 逐次経路（band_gain の実形状）。
        let a2_t = bench_us(5, || {
            let mut s = al_math::filter::BandFilter::new(&src, rate);
            let mut c = al_math::filter::BandFilter::new(&cap, rate);
            for &(lo, hi) in &edges {
                black_box(s.band(lo, hi));
                black_box(c.band(lo, hi));
            }
        });
        let mut sf = bands::BandFilter::new(&src, rate).unwrap();
        let mut cf = bands::BandFilter::new(&cap, rate).unwrap();
        let mut sb = vec![0.0; src.len()];
        let mut cb = vec![0.0; cap.len()];
        let b2_t = bench_us(5, || {
            for &(lo, hi) in &edges {
                sf.band_into(lo, hi, &mut sb).unwrap();
                cf.band_into(lo, hi, &mut cb).unwrap();
            }
        });
        println!(
            "BandFilter 2ch band_into loop: al {a2_t:.1} vs rf {b2_t:.1} (x{:.2})",
            a2_t / b2_t
        );
    }

    // -- Welch 蓄積 perf（reference shape: 93 frames × 4096 bins）--
    {
        let rate = 48_000;
        let xs = fixture_samples(192_000, rate);
        let ys: Vec<f64> = xs.iter().map(|&v| 0.9 * v).collect();
        let a = al_stft::stft(&xs, 4096, 2048, rate);
        let c = al_stft::stft(&ys, 4096, 2048, rate);
        let frames = a.frames().min(c.frames());
        let bins_n = a.fft_size;
        let a_t = bench_us(30, || {
            black_box(al_welch(&a.re, &a.im, &c.re, &c.im, frames, bins_n))
        });
        let mut acc = xspec::CrossSpectrum::new(bins_n);
        let mut ore = vec![0.0; bins_n];
        let mut oim = vec![0.0; bins_n];
        let b_t = bench_us(30, || {
            for f in 0..frames {
                acc.accumulate(&a.re[f], &a.im[f], &c.re[f], &c.im[f]).unwrap();
            }
            acc.coherence_into(&mut ore, &mut oim).unwrap();
            acc.transfer_h1_into(&mut ore, &mut oim).unwrap();
            black_box((&ore, &oim));
        });
        println!("welch+h1+coh {frames}f: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
    }

    // -- spectral_envelope perf --
    {
        let input = fixture_samples(8_192, 48_000);
        let a_t = bench_us(50, || {
            black_box(al_math::filter::spectral_envelope(&input, 1024, 32))
        });
        let b_t = bench_us(50, || {
            black_box(bands::spectral_envelope(&input, 1024, 32).unwrap())
        });
        println!("spectral_envelope 8k/1024/32: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
    }

    // -- band_gain 形状: stft×2 + psd×2 + mean + band_power --
    {
        let rate = 48_000;
        let xs = fixture_samples(192_000, rate);
        let ys: Vec<f64> = xs.iter().map(|&v| 0.9 * v).collect();
        let edges = al_stft::third_octave_bands(40.0, 16_000.0);
        // AL: stft → psd → 全フレーム平均 → band_power。
        let a_t = bench_us(5, || {
            let ms = al_stft::stft(&xs, 4096, 2048, rate);
            let mc = al_stft::stft(&ys, 4096, 2048, rate);
            let frames = ms.frames().min(mc.frames());
            let ps = ms.psd();
            let pc = mc.psd();
            let mut mean_s = vec![0.0; ms.fft_size];
            let mut mean_c = vec![0.0; ms.fft_size];
            for f in 0..frames {
                for k in 0..ms.fft_size {
                    mean_s[k] += ps[f][k];
                    mean_c[k] += pc[f][k];
                }
            }
            for k in 0..ms.fft_size {
                mean_s[k] /= frames as f64;
                mean_c[k] /= frames as f64;
            }
            black_box(al_stft::band_power(&mean_s, rate, ms.fft_size, &edges));
            black_box(al_stft::band_power(&mean_c, rate, ms.fft_size, &edges));
        });
        // rf: stft → psd → 全フレーム平均 → band_power（同等の手順を合成）。
        let plan = r_stft::StftPlan::new(4096, 2048, Window::Hann, rate).unwrap();
        let mut ws = r_stft::StftWorkspace::new(&plan);
        let bin_hz = plan.bin_hz();
        let b_t = bench_us(5, || {
            let ms = plan.stft(&mut ws, &xs).unwrap();
            let mc = plan.stft(&mut ws, &ys).unwrap();
            let frames = ms.frames().min(mc.frames());
            let bins_n = ms.fft_size();
            let ps = ms.psd();
            let pc = mc.psd();
            let mut mean_s = vec![0.0; bins_n];
            let mut mean_c = vec![0.0; bins_n];
            for f in 0..frames {
                for k in 0..bins_n {
                    mean_s[k] += ps[f * bins_n + k];
                    mean_c[k] += pc[f * bins_n + k];
                }
            }
            for k in 0..bins_n {
                mean_s[k] /= frames as f64;
                mean_c[k] /= frames as f64;
            }
            black_box(bands::band_power(&mean_s, bin_hz, &edges));
            black_box(bands::band_power(&mean_c, bin_hz, &edges));
        });
        println!("band_gain shape 2ch 4s: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
    }

    println!("\n=== メモリ / alloc 比較 (count, bytes) ===");

    // stft 呼出し alloc: AL Vec<Vec> vs rf 連続 vs stft_into。
    {
        let samples = fixture_samples(96_000, 48_000);
        let (c0, b0) = alloc_stats();
        let am = al_stft::stft(&samples, 4096, 2048, 48_000);
        let (c1, b1) = alloc_stats();
        black_box(&am);
        println!("al stft() call: +{} allocs, {} B", c1 - c0, b1 - b0);
        let (c0, b0) = alloc_stats();
        let bm = r_stft::stft(&samples, 4096, 2048, Window::Hann, 48_000).unwrap();
        let (c1, b1) = alloc_stats();
        println!("rf stft() call (plan+ws+matrix): +{} allocs, {} B", c1 - c0, b1 - b0);
        let plan = r_stft::StftPlan::new(4096, 2048, Window::Hann, 48_000).unwrap();
        let mut ws = r_stft::StftWorkspace::new(&plan);
        let frames = plan.frame_count(samples.len());
        let mut re = vec![0.0; frames * plan.fft_size()];
        let mut im = vec![0.0; frames * plan.fft_size()];
        let (c0, b0) = alloc_stats();
        plan.stft_into(&mut ws, &samples, &mut re, &mut im).unwrap();
        let (c1, b1) = alloc_stats();
        black_box((&re, &im));
        println!("rf stft_into call (buffers prealloc): +{} allocs, {} B", c1 - c0, b1 - b0);
        println!(
            "rf StftPlan {} B / ws {} B / matrix {} B (memory_bytes)",
            plan.memory_bytes(),
            ws.memory_bytes(),
            bm.memory_bytes()
        );
    }

    // BandFilter 構築・帯域生成 alloc。
    {
        let rate = 48_000;
        let input = fixture_samples(192_000, rate);
        let (c0, b0) = alloc_stats();
        let af = al_math::filter::BandFilter::new(&input, rate);
        let (c1, b1) = alloc_stats();
        black_box(&af);
        println!("al BandFilter::new: +{} allocs, {} B", c1 - c0, b1 - b0);
        let (c0, b0) = alloc_stats();
        let bf = bands::BandFilter::new(&input, rate).unwrap();
        let (c1, b1) = alloc_stats();
        println!("rf BandFilter::new: +{} allocs, {} B", c1 - c0, b1 - b0);
        println!("rf BandFilter memory_bytes: {} B", bf.memory_bytes());
        let mut af = al_math::filter::BandFilter::new(&input, rate);
        let (c0, b0) = alloc_stats();
        black_box(af.band(100.0, 1000.0));
        let (c1, b1) = alloc_stats();
        println!("al band() call: +{} allocs, {} B", c1 - c0, b1 - b0);
        let mut bf = bands::BandFilter::new(&input, rate).unwrap();
        let mut out = vec![0.0; input.len()];
        let (c0, b0) = alloc_stats();
        bf.band_into(100.0, 1000.0, &mut out).unwrap();
        let (c1, b1) = alloc_stats();
        println!("rf band_into call: +{} allocs, {} B", c1 - c0, b1 - b0);
    }

    // CrossSpectrum alloc。
    {
        let (c0, b0) = alloc_stats();
        let mut acc = xspec::CrossSpectrum::new(4096);
        let (c1, b1) = alloc_stats();
        println!("rf CrossSpectrum::new(4096): +{} allocs, {} B", c1 - c0, b1 - b0);
        let x = vec![0.0; 4096];
        let (c0, b0) = alloc_stats();
        acc.accumulate(&x, &x, &x, &x).unwrap();
        let (c1, b1) = alloc_stats();
        println!("rf accumulate call: +{} allocs, {} B", c1 - c0, b1 - b0);
    }
}
