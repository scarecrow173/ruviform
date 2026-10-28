//! Internal shared-memory segment ownership for the L1 proxy: unique
//! key generation, named-segment creation and the fixed header the
//! child validates on attach.
//!
//! `shared_memory` is confined to this module (adoption condition —
//! its types never appear on the public API or feature names). The
//! segment is created and owned by the proxy — one per instance, on
//! a shared child each registry entry gets its own; the child only
//! attaches (`open`) by the `os_id` carried in `CreateInstance`'s
//! `shm_key` field and never creates or unlinks. Dropping the [`Segment`] unlinks the named object (POSIX
//! `shm_unlink` via the owner `Drop`; on Windows the mapping is
//! unmapped and its persistence file is delete-on-close renamed).
//!
//! ## Deviation from `docs/designs/plugin-l1-out-of-process.md`
//!
//! The design recorded `Shmem::as_slice`/`as_mut_slice` as *safe* APIs
//! and made "zero self-`unsafe`" the crate-selection condition. In
//! `shared_memory` 0.12.4 both accessors are `unsafe fn` — named
//! `as_slice`/`as_slice_mut` — and no safe accessor exists. The only
//! `unsafe` in this crate is therefore the two scoped call sites in
//! [`bytes`]/[`bytes_mut`], each carrying a SAFETY comment;
//! everything else is byte-level serialize over the returned slice.
//! This matches the runner-side deviation recorded in
//! `ruviform-plugin-export/src/shm.rs` — reported to the lead for the
//! adoption-record update.
//!
//! ## Byte layout (all little-endian)
//!
//! The layout is defined once in [`wire::shm`] — the authoritative
//! wire-side contract consumed by both this module and the
//! runner-side `shm` module in `ruviform-plugin-export`. Header —
//! [`wire::shm::HEADER_LEN`] = 28 bytes:
//!
//! | offset | field | type | value |
//! | --- | --- | --- | --- |
//! | +0 | `magic` | `u32` | [`wire::shm::MAGIC`] (`"RVL1"`) |
//! | +4 | `wire_version` | `u32` | [`wire::WIRE_VERSION`] |
//! | +8 | `in_depth` | `u32` | in-slot count (`lookahead + 1`) |
//! | +12 | `out_depth` | `u32` | out-slot count (`lookahead + 1`) |
//! | +16 | `slot_bytes` | `u32` | [`wire::shm::SLOT_BYTES`] |
//! | +20 | `in_off` | `u32` | [`wire::shm::HEADER_LEN`] |
//! | +24 | `out_off` | `u32` | `HEADER_LEN + in_depth * SLOT_BYTES` |
//!
//! The slot body layout (`seq`/scalars/transport/result/events/planar
//! `f32` samples — the `wire::shm` slot offsets) is accessed through
//! [`View`]: the `driver` thread's `in_slot_mut` (sole in-slot
//! writer) and the `reader` thread's `out_slot` (sole out-slot
//! reader), one side per slot under strict alternation.

use std::process;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ruviform_plugin_core::wire::{self, shm};
use shared_memory::{Shmem, ShmemConf, ShmemError};

/// Process-wide counter backing [`unique_key`]'s counter component —
/// every key in this process is distinct even when a `slot` repeats.
static SEGMENT_COUNTER: AtomicU64 = AtomicU64::new(0);

/// How many distinct keys [`Segment::create`] tries before giving up.
/// A collision means a stale segment left by a crashed host whose PID
/// the OS reused (the counter restarts at 0 every process, so the
/// design's `pid + slot + counter` key alone can collide across PID
/// reuse; the salt below narrows that to a fluke, the retry absorbs
/// the fluke).
const MAX_KEY_ATTEMPTS: u32 = 8;

