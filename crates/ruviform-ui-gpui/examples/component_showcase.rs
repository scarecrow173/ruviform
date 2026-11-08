//! Issue #387 — Component/View renderer showcase.
//!
//! Run: `cargo run -p ruviform-ui-gpui --features gpui --example
//! component_showcase`
//!
//! Opens a GPUI window projecting the two vertical-slice `ViewSpec`s:
//! Synvelis-style Panel > Tabs > Faders, and an AudioLens-style
//! Waveform + Seek strip. Drag the faders / click-drag the seek to
//! see `Begin → Preview → Commit` intents on the window's `Action`
//! channel logged to stderr. Click a tab header to switch the Tabs
//! (`tab-select` intent).
//!
//! This is a manual-verification fixture — semantic equivalence is
//! proven headlessly in `ruviform-ui-test::component_view`; this
//! example exercises the real GPUI mount seam
//! (`ViewMount` + `create_view_window`). The example itself binds
//! only the neutral driver API — no `gpui` type appears here.

use ruviform_ui::behavior::{DetachPolicy, ValueDescriptor};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    A11yRole, A11ySpec, AttachEpochAllocator, ComponentKey, ComponentSpec, ComponentVisualState,
    ControlSpec, Orientation, PanelSpec, TabSpec, TabsSelectionSpec, TabsSpec, ViewHost, ViewSpec,
    WaveformBin, WaveformSpec,
};
use ruviform_ui::{DriverEvent, LogicalSize, WindowLifecycle, WindowSpec};
use ruviform_ui_gpui::{GpuiDriver, ViewMount};

fn key(name: &str) -> ComponentKey {
    ComponentKey(name.to_string())
}

fn a11y_slider(label: &str, o: Orientation) -> A11ySpec {
    A11ySpec {
        role: Some(A11yRole::Slider),
        label: Some(label.to_string()),
        orientation: Some(o),
        value_min: Some(0.0),
        value_max: Some(1.0),
        value_text: None,
    }
}

fn control(name: &str, label: &str, o: Orientation) -> ControlSpec {
    ControlSpec {
        key: key(name),
        control_id: None,
        target: Some(format!("mixer.{name}")),
        descriptor: ValueDescriptor {
            default_value: Some(0.5),
            step: Some(0.05),
            ..ValueDescriptor::ranged(0.0, 1.0)
        },
        orientation: o,
        detach_policy: DetachPolicy::Cancel,
        unsupported: false,
        visual: ComponentVisualState::default(),
        a11y: a11y_slider(label, o),
    }
}

fn showcase_spec() -> ViewSpec {
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: key("root"),
            title: Some("RF387 showcase".to_string()),
            children: vec![
                ComponentSpec::Tabs(TabsSpec {
                    key: key("tabs"),
                    active: 0,
                    tabs: vec![
                        TabSpec {
                            key: key("tab-a"),
                            title: "Ch 1".to_string(),
                            child: Box::new(ComponentSpec::Fader(control(
                                "fader-a",
                                "Ch 1",
                                Orientation::Vertical,
                            ))),
                        },
                        TabSpec {
                            key: key("tab-b"),
                            title: "Ch 2".to_string(),
                            child: Box::new(ComponentSpec::Fader(control(
                                "fader-b",
                                "Ch 2",
                                Orientation::Vertical,
                            ))),
                        },
                    ],
                    selection: Some(TabsSelectionSpec {
                        target: "showcase.tabs.select".to_string(),
                    }),
                    visual: ComponentVisualState::default(),
                    a11y: A11ySpec::default(),
                }),
                ComponentSpec::Waveform(WaveformSpec {
                    key: key("wave"),
                    sample_rate: 48_000,
                    channels: 1,
                    frames: 96_000,
                    viewport: 0..96_000,
                    channel_bins: vec![
                        (0..64u32)
                            .map(|i| {
                                let x = (f64::from(i) / 64.0 * std::f64::consts::TAU).sin();
                                WaveformBin {
                                    start: u64::from(i) * 1500,
                                    end: u64::from(i + 1) * 1500,
                                    min: x - 0.25,
                                    max: x + 0.25,
                                    rms: x.abs() * 0.5,
                                    count: 1500,
                                }
                            })
                            .collect(),
                    ],
                    visual: ComponentVisualState::default(),
                    a11y: A11ySpec {
                        role: Some(A11yRole::Img),
                        label: Some("Waveform".to_string()),
                        ..A11ySpec::default()
                    },
                }),
                ComponentSpec::Seek(control("seek", "Seek", Orientation::Horizontal)),
            ],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

fn main() {
    let mut driver = GpuiDriver::new();
    // The composition root owns the session's `AttachEpochAllocator`
    // — one allocator per session, minting per `ViewHost` instance.
    // (Never mint a second allocator for a fresh host: it restarts
    // the sequence and re-opens the stale-receipt collision.)
    let mut epochs = AttachEpochAllocator::new();
    let mut host = ViewHost::new(ViewInstanceId::new(1).expect("nonzero"), epochs.mint());
    host.mount(showcase_spec()).expect("mount showcase");
    let window = driver
        .create_view_window(
            &WindowSpec {
                title: "RF387 component showcase".to_string(),
                logical_size: LogicalSize {
                    width: 640,
                    height: 480,
                },
                position: None,
                resizable: true,
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
