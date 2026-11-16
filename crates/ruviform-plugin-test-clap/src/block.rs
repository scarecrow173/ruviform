//! The CLAP → neutral translation layer: activation-config mapping,
//! per-block channel-pair views, input-event translation and process
//! status mapping. Everything here is a `pub(crate)` function the
//! `plugin` module's ABI callbacks call — no binding logic lives in
//! `plugin.rs` that cannot be exercised through these testable
//! functions.
//!
//! ## Scope of this fixture's translation
//!
//! `test_gain` consumes exactly one event kind — `ParamValue` for
//! its gain parameter — and no transport state. MIDI, note and
//! transport events arriving in the input list are a defined skip
//! (the plugin declares no note ports and no process-context
//! requirements), identical in spirit to the VST3 `test_gain` slice
//! which declares no event buses at all.
//!
//! ## RT contract
//!
//! Every `pub(crate)` fn in this module is RT-suitable — bounded
//! work over at most [`MAX_CHANNELS`] channels and
//! [`MAX_EVENTS_PER_BLOCK`] events, no allocation on the process
//! path (`BlockScratch` is allocated once at `activate`).

use core::mem::size_of;
use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use clack_common::events::Match;
use clack_common::events::UnknownEvent;
use clack_common::events::event_types::{
    MidiEvent as ClapMidiEvent, TransportEvent, TransportFlags,
};
use clack_common::events::io::InputEvents;
use clack_common::events::spaces::CoreEventSpace;
use clack_plugin::plugin::PluginError;
use clack_plugin::process::audio::{BufferError, ChannelPair, PortPair, SampleType};
use clack_plugin::process::{PluginAudioConfiguration, ProcessStatus as ClapProcessStatus};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_media_core::{
    MediaDuration, MediaRange, MediaTime, MidiChannel, MidiController, MidiEvent, MidiGroup,
    MidiKind, MidiProgram, MidiValue, MusicalTimeInfo, NoteKey, TimeBase, TransportStatus,
};
use ruviform_plugin_core::{
    Event, EventKind, EventList, LifecycleError, LifecycleEvent, LifecycleState, ParamEvent,
    ParamId, ParamInfo, ProcessConfig, ProcessResult, ProcessStatus, TransportInfo,
    sort_events_by_frame_offset,
};

/// Channel capacity negotiated by this fixture: one stereo port pair.
pub(crate) const MAX_CHANNELS: usize = 2;

/// Maximum input events translated per block. Hosts may attach
/// arbitrary event counts; the per-block scan must stay bounded, so
/// oversized lists are a defined rejection rather than unbounded work.
/// 1024 matches the VST3 fixture's `MAX_BLOCK_EVENTS` and covers the
/// densest realistic host streams (the external `clap-validator`
/// note generators emit several hundred events per 512-frame block).
pub(crate) const MAX_EVENTS_PER_BLOCK: usize = 1_024;

/// Retry bound for the atomic lifecycle CAS — two threads (audio and
/// main) can contend, and contention beyond this cap means concurrent
/// calls the CLAP threading contract forbids (a defined rejection).
const MAX_CAS_ATTEMPTS: u32 = 8;

/// Per-instance scratch allocated at `activate` — channel-pointer
/// bookkeeping used by the alias scan and the neutral event list.
///
/// No `static`/`thread_local` scratch exists anywhere: all per-block
/// working memory is per-plugin-instance state sized to the
/// activation-time bounds (`MAX_CHANNELS`, `MAX_EVENTS_PER_BLOCK`).
/// `clear()` preserves capacity, so the bounded `push`/`extend`s in
/// `build_block_views`/`translate_events` never reallocate.
pub(crate) struct BlockScratch {
    /// Raw input channel addresses collected during pair iteration —
    /// drives the cross-alias check. Stored as `usize` (address only,
    /// never dereferenced) so the audio processor stays `Send`.
    input_ptrs: Vec<usize>,
    /// Raw output channel addresses — drives the duplicate-output
    /// and cross-alias checks. Capacity `MAX_CHANNELS`.
    output_ptrs: Vec<usize>,
    /// Neutral [`Event`]s translated from the input event list.
    /// Capacity `MAX_EVENTS_PER_BLOCK`.
    events: Vec<Event>,
    /// Alternating buffer for the merge sort inside
    /// [`translate_events`] — keeps the sort allocation-free on the
    /// RT path. Capacity `MAX_EVENTS_PER_BLOCK`.
    event_sort_scratch: Vec<Event>,
}

impl BlockScratch {
    /// Allocates scratch for the negotiated bounds (non-RT only —
    /// called from `activate`).
    pub(crate) fn new(max_channels: usize, max_events: usize) -> Self {
        Self {
            input_ptrs: Vec::with_capacity(max_channels),
            output_ptrs: Vec::with_capacity(max_channels),
            events: Vec::with_capacity(max_events),
            event_sort_scratch: Vec::with_capacity(max_events),
        }
    }
}

// ---------------------------------------------------------------------------
// Lifecycle driver
// ---------------------------------------------------------------------------

