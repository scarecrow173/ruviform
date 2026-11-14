//! Independent oracles (Issue #628).
//!
//! Expected values are produced by code paths **independent** of the
//! generators under test: the PCM oracle re-implements each
//! [`PcmPattern`](crate::pcm::PcmPattern) spec in its own code (using
//! libm `f64::sin` for the transcendental patterns, compared with
//! tolerance), and the MIDI oracle sorts cues and maps positions to
//! engine frames without touching `TransportPlanBuilder`,
//! `TransportScheduler` or `SpanSink`.
//!
//! Fixed literal vectors live in `tests/` — they pin both the
//! generators and these oracle implementations against known values.

use ruviform_media_core::{
    ClockEpochId, EngineFrame, LoopOccurrence, MediaTime, MidiEvent, TimeBase,
};

use crate::midi::{MidiCue, MidiTraceEntry};
use crate::pcm::{PcmFixture, PcmPattern};

/// PCM comparison mode.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PcmCompare {
    /// Bit-exact comparison (`f32::to_bits`) — for integer-domain
    /// patterns and noise where both implementations are exact.
    Exact,
    /// Absolute-difference tolerance — for sine/sweep where the oracle
    /// uses libm `f64::sin` against the generator's polynomial.
    AbsEpsilon(f32),
}

/// Maximum absolute difference between two equally-sized PCM buffers.
#[must_use]
pub fn pcm_max_abs_diff(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

/// Bit-exact equality of two PCM buffers (`f32::to_bits`).
#[must_use]
pub fn pcm_bits_eq(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| x.to_bits() == y.to_bits())
}

/// Asserts `got` matches `want` under `mode`; returns the observed
/// max abs diff (or `f32::INFINITY` on length mismatch / Exact failure).
///
/// # Errors
///
/// [`String`] describing the mismatch — test-helper style, not part of
/// the RT surface.
pub fn pcm_assert_eq(got: &[f32], want: &[f32], mode: PcmCompare) -> Result<f32, String> {
    if got.len() != want.len() {
        return Err(format!(
            "length mismatch: got {} samples, want {}",
            got.len(),
            want.len()
        ));
    }
    match mode {
        PcmCompare::Exact => {
            if pcm_bits_eq(got, want) {
                Ok(0.0)
            } else {
                Err(format!(
                    "bit mismatch: max_abs_diff={}",
                    pcm_max_abs_diff(got, want)
                ))
            }
        }
        PcmCompare::AbsEpsilon(eps) => {
            let diff = pcm_max_abs_diff(got, want);
            if diff <= eps {
                Ok(diff)
            } else {
                Err(format!("abs diff {diff} exceeds epsilon {eps}"))
            }
        }
    }
}

/// Independent reference PCM block generated directly from the fixture
/// spec — does **not** call [`PcmFixture::fill`] or
/// [`PcmFixture::sample_at`].
///
/// Transcendental patterns (`Sine`/`Sweep`) use libm `f64::sin` and
/// must be compared with [`PcmCompare::AbsEpsilon`]; all other
/// patterns are exact and may use [`PcmCompare::Exact`].
///
/// # Panics
///
/// Never for validated fixtures; unvalidated `period_frames == 0`
/// yields `0.0` (mirroring the generator's guard).
#[must_use]
pub fn pcm_reference_block(
    fixture: &PcmFixture,
    start_frame: u64,
    frames: u32,
    channel: u16,
) -> Vec<f32> {
    let rate = f64::from(fixture.sample_rate.get());
    (0..frames)
        .map(|i| {
            let frame = start_frame.wrapping_add(u64::from(i));
            ref_sample(&fixture.pattern, rate, channel, frame)
        })
        .collect()
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "independent IEEE-754 reference implementation; same narrowing spec"
)]
fn ref_sample(pattern: &PcmPattern, rate: f64, channel: u16, frame: u64) -> f32 {
    match *pattern {
        PcmPattern::Silence => 0.0,
        PcmPattern::Constant { value } => value,
        PcmPattern::Impulse {
            period_frames,
            amplitude,
        } => {
            let p = u64::from(period_frames);
            if p != 0 && frame.is_multiple_of(p) {
                amplitude
            } else {
                0.0
            }
        }
        PcmPattern::Ramp {
            amplitude,
            period_frames,
        } => {
            let p = u64::from(period_frames);
            if p == 0 {
                return 0.0;
            }
            let phase = (frame % p) as f64 / p as f64;
            (f64::from(amplitude) * (phase * 2.0 - 1.0)) as f32
        }
        PcmPattern::Square {
            amplitude,
            period_frames,
        } => {
            let p = u64::from(period_frames);
            if p == 0 {
                return 0.0;
            }
            let phase = (frame % p) as f64 / p as f64;
            if phase < 0.5 { amplitude } else { -amplitude }
        }
        PcmPattern::Sine { freq_hz, amplitude } => {
            // Independent path: direct f64 phase + libm sin.
            let turns = frame as f64 * freq_hz / rate + f64::from(channel) / 16.0;
            let phase = turns - turns.floor();
            let x = phase * std::f64::consts::TAU - std::f64::consts::PI;
            (x.sin() * f64::from(amplitude)) as f32
        }
        PcmPattern::Sweep {
            f0_hz,
            f1_hz,
            duration_frames,
            amplitude,
        } => {
            let d = duration_frames as f64;
            let t = frame as f64;
            let turns = if duration_frames == 0 || t <= d {
                let k = if duration_frames == 0 {
                    0.0
                } else {
                    (f1_hz - f0_hz) / (2.0 * d)
                };
                (f0_hz * t + k * t * t) / rate
            } else {
                let sweep_turns = (f0_hz * d + (f1_hz - f0_hz) * d / 2.0) / rate;
                sweep_turns + f1_hz * (t - d) / rate
            } + f64::from(channel) / 16.0;
            let phase = turns - turns.floor();
            let x = phase * std::f64::consts::TAU - std::f64::consts::PI;
            (x.sin() * f64::from(amplitude)) as f32
        }
        PcmPattern::Noise { seed, amplitude } => {
            // Independent re-implementation of the same spec: a
            // different mixer (FNV-1a-style fold) is NOT equivalent, so
            // the oracle re-implements the documented splitmix64 spec
            // in separate code — catching generator bugs, not spec drift.
            let h = oracle_mix(seed, channel, frame);
            let unit = ((h >> 40) as f64) * (1.0 / 8_388_608.0) - 1.0;
            (unit * f64::from(amplitude)) as f32
        }
    }
}

