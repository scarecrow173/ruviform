//! The L1 shared-memory **data plane**: seq-partitioned doorbells,
//! lock-free in/out staging and the RT try-legs of the `process`
//! equivalent (`docs/designs/plugin-l1-out-of-process.md` —
//! `RT caller ↔ SPSC ↔ driver/reader ↔ shm ↔ child`).
//!
//! ## Seq-space partition
//!
//! Command replies and block doorbells share the `u64` seq field of
//! `wire::Message::Reply`. To keep a `Reply` for a rejected `BlockIn`
//! from ever aliasing a command waiter, the host partitions the seq
//! space: command seqs come from `next_seq` (`0..`), and every
//! `BlockIn` seq carries [`BLOCK_SEQ_FLAG`] (`1 << 63`). The child
//! echoes the doorbell seq verbatim in `BlockDone` (success) or
//! `Reply{seq, Err}` (rejection — slot parse failure, lifecycle
//! rejection, plugin panic), so the reader demultiplexes on the flag
//! bit alone. `BlockDone` itself is a doorbell kind, but the flag is
//! still required on arrival — an unflagged `BlockDone` answers a
//! doorbell this host never rang and is a protocol violation.
//!
//! ## Staging (no shared control state in shm)
//!
//! The shm segment itself carries no control state — slot selection
//! is the doorbell `seq % D` under strict alternation. What the two
//! I/O threads and the RT caller exchange instead is [`Staging`]:
//! per-slot buffers made of plain atomics (samples as `f32` bit
//! patterns in `AtomicU32`, wire-byte regions as `AtomicU8`), shared
//! via `Arc` between the caller thread (writes `ins`), the driver
//! (reads `ins` into shm in-slots) and the reader (writes `outs`
//! from shm out-slots, read by the caller). Element accesses are
//! `Relaxed`: the cross-thread ordering edge is the SPSC channel —
//! `rtrb` publishes whatever was written before `try_push` to the
//! side that `try_pop`s the seq — plus the pipe doorbell fences
//! (atomic `fence` Release/Acquire) across the process boundary.
//! Nothing here locks, allocates or touches shm on the RT path.
//!
//! ## Strict alternation
//!
//! Slot `i` may be reused for block `s` only after the child is done
//! with the previous occupant `s - D`. The RT leg enforces this
//! *producer-side*: `s` is staged and pushed only when the done for
//! the slot's last pushed seq was already observed on the result
//! channel (`cleared[i] >= last_pushed[i]`). Because every seq
//! yields exactly one completion element in seq order — a real
//! `BlockDone`/rejection, or the driver's synthesized
//! [`mark_failed`] for a dropped submit — and
//! `expected = seq - lookahead` with `D = lookahead + 1`, that gate
//! also guarantees the shm in-slot is free before the driver writes
//! it, and that `staging.outs[i]` still holds block `s`'s data — the
//! consume call for `s` happens before `s + D` is even submitted, so
//! no overwrite can interleave. The out-stage `seq` tag is the
//! belt-and-suspenders check for a misbehaving child.
//!
//! ## Pass-through
//!
//! Every failure of the block path resolves to the defined RT
//! rejection — `Ok(ProcessResult::CONTINUE)` with substituted
//! output, never a hang:
//!
//! | condition | output |
//! | --- | --- |
//! | mirror `Failed` | silence |
//! | mirror not `Processing` and not `Failed` | `Err(Lifecycle)` — a caller contract violation, not pass-through |
//! | dims/events over the wire bound | `Err(Ipc)` — the block is unrepresentable on the wire |
//! | slot still busy / submit channel full | `last_output` (silence if never received) |
//! | `seq - lookahead` underflow | silence (no `expected` exists yet) |
//! | expected result absent, or marked failed | `last_output` (silence if never received) |
//! | matched result | child output + `last_output` update, child's `ProcessResult` |

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};

use ruviform_plugin_core::wire::{self, shm};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, HostError, HostInput, HostOutput, LifecycleError,
    LifecycleEvent, LifecycleState, ProcessResult, ProcessStatus, TransportInfo,
};
use ruviform_rt::{Consumer, Producer, Retirer};

use super::shm::{get_u32, get_u64, put_u32, put_u64};

/// High bit partitioning doorbell seqs from command seqs — see the
/// module docs. The child echoes it; the reader demultiplexes on it.
pub(crate) const BLOCK_SEQ_FLAG: u64 = 1 << 63;

/// Marks a result-channel element as "the block was rejected" (the
/// child answered a flagged `Reply{seq, Err}` instead of
/// `BlockDone`). It lives below [`BLOCK_SEQ_FLAG`]; the two never
/// collide because block counters use only the low 62 bits.
const RESULT_FAILED: u64 = 1 << 62;

/// Strips [`RESULT_FAILED`] from a result-channel element, keeping
/// the doorbell seq (with [`BLOCK_SEQ_FLAG`]) it was pushed for.
const SEQ_MASK: u64 = !RESULT_FAILED;

/// `true` when `seq` belongs to the doorbell space — the reader's
/// `Reply` demux and `BlockDone` legality check.
pub(crate) const fn is_block_seq(seq: u64) -> bool {
    seq & BLOCK_SEQ_FLAG != 0
}

/// A `submit`-channel element: the doorbell `seq` plus the
/// [`Staging::generation`] it was staged under. The driver drops a
/// submit whose generation is no longer the armed ring's — it can
/// only have been queued before a re-activation swapped
/// `Inner::staging`, and serializing it would doorbell the new
/// ring's zeroed stage under a real seq. A dropped seq is still
/// completed: the driver synthesizes a [`mark_failed`] result onto
/// the result channel so the strict-alternation gate frees the
/// slot and the leg resolves the block as the defined pass-through
/// (an unanswered seq would wedge the slot — then the ring —
/// permanently).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Submit {
    /// The doorbell seq (carries [`BLOCK_SEQ_FLAG`]).
    pub(crate) seq: u64,
    /// [`Staging::generation`] of the ring the seq was staged under.
    pub(crate) generation: u64,
}

