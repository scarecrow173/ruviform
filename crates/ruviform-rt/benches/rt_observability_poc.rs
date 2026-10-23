//! RT observability `PoC` benchmark harness (Issue #104, task T1).
//!
//! Measures the plan's measurement matrix — one `BenchmarkGroup` per
//! matrix row, one `bench_function` per column:
//!
//! - **`counter_equal_work`** — `disabled` (same arithmetic with no
//!   instrumentation), `flag/off` / `flag/on` (runtime-flag-gated
//!   instrumentation, gate closed / open), `always` (unconditional
//!   `fetch_add`). The enabled/disabled evaluation is a run-internal
//!   ratio across the variant columns, never a single absolute value.
//! - **`rt_bridge`** — `record` on a non-full ring / `record` on a full
//!   ring (drop + count path) / `drain` at empty·half·full occupancy /
//!   `overflows` reads.
//! - **`overflow_policy_nonzero`** — `drop` / `count` / `aggregate` on the same
//!   full-ring scenario, plus the `retire_channel` leak-on-full
//!   baseline column.
//! - **`rt_primitives`** — baseline (uninstrumented) cost of the
//!   existing RT primitives: `channel` `try_push` / `try_pop`,
//!   `retire` / `overflows`, `MasterClock::tick` / `publish_tick`,
//!   `slot` publish / read. These columns are the denominator for the
//!   counter-instrumentation ratio and the instrumentation headroom
//!   check on existing APIs.
//! - **`timestamp`** — `Instant::now()` / `Instant` → epoch-relative
//!   nanos → `i64` → record (`instant_to_media_time`-shaped surrogate)
//!   / `SystemTime::now()` as a contrast column.
//!
//! Nanosecond-scale operations sit at criterion's resolution limit, so
//! every cell batches `BATCH` operations per timed iteration with
//! `black_box` suppression and reports `Throughput::Elements(BATCH)`.
//! Cells whose measured path needs a prepared (pre-filled or empty)
//! fixture use `iter_batched_ref` + `BatchSize::NumIterations` so
//! fixture allocation/deallocation stays outside the measured window
//! (test-spec "測定窓の純粋性" — the same rule the module's
//! self-consistency test enforces on the scaffolding paths).
//!
//! - **ids**: `BenchmarkId::new(<column>, <variant>)` — criterion full
//!   ids read e.g. `rt_bridge/record/full`, `rt_primitives/slot/read`.
//! - **time budget**: reduced `warm_up_time` / `measurement_time` /
//!   `sample_size` on every group, same budget shape as
//!   `ruviform-math`'s `simd_poc`.
//! - **filter**: `cargo bench -p ruviform-rt -- <filter>` (e.g.
//!   `counter`, `rt_bridge`, `slot`) selects matching benchmarks — the
//!   quick-run convention.
//!
//! Run-start environment metadata (OS / CPU model / core count /
//! rustc / criterion / target crate version) is printed to stdout
//! exactly once, for transcription into the experiment log.

use std::hint::black_box;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Once};
use std::time::{Duration, Instant, SystemTime};

use criterion::{
    BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
    measurement::WallTime,
};
use ruviform_rt::observability_poc::{
    ObsRecord, RecordSink, RtRecorder, bridge, counter, instant_to_epoch_nanos, timestamped_record,
};
use ruviform_rt::{MasterClock, channel, retire_channel, slot};

/// Operations per timed iteration for nanosecond-scale cells — the
/// batch repetition that keeps single-digit-ns operations above
/// criterion's measurement resolution.
const BATCH: usize = 512;

/// Routine calls per `iter_batched_ref` input batch — bounds the
/// number of live fixtures while keeping fixture setup outside the
/// measured window.
const FIXTURE_BATCH: u64 = 64;

/// Ring capacity for the "full" scenarios (record/overflow policy).
const FULL_CAP: usize = 64;

/// Fixed record for record-path cells — content is irrelevant to cost.
const REC: ObsRecord = ObsRecord::new(1, 0, 0);

/// A non-zero payload prevents aggregation from measuring `fetch_add(0)`.
const POLICY_REC: ObsRecord = ObsRecord::new(1, 0, 1);

/// Creates the per-group `BenchmarkGroup` with the reduced `PoC` time
/// budget — same shape as `ruviform-math`'s `simd_poc` harness.
fn poc_group<'a>(c: &'a mut Criterion, name: &str) -> BenchmarkGroup<'a, WallTime> {
    let mut group = c.benchmark_group(name);
    group
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(1))
        .sample_size(20);
    group
}

