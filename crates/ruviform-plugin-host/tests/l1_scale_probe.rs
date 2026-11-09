//! Issue #120 measurement probe — current-state evidence for the
//! shared-reaper/io-multiplexing re-trigger evaluation.
//!
//! Every test here is `#[ignore]`d: they are manual measurement
//! fixtures, never part of the `cargo test` gate. Run explicitly:
//!
//! ```text
//! cargo test -p ruviform-plugin-host --test l1_scale_probe -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Configuration is via `L1_PROBE_*` env vars (see each test). The
//! `PROBE`-prefixed stdout lines are the recorded evidence; an
//! external sampler reads the printed `pid` for host-process CPU /
//! thread counts (the driver/reader/stderr-drain threads live in
//! *this* process — one set of 3 per child).
//!
//! `mod l1` is `pub(crate)` — same `#[path]` graft as
//! `l1_proxy_e2e.rs` (see that file's header for the constraint).

#[allow(dead_code)]
#[path = "../src/l1/mod.rs"]
mod l1;

use std::ffi::OsString;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_plugin_core::wire;
use ruviform_plugin_core::{
    EventList, HostError, HostInput, HostOutput, LifecycleState, PluginDescriptor, ProcessConfig,
    ProcessResult,
};

use l1::diag::DiagRegistry;
use l1::{L1Child, L1Instance, SpawnSpec};

/// The descriptor `l1-test-worker` reports for `Scan` — identical
/// oracle to `l1_proxy_e2e.rs`.
const WORKER_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.l1-test-worker",
    name: "L1 Test Worker",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// Generous bound for every asynchronous wait in this file — the
/// measurements themselves are microseconds-to-milliseconds; a miss
/// means the machinery broke, not that it was slow.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

fn spec(args: &[&str]) -> SpawnSpec {
    let mut spec = SpawnSpec::new(
        WORKER_DESCRIPTOR,
        OsString::from(env!("CARGO_BIN_EXE_l1-test-worker")),
    );
    spec.args = args.iter().map(OsString::from).collect();
    spec
}

