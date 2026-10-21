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
//! ## Aliasing / in-place policy
//!
//! Only *output* channels ever bounce. Per output channel `j`, if its
//! pointer equals any input channel pointer (`out[j] == in[i]`,
//! including partial cross aliases) or any earlier output channel
//! pointer (`out[j] == out[k], k < j`), the output slice is routed
//! through a per-instance bounce buffer and [`finish_block`] copies it
//! back — deterministic last-writer-wins for duplicated outputs.
//! Inputs are read-only `&[f32]`: they may freely alias each other or
//! the host memory of *bounced* outputs (never written during the
//! block). Non-bounced outputs are pairwise disjoint and disjoint from
//! every input, so no `&[f32]`/`&mut [f32]` (or `&mut`/`&mut`) pair
//! ever covers one region.
//!
//! ## Allocation contract
//!
//! [`ChannelScratch`]/[`EventScratch`] are per-plugin-instance state
//! allocated at `setupProcessing` time (non-RT) sized to the
//! negotiated channel counts and `maxSamplesPerBlock`.
//! `prepare_block`/`finish_block` never allocate — the `rt_` tests
//! exercise the path but cannot observe allocation; until the Issue
//! #22 verification lane lands the no-alloc guarantee is a documented
//! contract plus review target.

use std::mem::align_of;
use std::ptr::{self, NonNull};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
use ruviform_plugin_core::Event;

/// Maximum parameter events carried per block; host output beyond the
/// capacity is dropped (defined truncation, recorded in the `PoC`
/// findings).
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
    /// A bus's channel count differs from the negotiated arrangement.
    ChannelCount,
    /// A channel pointer inside a non-empty bus is null.
    NullChannel,
    /// A channel pointer is not 4-byte (`f32`) aligned.
    Misaligned,
}

/// Per-instance event list scratch — bounded at [`MAX_BLOCK_EVENTS`]
/// with capacity reserved at `configure` time. Sorted by frame offset
/// before [`EventList`][ruviform_plugin_core::EventList] validation —
/// the adapter-side sort the ordering contract allows for unsorted
/// host input.
pub(crate) struct EventScratch {
    events: Vec<Event>,
}

impl EventScratch {
    /// An empty scratch — allocates nothing.
    ///
    /// **RT contract:** RT-suitable — pure.
    pub(crate) const fn empty() -> Self {
        Self { events: Vec::new() }
    }

    /// Reserves event capacity — the `setupProcessing` allocation
    /// point. Non-RT only.
    pub(crate) fn configure(&mut self) {
        self.events.clear();
        self.events.reserve(MAX_BLOCK_EVENTS);
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

    /// Stable-sorts by frame offset — in place, allocation-free (the
    /// standard `sort_by_key` may allocate auxiliary scratch; the
    /// shared insertion sort is guaranteed in-place).
    ///
    /// **RT contract:** `O(n²)` bounded over at most
    /// `MAX_BLOCK_EVENTS` elements.
    pub(crate) fn sort(&mut self) {
        ruviform_plugin_core::sort_events_by_frame_offset(&mut self.events);
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
/// and the writeback bookkeeping.
///
/// The slice lists hold `*const [f32]`/`*mut [f32]` *raw* fat pointers
/// so no reference with a fake lifetime ever lives in the struct; the
/// single place where they become `&[f32]`/`&mut [f32]` is the scoped
/// reborrow inside [`prepare_block`] (see its SAFETY comments).
pub(crate) struct ChannelScratch {
    /// `*const [f32]` per input channel, rebuilt each block.
    in_slices: Vec<*const [f32]>,
    /// `*mut [f32]` per output channel, rebuilt each block — either a
    /// host channel or a bounce chunk.
    out_slices: Vec<*mut [f32]>,
    /// Host output pointer per output channel (writeback target).
    out_host: Vec<*mut f32>,
    /// Whether output channel `j` was bounced this block.
    bounced: Vec<bool>,
    /// Flat bounce region: `out_channels * max_frames` samples.
    bounce: Vec<f32>,
    in_channels: usize,
    out_channels: usize,
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
            bounce: Vec::new(),
            in_channels: 0,
            out_channels: 0,
            max_frames: 0,
        }
    }

    /// (Re)allocates the scratch for `max_frames` frames and the
    /// negotiated channel counts — the `setupProcessing` allocation
    /// point. Non-RT only.
    pub(crate) fn configure(&mut self, in_channels: usize, out_channels: usize, max_frames: usize) {
        self.in_channels = in_channels;
        self.out_channels = out_channels;
        self.max_frames = max_frames;
        // A dangling fat pointer is a safe placeholder: it is rewritten
        // every block before the list is reborrowed as slices.
        let dangling_in = ptr::slice_from_raw_parts::<f32>(NonNull::dangling().as_ptr(), 0);
        let dangling_out = ptr::slice_from_raw_parts_mut::<f32>(NonNull::dangling().as_ptr(), 0);
        self.in_slices.resize(in_channels, dangling_in);
        self.out_slices.resize(out_channels, dangling_out);
        self.out_host.resize(out_channels, ptr::null_mut());
        self.bounced.resize(out_channels, false);
        self.bounce.resize(out_channels * max_frames, 0.0);
    }

    /// The negotiated maximum block length.
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) fn max_frames(&self) -> usize {
        self.max_frames
    }

