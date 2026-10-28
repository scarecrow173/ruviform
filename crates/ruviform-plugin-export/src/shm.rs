//! Internal shared-memory data plane for the L1 runner: named-segment
//! attach, `[header][in-slots][out-slots]` validation and fixed-layout
//! slot access.
//!
//! `shared_memory` is confined to this module (adoption condition — its
//! types never appear on the public API or feature names). The segment
//! is owned by the parent proxy; the child only attaches (`open`) by the
//! `os_id` a `CreateInstance` command payload carries (wire v2 — the v1
//! `--ruviform-shm-key` argv flag is gone) and never creates or
//! unlinks.
//!
//! ## Deviation from `docs/designs/plugin-l1-out-of-process.md`
//!
//! The design recorded `Shmem::as_slice`/`as_mut_slice` as *safe* APIs
//! and made "zero self-`unsafe`" the crate-selection condition. In
//! `shared_memory` 0.12.4 (latest, 2022-03) both accessors are `unsafe
//! fn` — named `as_slice`/`as_slice_mut` — and no safe accessor exists.
//! The only `unsafe` in this crate is therefore the two scoped call
//! sites in [`bytes`]/[`bytes_mut`], each carrying a SAFETY comment;
//! everything else is byte-level parse/serialize over the returned
//! slices. Reported to the lead for the adoption-record update.
//!
//! ## Byte layout (all little-endian)
//!
//! The layout is defined once in [`wire::shm`] — the authoritative
//! wire-side contract. Header — `HEADER_LEN` = 28 bytes:
//!
//! | offset | field | type | required value |
//! | --- | --- | --- | --- |
//! | +0 | `magic` | `u32` | [`wire::shm::MAGIC`] (`"RVL1"`) |
//! | +4 | `wire_version` | `u32` | [`wire::WIRE_VERSION`] |
//! | +8 | `in_depth` | `u32` | ≥ 1 |
//! | +12 | `out_depth` | `u32` | ≥ 1 |
//! | +16 | `slot_bytes` | `u32` | [`wire::shm::SLOT_BYTES`] |
//! | +20 | `in_off` | `u32` | `HEADER_LEN` |
//! | +24 | `out_off` | `u32` | `HEADER_LEN + in_depth * SLOT_BYTES` |
//!
//! Total segment length must cover `out_off + out_depth * SLOT_BYTES`
//! (equality on POSIX; on Windows `Shmem` reports the
//! `VirtualQuery`-rounded mapping length, so the check is `≥`).
//!
//! Slot hand-off is strict alternation driven by the doorbell `seq` —
//! no control state lives in the segment; the runner fences
//! `Acquire` after reading a `BlockIn` and `Release` before writing the
//! `BlockDone` (see `run` in `lib.rs`).

use ruviform_plugin_core::ProcessResult;
use ruviform_plugin_core::wire::{self, shm};
use shared_memory::{Shmem, ShmemConf};

/// A parsed, validated in-slot: header scalars plus the three
/// variable-length regions borrowed from the segment.
pub(crate) struct InSlot<'a> {
    /// Block length in frames (≤ `WIRE_MAX_FRAMES`).
    pub(crate) frames: u32,
    /// Channel count (≤ `WIRE_MAX_CHANNELS`).
    pub(crate) channels: u32,
    /// Number of wire-encoded events in `events` (≤ `WIRE_MAX_EVENTS`).
    pub(crate) event_count: u32,
    /// Slot flags — only [`shm::FLAG_TRANSPORT`] is assigned.
    pub(crate) flags: u32,
    /// The `transport` region ([`shm::TRANSPORT_LEN`] bytes); decode
    /// with [`wire::decode_transport`] when `flags & FLAG_TRANSPORT`.
    pub(crate) transport: &'a [u8],
    /// `event_count` consecutive wire-encoded `Event` records.
    pub(crate) events: &'a [u8],
    /// The `samples` region for `channels` channels (channel stride
    /// [`shm::SAMPLE_STRIDE`] bytes; the first `frames * 4` bytes of
    /// each stride are live).
    pub(crate) samples: &'a [u8],
}

/// An attached, header-validated shared-memory segment.
pub(crate) struct Segment {
    mem: Shmem,
    in_depth: u64,
    out_depth: u64,
    in_off: usize,
    out_off: usize,
}

