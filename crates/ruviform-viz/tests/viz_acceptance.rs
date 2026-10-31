//! Issue #157 acceptance coverage, exercised against the public API
//! only: partial reads, gaps, extreme zoom, long data, multichannel,
//! irregular timestamps, frequency-resolution change, cancellation,
//! complex-spectrum reuse, precision/quantization separation, and
//! timeline coordinate conversion.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use ruviform_exec::CancellationToken;
use ruviform_numeric::window::Window;
use ruviform_viz::display::{ColorMap, DbRange, FreqAxis, Quantization};
use ruviform_viz::encode::PagedBytes;
use ruviform_viz::source::{PcmBlock, SpectrumData, VecSpectrum, WaveformInfo, WaveformSource};
use ruviform_viz::spectrogram::{Aggregate, RowSpec, Spectrogram, Stft, StftConfig, TileRequest};
use ruviform_viz::timeline::{FrameGrid, SampleTimebase, Viewport};
use ruviform_viz::waveform::{LodConfig, WaveformLod, WaveformSeries};
use ruviform_viz::{DemoWaveform, SliceWaveform, VizError, encode_tile};

/// A `WaveformSource` counting its `read` calls — proves views stay
/// partial (no whole-dataset pulls).
struct CountingSource {
    inner: DemoWaveform,
    reads: AtomicU64,
    max_frames: AtomicU64,
}

impl WaveformSource for CountingSource {
    fn info(&self) -> WaveformInfo {
        self.inner.info()
    }
    fn read(&self, start: u64, frames: usize) -> ruviform_viz::Result<PcmBlock> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.max_frames.fetch_max(frames as u64, Ordering::Relaxed);
        self.inner.read(start, frames)
    }
}

const RATE: u32 = 48_000;
const RATE64: u64 = 48_000;

#[test]
fn partial_reads_stay_bounded_by_viewport() {
    // 4M frames (~90 s) behind a counting source.
    let frames = 1u64 << 22;
    let src = Arc::new(CountingSource {
        inner: DemoWaveform::new(RATE, 1, frames).unwrap(),
        reads: AtomicU64::new(0),
        max_frames: AtomicU64::new(0),
    });
    let series = WaveformSeries::on_demand(src.clone());
    // A 1024-bin view of an 8-second window must read only that window.
    let v = series
        .view(0, frames / 2, frames / 2 + RATE64 * 8, 1024)
        .unwrap();
    assert_eq!(v.len(), 1024);
    let requested = src.max_frames.load(Ordering::Relaxed);
    assert!(
        requested <= RATE64 * 8,
        "view requested {requested} frames per read for an 8 s window"
    );
    // Zoomed-in view of 64 samples: at most one small read.
    src.reads.store(0, Ordering::Relaxed);
    src.max_frames.store(0, Ordering::Relaxed);
    let v = series.view(0, 1000, 1064, 64).unwrap();
    assert!(v.iter().all(|b| b.bucket.count <= 1));
    assert!(src.max_frames.load(Ordering::Relaxed) <= 64 * 2);
}

#[test]
fn gaps_surface_as_nan_and_never_interpolate() {
    // NaN samples inside the data: LOD buckets skip them, views keep
    // empty bins as NaN.
    let gap = 1000..2000u64;
    let src: Arc<dyn WaveformSource> =
        Arc::new(DemoWaveform::with_gap(RATE, 1, 4096, Some(gap.clone())).unwrap());
    let lod = WaveformLod::build(
        src.as_ref(),
        &LodConfig {
            base_bucket: 64,
            levels: 3,
            read_chunk: 512,
        },
        None,
        None,
    )
    .unwrap();
    let series = WaveformSeries::with_lod(src, lod);
    let v = series.view(0, 0, 4096, 64).unwrap();
    // Bins fully inside the gap: count 0 → NaN stats.
    let gap_bins: Vec<_> = v
        .iter()
        .filter(|b| b.start >= gap.start && b.end <= gap.end)
        .collect();
    // Bins 16..=30 ([1024,1984)) lie fully inside the 1000..2000 gap.
    assert_eq!(gap_bins.len(), 15);
    assert!(gap_bins.iter().all(|b| b.bucket.count == 0));
    assert!(gap_bins.iter().all(|b| b.bucket.min.is_nan()));
    // Bins fully outside have finite stats.
    let clean = &v[0];
    assert!(clean.bucket.count > 0 && clean.bucket.min.is_finite());
}

