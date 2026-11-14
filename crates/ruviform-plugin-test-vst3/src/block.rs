//! Per-block buffer preparation: the testable, mostly-safe core of the
//! VST3 `process` path.
//!
//! The FFI layer (`crate::component`) extracts the host's raw
//! channel-pointer lists (`channelBuffers32` → `&[*mut f32]` per bus)
//! and hands them to [`prepare_block`], which performs every spec'd
//! boundary validation and produces zero-copy
//! [`PlanarBufferView`]/[`PlanarBufferViewMut`] views over the host
//! buffers. [`finish_block`] writes the per-channel bounce buffers back
//! for any output channel that was bounced instead of touched directly.
//!
//! ## Channel-count policy
//!
//! The module declares a stereo (`kMain`, 2ch) bus pair, but `process`
//! accepts 1–2 channels per bus — the gain law is per-channel and
//! channel-count agnostic, so mono and stereo blocks are both legal
//! inputs. [`ChannelScratch`] is sized to the *capacity* (2 channels
//! per side); each block uses its actual negotiated count.
//!
//! ## Aliasing / in-place policy
//!
//! Only *output* channels ever bounce. Per output channel `j`, the
//! `[ptr, ptr + frames*4)` sample range is compared against every
//! input channel range and every earlier output channel range. An
//! exact-pointer match (`out[j] == in[i]` or `out[j] == out[k], k < j`)
//! routes the output through a per-instance bounce buffer and
//! [`finish_block`] copies it back — deterministic last-writer-wins
//! for duplicated outputs. A *partial* overlap (ranges intersect but
//! starts differ) is a malformed host buffer list and rejects the
//! block — it could never be a meaningful in-place mapping.
//! Inputs are read-only `&[f32]`: they may freely alias each other or
//! the host memory of *bounced* outputs (never written during the
//! block). Non-bounced outputs are pairwise disjoint and disjoint from
//! every input, so no `&[f32]`/`&mut [f32]` (or `&mut`/`&mut`) pair
//! ever covers one region.
//!
//! ## Allocation contract
//!
//! [`ChannelScratch`]/[`EventScratch`] are per-plugin-instance state
//! allocated at `setupProcessing` time (non-RT) sized to the channel
//! capacity and `maxSamplesPerBlock`.
//! `prepare_block`/`finish_block` never allocate — the `rt_` tests
//! wrap the path in `assert_no_alloc`, so any (de)allocation aborts
//! the test binary instead of relying on documented contract alone.

use std::mem::{align_of, size_of};
use std::ptr::{self, NonNull};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
use ruviform_plugin_core::Event;

/// Maximum parameter events carried per block; host output beyond the
/// capacity is dropped (defined truncation).
pub(crate) const MAX_BLOCK_EVENTS: usize = 1_024;

/// Why a block was rejected at the adapter boundary.
///
/// Every variant maps to a defined behavior at the FFI boundary — the
/// block is skipped (`EmptyBlock`) or rejected with `kResultFalse` —
/// never UB or a silent channel index shift.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Rejection {
    /// `numSamples == 0`: not an error — the block carries events only;
    /// the caller routes it through the event-only (zero-frame flush)
    /// path and never touches buffers.
    EmptyBlock,
    /// `numSamples` exceeds `maxSamplesPerBlock` negotiated at
    /// `setupProcessing` (mirrored as `ProcessConfig::max_frames`).
    TooManyFrames,
    /// A bus's channel count is zero or exceeds the negotiated
    /// capacity.
    ChannelCount,
    /// A channel pointer inside a non-empty bus is null.
    NullChannel,
    /// A channel pointer is not 4-byte (`f32`) aligned.
    Misaligned,
    /// An output channel's sample range *partially* overlaps an
    /// input or an earlier output channel (exact-pointer equality is
    /// the bounce case, not a rejection — but a shifted overlap is a
    /// malformed host buffer list).
    Overlap,
}