impl Segment {
    /// Attaches to the named segment `key` and validates the header,
    /// including that the header's `in_depth`/`out_depth` equal the
    /// depths the host declared in the `CreateInstance` payload — a
    /// mismatch means the segment belongs to a different ring layout
    /// than the host claims.
    ///
    /// Every failure is a defined rejection reported as `Err(&'static
    /// str)` — the caller maps it to `Err(HostError::Ipc)` on the
    /// `CreateInstance` reply while the child stays alive.
    pub(crate) fn attach(key: &str, in_depth: u32, out_depth: u32) -> Result<Self, &'static str> {
        let mem = ShmemConf::new()
            .os_id(key)
            .open()
            .map_err(|_| "shared-memory segment open failed")?;
        let bytes = bytes(&mem);
        if bytes.len() < shm::HEADER_LEN {
            return Err("shared-memory segment smaller than the header");
        }
        if get_u32(bytes, shm::OFF_MAGIC) != shm::MAGIC {
            return Err("shared-memory header magic mismatch");
        }
        if get_u32(bytes, shm::OFF_WIRE_VERSION) != u32::from(wire::WIRE_VERSION) {
            return Err("shared-memory header wire-version mismatch");
        }
        let header_in = get_u32(bytes, shm::OFF_IN_DEPTH);
        let header_out = get_u32(bytes, shm::OFF_OUT_DEPTH);
        if header_in == 0 || header_out == 0 {
            return Err("shared-memory header has a zero slot depth");
        }
        if header_in != in_depth || header_out != out_depth {
            return Err("shared-memory header slot depth differs from the declared depths");
        }
        if get_u32(bytes, shm::OFF_SLOT_BYTES) as usize != shm::SLOT_BYTES {
            return Err("shared-memory header slot size mismatch");
        }
        let in_off = get_u32(bytes, shm::OFF_IN_OFF) as usize;
        if in_off != shm::IN_SLOTS_OFF {
            return Err("shared-memory header in-offset mismatch");
        }
        let out_off = get_u32(bytes, shm::OFF_OUT_OFF) as usize;
        if out_off != shm::out_slots_off(header_in) {
            return Err("shared-memory header out-offset mismatch");
        }
        // `≥`, not `==`: on Windows the mapped length is the
        // `VirtualQuery` region size rounded to the 64 KiB allocation
        // granularity; slot accesses stay inside the declared layout
        // either way, so the invariant is "the mapping covers it".
        if bytes.len() < out_off + header_out as usize * shm::SLOT_BYTES {
            return Err("shared-memory segment smaller than the declared layout");
        }
        Ok(Self {
            mem,
            in_depth: u64::from(header_in),
            out_depth: u64::from(header_out),
            in_off,
            out_off,
        })
    }

    /// Parses and validates the in-slot for doorbell `seq`.
    ///
    /// The slot's stored `seq` must echo the doorbell — a mismatch
    /// means the producer violated the slot protocol.
    pub(crate) fn in_slot(&self, seq: u64) -> Result<InSlot<'_>, &'static str> {
        let index =
            usize::try_from(seq % self.in_depth).map_err(|_| "in-slot index does not fit usize")?;
        let start = self.in_off + index * shm::SLOT_BYTES;
        let slot = &bytes(&self.mem)[start..start + shm::SLOT_BYTES];
        if get_u64(slot, shm::SLOT_OFF_SEQ) != seq {
            return Err("in-slot seq does not echo the doorbell");
        }
        let frames = get_u32(slot, shm::SLOT_OFF_FRAMES);
        if frames > wire::WIRE_MAX_FRAMES {
            return Err("in-slot frames exceed WIRE_MAX_FRAMES");
        }
        let channels = get_u32(slot, shm::SLOT_OFF_CHANNELS);
        if channels > wire::WIRE_MAX_CHANNELS {
            return Err("in-slot channels exceed WIRE_MAX_CHANNELS");
        }
        let event_count = get_u32(slot, shm::SLOT_OFF_EVENT_COUNT);
        if event_count > wire::WIRE_MAX_EVENTS {
            return Err("in-slot event count exceeds WIRE_MAX_EVENTS");
        }
        let flags = get_u32(slot, shm::SLOT_OFF_FLAGS);
        if flags & !shm::FLAG_TRANSPORT != 0 {
            return Err("in-slot flags carry unassigned bits");
        }
        let events_end = shm::EVENTS_OFF + event_count as usize * shm::EVENT_WIRE_SIZE;
        let samples_len = channels as usize * shm::SAMPLE_STRIDE;
        Ok(InSlot {
            frames,
            channels,
            event_count,
            flags,
            transport: &slot[shm::TRANSPORT_OFF..shm::TRANSPORT_OFF + shm::TRANSPORT_LEN],
            events: &slot[shm::EVENTS_OFF..events_end],
            samples: &slot[shm::SAMPLES_OFF..shm::SAMPLES_OFF + samples_len],
        })
    }

    /// Writes the out-slot for doorbell `seq`: the slot scalars
    /// (`frames`/`channels` echo the processed block, `event_count` and
    /// `flags` are zero — the runner emits no events), the wire-encoded
    /// `result` and `samples` (`channels × frames` channel-major LE
    /// `f32`).
    pub(crate) fn write_out_slot(
        &mut self,
        seq: u64,
        frames: u32,
        channels: u32,
        result: ProcessResult,
        samples: &[f32],
    ) {
        // `seq % out_depth < u32::MAX` always fits a `usize` — any
        // target `usize` is at least 32 bits on supported platforms.
        let index =
            usize::try_from(seq % self.out_depth).expect("out-slot index always fits usize");
        let start = self.out_off + index * shm::SLOT_BYTES;
        let slot = &mut bytes_mut(&mut self.mem)[start..start + shm::SLOT_BYTES];
        put_u64(slot, shm::SLOT_OFF_SEQ, seq);
        put_u32(slot, shm::SLOT_OFF_FRAMES, frames);
        put_u32(slot, shm::SLOT_OFF_CHANNELS, channels);
        put_u32(slot, shm::SLOT_OFF_EVENT_COUNT, 0);
        put_u32(slot, shm::SLOT_OFF_FLAGS, 0);
        slot[shm::TRANSPORT_OFF..shm::TRANSPORT_OFF + shm::TRANSPORT_LEN].fill(0);
        let mut encoded = Vec::new();
        wire::encode_process_result(&result, &mut encoded);
        slot[shm::RESULT_OFF..shm::RESULT_OFF + encoded.len()].copy_from_slice(&encoded);
        slot[shm::RESULT_OFF + encoded.len()..shm::RESULT_OFF + shm::RESULT_LEN].fill(0);
        let frames = frames as usize;
        for channel in 0..channels as usize {
            let base = shm::SAMPLES_OFF + channel * shm::SAMPLE_STRIDE;
            let source = &samples[channel * frames..(channel + 1) * frames];
            for (frame, &sample) in source.iter().enumerate() {
                slot[base + frame * 4..base + frame * 4 + 4].copy_from_slice(&sample.to_le_bytes());
            }
        }
    }
}

