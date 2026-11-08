//! Issue #463 — Spectrogram cell intensity real-window fixture.
//!
//! Run: `cargo run -p ruviform-ui-gpui --features gpui --example
//! spectrogram_intensity`
//!
//! Opens a GPUI window projecting a `SpectrogramSpec` whose intensity
//! forms a horizontal gradient (dark → `viz.waveform` blue) while
//! neighbouring columns legally share frame slots — the
//! `ruviform-viz` `Tile::columns` contract when `cols > frames`, the
//! same shape `AudioLens` AL#179 rendered as an all-blue canvas before
//! this fix.
//!
//! Expected on a real OS session: the canvas shows a smooth
//! dark→bright gradient (intermediate intensities = intermediate
//! blended colours), NOT a uniform saturated blue. Capture the window
//! and check that multiple distinct cell colours appear.
//!
//! Manual-verification fixture — headless coverage lives in
//! `render::tests::spectrogram_cell_color_is_opaque_intensity_composite`.

use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    A11yRole, A11ySpec, AttachEpochAllocator, ComponentKey, ComponentSpec, ComponentVisualState,
    PanelSpec, SpectrogramColumn, SpectrogramSpec, ViewHost, ViewSpec,
};
use ruviform_ui::{DriverEvent, LogicalPosition, LogicalSize, WindowLifecycle, WindowSpec};
use ruviform_ui_gpui::{GpuiDriver, ViewMount};

fn key(name: &str) -> ComponentKey {
    ComponentKey(name.to_string())
}

/// Mirrors the `AudioLens` AL#179 shape: 512 columns over 7 viewport
/// frames × 128 rows = `MAX_SPECTROGRAM_CELLS`. Every frame slot is
/// shared by ~73 columns (the `cols > frames` overlap the tile
/// contract allows). Column `i` paints intensity `i / 512`, so the
/// visible stripes run dark → bright even though ~73 cells overlap
/// per x position.
fn intensity_spec() -> ViewSpec {
    const COLUMNS: u32 = 512;
    const FRAMES: u64 = 7;
    const ROWS: usize = 128;
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: key("root"),
            title: Some("RF463 spectrogram intensity".to_string()),
            children: vec![ComponentSpec::Spectrogram(SpectrogramSpec {
                key: key("spectrogram"),
                viewport: 0..FRAMES,
                rows: ROWS,
                magnitude_range: 0.0..1.0,
                columns: (0..COLUMNS)
                    .map(|i| {
                        // Contiguous runs: ~73 columns share each of
                        // the 7 one-frame slots, like `Tile::columns`.
                        let slot = u64::from(i) * FRAMES / u64::from(COLUMNS);
                        SpectrogramColumn {
                            start: slot,
                            end: slot + 1,
                            count: ROWS as u64,
                            magnitudes: vec![f64::from(i) / f64::from(COLUMNS); ROWS],
                        }
                    })
                    .collect(),
                visual: ComponentVisualState::default(),
                a11y: A11ySpec {
                    role: Some(A11yRole::Img),
                    label: Some("Spectrogram intensity gradient".to_string()),
                    ..A11ySpec::default()
                },
            })],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

fn main() {
    let mut driver = GpuiDriver::new();
    let mut epochs = AttachEpochAllocator::new();
    let mut host = ViewHost::new(ViewInstanceId::new(1).expect("nonzero"), epochs.mint());
    host.mount(intensity_spec()).expect("mount spectrogram");
    let window = driver
        .create_view_window(
            &WindowSpec {
                title: "RF463 spectrogram intensity".to_string(),
                logical_size: LogicalSize {
                    width: 960,
                    height: 440,
                },
                position: Some(LogicalPosition::new(80, 80)),
                resizable: false,
            },
            ViewMount { host, theme: None },
        )
        .expect("create_view_window");
    driver
        .run(None, move |event, _driver| {
            if let DriverEvent::Windowed {
                window: w,
                event: WindowLifecycle::Action(intent),
            } = event
                && w == window
            {
                eprintln!("[intent] op={} params={}", intent.op, intent.params);
            }
        })
        .expect("gpui driver run");
}