/// The driver-side stale-submit gate: a queued [`Submit`] may be
/// doorbell'd only while the armed ring is still the one it was
/// staged under. `None` means "drop it" — the seq never enters
/// `block_in_flight`, no `BlockIn` is written (so no shm in-slot
/// race exists), and the driver synthesizes a [`mark_failed`]
/// completion on the result channel: the RT leg resolves it as the
/// defined pass-through and the slot's alternation gate is freed.
pub(crate) fn armed_for(staging: Option<&Arc<Staging>>, submit: Submit) -> Option<Arc<Staging>> {
    match staging {
        Some(staging) if staging.generation == submit.generation => Some(Arc::clone(staging)),
        _ => None,
    }
}

/// The result-channel element for a rejected doorbell — the reader
/// pushes `mark_failed(seq)` when the child answered `BlockIn{seq}`
/// with a flagged `Reply{seq, Err}` instead of `BlockDone`.
pub(crate) const fn mark_failed(seq: u64) -> u64 {
    seq | RESULT_FAILED
}

/// The wire `ProcessResult` status tags (same values the wire codec
/// uses) packed with `tail_frames` into one `u64` for the
/// cross-thread out-stage.
fn pack_result(result: ProcessResult) -> u64 {
    let tag = match result.status {
        ProcessStatus::Continue => 0_u64,
        ProcessStatus::Sleep => 1,
        // `#[non_exhaustive]` — anything beyond `Sleep` is the wire
        // `Error` tag (decode rejects unassigned tags under the
        // current wire revision).
        _ => 2,
    };
    (tag << 32) | u64::from(result.tail_frames)
}

fn unpack_result(packed: u64) -> ProcessResult {
    let status = match packed >> 32 {
        0 => ProcessStatus::Continue,
        1 => ProcessStatus::Sleep,
        _ => ProcessStatus::Error,
    };
    ProcessResult::new(
        status,
        u32::try_from(packed & u64::from(u32::MAX)).unwrap_or(u32::MAX),
    )
}

/// `LifecycleState` as a plain `u8` for the atomic RT-side mirror —
/// the const-cheap snapshot the design's `process` flow inspects
/// instead of locking the shared mutex.
pub(crate) fn mirror_encode(state: LifecycleState) -> u8 {
    match state {
        LifecycleState::Created => 0,
        LifecycleState::Initialized => 1,
        LifecycleState::Activated => 2,
        LifecycleState::Processing => 3,
        LifecycleState::Destroyed => 5,
        // `#[non_exhaustive]` — `Failed` and any newer state are
        // conservatively terminal on the RT leg (fail closed).
        _ => 4,
    }
}

/// The inverse of [`mirror_encode`]; out-of-range decodes `Failed`
/// (fail closed).
pub(crate) fn mirror_decode(value: u8) -> LifecycleState {
    match value {
        0 => LifecycleState::Created,
        1 => LifecycleState::Initialized,
        2 => LifecycleState::Activated,
        3 => LifecycleState::Processing,
        5 => LifecycleState::Destroyed,
        _ => LifecycleState::Failed,
    }
}

/// One in-slot's cross-thread staging: the caller thread writes, the
/// driver reads. All fields are atomics so the handoff needs no lock
/// and no `unsafe`; ordering comes from the submit channel's
/// release/acquire edge, not the per-element order here.
pub(crate) struct InStage {
    pub(crate) frames: AtomicU32,
    pub(crate) channels: AtomicU32,
    pub(crate) event_count: AtomicU32,
    pub(crate) flags: AtomicU32,
    /// The wire-encoded transport record ([`shm::TRANSPORT_LEN`]
    /// bytes, zero-padded) — staged when `flags & FLAG_TRANSPORT`.
    pub(crate) transport: Vec<AtomicU8>,
    /// Wire-encoded `Event` records, `event_count` of them at stride
    /// [`shm::EVENT_WIRE_SIZE`].
    pub(crate) events: Vec<AtomicU8>,
    /// Planar `f32` bit patterns, channel stride
    /// [`Staging::frame_capacity`].
    pub(crate) samples: Vec<AtomicU32>,
}

/// One out-slot's cross-thread staging: the reader writes on
/// `BlockDone`, the RT caller reads on a matching `expected` seq.
/// `seq` is written last with `Release` and checked before the
/// copy — a mismatch means the slot no longer holds the consumed
/// block's data (misbehaving child / overwritten staging).
pub(crate) struct OutStage {
    pub(crate) seq: AtomicU64,
    pub(crate) frames: AtomicU32,
    pub(crate) channels: AtomicU32,
    /// `pack_result` of the `BlockDone`'s `ProcessResult`.
    pub(crate) result: AtomicU64,
    /// Planar `f32` bit patterns, channel stride
    /// [`Staging::frame_capacity`].
    pub(crate) samples: Vec<AtomicU32>,
}

/// The per-activation staging ring: `depth` in/out slot stages.
/// Allocated once per successful `Activate` (non-RT) at
/// `max_frames × WIRE_MAX_CHANNELS` per slot — the channel count is
/// only known at `process` time, so the wire bound is the reserve.
pub(crate) struct Staging {
    /// Per-channel sample capacity of one slot (the activated
    /// `max_frames`).
    pub(crate) frame_capacity: usize,
    /// Which ring generation this is — the host bumps it at every
    /// successful `Activate`. [`Submit`]s carry it so the driver can
    /// drop submissions queued before a re-activation swapped the
    /// ring (see [`armed_for`]).
    pub(crate) generation: u64,
    pub(crate) ins: Vec<InStage>,
    pub(crate) outs: Vec<OutStage>,
}

