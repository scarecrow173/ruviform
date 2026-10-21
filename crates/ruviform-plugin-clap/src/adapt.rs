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

use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use clack_common::events::UnknownEvent;
use clack_common::events::event_types::{TransportEvent, TransportFlags};
use clack_common::events::io::InputEvents;
use clack_common::events::spaces::CoreEventSpace;
use clack_plugin::plugin::PluginError;
use clack_plugin::process::audio::{BufferError, ChannelPair, PortPair, SampleType};
use clack_plugin::process::{PluginAudioConfiguration, ProcessStatus as ClapProcessStatus};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_media_core::{MediaDuration, MediaRange, MediaTime, TimeBase, TransportStatus};
use ruviform_plugin_core::{
    Event, EventKind, EventList, LifecycleError, LifecycleEvent, LifecycleState, ParamEvent,
    ParamId, ParamInfo, ProcessConfig, ProcessResult, ProcessStatus, TransportInfo,
};

/// Channel capacity negotiated by this `PoC`: one stereo port pair.
pub(crate) const MAX_CHANNELS: usize = 2;

/// Maximum input events translated per block. Hosts may attach
/// arbitrary event counts; the per-block scan must stay bounded, so
/// oversized lists are a defined rejection rather than unbounded work.
pub(crate) const MAX_EVENTS_PER_BLOCK: usize = 256;

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
}

impl BlockScratch {
    /// Allocates scratch for the negotiated bounds (non-RT only —
    /// called from `activate`).
    pub(crate) fn new(max_channels: usize, max_events: usize) -> Self {
        Self {
            input_ptrs: Vec::with_capacity(max_channels),
            output_ptrs: Vec::with_capacity(max_channels),
            events: Vec::with_capacity(max_events),
        }
    }

    /// The event scratch for [`translate_events`].
    ///
    /// **RT contract:** RT-suitable — borrow, no allocation.
    pub(crate) fn events_scratch(&mut self) -> &mut Vec<Event> {
        &mut self.events
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
        _ => STATE_UNKNOWN,
    }
}

fn decode_state(raw: u8) -> LifecycleState {
    match raw {
        STATE_CREATED => LifecycleState::Created,
        STATE_INITIALIZED => LifecycleState::Initialized,
        STATE_ACTIVATED => LifecycleState::Activated,
        STATE_PROCESSING => LifecycleState::Processing,
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

    /// The current state.
    ///
    /// **RT contract:** RT-suitable — atomic load, allocation-free.
    pub(crate) fn state(&self) -> LifecycleState {
        decode_state(self.state.load(Ordering::Acquire))
    }

    /// Feeds `event` through the shared transition table atomically:
    /// on a legal pair the machine stores `next`; on an illegal pair
    /// the [`LifecycleError::IllegalTransition`] built by plugin-core's
    /// pure table is returned and the machine stays put.
    ///
    /// **RT contract:** RT-suitable — pure table lookup + one load and
    /// one store; no allocation, no blocking, no retry.
    ///
    /// # Errors
    ///
    /// [`LifecycleError::IllegalTransition`] — the event is not legal
    /// from the current state.
    pub(crate) fn transition(
        &self,
        event: LifecycleEvent,
    ) -> Result<LifecycleState, LifecycleError> {
        let next = self.state().transition(event)?;
        self.state.store(encode_state(next), Ordering::Release);
        Ok(next)
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
) -> Result<BlockViews<'p>, PluginError> {
    // Channel-count bound vs negotiated stereo capacity.
    if pair.channel_pair_count() > MAX_CHANNELS {
        return Err(PluginError::Message(
            "channel count exceeds negotiated capacity",
        ));
    }
    let frames_count = pair.frames_count();

    // Per-channel null/alignment pre-validation on the raw pointer
    // lists — before any channel slice exists.
    if let Some(input) = pair.input() {
        match input.channels() {
            Ok(sample) => {
                if let Some(channels) = sample.as_f32() {
                    validate_channel_pointers(channels.raw_data(), frames_count)?;
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
                }
            }
            Err(_) => {
                return Err(PluginError::Message("output port without channel buffers"));
            }
        }
    }

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

    // Classify each channel pair; collect slices + raw pointers.
    scratch.input_ptrs.clear();
    scratch.output_ptrs.clear();
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
                scratch.input_ptrs.push(input.as_ptr().addr());
                scratch.output_ptrs.push(output.as_ptr().addr());
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
                scratch.output_ptrs.push(io.as_ptr().addr());
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

    // Alias checks at per-channel pointer granularity (spec):
    // clack classifies same-index equality as InPlace; here we catch
    // cross-index partial aliases and duplicate output regions, both
    // of which would put two &mut over one region.
    for (i, out) in scratch.output_ptrs.iter().enumerate() {
        if scratch.output_ptrs[i + 1..].contains(out) {
            return Err(PluginError::Message("duplicate output channel region"));
        }
        if pair_mode == PairMode::Paired && scratch.input_ptrs.contains(out) {
            return Err(PluginError::Message("input/output cross-alias detected"));
        }
    }
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
/// [`EventList`] borrowing `scratch` — allocation-free (pushes stay
/// within the pre-allocated capacity; bounds are checked first).
///
/// CLAP requires sorted input events; if a host still delivers
/// unsorted ones, the list is re-ordered in place with
/// `sort_unstable_by_key` — no auxiliary allocation (a stable sort
/// would allocate scratch). Caveat recorded for the ADR: the order
/// of same-offset events may differ from host order under an
/// unstable sort; it is benign for this single-param `PoC`.
///
/// **RT contract:** RT-suitable — bounded `O(n + n·log n)` scan,
/// allocation-free.
///
/// # Errors
///
/// `PluginError::Message` — event count exceeds
/// `MAX_EVENTS_PER_BLOCK`, or offsets exceed the block bounds (via
/// `EventList::new`).
pub(crate) fn translate_events<'a>(
    input: &InputEvents<'_>,
    scratch: &'a mut Vec<Event>,
    param: &ParamInfo,
    block_frames: u32,
) -> Result<EventList<'a>, PluginError> {
    if input.len() as usize > MAX_EVENTS_PER_BLOCK {
        return Err(PluginError::Message("event count exceeds per-block bound"));
    }
    scratch.clear();
    let mut sorted = true;
    let mut previous = 0_u32;
    for unknown in input.iter() {
        let Some(event) = translate_param_event(unknown, param) else {
            continue;
        };
        if event.frame_offset() < previous {
            sorted = false;
        }
        previous = event.frame_offset();
        scratch.push(event);
    }
    if !sorted {
        scratch.sort_unstable_by_key(|event: &Event| event.frame_offset());
    }
    EventList::new(scratch.as_slice(), block_frames)
        .map_err(|_| PluginError::Message("event offsets exceed block bounds"))
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

/// Maps a CLAP [`TransportEvent`] to the neutral [`TransportInfo`]
/// snapshot — media time only, no musical time.
///
/// Status: `IS_PLAYING` or `IS_RECORDING` → `Playing`, otherwise
/// `Stopped`. Position: `song_pos_seconds` × sample-rate when the
/// host supplies a seconds timeline, else the `steady_time` sample
/// counter, else zero. The loop range maps when both
/// `IS_LOOP_ACTIVE` and a seconds timeline are present and
/// `end > start`.
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
