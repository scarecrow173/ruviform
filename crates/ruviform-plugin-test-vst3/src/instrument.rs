//! The `test_instrument` signal law — a deterministic, allocation-free
//! polyphonic test voice plus a transport-clock click oracle.
//!
//! ## Voice law
//!
//! Up to [`MAX_VOICES`] voices render a triangle wave at
//! `440 Hz * 2^((key - 69) / 12)` (equal temperament, `f64` coefficient
//! table — every platform computes the identical IEEE-754 result).
//! Phase is a `u64` Q32.32 accumulator advanced once per output frame;
//! the waveform reads the top 24 bits, giving a deterministic
//! `[-1, 1]` triangle with no transcendental calls on the audio path.
//!
//! - `NoteOn` (a `velocity == 0` `NoteOn` follows the MIDI 1.0
//!   running-status convention and behaves as note-off) retriggers the
//!   voice holding the same key, else takes the first inactive voice,
//!   else steals the oldest (`max age`). Channel and UMP group are
//!   ignored — the fixture is channel-agnostic.
//! - `NoteOff` starts a fixed [`RELEASE_FRAMES`]-frame linear release;
//!   the voice deactivates when the release reaches zero.
//! - `ControlChange` 7 sets the master volume (canonical `u32` →
//!   `f32` scale-to-max, default `u32::MAX` = unity); CC 1 sets the
//!   tremolo depth (default `0` = off). All other controllers and
//!   pitch-bend/pressure/program events are defined no-ops.
//!
//! ## Mix law
//!
//! Per frame the mono voice sum is
//! `Σ triangle(phase_v) * velocity_v * envelope_v`, then
//!
//! `out[c][n] = voice_sum * gain * volume * tremolo + click[n]`
//!
//! where `tremolo = 1 - depth * (0.5 + 0.5 * lfo)` with a 4 Hz triangle
//! LFO (`lfo ∈ [-1, 1]`), so depth `0` is transparent and depth `1`
//! pulses between `0` and `1`. The result is written identically to
//! every output channel (mono instrument → dual mono).
//!
//! ## Transport click law
//!
//! While `transport.status() == Playing` **and** musical timing is
//! present (position ticks, tempo and ppqn all valid), the fixture
//! emits a single-sample impulse of amplitude `CLICK_AMP * volume` at
//! every `ppqn`-tick boundary strictly inside the block — i.e. at
//! every quarter-note beat. Tick→frame conversion is exact `i128`
//! ceiling division:
//! `frames = ceil((boundary_tick - position_ticks) * sample_rate *
//! 60000 / (bpm_milli * ppqn))`. Each block derives its click offsets
//! from its own block-start transport snapshot, so no click state
//! crosses block boundaries; `Stopped`/`Paused`, missing musical
//! info, or a zero tempo produce silence on the click path.
//! Loop/seek/chase are therefore exercised purely through the
//! transport snapshot the host delivers — the fixture holds no loop
//! state of its own. The impulse is deliberately a single frame: it
//! is the strongest possible frame-precision oracle for
//! event-offset and block-boundary verification, and overlapping
//! boundaries simply add.
//!
//! [`TestInstrumentProcessor`] implements the shared neutral
//! [`PluginProcessor`] contract — the same contract every Ruviform
//! adapter drives — while the DSP, voice law and click law are this
//! fixture's own (independent of every production adapter).

use ruviform_media_core::{MidiEvent, MidiKind, TransportStatus};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventKind, EventList, PluginProcessor, PrepareError, ProcessConfig,
    ProcessContext, ProcessResult, TransportInfo,
};

use crate::component::{InputSurface, TestComponent};
use crate::controller::INSTRUMENT_CONTROLLER_CID;
use crate::gain::GAIN_PARAM;
use vst3::Steinberg::TUID;
use vst3::uid;

/// Maximum simultaneous voices.
pub(crate) const MAX_VOICES: usize = 8;

/// Linear release length in output frames (50 ms at 48 kHz).
const RELEASE_FRAMES: u32 = 2_400;