/// Prints the run-start environment block to stdout exactly once; the
/// experiment log transcribes it.
fn print_env_metadata() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        println!("=== rt_observability_poc bench environment ===");
        println!("os: {} {}", std::env::consts::OS, std::env::consts::ARCH);
        println!("cpu: {}", cpu_model());
        println!("cores: {}", core_count());
        println!("rustc: {}", rustc_version());
        println!("criterion: {}", criterion_version());
        println!("ruviform-rt: {}", ruviform_rt::version());
    });
}

/// CPU model name for the environment block (brand string when available).
#[cfg(target_os = "windows")]
fn cpu_model() -> String {
    // `HKLM\...\CentralProcessor\0\ProcessorNameString` holds the brand
    // string ("Intel(R) Core(TM) …"); `reg` is a system tool on Windows.
    if let Ok(out) = std::process::Command::new("reg")
        .args([
            "query",
            r"HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0",
            "/v",
            "ProcessorNameString",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if line.contains("ProcessorNameString")
                && let Some((_, name)) = line.split_once("REG_SZ")
            {
                return name.trim().to_string();
            }
        }
    }
    // Fallback: family/model/stepping, not the marketing name.
    std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_string())
}

/// CPU model name for the environment block — `/proc/cpuinfo` `model name`.
#[cfg(target_os = "linux")]
fn cpu_model() -> String {
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in cpuinfo.lines() {
            if let Some((key, value)) = line.split_once(':')
                && key.trim() == "model name"
            {
                return value.trim().to_string();
            }
        }
    }
    "unknown".to_string()
}

/// CPU model name for the environment block — `sysctl` brand string.
#[cfg(target_os = "macos")]
fn cpu_model() -> String {
    if let Ok(out) = std::process::Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        && out.status.success()
    {
        return String::from_utf8_lossy(&out.stdout).trim().to_string();
    }
    "unknown".to_string()
}

/// CPU model name fallback on other targets.
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn cpu_model() -> String {
    "unknown".to_string()
}

/// Logical core count for the environment block.
fn core_count() -> String {
    std::thread::available_parallelism()
        .map_or_else(|_| "unknown".to_string(), |n| n.get().to_string())
}

/// `rustc --version` of the toolchain on `PATH` (the `mise`-pinned one
/// when run via `mise exec` / `mise run`).
fn rustc_version() -> String {
    std::process::Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string()))
        .arg("--version")
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |out| String::from_utf8_lossy(&out.stdout).trim().to_string(),
        )
}

/// criterion version actually linked — read from the workspace
/// `Cargo.lock` (the reproducibility anchor) rather than the
/// `Cargo.toml` requirement.
fn criterion_version() -> String {
    let lock = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let Ok(text) = std::fs::read_to_string(lock) else {
        return "unknown".to_string();
    };
    let mut found = false;
    for line in text.lines().map(str::trim) {
        if found && let Some(rest) = line.strip_prefix("version = ") {
            return rest.trim_matches('"').to_string();
        }
        found = line == "name = \"criterion\"";
    }
    "unknown".to_string()
}

/// A `bridge` prefilled to `fill` records (`fill <= capacity`).
fn prefilled_bridge(
    capacity: usize,
    fill: usize,
) -> (RtRecorder<ObsRecord>, RecordSink<ObsRecord>) {
    let (mut recorder, sink) = bridge::<ObsRecord>(capacity);
    for _ in 0..fill {
        recorder.record(REC);
    }
    (recorder, sink)
}

/// `counter` group — instrumentation cost of a prepared shared
/// counter, evaluated as run-internal ratios across variant columns.
fn bench_counter(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "counter_equal_work");
    group.throughput(Throughput::Elements(BATCH as u64));

    // Baseline: the same arithmetic with no instrumentation — a plain
    // non-atomic add on local state.
    group.bench_function("disabled", |b| {
        let mut value = 0_usize;
        b.iter(|| {
            for _ in 0..BATCH {
                value = black_box(value).wrapping_add(1);
            }
        });
        black_box(value);
    });

    // Runtime-flag-gated instrumentation: the flag check always runs;
    // `flag_off` never increments, `flag_on` always does.
    for (variant, flag_value) in [("off", false), ("on", true)] {
        group.bench_function(BenchmarkId::new("flag", variant), |b| {
            let flag = AtomicBool::new(flag_value);
            let (counter, _reader) = counter();
            let mut value = 0_usize;
            b.iter(|| {
                for _ in 0..BATCH {
                    value = black_box(value).wrapping_add(1);
                    if black_box(&flag).load(Ordering::Relaxed) {
                        counter.increment();
                    }
                }
            });
            black_box(value);
        });
    }

    // Unconditional instrumentation — one `Relaxed` `fetch_add` per op.
    group.bench_function("always", |b| {
        let (counter, _reader) = counter();
        let mut value = 0_usize;
        b.iter(|| {
            for _ in 0..BATCH {
                value = black_box(value).wrapping_add(1);
                counter.increment();
            }
        });
        black_box(value);
    });
    group.finish();
}

