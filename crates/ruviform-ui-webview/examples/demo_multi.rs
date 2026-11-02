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
//! contract via `ruviform-ui`'s fake-host tests instead.

use std::process::ExitCode;
use std::sync::Arc;

use ruviform_ui::{
    CloseDecision, DataRegistry, DemoService, HostEvent, MapAssetSource, UiRuntime, WindowConfig,
    WindowEvent, dispatch_script,
};
use ruviform_ui_webview::{SessionConfig, detect_backend, run_multi};

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
        windows: vec![
            WindowConfig {
                title: "ruviform demo_multi — main".to_string(),
                ..WindowConfig::default()
            },
            WindowConfig {
                title: "ruviform demo_multi — panel".to_string(),
                width: 800,
                height: 600,
                ..WindowConfig::default()
            },
        ],
        ..SessionConfig::single(&ruviform_ui_webview::HostConfig::default())
    };

    let result = run_multi(&config, assets, registry, |event, host| {
        let WindowEvent::Windowed { window, event } = event else {
            // Loop-global tick → drain routed job events.
            for out in rt.drain_jobs_routed() {
                match out.window {
                    Some(w) => host.eval(w, &dispatch_script(&out.event)),
                    // Unrouted (headless/legacy) jobs go to the first
                    // live window — the consumer's sink choice.
                    None => {
                        if let Some(&w) = host.windows().first() {
                            host.eval(w, &dispatch_script(&out.event));
                        }
                    }
                }
            }
            return;
        };

        // Consumer close policy: approve every close; exit only when
        // the last window is destroyed. A sub-window close never exits
        // the loop or cancels jobs — that policy lives here, not in
        // the host.
        if matches!(event, HostEvent::CloseRequested) {
            let _ = host.respond_close(window, CloseDecision::Approve);
            // `Destroyed` follows through the queue; last-window exit
            // is decided there.
        }
        if matches!(event, HostEvent::Destroyed) && host.windows().is_empty() {
            host.exit();
        }

        for out in rt.handle_window(window, event) {
            match out.window {
                Some(w) => host.eval(w, &dispatch_script(&out.event)),
                None => {
                    if let Some(&w) = host.windows().first() {
                        host.eval(w, &dispatch_script(&out.event));
                    }
                }
            }
        }
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("host exited with error: {e}");
            ExitCode::FAILURE
        }
    }
}

const DEMO_HTML: &str = include_str!("demo_app.html");
