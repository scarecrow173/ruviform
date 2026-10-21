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
//! - **Process status / tail** — the neutral [`ProcessResult`] mapping
//!   lives in [`crate::processor`]; adapters translate it to each
//!   standard's own status codes.
//!
//! [`MediaTime`]: ruviform_media_core::MediaTime

use ruviform_media_core::{Error, Result};

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

#[cfg(test)]
mod tests {
    use super::*;

    fn param_event(offset: u32, value: f64) -> Event {
        Event::new(
            offset,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(value).unwrap(),
            }),
        )
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
}
