//! Host-side sample for the `logging` feature — Issue #276 (OBS-01).
//!
//! Shows the ownership contract end to end:
//!
//! 1. The host maps *its own* configuration (env vars / CLI flags / GUI
//!    settings — none of which this adapter reads) into a `LoggingConfig`.
//! 2. `build_logging` returns a `Dispatch` plus writer guards, initial
//!    degraded info, and a health handle — **without installing anything
//!    globally**.
//! 3. The host decides how to use the dispatch: `set_global_default` once
//!    per process, or scoped `dispatch::with_default` (shown below, so the
//!    sample also works when a global default already exists).
//! 4. Non-RT code emits ordinary `tracing` events; nothing is written to
//!    stdout, so a structured stdout channel stays clean.
//! 5. On shutdown the host drops the guards, which drain and join each
//!    sink's background writer.
//!
//! Run with:
//!
//! ```sh
//! cargo run -p ruviform-observability --features logging --example logging_host
//! ```

use std::num::NonZeroUsize;

use ruviform_observability::logging::{
    LogFormat, LoggingConfig, RotationPeriod, SinkConfig, SinkDestination, SinkFailureMode,
    build_logging,
};

fn main() {
    // 1. Host-side configuration. A real host resolves this from its own
    //    env/CLI/GUI settings — e.g. "logging on = one stderr text sink +
    //    one JSON-lines daily rolling file kept for 7 generations".
    let config = LoggingConfig {
        sinks: vec![
            SinkConfig {
                destination: SinkDestination::Stderr,
                format: LogFormat::Text,
                filter: "info".to_owned(),
                failure_mode: SinkFailureMode::Required,
            },
            SinkConfig {
                destination: SinkDestination::RollingFile {
                    directory: std::env::temp_dir().join("ruviform-observability-example-logs"),
                    filename_prefix: "example".to_owned(),
                    rotation: RotationPeriod::Daily,
                    max_files: NonZeroUsize::new(7).unwrap(),
                    queue_capacity: NonZeroUsize::new(1024).unwrap(),
                },
                format: LogFormat::JsonLines,
                filter: "ruviform=debug,info".to_owned(),
                failure_mode: SinkFailureMode::BestEffort,
            },
        ],
    };

    // 2. Build — the adapter never installs a global subscriber.
    let result = match build_logging(&config) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("logging build failed: {error}");
            std::process::exit(1);
        }
    };

    // 3. Report any best-effort sinks that failed at build time — explicit,
    //    never silent.
    for degraded in &result.degraded {
        eprintln!(
            "logging: sink #{} ({}) degraded: {}",
            degraded.sink_index, degraded.kind, degraded.reason
        );
    }

    // 4. Emit events under the dispatch — scoped here so the sample never
    //    depends on global state. A long-running host would instead call
    //    `tracing::dispatcher::set_global_default(result.dispatch.clone())`
    //    once at startup.
    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!(target: "ruviform", answer = 42, "host started");
        tracing::debug!(target: "ruviform", "debug detail the stderr filter drops");
        tracing::warn!(target: "ruviform", "something worth noting");
    });

    // 5. Observe sink health — non-recursive, never logged through the
    //    pipeline itself.
    let snapshot = result.health.snapshot();
    for sink in &snapshot.sinks {
        eprintln!(
            "logging health: sink #{} ({}) state={} written={} dropped_queue_full={} \
             partial={} write_failures={} flush_failures={} disconnected={} abandoned={}",
            sink.sink_index,
            sink.kind,
            sink.state,
            sink.events_written,
            sink.dropped_queue_full,
            sink.partial_writes,
            sink.write_failures,
            sink.flush_failures,
            sink.channel_disconnected,
            sink.shutdown_abandoned,
        );
    }

    // 6. Drop guards to drain, flush, and join each sink's worker. Dropping
    //    `result` here shuts down every sink in order.
    drop(result);
    eprintln!("logging shutdown complete");
}