/// The segment as a read-only byte slice.
fn bytes(mem: &Shmem) -> &[u8] {
    // SAFETY: `Shmem::as_slice` is `unsafe` because the crate cannot
    // prove the mapped range immutable — another process maps the same
    // pages. The L1 slot protocol supplies that guarantee: header bytes
    // are written once by the owner before spawn and only read here,
    // and each slot is touched by exactly one side at a time under the
    // strict-alternation doorbell rule (the producer reuses in-slot
    // `seq % D` only after `done seq - D`; in this process the slice is
    // used on the single runner thread only, and the length is the
    // attach-validated mapping length).
    #[expect(unsafe_code)]
    unsafe {
        mem.as_slice()
    }
}

/// The segment as a mutable byte slice.
fn bytes_mut(mem: &mut Shmem) -> &mut [u8] {
    // SAFETY: same protocol argument as `bytes` — the out-slot we write
    // is owned by this child between `BlockIn` and `BlockDone` under the
    // strict-alternation rule, so no other process touches it meanwhile;
    // the `&mut Shmem` borrow keeps the slice exclusive within this
    // single-threaded runner.
    #[expect(unsafe_code)]
    unsafe {
        mem.as_slice_mut()
    }
}

fn get_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("u32 field is 4 bytes"),
    )
}

fn get_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("u64 field is 8 bytes"),
    )
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