/// Per-instance lifecycle state encoded in one `AtomicU8` — the
/// neutral [`LifecycleState`] table driven from the shared plugin
/// state so both the main thread (init/activate/deactivate/destroy)
/// and the audio thread (start/process/stop) feed one machine without
/// blocking.
///
/// **RT contract:** every transition is one atomic load, one pure
/// table lookup and one atomic compare-exchange — never a wait. A
/// bounded CAS loop keeps an audio-thread `Process` racing a
/// main-thread `StopProcessing` from losing an update; contention
/// beyond [`MAX_CAS_ATTEMPTS`] means concurrent calls the CLAP
/// threading contract forbids — surfaced as a defined rejection.
#[derive(Debug)]
pub(crate) struct AtomicLifecycle {
    state: AtomicU8,
}

const STATE_CREATED: u8 = 0;
const STATE_INITIALIZED: u8 = 1;
const STATE_ACTIVATED: u8 = 2;
const STATE_PROCESSING: u8 = 3;
const STATE_FAILED: u8 = 4;
/// Terminal encoding — `Destroyed` and every unknown/future state
/// decode here so any event rejects (defined rejection, never a
/// silent accept).
const STATE_TERMINAL: u8 = 5;
const STATE_UNKNOWN: u8 = u8::MAX;

fn encode_state(state: LifecycleState) -> u8 {
    match state {
        LifecycleState::Created => STATE_CREATED,
        LifecycleState::Initialized => STATE_INITIALIZED,
        LifecycleState::Activated => STATE_ACTIVATED,
        LifecycleState::Processing => STATE_PROCESSING,
        LifecycleState::Failed => STATE_FAILED,
        LifecycleState::Destroyed => STATE_TERMINAL,
        _ => STATE_UNKNOWN,
    }
}

fn decode_state(raw: u8) -> LifecycleState {
    match raw {
        STATE_CREATED => LifecycleState::Created,
        STATE_INITIALIZED => LifecycleState::Initialized,
        STATE_ACTIVATED => LifecycleState::Activated,
        STATE_PROCESSING => LifecycleState::Processing,
        STATE_FAILED => LifecycleState::Failed,
        // Unknown encodings (including `STATE_TERMINAL` and the
        // sentinel) decode to the terminal state — every event
        // rejects.
        _ => LifecycleState::Destroyed,
    }
}

impl AtomicLifecycle {
    /// A fresh machine in [`LifecycleState::Created`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    pub(crate) const fn new() -> Self {
        Self {
            state: AtomicU8::new(STATE_CREATED),
        }
    }

    /// The current state — test introspection (production code
    /// observes state only through `transition`).
    ///
    /// **RT contract:** RT-suitable — atomic load, allocation-free.
    #[cfg(test)]
    pub(crate) fn state(&self) -> LifecycleState {
        decode_state(self.state.load(Ordering::Acquire))
    }

    /// Feeds `event` through the shared transition table atomically:
    /// the next state is computed against the *observed* state and
    /// committed with a compare-exchange, so an audio-thread `Process`
    /// racing a main-thread `StopProcessing` cannot lose an update.
    /// On an illegal pair the [`LifecycleError::IllegalTransition`]
    /// built by plugin-core's pure table is returned and the machine
    /// stays put.
    ///
    /// **RT contract:** RT-suitable — pure table lookup + a CAS loop
    /// bounded to [`MAX_CAS_ATTEMPTS`]; no allocation, no blocking.
    ///
    /// # Errors
    ///
    /// [`LifecycleError::IllegalTransition`] — the event is not legal
    /// from the current state, or CAS contention exceeded the bound.
    pub(crate) fn transition(
        &self,
        event: LifecycleEvent,
    ) -> Result<LifecycleState, LifecycleError> {
        let mut observed = self.state.load(Ordering::Acquire);
        for _ in 0..MAX_CAS_ATTEMPTS {
            let next = decode_state(observed).transition(event)?;
            match self.state.compare_exchange_weak(
                observed,
                encode_state(next),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(next),
                Err(current) => observed = current,
            }
        }
        Err(LifecycleError::IllegalTransition {
            from: decode_state(observed),
            event,
        })
    }
}

/// The parameter value mirror shared between the audio thread
/// (event-driven changes during `process`) and the main thread
/// (`params` extension `get_value`/`flush`). Stored as `f64` bits.
#[derive(Debug)]
pub(crate) struct ParamMirror {
    plain_bits: AtomicU64,
}

