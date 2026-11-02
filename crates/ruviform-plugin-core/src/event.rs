//! Sample-accurate events: the [`Event`] value type, its [`EventKind`]
//! payload and the borrowed, validity-checked [`EventList`] handed to
//! processing callbacks.
//!
//! ## List semantics
//!
//! An [`EventList`] is the wire form of per-block events and defines:
//!
//! - **Ordering** — events are sorted by frame offset
//!   (non-decreasing). [`EventList::new`] *rejects* unsorted input; an
//!   adapter that receives unsorted host events must either pre-sort
//!   ([`sort_events_by_frame_offset`] is stable and allocation-free
//!   over a caller-provided scratch, preserving the order of
//!   same-offset events) or refuse the block.
//! - **Offset representation** — an event offset is a *within-block
//!   frame offset* bounded by `offset < block_frames`. Absolute
//!   [`MediaTime`] positions are deliberately **not** the wire
//!   representation: the block start is implicit in the processing
//!   call, so only the in-block delta is carried.
//! - **Zero-frame flushing** — a zero-frame block cannot contain
//!   offsets at all, so the bound degenerates to `offset == 0`: every
//!   listed event is due *now* and is applied immediately (flushed).
//!   Non-zero offsets in a zero-frame list are rejected.
//! - **Same-offset conflicts** — stable sorting preserves the source
//!   order of events that share a `frame_offset`. For two
//!   [`ParamEvent`]s that target the *same* [`ParamId`] at the same
//!   offset, the neutral application semantics are **last-wins**: the
//!   later event in list order is the effective value (processors
//!   apply changes in list order without interpolation). Events for
//!   *different* `ParamId`s at the same offset are independent and
//!   both apply.
//! - **Process status / tail** — the neutral [`ProcessResult`] mapping
//!   lives in [`crate::processor`]; adapters translate it to each
//!   standard's own status codes.
//!
//! [`MediaTime`]: ruviform_media_core::MediaTime

use ruviform_media_core::{Error, LedgerKey, MidiEvent, Result};

use crate::param::{ParamId, ParamValue};

/// A parameter change event payload: which parameter moved and to
/// which normalized value.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{ParamEvent, ParamId, ParamValue};
///
/// let change = ParamEvent {
///     param_id: ParamId(1),
///     value: ParamValue::new(0.75).unwrap(),
/// };
/// assert_eq!(change.param_id, ParamId(1));
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParamEvent {
    /// The parameter that changed.
    pub param_id: ParamId,
    /// The new normalized value (always finite and in `[0.0, 1.0]`,
    /// per the [`ParamValue`] NaN policy).
    pub value: ParamValue,
}

/// The payload of an [`Event`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new event kinds may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EventKind {
    /// A normalized parameter change ([`ParamEvent`]).
    ParamValue(ParamEvent),
    /// A channel-voice MIDI message ([`MidiEvent`] from
    /// `ruviform-media-core`). Native timestamps do not ride on the
    /// wire event — the frame offset *is* the delivery time; the
    /// source stamp lives in `TimedInputEvent` upstream.
    Midi(MidiEvent),
}

/// One sample-accurate event: a within-block frame offset plus its
/// [`EventKind`] payload.
///
/// `frame_offset` is raw at construction time; the bound
/// `frame_offset < block_frames` is enforced when events are grouped
/// into an [`EventList`]. `reserved` is an opaque extension field —
/// currently always `[0; 2]` — reserved for future payload kinds
/// (including musical-time data) without breaking the wire shape; the
/// same policy as `TransportSnapshot`'s `reserved` in
/// `ruviform-media-core`.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{Event, EventKind, ParamEvent, ParamId, ParamValue};
///
/// let event = Event::new(
///     128,
///     EventKind::ParamValue(ParamEvent {
///         param_id: ParamId(0),
///         value: ParamValue::new(0.5).unwrap(),
///     }),
/// );
/// assert_eq!(event.frame_offset(), 128);
/// assert_eq!(event.reserved(), [0; 2]);
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Event {
    frame_offset: u32,
    kind: EventKind,
    reserved: [u64; 2],
}

