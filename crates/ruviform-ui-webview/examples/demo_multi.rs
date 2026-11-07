//! `demo_multi` — the Issue #181 multi-window counterpart of
//! `demo_app`: one native event loop drives two windows/webviews
//! through [`run_multi`], with per-window event routing, a veto-able
//! close flow, and a consumer-owned last-window/app-exit policy.
//!
//! Run: `cargo run -p ruviform-ui-webview --features webview --example demo_multi`
//!
//! Manual regression surface:
//! - two windows share one `rvf://app` page; each issues its own
//!   WebView-local correlation ids (collisions are expected — replies
//!   must still reach the issuing window);
//! - `demo.sleep` started in one window keeps running after the other
//!   window closes (job lifetime ≠ window lifetime);
//! - closing a window shows the `CloseRequested` → approve →
//!   `Destroyed` flow; the loop only exits once every window is gone
//!   (the consumer's policy — here, last-window-wins).
//!
//! **Not automated** — a real OS webview backend is required
//! (`WebView2`/`WKWebView`/`WebKitGTK`); CI verifies the headless
//! contract via `ruviform-ui`'s fake-driver tests instead.

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
        windows: vec![
            WebViewWindowConfig {
                spec: WindowSpec {
                    title: "ruviform demo_multi — main".to_string(),
                    ..WindowSpec::default()
                },
                ..WebViewWindowConfig::default()
            },
            WebViewWindowConfig {
                spec: WindowSpec {
                    title: "ruviform demo_multi — panel".to_string(),
                    logical_size: ruviform_ui::LogicalSize::new(800, 600),
                    ..WindowSpec::default()
                },
                ..WebViewWindowConfig::default()
            },
        ],
        default_url: "rvf://app/index.html".to_string(),
        devtools: cfg!(debug_assertions),
    };

    let result = run_multi(&config, assets, registry, |event, host| match event {
        DriverEvent::Tick => route(host, rt.drain_jobs_routed()),
        DriverEvent::Windowed { window, event } => match event {
            WindowLifecycle::Action(intent) => {
                route(host, rt.dispatch_window_intent(window, intent));
            }
            // Consumer close policy: approve every close and announce
            // `closing` to that window. A sub-window close never exits
            // the loop or cancels jobs — that policy lives here, not in
            // the adapter.
            WindowLifecycle::CloseRequested => {
                host.post_event(window, &Event::lifecycle(LifecycleState::Closing));
                let _ = host.respond_close(window, CloseDecision::Approve);
            }
            // Last-window exit: `Destroyed` follows the approved close.
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

/// Delivers routed events to their window; unrouted ones (headless or
/// ownerless jobs) go to the first live window — the consumer's sink
/// choice.
fn route(host: &mut dyn WindowDriver, outs: Vec<RoutedEvent>) {
    for out in outs {
        let target = out.window.or_else(|| host.windows().first().copied());
        if let Some(w) = target {
            host.post_event(w, &out.event);
        }
    }
}

const DEMO_HTML: &str = include_str!("demo_app.html");