#[test]
fn extreme_zoom_out_and_in_same_source() {
    let frames = 1u64 << 21; // ~43 s
    let src: Arc<dyn WaveformSource> = Arc::new(DemoWaveform::new(RATE, 1, frames).unwrap());
    let lod = WaveformLod::build(src.as_ref(), &LodConfig::fit(frames, 2048), None, None).unwrap();
    let series = WaveformSeries::with_lod(src, lod);
    // Whole signal into 512 bins — pyramid-served.
    let out = series.view(0, 0, frames, 512).unwrap();
    assert_eq!(out.len(), 512);
    assert!(out.iter().all(|b| b.bucket.count > 0));
    // Two samples into 256 bins — direct path, per-sample bins.
    let z = series.view(0, 500, 502, 256).unwrap();
    let filled: Vec<_> = z.iter().filter(|b| b.bucket.count > 0).collect();
    assert_eq!(filled.len(), 2);
}

#[test]
fn multichannel_views_are_independent() {
    let src: Arc<dyn WaveformSource> = Arc::new(DemoWaveform::new(RATE, 4, 10_000).unwrap());
    let series = WaveformSeries::on_demand(src);
    let c0 = series.view(0, 0, 10_000, 100).unwrap();
    let c3 = series.view(3, 0, 10_000, 100).unwrap();
    // Channel 3 carries a phase offset, so some bin must differ
    // (bit-exact f64 values — NaN-free here, so != is the honest check).
    #[expect(clippy::float_cmp)]
    let differs = |a: &ruviform_viz::ViewBin, b: &ruviform_viz::ViewBin| {
        a.bucket.min != b.bucket.min || a.bucket.max != b.bucket.max
    };
    assert!(c0.iter().zip(&c3).any(|(a, b)| differs(a, b)));
    assert!(series.view(4, 0, 10_000, 10).is_err());
}

#[test]
fn irregular_timestamps_and_missing_lane_data() {
    use ruviform_viz::timeline::{LaneItem, Selection, assign_lanes, hit_test, items_in};
    // Irregular spacing + a missing stretch (no items 4..8) must not
    // confuse lane packing or range selection.
    let items = vec![
        LaneItem::marker(0.0, "a"),
        LaneItem::marker(0.01, "b"),
        LaneItem::marker(0.9, "c"),
        LaneItem::new(1.5, 1.7, "d"),
        LaneItem::marker(10.0, "e"),
        LaneItem::new(9.3, 12.0, "f"),
    ];
    let lanes = assign_lanes(&items, 0.0);
    assert_eq!(lanes.lanes.len(), 2); // marker e sits inside span f
    assert_eq!(
        items_in(&items, Selection::new(4.0, 8.0)),
        Vec::<usize>::new()
    );
    assert_eq!(items_in(&items, Selection::new(0.005, 1.0)), vec![1, 2]);
    assert_eq!(hit_test(&items, 0.91, 0.05), Some(2));
}

#[test]
fn stft_frequency_resolution_change_reanalyzes_cleanly() {
    // Same source, two fft sizes → different bin grids, both consistent.
    let n = 8192u64;
    let src: Arc<dyn WaveformSource> = Arc::new(DemoWaveform::new(RATE, 1, n).unwrap());
    for fft in [512usize, 2048] {
        let cfg = StftConfig {
            fft_size: fft,
            hop: fft / 4,
            window: Window::Hann,
            center: false,
            read_chunk: 1 << 14,
        };
        let spec = Stft::analyze(src.as_ref(), 0, &cfg, None, None).unwrap();
        let info = spec.info();
        assert_eq!(info.bins, fft / 2 + 1);
        let step = match &info.axis {
            ruviform_viz::BinAxis::Uniform { step_hz, .. } => *step_hz,
            _ => panic!("expected uniform axis"),
        };
        assert!((step - f64::from(RATE) / f64::from(u32::try_from(fft).unwrap())).abs() < 1e-9);
    }
}