/// Per-instance event list scratch — bounded at [`MAX_BLOCK_EVENTS`]
/// with capacity reserved at `configure` time. Sorted by frame offset
/// before [`EventList`][ruviform_plugin_core::EventList] validation —
/// the adapter-side sort the ordering contract allows for unsorted
/// host input. `sort_buf` backs the merge sort's alternating buffer
/// so the sort stays allocation-free on the RT path.
pub(crate) struct EventScratch {
    events: Vec<Event>,
    sort_buf: Vec<Event>,
}

impl EventScratch {
    /// An empty scratch — allocates nothing.
    ///
    /// **RT contract:** RT-suitable — pure.
    pub(crate) const fn empty() -> Self {
        Self {
            events: Vec::new(),
            sort_buf: Vec::new(),
        }
    }

    /// Reserves event capacity — the `setupProcessing` allocation
    /// point. Non-RT only.
    pub(crate) fn configure(&mut self) {
        self.events.clear();
        self.events.reserve(MAX_BLOCK_EVENTS);
        self.sort_buf.clear();
        self.sort_buf.reserve(MAX_BLOCK_EVENTS);
    }

    /// Clears the per-block list.
    ///
    /// **RT contract:** RT-suitable — `O(1)` length reset.
    pub(crate) fn clear(&mut self) {
        self.events.clear();
    }

    /// Pushes an event; `false` when the bounded capacity is full
    /// (defined truncation — the point is dropped).
    ///
    /// **RT contract:** RT-suitable — bounded, allocation-free while
    /// `len() < MAX_BLOCK_EVENTS`.
    pub(crate) fn push(&mut self, event: Event) -> bool {
        if self.events.len() >= MAX_BLOCK_EVENTS {
            return false;
        }
        self.events.push(event);
        true
    }

    /// Stable-sorts by frame offset — allocation-free over the
    /// pre-reserved `sort_buf` (the shared merge sort needs only this
    /// buffer).
    ///
    /// **RT contract:** `O(n log n)` bounded element copies over at
    /// most `MAX_BLOCK_EVENTS` elements.
    pub(crate) fn sort(&mut self) {
        // `sort_buf` is filled to the event count so it satisfies the
        // merge sort's `len >= events.len()` precondition without a
        // dummy element; capacity was reserved at `configure`.
        self.sort_buf.clear();
        self.sort_buf.extend_from_slice(&self.events);
        ruviform_plugin_core::sort_events_by_frame_offset(&mut self.events, &mut self.sort_buf);
    }

    /// The collected events as a slice.
    ///
    /// **RT contract:** RT-suitable — borrow.
    #[must_use]
    pub(crate) fn as_slice(&self) -> &[Event] {
        &self.events
    }
}

/// Per-instance channel scratch, allocated at `setupProcessing`
/// (non-RT): raw fat-pointer slice lists, the in-place bounce region
/// and the writeback bookkeeping. Sized to the channel *capacity*
/// (2 per side); each block uses its actual channel count.
///
/// The slice lists hold `*const [f32]`/`*mut [f32]` *raw* fat pointers
/// so no reference with a fake lifetime ever lives in the struct; the
/// single place where they become `&[f32]`/`&mut [f32]` is the scoped
/// reborrow inside [`prepare_block`] (see its SAFETY comments).
pub(crate) struct ChannelScratch {
    /// `*const [f32]` per input channel slot, rebuilt each block.
    in_slices: Vec<*const [f32]>,
    /// `*mut [f32]` per output channel slot, rebuilt each block —
    /// either a host channel or a bounce chunk.
    out_slices: Vec<*mut [f32]>,
    /// Host output pointer per output channel slot (writeback target).
    out_host: Vec<*mut f32>,
    /// Whether output channel slot `j` was bounced this block.
    bounced: Vec<bool>,
    /// Output channel count used by the current block — `finish_block`
    /// only inspects this prefix.
    out_used: usize,
    /// Flat bounce region: `out_capacity * max_frames` samples.
    bounce: Vec<f32>,
    in_capacity: usize,
    out_capacity: usize,
    max_frames: usize,
}