impl ParamMirror {
    /// A mirror at `plain` (the parameter's default at construction).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    pub(crate) const fn new(plain: f64) -> Self {
        Self {
            plain_bits: AtomicU64::new(plain.to_bits()),
        }
    }

    /// The mirrored plain value.
    ///
    /// **RT contract:** RT-suitable — atomic load, allocation-free.
    pub(crate) fn load(&self) -> f64 {
        f64::from_bits(self.plain_bits.load(Ordering::Acquire))
    }

    /// Records a new plain value (event- or flush-driven).
    ///
    /// **RT contract:** RT-suitable — atomic store, allocation-free.
    pub(crate) fn store(&self, plain: f64) {
        self.plain_bits.store(plain.to_bits(), Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// Activation-time config mapping
// ---------------------------------------------------------------------------

/// Maps the CLAP activation config to the neutral [`ProcessConfig`].
///
/// Defined rejections (→ `Err`, which the plugin reports as
/// `ActivateFailed`): non-finite or non-positive sample rate, rate
/// above `u32::MAX`, zero `max_frames`, or `min > max`. A fractional
/// rate negotiates the nearest integer Hz — CLAP delivers `f64`
/// rates and the neutral [`SampleRate`] is integer Hz (the same law
/// the VST3 fixture applies to `ProcessSetup.sampleRate`).
///
/// **RT contract:** non-RT only — called from `activate`.
pub(crate) fn to_process_config(
    audio_config: &PluginAudioConfiguration,
) -> Result<ProcessConfig, PluginError> {
    let rate = audio_config.sample_rate;
    if !rate.is_finite()
        || rate <= 0.0
        || rate > f64::from(u32::MAX)
        || audio_config.max_frames_count == 0
    {
        return Err(PluginError::Message("unsupported audio configuration"));
    }
    // Range-checked above: rounding stays inside u32 bounds.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let sample_rate = SampleRate::new(rate.round() as u32)
        .map_err(|_| PluginError::Message("unsupported sample rate"))?;
    ProcessConfig::new(
        sample_rate,
        audio_config.min_frames_count,
        audio_config.max_frames_count,
    )
    .map_err(|_| PluginError::Message("invalid frame bounds"))
}

// ---------------------------------------------------------------------------
// Per-block buffer views
// ---------------------------------------------------------------------------

/// How one port pair's channels relate to each other.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum PairMode {
    /// Every channel is `InputOutput` — distinct input/output regions.
    Paired,
    /// Every channel is `InPlace` — the output channel *is* the input
    /// (clack classified same-index pointer equality itself).
    InPlace,
}

/// The channel slice lists built for one block — borrowed from the
/// `PortPair`, so the views can never outlive the process call.
///
/// `InPlace` blocks keep `inputs` empty and put the shared in/out
/// region in `outputs` as `&mut` only — a `&[f32]` and a `&mut [f32]`
/// are never constructed over the same memory.
pub(crate) struct BlockViews<'p> {
    inputs: [&'p [f32]; MAX_CHANNELS],
    outputs: [&'p mut [f32]; MAX_CHANNELS],
    channel_count: usize,
    mode: PairMode,
}

impl BlockViews<'_> {
    /// Zero-channel views — used when the host supplies no port pair
    /// (a legal CLAP input: audio may be absent entirely).
    ///
    /// **RT contract:** RT-suitable — allocation-free.
    pub(crate) fn empty() -> Self {
        Self {
            inputs: [&[], &[]],
            outputs: [&mut [], &mut []],
            channel_count: 0,
            mode: PairMode::Paired,
        }
    }
}

/// Builds the borrowed input/output views for one block.
///
/// `outputs_scratch` is a caller-owned array living in the caller's
/// process scope: `PlanarBufferViewMut` is *invariant* in its lifetime
/// (`&'a mut [&'a mut [S]]`), so a `&mut [&'p mut [f32]]` borrowed from
/// `views` could never coexist with the shorter-lived input view —
/// each `'p` channel is therefore reborrowed through `outputs_scratch`
/// as `&'v mut [f32]`. Bounded `O(channels)` reborrow, no allocation.
///
/// For `InPlace` blocks the read-side view is empty: the output
/// channel carries the input data (see [`crate::gain::TestGainProcessor`]).
///
/// **RT contract:** RT-suitable — bounded scans, no allocation.
///
/// # Errors
///
/// `PluginError::Message` — heterogeneous channel frame lengths (can
/// only arise from an inconsistent clack pair).
pub(crate) fn planar_views<'v>(
    views: &'v mut BlockViews<'_>,
    outputs_scratch: &'v mut [&'v mut [f32]; MAX_CHANNELS],
) -> Result<(PlanarBufferView<'v>, PlanarBufferViewMut<'v>), PluginError> {
    let count = views.channel_count;
    // Reborrow each 'p mut as 'v — fills the caller-owned output list.
    for (slot, channel) in outputs_scratch
        .iter_mut()
        .zip(views.outputs[..count].iter_mut())
    {
        *slot = &mut **channel;
    }
    let input_view = match views.mode {
        PairMode::InPlace => PlanarBufferView::empty(),
        PairMode::Paired => PlanarBufferView::new(&views.inputs[..count])
            .map_err(|_| PluginError::Message("heterogeneous input channel frames"))?,
    };
    let output_view = PlanarBufferViewMut::new(&mut outputs_scratch[..count])
        .map_err(|_| PluginError::Message("heterogeneous output channel frames"))?;
    Ok((input_view, output_view))
}

/// Per-channel raw-pointer validation for one side of a port pair —
/// checks the raw pointer list *before* clack builds any channel
/// slice, so a null or unaligned channel pointer can never become a
/// Rust slice. All pointer reads are through clack's safe
/// `raw_data()` accessor — no `unsafe` in this crate.
fn validate_channel_pointers(ptrs: &[*mut f32], frames_count: u32) -> Result<(), PluginError> {
    for ptr in ptrs {
        // clack's slice constructor tolerates null only when the
        // length is zero — a null pointer with frames > 0 would be UB.
        if ptr.is_null() && frames_count > 0 {
            return Err(PluginError::Message("null channel pointer in port"));
        }
        if !ptr.is_null() && !ptr.is_aligned() {
            return Err(PluginError::Message("unaligned channel pointer in port"));
        }
    }
    Ok(())
}