/// Tremolo LFO frequency in Hz.
const TREMOLO_HZ: f64 = 4.0;

/// Click impulse amplitude (pre-volume).
const CLICK_AMP: f32 = 0.5;

/// Maximum click impulses scheduled per block — a defensive bound so
/// a hostile tempo cannot spin the scheduler; excess boundaries are
/// dropped (defined, deterministic).
const MAX_CLICKS_PER_BLOCK: usize = 512;

/// `2^(n/12)` for `n = 0..=11` — correctly-rounded `f64` literals, so
/// `SEMITONE[i] * 2^octave` is exact IEEE-754 arithmetic on every
/// platform.
const SEMITONE: [f64; 12] = [
    1.0,
    1.059_463_094_359_295_3,
    1.122_462_048_309_373,
    1.189_207_115_002_721,
    1.259_921_049_894_873_2,
    1.334_839_854_170_034_4,
    std::f64::consts::SQRT_2,
    1.498_307_076_876_792,
    1.587_401_051_015_199_7,
    1.681_792_830_507_429,
    1.781_797_436_280_678_5,
    1.887_748_625_363_387,
];

/// Frequency of MIDI key `key` in Hz — `440 * 2^((key-69)/12)`.
/// `f64` only; called on note-on (RT-bounded, no allocation).
fn frequency_hz(key: u8) -> f64 {
    let offset = i32::from(key) - 69;
    let semitone = SEMITONE[offset.rem_euclid(12) as usize];
    440.0 * semitone * 2f64.powi(offset.div_euclid(12))
}

/// Q32.32 phase increment per output frame for `freq_hz` at
/// `sample_rate`.
fn phase_increment(freq_hz: f64, sample_rate: u32) -> u64 {
    let rate = f64::from(sample_rate);
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let inc = (freq_hz * 4_294_967_296.0 / rate) as u64;
    inc
}

/// One polyphonic voice.
#[derive(Clone, Copy)]
struct Voice {
    /// Whether the voice is producing sound (including release tail).
    active: bool,
    /// MIDI key number `0..=127`.
    key: u8,
    /// Canonical velocity (`u32` scale-to-max).
    velocity: u32,
    /// Q32.32 phase accumulator.
    phase: u64,
    /// Q32.32 per-frame phase increment.
    phase_inc: u64,
    /// Frames since note-on — the deterministic steal policy key.
    age: u32,
    /// Frames left in the release envelope; `u32::MAX` = sustained.
    release_left: u32,
}

impl Voice {
    const INACTIVE: Self = Self {
        active: false,
        key: 0,
        velocity: 0,
        phase: 0,
        phase_inc: 0,
        age: 0,
        release_left: u32::MAX,
    };

    /// The voice's current amplitude envelope in `[0, 1]` — evaluated
    /// *before* this frame's decrement, so frame `k` of the release
    /// carries `(RELEASE_FRAMES - k) / RELEASE_FRAMES`.
    fn envelope(self) -> f32 {
        if self.release_left == u32::MAX {
            1.0
        } else {
            #[expect(clippy::cast_precision_loss)]
            let (left, total) = (self.release_left as f32, RELEASE_FRAMES as f32);
            (left / total).clamp(0.0, 1.0)
        }
    }

    /// Advances the voice one frame and returns its triangle sample
    /// (before velocity/envelope/gain scaling).
    fn render(&mut self) -> f32 {
        // Top 24 bits of the Q32.32 fraction → x ∈ [0, 1).
        #[expect(clippy::cast_precision_loss)]
        let x = (((self.phase >> 8) & 0xFF_FFFF) as f32) * (1.0 / 16_777_216.0);
        let tri = if x < 0.5 {
            4.0 * x - 1.0
        } else {
            3.0 - 4.0 * x
        };
        self.phase = self.phase.wrapping_add(self.phase_inc);
        self.age = self.age.wrapping_add(1);
        if self.release_left != u32::MAX {
            self.release_left = self.release_left.saturating_sub(1);
            if self.release_left == 0 {
                self.active = false;
            }
        }
        tri
    }
}

