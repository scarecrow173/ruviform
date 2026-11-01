//! The ABI→neutral translation layer of the CLAP adapter.
//!
//! Everything here is deliberately factored into pure functions and
//! small value types so the whole per-block path — boundary
//! validation, channel-pair collection, view construction, event
//! translation, transport/status mapping and the lifecycle driver —
//! can be exercised by `cargo test` without the raw `clap_plugin`
//! ABI. The `plugin` module is a thin shell over these helpers.
//!
//! ## Boundary validation contract (spec Requirements)
//!
//! Defined behavior for malformed host input — every case maps to a
//! `PluginError::Message` rejection (→ `CLAP_PROCESS_ERROR`) or an
//! explicit skip, never UB and never a silent channel-index shift:
//!
//! | Input | Handling |
//! | --- | --- |
//! | `frames_count` > negotiated `max_frames` | reject block |
//! | channel count > negotiated stereo capacity | reject block |
//! | `f64`-only port pair, or mismatched f32/f64 pair | reject block |
//! | `data32`/`data64` both null on a port | reject block |
//! | null channel pointer with `frames > 0`, or unaligned | reject block |
//! | `InputOnly`/`OutputOnly`/mixed `InPlace`+`InputOutput` | reject block |
//! | partial alias `out[i] == in[j]` or duplicate outputs | reject block |
//! | `constant_mask` | informational only — read, not acted on |
//!
//! Null channel pointers are legal *only* for zero-frame blocks: clack
//! builds empty slices for null+0, and a slice from a null pointer
//! with `frames > 0` would be UB — so the raw pointer list is checked
//! *before* any channel slice exists.
//!
//! ## RT contract
//!
//! Every `pub(crate)` fn in this module is RT-suitable — bounded
//! scans over `channels`/`events`, no allocation, no blocking.
//! [`BlockScratch`] is allocated once per instance at `activate`.

use core::mem::size_of;
use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use clack_common::events::UnknownEvent;
use clack_common::events::event_types::{TransportEvent, TransportFlags};
use clack_common::events::io::InputEvents;
use clack_common::events::spaces::CoreEventSpace;
use clack_plugin::plugin::PluginError;
use clack_plugin::process::audio::{BufferError, ChannelPair, PortPair, SampleType};
use clack_plugin::process::{PluginAudioConfiguration, ProcessStatus as ClapProcessStatus};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_media_core::{
    MediaDuration, MediaRange, MediaTime, MusicalTimeInfo, PPQN, TimeBase, TransportStatus,
};
use ruviform_plugin_core::{
    Event, EventKind, EventList, LifecycleError, LifecycleEvent, LifecycleState, ParamEvent,
    ParamId, ParamInfo, ProcessConfig, ProcessResult, ProcessStatus, TransportInfo,
    sort_events_by_frame_offset,
};

/// Channel capacity negotiated by this `PoC`: one stereo port pair.
pub(crate) const MAX_CHANNELS: usize = 2;