/// Collects the raw channel addresses into `scratch` and runs every
/// pointer-granularity check: null/alignment validation plus the
/// range-overlap alias verdict. Called before `channel_pair` so no
/// aliased `&`/`&mut` pair is ever constructed.
///
/// **RT contract:** RT-suitable — bounded `O(channels²)` scans
/// (channel counts are ≤ `MAX_CHANNELS`), no allocation.
///
/// # Errors
///
/// `PluginError::Message` — null/unaligned pointers, missing channel
/// buffers, or any overlapping channel range.
fn collect_and_check_channel_pointers(
    pair: &mut PortPair<'_>,
    scratch: &mut BlockScratch,
    frames_count: u32,
) -> Result<(), PluginError> {
    scratch.input_ptrs.clear();
    scratch.output_ptrs.clear();
    if let Some(input) = pair.input() {
        match input.channels() {
            Ok(sample) => {
                if let Some(channels) = sample.as_f32() {
                    validate_channel_pointers(channels.raw_data(), frames_count)?;
                    scratch
                        .input_ptrs
                        .extend(channels.raw_data().iter().map(|ptr| ptr.addr()));
                }
                // f64-only input port: the pair-level match below
                // either pairs it with f32 (mismatch → reject) or
                // leaves it f64-only (reject) — defined.
            }
            Err(_) => {
                return Err(PluginError::Message("input port without channel buffers"));
            }
        }
    }
    if let Some(mut output) = pair.output() {
        match output.channels() {
            Ok(sample) => {
                if let Some(channels) = sample.as_f32() {
                    validate_channel_pointers(channels.raw_data(), frames_count)?;
                    scratch
                        .output_ptrs
                        .extend(channels.raw_data().iter().map(|ptr| ptr.addr()));
                }
            }
            Err(_) => {
                return Err(PluginError::Message("output port without channel buffers"));
            }
        }
    }

    // Range-overlap alias check on the collected raw addresses. Every
    // channel range is `[ptr, ptr + frames_count*4)`. A same-index
    // input/output match is the legal in-place shape (clack classifies
    // it `InPlace` and only the mutable slice is used); *any other*
    // intersecting pair — cross-index, partial, or output↔output —
    // would put two `&mut` (or a `&`/`&mut` pair) over one region and
    // rejects the block. Inputs may alias each other (shared `&`
    // borrows only). `saturating_add` keeps a hostile pointer near the
    // address-space top from wrapping the range empty.
    let bytes = usize::try_from(frames_count)
        .unwrap_or(usize::MAX)
        .saturating_mul(size_of::<f32>());
    for (j, &out_start) in scratch.output_ptrs.iter().enumerate() {
        let out_end = out_start.saturating_add(bytes);
        let overlaps = |start: usize, end: usize| start < out_end && out_start < end;
        for &earlier in &scratch.output_ptrs[..j] {
            if overlaps(earlier, earlier.saturating_add(bytes)) {
                return Err(PluginError::Message("overlapping output channel regions"));
            }
        }
        for (i, &in_start) in scratch.input_ptrs.iter().enumerate() {
            let in_end = in_start.saturating_add(bytes);
            if overlaps(in_start, in_end) && !(i == j && in_start == out_start) {
                return Err(PluginError::Message("input/output channel overlap"));
            }
        }
    }
    Ok(())
}

