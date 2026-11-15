//! Thin MIDI adapter: [`MidiScenario`] cues → [`TimedMidiInput`] feed
//! for `ruviform_audio_core::midi_input::mock::MockPort` (Issue #628,
//! 2nd slice).
//!
//! The feed is a pure conversion layer — it owns no queue and no
//! clock. The test driver advances [`VirtualClockDriver`] and pumps
//! events whose stamp position is due, exactly like a backend's MIDI
//! thread would deliver them. `MockPort`'s `inject_dropped`/
//! `inject_unsupported`/`fail_next_open` fault surface is used
//! unchanged for loss/permission-style MIDI faults.
//!
//! **Stamp semantics:** `stamp.ticks` is the cue position converted
//! *exactly* into the scenario's `frame_base` (one tick = one engine
//! frame when `frame_base` is a sample-rate base). `stamp.base` is
//! that same `frame_base`, `stamp.domain` is the caller-chosen
//! [`ClockDomainId`] (typically `ClockDomainId::new(clock.epoch())`),
//! and `provenance` is [`TimestampProvenance::Carried`] — the
//! scenario declares the instant, so it is a carried fact, not an
//! estimate. Cues whose position is negative or not exactly
//! representable in `frame_base` are counted in [`Self::skipped`] and
//! never emitted (they precede stream start — a real device cannot
//! deliver them).

use ruviform_audio_core::midi_input::TimedMidiInput;
use ruviform_audio_core::midi_input::mock::MockPort;
use ruviform_media_core::{ClockDomainId, InputStamp, TimestampProvenance};

use crate::midi::MidiScenario;

/// Ordered, pumpable stream of [`TimedMidiInput`] events derived from
/// a [`MidiScenario`].
///
/// Events are sorted by `(stamp.ticks, source_sequence)` — the same
/// causal order the production [`TransportPlanBuilder`] uses, so a
/// consumer reading the port sees scenario order regardless of cue
/// list order.
#[derive(Debug)]
pub struct MidiFeed {
    /// Sorted events awaiting delivery (`cursor` = next undelivered).
    events: Vec<TimedMidiInput>,
    cursor: usize,
    domain: ClockDomainId,
    skipped: u64,
}

impl MidiFeed {
    /// Converts `scenario` cues into stamp-ordered [`TimedMidiInput`]s.
    ///
    /// `clock_domain` becomes every stamp's `domain` — pass
    /// `ClockDomainId::new(clock.epoch())` to tie stamps to the
    /// [`crate::clock::VirtualClock`] epoch (choose an epoch `>= 3` to
    /// avoid colliding with the well-known `SAMPLE_CLOCK`/`MONOTONIC`
    /// ids, or set an explicit domain).
    #[must_use]
    pub fn new(scenario: &MidiScenario, clock_domain: ClockDomainId) -> Self {
        // (ticks, source_sequence, event) — sort before stamping so
        // `source_sequence` ordering survives conversion.
        let mut keyed: Vec<(u64, u64, ruviform_media_core::MidiEvent)> = Vec::new();
        let mut skipped = 0u64;
        for cue in &scenario.cues {
            let Some(position) = cue.position.convert_exact(scenario.frame_base) else {
                skipped += 1;
                continue;
            };
            let Ok(ticks) = u64::try_from(position.value()) else {
                skipped += 1;
                continue;
            };
            keyed.push((ticks, cue.source_sequence, cue.event));
        }
        keyed.sort_unstable_by_key(|&(ticks, seq, _)| (ticks, seq));
        let events = keyed
            .into_iter()
            .map(|(ticks, _, event)| TimedMidiInput {
                stamp: InputStamp {
                    ticks,
                    base: scenario.frame_base,
                    domain: clock_domain,
                    provenance: TimestampProvenance::Carried,
                },
                event,
            })
            .collect();
        Self {
            events,
            cursor: 0,
            domain: clock_domain,
            skipped,
        }
    }

    /// The clock domain stamped on every event.
    #[must_use]
    pub fn domain(&self) -> ClockDomainId {
        self.domain
    }

    /// Cues dropped at conversion (negative or unrepresentable
    /// positions) — observable, never silent.
    #[must_use]
    pub fn skipped(&self) -> u64 {
        self.skipped
    }

    /// Events not yet delivered.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.events.len() - self.cursor
    }

    /// Moves events with `stamp.ticks <= upto_frame` into `out`, in
    /// order, and returns the count moved. `upto_frame` is the test
    /// driver's current virtual frame position — e.g.
    /// `VirtualClockDriver::elapsed_frames()` after
    /// `advance_frames(block)`.
    ///
    /// **RT contract:** non-RT — appends to a caller-owned `Vec`
    /// (preallocate it for bounded use).
    pub fn drain_due(&mut self, upto_frame: u64, out: &mut Vec<TimedMidiInput>) -> usize {
        let mut n = 0usize;
        while self.cursor < self.events.len() && self.events[self.cursor].stamp.ticks <= upto_frame
        {
            out.push(self.events[self.cursor]);
            self.cursor += 1;
            n += 1;
        }
        n
    }

    /// Pushes due events directly into a `MockPort` — the common case
    /// for port-level tests. Returns the count pushed.
    pub fn pump_into_port(&mut self, port: &MockPort, upto_frame: u64) -> usize {
        let mut n = 0usize;
        while self.cursor < self.events.len() && self.events[self.cursor].stamp.ticks <= upto_frame
        {
            port.push(self.events[self.cursor]);
            self.cursor += 1;
            n += 1;
        }
        n
    }
}