impl Event {
    /// Creates an event at `frame_offset` within its block; the offset
    /// bound itself is checked by [`EventList::new`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(frame_offset: u32, kind: EventKind) -> Self {
        Self {
            frame_offset,
            kind,
            reserved: [0; 2],
        }
    }

    /// The within-block frame offset — not an absolute position.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn frame_offset(self) -> u32 {
        self.frame_offset
    }

    /// The event payload.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn kind(self) -> EventKind {
        self.kind
    }

    /// Opaque extension reserve (currently always `[0; 2]`). Callers
    /// must not interpret its contents.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn reserved(&self) -> [u64; 2] {
        self.reserved
    }
}

/// A borrowed, validity-checked list of sample-accurate events for one
/// processing block.
///
/// `EventList` enforces the ordering and offset-bound semantics
/// described in the module docs at the boundary: construction scans
/// the slice once (`O(n)`, bounded, allocation-free) and rejects
/// unsorted input or out-of-block offsets. A constructed list is
/// sorted by [`Event::frame_offset`] and every offset satisfies
/// `offset < max(block_frames, 1)`.
///
/// **RT contract:** RT-suitable — borrowed view; construction is a
/// bounded scan with no allocation.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{Event, EventKind, EventList, ParamEvent, ParamId, ParamValue};
///
/// let events = [
///     Event::new(0, EventKind::ParamValue(ParamEvent {
///         param_id: ParamId(0), value: ParamValue::new(0.25).unwrap() })),
///     Event::new(64, EventKind::ParamValue(ParamEvent {
///         param_id: ParamId(0), value: ParamValue::new(0.75).unwrap() })),
/// ];
/// let list = EventList::new(&events, 128).unwrap();
/// assert_eq!(list.len(), 2);
///
/// // Unsorted input is rejected — the adapter sorts or refuses.
/// assert!(EventList::new(&[events[1], events[0]], 128).is_err());
/// // Offsets must stay inside the block.
/// assert!(EventList::new(&[events[1]], 64).is_err());
/// ```
#[derive(Copy, Clone, Debug)]
pub struct EventList<'a> {
    events: &'a [Event],
    block_frames: u32,
}

impl<'a> EventList<'a> {
    /// Creates an empty list for a block of `block_frames` frames —
    /// the no-events fast path.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn empty(block_frames: u32) -> Self {
        Self {
            events: &[],
            block_frames,
        }
    }

    /// Validates `events` against the block of `block_frames` frames
    /// and borrows them as a list.
    ///
    /// Offsets must satisfy `offset < block_frames`; when
    /// `block_frames == 0` only `offset == 0` is legal and every listed
    /// event is applied immediately (the zero-frame flush). See the
    /// module docs for the ordering contract — unsorted input is
    /// rejected, not re-ordered.
    ///
    /// **RT contract:** RT-suitable — `O(n)` bounded scan,
    /// allocation-free.
    ///
    /// # Errors
    ///
    /// - `Error::InvalidValue("event_offset")` — an offset reaches or
    ///   exceeds the block bound.
    /// - `Error::InvalidValue("event_order")` — offsets are not in
    ///   non-decreasing order.
    pub fn new(events: &'a [Event], block_frames: u32) -> Result<Self> {
        // A zero-frame block admits exactly offset 0: there is no
        // position inside an empty block, so every event flushes now.
        let bound = block_frames.max(1);
        let mut previous = 0_u32;
        for (index, event) in events.iter().enumerate() {
            let offset = event.frame_offset();
            if offset >= bound {
                return Err(Error::InvalidValue("event_offset"));
            }
            if index > 0 && offset < previous {
                return Err(Error::InvalidValue("event_order"));
            }
            previous = offset;
        }
        Ok(Self {
            events,
            block_frames,
        })
    }

    /// The frame length of the block this list belongs to.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn block_frames(self) -> u32 {
        self.block_frames
    }

    /// Number of events.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn len(self) -> usize {
        self.events.len()
    }

    /// Whether the list is empty.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.events.is_empty()
    }

    /// The events as a slice (sorted by frame offset).
    ///
    /// **RT contract:** RT-suitable — `const` borrow, pure.
    #[must_use]
    pub const fn as_slice(self) -> &'a [Event] {
        self.events
    }

    /// Iterates events in offset order by value.
    ///
    /// **RT contract:** RT-suitable — bounded iteration, no allocation.
    // must_use_candidate: the return `impl ExactSizeIterator` is already
    // must_use via the `Iterator` trait; `#[must_use]` would be double_must_use
    // (1.89-only suggestion, wrong direction).
    #[allow(clippy::must_use_candidate)]
    pub fn iter(self) -> impl ExactSizeIterator<Item = Event> + 'a {
        self.events.iter().copied()
    }
}