impl ChannelScratch {
    /// An empty scratch for the pre-`setupProcessing` state —
    /// allocates nothing (`Vec::new` never allocates).
    ///
    /// **RT contract:** RT-suitable — pure.
    pub(crate) const fn empty() -> Self {
        Self {
            in_slices: Vec::new(),
            out_slices: Vec::new(),
            out_host: Vec::new(),
            bounced: Vec::new(),
            out_used: 0,
            bounce: Vec::new(),
            in_capacity: 0,
            out_capacity: 0,
            max_frames: 0,
        }
    }

    /// (Re)allocates the scratch for `max_frames` frames and the
    /// channel capacities — the `setupProcessing` allocation point.
    /// Non-RT only.
    pub(crate) fn configure(&mut self, in_capacity: usize, out_capacity: usize, max_frames: usize) {
        self.in_capacity = in_capacity;
        self.out_capacity = out_capacity;
        self.max_frames = max_frames;
        self.out_used = 0;
        // A dangling fat pointer is a safe placeholder: it is rewritten
        // every block before the list is reborrowed as slices.
        let dangling_in = ptr::slice_from_raw_parts::<f32>(NonNull::dangling().as_ptr(), 0);
        let dangling_out = ptr::slice_from_raw_parts_mut::<f32>(NonNull::dangling().as_ptr(), 0);
        self.in_slices.resize(in_capacity, dangling_in);
        self.out_slices.resize(out_capacity, dangling_out);
        self.out_host.resize(out_capacity, ptr::null_mut());
        self.bounced.resize(out_capacity, false);
        self.bounce.resize(out_capacity * max_frames, 0.0);
    }

    /// The negotiated maximum block length.
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) fn max_frames(&self) -> usize {
        self.max_frames
    }
}

/// Maps a raw host sample offset to a block-relative frame offset:
/// `Some` when `0 <= raw_offset < max(frames, 1)` (offset 0 stays
/// legal for a zero-frame block, matching `EventList`'s flush
/// semantics); `None` — the event is dropped — otherwise. Out-of-block
/// events are dropped rather than clamped onto a wrong frame.
///
/// **RT contract:** RT-suitable — pure.
#[must_use]
pub(crate) fn block_offset(raw_offset: i32, frames: usize) -> Option<u32> {
    let raw = u32::try_from(raw_offset).ok()?;
    let hi = u32::try_from(frames.max(1)).unwrap_or(u32::MAX);
    (raw < hi).then_some(raw)
}

/// The validated, view-ready products of [`prepare_block`] — borrows
/// the scratch for the remainder of the call.
#[derive(Debug)]
pub(crate) struct PreparedBlock<'a> {
    /// Read view over the negotiated input bus (zero-copy host borrows).
    pub(crate) input: PlanarBufferView<'a>,
    /// Write view over the negotiated output bus — direct host borrows
    /// for non-aliased channels, bounce-backed for aliased ones.
    pub(crate) output: PlanarBufferViewMut<'a>,
}

