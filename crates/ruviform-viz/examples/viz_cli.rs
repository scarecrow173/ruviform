//! `viz_cli` — headless CLI over `ruviform-viz` (Issue #157 acceptance:
//! the same analysis data is usable without any GUI).
//!
//! Synthesizes a deterministic [`DemoWaveform`] (the source port needs
//! no pre-rendered data), then prints:
//!
//! - `waveform` — a LOD pyramid summary and two views (full-zoom-out
//!   and an extreme zoom-in), showing bounded partial reads.
//! - `spectrogram` — an STFT analysis, a viewport-shaped tile, and the
//!   same values after display mapping (dB → color) and encoding.
//! - `timeline` — coordinate conversions, lane packing and selection.
//!
//! Usage: `viz_cli [waveform|spectrogram|timeline|all]` (default `all`).

use std::sync::Arc;

use ruviform_numeric::window::Window;
use ruviform_viz::display::{ColorMap, DbRange, FreqAxis, Quantization};
use ruviform_viz::encode::PagedBytes;
use ruviform_viz::source::WaveformSource;
use ruviform_viz::spectrogram::{Aggregate, Stft, StftConfig, TileRequest};
use ruviform_viz::timeline::{
    LaneItem, SampleTimebase, Selection, Viewport, assign_lanes, items_in,
};
use ruviform_viz::waveform::{LodConfig, WaveformLod, WaveformSeries};
use ruviform_viz::{DemoWaveform, encode_buckets, encode_tile};

const RATE: u32 = 48_000;
const SECONDS: u64 = 60;

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());
    match mode.as_str() {
        "waveform" => waveform(),
        "spectrogram" => spectrogram(),
        "timeline" => timeline(),
        "all" => {
            waveform();
            spectrogram();
            timeline();
        }
        other => {
            eprintln!("unknown mode `{other}` (waveform|spectrogram|timeline|all)");
            std::process::exit(2);
        }
    }
}

fn waveform() {
    println!("== waveform ==");
    let frames = u64::from(RATE) * SECONDS;
    let src: Arc<dyn WaveformSource> =
        Arc::new(DemoWaveform::with_gap(RATE, 2, frames, Some(1_400_000..1_450_000)).unwrap());
    let lod = WaveformLod::build(src.as_ref(), &LodConfig::fit(frames, 4096), None, None).unwrap();
    println!(
        "lod: {} levels, base {} samples/bucket, {} channels",
        lod.levels().len(),
        lod.levels()[0].samples_per_bucket,
        src.info().channels
    );
    let series = WaveformSeries::with_lod(src, lod);

    // Full zoom-out: whole minute into 96 terminal columns.
    let view = series.view(0, 0, frames, 96).unwrap();
    let filled = view.iter().filter(|b| b.bucket.count > 0).count();
    let peak = view.iter().map(|b| b.bucket.peak()).fold(0.0f64, f64::max);
    let empty = view.len() - filled;
    println!("zoom-out 96 bins: {filled} filled, {empty} empty (gap), peak {peak:.4}");

    // Extreme zoom-in: 512 samples into 64 bins, direct source reads.
    let z = series.view(0, 1_000, 1_512, 64).unwrap();
    let zf = z.iter().filter(|b| b.bucket.count > 0).count();
    println!("zoom-in 512 samples -> 64 bins: {zf} populated bins");

    // Encoded transfer size for the same view.
    let packed = encode_buckets(&view);
    println!("encoded RVBW: {} bytes via PagedBytes", packed.bytes.len());
}

fn spectrogram() {
    println!("== spectrogram ==");
    let frames = u64::from(RATE) * 10;
    let src: Arc<dyn WaveformSource> = Arc::new(DemoWaveform::new(RATE, 1, frames).unwrap());
    let cfg = StftConfig {
        fft_size: 2048,
        hop: 1024,
        window: Window::Hann,
        center: true,
        read_chunk: 1 << 16,
    };
    let spec = Stft::analyze(src.as_ref(), 0, &cfg, None, None).unwrap();
    let info = spec.info();
    println!(
        "stft: {} frames x {} bins ({} Hz/bin), complex bins retained",
        info.frames,
        info.bins,
        match &info.axis {
            ruviform_viz::BinAxis::Uniform { step_hz, .. } => *step_hz,
            _ => 0.0,
        }
    );

    // Log-frequency tile: 20 Hz–20 kHz into 96 rows x 160 cols.
    let rows = FreqAxis::Log {
        min_hz: 20.0,
        max_hz: 20_000.0,
    }
    .row_spec(&info.axis, 96)
    .unwrap();
    let tile = spec
        .tile(&TileRequest {
            frames: 0..info.frames,
            rows,
            cols: 160,
            aggregate: Aggregate::Max,
        })
        .unwrap();
    let peak = tile
        .values
        .iter()
        .filter(|v| v.is_finite())
        .fold(0.0f64, |a, &v| a.max(v));
    println!("tile {}x{}: peak magnitude {peak:.4}", tile.cols, tile.rows);

    // Display boundary: dB window → unit → RGBA (data stays f64).
    let db = DbRange::new(-80.0, 0.0, 1.0).unwrap();
    let map = ColorMap::Viridis;
    let rgba = map.at(db.unit_of(peak));
    println!("display: peak -> {rgba:?} (viridis, -80..0 dBFS)");

    // Encoded sizes per quantization.
    let units: Vec<f64> = tile.values.iter().map(|&v| db.unit_of(v)).collect();
    let unit_tile = ruviform_viz::spectrogram::Tile {
        values: units,
        ..tile.clone()
    };
    for q in [Quantization::F64, Quantization::F32, Quantization::U8] {
        let b = encode_tile(&unit_tile, q);
        println!("encoded RVST {:?}: {} bytes", q, b.len());
    }
}

fn timeline() {
    println!("== timeline ==");
    let tb = SampleTimebase::new(RATE).unwrap();
    // Exact sample-domain labels; f64 only at display.
    for s in [0u64, 48_000, 2_400_000] {
        println!("sample {s}: {:.6} s", tb.sample_to_seconds(s));
    }
    let vp = Viewport::new(0.0, 60.0, 1200).unwrap();
    let marker_px = vp.to_px(tb.sample_to_seconds(48_000));
    println!("viewport 60s/1200px: 1 s mark at px {marker_px:.1}");

    // Generic annotation lanes (payloads are opaque to viz).
    let items = vec![
        LaneItem::new(0.0, 10.0, "take A"),
        LaneItem::new(5.0, 15.0, "take B"),
        LaneItem::marker(20.0, "marker"),
        LaneItem::new(30.0, 45.0, "take C"),
    ];
    let lanes = assign_lanes(&items, 0.0);
    println!("lanes: {} for {} items", lanes.lanes.len(), items.len());
    let sel = Selection::new(8.0, 12.0);
    println!(
        "selection {sel:?} overlaps items {:?}",
        items_in(&items, sel)
    );
}
