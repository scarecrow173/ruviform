//! Issue #152: ruviform-numeric::corr vs al-math::correlation（#98 凍結
//! baseline）の精度・性能・メモリ比較ハーネス。どちらのリポジトリも変更しない。
//!
//! 実行: cargo run --release（%TEMP%/r152-compare から）
//!
//! al-math 側は SIMD dispatch（pulp V3）込みの現行実装、ruviform 側は
//! scalar 実装。形状は al-locate/al-analysis の実運用形状から選ぶ。

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

// === 割付計測 allocator（#150 ハーネスと同じ）===
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

fn alloc_reset() {
    ALLOC_COUNT.store(0, Ordering::Relaxed);
    ALLOC_BYTES.store(0, Ordering::Relaxed);
}

// === 決定的入力（corr.rs テストと同じ LCG）===
fn noise(len: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..len)
        .map(|_| {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((s >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
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

fn argmax_abs(curve: &[f64]) -> usize {
    curve
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map(|(i, _)| i)
        .unwrap()
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

/// corr.rs の NccDirect 分子ループを verbatim 複写
/// （ルーティング検証で FFT 経路と実測比較するため）。
fn direct_ncc_num(template: &[f64], signal: &[f64], out: &mut [f64]) {
    out.fill(0.0);
    let k = out.len();
    for (i, &ai) in template.iter().enumerate() {
        for (o, &sv) in out.iter_mut().zip(&signal[i..i + k]) {
            *o += ai * sv;
        }
    }
}

fn main() {
    // ==================== 精度比較 ====================
    println!("=== 精度比較: NCC (al-math vs ruviform, max |diff|) ===");
    // (template_len, signal_len, 期待 mode)。mode は K=S−L+1 と
    // コストモデル L·K <= 8·n·log2(n) で推定（ruviform 側は fft_len()==0 が
    // NccDirect の観測手段）。
    for &(l, s, reps_note) in &[
        (4_410usize, 7_938usize, "band_gain window 44.1k"),
        (4_800, 8_640, "band_gain window 48k"),
        (4_410, 66_150, "narrow anchor mid"),
        (96_000, 100_800, "narrow anchor big"),
        (2_205_000, 2_205_236, "band_ncc/full_ncc (direct)"),
        (551_250, 551_309, "coarse_ncc (direct)"),
        (800, 1_438, "tiny 8kHz"),
    ] {
        let template = noise(l, 11);
        let signal = noise(s, 13);
        let ap = al_math::correlation::CorrelationPlan::new_ncc(l, s);
        let mut aws = al_math::correlation::CorrelationWorkspace::new(&ap);
        let ac = ap.ncc(&mut aws, &template, &signal, usize::MAX).to_vec();
        let rp = ruviform_numeric::corr::CorrelationPlan::new_ncc(l, s).unwrap();
        let mut rws = ruviform_numeric::corr::CorrelationWorkspace::new(&rp);
        let rc = rp
            .ncc(&mut rws, &template, &signal, usize::MAX)
            .unwrap()
            .values
            .to_vec();
        println!(
            "ncc L={l} S={s} ({reps_note}): max|d|={:.3e} argmax al={} rf={} rf_fft_len={}",
            max_abs_diff(&ac, &rc),
            argmax_abs(&ac),
            argmax_abs(&rc),
            rp.fft_len()
        );
    }

    println!("\n=== 精度比較: GCC-PHAT ===");
    for &(l, s, note) in &[
        (4_410usize, 7_938usize, "band_gain window"),
        (4_800, 8_640, "48k"),
        (4410, 66_150, "narrow anchor mid"),
    ] {
        let template = noise(l, 21);
        let mut signal = noise(s, 23);
        // 既知オフセットの埋込み（argmax 一致性も見る）。
        let off = (s - l) / 2;
        for (i, &v) in template.iter().enumerate() {
            signal[off + i] = v;
        }
        let ap = al_math::correlation::CorrelationPlan::new(l, s);
        let mut aws = al_math::correlation::CorrelationWorkspace::new(&ap);
        let ac = ap
            .gcc_phat(&mut aws, &template, &signal, usize::MAX)
            .to_vec();
        let rp = ruviform_numeric::corr::CorrelationPlan::new(l, s).unwrap();
        let mut rws = ruviform_numeric::corr::CorrelationWorkspace::new(&rp);
        let rc = rp
            .gcc_phat(&mut rws, &template, &signal, usize::MAX)
            .unwrap()
            .to_vec();
        println!(
            "phat L={l} S={s} ({note}): max|d|={:.3e} argmax al={} rf={} (true={off})",
            max_abs_diff(&ac, &rc),
            argmax_abs(&ac),
            argmax_abs(&rc)
        );
    }

    println!("\n=== 精度比較: overlap_corr_curve ===");
    for &(l, max_lag, note) in &[
        (8_192usize, 512usize, "reference lag_window/lag_max"),
        (1_024, 128, "small window"),
        (300, 130, "non-pow2 window"),
    ] {
        let x = noise(l, 7);
        let y = noise(l, 11);
        let ac = al_math::correlation::overlap_corr_curve(&x, &y, max_lag);
        let rc = ruviform_numeric::corr::overlap_corr_curve(&x, &y, max_lag).unwrap();
        println!(
            "overlap L={l} max_lag={max_lag} ({note}): max|d|={:.3e} argmax al={} rf={}",
            max_abs_diff(&ac, &rc),
            argmax_abs(&ac),
            argmax_abs(&rc)
        );
    }

    // ==================== 性能比較 ====================
    println!("\n=== 性能比較: per-call 中央値 [us]（plan+ws は再利用）===");
    for &(l, s, reps) in &[
        (4_410usize, 7_938usize, 50usize),
        (4_800, 8_640, 50),
        (4_410, 66_150, 20),
        (96_000, 100_800, 15),
        (2_205_000, 2_205_236, 5),
        (551_250, 551_309, 5),
        (800, 1_438, 100),
    ] {
        let template = noise(l, 11);
        let signal = noise(s, 13);
        let ap = al_math::correlation::CorrelationPlan::new_ncc(l, s);
        let mut aws = al_math::correlation::CorrelationWorkspace::new(&ap);
        let rp = ruviform_numeric::corr::CorrelationPlan::new_ncc(l, s).unwrap();
        let mut rws = ruviform_numeric::corr::CorrelationWorkspace::new(&rp);
        let ta = bench_us(reps, || {
            black_box(ap.ncc(&mut aws, &template, &signal, usize::MAX)[0])
        });
        let tr = bench_us(reps, || {
            black_box(
                rp.ncc(&mut rws, &template, &signal, usize::MAX)
                    .unwrap()
                    .values[0],
            )
        });
        println!(
            "ncc L={l} S={s}: al {ta:.1} rf {tr:.1} ratio {:.2}x (rf_fft_len={})",
            tr / ta,
            rp.fft_len()
        );
    }
    for &(l, s, reps) in &[
        (4_410usize, 7_938usize, 50usize),
        (4_800, 8_640, 50),
        (4_410, 66_150, 20),
    ] {
        let template = noise(l, 21);
        let signal = noise(s, 23);
        let ap = al_math::correlation::CorrelationPlan::new(l, s);
        let mut aws = al_math::correlation::CorrelationWorkspace::new(&ap);
        let rp = ruviform_numeric::corr::CorrelationPlan::new(l, s).unwrap();
        let mut rws = ruviform_numeric::corr::CorrelationWorkspace::new(&rp);
        let ta = bench_us(reps, || {
            black_box(ap.gcc_phat(&mut aws, &template, &signal, usize::MAX)[0])
        });
        let tr = bench_us(reps, || {
            black_box(
                rp.gcc_phat(&mut rws, &template, &signal, usize::MAX)
                    .unwrap()[0],
            )
        });
        println!(
            "phat L={l} S={s}: al {ta:.1} rf {tr:.1} ratio {:.2}x",
            tr / ta
        );
    }
    for &(l, max_lag, reps) in &[
        (8_192usize, 512usize, 30usize),
        (1_024, 128, 100),
        (300, 130, 200),
    ] {
        let x = noise(l, 7);
        let y = noise(l, 11);
        let ap = al_math::correlation::OverlapCorrPlan::new(l);
        let mut aws = al_math::correlation::OverlapCorrWorkspace::new(&ap);
        let rp = ruviform_numeric::corr::OverlapCorrPlan::new(l).unwrap();
        let mut rws = ruviform_numeric::corr::OverlapCorrWorkspace::with_max_lag(&rp, max_lag);
        let ta = bench_us(reps, || {
            black_box(ap.curve(&mut aws, &x, &y, max_lag)[0])
        });
        let tr = bench_us(reps, || {
            black_box(rp.curve(&mut rws, &x, &y, max_lag).unwrap().values[0])
        });
        println!(
            "overlap L={l} max_lag={max_lag}: al {ta:.1} rf {tr:.1} ratio {:.2}x",
            tr / ta
        );
    }

    // ==================== メモリ/alloc 比較 ====================
    println!("\n=== メモリ/alloc ===");
    for &(l, s) in &[(4_410usize, 7_938usize), (96_000, 100_800), (2_205_000, 2_205_236)] {
        alloc_reset();
        let ap = al_math::correlation::CorrelationPlan::new_ncc(l, s);
        let mut aws = al_math::correlation::CorrelationWorkspace::new(&ap);
        let (ac0, ab0) = alloc_stats();
        alloc_reset();
        let rp = ruviform_numeric::corr::CorrelationPlan::new_ncc(l, s).unwrap();
        let mut rws = ruviform_numeric::corr::CorrelationWorkspace::new(&rp);
        let (rc0, rb0) = alloc_stats();
        // per-call alloc（warmup 後の測定窓）。al-math は TLS FFT workspace の
        // 初回確保が入りうるため warmup を先に流す。
        let template = noise(l, 11);
        let signal = noise(s, 13);
        black_box(ap.ncc(&mut aws, &template, &signal, usize::MAX)[0]);
        alloc_reset();
        black_box(ap.ncc(&mut aws, &template, &signal, usize::MAX)[0]);
        let (ac1, _) = alloc_stats();
        alloc_reset();
        black_box(
            rp.ncc(&mut rws, &template, &signal, usize::MAX)
                .unwrap()
                .values[0],
        );
        let (rc1, _) = alloc_stats();
        println!(
            "ws L={l} S={s}: al {ac0}allocs/{ab0}B rf {rc0}allocs/{rb0}B | call-alloc al={ac1} rf={rc1} (rf_ws_bytes={})",
            rws.memory_bytes()
        );
    }

    // ==================== ルーティング検証 ====================
    println!("\n=== ルーティング検証: direct 実測 vs FFT 経路 ===");
    // 境界形状 (800,1025): K=226, L·K=180_800 vs 8·n·log2n=180_224 →
    // コストモデルは NccValid(FFT)。実測で direct が速ければ係数は
    // 保守側（境界直下）、遅ければ妥当。Full plan ncc 呼出を FFT 経路
    // コストとして使う（Full の n == NccValid の n になる形状を選ぶ）。
    for &(l, s) in &[
        (800usize, 840usize),  // K=41 → direct 深い側
        (800, 1_025),          // K=226 → 境界 (C_eff≈8.0)
        (800, 1_438),          // K=639 → FFT 深い側
        (2_000, 2_040),        // K=41 → 大 L の direct
    ] {
        let n = s.next_power_of_two();
        let k = s - l + 1;
        let c_eff = l as f64 * k as f64 / (n as f64 * (n as f64).log2());
        let template = noise(l, 11);
        let signal = noise(s, 13);
        // FFT 経路コスト: 同 n の Full plan ncc 呼出（実経路そのもの）。
        let rp_full = ruviform_numeric::corr::CorrelationPlan::new(l, s).unwrap();
        let mut ws_full = ruviform_numeric::corr::CorrelationWorkspace::new(&rp_full);
        let t_fft = bench_us(50, || {
            black_box(
                rp_full
                    .ncc(&mut ws_full, &template, &signal, usize::MAX)
                    .unwrap()
                    .values[0],
            )
        });
        // direct 経路コスト: verbatim 分子ループ（K ラグ出力）。
        let mut out = vec![0.0; k];
        let t_direct = bench_us(50, || {
            direct_ncc_num(&template, &signal, &mut out);
            black_box(out[0])
        });
        let rp = ruviform_numeric::corr::CorrelationPlan::new_ncc(l, s).unwrap();
        println!(
            "L={l} S={s} K={k} n={n}: C_eff={c_eff:.1} t_direct={t_direct:.1}us t_fft={t_fft:.1}us chosen_fft_len={}",
            rp.fft_len()
        );
    }
}