/// Maximum input events translated per block. Hosts may attach
/// arbitrary event counts; the per-block scan must stay bounded, so
/// oversized lists are a defined rejection rather than unbounded work.
pub(crate) const MAX_EVENTS_PER_BLOCK: usize = 256;

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
/// `clear()` preserves capacity, so the bounded `push`es in
/// `build_block_views`/`translate_events` never reallocate.
pub(crate) struct BlockScratch {
    /// Raw input channel addresses collected during pair iteration —
    /// drives the cross-alias check. Stored as `usize` (address only,
    /// never dereferenced) so the audio processor stays `Send`.
    /// Capacity `MAX_CHANNELS`.
    input_ptrs: Vec<usize>,
    /// Raw output channel addresses — drives the duplicate-output
    /// check and the cross-alias check. Capacity `MAX_CHANNELS`.
    output_ptrs: Vec<usize>,
    /// Neutral [`Event`]s translated from the input event list.
    /// Capacity `MAX_EVENTS_PER_BLOCK`.
    events: Vec<Event>,
    /// Alternating buffer for the merge sort inside
    /// [`translate_events`] — keeps the sort allocation-free on the RT
    /// path. Capacity `MAX_EVENTS_PER_BLOCK`.
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
/// table lookup and one atomic store — never a wait or a retry loop.
/// Sequential lifecycle calls observe a single correct machine.
/// Concurrent lifecycle calls are a host protocol violation outside
/// CLAP's thread model; even then the cell can only hold valid
/// encodings (no torn state — `AtomicU8` stores are atomic), so a
/// race can reorder *which* transition wins but every subsequent
/// event is still checked by the honest table.
#[derive(Debug)]
pub(crate) struct AtomicLifecycle {
    state: AtomicU8,
}

const STATE_CREATED: u8 = 0;
const STATE_INITIALIZED: u8 = 1;
const STATE_ACTIVATED: u8 = 2;
const STATE_PROCESSING: u8 = 3;
const STATE_DESTROYED: u8 = 4;
const STATE_FAILED: u8 = 5;
/// Forward-compat sentinel: a future `LifecycleState` variant encodes
/// here and rejects every event (defined rejection, never a silent
/// accept).
const STATE_UNKNOWN: u8 = u8::MAX;

fn encode_state(state: LifecycleState) -> u8 {
    match state {
        LifecycleState::Created => STATE_CREATED,
        LifecycleState::Initialized => STATE_INITIALIZED,
        LifecycleState::Activated => STATE_ACTIVATED,
        LifecycleState::Processing => STATE_PROCESSING,
        LifecycleState::Destroyed => STATE_DESTROYED,
        LifecycleState::Failed => STATE_FAILED,
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
        // Unknown encodings (including `STATE_DESTROYED` and the
        // sentinel) decode to the terminal state — every event rejects.
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
    /// racing a main-thread `StopProcessing` cannot lose an update
    /// (a plain load→store pair would let the later store resurrect a
    /// stale state and wedge the machine). On an illegal pair the
    /// [`LifecycleError::IllegalTransition`] built by plugin-core's
    /// pure table is returned and the machine stays put.
    ///
    /// **RT contract:** RT-suitable — pure table lookup + a CAS loop
    /// bounded to [`MAX_CAS_ATTEMPTS`]; no allocation, no blocking.
    /// Contention beyond the cap means concurrent calls the CLAP
    /// threading contract forbids — surfaced as a defined rejection.
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
    gain_plain_bits: AtomicU64,
}

impl ParamMirror {
    /// A mirror at `plain` (the parameter's default at activation).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    pub(crate) const fn new(plain: f64) -> Self {
        Self {
            gain_plain_bits: AtomicU64::new(plain.to_bits()),
        }
    }

    /// The mirrored plain value.
    ///
    /// **RT contract:** RT-suitable — atomic load, allocation-free.
    pub(crate) fn load(&self) -> f64 {
        f64::from_bits(self.gain_plain_bits.load(Ordering::Acquire))
    }

    /// Records a new plain value (event- or flush-driven).
    ///
    /// **RT contract:** RT-suitable — atomic store, allocation-free.
    pub(crate) fn store(&self, plain: f64) {
        self.gain_plain_bits
            .store(plain.to_bits(), Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// Activation-time config mapping
// ---------------------------------------------------------------------------

/// Maps the CLAP activation config to the neutral [`ProcessConfig`].
///
/// Defined rejections (→ `Err`, which the adapter reports as
/// `ActivateFailed`): non-finite or non-positive sample rate,
/// fractional rate (the neutral `SampleRate` is integer Hz), rate
/// above `u32::MAX`, zero `max_frames`, or `min > max`.
///
/// **RT contract:** non-RT only — called from `activate`.
pub(crate) fn to_process_config(
    audio_config: &PluginAudioConfiguration,
) -> Result<ProcessConfig, PluginError> {
    let rate = audio_config.sample_rate;
    if !rate.is_finite()
        || rate <= 0.0
        || rate > f64::from(u32::MAX)
        || rate.fract() != 0.0
        || audio_config.max_frames_count == 0
    {
        return Err(PluginError::Message("unsupported audio configuration"));
    }
    // Range-checked above: the cast is a defined truncation of an
    // integer-valued f64 inside u32 bounds.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let sample_rate = SampleRate::new(rate as u32)
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
/// region in `outputs` as `&mut` only — a `&[f32]` and a
/// `&mut [f32]` are never constructed over the same memory.
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
    /// **RT contract:** RT-suitable — `const`-shape, allocation-free.
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
/// channel carries the input data (adapter contract, see
/// [`GainProcessor`](crate::GainProcessor)).
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
/// `PluginError::Message` — any malformed boundary input (see module
/// table): excess channels, null/unaligned pointers, `f64`-only or
/// mismatched pairs, asymmetric pairs, mixed in-place classification
/// or any input↔output / output↔output aliasing.
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
/// a parameter-value event for `param` — the only event kind this
/// `PoC` consumes.
///
/// Defined skips (the event simply does not enter the list):
/// non-param-value event kinds (notes/MIDI/transport events —
/// transport state arrives via `Process::transport`), `ParamValue`
/// for another parameter id, `CLAP_INVALID_ID`, and non-finite
/// values (`ParamInfo::normalize` NaN policy). Per-note `pckn`
/// targeting is not part of the neutral model and is applied as if
/// global — recorded as an ADR input.
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

/// Translates the input event list into a validated neutral
/// [`EventList`] borrowing `scratch.events` — allocation-free (pushes
/// stay within the pre-allocated capacity; bounds are checked first).
///
/// CLAP requires sorted input events; if a host still delivers
/// unsorted ones, the list is re-ordered with
/// [`sort_events_by_frame_offset`] over `scratch.event_sort_scratch` —
/// the shared stable merge sort (allocation-free over the
/// pre-allocated buffer), so the relative order of same-offset events
/// is preserved. Events with out-of-block offsets are dropped
/// individually — the shared drop policy, identical to the VST3
/// adapter.
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
    param: &ParamInfo,
    block_frames: u32,
) -> Result<EventList<'a>, PluginError> {
    if input.len() as usize > MAX_EVENTS_PER_BLOCK {
        return Err(PluginError::Message("event count exceeds per-block bound"));
    }
    let events = &mut scratch.events;
    events.clear();
    let mut sorted = true;
    let mut previous = 0_u32;
    for unknown in input.iter() {
        let Some(event) = translate_param_event(unknown, param) else {
            continue;
        };
        // Defined skip: an out-of-block offset drops just that event —
        // the shared drop policy (never clamped onto a wrong frame),
        // identical to the VST3 adapter.
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
// Transport
// ---------------------------------------------------------------------------

/// Converts a CLAP [`BeatTime`] (i64 fixed-point scaled by
/// `CLAP_BEATTIME_FACTOR` = 2³¹) to PPQN-960 ticks:
/// `bits × 960 / 2³¹`, rounded half away from zero — the exact
/// inverse of `tick × 2³¹ / 960`.
///
/// **RT contract:** RT-suitable — pure math, allocation-free.
fn beats_to_ticks(beats: clack_common::utils::BeatTime) -> i64 {
    const HALF: i128 = (clack_common::utils::BeatTime::FACTOR / 2) as i128;
    const FACTOR: i128 = clack_common::utils::BeatTime::FACTOR as i128;
    let num = i128::from(beats.to_bits()) * 960;
    let ticks = (num + if num >= 0 { HALF } else { -HALF }) / FACTOR;
    // Clamped to the `i64` range, so this cast is lossless.
    #[expect(clippy::cast_possible_truncation)]
    {
        ticks.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
    }
}

/// Maps a CLAP [`TransportEvent`] to the neutral [`TransportInfo`]
/// snapshot.
///
/// Status: `IS_PLAYING` or `IS_RECORDING` → `Playing`, otherwise
/// `Stopped`. Position: `song_pos_seconds` × sample-rate when the
/// host supplies a seconds timeline, else the `steady_time` sample
/// counter, else zero. The loop range maps when both
/// `IS_LOOP_ACTIVE` and a seconds timeline are present and
/// `end > start`.
///
/// Musical transport maps to [`MusicalTimeInfo`] when both
/// `HAS_BEATS_TIMELINE` and `HAS_TEMPO` are set and `tempo` is a
/// finite positive BPM; otherwise `musical()` is `None` — no implicit
/// 4/4 or tempo is fabricated. Signature fields map only with
/// `HAS_TIME_SIGNATURE` (else `0`/`0`), and `bar_start_tick` stays
/// `0` when no signature applies.
///
/// **RT contract:** RT-suitable — pure math, allocation-free.
pub(crate) fn adapt_transport(
    transport: &TransportEvent,
    sample_rate: SampleRate,
    steady_time: Option<u64>,
) -> TransportInfo {
    let flags = transport.flags;
    let status = if flags.intersects(TransportFlags::IS_PLAYING | TransportFlags::IS_RECORDING) {
        TransportStatus::Playing
    } else {
        TransportStatus::Stopped
    };
    let rate = sample_rate.get();
    let base = TimeBase::sample_rate(rate).unwrap_or(TimeBase::COMMON_90K);
    let position = if flags.contains(TransportFlags::HAS_SECONDS_TIMELINE) {
        // SecondsTime → samples; clamped to i64 range (CLAP seconds
        // are already bounded in practice).
        let seconds: f64 = transport.song_pos_seconds.into();
        #[expect(clippy::cast_possible_truncation)]
        let samples = (seconds * f64::from(rate)).round() as i64;
        MediaTime::new(samples, base)
    } else {
        // `try_from` is infallible here: the `min` caps at i64::MAX —
        // CLAP bounds steady_time to a sample counter by spec.
        let samples = steady_time.map_or(0, |t| {
            i64::try_from(t.min(i64::MAX as u64)).unwrap_or(i64::MAX)
        });
        MediaTime::new(samples, base)
    };
    let mut info = TransportInfo::new(status, position);
    if flags.contains(TransportFlags::IS_LOOP_ACTIVE | TransportFlags::HAS_SECONDS_TIMELINE) {
        let start: f64 = transport.loop_start_seconds.into();
        let end: f64 = transport.loop_end_seconds.into();
        #[expect(clippy::cast_possible_truncation)]
        let (start_s, end_s) = (
            (start * f64::from(rate)).round() as i64,
            (end * f64::from(rate)).round() as i64,
        );
        if end_s > start_s {
            info = info.with_loop_range(MediaRange::new(
                MediaTime::new(start_s, base),
                MediaDuration::new(end_s - start_s, base),
            ));
        }
    }
    if flags.contains(TransportFlags::HAS_BEATS_TIMELINE | TransportFlags::HAS_TEMPO)
        && transport.tempo.is_finite()
        && transport.tempo > 0.0
    {
        let bpm_milli = (transport.tempo * 1_000.0)
            .round()
            .clamp(1.0, 999_999_999.0);
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bpm_milli = bpm_milli as u32;
        let (sig_num, sig_den) = if flags.contains(TransportFlags::HAS_TIME_SIGNATURE) {
            match (
                u8::try_from(transport.time_signature_numerator),
                u8::try_from(transport.time_signature_denominator),
            ) {
                (Ok(n @ 1..), Ok(d @ 1..)) => (n, d),
                _ => (0, 0),
            }
        } else {
            (0, 0)
        };
        info = info.with_musical(MusicalTimeInfo {
            position_ticks: beats_to_ticks(transport.song_pos_beats),
            bar_start_tick: if sig_num != 0 {
                beats_to_ticks(transport.bar_start)
            } else {
                0
            },
            bpm_milli,
            ppqn: PPQN.ticks_per_quarter(),
            sig_num,
            sig_den,
        });
    }
    info
}

// ---------------------------------------------------------------------------
// Status mapping
// ---------------------------------------------------------------------------

/// Maps a neutral [`ProcessResult`] to the CLAP process status.
///
/// `Continue` + `tail_frames > 0` maps to [`ClapProcessStatus::Tail`]
/// — the CLAP-idiomatic "meaningful tail remains" hint.
/// `ProcessStatus::Error` and unknown neutral variants map to
/// `Err` → `CLAP_PROCESS_ERROR`.
///
/// **RT contract:** RT-suitable — pure match, allocation-free.
///
/// # Errors
///
/// `PluginError::Message` — the neutral status is `Error` or a
/// variant this adapter does not map.
pub(crate) fn map_process_result(result: ProcessResult) -> Result<ClapProcessStatus, PluginError> {
    match result.status {
        ProcessStatus::Continue if result.tail_frames > 0 => Ok(ClapProcessStatus::Tail),
        ProcessStatus::Continue => Ok(ClapProcessStatus::Continue),
        ProcessStatus::Sleep => Ok(ClapProcessStatus::Sleep),
        _ => Err(PluginError::Message(
            "processor returned an unmapped status",
        )),
    }
}

/// Test-support fixtures shared by this crate's test modules —
/// `clap_audio_buffer` descriptor plumbing built exclusively through
/// clack-host's safe `AudioPorts` builders (no `unsafe` anywhere in
/// this crate, tests included).
///
/// The returned [`InputAudioBuffers`]/[`OutputAudioBuffers`] must
/// stay *locals of the test fn*: the paired plugin-side `Audio` borrows
/// their descriptor slices for `'a`, so callers fuse them with
/// [`OutputAudioBuffers::as_plugin_audio_with_inputs`] inline.
#[cfg(test)]
pub(crate) mod test_support {
    use clack_host::process::audio_buffers::{
        AudioPortBuffer, AudioPortBufferType, AudioPorts, InputAudioBuffers, InputChannel,
        OutputAudioBuffers,
    };

    use super::MAX_CHANNELS;

    /// A stereo `f32` port pair: distinct input/output backing `Vec`s
    /// plus the `AudioPorts` descriptor storage a host would own for
    /// one `process` call.
    pub(crate) struct StereoFixture {
        /// Input channel data — also the shared region for in-place
        /// blocks (the output descriptor is repointed at these
        /// addresses, which is how real hosts request in-place
        /// processing).
        pub(crate) inputs: [Vec<f32>; MAX_CHANNELS],
        /// Output channel data.
        pub(crate) outputs: [Vec<f32>; MAX_CHANNELS],
        /// Input-side `AudioPorts` storage.
        pub(crate) input_ports: AudioPorts,
        /// Output-side `AudioPorts` storage.
        pub(crate) output_ports: AudioPorts,
    }

    impl StereoFixture {
        /// `frames`-sample channels pre-filled with a non-unity
        /// input pattern and silent outputs.
        pub(crate) fn new(frames: usize) -> Self {
            Self {
                inputs: [vec![0.5; frames], vec![-0.25; frames]],
                outputs: [vec![0.0; frames], vec![0.0; frames]],
                input_ports: AudioPorts::with_capacity(MAX_CHANNELS, 1),
                output_ports: AudioPorts::with_capacity(MAX_CHANNELS, 1),
            }
        }

        /// The host-side buffer pair for this fixture — the exact
        /// `clap_audio_buffer` descriptor shape `clap_process`
        /// carries. Fuse them per test via
        /// [`OutputAudioBuffers::as_plugin_audio_with_inputs`].
        pub(crate) fn guards(&mut self) -> (InputAudioBuffers<'_>, OutputAudioBuffers<'_>) {
            let Self {
                inputs,
                outputs,
                input_ports,
                output_ports,
            } = self;
            let input_buffers = input_ports.with_input_buffers([AudioPortBuffer {
                latency: 0,
                channels: AudioPortBufferType::f32_input_only(
                    inputs.iter_mut().map(InputChannel::variable),
                ),
            }]);
            let output_buffers = output_ports.with_output_buffers([AudioPortBuffer {
                latency: 0,
                channels: AudioPortBufferType::f32_output_only(
                    outputs.iter_mut().map(Vec::as_mut_slice),
                ),
            }]);
            (input_buffers, output_buffers)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::StereoFixture;
    use super::*;
    use clack_common::events::event_types::{MidiEvent, ParamValueEvent};
    use clack_common::events::io::EventBuffer;
    use clack_common::events::{EventFlags, EventHeader, Pckn};
    use clack_common::utils::{BeatTime, ClapId, SecondsTime};
    use clack_host::process::audio_buffers::{
        AudioPortBuffer, AudioPortBufferType, AudioPorts, InputChannel,
    };

    use crate::gain::{GAIN_PARAM, GainProcessor};
    use ruviform_plugin_core::{PluginProcessor, ProcessContext};

    const FRAMES: u32 = 64;
    const FRAMES_USIZE: usize = 64;
    const RATE: u32 = 48_000;

    /// `GAIN_PARAM`'s id as a [`ClapId`].
    const fn gain_clap_id() -> ClapId {
        ClapId::new(GAIN_PARAM.id.0)
    }

    /// A `ParamValue` event for our gain parameter; `value` is the
    /// *plain* multiplier (the CLAP wire domain — `normalize` maps
    /// `[0, 2]` onto `[0, 1]`).
    const fn gain_event(time: u32, value: f64) -> ParamValueEvent {
        ParamValueEvent::new(time, gain_clap_id(), Pckn::match_all(), value)
    }

    /// Drives `machine` through `events` in order, asserting both the
    /// returned and the stored state after each transition.
    fn assert_chain(machine: &AtomicLifecycle, chain: &[(LifecycleEvent, LifecycleState)]) {
        for (event, expected) in chain {
            assert_eq!(machine.transition(*event), Ok(*expected));
            assert_eq!(machine.state(), *expected);
        }
    }

    /// A machine pre-driven to `state` along the legal path.
    fn lifecycle_at(state: LifecycleState) -> AtomicLifecycle {
        use LifecycleEvent as E;
        let machine = AtomicLifecycle::new();
        let path: &[E] = match state {
            LifecycleState::Created => &[],
            LifecycleState::Initialized => &[E::Initialize],
            LifecycleState::Activated => &[E::Initialize, E::Activate],
            LifecycleState::Processing => &[E::Initialize, E::Activate, E::StartProcessing],
            // `Failed` is reachable straight from `Created` — the host
            // reports the dead context, no init required.
            LifecycleState::Failed => &[E::Failed],
            _ => panic!("unsupported precondition state"),
        };
        for event in path {
            machine.transition(*event).unwrap();
        }
        assert_eq!(machine.state(), state);
        machine
    }

    // -- Lifecycle driver -------------------------------------------------

    #[test]
    fn atomic_lifecycle_forward_chain_and_destroy() {
        use LifecycleEvent as E;
        use LifecycleState as S;
        let machine = AtomicLifecycle::new();
        assert_eq!(machine.state(), S::Created);
        assert_chain(
            &machine,
            &[
                (E::Initialize, S::Initialized),
                (E::Activate, S::Activated),
                (E::StartProcessing, S::Processing),
                (E::Process, S::Processing), // per-block self-loop
                (E::Process, S::Processing),
                (E::StopProcessing, S::Activated),
                (E::Deactivate, S::Initialized),
                (E::Destroy, S::Destroyed),
            ],
        );
    }

    #[test]
    fn atomic_lifecycle_reactivation_cycle() {
        use LifecycleEvent as E;
        use LifecycleState as S;
        let machine = lifecycle_at(S::Activated);
        assert_chain(
            &machine,
            &[
                (E::StartProcessing, S::Processing),
                (E::StopProcessing, S::Activated),
                (E::Deactivate, S::Initialized),
                // Re-activation after a full deactivate is legal.
                (E::Activate, S::Activated),
                (E::StartProcessing, S::Processing),
                (E::StopProcessing, S::Activated),
                (E::Deactivate, S::Initialized),
            ],
        );
    }

    #[test]
    fn atomic_lifecycle_violations_leave_honest_state() {
        use LifecycleEvent as E;
        use LifecycleState as S;
        // Every (state, event) pair outside the transition table is a
        // defined rejection that leaves the machine where it was —
        // never a fabricated transition.
        for (state, event) in [
            // Process anywhere outside Processing — including the
            // "process before start_processing" case from Activated.
            (S::Created, E::Process),
            (S::Initialized, E::Process),
            (S::Activated, E::Process),
            // StartProcessing outside Activated.
            (S::Created, E::StartProcessing),
            (S::Initialized, E::StartProcessing),
            (S::Processing, E::StartProcessing), // double start
            // StopProcessing outside Processing.
            (S::Activated, E::StopProcessing), // stop before start
            // Deactivate requires Activated.
            (S::Created, E::Deactivate),
            (S::Initialized, E::Deactivate), // deactivate without activate
            (S::Processing, E::Deactivate),  // processing → activate needs stop
            // Destroy requires Initialized/Created.
            (S::Activated, E::Destroy),
            (S::Processing, E::Destroy),
            // Initialize/Activate out of order.
            (S::Initialized, E::Initialize),
            (S::Created, E::Activate),
        ] {
            let machine = lifecycle_at(state);
            assert_eq!(
                machine.transition(event),
                Err(LifecycleError::IllegalTransition { from: state, event }),
                "{event:?} must reject from {state:?}"
            );
            assert_eq!(
                machine.state(),
                state,
                "rejected transition must not move the machine"
            );
        }
        // Destroyed is terminal for every event — including a double
        // destroy.
        let destroyed = lifecycle_at(S::Initialized);
        destroyed.transition(E::Destroy).unwrap();
        for event in [
            E::Initialize,
            E::Activate,
            E::StartProcessing,
            E::Process,
            E::StopProcessing,
            E::Deactivate,
            E::Destroy,
        ] {
            assert!(destroyed.transition(event).is_err());
            assert_eq!(destroyed.state(), S::Destroyed);
        }
    }

    #[test]
    fn atomic_lifecycle_failed_survives_the_atomic_round_trip() {
        use LifecycleEvent as E;
        use LifecycleState as S;
        // `Failed` must encode and decode as itself — a decode that
        // collapsed it into `Destroyed` would still read terminal, but
        // the table's only escape (`Destroy`) would become illegal and
        // wedge the instance.
        let machine = lifecycle_at(S::Failed);
        assert_eq!(machine.state(), S::Failed);
        for event in [
            E::Initialize,
            E::Activate,
            E::StartProcessing,
            E::Process,
            E::StopProcessing,
            E::Deactivate,
            E::Failed, // re-reporting Failed is illegal
        ] {
            assert!(machine.transition(event).is_err());
            assert_eq!(machine.state(), S::Failed);
        }
        assert_eq!(machine.transition(E::Destroy), Ok(S::Destroyed));
        assert_eq!(machine.state(), S::Destroyed);
    }

    #[test]
    fn param_mirror_roundtrip_is_atomic_plain() {
        let mirror = ParamMirror::new(1.0);
        assert_eq!(mirror.load(), 1.0);
        mirror.store(0.5);
        assert_eq!(mirror.load(), 0.5);
    }

    // -- Activation config -------------------------------------------------

    #[test]
    fn to_process_config_maps_activation_bounds() {
        let config = to_process_config(&PluginAudioConfiguration {
            sample_rate: 48_000.0,
            min_frames_count: 1,
            max_frames_count: 512,
        })
        .unwrap();
        assert_eq!(config.sample_rate().get(), RATE);
        assert_eq!(config.min_frames(), 1);
        assert_eq!(config.max_frames(), 512);
    }

    #[test]
    fn to_process_config_rejects_invalid_activation() {
        // Non-finite, non-positive, fractional and over-u32 sample
        // rates are all defined rejects — the neutral SampleRate is
        // integer Hz.
        for rate in [
            0.0,
            -1.0,
            f64::NAN,
            f64::INFINITY,
            44_100.5,
            f64::from(u32::MAX) + 1.0,
        ] {
            assert!(
                to_process_config(&PluginAudioConfiguration {
                    sample_rate: rate,
                    min_frames_count: 1,
                    max_frames_count: 512,
                })
                .is_err(),
                "sample rate {rate} must reject"
            );
        }
        // Zero max frames and min > max frame bounds.
        for (min, max) in [(1, 0), (8, 4)] {
            assert!(
                to_process_config(&PluginAudioConfiguration {
                    sample_rate: 48_000.0,
                    min_frames_count: min,
                    max_frames_count: max,
                })
                .is_err()
            );
        }
    }

    // -- Per-block buffer views --------------------------------------------

    #[test]
    fn build_block_views_paired_stereo_is_borrowed_zero_copy() {
        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let in_left = fixture.inputs[0].as_ptr();
        let in_right = fixture.inputs[1].as_ptr();
        let out_left = fixture.outputs[0].as_ptr();
        let out_right = fixture.outputs[1].as_ptr();
        let mut scratch = BlockScratch::new(MAX_CHANNELS, MAX_EVENTS_PER_BLOCK);

        let (input_buffers, mut output_buffers) = fixture.guards();
        let mut audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
        let mut pair = audio.port_pair(0).unwrap();
        let mut views = build_block_views(&mut pair, &mut scratch, FRAMES).unwrap();
        assert_eq!(views.channel_count, 2);
        assert_eq!(views.mode, PairMode::Paired);
        // The collected slices are the host buffers themselves —
        // pointer equality is the zero-copy proof.
        assert_eq!(views.inputs[0].as_ptr(), in_left);
        assert_eq!(views.inputs[1].as_ptr(), in_right);
        assert_eq!(views.outputs[0].as_ptr(), out_left);
        assert_eq!(views.outputs[1].as_ptr(), out_right);

        let mut output_list: [&mut [f32]; MAX_CHANNELS] = [&mut [], &mut []];
        let (input_view, output_view) = planar_views(&mut views, &mut output_list).unwrap();
        assert_eq!(input_view.channels(), 2);
        assert_eq!(output_view.channels(), 2);
        assert_eq!(input_view.channel(0).unwrap().as_ptr(), in_left);
        assert_eq!(output_view.channel(0).unwrap().as_ptr(), out_left);
        assert_eq!(output_view.channel(1).unwrap().as_ptr(), out_right);
    }

    #[test]
    fn build_block_views_rejects_asymmetric_pairs() {
        let mut scratch = BlockScratch::new(MAX_CHANNELS, MAX_EVENTS_PER_BLOCK);

        // Input-only pair: no output port at index 0.
        let mut inputs = [[0.5_f32; FRAMES_USIZE]; MAX_CHANNELS];
        let mut input_ports = AudioPorts::with_capacity(MAX_CHANNELS, 1);
        let input_buffers = input_ports.with_input_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_input_only(
                inputs.iter_mut().map(InputChannel::variable),
            ),
        }]);
        let mut input_only_audio = input_buffers.as_plugin_audio();
        let mut input_pair = input_only_audio.port_pair(0).unwrap();
        assert!(build_block_views(&mut input_pair, &mut scratch, FRAMES).is_err());

        // Output-only pair: no input port at index 0.
        let mut outputs = [[0.0_f32; FRAMES_USIZE]; MAX_CHANNELS];
        let mut output_ports = AudioPorts::with_capacity(MAX_CHANNELS, 1);
        let mut output_buffers = output_ports.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_output_only(
                outputs.iter_mut().map(<[f32; FRAMES_USIZE]>::as_mut_slice),
            ),
        }]);
        let mut output_only_audio = output_buffers.as_plugin_audio();
        let mut output_pair = output_only_audio.port_pair(0).unwrap();
        assert!(build_block_views(&mut output_pair, &mut scratch, FRAMES).is_err());
    }

    #[test]
    fn build_block_views_rejects_excess_channels() {
        let mut ins = [[0.5_f32; FRAMES_USIZE], [0.0; 64], [-0.5; 64]];
        let mut outs = [[0.0_f32; FRAMES_USIZE]; 3];
        let mut input_ports = AudioPorts::with_capacity(3, 1);
        let mut output_ports = AudioPorts::with_capacity(3, 1);
        let input_buffers = input_ports.with_input_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_input_only(
                ins.iter_mut().map(InputChannel::variable),
            ),
        }]);
        let mut output_buffers = output_ports.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_output_only(
                outs.iter_mut().map(<[f32; FRAMES_USIZE]>::as_mut_slice),
            ),
        }]);
        let mut audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
        let mut pair = audio.port_pair(0).unwrap();
        let mut scratch = BlockScratch::new(MAX_CHANNELS, MAX_EVENTS_PER_BLOCK);
        assert_eq!(pair.channel_pair_count(), 3);
        assert!(build_block_views(&mut pair, &mut scratch, FRAMES).is_err());
    }

    #[test]
    fn build_block_views_rejects_f64_only_and_mismatched_pairs() {
        let mut scratch = BlockScratch::new(MAX_CHANNELS, MAX_EVENTS_PER_BLOCK);

        // f64-only pair — the adapter is f32-only; `SampleType::F64`
        // is a defined reject, never a reinterpret-cast.
        let mut ins64 = [[0.5_f64; FRAMES_USIZE]; MAX_CHANNELS];
        let mut outs64 = [[0.0_f64; FRAMES_USIZE]; MAX_CHANNELS];
        let mut in_ports = AudioPorts::with_capacity(MAX_CHANNELS, 1);
        let mut out_ports = AudioPorts::with_capacity(MAX_CHANNELS, 1);
        let input_buffers = in_ports.with_input_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f64_input_only(
                ins64.iter_mut().map(InputChannel::variable),
            ),
        }]);
        let mut output_buffers = out_ports.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f64_output_only(
                outs64.iter_mut().map(<[f64; FRAMES_USIZE]>::as_mut_slice),
            ),
        }]);
        let mut f64_audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
        let mut f64_pair = f64_audio.port_pair(0).unwrap();
        assert!(build_block_views(&mut f64_pair, &mut scratch, FRAMES).is_err());

        // Mismatched f64-in / f32-out — `try_match` reports
        // `MismatchedBufferPair`, mapped to a reject.
        let mut ins64b = [[0.5_f64; FRAMES_USIZE]; MAX_CHANNELS];
        let mut outs32 = [[0.0_f32; FRAMES_USIZE]; MAX_CHANNELS];
        let input_buffers = in_ports.with_input_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f64_input_only(
                ins64b.iter_mut().map(InputChannel::variable),
            ),
        }]);
        let mut output_buffers = out_ports.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_output_only(
                outs32.iter_mut().map(<[f32; FRAMES_USIZE]>::as_mut_slice),
            ),
        }]);
        let mut mixed_audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
        let mut mixed_pair = mixed_audio.port_pair(0).unwrap();
        assert!(build_block_views(&mut mixed_pair, &mut scratch, FRAMES).is_err());
    }

    #[test]
    fn build_block_views_rejects_malformed_output_descriptors() {
        let mut scratch = BlockScratch::new(MAX_CHANNELS, MAX_EVENTS_PER_BLOCK);
        let mut fixture = StereoFixture::new(FRAMES_USIZE);

        // Both data32 and data64 null on the port — no backing store
        // at all: reject before any channel slice exists.
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            output_buffers.as_raw_buffers()[0].data32 = core::ptr::null_mut();
            let mut audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            let mut pair = audio.port_pair(0).unwrap();
            assert!(build_block_views(&mut pair, &mut scratch, FRAMES).is_err());
        }

        // Null channel pointer with frames > 0 — would form a slice
        // over null (UB); caught on the raw pointer list first.
        {
            let mut ptr_list = [core::ptr::null_mut::<f32>(), fixture.inputs[1].as_mut_ptr()];
            let (input_buffers, mut output_buffers) = fixture.guards();
            output_buffers.as_raw_buffers()[0].data32 = ptr_list.as_mut_ptr();
            let mut audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            let mut pair = audio.port_pair(0).unwrap();
            assert!(build_block_views(&mut pair, &mut scratch, FRAMES).is_err());
        }

        // Unaligned channel pointer — likewise caught pre-slice. One
        // byte past an f32-aligned base; the pointer is never
        // dereferenced (`is_aligned` checks the address only).
        {
            let mut ptr_list = [
                fixture.inputs[0].as_mut_ptr().wrapping_byte_add(1),
                fixture.inputs[1].as_mut_ptr(),
            ];
            let (input_buffers, mut output_buffers) = fixture.guards();
            output_buffers.as_raw_buffers()[0].data32 = ptr_list.as_mut_ptr();
            let mut audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            let mut pair = audio.port_pair(0).unwrap();
            assert!(build_block_views(&mut pair, &mut scratch, FRAMES).is_err());
        }
    }

    #[test]
    fn null_channel_pointers_are_legal_only_at_zero_frames() {
        // Directly on the raw-pointer validator: null + 0 frames is
        // the legal empty-slice case; any frames make it a reject.
        let ptrs = [core::ptr::null_mut::<f32>(), core::ptr::null_mut::<f32>()];
        assert!(validate_channel_pointers(&ptrs, 0).is_ok());
        assert!(validate_channel_pointers(&ptrs, 1).is_err());
    }

    #[test]
    fn build_block_views_rejects_cross_alias_and_duplicate_outputs() {
        let mut scratch = BlockScratch::new(MAX_CHANNELS, MAX_EVENTS_PER_BLOCK);
        let mut fixture = StereoFixture::new(FRAMES_USIZE);

        // Partial alias: out[0] == in[1] and out[1] == in[0] — the
        // stereo channel-swap a careless host could request. Two &mut
        // regions would overlap the inputs.
        {
            let mut ptr_list = [
                fixture.inputs[1].as_mut_ptr(),
                fixture.inputs[0].as_mut_ptr(),
            ];
            let (input_buffers, mut output_buffers) = fixture.guards();
            output_buffers.as_raw_buffers()[0].data32 = ptr_list.as_mut_ptr();
            let mut audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            let mut pair = audio.port_pair(0).unwrap();
            assert!(build_block_views(&mut pair, &mut scratch, FRAMES).is_err());
        }

        // Duplicate output channel regions — two &mut over one region.
        {
            let mut ptr_list = [
                fixture.outputs[0].as_mut_ptr(),
                fixture.outputs[0].as_mut_ptr(),
            ];
            let (input_buffers, mut output_buffers) = fixture.guards();
            output_buffers.as_raw_buffers()[0].data32 = ptr_list.as_mut_ptr();
            let mut audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            let mut pair = audio.port_pair(0).unwrap();
            assert!(build_block_views(&mut pair, &mut scratch, FRAMES).is_err());
        }
    }

    #[test]
    fn build_block_views_rejects_partial_range_overlap() {
        let mut scratch = BlockScratch::new(MAX_CHANNELS, MAX_EVENTS_PER_BLOCK);
        let mut fixture = StereoFixture::new(FRAMES_USIZE);

        // Shifted overlap: out[0] = in[0] + 1 — the ranges intersect
        // but the pointers differ, so equality-only detection would
        // miss it. Caught on the raw pointer lists before any slice.
        {
            let mut ptr_list = [
                fixture.inputs[0].as_mut_ptr().wrapping_add(1),
                fixture.outputs[1].as_mut_ptr(),
            ];
            let (input_buffers, mut output_buffers) = fixture.guards();
            output_buffers.as_raw_buffers()[0].data32 = ptr_list.as_mut_ptr();
            let mut audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            let mut pair = audio.port_pair(0).unwrap();
            assert!(build_block_views(&mut pair, &mut scratch, FRAMES).is_err());
        }

        // Shifted output↔output overlap: out[1] = out[0] + 8.
        {
            let mut ptr_list = [
                fixture.outputs[0].as_mut_ptr(),
                fixture.outputs[0].as_mut_ptr().wrapping_add(8),
            ];
            let (input_buffers, mut output_buffers) = fixture.guards();
            output_buffers.as_raw_buffers()[0].data32 = ptr_list.as_mut_ptr();
            let mut audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            let mut pair = audio.port_pair(0).unwrap();
            assert!(build_block_views(&mut pair, &mut scratch, FRAMES).is_err());
        }
    }

    #[test]
    fn in_place_pair_classifies_and_scales_shared_buffer() {
        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let io_const = [fixture.inputs[0].as_ptr(), fixture.inputs[1].as_ptr()];
        let io_ptrs = [
            fixture.inputs[0].as_mut_ptr(),
            fixture.inputs[1].as_mut_ptr(),
        ];
        let mut scratch = BlockScratch::new(MAX_CHANNELS, MAX_EVENTS_PER_BLOCK);

        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            // The output descriptor's channel list is replaced by the
            // *input* addresses — a host reusing one buffer in/out
            // makes clack classify `ChannelPair::InPlace` per channel.
            let mut shared_list = io_ptrs;
            output_buffers.as_raw_buffers()[0].data32 = shared_list.as_mut_ptr();
            let mut audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            let mut pair = audio.port_pair(0).unwrap();

            let mut views = build_block_views(&mut pair, &mut scratch, FRAMES).unwrap();
            assert_eq!(views.mode, PairMode::InPlace);
            // The shared region lands in `outputs` only — no &[f32]
            // is ever built over the same memory (adapter contract).
            let mut output_list: [&mut [f32]; MAX_CHANNELS] = [&mut [], &mut []];
            let (input_view, output_view) = planar_views(&mut views, &mut output_list).unwrap();
            assert_eq!(input_view.channels(), 0);
            assert_eq!(output_view.channel(0).unwrap().as_ptr(), io_const[0]);
            assert_eq!(output_view.channel(1).unwrap().as_ptr(), io_const[1]);

            // Drive the processor exactly as `process` does: the empty
            // input view makes each channel scale its own output
            // buffer.
            let mut processor = GainProcessor::new();
            let mut context = ProcessContext::new();
            let config = ProcessConfig::new(SampleRate::new(RATE).unwrap(), 1, FRAMES).unwrap();
            processor.prepare(&config, &mut context).unwrap();
            processor.set_gain_plain(0.5);
            let mut outputs = [output_view];
            let result = processor.process(
                &[input_view],
                &mut outputs,
                EventList::empty(FRAMES),
                None,
                &mut context,
            );
            assert_eq!(result.status, ProcessStatus::Continue);
        }

        // The shared region was scaled in place — no copy, no alias.
        assert_eq!(fixture.inputs[0], vec![0.25; FRAMES_USIZE]);
        assert_eq!(fixture.inputs[1], vec![-0.125; FRAMES_USIZE]);
    }

    // -- Events ------------------------------------------------------------

    /// Translates `buffer`'s contents into an [`EventList`],
    /// returning the `(offset, normalized value)` sequence for
    /// order/stability assertions.
    fn translated_sequence(
        buffer: &EventBuffer,
        block_frames: u32,
    ) -> Result<Vec<(u32, f64)>, PluginError> {
        let input = buffer.as_input();
        let mut scratch = BlockScratch::new(MAX_CHANNELS, MAX_EVENTS_PER_BLOCK);
        let list = translate_events(&input, &mut scratch, &GAIN_PARAM, block_frames)?;
        Ok(list
            .iter()
            .map(|event| {
                let value = match event.kind() {
                    EventKind::ParamValue(param_event) => param_event.value.get(),
                    _ => unreachable!("only ParamValue events are translated"),
                };
                (event.frame_offset(), value)
            })
            .collect())
    }

    #[test]
    fn translate_events_sorts_same_offset_events_stably() {
        let mut buffer = EventBuffer::new();
        // Unsorted input with two same-offset events — plain values
        // 0.5 / 1.5 normalize to .25 / .75, making the original order
        // distinguishable after sorting.
        buffer.push(&gain_event(10, 0.5));
        buffer.push(&gain_event(10, 1.5));
        buffer.push(&gain_event(2, 1.0));
        assert_eq!(
            translated_sequence(&buffer, FRAMES).unwrap(),
            [(2, 0.5), (10, 0.25), (10, 0.75)],
            "stable sort keeps same-offset events in arrival order"
        );
    }

    #[test]
    fn translate_events_skips_non_param_and_foreign_events() {
        let mut buffer = EventBuffer::new();
        buffer.push(&MidiEvent::new(0, 0, [0x90, 60, 100])); // non-param kind
        buffer.push(&gain_event(0, f64::NAN)); // non-finite value
        buffer.push(&ParamValueEvent::new(
            4,
            ClapId::new(42), // a different parameter id
            Pckn::match_all(),
            0.5,
        ));
        // `CLAP_INVALID_ID` — `ClapId::new(u32::MAX)` panics, so the
        // invalid id is injected into the raw event data (the exact
        // byte shape a malformed host can deliver). `param_id()`
        // decodes it to `None` → defined skip.
        let mut raw = *gain_event(2, 0.5).as_raw();
        raw.param_id = u32::MAX;
        buffer.push(&ParamValueEvent::from_raw(&raw));
        buffer.push(&gain_event(8, 0.5));
        let translated = translated_sequence(&buffer, FRAMES).unwrap();
        assert_eq!(translated, [(8, 0.25)]);
    }

    #[test]
    fn translate_events_bounds_count_and_offsets() {
        // A host may attach arbitrary event counts; the bounded scan
        // rejects past MAX_EVENTS_PER_BLOCK before translating.
        let mut buffer = EventBuffer::new();
        for _ in 0..=MAX_EVENTS_PER_BLOCK {
            buffer.push(&gain_event(0, 0.5));
        }
        assert!(translated_sequence(&buffer, FRAMES).is_err());

        // An out-of-block offset drops just that event — the shared
        // drop policy, identical to the VST3 adapter.
        let mut buffer = EventBuffer::new();
        buffer.push(&gain_event(FRAMES, 0.5));
        assert_eq!(translated_sequence(&buffer, FRAMES).unwrap().len(), 0);
        buffer.clear();
        buffer.push(&gain_event(FRAMES - 1, 0.5));
        assert_eq!(translated_sequence(&buffer, FRAMES).unwrap().len(), 1);

        // Zero-frame blocks accept only offset zero — the neutral
        // bound is `offset < max(block_frames, 1)`; anything else is
        // dropped, not an error.
        buffer.clear();
        buffer.push(&gain_event(0, 0.5));
        assert_eq!(translated_sequence(&buffer, 0).unwrap().len(), 1);
        buffer.push(&gain_event(1, 0.5));
        assert_eq!(translated_sequence(&buffer, 0).unwrap().len(), 1);
    }

    // -- Transport -----------------------------------------------------------

    /// A [`TransportEvent`] value with `flags` — all other fields at
    /// neutral defaults except the seconds positions.
    fn transport_with(flags: TransportFlags, song_pos: f64) -> TransportEvent {
        TransportEvent {
            header: EventHeader::<TransportEvent>::new_core(0, EventFlags::empty()),
            flags,
            song_pos_beats: BeatTime::from_int(0),
            song_pos_seconds: SecondsTime::from_float(song_pos),
            tempo: 120.0,
            tempo_inc: 0.0,
            loop_start_beats: BeatTime::from_int(0),
            loop_end_beats: BeatTime::from_int(0),
            loop_start_seconds: SecondsTime::from_float(1.0),
            loop_end_seconds: SecondsTime::from_float(3.0),
            bar_start: BeatTime::from_int(0),
            bar_number: 0,
            time_signature_numerator: 4,
            time_signature_denominator: 4,
        }
    }

    #[test]
    fn adapt_transport_maps_status_position_and_loop() {
        let rate = SampleRate::new(RATE).unwrap();
        let base = TimeBase::sample_rate(RATE).unwrap();

        // Playing + seconds timeline: position is seconds × rate.
        let info = adapt_transport(
            &transport_with(
                TransportFlags::IS_PLAYING | TransportFlags::HAS_SECONDS_TIMELINE,
                1.5,
            ),
            rate,
            Some(999),
        );
        assert_eq!(info.status(), TransportStatus::Playing);
        assert_eq!(info.position(), MediaTime::new(72_000, base));
        assert_eq!(info.loop_range(), None);

        // Recording also maps to Playing; without the seconds
        // timeline the steady_time sample counter is the position.
        let info = adapt_transport(
            &transport_with(TransportFlags::IS_RECORDING, 1.5),
            rate,
            Some(9_999),
        );
        assert_eq!(info.status(), TransportStatus::Playing);
        assert_eq!(info.position(), MediaTime::new(9_999, base));

        // No flags → Stopped; no steady time either → zero position.
        let info = adapt_transport(&transport_with(TransportFlags::empty(), 1.5), rate, None);
        assert_eq!(info.status(), TransportStatus::Stopped);
        assert_eq!(info.position(), MediaTime::new(0, base));

        // Loop range maps only when loop + seconds timeline and
        // end > start: 1.0s..3.0s at 48 kHz.
        let info = adapt_transport(
            &transport_with(
                TransportFlags::IS_PLAYING
                    | TransportFlags::HAS_SECONDS_TIMELINE
                    | TransportFlags::IS_LOOP_ACTIVE,
                2.0,
            ),
            rate,
            None,
        );
        assert_eq!(
            info.loop_range(),
            Some(MediaRange::new(
                MediaTime::new(48_000, base),
                MediaDuration::new(96_000, base),
            ))
        );
    }

    #[test]
    fn adapt_transport_maps_musical_fields() {
        let rate = SampleRate::new(RATE).unwrap();
        let flags = TransportFlags::HAS_BEATS_TIMELINE
            | TransportFlags::HAS_TEMPO
            | TransportFlags::HAS_TIME_SIGNATURE;
        let mut transport = transport_with(flags, 0.0);
        transport.song_pos_beats = BeatTime::from_float(2.5);
        transport.bar_start = BeatTime::from_float(2.0);
        transport.tempo = 125.5;
        transport.time_signature_numerator = 6;
        transport.time_signature_denominator = 8;

        let musical = adapt_transport(&transport, rate, None).musical().unwrap();
        assert_eq!(musical.position_ticks, 2_400);
        assert_eq!(musical.bar_start_tick, 1_920);
        assert_eq!(musical.bpm_milli, 125_500);
        assert_eq!(musical.ppqn, 960);
        assert_eq!((musical.sig_num, musical.sig_den), (6, 8));

        // Rounding is half away from zero around a half tick:
        // 0.5 tick is 2³⁰/960 ≈ 1_118_481.07 bits — one bit below
        // rounds to 0, one bit above rounds to 1.
        transport.song_pos_beats = BeatTime::from_bits(1_118_481);
        assert_eq!(
            adapt_transport(&transport, rate, None)
                .musical()
                .unwrap()
                .position_ticks,
            0
        );
        transport.song_pos_beats = BeatTime::from_bits(1_118_482);
        assert_eq!(
            adapt_transport(&transport, rate, None)
                .musical()
                .unwrap()
                .position_ticks,
            1
        );
    }

    /// Golden verification of the CLAP `BeatTime` → PPQN-960 tick
    /// projection against the official representation: `BeatTime` is an
    /// `i64` fixed-point value scaled by `CLAP_BEATTIME_FACTOR = 1<<31`.
    /// (Issue #189 — the adapter formula `bits × 960 / 2³¹` with
    /// half-away-from-zero rounding and `i64` saturation.)
    #[test]
    fn beats_to_ticks_clap_factor_golden() {
        // The adapter's factor *is* the spec factor 2³¹.
        assert_eq!(BeatTime::FACTOR, 1_i64 << 31);

        // Whole beats convert exactly — 1 beat = 960 ticks.
        assert_eq!(beats_to_ticks(BeatTime::from_int(1)), 960);
        assert_eq!(beats_to_ticks(BeatTime::from_int(4)), 3_840);
        assert_eq!(beats_to_ticks(BeatTime::from_int(-1)), -960);
        assert_eq!(beats_to_ticks(BeatTime::from_int(-4)), -3_840);

        // The exact inverse: `tick × 2³¹ / 960` round-trips whole
        // ticks through bits losslessly.
        for &tick in &[0i64, 1, 480, 960, 9_600, -960, -30] {
            let bits =
                i64::try_from(i128::from(tick) * i128::from(BeatTime::FACTOR) / 960).unwrap();
            assert_eq!(beats_to_ticks(BeatTime::from_bits(bits)), tick);
        }

        // Negative half-away rounding is symmetric: one bit below the
        // -0.5-tick boundary rounds to 0, at/above to -1.
        assert_eq!(beats_to_ticks(BeatTime::from_bits(-1_118_481)), 0);
        assert_eq!(beats_to_ticks(BeatTime::from_bits(-1_118_482)), -1);

        // The full `i64` input range maps to ≈±4.12×10¹² ticks — the
        // `i64` clamp is a defensive bound that cannot saturate on real
        // input, and the ends stay symmetric (no wrap).
        assert_eq!(
            beats_to_ticks(BeatTime::from_bits(i64::MAX)),
            4_123_168_604_160
        );
        assert_eq!(
            beats_to_ticks(BeatTime::from_bits(i64::MIN)),
            -4_123_168_604_160
        );
    }

    #[test]
    fn adapt_transport_musical_gates_and_unset_forms() {
        let rate = SampleRate::new(RATE).unwrap();

        // Missing beats timeline or tempo → no musical snapshot.
        for flags in [
            TransportFlags::HAS_TEMPO,
            TransportFlags::HAS_BEATS_TIMELINE,
            TransportFlags::empty(),
        ] {
            let mut transport = transport_with(flags, 0.0);
            transport.tempo = 120.0;
            assert_eq!(adapt_transport(&transport, rate, None).musical(), None);
        }

        // Non-positive / non-finite tempo → None.
        for tempo in [0.0, -120.0, f64::NAN, f64::INFINITY] {
            let mut transport = transport_with(
                TransportFlags::HAS_BEATS_TIMELINE | TransportFlags::HAS_TEMPO,
                0.0,
            );
            transport.tempo = tempo;
            assert_eq!(adapt_transport(&transport, rate, None).musical(), None);
        }

        // Without HAS_TIME_SIGNATURE: sig is 0/0 and bar_start is 0,
        // even though the fixture carries a signature.
        let flags = TransportFlags::HAS_BEATS_TIMELINE | TransportFlags::HAS_TEMPO;
        let mut transport = transport_with(flags, 0.0);
        transport.bar_start = BeatTime::from_float(4.0);
        let musical = adapt_transport(&transport, rate, None).musical().unwrap();
        assert_eq!((musical.sig_num, musical.sig_den), (0, 0));
        assert_eq!(musical.bar_start_tick, 0);
        assert_eq!(musical.bpm_milli, 120_000);
    }

    // -- Status mapping ------------------------------------------------------

    #[test]
    fn map_process_result_maps_neutral_statuses() {
        assert_eq!(
            map_process_result(ProcessResult::CONTINUE).unwrap(),
            ClapProcessStatus::Continue
        );
        assert_eq!(
            map_process_result(ProcessResult::new(ProcessStatus::Continue, 10)).unwrap(),
            ClapProcessStatus::Tail
        );
        assert_eq!(
            map_process_result(ProcessResult::SLEEP).unwrap(),
            ClapProcessStatus::Sleep
        );
        assert!(map_process_result(ProcessResult::new(ProcessStatus::Error, 0)).is_err());
    }
}