fn config() -> ProcessConfig {
    ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap()
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_secs(name: &str, default: u64) -> Duration {
    Duration::from_secs(
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(default),
    )
}

fn processing_instance(args: &[&str]) -> L1Instance {
    processing_instance_lb(args, 2)
}

/// `processing_instance` with an explicit `lookahead_blocks` — the
/// shm slot depth `D = lookahead + 1` sets the leg's in-flight
/// alternation bound, so deeper lookahead raises the per-instance
/// sustained block-rate ceiling the probe measures.
fn processing_instance_lb(args: &[&str], lookahead_blocks: u32) -> L1Instance {
    let mut spawn_spec = spec(args);
    spawn_spec.lookahead_blocks = lookahead_blocks;
    let mut instance = L1Instance::spawn(&spawn_spec, &DiagRegistry::new(), None).unwrap();
    instance.execute(wire::Command::Initialize).unwrap();
    instance
        .execute(wire::Command::Activate { config: config() })
        .unwrap();
    instance.execute(wire::Command::StartProcessing).unwrap();
    instance
}

/// One `process` call over mono planar views — same shape as the
/// `run_block` helper in `l1_proxy_e2e.rs`.
fn run_block(
    instance: &mut L1Instance,
    input: &[f32],
    output: &mut [f32],
) -> Result<ProcessResult, HostError> {
    let in_list = [input];
    let input_view = PlanarBufferView::new(&in_list).unwrap();
    let inputs = [HostInput(&input_view)];
    let mut out_list = [&mut output[..]];
    let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
    let mut outputs = [HostOutput(&mut output_view)];
    instance.process(
        &inputs,
        &mut outputs,
        EventList::empty(u32::try_from(input.len()).unwrap()),
        None,
    )
}

/// Polls `try_recv_notification` until a notice arrives — returns
/// the elapsed wait. Panics past `PROBE_TIMEOUT`.
fn wait_notice(instance: &L1Instance, since: Instant) -> Duration {
    loop {
        if instance.try_recv_notification().is_some() {
            return since.elapsed();
        }
        assert!(
            since.elapsed() < PROBE_TIMEOUT,
            "Failed notification never arrived"
        );
        thread::sleep(Duration::from_micros(20));
    }
}

/// Polls the lifecycle mirror until `Failed` — returns the elapsed
/// wait. Panics past `PROBE_TIMEOUT`.
fn wait_failed(instance: &L1Instance, since: Instant) -> Duration {
    loop {
        if instance.lifecycle_state() == LifecycleState::Failed {
            return since.elapsed();
        }
        assert!(
            since.elapsed() < PROBE_TIMEOUT,
            "instance never reached Failed"
        );
        thread::sleep(Duration::from_micros(20));
    }
}

/// Spawns `children` L1 children with `per_child` minted instances
/// each (the Bundled pool shape; `per_child == 1` is the Dedicated
/// shape). Returns the instances and the child `Arc`s — the extra
/// `Arc` clones must be dropped (or `teardown()`d) before the
/// last-referrer rule can fire, so callers own cleanup explicitly.
fn spawn_topology(
    children: usize,
    per_child: usize,
    args: &[&str],
) -> (Vec<L1Instance>, Vec<Arc<L1Child>>) {
    let spawn_spec = spec(args);
    let diag = DiagRegistry::new();
    let depth = spawn_spec.instance_depth().unwrap();
    let mut instances = Vec::new();
    let mut child_arcs = Vec::new();
    for c in 0..children {
        let t0 = Instant::now();
        let child = L1Child::spawn_scanned(&spawn_spec.view(), &diag).unwrap();
        println!(
            "PROBE spawn child={c} ms={:.3}",
            t0.elapsed().as_secs_f64() * 1e3
        );
        for i in 0..per_child {
            let t1 = Instant::now();
            let parts = child
                .create_instance(&spawn_spec, depth, &diag, None)
                .unwrap();
            println!(
                "PROBE mint child={c} instance={i} ms={:.3}",
                t1.elapsed().as_secs_f64() * 1e3
            );
            instances.push(L1Instance::from_parts(Arc::clone(&child), parts));
        }
        child_arcs.push(child);
    }
    (instances, child_arcs)
}

/// Scale probe: builds a child/instance topology, prints a READY
/// marker for an external CPU/thread sampler, idles, then tears
/// down. `L1_PROBE_CHILDREN` (def 1), `L1_PROBE_INSTANCES` (def 1 —
/// instances per child), `L1_PROBE_IDLE_SECS` (def 30),
/// `L1_PROBE_PRE_SECS` (def 3 — baseline sampling window before
/// spawn).
#[test]
#[ignore = "manual Issue #120 measurement — run explicitly"]
fn probe_scale_idle() {
    let children = env_usize("L1_PROBE_CHILDREN", 1);
    let per_child = env_usize("L1_PROBE_INSTANCES", 1);
    let pre = env_secs("L1_PROBE_PRE_SECS", 3);
    let idle = env_secs("L1_PROBE_IDLE_SECS", 30);

    println!(
        "PROBE phase=baseline pid={} children={children} instances_per_child={per_child} total_instances={}",
        std::process::id(),
        children * per_child
    );
    thread::sleep(pre);

    let (mut instances, child_arcs) = spawn_topology(children, per_child, &[]);
    println!(
        "PROBE phase=spawning_done children={children} total_instances={}",
        instances.len()
    );
    println!(
        "PROBE phase=idle pid={} children={children} instances_per_child={per_child} total_instances={} idle_secs={}",
        std::process::id(),
        instances.len(),
        idle.as_secs()
    );
    thread::sleep(idle);
    println!("PROBE phase=idle_done");

    // Graceful teardown: `destroy` reclaims each entry; the explicit
    // `teardown` covers the held `Arc` clones so no child leaks
    // past the test process.
    for instance in &mut instances {
        instance.destroy();
    }
    for child in &child_arcs {
        child.teardown();
    }
    drop(instances);
    drop(child_arcs);
    println!("PROBE phase=teardown_done");
}

/// Crash-detection latency probe — three convergence paths:
///
/// * `eof_kill`: healthy child, host `kill()` — stdout EOF reaches
///   the reader immediately *and* `try_wait` is the backstop.
/// * `eof_crash`: `--crash-after-msgs=6` — the child exits(3) on
///   receipt of the 6th message (the `SaveState` below), a real
///   crash with EOF present.
/// * `noeof_kill`: `--spawn-heir` — a stdio-inheriting grandchild
///   holds the pipes open after `kill()`, so EOF *never* arrives
///   and `try_wait`'s 200 µs poll is the sole detector (the
///   sub-poll-detection bound).
///
/// Message accounting (worker): `Scan`=1, `CreateInstance`=2,
/// `Initialize`=3, `Activate`=4, `StartProcessing`=5, the trigger
/// `SaveState`=6.
///
/// `L1_PROBE_REPS` (def 10). Prints per-path min/median/max µs to
/// the `Failed` mirror and to the `FailedNotice`.
#[test]
#[ignore = "manual Issue #120 measurement — run explicitly"]
fn probe_crash_latency() {
    let reps = env_usize("L1_PROBE_REPS", 10).max(1);

    // eof_kill / noeof_kill share the drive-to-Processing shape;
    // only the spawn args differ.
    for (name, args) in [
        ("eof_kill", Vec::new()),
        ("noeof_kill", vec!["--spawn-heir"]),
    ] {
        let mut failed_us = Vec::new();
        let mut notice_us = Vec::new();
        for _ in 0..reps {
            let instance = processing_instance(&args);
            let t0 = Instant::now();
            instance.kill();
            failed_us.push(wait_failed(&instance, t0).as_secs_f64() * 1e6);
            notice_us.push(wait_notice(&instance, t0).as_secs_f64() * 1e6);
        }
        failed_us.sort_by(f64::total_cmp);
        notice_us.sort_by(f64::total_cmp);
        println!(
            "PROBE crash_path={name} reps={reps} failed_us min={:.1} med={:.1} max={:.1} notice_us min={:.1} med={:.1} max={:.1}",
            failed_us[0],
            failed_us[reps / 2],
            failed_us[reps - 1],
            notice_us[0],
            notice_us[reps / 2],
            notice_us[reps - 1],
        );
    }

    // eof_crash: the child dies on receipt of message 6 — measure
    // from just before the triggering `execute` to its `Err(Ipc)`
    // return (the convergence wakes the pending waiter) and to the
    // `Failed` mirror.
    let mut exec_us = Vec::new();
    let mut failed_us = Vec::new();
    for _ in 0..reps {
        let mut instance = processing_instance(&["--crash-after-msgs=6"]);
        let t0 = Instant::now();
        let result = instance.execute(wire::Command::SaveState);
        exec_us.push(t0.elapsed().as_secs_f64() * 1e6);
        assert!(matches!(result, Err(HostError::Ipc)));
        failed_us.push(wait_failed(&instance, t0).as_secs_f64() * 1e6);
    }
    exec_us.sort_by(f64::total_cmp);
    failed_us.sort_by(f64::total_cmp);
    println!(
        "PROBE crash_path=eof_crash reps={reps} exec_err_us min={:.1} med={:.1} max={:.1} failed_us min={:.1} med={:.1} max={:.1}",
        exec_us[0],
        exec_us[reps / 2],
        exec_us[reps - 1],
        failed_us[0],
        failed_us[reps / 2],
        failed_us[reps - 1],
    );
}

/// Timer-granularity probe: what `recv_timeout(200µs)` actually
/// waits on this OS — the driver's crash-watch/command-drain cadence
/// is `DRIVER_POLL_INTERVAL`, and OS timer quantization decides the
/// *effective* poll rate (the `noeof_kill` latency floor already
/// hints it is far coarser than 200 µs on Windows).
///
/// `L1_PROBE_REPS` (def 300).
// 比率表示のみの計測のため usize→f64 キャストを許容する。
#[test]
#[ignore = "manual Issue #120 measurement — run explicitly"]
#[expect(clippy::cast_precision_loss)]
fn probe_recv_timeout_cadence() {
    let reps = env_usize("L1_PROBE_REPS", 300).max(1);
    let (_tx, rx) = std::sync::mpsc::channel::<()>();
    let mut elapsed = Vec::with_capacity(reps);
    for _ in 0..reps {
        let t = Instant::now();
        // Sender stays alive — every call pays the full timeout.
        let _ = rx.recv_timeout(Duration::from_micros(200));
        elapsed.push(t.elapsed().as_secs_f64() * 1e6);
    }
    elapsed.sort_by(f64::total_cmp);
    let avg = elapsed.iter().sum::<f64>() / elapsed.len() as f64;
    println!(
        "PROBE recv_timeout_cadence reps={reps} requested_us=200 avg_us={avg:.1} min_us={:.1} med_us={:.1} max_us={:.1}",
        elapsed[0],
        elapsed[reps / 2],
        elapsed[reps - 1],
    );
}

/// Command-plane throughput: `SaveState` round-trips through the
/// single driver + reader pair — the control-path rate a shared
/// reader must sustain per child. `L1_PROBE_REPS` (def 500).
// 比率表示のみの計測のため usize→f64 キャストを許容する
// （`issue255_perf.rs` と同じ規約）。
#[test]
#[ignore = "manual Issue #120 measurement — run explicitly"]
#[expect(clippy::cast_precision_loss)]
fn probe_command_rtt() {
    let reps = env_usize("L1_PROBE_REPS", 500).max(1);
    let mut instance = processing_instance(&[]);
    // Warm-up — first round-trips pay lazy attach costs.
    for _ in 0..10 {
        instance.execute(wire::Command::SaveState).unwrap();
    }
    let mut lat_us = Vec::with_capacity(reps);
    let t0 = Instant::now();
    for _ in 0..reps {
        let t = Instant::now();
        instance.execute(wire::Command::SaveState).unwrap();
        lat_us.push(t.elapsed().as_secs_f64() * 1e6);
    }
    let total = t0.elapsed();
    lat_us.sort_by(f64::total_cmp);
    let avg = lat_us.iter().sum::<f64>() / lat_us.len() as f64;
    println!(
        "PROBE command_rtt reps={reps} ops_per_s={:.1} avg_us={avg:.1} min_us={:.1} med_us={:.1} max_us={:.1} total_ms={:.1}",
        reps as f64 / total.as_secs_f64(),
        lat_us[0],
        lat_us[reps / 2],
        lat_us[reps - 1],
        total.as_secs_f64() * 1e3,
    );
}

/// Bundled-child aggregate data-plane probe: `L1_PROBE_INSTANCES`
/// instances (def 16) share ONE child; every round calls `process`
/// once per instance, spin-paced so each instance is offered
/// `L1_PROBE_INSTANCE_PACE_US` (def 10000 → ~100 blocks/s, the
/// 512-frame/48 kHz audio rate is ~94/s). `L1_PROBE_ROUNDS` (def
/// 200). Reports the summed `dropped_submits`/`passthrough_submits`
/// deltas across all instances — the per-child aggregate ceiling
/// question behind re-trigger conditions 1 and 5.
// 比率表示のみの計測のため usize→f64 キャストを許容する。
#[test]
#[ignore = "manual Issue #120 measurement — run explicitly"]
#[expect(clippy::cast_precision_loss)]
fn probe_bundled_block_rate() {
    let instances_n = env_usize("L1_PROBE_INSTANCES", 16).max(1);
    let rounds = env_usize("L1_PROBE_ROUNDS", 200).max(1);
    let pace = Duration::from_micros(env_usize("L1_PROBE_INSTANCE_PACE_US", 10_000) as u64);
    let frame = env_usize("L1_PROBE_FRAME", 512).max(1);

    let (mut instances, child_arcs) = spawn_topology(1, instances_n, &[]);
    for instance in &mut instances {
        instance.execute(wire::Command::Initialize).unwrap();
        instance
            .execute(wire::Command::Activate { config: config() })
            .unwrap();
        instance.execute(wire::Command::StartProcessing).unwrap();
    }
    let counter_sum = |instances: &[L1Instance]| {
        instances.iter().fold((0_u64, 0_u64), |acc, i| {
            let (d, p) = i.audio_counters().map_or((0, 0), |r| {
                (r.dropped_submits.read(), r.passthrough_submits.read())
            });
            (acc.0 + d, acc.1 + p)
        })
    };
    let input = vec![0.25_f32; frame];
    let mut outputs: Vec<Vec<f32>> = (0..instances_n).map(|_| vec![0.0; frame]).collect();
    // One round = one `process` per instance; the round is paced so
    // each instance sees `pace` between its own calls.
    let round_interval = pace * u32::try_from(instances_n).unwrap_or(u32::MAX);
    let paced_round = |instances: &mut [L1Instance], outputs: &mut [Vec<f32>]| {
        let tr = Instant::now();
        for (i, instance) in instances.iter_mut().enumerate() {
            run_block(instance, &input, &mut outputs[i]).unwrap();
        }
        while tr.elapsed() < round_interval {
            std::hint::spin_loop();
        }
    };
    // Paced warm-up past every instance's lookahead horizon — the
    // offered rate stays under the per-instance ceiling so the
    // pipeline fills without slot-alternation drops, and the counter
    // snapshot below then measures steady state only.
    for _ in 0..10 {
        paced_round(&mut instances, &mut outputs);
    }
    let before = counter_sum(&instances);

    let t0 = Instant::now();
    for _ in 0..rounds {
        paced_round(&mut instances, &mut outputs);
    }
    let total = t0.elapsed();
    let after = counter_sum(&instances);

    let blocks_total = rounds * instances_n;
    println!(
        "PROBE bundled_block_rate children=1 instances={instances_n} rounds={rounds} frame={frame} instance_pace_us={} blocks_per_s={:.1} dropped_submits_delta={} passthrough_submits_delta={} total_ms={:.1}",
        pace.as_micros(),
        blocks_total as f64 / total.as_secs_f64(),
        after.0.saturating_sub(before.0),
        after.1.saturating_sub(before.1),
        total.as_secs_f64() * 1e3,
    );

    for instance in &mut instances {
        instance.destroy();
    }
    for child in &child_arcs {
        child.teardown();
    }
}

/// Data-plane throughput: `process` blocks through the doorbell →
/// driver → shm/`BlockIn` → child → `BlockDone` → reader → result
/// leg pipeline — the single-reader/driver sustained rate per
/// child. `L1_PROBE_BLOCKS` (def 2000), `L1_PROBE_FRAME` (def 512 —
/// must match `config()`'s frame cap), `L1_PROBE_PACE_US` (def 0 —
/// spin-paced inter-call interval; `0` is an unthrottled saturation
/// attempt whose defined drops/pass-throughs mark the ceiling, a
/// positive value paces the caller under it), `L1_PROBE_LOOKAHEAD`
/// (def 2 — shm slot depth `lookahead + 1` sets the in-flight bound).
// 比率表示のみの計測のため usize→f64 キャストを許容する。
#[test]
#[ignore = "manual Issue #120 measurement — run explicitly"]
#[expect(clippy::cast_precision_loss)]
fn probe_block_rate() {
    let blocks = env_usize("L1_PROBE_BLOCKS", 2000).max(1);
    let frame = env_usize("L1_PROBE_FRAME", 512).max(1);
    let pace = Duration::from_micros(env_usize("L1_PROBE_PACE_US", 0) as u64);
    let lookahead = u32::try_from(env_usize("L1_PROBE_LOOKAHEAD", 2)).unwrap_or(2);
    let mut instance = processing_instance_lb(&[], lookahead);

    let input = vec![0.25_f32; frame];
    let mut output = vec![0.0_f32; frame];
    // Warm-up past the lookahead horizon — unpaced, so its slot-
    // alternation drops are excluded by snapshotting counters after.
    for _ in 0..lookahead as usize + 8 {
        run_block(&mut instance, &input, &mut output).unwrap();
    }
    let counters_before = instance.audio_counters().map_or((0, 0), |r| {
        (r.dropped_submits.read(), r.passthrough_submits.read())
    });
    let t0 = Instant::now();
    for _ in 0..blocks {
        // Spin pacing — `thread::sleep` quantizes to the same OS
        // timer tick the cadence probe measures, which would make
        // sub-ms rates meaningless on Windows.
        let tp = Instant::now();
        run_block(&mut instance, &input, &mut output).unwrap();
        if !pace.is_zero() {
            while tp.elapsed() < pace {
                std::hint::spin_loop();
            }
        }
    }
    let total = t0.elapsed();

    let counters_after = instance.audio_counters().map_or((0, 0), |r| {
        (r.dropped_submits.read(), r.passthrough_submits.read())
    });
    println!(
        "PROBE block_rate blocks={blocks} frame={frame} pace_us={} lookahead={lookahead} blocks_per_s={:.1} frames_per_s={:.1} dropped_submits_delta={} passthrough_submits_delta={} total_ms={:.1}",
        pace.as_micros(),
        blocks as f64 / total.as_secs_f64(),
        blocks as f64 * frame as f64 / total.as_secs_f64(),
        counters_after.0.saturating_sub(counters_before.0),
        counters_after.1.saturating_sub(counters_before.1),
        total.as_secs_f64() * 1e3,
    );
}