/// Sorts `events` by frame offset in place — stable, so the relative
/// order of same-offset events is preserved.
///
/// [`EventList::new`] requires sorted input and rejects unsorted
/// sequences. Adapters that receive unsorted host events normalize
/// them into pre-allocated scratch with this helper before
/// construction: unlike the standard library's stable
/// `sort`/`sort_by_key` (which may allocate auxiliary scratch) and
/// `sort_unstable` (which does not preserve same-offset order), this
/// is a bottom-up merge sort over a caller-provided `scratch` —
/// stable, allocation-free, and `O(n log n)` element moves over the
/// bounded per-block event count (the worst case — fully reversed
/// input — costs `n·⌈log2 n⌉ + 2n` copies, not `n²`).
///
/// `scratch` must be at least `events.len()` long; adapters carry it
/// in per-instance scratch reserved at activation/setup. A shorter
/// scratch is a contract violation handled by a defined fallback —
/// the bounded in-place insertion sort (still stable, still
/// allocation-free).
///
/// **RT contract:** RT-suitable — `O(n log n)` bounded element moves
/// on `Copy` elements, allocation-free when `scratch` is pre-sized.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{
///     sort_events_by_frame_offset, Event, EventKind, EventList,
///     ParamEvent, ParamId, ParamValue,
/// };
///
/// let mut events = [
///     Event::new(
///         96,
///         EventKind::ParamValue(ParamEvent {
///             param_id: ParamId(1),
///             value: ParamValue::new(0.5).unwrap(),
///         }),
///     ),
///     Event::new(
///         0,
///         EventKind::ParamValue(ParamEvent {
///             param_id: ParamId(1),
///             value: ParamValue::new(0.25).unwrap(),
///         }),
///     ),
/// ];
/// let mut sort_scratch = events;
/// sort_events_by_frame_offset(&mut events, &mut sort_scratch);
/// assert!(EventList::new(&events, 128).is_ok());
/// ```
pub fn sort_events_by_frame_offset(events: &mut [Event], scratch: &mut [Event]) {
    if scratch.len() < events.len() {
        // Contract violation: the documented precondition is a scratch
        // at least `events.len()` long. Fall back to the bounded
        // in-place sort rather than allocating or panicking.
        insertion_sort_by_frame_offset(events);
        return;
    }
    merge_sort_events(events, &mut scratch[..events.len()]);
}

/// In-place insertion sort — the defined fallback when the merge
/// scratch is undersized. Stable and allocation-free at `O(n²)`.
fn insertion_sort_by_frame_offset(events: &mut [Event]) {
    for index in 1..events.len() {
        let value = events[index];
        let mut hole = index;
        while hole > 0 && events[hole - 1].frame_offset() > value.frame_offset() {
            events[hole] = events[hole - 1];
            hole -= 1;
        }
        events[hole] = value;
    }
}

/// Bottom-up stable merge sort alternating between `events` and
/// `scratch` — `O(n log n)` bounded element copies, allocation-free.
/// Returns the number of element copies performed (worst-case
/// instrumentation for the RT bound).
fn merge_sort_events(events: &mut [Event], scratch: &mut [Event]) -> usize {
    let len = events.len();
    debug_assert_eq!(scratch.len(), len);
    if len < 2 {
        return 0;
    }
    let mut moves = len;
    scratch.copy_from_slice(events);
    // Tracks which buffer currently holds the sorted runs — passes
    // alternate `scratch → events` then `events → scratch`.
    let mut sorted_in_events = false;
    let mut width = 1_usize;
    while width < len {
        if sorted_in_events {
            merge_pass(&*events, scratch, width);
        } else {
            merge_pass(&*scratch, events, width);
        }
        sorted_in_events = !sorted_in_events;
        moves += len;
        width *= 2;
    }
    if !sorted_in_events {
        events.copy_from_slice(scratch);
        moves += len;
    }
    moves
}

