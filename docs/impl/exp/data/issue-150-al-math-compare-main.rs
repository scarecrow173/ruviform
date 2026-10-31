//! Issue #150: ruviform-numeric vs al-math (#98 frozen baseline) の
//! 精度・性能・メモリ比較ハーネス。どちらのリポジトリも変更しない。
//!
//! 実行: cargo run --release (from %TEMP%/r150-compare)

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

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

// === al-locate::dsp::conv を verbatim 移植（依存は al_math::fft のみ）===
fn al_conv(signal: &[f64], kernel: &[f64]) -> Vec<f64> {
    if kernel.is_empty() {
        return vec![0.0; signal.len()];
    }
    if signal.is_empty() {
        return Vec::new();
    }
    let n = signal.len();
    let m = kernel.len();
    let b = (2 * m).next_power_of_two().max(1024);
    let l = b - m + 1;
    let bins = b / 2 + 1;
    let mut plan = al_math::fft::RealFft::new(b);
    let mut hbuf = vec![0.0; b];
    hbuf[..m].copy_from_slice(kernel);
    let mut hre = vec![0.0; bins];
    let mut him = vec![0.0; bins];
    plan.forward(&hbuf, &mut hre, &mut him);
    let mut xbuf = vec![0.0; b];
    let mut xre = vec![0.0; bins];
    let mut xim = vec![0.0; bins];
    let mut obuf = vec![0.0; b];
    let mut y = vec![0.0; n];
    for off in (0..n).step_by(l) {
        let take = l.min(n - off);
        xbuf.copy_within(b + 1 - m.., 0);
        xbuf[m - 1..m - 1 + take].copy_from_slice(&signal[off..off + take]);
        xbuf[m - 1 + take..].fill(0.0);
        plan.forward(&xbuf, &mut xre, &mut xim);
        for k in 0..bins {
            let zre = xre[k] * hre[k] - xim[k] * him[k];
            let zim = xre[k] * him[k] + xim[k] * hre[k];
            xre[k] = zre;
            xim[k] = zim;
        }
        plan.inverse(&xre, &xim, &mut obuf);
        y[off..off + take].copy_from_slice(&obuf[m - 1..m - 1 + take]);
    }
    y
}

// === 決定的入力 ===
fn signal(n: usize, f1: f64, f2: f64) -> Vec<f64> {
    (0..n)
        .map(|i| (i as f64 * f1).sin() + 0.5 * (i as f64 * f2).cos())
        .collect()
}
fn complex_input(n: usize) -> (Vec<f64>, Vec<f64>) {
    (signal(n, 0.37, 0.11), signal(n, 0.23, 0.41))
}

fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(&x, &y)| (x - y).abs())
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

    // -- complex FFT --
    for &n in &[1024usize, 8192, 65536] {
        let (re0, im0) = complex_input(n);
        let (mut ar, mut ai) = (re0.clone(), im0.clone());
        let (mut br, mut bi) = (re0.clone(), im0.clone());
        al_math::fft::Fft::new(n).transform(&mut ar, &mut ai, false);
        let plan = ruviform_numeric::fft::FftPlan::new(n).unwrap();
        let mut ws = ruviform_numeric::fft::FftWorkspace::new(&plan);
        plan.transform(&mut ws, &mut br, &mut bi, false).unwrap();
        println!(
            "cfft fwd n={n}: re {:.3e} im {:.3e}",
            max_abs_diff(&ar, &br),
            max_abs_diff(&ai, &bi)
        );
        al_math::fft::Fft::new(n).transform(&mut ar, &mut ai, true);
        plan.transform(&mut ws, &mut br, &mut bi, true).unwrap();
        println!(
            "cfft roundtrip n={n}: re {:.3e} im {:.3e}",
            max_abs_diff(&ar, &br),
            max_abs_diff(&ai, &bi)
        );
    }

    // -- real FFT --
    for &n in &[1024usize, 8192, 65536] {
        let input = signal(n, 0.37, 0.13);
        let bins = n / 2 + 1;
        let (mut are, mut aim) = (vec![0.0; bins], vec![0.0; bins]);
        let (mut bre, mut bim) = (vec![0.0; bins], vec![0.0; bins]);
        let mut afft = al_math::fft::RealFft::new(n);
        afft.forward(&input, &mut are, &mut aim);
        let mut bfft = ruviform_numeric::fft::RealFft::new(n).unwrap();
        bfft.forward(&input, &mut bre, &mut bim).unwrap();
        println!(
            "rfft fwd n={n}: re {:.3e} im {:.3e}",
            max_abs_diff(&are, &bre),
            max_abs_diff(&aim, &bim)
        );
        let (mut ao, mut bo) = (vec![0.0; n], vec![0.0; n]);
        afft.inverse(&are, &aim, &mut ao);
        bfft.inverse(&are, &aim, &mut bo).unwrap();
        println!("rfft inv n={n}: {:.3e}", max_abs_diff(&ao, &bo));
    }

    // -- vec ops --
    {
        let n = 1 << 20;
        let x = signal(n, 0.013, 0.007);
        let y = signal(n, 0.019, 0.003);
        use al_math::vec as av;
        use ruviform_numeric::vec as bv;
        println!(
            "dot: {:.3e}",
            (av::dot(&x, &y) - bv::dot(&x, &y).unwrap()).abs()
        );
        println!("sum_sq: {:.3e}", (av::sum_sq(&x) - bv::sum_sq(&x)).abs());
        println!(
            "corr0: {:.3e}",
            (av::corr0(&x, &y) - bv::corr0(&x, &y).unwrap()).abs()
        );
        println!("rms: {:.3e}", (av::rms(&x) - bv::rms(&x)).abs());
        let (mut ao, mut bo) = (vec![0.0; n], vec![0.0; n]);
        av::scale(&x, 1.7, &mut ao);
        bv::scale(&x, 1.7, &mut bo).unwrap();
        println!("scale: {:.3e}", max_abs_diff(&ao, &bo));
        let (mut ay, mut by) = (y.clone(), y.clone());
        av::axpy(&mut ay, 0.3, &x);
        bv::axpy(&mut by, 0.3, &x).unwrap();
        println!("axpy: {:.3e}", max_abs_diff(&ay, &by));
        av::sub_scaled(&mut ay, 0.2, &x);
        bv::sub_scaled(&mut by, 0.2, &x).unwrap();
        println!("sub_scaled: {:.3e}", max_abs_diff(&ay, &by));
        av::mag_sq(&x, &y, &mut ao);
        bv::mag_sq(&x, &y, &mut bo).unwrap();
        println!("mag_sq: {:.3e}", max_abs_diff(&ao, &bo));
    }

    // -- resample --
    for &(src, tgt, frames) in &[(48000u32, 96000u32, 48000usize), (48000, 44100, 48000)] {
        let input = signal(frames, 0.05, 0.031);
        let aplan = al_math::resample::ResamplePlan::new(src, tgt).unwrap();
        let ao = al_math::resample::resample(&input, &aplan).unwrap();
        let bplan = ruviform_numeric::resample::ResamplePlan::new(src, tgt).unwrap();
        let bo = bplan.resample(&input).unwrap();
        assert_eq!(ao.len(), bo.len());
        println!("resample {src}->{tgt}: {:.3e}", max_abs_diff(&ao, &bo));
    }

    // -- linalg --
    {
        let c1: Vec<f64> = (0..64).map(|i| (i as f64 * 0.1).sin()).collect();
        let c2: Vec<f64> = (0..64).map(|i| (i as f64 * 0.07).cos()).collect();
        let y: Vec<f64> = (0..64).map(|i| 0.7 * c1[i] - 0.3 * c2[i] + 0.01).collect();
        let (ax, arms) = al_math::linalg::least_squares(&[&c1, &c2], &y, 1e-3).unwrap();
        let (bx, brms) = ruviform_numeric::linalg::least_squares(&[&c1, &c2], &y, 1e-3).unwrap();
        println!(
            "lstsq: x {:.3e} rms {:.3e}",
            max_abs_diff(&ax, &bx),
            (arms - brms).abs()
        );
    }

    // -- conv --
    {
        let input = signal(100_000, 0.021, 0.013);
        let kernel = signal(256, 0.11, 0.07);
        let ao = al_conv(&input, &kernel);
        let bo = ruviform_numeric::conv::conv(&input, &kernel);
        println!("conv: {:.3e}", max_abs_diff(&ao, &bo));
    }

    println!("\n=== 性能比較 (median us/call, release) ===");

    // -- FFT perf --
    for &n in &[1024usize, 16384, 131072] {
        let (re0, im0) = complex_input(n);
        let afft = al_math::fft::Fft::new(n);
        let bplan = ruviform_numeric::fft::FftPlan::new(n).unwrap();
        let mut bws = ruviform_numeric::fft::FftWorkspace::new(&bplan);
        let (mut ar, mut ai) = (re0.clone(), im0.clone());
        let (mut br, mut bi) = (re0, im0);
        let a_t = bench_us(30, || black_box(afft.transform(&mut ar, &mut ai, false)));
        let b_t = bench_us(30, || {
            black_box(bplan.transform(&mut bws, &mut br, &mut bi, false).unwrap())
        });
        println!("cfft n={n}: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
    }
    for &n in &[8192usize, 65536] {
        let input = signal(n, 0.37, 0.13);
        let bins = n / 2 + 1;
        let mut afft = al_math::fft::RealFft::new(n);
        let (mut are, mut aim) = (vec![0.0; bins], vec![0.0; bins]);
        let bplan = ruviform_numeric::fft::RealFftPlan::new(n).unwrap();
        let mut bws = ruviform_numeric::fft::RealFftWorkspace::new(&bplan);
        let (mut bre, mut bim) = (vec![0.0; bins], vec![0.0; bins]);
        let a_t = bench_us(30, || black_box(afft.forward(&input, &mut are, &mut aim)));
        let b_t = bench_us(30, || {
            black_box(bplan.forward(&mut bws, &input, &mut bre, &mut bim).unwrap())
        });
        println!("rfft fwd n={n}: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
    }

    // -- vec perf --
    {
        let n = 1 << 20;
        let x = signal(n, 0.013, 0.007);
        let y = signal(n, 0.019, 0.003);
        let a_t = bench_us(50, || black_box(al_math::vec::dot(&x, &y)));
        let b_t = bench_us(50, || {
            black_box(ruviform_numeric::vec::dot(&x, &y).unwrap())
        });
        println!("dot n={n}: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
        let a_t = bench_us(50, || black_box(al_math::vec::sum_sq(&x)));
        let b_t = bench_us(50, || black_box(ruviform_numeric::vec::sum_sq(&x)));
        println!("sum_sq n={n}: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
    }

    // -- resample perf --
    for &(src, tgt, frames) in &[(48000u32, 96000u32, 48000usize), (48000, 44100, 48000)] {
        let input = signal(frames, 0.05, 0.031);
        let aplan = al_math::resample::ResamplePlan::new(src, tgt).unwrap();
        let bplan = ruviform_numeric::resample::ResamplePlan::new(src, tgt).unwrap();
        let outlen = bplan.output_len(frames).unwrap();
        let mut bout = vec![0.0; outlen];
        let a_t = bench_us(10, || {
            black_box(al_math::resample::resample(&input, &aplan).unwrap())
        });
        let b_t = bench_us(10, || {
            black_box(bplan.resample_into(&input, &mut bout).unwrap())
        });
        println!("resample {src}->{tgt} {frames}f: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
    }

    // -- conv perf --
    {
        let input = signal(1_000_000, 0.021, 0.013);
        let kernel = signal(256, 0.11, 0.07);
        let bplan = ruviform_numeric::conv::ConvPlan::new(&kernel).unwrap();
        let mut bws = ruviform_numeric::conv::ConvWorkspace::new(&bplan);
        let mut bout = vec![0.0; input.len()];
        let a_t = bench_us(7, || black_box(al_conv(&input, &kernel)));
        let b_t = bench_us(7, || {
            black_box(bplan.convolve(&mut bws, &input, &mut bout).unwrap())
        });
        println!("conv n=1M m=256: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
    }

    // -- linalg perf --
    {
        let cols_vec: Vec<Vec<f64>> = (0..4)
            .map(|j| (0..256).map(|i| (i as f64 * 0.01 * (j + 1) as f64).sin()).collect())
            .collect();
        let cols: Vec<&[f64]> = cols_vec.iter().map(Vec::as_slice).collect();
        let y = signal(256, 0.05, 0.02);
        let a_t = bench_us(200, || {
            black_box(al_math::linalg::least_squares(&cols, &y, 1e-3).unwrap())
        });
        let b_t = bench_us(200, || {
            black_box(ruviform_numeric::linalg::least_squares(&cols, &y, 1e-3).unwrap())
        });
        println!("lstsq m=256 n=4: al {a_t:.1} vs rf {b_t:.1} (x{:.2})", a_t / b_t);
    }

    println!("\n=== メモリ / alloc 比較 (count, bytes) ===");

    // plan 構築時 alloc
    let (c0, b0) = alloc_stats();
    let afft = al_math::fft::Fft::new(16384);
    let (c1, b1) = alloc_stats();
    black_box(&afft);
    println!("al Fft::new(16384): +{} allocs, {} B", c1 - c0, b1 - b0);
    let (c0, b0) = alloc_stats();
    let bplan = ruviform_numeric::fft::FftPlan::new(16384).unwrap();
    let (c1, b1) = alloc_stats();
    black_box(&bplan);
    println!("rf FftPlan::new(16384): +{} allocs, {} B", c1 - c0, b1 - b0);
    let (c0, b0) = alloc_stats();
    let bws = ruviform_numeric::fft::FftWorkspace::new(&bplan);
    let (c1, b1) = alloc_stats();
    black_box(&bws);
    println!("rf FftWorkspace::new: +{} allocs, {} B", c1 - c0, b1 - b0);

    // transform 呼出し中 alloc（al-math はスレッド毎に初回 TLS 確保、
    // 以降 0; ruviform は常時 0）。フレッシュスレッドで TLS を測る。
    let (re0, im0) = complex_input(16384);
    let (mut ar, mut ai) = (re0.clone(), im0.clone());
    let afft2 = al_math::fft::Fft::new(16384);
    std::thread::scope(|s| {
        s.spawn(|| {
            let (c0, b0) = alloc_stats();
            afft2.transform(&mut ar, &mut ai, false);
            let (c1, b1) = alloc_stats();
            println!(
                "al transform 1st call (fresh thread): +{} allocs, {} B (TLS work)",
                c1 - c0,
                b1 - b0
            );
            let (c0, b0) = alloc_stats();
            afft2.transform(&mut ar, &mut ai, false);
            let (c1, b1) = alloc_stats();
            println!("al transform 2nd call (same thread): +{} allocs, {} B", c1 - c0, b1 - b0);
        })
        .join()
        .unwrap();
    });
    let (mut br, mut bi) = (re0.clone(), im0.clone());
    let mut bws = ruviform_numeric::fft::FftWorkspace::new(&bplan);
    let (c0, b0) = alloc_stats();
    bplan.transform(&mut bws, &mut br, &mut bi, false).unwrap();
    let (c1, b1) = alloc_stats();
    println!("rf transform 1st call: +{} allocs, {} B", c1 - c0, b1 - b0);

    // resample 呼出し中 alloc
    let input = signal(48000, 0.05, 0.031);
    let aplan = al_math::resample::ResamplePlan::new(48000, 96000).unwrap();
    black_box(al_math::resample::resample(&input, &aplan).unwrap()); // warm
    let (c0, b0) = alloc_stats();
    black_box(al_math::resample::resample(&input, &aplan).unwrap());
    let (c1, b1) = alloc_stats();
    println!("al resample 2nd call: +{} allocs, {} B", c1 - c0, b1 - b0);
    let bplan = ruviform_numeric::resample::ResamplePlan::new(48000, 96000).unwrap();
    let mut bout = vec![0.0; bplan.output_len(48000).unwrap()];
    let (c0, b0) = alloc_stats();
    bplan.resample_into(&input, &mut bout).unwrap();
    let (c1, b1) = alloc_stats();
    println!("rf resample_into call: +{} allocs, {} B", c1 - c0, b1 - b0);
    println!("rf ResamplePlan table: {} B (memory_bytes)", bplan.memory_bytes());
}
