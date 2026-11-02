//! Issue #255 変換量削減の実測プローブ（手動計測用、CI 対象外）。
//! `cargo run --release -p ruviform-numeric --example issue255_perf`
//!
//! `AudioLens` 支配形状で、旧 `nextpow2` 変換長と現行 smooth 変換長の
//! 実数 FFT forward/inverse ペア時間を比較する。ベンチ枠組みではなく
//! 簡易計時だが、選択アルゴリズムの perf 回帰確認に使える。

use ruviform_numeric::fft::{RealFftPlan, RealFftWorkspace};
use std::time::Instant;

// 比率表示のみの計測例のため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
fn measure(n: usize) -> f64 {
    let plan = RealFftPlan::new(n).unwrap();
    let mut ws = RealFftWorkspace::new(&plan);
    let input = vec![0.5f64; n];
    let bins = n / 2 + 1;
    let mut fre = vec![0.0; bins];
    let mut fim = vec![0.0; bins];
    let mut out = vec![0.0; n];
    let iters = (96_000_000usize / n).max(3);
    plan.forward(&mut ws, &input, &mut fre, &mut fim).unwrap();
    let t0 = Instant::now();
    for _ in 0..iters {
        plan.forward(&mut ws, &input, &mut fre, &mut fim).unwrap();
        plan.inverse(&mut ws, &fre, &fim, &mut out).unwrap();
    }
    t0.elapsed().as_secs_f64() / iters as f64 * 1e6
}

#[expect(clippy::cast_precision_loss)]
fn main() {
    // (shape 名, 旧 nextpow2 長, 新 smooth 長)
    for (label, old_n, new_n) in [
        ("S=8,640 ncc-valid", 16_384usize, 8_640usize),
        ("S=66,150 anchor-mid", 131_072, 67_500),
        ("S=2,205,236 band-ncc", 4_194_304, 2_211_840),
        ("S=2,400,240 anchor-ncc", 4_194_304, 2_430_000),
    ] {
        let t_old = measure(old_n);
        let t_new = measure(new_n);
        println!(
            "{label:>22}: n {old_n:>8} -> {new_n:>8} ({:>5.1}% len), \
             {t_old:>10.1} -> {t_new:>10.1} us ({:>5.1}% time)",
            100.0 * new_n as f64 / old_n as f64,
            100.0 * t_new / t_old,
        );
    }
    // 旧経路が保持されることの sanity: 2 冪長同士の比較は ~1.0 になるはず。
    let t_a = measure(65_536);
    let t_b = measure(65_536);
    println!(
        "pow2 self-check 65536: {t_a:.1} vs {t_b:.1} us (ratio {:.3})",
        t_b / t_a
    );
}