/// Merges consecutive `width`-sized run pairs from `src` into `dst`.
fn merge_pass(src: &[Event], dst: &mut [Event], width: usize) {
    let len = src.len();
    let mut start = 0;
    while start < len {
        let mid = start.saturating_add(width).min(len);
        let end = start.saturating_add(2 * width).min(len);
        merge_runs(&src[start..mid], &src[mid..end], &mut dst[start..end]);
        start = end;
    }
}

/// Stable run merge — on equal offsets the left (earlier-arriving)
/// run's element wins, preserving input order.
fn merge_runs(left: &[Event], right: &[Event], dst: &mut [Event]) {
    let (mut left_index, mut right_index, mut out) = (0, 0, 0);
    while left_index < left.len() && right_index < right.len() {
        dst[out] = if left[left_index].frame_offset() <= right[right_index].frame_offset() {
            left_index += 1;
            left[left_index - 1]
        } else {
            right_index += 1;
            right[right_index - 1]
        };
        out += 1;
    }
    let left_rest = left.len() - left_index;
    dst[out..out + left_rest].copy_from_slice(&left[left_index..]);
    dst[out + left_rest..].copy_from_slice(&right[right_index..]);
}

/// Maximum number of ordered sources [`merge_call_events`] combines
/// in one call (release + plan + live uses three).
pub const MAX_MERGE_SOURCES: usize = 8;

/// The reason [`merge_call_events`] rejected its input — a defined
/// rejection, never a partial or silent drop.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MergeError {
    /// The merged event count exceeded the caller-provided scratch.
    Overflow {
        /// Elements the merge needed.
        needed: usize,
        /// Elements the scratch could hold.
        capacity: usize,
    },
    /// More sources than [`MAX_MERGE_SOURCES`] were supplied.
    TooManySources {
        /// The supplied source count.
        count: usize,
    },
}

impl core::fmt::Display for MergeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Overflow { needed, capacity } => write!(
                f,
                "event merge scratch overflow: needed {needed} > capacity {capacity}"
            ),
            Self::TooManySources { count } => {
                write!(
                    f,
                    "event merge source count {count} exceeds {MAX_MERGE_SOURCES}"
                )
            }
        }
    }
}

impl core::error::Error for MergeError {}

/// The [`LedgerKey`] a [`EventKind::Midi`] event tracks, or `None` for
/// non-note events.
fn event_ledger_key(event: Event) -> Option<LedgerKey> {
    match event.kind() {
        EventKind::Midi(midi) => midi.ledger_key(),
        _ => None,
    }
}

/// Whether `event` is a MIDI note-off for ordering purposes
/// (including the velocity-zero Note-On convention).
fn event_is_note_off(event: Event) -> bool {
    match event.kind() {
        EventKind::Midi(midi) => midi.is_note_off(),
        _ => false,
    }
}

/// Whether `event` is a MIDI note-on for ordering purposes
/// (a velocity-zero Note-On resolves to note-off).
fn event_is_note_on(event: Event) -> bool {
    match event.kind() {
        EventKind::Midi(midi) => matches!(midi.effective_note_state(), Some(true)),
        _ => false,
    }
}