/// The `test_instrument` processor.
pub(crate) struct TestInstrumentProcessor {
    /// Persistent voices.
    voices: [Voice; MAX_VOICES],
    /// Current gain in the plain domain (`[0.0, 2.0]`), `f64` for
    /// exact parameter-domain precision — same law as `test_gain`.
    gain: f64,
    /// CC 7 master volume, canonical `u32` scale-to-max.
    volume: u32,
    /// CC 1 tremolo depth, canonical `u32` scale-to-max.
    tremolo_depth: u32,
    /// Q32.32 tremolo LFO phase accumulator.
    lfo_phase: u64,
    /// Q32.32 tremolo LFO per-frame increment.
    lfo_inc: u64,
    /// Prepared sample rate (Hz).
    sample_rate: u32,
    /// This block's scheduled click offsets (ascending, ≤
    /// [`MAX_CLICKS_PER_BLOCK`]).
    click_scratch: [u32; MAX_CLICKS_PER_BLOCK],
    /// Number of valid entries in `click_scratch`.
    click_count: usize,
    /// Read cursor into `click_scratch` — clicks are consumed in
    /// ascending frame order across the block's segments.
    click_cursor: usize,
}

impl TestInstrumentProcessor {
    /// A fresh processor at deterministic defaults.
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            voices: [Voice::INACTIVE; MAX_VOICES],
            gain: GAIN_PARAM.default_value,
            volume: u32::MAX,
            tremolo_depth: 0,
            lfo_phase: 0,
            lfo_inc: 0,
            sample_rate: 48_000,
            click_scratch: [0; MAX_CLICKS_PER_BLOCK],
            click_count: 0,
            click_cursor: 0,
        }
    }

    /// Applies one MIDI channel-voice event. Channel/group agnostic.
    fn apply_midi(&mut self, midi: &MidiEvent) {
        match midi.kind {
            MidiKind::NoteOn { key, velocity } => {
                if velocity != 0 {
                    self.note_on(key.get(), velocity);
                } else {
                    self.note_off(key.get());
                }
            }
            MidiKind::NoteOff { key, .. } => self.note_off(key.get()),
            MidiKind::ControlChange { controller, value } => match controller.get() {
                7 => self.volume = value,
                1 => self.tremolo_depth = value,
                _ => {}
            },
            _ => {}
        }
    }

    /// Note-on: retrigger the same-key voice, else first inactive,
    /// else steal the oldest.
    fn note_on(&mut self, key: u8, velocity: u32) {
        let slot = self
            .voices
            .iter()
            .position(|v| v.active && v.key == key)
            .or_else(|| self.voices.iter().position(|v| !v.active))
            .unwrap_or_else(|| {
                // Steal the oldest voice — `max age` is deterministic.
                self.voices
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, v)| v.age)
                    .map_or(0, |(i, _)| i)
            });
        #[expect(clippy::indexing_slicing)]
        let voice = &mut self.voices[slot];
        *voice = Voice {
            active: true,
            key,
            velocity,
            phase: 0,
            phase_inc: phase_increment(frequency_hz(key), self.sample_rate),
            age: 0,
            release_left: u32::MAX,
        };
    }

    /// Note-off: begin the fixed linear release on the matching voice.
    fn note_off(&mut self, key: u8) {
        if let Some(voice) = self.voices.iter_mut().find(|v| v.active && v.key == key) {
            voice.release_left = RELEASE_FRAMES;
        }
    }

    /// Schedules this block's click impulse offsets into
    /// `click_scratch` (ascending, bounded). Empty unless transport is
    /// playing with full musical timing — see module docs.
    fn schedule_clicks(&mut self, transport: Option<TransportInfo>, frames: usize) {
        self.click_count = 0;
        self.click_cursor = 0;
        let Some(transport) = transport else {
            return;
        };
        if transport.status() != TransportStatus::Playing {
            return;
        }
        let Some(musical) = transport.musical() else {
            return;
        };
        let bpm_milli = i128::from(musical.bpm_milli);
        let ppqn = i128::from(u32::from(musical.ppqn));
        if bpm_milli <= 0 || ppqn <= 0 {
            return;
        }
        // frames = ceil((boundary - pos) * rate * 60000 / (bpm * ppqn)).
        let den = bpm_milli * ppqn;
        let pos = i128::from(musical.position_ticks);
        let rate = i128::from(self.sample_rate);
        // First `ppqn` boundary at or after the block-start position —
        // a position exactly on a boundary clicks at frame 0.
        let mut boundary = pos.div_euclid(ppqn) * ppqn;
        if boundary < pos {
            boundary += ppqn;
        }
        while self.click_count < MAX_CLICKS_PER_BLOCK {
            let delta = boundary - pos;
            let num = delta * rate * 60_000;
            let frame = num.div_euclid(den) + i128::from(num.rem_euclid(den) != 0);
            if frame >= i128::from(u32::try_from(frames).unwrap_or(u32::MAX)) {
                break;
            }
            if frame < 0 {
                // Boundary behind the block start cannot happen
                // (`boundary > pos` by construction), but stay
                // defined if musical data is inconsistent.
                boundary += ppqn;
                continue;
            }
            // `frame` is in `[0, frames)` — the bounds checks above —
            // and `frames <= u32::MAX`, so the conversion cannot fail.
            let frame = u32::try_from(frame).unwrap_or(u32::MAX);
            self.click_scratch[self.click_count] = frame;
            self.click_count += 1;
            boundary += ppqn;
        }
    }

    /// Renders frame `frame` (absolute block offset) to `out`.
    /// Advances the click cursor — frames must be rendered in
    /// ascending order within a `process` call.
    fn render_frame(&mut self, frame: u32) -> f32 {
        let mut sum = 0.0_f32;
        for voice in &mut self.voices {
            if voice.active {
                // Envelope is read before `render` decrements the
                // release counter — frame `k` of the release carries
                // `(RELEASE_FRAMES - k) / RELEASE_FRAMES`.
                let envelope = voice.envelope();
                let tri = voice.render();
                #[expect(clippy::cast_precision_loss)]
                let vel = voice.velocity as f32 / 4_294_967_295.0;
                sum += tri * vel * envelope;
            }
        }
        // Tremolo LFO — 4 Hz triangle, phase advances once per frame.
        #[expect(clippy::cast_precision_loss)]
        let lx = (((self.lfo_phase >> 8) & 0xFF_FFFF) as f32) * (1.0 / 16_777_216.0);
        let lfo = if lx < 0.5 {
            4.0 * lx - 1.0
        } else {
            3.0 - 4.0 * lx
        };
        self.lfo_phase = self.lfo_phase.wrapping_add(self.lfo_inc);
        #[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let (depth, volume, gain) = (
            self.tremolo_depth as f32 / 4_294_967_295.0,
            self.volume as f32 / 4_294_967_295.0,
            self.gain as f32,
        );
        let tremolo = 1.0 - depth * (0.5 + 0.5 * lfo);
        let mut out = sum * gain * volume * tremolo;
        // Consume click impulses at this frame (sorted, ascending).
        while self.click_cursor < self.click_count && self.click_scratch[self.click_cursor] < frame
        {
            self.click_cursor += 1;
        }
        while self.click_cursor < self.click_count && self.click_scratch[self.click_cursor] == frame
        {
            out += CLICK_AMP * volume;
            self.click_cursor += 1;
        }
        out
    }

    /// Writes one rendered frame to every output channel at `frame`.
    fn write_frame<O: BufferViewMut>(&mut self, outputs: &mut [O], frame: usize) {
        let sample = self.render_frame(u32::try_from(frame).unwrap_or(u32::MAX));
        for output in outputs.iter_mut() {
            let channels = output.channels();
            for channel in 0..channels {
                if let Some(dst) = output.channel_mut(channel)
                    && let Some(slot) = dst.get_mut(frame)
                {
                    *slot = sample;
                }
            }
        }
    }

    /// Renders frames `[start, end)` of the block.
    fn render_segment<O: BufferViewMut>(&mut self, outputs: &mut [O], start: usize, end: usize) {
        for frame in start..end {
            self.write_frame(outputs, frame);
        }
    }
}

