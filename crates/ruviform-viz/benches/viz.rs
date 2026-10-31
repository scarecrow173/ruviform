//! `ruviform-viz` benchmarks (Issue #157) — chunked LOD build,
//! extreme-zoom view decimation, STFT analysis, tile serving, encode.
//!
//! Inputs are the synthetic `DemoWaveform` (no I/O, deterministic),
//! so the numbers measure the viz pipeline itself: read-path cost,
//! decimation, FFT throughput, and encode size. `cargo bench -p
//! ruviform-viz -- --test` runs the bench-smoke lane.

use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ruviform_numeric::window::Window;
use ruviform_viz::display::Quantization;
use ruviform_viz::source::WaveformSource;
use ruviform_viz::spectrogram::{Aggregate, RowSpec, Stft, StftConfig, TileRequest};
use ruviform_viz::waveform::{LodConfig, WaveformLod, WaveformSeries};
use ruviform_viz::{DemoWaveform, encode_tile};

const RATE: u32 = 48_000;

fn source(frames: u64) -> Arc<dyn WaveformSource> {
    Arc::new(DemoWaveform::new(RATE, 2, frames).unwrap())
}

fn lod_build(c: &mut Criterion) {
    let mut g = c.benchmark_group("viz/lod_build");
    for frames in [1u64 << 20, 1 << 22] {
        let src = source(frames);
        let cfg = LodConfig::fit(frames, 4096);
        g.bench_with_input(BenchmarkId::new("frames", frames), &src, |b, src| {
            b.iter(|| WaveformLod::build(src.as_ref(), &cfg, None, None).unwrap());
        });
    }
    g.finish();
}

fn view_zoom(c: &mut Criterion) {
    // 4M frames (~90 s @ 48 kHz): zoom-out over the whole file is served
    // by the pyramid; zoom-in reads the source directly.
    let frames = 1u64 << 22;
    let src = source(frames);
    let lod = WaveformLod::build(src.as_ref(), &LodConfig::fit(frames, 4096), None, None).unwrap();
    let series = WaveformSeries::with_lod(src, lod);
    let mut g = c.benchmark_group("viz/view");
    g.bench_function("zoom_out_1024px", |b| {
        b.iter(|| series.view(0, 0, frames, 1024).unwrap());
    });
    g.bench_function("zoom_in_8s_1024px", |b| {
        b.iter(|| {
            series
                .view(0, frames / 2, frames / 2 + u64::from(RATE) * 8, 1024)
                .unwrap()
        });
    });
    g.bench_function("zoom_extreme_256samples", |b| {
        b.iter(|| series.view(0, 1000, 1000 + 256, 256).unwrap());
    });
    g.finish();
}

fn stft_analyze(c: &mut Criterion) {
    let mut g = c.benchmark_group("viz/stft");
    let src = source(u64::from(RATE) * 30); // 30 s
    for fft in [1024usize, 4096] {
        let cfg = StftConfig {
            fft_size: fft,
            hop: fft / 2,
            window: Window::Hann,
            center: false,
            read_chunk: 1 << 16,
        };
        g.bench_with_input(BenchmarkId::new("fft", fft), &cfg, |b, cfg| {
            b.iter(|| Stft::analyze(src.as_ref(), 0, cfg, None, None).unwrap());
        });
    }
    g.finish();
}

fn tile_serve(c: &mut Criterion) {
    let src = source(u64::from(RATE) * 30);
    let spec = Stft::analyze(
        src.as_ref(),
        0,
        &StftConfig {
            fft_size: 2048,
            hop: 1024,
            window: Window::Hann,
            center: false,
            read_chunk: 1 << 16,
        },
        None,
        None,
    )
    .unwrap();
    let frames = spec.info().frames;
    let mut g = c.benchmark_group("viz/tile");
    for cols in [512usize, 2048] {
        let req = TileRequest {
            frames: 0..frames,
            rows: RowSpec::uniform(1025, 256),
            cols,
            aggregate: Aggregate::Max,
        };
        g.bench_with_input(BenchmarkId::new("cols", cols), &req, |b, req| {
            b.iter(|| spec.tile(req).unwrap());
        });
    }
    g.finish();
}

fn encode(c: &mut Criterion) {
    let src = source(u64::from(RATE) * 30);
    let spec = Stft::analyze(
        src.as_ref(),
        0,
        &StftConfig {
            fft_size: 2048,
            hop: 1024,
            window: Window::Hann,
            center: false,
            read_chunk: 1 << 16,
        },
        None,
        None,
    )
    .unwrap();
    let tile = spec
        .tile(&TileRequest {
            frames: 0..spec.info().frames,
            rows: RowSpec::uniform(1025, 256),
            cols: 512,
            aggregate: Aggregate::Max,
        })
        .unwrap();
    let mut g = c.benchmark_group("viz/encode_tile");
    for q in [Quantization::F64, Quantization::F32, Quantization::U8] {
        g.bench_with_input(BenchmarkId::new("dtype", q.bytes()), &q, |b, q| {
            b.iter(|| encode_tile(&tile, *q));
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    lod_build,
    view_zoom,
    stft_analyze,
    tile_serve,
    encode
);
criterion_main!(benches);