/// Merges ordered event sources into one [`EventList`]-legal run —
/// the single authority for combining scheduled (plan) events with
/// live input events inside one processing block.
///
/// Every source slice must already be sorted by `frame_offset`
/// (non-decreasing) — the [`EventList`] wire invariant the merge
/// preserves. Sources are merged by a stable `k`-way selection; on
/// equal offsets the **earlier source index wins**, so callers
/// ordering their sources `[release, plan, live]` get a deterministic
/// plan-before-live tie-break while each source's own causal order is
/// always preserved.
///
/// ## Note-Off priority
///
/// Within a same-offset run, a MIDI note-off that shares its
/// `(group, channel, key)` with a note-on from a **different** source
/// is emitted **before** that note-on — the stuck-note fail-safe and
/// retrigger convention: a generated release must land before a new
/// attack on the same key. The convention applies only across sources;
/// authored order *within* one source is always preserved, so an
/// explicit same-frame play-then-release pair keeps its dead-note
/// semantics. Velocity-zero Note-Ons count as note-offs per the
/// MIDI 1.0 running-status convention.
///
/// `scratch` must hold `Σ sources[i].len()`; a smaller scratch is a
/// defined [`MergeOverflow`], never a partial merge. `src_scratch`
/// must be at least that long too — it receives each emitted event's
/// source index (transient provenance used by the collision pass).
///
/// **RT contract:** RT-suitable — `O(total · k + run²)` bounded work
/// on `Copy` elements (`k = sources.len()`, run = same-offset group);
/// no allocation.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MidiChannel, MidiEvent, MidiGroup, MidiKind, NoteKey};
/// use ruviform_plugin_core::{
///     merge_call_events, Event, EventKind, ParamEvent, ParamId, ParamValue,
/// };
///
/// let plan = [Event::new(64, EventKind::ParamValue(ParamEvent {
///     param_id: ParamId(0), value: ParamValue::new(0.5).unwrap() }))];
/// let note = |kind| Event::new(64, EventKind::Midi(MidiEvent {
///     group: MidiGroup::ZERO, channel: MidiChannel::new(0).unwrap(), kind }));
/// let live = [
///     note(MidiKind::NoteOn { key: NoteKey::new(60).unwrap(), velocity: 1 }),
///     note(MidiKind::NoteOff { key: NoteKey::new(60).unwrap(), velocity: 0 }),
/// ];
/// let mut scratch = [plan[0]; 8];
/// let mut src_scratch = [0u8; 8];
/// let merged =
///     merge_call_events(&[&plan, &live], &mut scratch, &mut src_scratch).unwrap();
/// // plan wins the offset tie; the authored live order is preserved.
/// let offs: Vec<u32> = merged.iter().map(|e| e.frame_offset()).collect();
/// assert_eq!(offs, [64, 64, 64]);
/// ```
pub fn merge_call_events<'a>(
    sources: &[&[Event]],
    scratch: &'a mut [Event],
    src_scratch: &mut [u8],
) -> core::result::Result<&'a mut [Event], MergeError> {
    if sources.len() > MAX_MERGE_SOURCES {
        return Err(MergeError::TooManySources {
            count: sources.len(),
        });
    }
    let needed: usize = sources.iter().map(|s| s.len()).sum();
    if scratch.len() < needed || src_scratch.len() < needed {
        return Err(MergeError::Overflow {
            needed,
            capacity: scratch.len().min(src_scratch.len()),
        });
    }
    let out = &mut scratch[..needed];
    let out_src = &mut src_scratch[..needed];
    // Stable k-way merge: pick the smallest offset; on ties prefer the
    // earlier source, preserving that source's own arrival order.
    let mut heads = [0usize; MAX_MERGE_SOURCES];
    let mut w = 0usize;
    loop {
        let mut best: Option<usize> = None;
        let mut best_off = u32::MAX;
        for (s, source) in sources.iter().enumerate() {
            if let Some(event) = source.get(heads[s])
                && event.frame_offset() < best_off
            {
                best = Some(s);
                best_off = event.frame_offset();
            }
        }
        let Some(s) = best else { break };
        out[w] = sources[s][heads[s]];
        out_src[w] = u8::try_from(s).unwrap_or_default();
        heads[s] += 1;
        w += 1;
    }
    debug_assert_eq!(w, needed);
    apply_note_off_priority(out, out_src);
    Ok(out)
}

/// Applies the same-offset Note-Off-priority convention in place:
/// inside each maximal same-offset run, every note-off that collides
/// with a same-key note-on from a different source is moved before
/// that note-on. Other events keep their relative order; authored
/// order within a single source is never disturbed.
fn apply_note_off_priority(events: &mut [Event], event_src: &mut [u8]) {
    let mut start = 0usize;
    while start < events.len() {
        let offset = events[start].frame_offset();
        let mut end = start + 1;
        while end < events.len() && events[end].frame_offset() == offset {
            end += 1;
        }
        note_off_priority_run(&mut events[start..end], &mut event_src[start..end]);
        start = end;
    }
}