/// Builds the borrowed channel slice lists for port pair `index` of
/// one block.
///
/// **RT contract:** RT-suitable — bounded `O(channels²)` scans
/// (channel counts are ≤ `MAX_CHANNELS`), no allocation.
///
/// # Errors
///
/// `PluginError::Message` — any malformed boundary input: excess
/// channels, null/unaligned pointers, `f64`-only or mismatched pairs,
/// asymmetric pairs, mixed in-place classification or any
/// input↔output / output↔output aliasing.
pub(crate) fn build_block_views<'p>(
    pair: &mut PortPair<'p>,
    scratch: &mut BlockScratch,
    max_frames: u32,
) -> Result<BlockViews<'p>, PluginError> {
    // Channel-count bound vs negotiated stereo capacity.
    if pair.channel_pair_count() > MAX_CHANNELS {
        return Err(PluginError::Message(
            "channel count exceeds negotiated capacity",
        ));
    }
    let frames_count = pair.frames_count();
    // The buffer-level `frames_count` is a separate host-supplied
    // field from the process-level one (already bounded by the
    // caller): bound it too so channel slices can never be built
    // wider than the negotiated maximum.
    if frames_count > max_frames {
        return Err(PluginError::Message(
            "buffer frames_count exceeds negotiated maximum",
        ));
    }

    // Per-channel null/alignment pre-validation, raw-pointer
    // collection and the range-overlap alias check — all before clack
    // builds any channel slice inside `channel_pair`, so an aliasing
    // verdict is reached from the raw pointer lists alone (a `usize`
    // address is never dereferenced here).
    collect_and_check_channel_pointers(pair, scratch, frames_count)?;

    // Paired channels — f32 canonical path. `SampleType::Both` uses
    // the f32 side; f64-only and mismatched pairs are defined
    // rejections (never reinterpreted as f32).
    let mut paired = match pair.channels() {
        Ok(sample) => match sample {
            SampleType::F32(paired) | SampleType::Both(paired, _) => paired,
            SampleType::F64(_) => {
                return Err(PluginError::Message("f64-only port pair rejected"));
            }
        },
        Err(BufferError::InvalidChannelBuffer) => {
            // Both data32 and data64 null on a port — the port has no
            // backing store at all: malformed, not silent.
            return Err(PluginError::Message("port without channel buffers"));
        }
        Err(BufferError::MismatchedBufferPair) => {
            return Err(PluginError::Message("mismatched f32/f64 port pair"));
        }
    };

    // Classify each channel pair; collect slices. The raw pointer
    // lists were already collected and range-checked above.
    let mut views = BlockViews::empty();
    let mut mode = None;
    let count = paired.channel_pair_count();
    for index in 0..count {
        match paired.channel_pair(index) {
            Some(ChannelPair::InputOutput(input, output)) => {
                if matches!(mode, Some(PairMode::InPlace)) {
                    return Err(PluginError::Message(
                        "mixed in-place/paired channel classification",
                    ));
                }
                mode = Some(PairMode::Paired);
                views.inputs[index] = input;
                views.outputs[index] = output;
            }
            Some(ChannelPair::InPlace(io)) => {
                if matches!(mode, Some(PairMode::Paired)) {
                    return Err(PluginError::Message(
                        "mixed in-place/paired channel classification",
                    ));
                }
                mode = Some(PairMode::InPlace);
                // Only the mutable slice is collected — the input view
                // stays empty so no &T/&mut T alias is ever built.
                views.outputs[index] = io;
            }
            Some(ChannelPair::InputOnly(_) | ChannelPair::OutputOnly(_)) => {
                return Err(PluginError::Message("asymmetric channel pair rejected"));
            }
            None => break,
        }
    }
    let Some(pair_mode) = mode else {
        return Err(PluginError::Message("port pair without usable channels"));
    };
    views.mode = pair_mode;
    views.channel_count = count;
    Ok(views)
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Translates one `UnknownEvent` into a neutral [`Event`] when it is
/// a parameter-value event for `param`.
///
/// Defined skips (the event simply does not enter the list):
/// non-`ParamValue` event kinds (notes/MIDI/transport — transport
/// state arrives via `Process::transport`, which this fixture does
/// not consume), `ParamValue` for another parameter id,
/// `CLAP_INVALID_ID`, and non-finite values (`ParamInfo::normalize`
/// NaN policy). Per-note `pckn` targeting is not part of the neutral
/// model and is applied as if global — recorded as a suite input.
pub(crate) fn translate_param_event(unknown: &UnknownEvent, param: &ParamInfo) -> Option<Event> {
    let Some(CoreEventSpace::ParamValue(event)) = unknown.as_core_event() else {
        return None;
    };
    let param_id = event.param_id()?;
    if param_id.get() != param.id.0 {
        return None;
    }
    let value = param.normalize(event.value()).ok()?;
    Some(Event::new(
        unknown.header().time(),
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(param_id.get()),
            value,
        }),
    ))
}

// ---------------------------------------------------------------------------
// Instrument events (notes / raw MIDI) and transport
// ---------------------------------------------------------------------------

/// CLAP's PPQN is not carried on the wire — the `test_instrument`
/// fixture fixes this resolution for its musical-time mapping
/// (`position_ticks = round(song_pos_beats * CLAP_PPQN)`).
pub(crate) const CLAP_PPQN: u16 = 960;

/// Maps a CLAP `[0.0, 1.0]` velocity/pressure to the neutral
/// canonical `u32` scale-to-max domain — `round(v * u32::MAX)` after
/// clamping non-finite/out-of-range input to the defined endpoints.
fn canonical_u32(unit: f64) -> u32 {
    let clamped = if unit.is_finite() {
        unit.clamp(0.0, 1.0)
    } else {
        0.0
    };
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let scaled = (clamped * f64::from(u32::MAX)).round() as u32;
    scaled
}

/// Builds the neutral [`MidiEvent`] routing for one CLAP event —
/// group `ZERO` (MIDI 1.0 traffic), the concrete channel from the
/// PCKN match or the MIDI status byte.
fn midi_envelope(channel: u8) -> Option<(MidiGroup, MidiChannel)> {
    let channel = MidiChannel::new(channel).ok()?;
    Some((MidiGroup::ZERO, channel))
}

/// Extracts the concrete key from a [`Match`] — `Match::All` has no
/// single key and is a defined skip.
fn specific_u8(v: Match<u16>) -> Option<u8> {
    match v {
        Match::Specific(k) if k <= 127 => u8::try_from(k).ok(),
        _ => None,
    }
}