impl Staging {
    /// Allocates the ring — non-RT only (runs inside the `Activate`
    /// command path). `generation` is the host-side ring counter —
    /// see [`Staging::generation`].
    pub(crate) fn new(depth: u32, max_frames: u32, generation: u64) -> Self {
        let frame_capacity = max_frames as usize;
        let samples = frame_capacity * wire::WIRE_MAX_CHANNELS as usize;
        let events = wire::WIRE_MAX_EVENTS as usize * shm::EVENT_WIRE_SIZE;
        let atomics_u32 =
            |cap: usize| -> Vec<AtomicU32> { (0..cap).map(|_| AtomicU32::new(0)).collect() };
        let atomics_u8 =
            |cap: usize| -> Vec<AtomicU8> { (0..cap).map(|_| AtomicU8::new(0)).collect() };
        let ins = (0..depth)
            .map(|_| InStage {
                frames: AtomicU32::new(0),
                channels: AtomicU32::new(0),
                event_count: AtomicU32::new(0),
                flags: AtomicU32::new(0),
                transport: atomics_u8(shm::TRANSPORT_LEN),
                events: atomics_u8(events),
                samples: atomics_u32(samples),
            })
            .collect();
        let outs = (0..depth)
            .map(|_| OutStage {
                seq: AtomicU64::new(0),
                frames: AtomicU32::new(0),
                channels: AtomicU32::new(0),
                result: AtomicU64::new(pack_result(ProcessResult::CONTINUE)),
                samples: atomics_u32(samples),
            })
            .collect();
        Self {
            frame_capacity,
            generation,
            ins,
            outs,
        }
    }
}

/// The block shape + payload the RT caller hands one in-slot stage —
/// dims are validated once in [`RtLeg::process`] and bundled so the
/// staging writer stays under the argument-count limit.
struct BlockData<'a> {
    frames: u32,
    channels: u32,
    events: EventList<'a>,
    transport: Option<TransportInfo>,
    inputs: &'a [HostInput<'a>],
}

/// Serializes one staged in-slot into the shm slot record — the
/// driver's half of the data plane (non-RT). `slot` is the
/// `wire::shm`-layout byte region for `seq`; `stage` is the
/// `seq % depth` [`InStage`]. Mirrors the child's `write_out_slot`
/// byte for byte; unused regions (in-slot `result`, the transport
/// tail) are zero-filled.
pub(crate) fn write_in_slot(slot: &mut [u8], seq: u64, stage: &InStage, frame_capacity: usize) {
    put_u64(slot, shm::SLOT_OFF_SEQ, seq);
    let frames = stage.frames.load(Ordering::Relaxed);
    let channels = stage.channels.load(Ordering::Relaxed);
    let event_count = stage.event_count.load(Ordering::Relaxed);
    let flags = stage.flags.load(Ordering::Relaxed);
    put_u32(slot, shm::SLOT_OFF_FRAMES, frames);
    put_u32(slot, shm::SLOT_OFF_CHANNELS, channels);
    put_u32(slot, shm::SLOT_OFF_EVENT_COUNT, event_count);
    put_u32(slot, shm::SLOT_OFF_FLAGS, flags);
    for (i, b) in slot[shm::TRANSPORT_OFF..shm::TRANSPORT_OFF + shm::TRANSPORT_LEN]
        .iter_mut()
        .enumerate()
    {
        *b = stage.transport[i].load(Ordering::Relaxed);
    }
    // `result` is an out-slot field — zeroed here like the child's
    // zero-fill of unused regions.
    slot[shm::RESULT_OFF..shm::RESULT_OFF + shm::RESULT_LEN].fill(0);
    for record in 0..event_count as usize {
        let base = shm::EVENTS_OFF + record * shm::EVENT_WIRE_SIZE;
        for (j, b) in slot[base..base + shm::EVENT_WIRE_SIZE]
            .iter_mut()
            .enumerate()
        {
            *b = stage.events[record * shm::EVENT_WIRE_SIZE + j].load(Ordering::Relaxed);
        }
    }
    let frames = frames as usize;
    for channel in 0..channels as usize {
        let base = shm::SAMPLES_OFF + channel * shm::SAMPLE_STRIDE;
        for frame in 0..frames {
            let bits = stage.samples[channel * frame_capacity + frame].load(Ordering::Relaxed);
            slot[base + frame * 4..base + frame * 4 + 4].copy_from_slice(&bits.to_le_bytes());
        }
    }
}

/// Deserializes one shm out-slot into the staged out-slot — the
/// reader's half (non-RT). Returns `false` — nothing is published —
/// when the slot's `seq` field does not echo the doorbell, or when
/// the child-published `frames`/`channels` exceed the activated
/// contract: `stage.samples` holds exactly `frame_capacity ×
/// WIRE_MAX_CHANNELS`, so `frames > frame_capacity` or
/// `channels > WIRE_MAX_CHANNELS` would index it out of bounds —
/// a protocol violation handled exactly like the seq mismatch (the
/// RT leg's seq-tag check fails closed → defined pass-through), not
/// a truncation case. On success the `seq` tag is stored last
/// (`Release`) so the RT side only ever reads a fully-published
/// stage.
pub(crate) fn read_out_slot(
    slot: &[u8],
    seq: u64,
    stage: &OutStage,
    frame_capacity: usize,
    result: ProcessResult,
) -> bool {
    let slot_seq = get_u64(slot, shm::SLOT_OFF_SEQ);
    if slot_seq != seq {
        return false;
    }
    // Reject dims beyond the activated contract — `frame_capacity`
    // is the activated `max_frames` (already ≤ `WIRE_MAX_FRAMES`),
    // so the bounds below keep the copy inside both the staging
    // buffer and the wire sample region. A child that reports dims
    // past the negotiated bounds is a protocol violation; withhold
    // the result rather than truncate it into a corrupted publish.
    let frames = get_u32(slot, shm::SLOT_OFF_FRAMES);
    let channels = get_u32(slot, shm::SLOT_OFF_CHANNELS);
    if frames as usize > frame_capacity || channels > wire::WIRE_MAX_CHANNELS {
        return false;
    }
    stage.frames.store(frames, Ordering::Relaxed);
    stage.channels.store(channels, Ordering::Relaxed);
    stage.result.store(pack_result(result), Ordering::Relaxed);
    for channel in 0..channels as usize {
        let base = shm::SAMPLES_OFF + channel * shm::SAMPLE_STRIDE;
        for frame in 0..frames as usize {
            let bytes: [u8; 4] = slot[base + frame * 4..base + frame * 4 + 4]
                .try_into()
                .expect("f32 sample is 4 bytes");
            stage.samples[channel * frame_capacity + frame]
                .store(u32::from_le_bytes(bytes), Ordering::Relaxed);
        }
    }
    // Publish last — the acquiring side checks this tag first.
    stage.seq.store(slot_seq, Ordering::Release);
    true
}