/// Per-process salt mixed into every key: `SystemTime` nanos captured
/// once. Two hosts sharing a PID across a crash/restart boundary
/// generate disjoint key spaces unless they also booted their key
/// generator at the same nanosecond — at which point
/// [`Segment::create`]'s `MappingIdExists` retry is the backstop.
fn process_salt() -> u128 {
    static SALT: OnceLock<u128> = OnceLock::new();
    *SALT.get_or_init(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    })
}

/// A fresh named-segment key:
/// `/rvl1-shm-<pid>-<slot>-<salt>-<counter>` — the design's
/// `pid + slot + counter` form plus a per-process salt (the counter
/// restarts at 0 every process, so `pid + slot + counter` alone can
/// regenerate a key abandoned by a crashed host after PID reuse).
///
/// The leading `/` is the POSIX `shm_open` name form (`shared_memory`
/// passes `os_id` through verbatim); Windows accepts any name without
/// a backslash. `slot` is the caller's instance/slot discriminator,
/// `salt` keeps this process's keys disjoint from a dead
/// predecessor's, `counter` makes every key unique within this
/// process. Collisions remain *possible* — a named segment is a
/// global resource — so [`Segment::create`] retries on
/// `MappingIdExists` rather than promising uniqueness.
pub(crate) fn unique_key(slot: u32) -> String {
    let counter = SEGMENT_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "/rvl1-shm-{}-{slot}-{:x}-{counter}",
        process::id(),
        process_salt()
    )
}

/// A created, header-initialized shared-memory segment owned by this
/// proxy. `Drop` unlinks the named object — the child's attach only
/// borrows the `os_id`.
pub(crate) struct Segment {
    mem: Shmem,
    /// Total mapped bytes the child should expect to cover
    /// ([`shm::segment_len`]); on Windows the reported mapping length
    /// may be rounded up to the allocation granularity, so this
    /// records the *declared* length.
    declared_len: usize,
}

/// Why one create attempt failed — only `Exists` is retryable
/// (another key may succeed; a real mapping error repeats).
#[derive(Debug)]
pub(crate) enum CreateFail {
    /// `os_id` already taken — a stale segment from a crashed host.
    Exists,
    /// Any other create/header failure.
    Other,
}

impl Segment {
    /// Creates a uniquely-keyed segment for `slot` with
    /// `in_depth`/`out_depth` slots and writes the fixed header the
    /// child validates — retrying [`MAX_KEY_ATTEMPTS`] distinct keys
    /// when a stale segment name collides (`ShmemError::
    /// MappingIdExists`). Returns `Err(())` on any mapping failure —
    /// the caller maps it to [`ruviform_plugin_core::HostError::Ipc`].
    pub(crate) fn create(slot: u32, in_depth: u32, out_depth: u32) -> Result<Self, ()> {
        Self::create_keyed(slot, in_depth, out_depth, &unique_key)
    }

    /// The key-generation seam behind [`Segment::create`] — tests
    /// inject a generator to drive `MappingIdExists` collisions
    /// deterministically.
    pub(crate) fn create_keyed(
        slot: u32,
        in_depth: u32,
        out_depth: u32,
        key: &dyn Fn(u32) -> String,
    ) -> Result<Self, ()> {
        for _ in 0..MAX_KEY_ATTEMPTS {
            match Self::create_named(&key(slot), in_depth, out_depth) {
                Ok(segment) => return Ok(segment),
                Err(CreateFail::Other) => return Err(()),
                Err(CreateFail::Exists) => {}
            }
        }
        Err(())
    }

    /// One create attempt at an explicit `os_id` — the header write
    /// included. `Err(CreateFail::Exists)` is the retryable
    /// stale-name collision; everything else is `Other`.
    pub(crate) fn create_named(
        key: &str,
        in_depth: u32,
        out_depth: u32,
    ) -> Result<Self, CreateFail> {
        let declared_len = shm::segment_len(in_depth, out_depth);
        let mut mem = ShmemConf::new()
            .os_id(key)
            .size(declared_len)
            .create()
            .map_err(|error| match error {
                ShmemError::MappingIdExists => CreateFail::Exists,
                _ => CreateFail::Other,
            })?;
        write_header(&mut mem, in_depth, out_depth, declared_len)
            .map_err(|()| CreateFail::Other)?;
        Ok(Self { mem, declared_len })
    }

