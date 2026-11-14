//! `VirtualClock` determinism tests (Issue #628).

use std::sync::Arc;

use ruviform_audio_test::clock::{
    JitterSpec, VirtualClock, VirtualClockConfig, VirtualClockDriver,
};

fn cfg() -> VirtualClockConfig {
    VirtualClockConfig {
        sample_rate: 48_000,
        epoch: 7,
        offset_ticks: 100,
        drift_ppm: 0,
        jitter: JitterSpec::DISABLED,
    }
}

#[test]
fn tick_is_pure_function_of_elapsed() {
    let c = cfg();
    assert_eq!(c.tick_at(0), 100);
    assert_eq!(c.tick_at(48_000), 48_100);
    assert_eq!(c.tick_at(u64::MAX), u64::MAX); // clamps, never wraps negative
}

#[test]
fn negative_offset_clamps_at_zero() {
    let c = VirtualClockConfig {
        offset_ticks: -50,
        ..cfg()
    };
    assert_eq!(c.tick_at(0), 0);
    assert_eq!(c.tick_at(100), 50);
}

#[test]
fn drift_is_deterministic_ppm() {
    let c = VirtualClockConfig {
        drift_ppm: 1_000, // +0.1%
        ..cfg()
    };
    // elapsed=1_000_000 → drift = 1_000 ticks
    assert_eq!(c.tick_at(1_000_000), 100 + 1_000_000 + 1_000);
    // same input → same output, always
    assert_eq!(c.tick_at(1_000_000), c.tick_at(1_000_000));
}

#[test]
fn jitter_is_bounded_and_deterministic() {
    let c = VirtualClockConfig {
        jitter: JitterSpec {
            seed: 42,
            period_frames: 64,
            amplitude_ticks: 3,
        },
        ..cfg()
    };
    for elapsed in [0u64, 1, 63, 64, 65, 1_000, 1_000_000] {
        let base = 100 + elapsed; // drift = 0
        let tick = c.tick_at(elapsed);
        let dev = i128::from(tick) - i128::from(base);
        assert!(
            (-3..=3).contains(&dev),
            "elapsed={elapsed} tick={tick} deviates {dev} beyond ±3"
        );
        // determinism: identical elapsed → identical tick
        assert_eq!(tick, c.tick_at(elapsed));
    }
    // different seeds produce different jitter streams (almost surely)
    let c2 = VirtualClockConfig {
        jitter: JitterSpec {
            seed: 43,
            ..c.jitter
        },
        ..cfg()
    };
    let diffs: Vec<u64> = (0..256)
        .filter(|e| c.tick_at(*e) != c2.tick_at(*e))
        .collect();
    assert!(!diffs.is_empty(), "seed change must alter jitter stream");
}

#[test]
fn monotonic_nanos_is_exact_rational() {
    let c = cfg();
    assert_eq!(c.monotonic_nanos_at(0), 0);
    assert_eq!(c.monotonic_nanos_at(48_000), 1_000_000_000);
    assert_eq!(c.monotonic_nanos_at(1), 20_833); // truncation, deterministic
}

#[test]
fn driver_pause_resume_and_seek() {
    let clock = Arc::new(VirtualClock::new(cfg()).unwrap());
    let mut driver = VirtualClockDriver::new(Arc::clone(&clock));
    // initial publish at elapsed 0
    assert_eq!(clock.tick(), 100);
    assert_eq!(clock.monotonic_nanos(), 0);

    driver.advance_frames(480);
    assert_eq!(clock.tick(), 580);
    assert_eq!(clock.monotonic_nanos(), 10_000_000);

    driver.pause();
    driver.advance_frames(1_000);
    assert_eq!(clock.tick(), 580, "paused clock must not advance");
    assert_eq!(driver.elapsed_frames(), 480);

    driver.resume();
    driver.advance_frames(480);
    assert_eq!(clock.tick(), 1_060);
    assert_eq!(driver.elapsed_frames(), 960);

    // seek = deterministic discontinuity
    driver.seek_frames(96_000);
    assert_eq!(clock.tick(), 96_100);
    assert_eq!(clock.monotonic_nanos(), 2_000_000_000);
}

#[test]
fn driver_deadline_tracks_elapsed() {
    let clock = Arc::new(VirtualClock::new(cfg()).unwrap());
    let mut driver = VirtualClockDriver::new(clock);
    driver.advance_frames(100);
    assert_eq!(driver.deadline_in(50), 250); // tick_at(150) = 100+150
}

#[test]
fn zero_sample_rate_is_rejected() {
    let bad = VirtualClockConfig {
        sample_rate: 0,
        ..cfg()
    };
    assert!(VirtualClock::new(bad).is_err());
}

#[test]
fn identical_config_reproduces_identical_streams() {
    // R1 determinism across independently constructed clocks.
    let run = || {
        let clock = Arc::new(VirtualClock::new(cfg()).unwrap());
        let mut d = VirtualClockDriver::new(clock);
        let mut ticks = Vec::new();
        for _ in 0..8 {
            d.advance_frames(1_337);
            ticks.push(d.clock().tick());
        }
        ticks
    };
    assert_eq!(run(), run());
}