/// Validates the host channel-pointer lists and builds zero-copy
/// borrowed views over them.
///
/// `input_ptrs`/`output_ptrs` are the channel-pointer lists of the
/// input/output bus (already extracted from `channelBuffers32` by the
/// FFI layer — pointer-list reads are the FFI's job). `frames` is
/// `numSamples` — zero is a defined skip. Each side must carry 1..
/// =capacity channels.
///
/// Boundary validation, in order: `frames` non-zero and within
/// `scratch.max_frames`, channel counts within capacity, every channel
/// pointer non-null and 4-byte aligned. Then per-channel alias
/// detection (module docs) routes colliding outputs through the
/// bounce region.
///
/// **RT contract:** RT-suitable — `O(in + out²)` bounded scans
/// (channel counts are small); allocation-free.
///
/// # Errors
///
/// [`Rejection`] — every variant is a defined skip/reject at the FFI
/// boundary, never UB.
#[expect(unsafe_code)]
pub(crate) fn prepare_block<'a>(
    input_ptrs: &[*mut f32],
    output_ptrs: &[*mut f32],
    frames: usize,
    scratch: &'a mut ChannelScratch,
) -> Result<PreparedBlock<'a>, Rejection> {
    if frames == 0 {
        return Err(Rejection::EmptyBlock);
    }
    if frames > scratch.max_frames {
        return Err(Rejection::TooManyFrames);
    }
    if input_ptrs.is_empty()
        || output_ptrs.is_empty()
        || input_ptrs.len() > scratch.in_capacity
        || output_ptrs.len() > scratch.out_capacity
    {
        return Err(Rejection::ChannelCount);
    }
    for &ptr in input_ptrs.iter().chain(output_ptrs.iter()) {
        if ptr.is_null() {
            return Err(Rejection::NullChannel);
        }
        if !(ptr as usize).is_multiple_of(align_of::<f32>()) {
            return Err(Rejection::Misaligned);
        }
    }

    // Input channels: read-only borrows of host memory. Aliasing
    // between inputs is legal (`&[f32]` only); a bounced output's host
    // region may also alias an input — it is written only by
    // `finish_block`, after the views die.
    for (slot, &ptr) in scratch.in_slices.iter_mut().zip(input_ptrs.iter()) {
        *slot = ptr::slice_from_raw_parts(ptr, frames);
    }

    // Output channels: per-channel *range* overlap detection — an
    // exact-pointer match bounces, a shifted/partial overlap rejects
    // the block (`Rejection::Overlap`).
    let bytes = frames * size_of::<f32>();
    // Reset the whole capacity's bounce flags: `finish_block` iterates
    // the capacity prefix, and a smaller block must not inherit a
    // previous block's stale bounce marks.
    scratch.bounced.fill(false);
    for (j, &ptr) in output_ptrs.iter().enumerate() {
        // `saturating_add`: a hostile pointer near the address-space
        // top must not wrap the range empty — saturation makes every
        // comparison overlap and the block rejects.
        let start = ptr.addr();
        let end = start.saturating_add(bytes);
        let mut bounced = false;
        for &other in input_ptrs.iter().chain(&output_ptrs[..j]) {
            let other_start = other.addr();
            let other_end = other_start.saturating_add(bytes);
            if other_start < end && start < other_end {
                if other_start == start {
                    bounced = true;
                } else {
                    return Err(Rejection::Overlap);
                }
            }
        }
        scratch.bounced[j] = bounced;
        scratch.out_host[j] = ptr;
        scratch.out_slices[j] = if bounced {
            let chunk = &mut scratch.bounce[j * scratch.max_frames..][..frames];
            ptr::slice_from_raw_parts_mut(chunk.as_mut_ptr(), frames)
        } else {
            ptr::slice_from_raw_parts_mut(ptr, frames)
        };
    }
    scratch.out_used = output_ptrs.len();

    let in_list: &'a [&'a [f32]] = {
        let raw: &mut [*const [f32]] = &mut scratch.in_slices[..input_ptrs.len()];
        // SAFETY: `*const [f32]` and `&[f32]` share the same fat-pointer
        // layout (data pointer + length), so reinterpreting only
        // attaches the lifetime. Every element was written above from
        // a validated non-null, aligned host channel pointer whose
        // memory the VST3 contract guarantees valid and un-mutated for
        // the rest of this `process` call (`'a` never outlives it —
        // the list is a borrow of `scratch`, dropped with the call).
        // Input channels may alias each other — shared `&` borrows
        // are legal — and may alias the host region of a *bounced*
        // output channel, which is not written while the views live.
        unsafe { std::slice::from_raw_parts(raw.as_ptr().cast::<&'a [f32]>(), raw.len()) }
    };
    let input = PlanarBufferView::new(in_list).map_err(|_| Rejection::ChannelCount)?;

    let out_list: &'a mut [&'a mut [f32]] = {
        let raw: &mut [*mut [f32]] = &mut scratch.out_slices[..output_ptrs.len()];
        // SAFETY: `*mut [f32]` and `&mut [f32]` share the same
        // fat-pointer layout. Every element was written above from
        // either a validated non-null, aligned host channel pointer
        // (host memory valid for `'a`) or this instance's private
        // bounce chunk (valid for `'a`). By construction every `&mut`
        // covers a distinct region: alias-detected channels point into
        // disjoint bounce chunks and non-bounced channels are pairwise
        // disjoint and disjoint from every input slice — no `&`/`&mut`
        // or `&mut`/`&mut` pair ever covers one region.
        unsafe {
            std::slice::from_raw_parts_mut(raw.as_mut_ptr().cast::<&'a mut [f32]>(), raw.len())
        }
    };
    let output = PlanarBufferViewMut::new(out_list).map_err(|_| Rejection::ChannelCount)?;

    Ok(PreparedBlock { input, output })
}

/// Writes bounced output channels back to their host buffers — called
/// after the processor ran and the views were dropped.
///
/// **RT contract:** RT-suitable — `O(bounced × frames)` bounded copy;
/// allocation-free.
///
/// # Safety contract (internal)
///
/// Relies on `prepare_block` having run this block: `out_host[j]` is a
/// validated host channel pointer, the bounce chunk is the same one the
/// output view wrote, and the two regions are disjoint (bounce memory
/// is private to this instance).
#[expect(unsafe_code)]
pub(crate) fn finish_block(scratch: &mut ChannelScratch, frames: usize) {
    for j in 0..scratch.out_used {
        if scratch.bounced[j] {
            let chunk = &scratch.bounce[j * scratch.max_frames..][..frames];
            // SAFETY: `out_host[j]` was validated non-null and 4-byte
            // aligned in `prepare_block`, and the VST3 contract
            // guarantees it remains valid for `frames` elements for
            // the rest of this `process` call. The bounce chunk is
            // instance-private memory — the regions are disjoint.
            unsafe { std::slice::from_raw_parts_mut(scratch.out_host[j], frames) }
                .copy_from_slice(chunk);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_plugin_core::{EventKind, ParamEvent, ParamId, ParamValue};

    /// Issue #22 RT-verification lane: `AllocDisabler` swaps the test
    /// binary's allocator so `assert_no_alloc` catches any
    /// (de)allocation inside its measurement window. One static per
    /// crate test binary; `component.rs` tests measure against the
    /// same allocator.
    #[cfg(debug_assertions)]
    #[global_allocator]
    static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

    fn scratch_stereo(max_frames: usize) -> ChannelScratch {
        let mut scratch = ChannelScratch::empty();
        scratch.configure(2, 2, max_frames);
        scratch
    }

    fn ptrs(bufs: &mut [Vec<f32>]) -> Vec<*mut f32> {
        bufs.iter_mut().map(Vec::as_mut_ptr).collect()
    }

    #[test]
    fn prepare_builds_zero_copy_views() {
        let mut input = [vec![0.5_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let in_ptrs = ptrs(&mut input);
        let out_ptrs = ptrs(&mut output);
        let mut scratch = scratch_stereo(64);

        {
            let mut prepared = prepare_block(&in_ptrs, &out_ptrs, 64, &mut scratch).unwrap();
            assert_eq!(prepared.input.channels(), 2);
            assert_eq!(prepared.input.frames(), 64);
            assert_eq!(prepared.input.channel(0), Some(&input[0][..]));
            prepared.output.channel_mut(0).unwrap().fill(0.75);
        }
        assert_eq!(output[0], vec![0.75_f32; 64]);
        assert_eq!(output[1], vec![0.0_f32; 64]);
        // Non-aliased outputs wrote host memory directly — zero copy.
        assert!(!scratch.bounced.iter().any(|&b| b));
    }

    #[test]
    fn prepare_accepts_mono_within_capacity() {
        let mut input = [vec![0.5_f32; 32]];
        let mut output = [vec![0.0_f32; 32]];
        let in_ptrs = ptrs(&mut input);
        let out_ptrs = ptrs(&mut output);
        let mut scratch = scratch_stereo(32);

        let prepared = prepare_block(&in_ptrs, &out_ptrs, 32, &mut scratch).unwrap();
        assert_eq!(prepared.input.channels(), 1);
        assert_eq!(prepared.output.channels(), 1);
    }

    #[test]
    fn prepare_rejects_each_boundary_violation() {
        let mut input = [vec![0.0_f32; 16], vec![0.0; 16]];
        let mut output = [vec![0.0_f32; 16], vec![0.0; 16]];
        let in_ptrs = ptrs(&mut input);
        let out_ptrs = ptrs(&mut output);
        let mut scratch = scratch_stereo(16);

        assert_eq!(
            prepare_block(&in_ptrs, &out_ptrs, 0, &mut scratch).unwrap_err(),
            Rejection::EmptyBlock
        );
        assert_eq!(
            prepare_block(&in_ptrs, &out_ptrs, 17, &mut scratch).unwrap_err(),
            Rejection::TooManyFrames
        );
        assert_eq!(
            prepare_block(&[], &out_ptrs, 16, &mut scratch).unwrap_err(),
            Rejection::ChannelCount
        );
        let mut three_in = in_ptrs.clone();
        three_in.push(out_ptrs[0]);
        assert_eq!(
            prepare_block(&three_in, &out_ptrs, 16, &mut scratch).unwrap_err(),
            Rejection::ChannelCount
        );
        let mut null_in = in_ptrs.clone();
        null_in[1] = ptr::null_mut();
        assert_eq!(
            prepare_block(&null_in, &out_ptrs, 16, &mut scratch).unwrap_err(),
            Rejection::NullChannel
        );
        let mut misaligned = out_ptrs.clone();
        // +1 byte past a valid f32 base — still in-bounds region, wrong align.
        misaligned[0] = out_ptrs[0].wrapping_byte_add(1);
        assert_eq!(
            prepare_block(&in_ptrs, &misaligned, 16, &mut scratch).unwrap_err(),
            Rejection::Misaligned
        );
    }

    #[test]
    fn in_place_aliased_output_bounces_and_writes_back() {
        let mut input = [vec![0.5_f32; 16], vec![-0.25; 16]];
        let mut output = [vec![0.0_f32; 16], vec![0.0; 16]];
        let in_ptrs = ptrs(&mut input);
        // Output channel 0 aliases input channel 0 exactly (in-place);
        // output channel 1 is a distinct host buffer.
        let out_ptrs = vec![in_ptrs[0], ptrs(&mut output)[1]];
        let mut scratch = scratch_stereo(16);

        {
            let mut prepared = prepare_block(&in_ptrs, &out_ptrs, 16, &mut scratch).unwrap();
            prepared.output.channel_mut(0).unwrap().fill(0.75);
            prepared.output.channel_mut(1).unwrap().fill(-0.5);
        }
        assert!(scratch.bounced[0]);
        assert!(!scratch.bounced[1]);
        finish_block(&mut scratch, 16);
        // Bounced channel 0 wrote back into the aliased input buffer.
        assert_eq!(input[0], vec![0.75_f32; 16]);
        assert_eq!(output[1], vec![-0.5_f32; 16]);
    }

    #[test]
    fn partial_overlap_rejects() {
        let mut input = [vec![0.0_f32; 16], vec![0.0; 16]];
        let mut output = [vec![0.0_f32; 16], vec![0.0; 16]];
        let in_ptrs = ptrs(&mut input);
        let mut out_ptrs = ptrs(&mut output);
        // Shift output channel 0 one sample into input channel 0 —
        // partial overlap, not an exact-pointer match.
        out_ptrs[0] = in_ptrs[0].wrapping_add(1);
        let mut scratch = scratch_stereo(16);
        assert_eq!(
            prepare_block(&in_ptrs, &out_ptrs, 16, &mut scratch).unwrap_err(),
            Rejection::Overlap
        );
    }

    #[test]
    fn duplicated_output_channels_bounce_last_writer_wins() {
        let mut input = [vec![0.5_f32; 8], vec![0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        let in_ptrs = ptrs(&mut input);
        let out_ptrs = ptrs(&mut output);
        // Both output channels point at the same host buffer: channel
        // 0 writes it directly, channel 1 (exact duplicate of an
        // earlier output) bounces.
        let dup_ptrs = vec![out_ptrs[0], out_ptrs[0]];
        let mut scratch = scratch_stereo(8);
        {
            let mut prepared = prepare_block(&in_ptrs, &dup_ptrs, 8, &mut scratch).unwrap();
            prepared.output.channel_mut(0).unwrap().fill(1.0);
            prepared.output.channel_mut(1).unwrap().fill(2.0);
        }
        assert!(!scratch.bounced[0]);
        assert!(scratch.bounced[1]);
        finish_block(&mut scratch, 8);
        // Deterministic last-writer-wins: channel 1 wrote last.
        assert_eq!(output[0], vec![2.0_f32; 8]);
    }

    #[test]
    fn smaller_block_does_not_inherit_stale_bounce_flags() {
        let mut input = [vec![0.5_f32; 16], vec![0.5; 16]];
        let mut output = [vec![0.0_f32; 16], vec![0.0; 16]];
        let in_ptrs = ptrs(&mut input);
        let mut scratch = scratch_stereo(16);
        // Block 1: stereo with channel 0 aliased → bounced.
        {
            let out_ptrs = vec![in_ptrs[0], ptrs(&mut output)[1]];
            let _prepared = prepare_block(&in_ptrs, &out_ptrs, 16, &mut scratch).unwrap();
        }
        finish_block(&mut scratch, 16);
        assert!(scratch.bounced[0]);
        // Block 2: mono, non-aliased — the stale flag must be cleared.
        let mono_in = &in_ptrs[..1];
        let mono_out = vec![ptrs(&mut output)[1]];
        {
            let _prepared = prepare_block(mono_in, &mono_out, 16, &mut scratch).unwrap();
        }
        assert!(!scratch.bounced[0]);
        finish_block(&mut scratch, 16);
    }

    #[test]
    fn rt_prepare_finish_block_does_not_allocate() {
        let mut input = [vec![0.5_f32; 64], vec![0.5; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let in_ptrs = ptrs(&mut input);
        let out_ptrs = ptrs(&mut output);
        let mut scratch = scratch_stereo(64);
        assert_no_alloc::assert_no_alloc(|| {
            {
                let mut prepared = prepare_block(&in_ptrs, &out_ptrs, 64, &mut scratch).unwrap();
                prepared.output.channel_mut(0).unwrap().fill(0.25);
            }
            finish_block(&mut scratch, 64);
        });
    }

    #[test]
    fn event_scratch_sorts_and_bounds() {
        let mut scratch = EventScratch::empty();
        scratch.configure();
        scratch.push(Event::new(
            10,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(0.5).unwrap(),
            }),
        ));
        scratch.push(Event::new(
            2,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(0.25).unwrap(),
            }),
        ));
        scratch.sort();
        let events = scratch.as_slice();
        assert_eq!(events[0].frame_offset(), 2);
        assert_eq!(events[1].frame_offset(), 10);
    }

    #[test]
    fn block_offset_bounds() {
        assert_eq!(block_offset(0, 64), Some(0));
        assert_eq!(block_offset(63, 64), Some(63));
        assert_eq!(block_offset(64, 64), None);
        assert_eq!(block_offset(-1, 64), None);
        // Zero-frame block: only offset 0 is legal (EventList flush).
        assert_eq!(block_offset(0, 0), Some(0));
        assert_eq!(block_offset(1, 0), None);
    }
}