/// Within one same-offset run, move a colliding note-off left past
/// cross-source events until it precedes the first colliding note-on.
/// A note-off "collides" when a note-on with the same [`LedgerKey`]
/// from a **different** source precedes it in the run. The off never
/// crosses an event from its own source: same-source authored order
/// encodes intent (a same-frame `[on, off]` pair is a legitimate
/// dead note, not a collision to normalize).
///
/// `run` and `run_src` are parallel views (event plus its source
/// index); both are rotated together so provenance stays aligned.
fn note_off_priority_run(run: &mut [Event], run_src: &mut [u8]) {
    // Position after the last off moved by this pass — later moved
    // offs land after it so moved offs keep their relative order.
    let mut frontier = 0usize;
    for i in 0..run.len() {
        if !event_is_note_off(run[i]) {
            continue;
        }
        let Some(off_key) = event_ledger_key(run[i]) else {
            continue;
        };
        // Earliest colliding on: different source, same key, before
        // the off. Same-source events before the off bound how far
        // left it may travel.
        let mut target = 0usize;
        let mut collides = false;
        for (j, event) in run.iter().enumerate().take(i) {
            if run_src[j] == run_src[i] {
                target = j + 1;
            } else if !collides
                && event_is_note_on(*event)
                && event_ledger_key(*event) == Some(off_key)
            {
                collides = true;
            }
        }
        if collides {
            target = target.max(frontier);
            if target < i {
                let off = run[i];
                let src = run_src[i];
                run.copy_within(target..i, target + 1);
                run_src.copy_within(target..i, target + 1);
                run[target] = off;
                run_src[target] = src;
                frontier = target + 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::{MidiChannel, MidiGroup, MidiKind, MidiProgram, MidiValue, NoteKey};

    fn param_event(offset: u32, value: f64) -> Event {
        Event::new(
            offset,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(value).unwrap(),
            }),
        )
    }

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

    fn key(v: u8) -> NoteKey {
        NoteKey::new(v).unwrap()
    }

    #[test]
    fn sorted_bounded_list_is_accepted() {
        let events = [
            param_event(0, 0.1),
            param_event(0, 0.2), // equal offsets keep insertion order
            param_event(127, 0.3),
        ];
        let list = EventList::new(&events, 128).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list.block_frames(), 128);
        let offsets: Vec<u32> = list.iter().map(Event::frame_offset).collect();
        assert_eq!(offsets, [0, 0, 127]);
    }

    #[test]
    fn unsorted_input_is_rejected() {
        let events = [param_event(10, 0.1), param_event(5, 0.2)];
        assert_eq!(
            EventList::new(&events, 128).unwrap_err(),
            Error::InvalidValue("event_order")
        );
    }

    #[test]
    fn offsets_are_bounded_by_block_frames() {
        let at_bound = [param_event(128, 0.1)];
        assert_eq!(
            EventList::new(&at_bound, 128).unwrap_err(),
            Error::InvalidValue("event_offset")
        );
        let just_inside = [param_event(127, 0.1)];
        assert!(EventList::new(&just_inside, 128).is_ok());
    }

    #[test]
    fn zero_frame_block_flushes_only_offset_zero() {
        let due = [param_event(0, 0.1), param_event(0, 0.2)];
        let list = EventList::new(&due, 0).unwrap();
        assert_eq!(list.block_frames(), 0);
        assert_eq!(list.len(), 2);

        let future = [param_event(1, 0.1)];
        assert_eq!(
            EventList::new(&future, 0).unwrap_err(),
            Error::InvalidValue("event_offset")
        );
    }

    #[test]
    fn empty_list_and_event_reserved() {
        let list = EventList::empty(256);
        assert!(list.is_empty());
        assert_eq!(list.as_slice(), &[]);
        assert_eq!(param_event(0, 0.5).reserved(), [0; 2]);
    }

    #[test]
    fn sort_events_is_stable_and_alloc_free() {
        // Same-offset events keep their input order (stability) — the
        // distinguishable values ride on the payload.
        let mut events = [
            param_event(10, 0.1),
            param_event(5, 0.5),
            param_event(10, 0.2),
            param_event(0, 0.9),
            param_event(5, 0.6),
        ];
        let mut scratch = events;
        sort_events_by_frame_offset(&mut events, &mut scratch);
        let pairs: Vec<(u32, f64)> = events
            .iter()
            .map(|event| {
                (
                    event.frame_offset(),
                    match event.kind() {
                        EventKind::ParamValue(param) => param.value.get(),
                        _ => f64::NAN,
                    },
                )
            })
            .collect();
        assert_eq!(pairs, [(0, 0.9), (5, 0.5), (5, 0.6), (10, 0.1), (10, 0.2)]);

        let mut already_sorted = [param_event(0, 0.1), param_event(5, 0.5)];
        let mut scratch = already_sorted;
        sort_events_by_frame_offset(&mut already_sorted, &mut scratch);
        assert_eq!(already_sorted[1].frame_offset(), 5);
    }

    /// Worst-case cost is `O(n log n)`, measured — not just
    /// allocation-free: fully reversed input at the VST3 adapter bound
    /// (`MAX_BLOCK_EVENTS = 1_024`) costs `n·⌈log2 n⌉ + 2n` element
    /// copies (≈12k) where the previous insertion sort moved ≈524k.
    #[test]
    fn rt_sort_worst_case_moves_stay_n_log_n() {
        const LEN: usize = 1_024;
        let mut events = vec![param_event(0, 0.0); LEN];
        for (index, event) in events.iter_mut().enumerate() {
            *event = param_event(
                u32::try_from(LEN - index).unwrap_or(u32::MAX),
                f64::from(u32::try_from(index).unwrap_or(u32::MAX)),
            );
        }
        let mut scratch = vec![param_event(0, 0.0); LEN];
        let moves = merge_sort_events(&mut events, &mut scratch);
        assert!(
            events
                .windows(2)
                .all(|pair| pair[0].frame_offset() <= pair[1].frame_offset())
        );
        // ⌈log2 LEN⌉ passes + the initial and possible final copy.
        let passes = usize::try_from(usize::BITS - (LEN - 1).leading_zeros()).unwrap_or(usize::MAX);
        assert!(
            moves <= LEN * (passes + 2),
            "moves={moves} exceeds n·⌈log2 n⌉+2n"
        );
    }

    /// An undersized scratch is a defined fallback — the bounded
    /// in-place insertion sort — not a panic or an allocation.
    #[test]
    fn sort_events_undersized_scratch_falls_back_to_insertion() {
        let mut events = [
            param_event(10, 0.1),
            param_event(5, 0.5),
            param_event(0, 0.9),
        ];
        let mut scratch = [param_event(0, 0.0)];
        sort_events_by_frame_offset(&mut events, &mut scratch);
        assert_eq!(
            events
                .iter()
                .map(|event| event.frame_offset())
                .collect::<Vec<_>>(),
            [0, 5, 10]
        );
    }

    #[test]
    fn merge_call_events_stable_plan_before_live() {
        let plan = [param_event(32, 0.1), param_event(64, 0.2)];
        let live = [
            midi_event(
                64,
                MidiKind::ProgramChange {
                    program: MidiProgram::new(5).unwrap(),
                },
            ),
            midi_event(
                96,
                MidiKind::PitchBend {
                    value: MidiValue::bend_from_u14(0x2000),
                },
            ),
        ];
        let mut scratch = [param_event(0, 0.0); 8];
        let mut src_scratch = [0u8; 8];
        let merged = merge_call_events(&[&plan, &live], &mut scratch, &mut src_scratch).unwrap();
        // plan wins the offset-64 tie.
        assert!(matches!(merged[1].kind(), EventKind::ParamValue(_)));
        assert!(matches!(merged[2].kind(), EventKind::Midi(_)));
        assert!(EventList::new(merged, 128).is_ok());
    }

    #[test]
    fn merge_call_events_note_off_priority_same_key() {
        // Live NoteOn then generated release NoteOff at the same offset
        // and key: the off must be emitted first.
        let on = midi_event(
            0,
            MidiKind::NoteOn {
                key: key(60),
                velocity: MidiValue::from_u7(100),
            },
        );
        let off = midi_event(
            0,
            MidiKind::NoteOff {
                key: key(60),
                velocity: 0,
            },
        );
        let live = [on];
        let release = [off];
        let mut scratch = [param_event(0, 0.0); 8];
        let mut src_scratch = [0u8; 8];
        let merged = merge_call_events(&[&release, &live], &mut scratch, &mut src_scratch).unwrap();
        assert!(matches!(
            merged[0].kind(),
            EventKind::Midi(m) if m.is_note_off()
        ));
        assert!(matches!(
            merged[1].kind(),
            EventKind::Midi(m) if matches!(m.effective_note_state(), Some(true))
        ));
        // Unrelated key: order untouched.
        let on_other = midi_event(
            0,
            MidiKind::NoteOn {
                key: key(64),
                velocity: MidiValue::from_u7(100),
            },
        );
        let live2 = [on_other, on];
        let mut scratch2 = [param_event(0, 0.0); 8];
        let mut src_scratch2 = [0u8; 8];
        let merged2 = merge_call_events(&[&live2], &mut scratch2, &mut src_scratch2).unwrap();
        assert_eq!(merged2.len(), 2);
        assert!(matches!(
            merged2[0].kind(),
            EventKind::Midi(m) if matches!(m.kind, MidiKind::NoteOn{key: k,..} if k == key(64))
        ));
    }

    #[test]
    fn merge_preserves_authored_same_source_on_off_order() {
        // A same-offset `[on, off]` pair authored inside ONE source —
        // a legitimate zero-length note — must keep its order: inverting
        // it to `[off, on]` would leave the note sounding forever.
        let on = midi_event(
            0,
            MidiKind::NoteOn {
                key: key(60),
                velocity: MidiValue::from_u7(100),
            },
        );
        let off = midi_event(
            0,
            MidiKind::NoteOff {
                key: key(60),
                velocity: 0,
            },
        );
        let live = [on, off];
        let mut scratch = [param_event(0, 0.0); 8];
        let mut src_scratch = [0u8; 8];
        let merged = merge_call_events(&[&live], &mut scratch, &mut src_scratch).unwrap();
        assert!(matches!(
            merged[0].kind(),
            EventKind::Midi(m) if matches!(m.effective_note_state(), Some(true))
        ));
        assert!(matches!(
            merged[1].kind(),
            EventKind::Midi(m) if m.is_note_off()
        ));
    }

    #[test]
    fn merge_off_priority_still_normalizes_cross_source() {
        // On from an earlier source, off from a later source at the
        // same offset/key — the convention still lifts the off before
        // the on.
        let on = midi_event(
            0,
            MidiKind::NoteOn {
                key: key(60),
                velocity: MidiValue::from_u7(100),
            },
        );
        let off = midi_event(
            0,
            MidiKind::NoteOff {
                key: key(60),
                velocity: 0,
            },
        );
        let plan = [on];
        let live = [off];
        let mut scratch = [param_event(0, 0.0); 8];
        let mut src_scratch = [0u8; 8];
        let merged = merge_call_events(&[&plan, &live], &mut scratch, &mut src_scratch).unwrap();
        assert!(matches!(
            merged[0].kind(),
            EventKind::Midi(m) if m.is_note_off()
        ));
        assert!(matches!(
            merged[1].kind(),
            EventKind::Midi(m) if matches!(m.effective_note_state(), Some(true))
        ));
    }

    #[test]
    fn merge_call_events_rejects_overflow_and_too_many_sources() {
        let one = [param_event(0, 0.1)];
        let mut small = [param_event(0, 0.0); 1];
        let mut small_src = [0u8; 1];
        assert_eq!(
            merge_call_events(&[&one, &one], &mut small, &mut small_src),
            Err(MergeError::Overflow {
                needed: 2,
                capacity: 1,
            })
        );
        let sources = [&one[..]; 9];
        let mut big = [param_event(0, 0.0); 16];
        let mut big_src = [0u8; 16];
        assert_eq!(
            merge_call_events(&sources, &mut big, &mut big_src),
            Err(MergeError::TooManySources { count: 9 })
        );
    }

    /// RT contract: merge is bounded copy/compare work — no allocation.
    #[test]
    fn rt_merge_call_events_no_alloc() {
        let plan = [param_event(32, 0.1), param_event(64, 0.2)];
        let live = [midi_event(
            64,
            MidiKind::NoteOff {
                key: key(60),
                velocity: 0,
            },
        )];
        let mut scratch = [param_event(0, 0.0); 8];
        let mut src_scratch = [0u8; 8];
        let _ = assert_no_alloc::assert_no_alloc(|| {
            core::hint::black_box(merge_call_events(
                &[&plan, &live],
                &mut scratch,
                &mut src_scratch,
            ))
            .unwrap()
        });
    }
}