    /// The `os_id` the child attaches with — the value carried by
    /// `CreateInstance`'s `shm_key` field.
    pub(crate) fn key(&self) -> &str {
        self.mem.get_os_id()
    }

    /// The declared segment length (header + all slots). On Windows
    /// the mapping's reported length can exceed this; the invariant is
    /// "the mapping covers it".
    // Test-support surface — reachable through the `#[path]` include
    // in `tests/l1_proxy_e2e.rs`, not through in-crate code.
    #[allow(dead_code)]
    pub(crate) fn declared_len(&self) -> usize {
        self.declared_len
    }

    /// Re-reads the header and checks every field against the values
    /// this segment was created with — the self-verification half of
    /// the header write/verify pair (used by tests and by the data
    /// plane's `View` setup).
    // Test-support surface — reachable through the `#[path]` include
    // in `tests/l1_proxy_e2e.rs`, not through in-crate code.
    #[allow(dead_code)]
    pub(crate) fn verify_header(&self, in_depth: u32, out_depth: u32) -> bool {
        let bytes = bytes(&self.mem);
        if bytes.len() < self.declared_len {
            return false;
        }
        get_u32(bytes, shm::OFF_MAGIC) == shm::MAGIC
            && get_u32(bytes, shm::OFF_WIRE_VERSION) == u32::from(wire::WIRE_VERSION)
            && get_u32(bytes, shm::OFF_IN_DEPTH) == in_depth
            && get_u32(bytes, shm::OFF_OUT_DEPTH) == out_depth
            && get_u32(bytes, shm::OFF_SLOT_BYTES) as usize == shm::SLOT_BYTES
            && get_u32(bytes, shm::OFF_IN_OFF) as usize == shm::IN_SLOTS_OFF
            && get_u32(bytes, shm::OFF_OUT_OFF) as usize == shm::out_slots_off(in_depth)
    }
}

/// Writes the fixed segment header (see module docs for the field
/// table). `declared_len` is only used for the bounds check.
fn write_header(
    mem: &mut Shmem,
    in_depth: u32,
    out_depth: u32,
    declared_len: usize,
) -> Result<(), ()> {
    let bytes = bytes_mut(mem);
    if bytes.len() < declared_len {
        return Err(());
    }
    put_u32(bytes, shm::OFF_MAGIC, shm::MAGIC);
    put_u32(bytes, shm::OFF_WIRE_VERSION, u32::from(wire::WIRE_VERSION));
    put_u32(bytes, shm::OFF_IN_DEPTH, in_depth);
    put_u32(bytes, shm::OFF_OUT_DEPTH, out_depth);
    put_u32(
        bytes,
        shm::OFF_SLOT_BYTES,
        u32::try_from(shm::SLOT_BYTES).map_err(|_| ())?,
    );
    put_u32(
        bytes,
        shm::OFF_IN_OFF,
        u32::try_from(shm::IN_SLOTS_OFF).map_err(|_| ())?,
    );
    put_u32(
        bytes,
        shm::OFF_OUT_OFF,
        u32::try_from(shm::out_slots_off(in_depth)).map_err(|_| ())?,
    );
    Ok(())
}