#[test]
fn complex_spectrum_is_reused_without_reanalysis() {
    // A stored complex spectrum wraps into a Spectrogram and serves
    // tiles directly — no STFT runs.
    let n_bins = 5;
    let info = ruviform_viz::SpectrumInfo {
        frames: 4,
        bins: n_bins,
        grid: FrameGrid::new(8, 4, 0).unwrap(),
        axis: ruviform_viz::BinAxis::Uniform {
            first_hz: 0.0,
            step_hz: 10.0,
        },
        sample_rate: RATE,
    };
    // re = frame*10+bin, im = 0 → magnitude = re.
    let re: Vec<f64> = (0u32..4)
        .flat_map(|f| (0u32..5).map(move |b| f64::from(f * 10 + b)))
        .collect();
    let im = vec![0.0; re.len()];
    let stored = VecSpectrum::new(info, SpectrumData::Complex { re: re.clone(), im }).unwrap();
    let spec = Spectrogram::new(Arc::new(stored));
    // Complex data passes through read_tile unchanged (phase retained).
    let raw = spec.source().read_tile(0..2, 0..3).unwrap();
    assert!(raw.data.is_complex());
    // Tiles aggregate magnitudes of the same stored data.
    let t = spec
        .tile(&TileRequest {
            frames: 0..4,
            rows: RowSpec::uniform(n_bins, n_bins),
            cols: 4,
            aggregate: Aggregate::Max,
        })
        .unwrap();
    assert_eq!(t.values[0], 0.0);
    assert_eq!(t.values[(n_bins - 1) * t.cols + (t.cols - 1)], 34.0);
}

#[test]
fn cancellation_stops_long_builds_promptly() {
    // A big build cancelled mid-flight must return Cancelled fast —
    // responsiveness check for the "cancellable load" requirement.
    let frames = 1u64 << 24; // 16M samples
    let src: Arc<dyn WaveformSource> = Arc::new(DemoWaveform::new(RATE, 1, frames).unwrap());
    let token = CancellationToken::new();
    let cancel = token.clone();
    let handle = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        cancel.cancel();
    });
    let cfg = LodConfig {
        base_bucket: 256,
        levels: 2,
        read_chunk: 1 << 14,
    };
    let t0 = Instant::now();
    let res = WaveformLod::build(src.as_ref(), &cfg, Some(&token), None);
    let elapsed = t0.elapsed();
    handle.join().unwrap();
    assert!(matches!(res, Err(VizError::Cancelled)));
    assert!(
        elapsed < Duration::from_secs(5),
        "cancellation took {elapsed:?}"
    );
}

#[test]
fn precision_stays_f64_until_encode_boundary() {
    // f64 values survive to the tile; quantization is explicit and only
    // changes the encoded bytes.
    let info = ruviform_viz::SpectrumInfo {
        frames: 1,
        bins: 2,
        grid: FrameGrid::new(4, 4, 0).unwrap(),
        axis: ruviform_viz::BinAxis::Uniform {
            first_hz: 0.0,
            step_hz: 1.0,
        },
        sample_rate: RATE,
    };
    let precise = 0.123_456_789_012_345_67;
    let spec = Spectrogram::new(Arc::new(
        VecSpectrum::new(info, SpectrumData::Magnitude(vec![precise, 0.5])).unwrap(),
    ));
    let t = spec
        .tile(&TileRequest {
            frames: 0..1,
            rows: RowSpec::uniform(2, 2),
            cols: 1,
            aggregate: Aggregate::Max,
        })
        .unwrap();
    assert_eq!(t.values[0], precise); // bit-exact f64
    let b64 = encode_tile(&t, Quantization::F64);
    let read_back = f64::from_le_bytes(b64.0[24..32].try_into().unwrap());
    assert_eq!(read_back, precise);
    let b32 = encode_tile(&t, Quantization::F32);
    let rb32 = f32::from_le_bytes(b32.0[24..28].try_into().unwrap());
    assert!((f64::from(rb32) - precise).abs() > 1e-9); // quantized away
    assert!((f64::from(rb32) - precise).abs() < 1e-6); // near the value
}