/// Translates one note/core event into a neutral [`Event`] carrying
/// [`EventKind::Midi`] — used by `test_instrument`'s event surface.
///
/// Defined skips: `Match::All`/out-of-range PCKN fields (no concrete
/// key/channel), non-finite velocities (mapped to `0`), event kinds
/// the fixture does not consume (note choke/end, expression,
/// note-specific controllers).
fn translate_note_event(unknown: &UnknownEvent) -> Option<Event> {
    let (kind, channel) = match unknown.as_core_event()? {
        CoreEventSpace::NoteOn(note) => {
            let key = specific_u8(note.key())?;
            let channel = match note.channel() {
                Match::Specific(c) if c <= 15 => u8::try_from(c).ok()?,
                _ => return None,
            };
            (
                MidiKind::NoteOn {
                    key: NoteKey::new(key).ok()?,
                    velocity: canonical_u32(note.velocity()),
                },
                channel,
            )
        }
        CoreEventSpace::NoteOff(note) => {
            let key = specific_u8(note.key())?;
            let channel = match note.channel() {
                Match::Specific(c) if c <= 15 => u8::try_from(c).ok()?,
                _ => return None,
            };
            (
                MidiKind::NoteOff {
                    key: NoteKey::new(key).ok()?,
                    velocity: canonical_u32(note.velocity()),
                },
                channel,
            )
        }
        CoreEventSpace::Midi(midi) => return translate_raw_midi(midi, unknown.header().time()),
        _ => return None,
    };
    let (group, channel) = midi_envelope(channel)?;
    Some(Event::new(
        unknown.header().time(),
        EventKind::Midi(MidiEvent {
            group,
            channel,
            kind,
        }),
    ))
}

/// Translates a raw MIDI 1.0 `clap_event_midi` (3 data bytes) into a
/// neutral [`Event`]. Only channel-voice messages map; system
/// messages and running-status continuations (status `< 0x80`) are
/// defined skips.
fn translate_raw_midi(midi: &ClapMidiEvent, time: u32) -> Option<Event> {
    let [status, d1, d2] = midi.data();
    // Every channel-voice status consumes `d1` as a 7-bit data byte;
    // `d2` is data for all but program change / channel pressure
    // (where it is ignored pad). Out-of-range bytes are malformed
    // MIDI 1.0 — a defined skip, never a wrapped canonical value.
    let d2_is_data = matches!(status & 0xF0, 0x80 | 0x90 | 0xA0 | 0xB0 | 0xE0);
    if status < 0x80 || d1 > 0x7F || (d2_is_data && d2 > 0x7F) {
        return None;
    }
    let kind = match status & 0xF0 {
        0x80 => MidiKind::NoteOff {
            key: NoteKey::new(d1).ok()?,
            velocity: MidiValue::from_u7(d2),
        },
        0x90 => MidiKind::NoteOn {
            key: NoteKey::new(d1).ok()?,
            velocity: MidiValue::from_u7(d2),
        },
        0xA0 => MidiKind::PolyPressure {
            key: NoteKey::new(d1).ok()?,
            pressure: MidiValue::from_u7(d2),
        },
        0xB0 => MidiKind::ControlChange {
            controller: MidiController::new(d1).ok()?,
            value: MidiValue::from_u7(d2),
        },
        0xC0 => MidiKind::ProgramChange {
            program: MidiProgram::new(d1).ok()?,
        },
        0xD0 => MidiKind::ChannelPressure {
            pressure: MidiValue::from_u7(d1),
        },
        0xE0 => {
            // 14-bit bend `0..=16383`, center `8192` → canonical
            // signed scale-to-max via the neutral `MidiValue` law.
            let raw = u16::from(d1) | (u16::from(d2) << 7);
            MidiKind::PitchBend {
                value: MidiValue::bend_from_u14(raw),
            }
        }
        _ => return None,
    };
    let (group, channel) = midi_envelope(status & 0x0F)?;
    Some(Event::new(
        time,
        EventKind::Midi(MidiEvent {
            group,
            channel,
            kind,
        }),
    ))
}

/// Translates one `UnknownEvent` into a neutral [`Event`] for
/// `test_instrument`'s input surface — parameter changes plus
/// notes/raw MIDI. Transport arrives via `Process::transport`, not
/// the event list.
pub(crate) fn translate_instrument_event(
    unknown: &UnknownEvent,
    param: &ParamInfo,
) -> Option<Event> {
    translate_param_event(unknown, param).or_else(|| translate_note_event(unknown))
}