/// A non-owning local attach to the proxy's own segment — the
/// data-plane view the `driver`/`reader` threads open for
/// themselves, one per registered instance (the driver's
/// `views` map on `DriverCmd::AddInstance`, the reader's lazily on
/// the first `BlockDone`/`Reply` attributed to the instance).
///
/// `Shmem` is `!Send` (its `MapData` holds a raw mapping pointer and
/// the crate declares no `Send`/`Sync` impls), so the owning
/// [`Segment`] cannot be shared with — or moved to — the I/O
/// threads. Instead each thread `open`s the same `os_id` locally:
/// `open` never sets `is_owner`, so dropping a `View` unmaps the
/// thread's own mapping and never unlinks the segment (the owner
/// `Segment` in the instance's `InstShared` entry keeps that duty).
///
/// Which thread maps which region is fixed by the thread table:
/// the `driver` attaches [`View::in_slot_mut`] access only (it is
/// the sole writer of the in-slot array), the `reader` attaches
/// [`View::out_slot`] access only (sole reader of the out-slot
/// array). The header fields ([`Segment::declared_len`], the slot
/// depths) are known to both through [`wire::shm`]'s consts and the
/// spawn-time depth — a `View` does not re-validate the header.
pub(crate) struct View {
    mem: Shmem,
}

impl View {
    /// Attaches to the named segment `key` on the calling thread —
    /// `Err(())` when the mapping fails. A segment the child attached
    /// cannot be absent for the owner, so either caller treats it as
    /// a host-side environmental failure and converges the whole
    /// child (`converge_child`).
    pub(crate) fn attach(key: &str) -> Result<Self, ()> {
        ShmemConf::new()
            .os_id(key)
            .open()
            .map(|mem| Self { mem })
            .map_err(|_| ())
    }

    /// The in-slot byte slice for doorbell `seq` (`seq % in_depth`
    /// selects the slot, per the wire contract). `in_depth` is the
    /// spawn-time depth.
    pub(crate) fn in_slot_mut(&mut self, in_depth: u32, seq: u64) -> &mut [u8] {
        let index =
            usize::try_from(seq % u64::from(in_depth)).expect("in-slot index always fits usize");
        let start = shm::IN_SLOTS_OFF + index * shm::SLOT_BYTES;
        &mut bytes_mut(&mut self.mem)[start..start + shm::SLOT_BYTES]
    }

    /// The out-slot byte slice for doorbell `seq` — the slot the
    /// child filled (`seq % out_depth`; out slots start at
    /// `out_slots_off(in_depth)`).
    pub(crate) fn out_slot(&self, in_depth: u32, out_depth: u32, seq: u64) -> &[u8] {
        let index =
            usize::try_from(seq % u64::from(out_depth)).expect("out-slot index always fits usize");
        let start = shm::out_slots_off(in_depth) + index * shm::SLOT_BYTES;
        &bytes(&self.mem)[start..start + shm::SLOT_BYTES]
    }
}

/// The segment as a read-only byte slice.
fn bytes(mem: &Shmem) -> &[u8] {
    // SAFETY: `Shmem::as_slice` is `unsafe` because the crate cannot
    // prove the mapped range immutable — another process maps the same
    // pages. The L1 slot protocol supplies that guarantee: header bytes
    // are written once by this owner before spawn, and each slot is
    // touched by exactly one side at a time under the strict-
    // alternation doorbell rule (the producer reuses slot `seq % D`
    // only after `done seq - D`; `View::out_slot` is called only on the
    // reader thread after the `BlockDone` acquire edge; the length is
    // the attach/create-validated mapping length).
    #[expect(unsafe_code)]
    unsafe {
        mem.as_slice()
    }
}

/// The segment as a mutable byte slice.
fn bytes_mut(mem: &mut Shmem) -> &mut [u8] {
    // SAFETY: same protocol argument as `bytes` — header bytes are
    // written once before spawn, and the in-slot written through
    // `View::in_slot_mut` is owned by this host between doorbells under
    // the strict-alternation rule, so no other process touches it
    // meanwhile; the `&mut Shmem` borrow keeps the slice exclusive
    // within this call.
    #[expect(unsafe_code)]
    unsafe {
        mem.as_slice_mut()
    }
}

pub(crate) fn get_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("u32 field is 4 bytes"),
    )
}

pub(crate) fn get_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("u64 field is 8 bytes"),
    )
}

pub(crate) fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