/// Wire-encode scratch capacity — sized so every encode `stage_in`
/// performs fits without a grow: an `Event` record encodes to
/// exactly [`shm::EVENT_WIRE_SIZE`] bytes and a `TransportInfo`
/// record is bounded by [`shm::TRANSPORT_LEN`] (the wire slot layout
/// already reserves `TRANSPORT_LEN` for it — see the `wire.rs`
/// `TRANSPORT_LEN >= 91` assertion). `stage_in` `clear()`s the
/// scratch before each encode, so `len` can never exceed
/// `capacity` and the `Vec` `push`/`extend` inside the codecs
/// provably never reallocate: the RT path is allocation-free by
/// construction, not just by observation.
const SCRATCH_CAP: usize = if shm::TRANSPORT_LEN > shm::EVENT_WIRE_SIZE {
    shm::TRANSPORT_LEN
} else {
    shm::EVENT_WIRE_SIZE
};

/// The RT leg: the caller-thread end of the data plane. Holds the
/// `submit` producer / `result` consumer endpoints, the staging
/// `Arc`, the strict-alternation bookkeeping (`last_pushed`,
/// `cleared`), the early-result stash (`held`), the `last_output`
/// pass-through history and the reusable encode scratch — every
/// buffer preallocated at `new`/`arm`, so `process` never
/// allocates, locks, touches pipes or touches shm.
pub(crate) struct RtLeg {
    submit: Producer<Submit>,
    result: Consumer<u64>,
    staging: Option<Arc<Staging>>,
    /// `D` — the shm slot count, fixed at spawn as `lookahead + 1`.
    depth: u32,
    lookahead: u32,
    /// Monotone block counter (unflagged — the wire seq is
    /// `n | BLOCK_SEQ_FLAG`). Never resets across activations, so a
    /// stale in-flight reply can never alias a re-armed seq.
    next_block: u64,
    /// Last wire seq pushed per slot (`None` until first use) — the
    /// producer-side strict-alternation gate input.
    last_pushed: Vec<Option<u64>>,
    /// Last done wire seq observed per slot (drained results).
    cleared: Vec<u64>,
    /// Wire seqs popped early (`> expected`) kept for their turn —
    /// bounded by the channel capacity.
    held: Vec<u64>,
    /// `(frames, channels)` submitted per slot — the authoritative
    /// dims of the block a done seq answers.
    submit_dims: Vec<(u32, u32)>,
    /// Pass-through history: the last real output block received,
    /// channel-major at `frame_capacity` stride.
    last_output: Vec<f32>,
    frame_capacity: usize,
    last_frames: u32,
    last_channels: u32,
    have_output: bool,
    /// Reusable wire-encode scratch (one event record or one
    /// transport record at a time); capacity reserved at `new` to
    /// [`SCRATCH_CAP`] — ≥ the largest encode the wire format can
    /// emit — and never regrown on the RT path (see the constant).
    scratch: Vec<u8>,
}

impl RtLeg {
    /// Builds the leg around the channel endpoints — non-RT only
    /// (allocates the bookkeeping buffers). `depth` is the shm slot
    /// count; the staging ring arrives via [`RtLeg::arm`] on
    /// `Activate`.
    pub(crate) fn new(
        depth: u32,
        lookahead: u32,
        submit: Producer<Submit>,
        result: Consumer<u64>,
    ) -> Self {
        let depth = depth.max(1);
        let leg = Self {
            submit,
            result,
            staging: None,
            depth,
            lookahead,
            next_block: 0,
            last_pushed: vec![None; depth as usize],
            cleared: vec![0; depth as usize],
            held: Vec::with_capacity(depth as usize),
            submit_dims: vec![(0, 0); depth as usize],
            last_output: Vec::new(),
            frame_capacity: 0,
            last_frames: 0,
            last_channels: 0,
            have_output: false,
            scratch: Vec::with_capacity(SCRATCH_CAP),
        };
        // The encode bound the whole RT path relies on: every codec
        // output stays inside the preallocated capacity, so
        // `extend`/`push` can never reallocate inside `process`.
        debug_assert!(leg.scratch.capacity() >= SCRATCH_CAP);
        leg
    }

    /// Installs a fresh staging ring on a successful `Activate` —
    /// non-RT only (the command path). Re-activation swaps the ring
    /// and resets the per-slot gate and pass-through history;
    /// `next_block` stays monotone so a stale done can never alias a
    /// re-armed seq.
    pub(crate) fn arm(&mut self, staging: Arc<Staging>) {
        self.frame_capacity = staging.frame_capacity;
        self.last_output.clear();
        self.last_output.resize(
            staging.frame_capacity * wire::WIRE_MAX_CHANNELS as usize,
            0.0,
        );
        self.staging = Some(staging);
        self.last_pushed.fill(None);
        self.cleared.fill(0);
        self.held.clear();
        self.submit_dims.fill((0, 0));
        self.last_frames = 0;
        self.last_channels = 0;
        self.have_output = false;
    }