impl PluginProcessor for TestInstrumentProcessor {
    /// Activation hook — resets voices, controllers and LFO to
    /// deterministic defaults; captures the sample rate for the
    /// frequency/tempo laws. Non-RT.
    fn prepare(
        &mut self,
        config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        self.voices = [Voice::INACTIVE; MAX_VOICES];
        self.gain = GAIN_PARAM.default_value;
        self.volume = u32::MAX;
        self.tremolo_depth = 0;
        self.lfo_phase = 0;
        self.sample_rate = config.sample_rate().get();
        self.lfo_inc = phase_increment(TREMOLO_HZ, self.sample_rate);
        self.click_count = 0;
        self.click_cursor = 0;
        Ok(())
    }

    /// Runs one block: schedules transport clicks, then walks the
    /// (sorted) event list once — rendering `[cursor, offset)` under
    /// the current state, applying the event at its offset, and
    /// finishing with the tail segment. Identical offset law to
    /// `test_gain`: an event affects its own offset frame onward;
    /// events on a zero-frame block still update persistent state.
    ///
    /// `inputs` are ignored — the instrument is generative; `outputs`
    /// are the zero-copy channel views built by the block layer.
    ///
    /// **RT contract:** RT-suitable — bounded `O(frames × (voices +
    /// channels) + events + clicks)` work; no allocation, locking,
    /// waiting or I/O.
    fn process<I, O>(
        &mut self,
        _inputs: &[I],
        outputs: &mut [O],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        let frames = outputs.first().map_or(0, BufferView::frames);
        self.schedule_clicks(transport, frames);
        let mut cursor = 0_usize;
        for event in events.iter() {
            let offset = usize::try_from(event.frame_offset())
                .unwrap_or(usize::MAX)
                .min(frames);
            self.render_segment(outputs, cursor, offset);
            match event.kind() {
                EventKind::ParamValue(param) if param.param_id == GAIN_PARAM.id => {
                    self.gain = GAIN_PARAM.denormalize(param.value);
                }
                EventKind::Midi(midi) => self.apply_midi(&midi),
                _ => {}
            }
            cursor = offset;
        }
        self.render_segment(outputs, cursor, frames);
        if outputs.is_empty() {
            ProcessResult::SLEEP
        } else {
            ProcessResult::CONTINUE
        }
    }
}