/// Maps the CLAP `Process::transport` snapshot to the neutral
/// [`TransportInfo`]. `None` input → `None` output (no transport
/// declared for the block).
///
/// Mapping law: `IS_PLAYING` → [`TransportStatus::Playing`],
/// otherwise `Stopped` (CLAP has no paused state). Position is the
/// seconds timeline projected onto the sample-rate [`TimeBase`];
/// musical info is present only when the host supplies both the
/// beats timeline and a positive tempo — `position_ticks =
/// round(song_pos_beats * CLAP_PPQN)`, `bpm_milli = round(tempo *
/// 1000)`. An active loop maps to a [`MediaRange`] over the seconds
/// loop bounds. Missing/invalid fields degrade to `None` fields —
/// the fixture's click oracle then stays silent, matching the VST3
/// fixture's missing-musical-info law.
///
/// **RT contract:** RT-suitable — bounded arithmetic, no allocation.
pub(crate) fn transport_info(
    transport: Option<&TransportEvent>,
    sample_rate: u32,
) -> Option<TransportInfo> {
    let event = transport?;
    let flags = event.flags;
    let status = if flags.contains(TransportFlags::IS_PLAYING) {
        TransportStatus::Playing
    } else {
        TransportStatus::Stopped
    };
    let Ok(base) = TimeBase::sample_rate(sample_rate) else {
        return None;
    };
    // song_pos_seconds → samples at the negotiated rate; a negative
    // or non-finite position clamps to 0 (defined). `FixedPoint`
    // converts through `to_float` — the CLAP fixed-point decimal.
    let seconds = event.song_pos_seconds.to_float();
    let seconds = if seconds.is_finite() {
        seconds.max(0.0)
    } else {
        0.0
    };
    #[expect(clippy::cast_possible_truncation)]
    let position_ticks = (seconds * f64::from(sample_rate)).round() as i64;
    let position = MediaTime::new(position_ticks, base);

    let mut info = TransportInfo::new(status, position);

    if flags.contains(TransportFlags::IS_LOOP_ACTIVE)
        && flags.contains(TransportFlags::HAS_SECONDS_TIMELINE)
    {
        let start = event.loop_start_seconds.to_float().max(0.0);
        let end = event.loop_end_seconds.to_float().max(start);
        #[expect(clippy::cast_possible_truncation)]
        let start_ticks = (start * f64::from(sample_rate)).round() as i64;
        #[expect(clippy::cast_possible_truncation)]
        let len_ticks = ((end - start) * f64::from(sample_rate)).round() as i64;
        if len_ticks > 0 {
            info = info.with_loop_range(MediaRange::new(
                MediaTime::new(start_ticks, base),
                MediaDuration::new(len_ticks, base),
            ));
        }
    }

    if flags.contains(TransportFlags::HAS_BEATS_TIMELINE)
        && flags.contains(TransportFlags::HAS_TEMPO)
        && event.tempo.is_finite()
        && event.tempo > 0.0
    {
        let beats = event.song_pos_beats.to_float();
        let bar_start = event.bar_start.to_float();
        #[expect(clippy::cast_possible_truncation)]
        let position_ticks = (beats * f64::from(CLAP_PPQN)).round() as i64;
        #[expect(clippy::cast_possible_truncation)]
        let bar_start_tick = (bar_start * f64::from(CLAP_PPQN)).round() as i64;
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bpm_milli = (event.tempo * 1000.0).round() as u32;
        let (sig_num, sig_den) = if flags.contains(TransportFlags::HAS_TIME_SIGNATURE) {
            (
                u8::try_from(event.time_signature_numerator).unwrap_or(0),
                u8::try_from(event.time_signature_denominator).unwrap_or(0),
            )
        } else {
            (0, 0)
        };
        info = info.with_musical(MusicalTimeInfo {
            position_ticks,
            bar_start_tick,
            bpm_milli,
            ppqn: CLAP_PPQN,
            sig_num,
            sig_den,
        });
    }
    Some(info)
}

/// Translates the input event list into a validated neutral
/// [`EventList`] borrowing `scratch.events` — allocation-free (pushes
/// stay within the pre-allocated capacity; bounds are checked first).
/// `translate` is the fixture's event-kind policy: `test_gain`-family
/// plugins pass [`translate_param_event`], `test_instrument` passes
/// [`translate_instrument_event`].
///
/// CLAP requires sorted input events; if a host still delivers
/// unsorted ones, the list is re-ordered with
/// [`sort_events_by_frame_offset`] over `scratch.event_sort_scratch` —
/// the shared stable merge sort (allocation-free over the
/// pre-allocated buffer), so the relative order of same-offset events
/// is preserved. Events with out-of-block offsets are dropped
/// individually — the shared drop policy, identical to the VST3
/// `test_gain` slice.
///
/// **RT contract:** RT-suitable — bounded `O(n + n log n)` work over
/// at most `MAX_EVENTS_PER_BLOCK` events, allocation-free.
///
/// # Errors
///
/// `PluginError::Message` — event count exceeds
/// `MAX_EVENTS_PER_BLOCK` (a defined rejection).
pub(crate) fn translate_events<'a>(
    input: &InputEvents<'_>,
    scratch: &'a mut BlockScratch,
    block_frames: u32,
    translate: impl Fn(&UnknownEvent) -> Option<Event>,
) -> Result<EventList<'a>, PluginError> {
    if input.len() as usize > MAX_EVENTS_PER_BLOCK {
        return Err(PluginError::Message("event count exceeds per-block bound"));
    }
    let events = &mut scratch.events;
    events.clear();
    let mut sorted = true;
    let mut previous = 0_u32;
    for unknown in input.iter() {
        let Some(event) = translate(unknown) else {
            continue;
        };
        // Defined skip: an out-of-block offset drops just that event —
        // the shared drop policy (never clamped onto a wrong frame),
        // identical to the VST3 test_gain.
        if event.frame_offset() >= block_frames.max(1) {
            continue;
        }
        if event.frame_offset() < previous {
            sorted = false;
        }
        previous = event.frame_offset();
        events.push(event);
    }
    if !sorted {
        // `event_sort_scratch` is filled to the event count so it
        // satisfies the merge sort's `len >= events.len()` precondition;
        // capacity was reserved at `activate`.
        scratch.event_sort_scratch.clear();
        scratch
            .event_sort_scratch
            .extend_from_slice(&scratch.events);
        sort_events_by_frame_offset(&mut scratch.events, &mut scratch.event_sort_scratch);
    }
    // The filtered list always satisfies `EventList`'s bounds; a
    // failure means the invariant broke — a defined reject, never a
    // silently-empty event set.
    EventList::new(scratch.events.as_slice(), block_frames).map_err(|_| {
        debug_assert!(false, "filtered events must satisfy EventList");
        PluginError::Message("event list invariant violated")
    })
}