    /// The endpoint-side ring swap — **RT-suitable** twin of
    /// [`RtLeg::arm`], called by `L1Endpoint`'s `process` drain for
    /// a newer-generation `Arc<Staging>` arriving on the arm
    /// channel. Unlike `arm` (the host-side `Activate` path), only
    /// the ring-binding state resets: `staging`/`frame_capacity`/
    /// the `last_output` contents/`last_frames`/`last_channels`/
    /// `have_output`. The displaced ring goes to `retirer` — a
    /// push onto the retire channel, so no `Staging` destructor
    /// runs on this thread.
    ///
    /// The seq-space bookkeeping (`last_pushed`/`cleared`/`held`/
    /// `submit_dims`/`next_block`/`depth`/`lookahead`) **persists**
    /// across the swap: blocks submitted under the old ring can
    /// still be in flight, so the strict-alternation invariant
    /// `cleared[i] >= last_pushed[i] ⇒ slot i free` must not be
    /// lost, and a stale done must never alias a re-armed seq
    /// (`next_block` stays monotone as before).
    ///
    /// `last_output` is cleared and resized inside the capacity
    /// `L1Instance::prepare_endpoint` reserved
    /// (`bound.max_frames × WIRE_MAX_CHANNELS` — re-arms above the
    /// bound are rejected by the control-side `Activate` gate plus
    /// the endpoint drain's `frame_capacity > bound` backstop), so
    /// the `resize` below never reallocs on the RT path.
    pub(crate) fn arm_apply(&mut self, arm: Arc<Staging>, retirer: &mut Retirer<Arc<Staging>>) {
        self.frame_capacity = arm.frame_capacity;
        self.last_output.clear();
        debug_assert!(
            self.last_output.capacity() >= arm.frame_capacity * wire::WIRE_MAX_CHANNELS as usize,
            "arm_apply resize must fit the prepare_endpoint reserve"
        );
        self.last_output
            .resize(arm.frame_capacity * wire::WIRE_MAX_CHANNELS as usize, 0.0);
        if let Some(old) = self.staging.replace(arm) {
            retirer.retire(old);
        }
        self.last_frames = 0;
        self.last_channels = 0;
        self.have_output = false;
    }

    /// The generation of the currently armed ring — `0` when
    /// unarmed (`staging_generation` starts at 1, so an unarmed
    /// leg accepts the first arm offered). The endpoint drain's
    /// `generation > armed` monotone-apply gate reads this; the
    /// private `staging` field is not visible outside this module.
    pub(crate) fn armed_generation(&self) -> u64 {
        self.staging
            .as_ref()
            .map_or(0, |staging| staging.generation)
    }

    /// Per-channel sample capacity of the currently armed ring —
    /// `0` when unarmed. `EndpointInner::L1::process` runs the
    /// per-block dims pre-gate (`frames ≤ armed capacity`) against
    /// this so `block_data`'s `Err(Ipc)` for the same condition
    /// never surfaces on the endpoint's public face.
    pub(crate) fn armed_frame_capacity(&self) -> usize {
        self.staging
            .as_ref()
            .map_or(0, |staging| staging.frame_capacity)
    }

    /// Ensures `last_output` capacity covers `sample_count` samples
    /// — called once by `L1Instance::prepare_endpoint` (non-RT)
    /// with `bound.max_frames × WIRE_MAX_CHANNELS`, after which
    /// [`RtLeg::arm_apply`]'s `resize` always fits inside capacity
    /// and never reallocs on the RT path.
    pub(crate) fn reserve_last_output(&mut self, sample_count: usize) {
        self.last_output
            .reserve(sample_count.saturating_sub(self.last_output.len()));
    }