/// Oracle-side re-implementation of the noise hash spec — same
/// splitmix64 rounds, independently written.
fn oracle_mix(seed: u64, channel: u16, frame: u64) -> u64 {
    let mut x = seed
        ^ frame.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(channel).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Expected play-through MIDI trace, computed independently: cues are
/// sorted by `(position_ticks, source_sequence)` with the oracle's own
/// position→tick conversion, then each event inside
/// `[0, total_frames)` is expected at engine frame `position_ticks`
/// (play-from-0, one tick per frame, no commands).
///
/// Does **not** call `TransportPlanBuilder`, `TransportScheduler` or
/// `SpanSink`. Only valid for scenarios run with `Play` at frame 0 and
/// no transport commands — command scripts are covered by receipt/fault
/// assertions and [`midi_trace_invariants_hold`].
///
/// `total_frames` bounds delivery: events at or beyond it are not
/// expected; negative positions never fire.
#[must_use]
pub fn midi_expected_playthrough(
    frame_base: TimeBase,
    cues: &[MidiCue],
    total_frames: u64,
) -> Vec<MidiTraceEntry> {
    let mut keyed: Vec<(i64, u64, MidiEvent)> = cues
        .iter()
        .filter_map(|cue| {
            cue.position
                .convert_exact(frame_base)
                .map(|t| (t.value(), cue.source_sequence, cue.event))
        })
        .collect();
    keyed.sort_by_key(|(ticks, seq, _)| (*ticks, *seq));
    keyed
        .into_iter()
        .filter_map(|(ticks, seq, event)| {
            let frame = u64::try_from(ticks).ok()?;
            (frame < total_frames).then_some((ticks, frame, seq, event))
        })
        .map(|(ticks, frame, seq, event)| MidiTraceEntry {
            frame: EngineFrame::new(ClockEpochId::new(0), frame),
            position_ticks: ticks,
            occurrence: LoopOccurrence::new(0),
            source_sequence: seq,
            event,
        })
        .collect()
}

/// Structural invariants every recorded [`crate::midi::MidiTrace`]
/// must satisfy, independent of scenario specifics:
///
/// - entries are ordered by non-decreasing `(frame, source_sequence)`
///   within the same occurrence (delivery is span-ordered);
/// - `position_ticks` equals the event's plan position (checked by the
///   caller against the plan — not repeated here);
/// - frames never decrease across entries.
#[must_use]
pub fn midi_trace_frames_monotonic(entries: &[MidiTraceEntry]) -> bool {
    entries.windows(2).all(|w| {
        w[0].frame.frame() < w[1].frame.frame()
            || (w[0].frame.frame() == w[1].frame.frame()
                && w[0].source_sequence <= w[1].source_sequence)
    })
}

/// Returns true when `trace.entries()` equals `expected` entry-for-entry
/// on `(frame, position_ticks, occurrence, source_sequence, event)`.
#[must_use]
pub fn midi_trace_eq(trace_entries: &[MidiTraceEntry], expected: &[MidiTraceEntry]) -> bool {
    trace_entries.len() == expected.len()
        && trace_entries.iter().zip(expected.iter()).all(|(a, b)| {
            a.frame == b.frame
                && a.position_ticks == b.position_ticks
                && a.occurrence == b.occurrence
                && a.source_sequence == b.source_sequence
                && a.event == b.event
        })
}

/// A `MediaTime` helper kept for oracle callers constructing cue
/// positions without importing media-core directly.
#[must_use]
pub fn media_time(value: i64, base: TimeBase) -> MediaTime {
    MediaTime::new(value, base)
}

/// Convenience re-export of the MIDI payload type for oracle callers.
pub type OracleMidiEvent = MidiEvent;
