//! `demo_app` — minimal real-webview app on the driver-neutral
//! `ruviform-ui` host boundary. It drives the same [`DemoService`] as
//! `demo_cli` (headless) — the Issue #156 "same non-UI computation from
//! GUI and CLI" contract — with the difference that intents arrive
//! through the webview's `window.ipc.postMessage` as typed
//! `WindowLifecycle::Action`s instead of `dispatch_op`.
//!
//! Run: `cargo run -p ruviform-ui-webview --features webview --example demo_app`
//!
//! Manual regression surface (Issue #156 verification):
//! - `demo.synthesize` → progress bar → finished → peaks pulled via
//!   `rvf://data` ranges → `Canvas2D` min/max rendering.
//! - `demo.sleep` + `job.cancel` from the UI.
//! - `demo.fail` / `demo.panic` error surfaces.
//! - focus/blur, resize, DPI (move window across monitors), close.
//! - devtools: debug builds only (`cfg!(debug_assertions)`).

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use ruviform_ui::{
    CloseDecision, DataRegistry, DemoService, DriverEvent, Event, LifecycleState, MapAssetSource,
    RoutedEvent, UiRuntime, WindowDriver, WindowLifecycle, WindowSpec,
};
use ruviform_ui_webview::{SessionConfig, WebViewWindowConfig, detect_backend, run_multi};

fn main() -> ExitCode {
    let report = detect_backend();
    eprintln!(
        "backend: {} state={:?} version={:?}",
        report.name, report.state, report.version
    );
    for note in &report.notes {
        eprintln!("  note: {note}");
    }
    if !ruviform_ui_webview::webview_available() {
        eprintln!("webview backend unavailable — install the runtime above and retry");
        return ExitCode::from(2);
    }

    let assets: Arc<dyn ruviform_ui::AssetSource> = Arc::new(MapAssetSource::new().with_asset(
        "index.html",
        None,
        DEMO_HTML.as_bytes().to_vec(),
    ));
    let registry = DataRegistry::new();
    let mut rt = UiRuntime::new(registry.clone());
    rt.register_service(DemoService::new())
        .expect("demo service registration");

    let config = SessionConfig {
        data_dir: None,
        tick: Some(Duration::from_millis(16)),
        windows: vec![WebViewWindowConfig {
            spec: WindowSpec {
                title: "ruviform demo_app".to_string(),
                ..WindowSpec::default()
            },
            ..WebViewWindowConfig::default()
        }],
        default_url: "rvf://app/index.html".to_string(),
        devtools: cfg!(debug_assertions),
    };

    let result = run_multi(&config, assets, registry, |event, host| match event {
        DriverEvent::Tick => route(host, rt.drain_jobs_routed()),
        DriverEvent::Windowed { window, event } => match event {
            WindowLifecycle::Action(intent) => {
                route(host, rt.dispatch_window_intent(window, intent));
            }
            WindowLifecycle::CloseRequested => {
                host.post_event(window, &Event::lifecycle(LifecycleState::Closing));
                // Single window: approve; the loop exits on `Destroyed`.
                let _ = host.respond_close(window, CloseDecision::Approve);
            }
            WindowLifecycle::Destroyed if host.windows().is_empty() => host.exit(),
            _ => {}
        },
        _ => {}
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("host exited with error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Delivers routed events; unrouted ones go to the first live window —
/// the consumer's sink choice.
fn route(host: &mut dyn WindowDriver, outs: Vec<RoutedEvent>) {
    for out in outs {
        let target = out.window.or_else(|| host.windows().first().copied());
        if let Some(w) = target {
            host.post_event(w, &out.event);
        }
    }
}

const DEMO_HTML: &str = include_str!("demo_app.html");