/// `rt_bridge` group — the record handoff path and its observation
/// accessors.
fn bench_rt_bridge(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "rt_bridge");

    // `record` on a non-full ring — fresh empty bridge per routine
    // call, fixture setup outside the timed window.
    group.throughput(Throughput::Elements(BATCH as u64));
    group.bench_function(BenchmarkId::new("record", "not_full"), |b| {
        b.iter_batched_ref(
            || prefilled_bridge(BATCH, 0),
            |(recorder, _sink)| {
                for _ in 0..BATCH {
                    recorder.record(REC);
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });

    // `record` on a full ring — every call takes the drop + count path.
    group.bench_function(BenchmarkId::new("record", "full"), |b| {
        b.iter_batched_ref(
            || prefilled_bridge(FULL_CAP, FULL_CAP),
            |(recorder, _sink)| {
                for _ in 0..BATCH {
                    recorder.record(REC);
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });

    // `drain` on an empty ring — the call itself is O(1), so the
    // routine batches `BATCH` calls; throughput counts drain calls.
    group.bench_function(BenchmarkId::new("drain", "empty"), |b| {
        b.iter_batched_ref(
            || prefilled_bridge(BATCH, 0),
            |(_recorder, sink)| {
                for _ in 0..BATCH {
                    black_box(sink.drain(|rec| {
                        black_box(rec);
                    }));
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });

    // `drain` at half / full occupancy — one bounded call per routine;
    // throughput counts the prefilled records the call consumes.
    for (variant, fill) in [("half", BATCH / 2), ("full", BATCH)] {
        group.throughput(Throughput::Elements(fill as u64));
        group.bench_function(BenchmarkId::new("drain", variant), |b| {
            b.iter_batched_ref(
                || prefilled_bridge(BATCH, fill),
                |(_recorder, sink)| {
                    black_box(sink.drain(|rec| {
                        black_box(rec);
                    }));
                },
                BatchSize::NumIterations(FIXTURE_BATCH),
            );
        });
    }

    // `overflows` accessor — a `Relaxed` load per call.
    group.throughput(Throughput::Elements(BATCH as u64));
    group.bench_function(BenchmarkId::new("overflows", "read"), |b| {
        let (recorder, _sink) = bridge::<ObsRecord>(4);
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(recorder.overflows());
            }
        });
    });
    group.finish();
}

/// `overflow_policy` group — drop / count / aggregate variants on the
/// same full-ring scenario plus the `retire_channel` leak-on-full
/// baseline.
fn bench_overflow_policy(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "overflow_policy_nonzero");
    group.throughput(Throughput::Elements(BATCH as u64));

    group.bench_function("drop", |b| {
        b.iter_batched_ref(
            || prefilled_bridge(FULL_CAP, FULL_CAP),
            |(recorder, _sink)| {
                for _ in 0..BATCH {
                    recorder.record_drop(black_box(POLICY_REC));
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });
    group.bench_function("count", |b| {
        b.iter_batched_ref(
            || prefilled_bridge(FULL_CAP, FULL_CAP),
            |(recorder, _sink)| {
                for _ in 0..BATCH {
                    recorder.record(black_box(POLICY_REC));
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });
    group.bench_function("aggregate", |b| {
        b.iter_batched_ref(
            || prefilled_bridge(FULL_CAP, FULL_CAP),
            |(recorder, _sink)| {
                for _ in 0..BATCH {
                    recorder.record_aggregate(black_box(POLICY_REC), ObsRecord::value);
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });

    // Baseline column: the existing `retire_channel` leak-on-full path
    // (full channel → `mem::forget` + shared counter increment).
    group.bench_function("retire_leak", |b| {
        b.iter_batched_ref(
            || {
                let (mut retirer, collector) = retire_channel::<u64>(FULL_CAP);
                for _ in 0..FULL_CAP {
                    retirer.retire(0);
                }
                (retirer, collector)
            },
            |(retirer, _collector)| {
                for _ in 0..BATCH {
                    retirer.retire(black_box(9));
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });
    group.finish();
}

/// `rt_primitives` group — uninstrumented baseline costs of the
/// existing RT primitives.
fn bench_rt_primitives(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "rt_primitives");
    group.throughput(Throughput::Elements(BATCH as u64));

    group.bench_function(BenchmarkId::new("channel", "try_push"), |b| {
        b.iter_batched_ref(
            || channel::<u64>(BATCH),
            |(producer, _consumer)| {
                for _ in 0..BATCH {
                    let _ = black_box(producer.try_push(black_box(7)));
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });
    group.bench_function(BenchmarkId::new("channel", "try_pop"), |b| {
        b.iter_batched_ref(
            || {
                let (mut producer, consumer) = channel::<u64>(BATCH);
                for _ in 0..BATCH {
                    let _ = producer.try_push(7);
                }
                (producer, consumer)
            },
            |(_producer, consumer)| {
                for _ in 0..BATCH {
                    let _ = black_box(consumer.try_pop());
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });

    // `retire` on a non-full channel — the enqueue path.
    group.bench_function(BenchmarkId::new("retire", "retire"), |b| {
        b.iter_batched_ref(
            || retire_channel::<u64>(BATCH),
            |(retirer, _collector)| {
                for _ in 0..BATCH {
                    retirer.retire(black_box(7));
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });
    group.bench_function(BenchmarkId::new("retire", "overflows"), |b| {
        let (retirer, _collector) = retire_channel::<u64>(4);
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(retirer.overflows());
            }
        });
    });

    group.bench_function(BenchmarkId::new("clock", "tick"), |b| {
        let clock = MasterClock::new(1, 48_000).unwrap();
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(clock.tick());
            }
        });
    });
    group.bench_function(BenchmarkId::new("clock", "publish_tick"), |b| {
        let clock = MasterClock::new(1, 48_000).unwrap();
        b.iter(|| {
            for _ in 0..BATCH {
                clock.publish_tick(black_box(7));
            }
        });
    });

    // `publish` — the non-RT-side swap + displaced-`Arc` handoff.
    group.bench_function(BenchmarkId::new("slot", "publish"), |b| {
        b.iter_batched_ref(
            || {
                let (writer, _reader) = slot(Arc::new(0_u32), 4);
                let snapshots: Vec<Arc<u32>> = (0..BATCH).map(|_| Arc::new(0_u32)).collect();
                (writer, snapshots)
            },
            |(writer, snapshots)| {
                while let Some(snapshot) = snapshots.pop() {
                    writer.publish(snapshot);
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });

    // `read` + guard drop — the guard's retire-channel push is part of
    // the measured path. The reader is warmed in setup (first-read
    // debt node) and the retire capacity is `BATCH + 1` so the warmed
    // guard plus every loop guard fits — nothing leaks.
    group.bench_function(BenchmarkId::new("slot", "read"), |b| {
        b.iter_batched_ref(
            || {
                let (writer, reader) = slot(Arc::new(0_u32), BATCH + 1);
                drop(reader.read()); // warm the per-thread debt node
                (writer, reader)
            },
            |(_writer, reader)| {
                for _ in 0..BATCH {
                    let guard = reader.read();
                    black_box(*guard);
                }
            },
            BatchSize::NumIterations(FIXTURE_BATCH),
        );
    });
    group.finish();
}

/// `timestamp` group — timestamp acquisition cost on this platform.
fn bench_timestamp(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "timestamp");
    group.throughput(Throughput::Elements(BATCH as u64));
    let epoch = Instant::now();

    group.bench_function("instant_now", |b| {
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(Instant::now());
            }
        });
    });
    group.bench_function("instant_to_epoch_nanos", |b| {
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(instant_to_epoch_nanos(Instant::now(), epoch));
            }
        });
    });
    group.bench_function("timestamped_record", |b| {
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(timestamped_record(1, 2, epoch));
            }
        });
    });
    // Contrast column: the wall-clock source an RT path must not use.
    group.bench_function("systemtime_now", |b| {
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(SystemTime::now());
            }
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_counter,
    bench_rt_bridge,
    bench_overflow_policy,
    bench_rt_primitives,
    bench_timestamp
);
criterion_main!(benches);