// ---------------------------------------------------------------------------
// Status mapping
// ---------------------------------------------------------------------------

/// Maps a neutral [`ProcessResult`] to the CLAP process status.
///
/// `Continue` + `tail_frames > 0` maps to [`ClapProcessStatus::Tail`]
/// — the CLAP-idiomatic "meaningful tail remains" hint.
/// `ProcessStatus::Error` and unknown neutral variants map to `Err`
/// → `CLAP_PROCESS_ERROR`.
///
/// **RT contract:** RT-suitable — pure match, allocation-free.
///
/// # Errors
///
/// `PluginError::Message` — the neutral status is `Error` or a
/// variant this fixture does not map.
pub(crate) fn map_process_status(result: ProcessResult) -> Result<ClapProcessStatus, PluginError> {
    match result.status {
        ProcessStatus::Continue if result.tail_frames > 0 => Ok(ClapProcessStatus::Tail),
        ProcessStatus::Continue => Ok(ClapProcessStatus::Continue),
        ProcessStatus::Sleep => Ok(ClapProcessStatus::Sleep),
        _ => Err(PluginError::Message(
            "processor returned an unmapped status",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clack_common::events::Pckn;
    use clack_common::events::event_types::{NoteOffEvent, NoteOnEvent};
    use clack_common::events::io::EventBuffer;

    fn pckn() -> Pckn {
        Pckn::new(
            Match::All,
            Match::Specific(0),
            Match::Specific(69),
            Match::All,
        )
    }

    /// Structured CLAP note events must surface as neutral
    /// `EventKind::Midi` — `test_instrument`'s only note input path.
    #[test]
    fn note_on_and_off_translate_to_neutral_midi() {
        let mut buffer = EventBuffer::new();
        buffer.push(&NoteOnEvent::new(0, pckn(), 1.0));
        buffer.push(&NoteOffEvent::new(8, pckn(), 0.0));
        let input = buffer.as_input();
        let mut iter = input.iter();
        let on = translate_note_event(iter.next().unwrap()).unwrap();
        let off = translate_note_event(iter.next().unwrap()).unwrap();
        assert!(matches!(
            on.kind(),
            EventKind::Midi(m) if matches!(m.kind, MidiKind::NoteOn { .. })
        ));
        assert!(matches!(
            off.kind(),
            EventKind::Midi(m) if matches!(m.kind, MidiKind::NoteOff { .. })
        ));
    }

    /// Raw MIDI 1.0 bytes map channel-voice messages to the same
    /// neutral kinds — CC7 included (`test_instrument`'s volume).
    #[test]
    fn raw_midi_bytes_translate_to_neutral_midi() {
        let mut buffer = EventBuffer::new();
        // CC7 = 0 on channel 0.
        buffer.push(&ClapMidiEvent::new(0, 0, [0xB0, 7, 0]));
        let input = buffer.as_input();
        let event = translate_note_event(input.iter().next().unwrap()).unwrap();
        assert!(matches!(
            event.kind(),
            EventKind::Midi(m)
                if matches!(m.kind, MidiKind::ControlChange { controller, value }
                    if controller.get() == 7 && value == 0)
        ));
    }

    /// CLAP delivers `sample_rate` as `f64` — fractional rates are
    /// legal host input (the external `clap-validator` exercises
    /// 1234.5678 Hz). The neutral `SampleRate` is integer Hz, so the
    /// config negotiates the nearest Hz — the same law the VST3
    /// fixture applies to `ProcessSetup.sampleRate`.
    #[test]
    fn fractional_sample_rate_rounds_to_nearest_hz() {
        let audio_config = PluginAudioConfiguration {
            sample_rate: 1234.5678,
            min_frames_count: 1,
            max_frames_count: 512,
        };
        let config = to_process_config(&audio_config).unwrap();
        assert_eq!(config.sample_rate().get(), 1235);
    }

    /// Dense note streams above the old 256-event bound must still
    /// translate — `clap-validator`'s note generators emit several
    /// hundred events per 512-frame block. The bound exists to keep
    /// the RT scan finite, not to reject realistic host density.
    #[test]
    fn dense_event_stream_within_bound_translates() {
        let mut buffer = EventBuffer::new();
        for i in 0..300_u32 {
            buffer.push(&NoteOnEvent::new(i % 512, pckn(), 1.0));
        }
        let input = buffer.as_input();
        let mut scratch = BlockScratch::new(MAX_CHANNELS, MAX_EVENTS_PER_BLOCK);
        let list = translate_events(&input, &mut scratch, 512, translate_note_event).unwrap();
        assert_eq!(list.len(), 300);
    }
}
