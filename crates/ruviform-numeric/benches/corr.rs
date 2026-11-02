//! `corr` コストモデル校正用マイクロベンチ（Issue #265）。
//!
//! 2 つの `BenchmarkGroup` を持つ:
//!
//! - **`fft_len`**: 選択候補となる変換長の `forward`+`inverse` ペア実測。
//!   `Throughput::Elements(n)` なので per-element 実コストが直接読め、
//!   `fft_cost_units` の radix 係数（2 冪で `u=log2(n)` となるよう
//!   正規化）を再フィットするデータになる。pow2 アンカーと
//!   `{2,3,5}`-smooth の同規模長を対で置く。
//! - **`ncc_plan`**: `CorrelationPlan::new_ncc` がコストモデルで選んだ
//!   経路の E2E `ncc` 時間。Issue #265 の回帰形状（`tpl≈sig`）では
//!   `NccDirect`（axpy 直接カーネル）を選び、`fft_len` の対応する
//!   smooth/pow2 ペア計測（3 変換 ≈ 1.5× ペア）と比較して
//!   「旧 pow2 選択で direct だった形状が新選択で遅い FFT 化されない」
//!   ことを実測で担保する。
//!
//! Run: `cargo bench -p ruviform-numeric --bench corr`。大きな形状では
//! iteration 自体が ~100ms 超のため `measurement_time` を縮めている。

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ruviform_numeric::corr::{CorrelationPlan, CorrelationWorkspace};
use ruviform_numeric::fft::{RealFftPlan, RealFftWorkspace};
use std::hint::black_box;
use std::time::Duration;

/// `fft_cost_units` 校正用の代表変換長。pow2 アンカー（`*` 印）と
/// 同規模の混合基数長を対にして per-element プレミアムを観測する。
const FFT_LENS: &[usize] = &[
    // 8K 帯: smooth 候補 vs pow2
    8_192, 8_640, 8_748, 10_240, 16_384, //
    // 32K–128K 帯
    32_768, 34_992, 67_500, 69_120, 131_072, //
    // 256K–1M 帯（キャッシュ境界）
    262_144, 279_936, 552_960, 1_048_576, 1_250_000, //
    // 2M–4M 帯（Issue #265 回帰形状の変換長）
    2_097_152, 2_211_840, 2_359_296, 2_430_000, 2_457_600, //
    4_194_304, 4_251_528, 4_374_000,
];

/// `new_ncc` の mode 選択を計測する代表形状 `(template_len, signal_len)`。
/// 先頭 4 件は Issue #265 の direct 維持形状（実測で axpy 直接カーネルが
/// FFT 経路を 1.5–3x 上回る）、末尾 2 件は FFT 経路が正しい形状。
const NCC_SHAPES: &[(usize, usize)] = &[
    (2_400_000, 2_400_240),
    (2_205_000, 2_205_236),
    (551_250, 551_309),
    (100_000, 100_101),
    (8_192, 8_400),
    (4_410, 7_938),
];

fn bench_fft_len(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_len");
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(30);
    for &n in FFT_LENS {
        let plan = RealFftPlan::new(n).unwrap();
        let mut ws = RealFftWorkspace::new(&plan);
        let input = vec![0.5f64; n];
        let bins = n / 2 + 1;
        let mut fre = vec![0.0; bins];
        let mut fim = vec![0.0; bins];
        let mut out = vec![0.0; n];
        plan.forward(&mut ws, &input, &mut fre, &mut fim).unwrap();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                plan.forward(&mut ws, black_box(&input), &mut fre, &mut fim)
                    .unwrap();
                plan.inverse(&mut ws, black_box(&fre), black_box(&fim), &mut out)
                    .unwrap();
            });
        });
    }
    group.finish();
}

fn bench_ncc_plan(c: &mut Criterion) {
    let mut group = c.benchmark_group("ncc_plan");
    // 2M 級形状は 1 iteration が ~100ms 超なので短めに抑える。
    group.measurement_time(Duration::from_secs(4));
    group.sample_size(10);
    for &(l, s) in NCC_SHAPES {
        let plan = CorrelationPlan::new_ncc(l, s).unwrap();
        let mut ws = CorrelationWorkspace::new(&plan);
        let tpl = vec![0.37f64; l];
        let sig = vec![0.53f64; s];
        plan.ncc(&mut ws, &tpl, &sig, usize::MAX).unwrap();
        group.throughput(Throughput::Elements((l * (s - l + 1)) as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{l}/{s}")),
            &(l, s),
            |b, _| {
                b.iter(|| {
                    plan.ncc(&mut ws, black_box(&tpl), black_box(&sig), usize::MAX)
                        .unwrap();
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_fft_len, bench_ncc_plan);
criterion_main!(benches);