    /// Whether output channel `j` was bounced this block — test
    /// introspection for the writeback bookkeeping.
    #[cfg(test)]
    pub(crate) fn bounced(&self, index: usize) -> bool {
        self.bounced[index]
    }
}

/// Builds a block-relative frame offset from a raw host sample offset:
/// clamps into `[0, frames-1]` (or `0` for a zero-frame block) so every
/// event stays inside the block — a defined boundary mapping, recorded
/// in the `PoC` findings.
///
/// **RT contract:** RT-suitable — pure.
#[must_use]
pub(crate) fn block_offset(raw_offset: i32, frames: usize) -> u32 {
    // Negative offsets become zero (block start); offsets at or past
    // the end clamp to the last frame.
    let Ok(raw) = u32::try_from(raw_offset) else {
        return 0;
    };
    // `saturating_sub` keeps `frames == 0` at bound 0, matching
    // `EventList`'s "only offset 0 is due" zero-frame flush.
    let hi = u32::try_from(frames.saturating_sub(1)).unwrap_or(u32::MAX);
    raw.min(hi)
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
/// negotiated input/output bus (already extracted from
/// `channelBuffers32` by the FFI layer — pointer-list reads are the
/// FFI's job). `frames` is `numSamples` — zero is a defined skip.
///
/// Boundary validation, in order: `frames` non-zero and within
/// `scratch.max_frames`, channel counts equal the negotiated counts,
/// every channel pointer non-null and 4-byte aligned. Then per-channel
/// alias detection (module docs) routes colliding outputs through the
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
    if input_ptrs.len() != scratch.in_channels || output_ptrs.len() != scratch.out_channels {
        return Err(Rejection::ChannelCount);
    }
    for &ptr in input_ptrs.iter().chain(output_ptrs.iter()) {
        if ptr.is_null() {
            return Err(Rejection::NullChannel);
        }
        if (ptr as usize) % align_of::<f32>() != 0 {
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

    // Output channels: per-channel-pair alias detection.
    for (j, &ptr) in output_ptrs.iter().enumerate() {
        let aliased_input = input_ptrs.contains(&ptr);
        let aliased_output = output_ptrs[..j].contains(&ptr);
        let bounced = aliased_input || aliased_output;
        scratch.bounced[j] = bounced;
        scratch.out_host[j] = ptr;
        scratch.out_slices[j] = if bounced {
            let chunk = &mut scratch.bounce[j * scratch.max_frames..][..frames];
            ptr::slice_from_raw_parts_mut(chunk.as_mut_ptr(), frames)
        } else {
            ptr::slice_from_raw_parts_mut(ptr, frames)
        };
    }

    let in_list: &'a [&'a [f32]] = {
        let raw: &mut [*const [f32]] = &mut scratch.in_slices[..scratch.in_channels];
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
        let raw: &mut [*mut [f32]] = &mut scratch.out_slices[..scratch.out_channels];
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
    for j in 0..scratch.out_channels {
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
            prepare_block(&in_ptrs[..1], &out_ptrs, 16, &mut scratch).unwrap_err(),
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
    fn in_place_channel_bounces_and_writes_back() {
        let mut input = [vec![0.5_f32; 8], vec![0.25; 8]];
        let in_ptrs = ptrs(&mut input);
        // out[0] == in[0]: full in-place on channel 0; out[1] distinct.
        let mut extra = vec![0.0_f32; 8];
        let out_ptrs = [in_ptrs[0], extra.as_mut_ptr()];
        let mut scratch = scratch_stereo(8);

        {
            let mut prepared = prepare_block(&in_ptrs, &out_ptrs, 8, &mut scratch).unwrap();
            prepared.output.channel_mut(0).unwrap().fill(2.0);
            prepared.output.channel_mut(1).unwrap().fill(3.0);
        }
        assert!(scratch.bounced(0));
        assert!(!scratch.bounced(1));
        // Input buffer not yet overwritten during processing.
        assert_eq!(input[0], vec![0.5_f32; 8]);
        finish_block(&mut scratch, 8);
        assert_eq!(input[0], vec![2.0_f32; 8]);
        assert_eq!(extra, vec![3.0_f32; 8]);
    }

    #[test]
    fn partial_cross_alias_and_output_duplicate_are_detected() {
        let mut input = [vec![1.0_f32; 8], vec![2.0; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        let in_ptrs = ptrs(&mut input);
        let out_ptrs = ptrs(&mut output);
        let mut scratch = scratch_stereo(8);

        // Partial alias: out[1] == in[0] (cross-channel).
        let cross = [out_ptrs[0], in_ptrs[0]];
        {
            let _prepared = prepare_block(&in_ptrs, &cross, 8, &mut scratch).unwrap();
        }
        assert!(scratch.bounced(1));
        assert!(!scratch.bounced(0));
        // Duplicate output pointers: out[1] == out[0].
        let dup = [out_ptrs[0], out_ptrs[0]];
        {
            let _prepared = prepare_block(&in_ptrs, &dup, 8, &mut scratch).unwrap();
        }
        assert!(scratch.bounced(1));
        assert!(!scratch.bounced(0));
    }

    #[test]
    fn event_scratch_collects_sorts_and_truncates() {
        let mut scratch = EventScratch::empty();
        scratch.configure();
        for i in [5_u32, 1, 3] {
            assert!(scratch.push(Event::new(
                i,
                EventKind::ParamValue(ParamEvent {
                    param_id: ParamId(0),
                    value: ParamValue::new(0.5).unwrap(),
                }),
            )));
        }
        scratch.sort();
        let offsets: Vec<u32> = scratch
            .as_slice()
            .iter()
            .map(|e| e.frame_offset())
            .collect();
        assert_eq!(offsets, [1, 3, 5]);
    }

    #[test]
    fn block_offset_clamps_into_block() {
        assert_eq!(block_offset(-5, 64), 0);
        assert_eq!(block_offset(0, 64), 0);
        assert_eq!(block_offset(63, 64), 63);
        assert_eq!(block_offset(64, 64), 63);
        assert_eq!(block_offset(i32::MAX, 64), 63);
        assert_eq!(block_offset(10, 0), 0);
    }

    /// RT contract: `prepare_block` + view access + `finish_block` are
    /// bounded borrows/copies over pre-allocated scratch — no
    /// allocation on the process path (`rt_` cannot observe alloc;
    /// contract documented in the module docs).
    #[test]
    fn rt_prepare_process_finish_round_trip() {
        let mut input = [vec![0.5_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let in_ptrs = ptrs(&mut input);
        let out_ptrs = ptrs(&mut output);
        let mut scratch = scratch_stereo(64);

        {
            let mut prepared = prepare_block(&in_ptrs, &out_ptrs, 64, &mut scratch).unwrap();
            for c in 0..prepared.input.channels() {
                let src = prepared.input.channel(c).unwrap();
                let dst = prepared.output.channel_mut(c).unwrap();
                dst.copy_from_slice(src);
            }
        }
        finish_block(&mut scratch, 64);
        assert_eq!(output[0], input[0]);
        assert_eq!(output[1], input[1]);
    }
}
