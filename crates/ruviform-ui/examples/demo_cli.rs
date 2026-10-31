//! `demo_cli` — headless driver for the shared `DemoService`.
//!
//! Proves the Issue #156 contract that the *same* non-UI computation
//! runs from a GUI and a CLI: it dispatches intents through
//! [`UiRuntime::dispatch_op`] — the identical routing a webview
//! `HostEvent::Message` takes — then polls job events on
//! [`UiRuntime::poll`], reads the published peaks through the
//! `DataRegistry` pull path and prints a compact report.
//!
//! Usage:
//!   `demo_cli` [seconds]   (default 5)
//!
//! Exit codes: 0 = job finished, 1 = usage/IO/job failure,
//! 2 = unexpected protocol shape.

use std::sync::Arc;
use std::time::{Duration, Instant};

use ruviform_ui::{
    DataRegistry, DataRequest, DemoService, Event, JobState, UiRuntime, VecDataSource,
};

fn main() {
    let seconds: u64 = match std::env::args().nth(1).as_deref() {
        None => 5,
        Some("cancel") => {
            std::process::exit(cancel_demo());
        }
        Some(arg) => match arg.parse() {
            Ok(s) if (1..=600).contains(&s) => s,
            _ => {
                eprintln!("usage: demo_cli [seconds|cancel]");
                std::process::exit(1);
            }
        },
    };

    let registry = DataRegistry::new();
    let mut rt = UiRuntime::new(registry.clone());
    rt.register_service(DemoService::new())
        .expect("demo service registration");

    // Seed a pre-registered source to exercise the pull path too.
    registry.register(
        "demo/seed",
        Arc::new(VecDataSource::new(b"seed-payload".to_vec())),
    );

    for ev in rt.dispatch_op("cap-1", "sys.capabilities", serde_json::json!({})) {
        if let Event::Result { ok: true, data, .. } = ev {
            let ops = data
                .as_ref()
                .and_then(|d| d.get("ops"))
                .map_or(0, |o| o.as_array().map_or(0, Vec::len));
            println!("capabilities: schema={} ops={ops}", data.unwrap()["schema"]);
        }
    }

    let started = Instant::now();
    for ev in rt.dispatch_op(
        "job-1",
        "demo.synthesize",
        serde_json::json!({ "seconds": seconds }),
    ) {
        match ev {
            Event::Result {
                ok: true,
                data: Some(d),
                ..
            } => println!("job started: {}", d["job"]),
            Event::Result { error: Some(e), .. } => {
                eprintln!("dispatch failed: {e}");
                std::process::exit(1);
            }
            _ => std::process::exit(2),
        }
    }

    while rt.has_running_jobs() {
        for ev in rt.poll(Some(Duration::from_millis(100))) {
            if let Event::Job {
                job,
                state,
                done,
                total,
                ..
            } = ev
            {
                match state {
                    JobState::Progress => println!(
                        "job {job} progress {}/{}",
                        done.unwrap_or(0),
                        total.unwrap_or(0)
                    ),
                    JobState::Finished => {
                        println!("job {job} finished in {:.2?}", started.elapsed());
                        report_pull(&registry);
                        std::process::exit(0);
                    }
                    JobState::Cancelled => {
                        eprintln!("job {job} cancelled");
                        std::process::exit(1);
                    }
                    JobState::Failed => {
                        eprintln!("job {job} failed");
                        std::process::exit(1);
                    }
                    _ => {}
                }
            }
        }
    }
    std::process::exit(2);
}

/// Reads the published peaks source in bounded windows — the headless
/// analogue of the frontend's `rvf://data` pull loop.
fn report_pull(registry: &DataRegistry) {
    let first = registry.read(&DataRequest {
        name: "demo/peaks".to_string(),
        offset: 0,
        len: Some(1024),
    });
    match first {
        Ok(chunk) => {
            let mut pulled = 0u64;
            let mut offset = 0u64;
            loop {
                let c = registry.read(&DataRequest {
                    name: "demo/peaks".to_string(),
                    offset,
                    len: Some(64 * 1024),
                });
                match c {
                    Ok(c) => {
                        if c.bytes.is_empty() {
                            break;
                        }
                        pulled += c.bytes.len() as u64;
                        offset += c.bytes.len() as u64;
                        if offset >= c.total {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("pull failed: {e}");
                        break;
                    }
                }
            }
            println!(
                "pull: total={}B pulled={pulled}B chunk_cap={}B",
                chunk.total,
                registry.max_chunk()
            );
        }
        Err(e) => {
            eprintln!("peaks source missing: {e}");
        }
    }
}

/// Cancellation path: start a long sleep job, cancel it, expect the
/// `cancelled` terminal.
fn cancel_demo() -> i32 {
    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(DemoService::new())
        .expect("demo service registration");
    let job = rt
        .dispatch_op("j", "demo.sleep", serde_json::json!({ "ms": 60_000 }))
        .into_iter()
        .find_map(|ev| match ev {
            Event::Result {
                ok: true,
                data: Some(d),
                ..
            } => d["job"].as_u64(),
            _ => None,
        })
        .unwrap_or_else(|| {
            eprintln!("job dispatch failed");
            std::process::exit(1);
        });
    for ev in rt.dispatch_op("c", "job.cancel", serde_json::json!({ "job": job })) {
        if !matches!(ev, Event::Result { ok: true, .. }) {
            eprintln!("cancel rejected: {}", ev.to_json());
            return 1;
        }
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        for ev in rt.poll(Some(Duration::from_millis(50))) {
            if matches!(
                ev,
                Event::Job {
                    state: JobState::Cancelled,
                    ..
                }
            ) {
                println!("job {job} cancelled as requested");
                return 0;
            }
        }
    }
    eprintln!("cancel did not terminate the job in time");
    1
}
