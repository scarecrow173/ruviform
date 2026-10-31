//! `demo_app` — minimal real-webview app on the `ruviform-ui` host
//! boundary. It drives the same [`DemoService`] as `demo_cli`
//! (headless) — the Issue #156 "same non-UI computation from GUI and
//! CLI" contract — with the difference that intents arrive through the
//! webview's `window.ipc.postMessage` instead of `dispatch_op`.
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

use ruviform_ui::{DataRegistry, DemoService, MapAssetSource, UiRuntime, dispatch_script};
use ruviform_ui_webview::{HostConfig, detect_backend, run};

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

    let config = HostConfig {
        title: "ruviform demo_app".to_string(),
        ..HostConfig::default()
    };

    let result = run(&config, assets, registry, |event, host| {
        for out in rt.handle(event) {
            host.eval(&dispatch_script(&out));
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