/// `test_instrument`'s component — the shared [`TestComponent`] shell
/// over [`TestInstrumentProcessor`] with the `MidiTransport` input
/// surface (MIDI events + process-context transport).
pub(crate) type TestInstrumentComponent = TestComponent<TestInstrumentProcessor>;

impl TestInstrumentComponent {
    /// The component class ID — a fresh `uid` for the test suite,
    /// distinct from every other fixture CID.
    pub(crate) const CID: TUID = uid(0x4A7E_1C93, 0x8D5F_2B46, 0xE9A3_7C81, 0x5F2D_8B64);

    pub(crate) fn new() -> Self {
        Self::with(
            INSTRUMENT_CONTROLLER_CID,
            GAIN_PARAM.id,
            TestInstrumentProcessor::new(),
        )
        .with_surface(InputSurface::MidiTransport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
    use ruviform_media_core::{
        MediaTime, MidiChannel, MidiController, MidiGroup, MusicalTimeInfo, NoteKey, TimeBase,
    };
    use ruviform_plugin_core::{Event, ParamEvent, ParamValue};

    const RATE: u32 = 48_000;

    fn midi_event(offset: u32, kind: MidiKind) -> Event {
        Event::new(
            offset,
            EventKind::Midi(MidiEvent {
                group: MidiGroup::ZERO,
                channel: MidiChannel::new(0).unwrap(),
                kind,
            }),
        )
    }

    fn note_on(offset: u32, key: u8, velocity: u32) -> Event {
        midi_event(
            offset,
            MidiKind::NoteOn {
                key: NoteKey::new(key).unwrap(),
                velocity,
            },
        )
    }

    fn note_off(offset: u32, key: u8) -> Event {
        midi_event(
            offset,
            MidiKind::NoteOff {
                key: NoteKey::new(key).unwrap(),
                velocity: 0,
            },
        )
    }

    fn cc(offset: u32, controller: u8, value: u32) -> Event {
        midi_event(
            offset,
            MidiKind::ControlChange {
                controller: MidiController::new(controller).unwrap(),
                value,
            },
        )
    }

    /// Runs one prepared block through the neutral contract — the
    /// same call shape the component's `process` makes. `inputs` are
    /// ignored by the generative instrument, so the slice is empty.
    fn run_block(
        processor: &mut TestInstrumentProcessor,
        output: &mut [Vec<f32>],
        events: &[Event],
        transport: Option<TransportInfo>,
    ) {
        let frames = u32::try_from(output.first().map_or(0, Vec::len)).unwrap_or(u32::MAX);
        let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
        let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
        let list = EventList::new(events, frames).unwrap();
        let mut context = ProcessContext::new();
        processor.process::<PlanarBufferView, PlanarBufferViewMut>(
            &[],
            &mut outputs,
            list,
            transport,
            &mut context,
        );
    }

    fn prepared() -> TestInstrumentProcessor {
        let mut processor = TestInstrumentProcessor::new();
        let config = ProcessConfig::new(SampleRate::new(RATE).unwrap(), 1, 8192).unwrap();
        processor
            .prepare(&config, &mut ProcessContext::new())
            .unwrap();
        processor
    }

    fn playing_transport(position_ticks: i64, bpm_milli: u32) -> TransportInfo {
        TransportInfo::new(
            TransportStatus::Playing,
            MediaTime::new(0, TimeBase::sample_rate(RATE).unwrap()),
        )
        .with_musical(MusicalTimeInfo {
            position_ticks,
            bar_start_tick: 0,
            bpm_milli,
            ppqn: 960,
            sig_num: 4,
            sig_den: 4,
        })
    }

    #[test]
    fn silence_without_events() {
        let mut processor = prepared();
        let mut output = [vec![9.0_f32; 16], vec![9.0; 16]];
        run_block(&mut processor, &mut output, &[], None);
        assert!(output[0].iter().all(|&s| s == 0.0));
        assert!(output[1].iter().all(|&s| s == 0.0));
    }

    #[test]
    fn note_on_renders_deterministic_voice() {
        let mut processor = prepared();
        let events = [note_on(0, 69, u32::MAX)];
        let mut output = [vec![0.0_f32; 32]];
        run_block(&mut processor, &mut output, &events, None);
        // Frame 0: phase 0 → triangle −1 → out = −1 * 1 * 1 * 1 * 1.
        assert_eq!(output[0][0], -1.0);
        // A fresh processor reproduces the block bit-exactly.
        let mut fresh = prepared();
        let mut oracle = [vec![0.0_f32; 32]];
        run_block(&mut fresh, &mut oracle, &events, None);
        assert_eq!(output[0], oracle[0]);
    }

    #[test]
    fn note_off_releases_then_silences() {
        let mut processor = prepared();
        // Note on for one block, off at offset 0 of the next.
        let on = [note_on(0, 69, u32::MAX)];
        let mut output = [vec![0.0_f32; 8]];
        run_block(&mut processor, &mut output, &on, None);
        let off = [note_off(0, 69)];
        let mut tail = [vec![0.0_f32; (RELEASE_FRAMES + 8) as usize]];
        run_block(&mut processor, &mut tail, &off, None);
        // Release envelope decays linearly; after RELEASE_FRAMES the
        // voice is inactive → exact silence.
        assert!(tail[0][RELEASE_FRAMES as usize..].iter().all(|&s| s == 0.0));
        // The release start is non-zero (envelope ~1.0 at frame 0).
        assert_ne!(tail[0][0], 0.0);
    }

    #[test]
    fn velocity_scales_amplitude() {
        let mut quiet = prepared();
        let mut loud = prepared();
        let half = u32::MAX / 2;
        let mut q = [vec![0.0_f32; 4]];
        let mut l = [vec![0.0_f32; 4]];
        run_block(&mut quiet, &mut q, &[note_on(0, 69, half)], None);
        run_block(&mut loud, &mut l, &[note_on(0, 69, u32::MAX)], None);
        // Same phase → ratio is exactly velocity ratio.
        let ratio = l[0][0] / q[0][0];
        #[expect(clippy::cast_precision_loss)]
        let expected = (u32::MAX as f32) / (half as f32);
        assert!((ratio - expected).abs() < 1e-5);
    }

    #[test]
    fn cc7_scales_volume_and_cc1_enables_tremolo() {
        let mut processor = prepared();
        // CC7 = half → volume ~0.5; then a note.
        let events = [cc(0, 7, u32::MAX / 2), note_on(0, 69, u32::MAX)];
        let mut output = [vec![0.0_f32; 8]];
        run_block(&mut processor, &mut output, &events, None);
        // volume = (u32::MAX/2)/u32::MAX ≈ 0.5 → frame0 ≈ −0.5.
        assert!((output[0][0] + 0.5).abs() < 0.01);
        // CC1 full depth → tremolo modulates: later frames differ
        // from the unmodulated case.
        let mut modulated = prepared();
        let mut plain = prepared();
        let mut m = [vec![0.0_f32; 480]];
        let mut p = [vec![0.0_f32; 480]];
        run_block(
            &mut modulated,
            &mut m,
            &[cc(0, 1, u32::MAX), note_on(0, 69, u32::MAX)],
            None,
        );
        run_block(&mut plain, &mut p, &[note_on(0, 69, u32::MAX)], None);
        assert_ne!(m[0], p[0]);
    }

    #[test]
    fn gain_param_scales_voices() {
        let mut processor = prepared();
        // normalized 0.0 → plain 0.0 → silence despite an active voice.
        let events = [
            note_on(0, 69, u32::MAX),
            Event::new(
                0,
                EventKind::ParamValue(ParamEvent {
                    param_id: GAIN_PARAM.id,
                    value: ParamValue::new(0.0).unwrap(),
                }),
            ),
        ];
        let mut output = [vec![0.0_f32; 4]];
        run_block(&mut processor, &mut output, &events, None);
        assert!(output[0].iter().all(|&s| s == 0.0));
    }

    #[test]
    fn note_on_offset_gates_voice_start() {
        let mut processor = prepared();
        // NoteOn at offset 4 → frames 0..4 silent, frame 4 sounds.
        let events = [note_on(4, 69, u32::MAX)];
        let mut output = [vec![0.0_f32; 8]];
        run_block(&mut processor, &mut output, &events, None);
        assert!(output[0][..4].iter().all(|&s| s == 0.0));
        assert_ne!(output[0][4], 0.0);
    }

    #[test]
    fn ninth_note_steals_oldest_voice() {
        let mut processor = prepared();
        let events: Vec<Event> = (0..9_u8).map(|k| note_on(0, k, u32::MAX)).collect();
        let mut output = [vec![0.0_f32; 4]];
        run_block(&mut processor, &mut output, &events, None);
        // Nine note-ons on eight voices — the ninth steals; output is
        // still deterministic sound (which voice is an implementation
        // detail the law fixes, not this test's oracle).
        assert!(output[0].iter().any(|&s| s != 0.0));
    }

    #[test]
    fn retrigger_same_key_resets_phase() {
        let mut processor = prepared();
        let events = [note_on(0, 69, u32::MAX)];
        let mut first = [vec![0.0_f32; 8]];
        run_block(&mut processor, &mut first, &events, None);
        // Retrigger at offset 0 of next block → frame 0 is −1 again.
        let mut second = [vec![0.0_f32; 8]];
        run_block(&mut processor, &mut second, &events, None);
        assert_eq!(second[0][0], -1.0);
    }

    #[test]
    fn playing_transport_clicks_at_beat_boundaries() {
        let mut processor = prepared();
        // 120 BPM, ppqn 960 → beat = 480000*... ticks/sec = 120000/60000*960
        // = 1920 ticks/s → at 48kHz a beat = 48000*60000/(120000*960)
        // = 25 frames? No: frames/beat = rate*60/bpm... = 48000*0.5 = 24000.
        // position_ticks = 0 → first boundary at tick 960 → frame =
        // ceil(960 * 48000 * 60000 / (120000 * 960)) = ceil(24000) = 24000.
        // Use a small block around it: position such that boundary
        // lands inside — pos = 960 - small. Simpler: pos_ticks = 0,
        // block of 48001 frames → click at frame 24000.
        let transport = playing_transport(0, 120_000);
        let mut output = [vec![0.0_f32; 48_001]];
        run_block(&mut processor, &mut output, &[], Some(transport));
        assert_eq!(output[0][24_000], CLICK_AMP);
        assert_eq!(output[0][24_001], 0.0);
    }

    #[test]
    fn stopped_transport_produces_no_clicks() {
        let mut processor = prepared();
        let transport = TransportInfo::new(
            TransportStatus::Stopped,
            MediaTime::new(0, TimeBase::sample_rate(RATE).unwrap()),
        )
        .with_musical(MusicalTimeInfo {
            position_ticks: 0,
            bar_start_tick: 0,
            bpm_milli: 120_000,
            ppqn: 960,
            sig_num: 4,
            sig_den: 4,
        });
        let mut output = [vec![0.0_f32; 48_001]];
        run_block(&mut processor, &mut output, &[], Some(transport));
        assert!(output[0].iter().all(|&s| s == 0.0));
    }

    #[test]
    fn seek_shifts_click_position() {
        // Same tempo; position_ticks jumps forward by one beat → the
        // next boundary moves with it.
        let mut a = prepared();
        let mut b = prepared();
        let t_a = playing_transport(0, 120_000);
        // pos = 480 ticks (half a beat in) → boundaries at 960, 1920,
        // … → deltas 480, 1440, … → click frames 12000, 36000, …
        let t_b = playing_transport(480, 120_000);
        let mut out_a = [vec![0.0_f32; 48_001]];
        let mut out_b = [vec![0.0_f32; 48_001]];
        run_block(&mut a, &mut out_a, &[], Some(t_a));
        run_block(&mut b, &mut out_b, &[], Some(t_b));
        // pos=0: boundary at tick 0 → click at frame 0, then 24000.
        assert_eq!(out_a[0][0], CLICK_AMP);
        assert_eq!(out_a[0][24_000], CLICK_AMP);
        // pos=480: no boundary at frame 0 — first click at 12000.
        assert_eq!(out_b[0][0], 0.0);
        assert_eq!(out_b[0][12_000], CLICK_AMP);
        assert_eq!(out_b[0][36_000], CLICK_AMP);
    }

    #[test]
    fn click_respects_cc7_volume() {
        let mut processor = prepared();
        let transport = playing_transport(0, 120_000);
        let events = [cc(0, 7, 0)]; // volume 0 → click silent too
        let mut output = [vec![0.0_f32; 48_001]];
        run_block(&mut processor, &mut output, &events, Some(transport));
        assert!(output[0].iter().all(|&s| s == 0.0));
    }

    #[test]
    fn zero_frame_block_flushes_events() {
        let mut processor = prepared();
        let on = [note_on(0, 69, u32::MAX)];
        processor.process::<PlanarBufferView, PlanarBufferViewMut>(
            &[],
            &mut [],
            EventList::new(&on, 0).unwrap(),
            None,
            &mut ProcessContext::new(),
        );
        let mut output = [vec![0.0_f32; 4]];
        run_block(&mut processor, &mut output, &[], None);
        assert_ne!(output[0][0], 0.0);
    }
}