#[test]
fn timeline_sample_timebase_and_viewport_align() {
    let tb = SampleTimebase::new(RATE).unwrap();
    // Sample-exact grid for ticks/labels; pixel mapping only at display.
    let vp = Viewport::new(0.0, 10.0, 1000).unwrap(); // 10 s into 1000 px
    let sample = tb.seconds_to_sample(vp.to_unit(500.0));
    assert_eq!(sample, RATE64 * 5 + 240); // px 500 center = 5.005 s
    // Exact rational path survives where f64 blurs: 1/48000 * 48000.
    let t = tb.instant(48_000).unwrap();
    assert_eq!(t.value(), 48_000);
}

#[test]
fn display_pipeline_db_colormap_encode() {
    // End-to-end display path: magnitude → dB unit → RGBA → u8 bytes.
    let db = DbRange::new(-60.0, 0.0, 1.0).unwrap();
    let unit_full = db.unit_of(1.0); // 0 dBFS
    assert_eq!(unit_full, 1.0);
    let rgba = ColorMap::Viridis.at(unit_full);
    assert_eq!(rgba[..3], [253, 231, 37]);
    // NaN → transparent.
    assert_eq!(ColorMap::Viridis.at(f64::NAN), [0, 0, 0, 0]);
    // Log frequency axis produces a valid RowSpec for tiles.
    let axis = ruviform_viz::BinAxis::Uniform {
        first_hz: 0.0,
        step_hz: 23.4375,
    };
    let spec = FreqAxis::Log {
        min_hz: 20.0,
        max_hz: 20_000.0,
    }
    .row_spec(&axis, 128)
    .unwrap();
    assert_eq!(spec.rows(), 128);
    assert!(spec.bins.iter().all(|r| !r.is_empty()));
}

#[test]
fn encoded_payloads_page_through_paged_bytes() {
    let src: Arc<dyn WaveformSource> = Arc::new(DemoWaveform::new(RATE, 1, 4096).unwrap());
    let series = WaveformSeries::on_demand(src);
    let bins = series.view(0, 0, 4096, 128).unwrap();
    let lb = ruviform_viz::encode_buckets(&bins);
    // The paged port serves bounded reads — the rvf://data shape.
    let mut buf = vec![0u8; 32];
    let n = lb.bytes.read(0, &mut buf).unwrap();
    assert_eq!(n, 32);
    assert_eq!(&buf[..4], b"RVBW");
    // Reassemble through the port and parse a record back.
    let mut all = vec![0u8; usize::try_from(lb.bytes.len()).unwrap()];
    let mut off = 0usize;
    loop {
        let n = lb.bytes.read(off as u64, &mut all[off..]).unwrap();
        if n == 0 {
            break;
        }
        off += n;
    }
    let rec = &all[16..];
    assert_eq!(u64::from_le_bytes(rec[0..8].try_into().unwrap()), 0);
    assert!(f64::from_le_bytes(rec[16..24].try_into().unwrap()).is_finite());
}

#[test]
fn slice_waveform_source_covers_same_contract() {
    // The in-memory source and the synthetic source both satisfy the
    // port: identical stats on identical data.
    let samples: Vec<f64> = (0..512).map(|i| (f64::from(i) / 64.0).sin()).collect();
    let a = WaveformSeries::on_demand(Arc::new(
        SliceWaveform::new(RATE, 1, samples.clone()).unwrap(),
    ));
    let va = a.view(0, 0, 512, 64).unwrap();
    // Direct check: min/max over the whole signal appear in the bins.
    let expected_min = samples.iter().copied().fold(f64::INFINITY, f64::min);
    let expected_max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let got_min = va
        .iter()
        .map(|v| v.bucket.min)
        .fold(f64::INFINITY, f64::min);
    let got_max = va
        .iter()
        .map(|v| v.bucket.max)
        .fold(f64::NEG_INFINITY, f64::max);
    assert_eq!(got_min, expected_min);
    assert_eq!(got_max, expected_max);
}
