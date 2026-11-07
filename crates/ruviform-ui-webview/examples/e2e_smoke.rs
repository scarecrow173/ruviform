//! `e2e_smoke` — automated real-WebView round trip (Issue #431).
//!
//! Opens one window on the real OS webview backend and checks four
//! milestones: `Created` (window + webview built), `Ready` (page load
//! finished), `ping` intent (page → Rust IPC), and `received` intent
//! (the page resolved the `ping` call from a result event pushed back
//! by Rust — Rust → page delivery). `Created` → `ping` → `received`
//! must occur in that order, with `Ready` after `Created`. Exits 0 only
//! when all of that holds; exits 1 on timeout or a missing/out-of-order
//! milestone; exits 2 when no webview backend is available.
//!
//! Run: `cargo run -p ruviform-ui-webview --features webview --example e2e_smoke`
//!
//! Needs a real OS webview (`WebView2` on Windows). Not a CI lane.

use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ruviform_ui::{
    CloseDecision, DataRegistry, DemoService, DriverEvent, MapAssetSource, RoutedEvent, UiRuntime,
    WindowDriver, WindowLifecycle, WindowSpec,
};
use ruviform_ui_webview::{SessionConfig, WebViewWindowConfig, detect_backend, run_multi};

const TIMEOUT: Duration = Duration::from_secs(30);
const PROBE_HTML: &str = include_str!("e2e_smoke.html");

fn main() -> ExitCode {
    let report = detect_backend();
    eprintln!(
        "[e2e] backend: {} state={:?} version={:?}",
        report.name, report.state, report.version
    );
    if !ruviform_ui_webview::webview_available() {
        eprintln!("[e2e] FAIL: webview backend unavailable");
        return ExitCode::from(2);
    }

    let assets: Arc<dyn ruviform_ui::AssetSource> = Arc::new(MapAssetSource::new().with_asset(
        "index.html",
        None,
        PROBE_HTML.as_bytes().to_vec(),
    ));
    let registry = DataRegistry::new();
    let mut rt = UiRuntime::new(registry.clone());
    rt.register_service(DemoService::new())
        .expect("demo service registration");

    let config = SessionConfig {
        data_dir: None,
        tick: Some(Duration::from_millis(50)),
        windows: vec![WebViewWindowConfig {
            spec: WindowSpec {
                title: "ruviform e2e_smoke".to_string(),
                ..WindowSpec::default()
            },
            devtools: false,
            ..WebViewWindowConfig::default()
        }],
        default_url: "rvf://app/index.html".to_string(),
        devtools: false,
    };

    let started = Instant::now();
    let mut seen: Vec<&'static str> = Vec::new();
    let mut verdict: Option<bool> = None;
    let result = run_multi(&config, assets, registry, |event, host| match event {
        DriverEvent::Tick if started.elapsed() > TIMEOUT => {
            eprintln!("[e2e] timeout after {TIMEOUT:?}; milestones={seen:?}");
            verdict = Some(false);
            host.exit();
        }
        DriverEvent::Tick => route(host, rt.drain_jobs_routed()),
        DriverEvent::Windowed { window, event } => match event {
            WindowLifecycle::Created => milestone(&mut seen, "Created"),
            WindowLifecycle::Ready => milestone(&mut seen, "Ready"),
            WindowLifecycle::Action(intent) => {
                let probe = intent.params["probe"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                match probe.as_str() {
                    "ping" => milestone(&mut seen, "ipc:ping"),
                    "received" => {
                        milestone(&mut seen, "ipc:received");
                        verdict = Some(in_order(&seen));
                        host.exit();
                        return;
                    }
                    _ => {}
                }
                route(host, rt.dispatch_window_intent(window, intent));
            }
            WindowLifecycle::CloseRequested => {
                let _ = host.respond_close(window, CloseDecision::Approve);
            }
            _ => {}
        },
        _ => {}
    });

    let pass = matches!(verdict, Some(true)) && result.is_ok();
    if pass {
        eprintln!("[e2e] PASS milestones={seen:?}");
        ExitCode::SUCCESS
    } else {
        eprintln!("[e2e] FAIL milestones={seen:?} result={result:?}");
        ExitCode::FAILURE
    }
}

fn milestone(seen: &mut Vec<&'static str>, name: &'static str) {
    eprintln!("[e2e] {name}");
    seen.push(name);
}

/// Every milestone was seen, `Created` < `ipc:ping` < `ipc:received`,
/// and `Ready` came after `Created`.
fn in_order(seen: &[&str]) -> bool {
    let at = |m: &str| seen.iter().position(|s| *s == m);
    match (
        at("Created"),
        at("Ready"),
        at("ipc:ping"),
        at("ipc:received"),
    ) {
        (Some(c), Some(r), Some(p), Some(x)) => c < r && c < p && p < x,
        _ => false,
    }
}

/// Delivers routed events; unrouted ones go to the first live window.
fn route(host: &mut dyn WindowDriver, outs: Vec<RoutedEvent>) {
    for out in outs {
        let target = out.window.or_else(|| host.windows().first().copied());
        if let Some(w) = target {
            host.post_event(w, &out.event);
        }
    }
}