    /// The `process` equivalent — RT-suitable: bounded atomic
    /// staging writes, `try_push`/`try_pop` only, no locks, no shm,
    /// no allocation. `mirror` is the const-cheap lifecycle snapshot
    /// (the shared atomic), keeping this method unit-testable
    /// without a child.
    ///
    /// Decision table: see the module docs — every non-success path
    /// is either the defined `Err` for a caller contract violation
    /// (`Lifecycle` outside `Processing`, `Ipc` for an
    /// unrepresentable block) or `Ok(CONTINUE)` with substituted
    /// output.
    pub(crate) fn process(
        &mut self,
        mirror: LifecycleState,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> Result<ProcessResult, HostError> {
        // (1) The const-cheap lifecycle gate — `Failed` is the
        // defined silence path; anything else outside `Processing`
        // is the same `Lifecycle` error L0 returns.
        if mirror.is_failed() {
            Self::silence(outputs);
            return Ok(ProcessResult::CONTINUE);
        }
        if !mirror.accepts_processing() {
            return Err(HostError::Lifecycle(LifecycleError::IllegalTransition {
                from: mirror,
                event: LifecycleEvent::Process,
            }));
        }
        let Some(staging) = self.staging.clone() else {
            // `Processing` implies a successful `Activate`, hence an
            // armed staging — unreachable in-protocol.
            return Err(HostError::Ipc);
        };
        let block_data = Self::block_data(inputs, outputs, events, transport, &staging)?;

        // (2) Stage the input into `ins[seq % D]` and submit the
        // doorbell — only while the slot's previous occupant is done
        // (strict alternation, producer-side).
        let block = self.next_block;
        self.next_block += 1;
        // Block counters live below `RESULT_FAILED` — the result-
        // channel packing (`mark_failed`) and the seq partition rely
        // on it.
        debug_assert!(block < RESULT_FAILED);
        let seq = block | BLOCK_SEQ_FLAG;
        let slot =
            usize::try_from(seq % u64::from(self.depth)).expect("slot index always fits usize");
        if self.last_pushed[slot].is_none_or(|p| self.cleared[slot] >= p) {
            self.stage_in(&staging, slot, &block_data);
            if self
                .submit
                .try_push(Submit {
                    seq,
                    generation: staging.generation,
                })
                .is_ok()
            {
                self.last_pushed[slot] = Some(seq);
                self.submit_dims[slot] = (block_data.frames, block_data.channels);
            }
            // Channel full → submission abandoned → pass-through
            // (the seq is consumed but never doorbell'd).
        }
        // Slot still busy → this block is dropped → pass-through.

        // (3) The block this call may output is `block - lookahead`
        // — underflow means none exists yet (defined silence), but
        // the drain still runs so `cleared` stays fresh.
        let Some(expected) = block.checked_sub(u64::from(self.lookahead)) else {
            self.drain_to(None);
            self.pass_through(outputs);
            return Ok(ProcessResult::CONTINUE);
        };
        let expected = expected | BLOCK_SEQ_FLAG;

        // (4) Drain the result channel up to `expected`: stale (`<`)
        // seqs are discarded, future (`>`) seqs are stashed, and the
        // drain is bounded by the channel capacity.
        let resolved = self.drain_to(Some(expected));

        // (5) A matched, non-failed done publishes `outs[slot]` —
        // copy it to the outputs and refresh `last_output`.
        if let Some(el) = resolved
            && el & RESULT_FAILED == 0
        {
            let out_slot = usize::try_from(expected % u64::from(self.depth))
                .expect("slot index always fits usize");
            let stage = &staging.outs[out_slot];
            if stage.seq.load(Ordering::Acquire) == expected {
                let (frames, channels) = self.submit_dims[out_slot];
                self.copy_out(&staging, out_slot, frames, channels, outputs);
                return Ok(unpack_result(stage.result.load(Ordering::Relaxed)));
            }
        }
        self.pass_through(outputs);
        Ok(ProcessResult::CONTINUE)
    }

    /// Computes and bounds-checks the block shape — the wire
    /// capacity rejections (`frames`/`channels`/`event_count`) are
    /// `Err(Ipc)`: the block is unrepresentable on the transport.
    fn block_data<'a>(
        inputs: &'a [HostInput<'a>],
        outputs: &[HostOutput<'_>],
        events: EventList<'a>,
        transport: Option<TransportInfo>,
        staging: &Staging,
    ) -> Result<BlockData<'a>, HostError> {
        let in_bus = inputs.first();
        let out_bus = outputs.first();
        let in_channels = in_bus.map_or(0, BufferView::channels);
        let out_channels = out_bus.map_or(0, BufferView::channels);
        let channels = match (in_bus.is_some(), out_bus.is_some()) {
            (true, true) => in_channels.min(out_channels),
            (true, false) => in_channels,
            (false, true) => out_channels,
            (false, false) => 0,
        };
        let frames =
            in_bus.map_or_else(|| out_bus.map_or(0, BufferView::frames), BufferView::frames);
        if channels > wire::WIRE_MAX_CHANNELS as usize
            || frames > staging.frame_capacity
            || events.len() > wire::WIRE_MAX_EVENTS as usize
        {
            return Err(HostError::Ipc);
        }
        Ok(BlockData {
            frames: u32::try_from(frames).unwrap_or(u32::MAX),
            channels: u32::try_from(channels).unwrap_or(u32::MAX),
            events,
            transport,
            inputs,
        })
    }

    /// Stages one block's scalars, transport record, event records
    /// and samples into `staging.ins[slot]` — all `Relaxed`; the
    /// submit channel's release edge publishes them.
    ///
    /// Every plane `stage.channels` claims must carry defined
    /// samples: a channel without a source (an outputs-only block, or
    /// a bus exposing fewer channel slices than `block.channels`)
    /// zero-fills its range — never the previous occupant's stale
    /// data, which would otherwise be serialized to the child by
    /// [`write_in_slot`].
    ///
    /// The transport-encode failure is a defined *drop*, not an
    /// `Err`: `FLAG_TRANSPORT` simply stays clear, so the child sees
    /// a block without transport data. An encode failure means the
    /// `TransportInfo` variant is unrepresentable under this wire
    /// revision — a plugin-supplied side-channel record, not a block
    /// the wire cannot express — so `process` reports the block
    /// itself rather than failing the call.
    fn stage_in(&mut self, staging: &Staging, slot: usize, block: &BlockData<'_>) {
        let stage = &staging.ins[slot];
        stage.frames.store(block.frames, Ordering::Relaxed);
        stage.channels.store(block.channels, Ordering::Relaxed);
        for (i, event) in block.events.iter().enumerate() {
            self.scratch.clear();
            wire::encode_event(&event, &mut self.scratch);
            debug_assert_eq!(self.scratch.len(), shm::EVENT_WIRE_SIZE);
            let base = i * shm::EVENT_WIRE_SIZE;
            for (j, &b) in self.scratch.iter().take(shm::EVENT_WIRE_SIZE).enumerate() {
                stage.events[base + j].store(b, Ordering::Relaxed);
            }
        }
        stage.event_count.store(
            u32::try_from(block.events.len()).unwrap_or(u32::MAX),
            Ordering::Relaxed,
        );
        let mut flags = 0_u32;
        if let Some(info) = block.transport {
            self.scratch.clear();
            if wire::encode_transport(&info, &mut self.scratch).is_ok()
                && self.scratch.len() <= shm::TRANSPORT_LEN
            {
                flags = shm::FLAG_TRANSPORT;
                self.scratch.resize(shm::TRANSPORT_LEN, 0);
                for (i, &b) in self.scratch.iter().enumerate() {
                    stage.transport[i].store(b, Ordering::Relaxed);
                }
            }
        }
        stage.flags.store(flags, Ordering::Relaxed);
        // Defined samples for every claimed plane — a channel with
        // no source (no input bus, or a `channel` hole) zero-fills
        // its range instead of leaking the previous occupant's data.
        let first = block.inputs.first();
        for channel in 0..block.channels {
            let base = channel as usize * staging.frame_capacity;
            let src = first.and_then(|input| input.channel(channel as usize));
            for f in 0..block.frames as usize {
                let bits = src.and_then(|src| src.get(f)).map_or(0, |s| s.to_bits());
                stage.samples[base + f].store(bits, Ordering::Relaxed);
            }
        }
    }

    /// Pops the result channel until `expected` (a wire seq) is
    /// found, the channel empties, or the bounded drain count is
    /// exhausted. Every observed seq updates `cleared` (the
    /// alternation gate); seqs beyond `expected` are stashed for
    /// their turn; stale seqs are discarded. `None` drains without
    /// matching — the underflow call, where every drained seq is by
    /// definition still future and goes to `held`.
    fn drain_to(&mut self, expected: Option<u64>) -> Option<u64> {
        if let Some(expected) = expected
            && let Some(pos) = self.held.iter().position(|&el| el & SEQ_MASK == expected)
        {
            return Some(self.held.remove(pos));
        }
        for _ in 0..self.result.capacity() {
            match self.result.try_pop() {
                Err(_) => break,
                Ok(el) => {
                    let seq = el & SEQ_MASK;
                    let slot = usize::try_from(seq % u64::from(self.depth))
                        .expect("slot index always fits usize");
                    self.cleared[slot] = seq;
                    if Some(seq) == expected {
                        return Some(el);
                    }
                    let stale = expected.is_some_and(|expected| seq < expected);
                    if !stale && self.held.len() < self.held.capacity() {
                        self.held.push(el);
                    }
                }
            }
        }
        None
    }

    /// Copies the staged output block (at its submitted
    /// `(frames, channels)`) into `outputs` and records it as
    /// `last_output` — the pass-through source for later underruns.
    /// Regions the block does not cover are zeroed.
    fn copy_out(
        &mut self,
        staging: &Staging,
        slot: usize,
        frames: u32,
        channels: u32,
        outputs: &mut [HostOutput<'_>],
    ) {
        let stage = &staging.outs[slot];
        // Clamp to what the reader actually published — the staged
        // scalars are child-derived, so they bound the copy.
        let staged_frames = stage.frames.load(Ordering::Relaxed).min(frames);
        let staged_channels = stage.channels.load(Ordering::Relaxed).min(channels);
        if let Some(output) = outputs.first_mut() {
            for channel in 0..output.channels() {
                if let Some(dst) = output.channel_mut(channel) {
                    let c = u32::try_from(channel).unwrap_or(u32::MAX);
                    if c < staged_channels {
                        let base = c as usize * staging.frame_capacity;
                        let n = staged_frames as usize;
                        for (f, d) in dst.iter_mut().enumerate() {
                            *d = if f < n {
                                f32::from_bits(stage.samples[base + f].load(Ordering::Relaxed))
                            } else {
                                0.0
                            };
                        }
                    } else {
                        dst.fill(0.0);
                    }
                }
            }
        }
        // Record as pass-through history (the staged dims).
        self.last_frames = staged_frames;
        self.last_channels = staged_channels;
        self.have_output = true;
        for channel in 0..staged_channels as usize {
            let base = channel * self.frame_capacity;
            for f in 0..staged_frames as usize {
                self.last_output[base + f] =
                    f32::from_bits(stage.samples[base + f].load(Ordering::Relaxed));
            }
        }
    }

    /// `last_output` (or silence when nothing was ever received)
    /// into `outputs` — the pass-through fallback for underruns,
    /// dropped submissions, and missing or failed results.
    /// `pub(crate)` so `L1Endpoint::process` can converge a block
    /// the arm drain flagged unrepresentable onto the same defined
    /// output.
    pub(crate) fn pass_through(&mut self, outputs: &mut [HostOutput<'_>]) {
        for output in outputs.iter_mut() {
            for channel in 0..output.channels() {
                if let Some(dst) = output.channel_mut(channel) {
                    let c = u32::try_from(channel).unwrap_or(u32::MAX);
                    if self.have_output && c < self.last_channels {
                        let base = c as usize * self.frame_capacity;
                        for (f, d) in dst.iter_mut().enumerate() {
                            *d = if f < self.last_frames as usize {
                                self.last_output[base + f]
                            } else {
                                0.0
                            };
                        }
                    } else {
                        dst.fill(0.0);
                    }
                }
            }
        }
    }

    /// Zeroes every reachable output channel — the `Failed` path.
    fn silence(outputs: &mut [HostOutput<'_>]) {
        for output in outputs.iter_mut() {
            for channel in 0..output.channels() {
                if let Some(dst) = output.channel_mut(channel) {
                    dst.fill(0.0);
                }
            }
        }
    }
}

/// The pieces `L1Instance::prepare_endpoint` hands to
/// `crate::endpoint` for assembling the detached L1 endpoint —
/// everything the endpoint owns and nothing it must not. This is
/// the crate-free boundary type for the leg extraction: it lives
/// in `l1::data` (which resolves inside the `#[path]`-included
/// `tests/l1_proxy_e2e.rs` module tree) and names only
/// `l1`/external types — the `crate::endpoint` types it feeds
/// (`L1Endpoint`/`EndpointBounds`) are never named here.
///
/// - `leg`: the checked-out caller-thread data-plane half —
///   submit producer, result consumer, armed `Arc<Staging>`,
///   seq bookkeeping, `last_output` pass-through buffer.
/// - `mirror`: a clone of `Shared::rt_state` — the same atomic,
///   so `set_mirror` publishes reach the endpoint with the same
///   monotonic behavior the host leg observed.
/// - `arm_rx`: consumer half of the per-attach arm channel
///   (capacity 2). `L1Instance::execute`/`prepare_endpoint`/
///   teardown push `Arc<Staging>` activations through the producer
///   the instance keeps; `L1Endpoint::process` drains it.
/// - `arm_retirer`: the endpoint's `Retirer` — every drained arm
///   (applied or stale) and every displaced ring is pushed here so
///   `Arc<Staging>` destructors never run on the RT thread; the
///   matching `DropCollector` lives on `L1Instance` and drains on
///   non-RT `&mut` entry points.
/// - `return_tx`: producer half of the leg-return channel
///   (capacity 1). `L1Endpoint::drop` `try_push`es the leg back;
///   on `Full` (already occupied — reachable only through a
///   double-drop anomaly) it recovers the leg via
///   `TryPushError::into_inner` and drops it on the dropping
///   thread instead.
pub(crate) struct EndpointLegParts {
    pub(crate) leg: RtLeg,
    pub(crate) mirror: Arc<AtomicU8>,
    pub(crate) arm_rx: Consumer<Arc<Staging>>,
    pub(crate) arm_retirer: Retirer<Arc<Staging>>,
    pub(crate) return_tx: Producer<RtLeg>,
}

#[cfg(test)]
mod tests {
    //! `RtLeg::arm_apply` contract tests — the endpoint-side ring
    //! swap (Issue #113 task 3): ring-binding reset vs seq-space
    //! bookkeeping persistence, retirement of the displaced ring
    //! and the no-allocation RT path.

    use super::*;

    /// A leg + armed gen-1 ring — bounded channels only; the
    /// channel peer ends are dropped (try-ops stay bounded).
    fn armed_leg(depth: u32, frames: u32) -> (RtLeg, Arc<Staging>) {
        let (submit_tx, _submit_rx) = ruviform_rt::channel::<Submit>(depth as usize);
        let (_result_tx, result_rx) = ruviform_rt::channel::<u64>(depth as usize);
        let mut leg = RtLeg::new(depth, depth - 1, submit_tx, result_rx);
        let staging = Arc::new(Staging::new(depth, frames, 1));
        leg.arm(Arc::clone(&staging));
        (leg, staging)
    }

    /// `arm_apply` replaces only the ring binding — `staging`,
    /// `frame_capacity`, `last_output` contents, `last_frames`,
    /// `last_channels`, `have_output` — while every seq-space
    /// counter (`next_block`/`last_pushed`/`cleared`/`held`/
    /// `submit_dims`) survives the swap so in-flight blocks of the
    /// old ring still resolve.
    #[test]
    fn arm_apply_swaps_ring_and_keeps_seq_bookkeeping() {
        let (mut leg, old_staging) = armed_leg(3, 512);
        let (mut retirer, mut collector) = ruviform_rt::retire_channel::<Arc<Staging>>(4);
        // Seq/bookkeeping state a swap must preserve.
        leg.next_block = 7;
        leg.last_pushed[1] = Some(9);
        leg.cleared[0] = 3;
        leg.held.push(5);
        leg.submit_dims[2] = (8, 2);
        // Ring-binding state a swap must reset.
        leg.have_output = true;
        leg.last_frames = 100;
        leg.last_channels = 2;

        let new_staging = Arc::new(Staging::new(3, 256, 2));
        leg.arm_apply(Arc::clone(&new_staging), &mut retirer);

        // The new ring is bound.
        assert_eq!(leg.armed_generation(), 2);
        assert_eq!(leg.armed_frame_capacity(), 256);
        assert_eq!(leg.frame_capacity, 256);
        assert_eq!(
            leg.last_output.len(),
            256 * wire::WIRE_MAX_CHANNELS as usize
        );
        // Output-history state resets with the ring.
        assert!(!leg.have_output);
        assert_eq!(leg.last_frames, 0);
        assert_eq!(leg.last_channels, 0);
        // Seq-space bookkeeping persists.
        assert_eq!(leg.next_block, 7);
        assert_eq!(leg.last_pushed[1], Some(9));
        assert_eq!(leg.cleared[0], 3);
        assert_eq!(leg.held, vec![5]);
        assert_eq!(leg.submit_dims[2], (8, 2));
        // The displaced ring went to the retirer — the collector's
        // drain is the only place it can drop.
        assert_eq!(collector.drain(), 1);
        assert_eq!(Arc::strong_count(&old_staging), 1);
        assert_eq!(Arc::strong_count(&new_staging), 2);
    }

    /// Issue #22 lane: `arm_apply` — `last_output` clear + resize
    /// inside the `prepare_endpoint` reserve, `staging` swap and
    /// the retire `try_push` — must not allocate. Fixture, the new
    /// ring and the reserve all sit outside the window.
    #[test]
    fn rt_l1_arm_apply_no_alloc() {
        let (mut leg, _old_staging) = armed_leg(3, 512);
        let (mut retirer, mut collector) = ruviform_rt::retire_channel::<Arc<Staging>>(4);
        let new_staging = Arc::new(Staging::new(3, 256, 2));
        // The `prepare_endpoint` reserve makes the resize free.
        leg.reserve_last_output(512 * wire::WIRE_MAX_CHANNELS as usize);
        // Warm-up outside the window.
        leg.arm_apply(Arc::clone(&new_staging), &mut retirer);
        let newer = Arc::new(Staging::new(3, 512, 3));
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..8 {
                leg.arm_apply(Arc::clone(&newer), &mut retirer);
            }
        });
        // Retirement ran inside the window — the bounded channel
        // drains at most its capacity per pass (overflow is the
        // design's leak-on-full signal, not an allocation).
        assert!(collector.drain() > 0);
        assert_eq!(leg.armed_generation(), 3);
    }
}
