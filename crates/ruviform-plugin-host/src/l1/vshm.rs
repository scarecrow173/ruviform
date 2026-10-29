//! The host-owned video shared-memory segment (`vshm`) — one per L1
//! video instance, sized from its [`VideoProcessConfig`] and carrying
//! one input region plus one output region behind the fixed
//! [`wire::vshm`] header. This is the frame transport of
//! [`Command::VideoRender`](ruviform_plugin_core::wire::Command::VideoRender):
//! the facade repacks the caller's `VideoFrame` into the `in` region
//! before the request, the child renders into the `out` region, and
//! the facade restores a `VideoFrame` from it after the `Rendered`
//! reply arrives.
//!
//! `shared_memory` stays confined to `l1::shm` + this module — its
//! types never appear on the public API. The segment is created and
//! owned by the host; the child only attaches (`open`) by the `os_id`
//! carried in `CreateVideoInstance`'s `shm_key` field and never
//! creates or unlinks. Dropping the [`Segment`] unlinks the named
//! object exactly like the audio `shm::Segment` (POSIX `shm_unlink`
//! via the owner `Drop`; on Windows the mapping is unmapped and its
//! persistence file is delete-on-close renamed).
//!
//! ## Ownership window
//!
//! Unlike the audio segment — whose slots are touched by the driver
//! and reader threads under strict alternation — every access here
//! happens on the host control thread, serialized by the synchronous
//! `VideoRender` request→reply window: the host writes the `in`
//! region only while no render is in flight, the child reads `in`
//! and writes `out` only while it serves the request, and the host
//! reads `out` only after a `Rendered` reply arrived (never on an
//! `Err` reply — commit-on-success). Ordering across the process
//! boundary rests on the `fence(Ordering::Release)` after the `in`
//! write, the pipe-write/read syscall boundary, and the
//! `fence(Ordering::Acquire)` before the `out` read — the same
//! `Ordering` rules the audio in/out slots and the runner serve loop
//! follow. `Shmem` is `!Send`, so the owner `Segment` lives on the
//! facade (`VideoL1Instance`) directly — no driver-thread relay like
//! the audio data plane needs.
//!
//! ## Byte layout
//!
//! The layout is defined once in [`wire::vshm`] — the authoritative
//! wire-side contract the child's attach validates. Header —
//! [`wire::vshm::HEADER_LEN`] = 28 bytes:
//!
//! | offset | field | type | value |
//! | --- | --- | --- | --- |
//! | +0 | `magic` | `u32` | [`wire::vshm::MAGIC`] (`"RVLV"`) |
//! | +4 | `wire_version` | `u32` | [`wire::WIRE_VERSION`] |
//! | +8 | `in_off` | `u32` | [`wire::vshm::HEADER_LEN`] |
//! | +12 | `in_bytes` | `u32` | `frame_bytes(config)` |
//! | +16 | `out_off` | `u32` | `HEADER_LEN + in_bytes` |
//! | +20 | `out_bytes` | `u32` | `frame_bytes(config)` |
//! | +24 | `reserved` | `u32` | `0` |
//!
//! Both regions are `frame_bytes(config)` long and carry planes in
//! canonical packing (`stride == row_bytes`,
//! `offset(p) = Σ len(0..p)` — see [`wire::vshm::plane_layout`]).

use ruviform_plugin_core::{VideoProcessConfig, wire::vshm};
use shared_memory::{Shmem, ShmemConf, ShmemError};

use super::shm::{self, get_u32};

/// Distinct keys tried before a create gives up — same budget the
/// audio [`shm::Segment`] uses: collisions are a crashed host's
/// stale segment, so a fresh key is the defined recovery.
const MAX_KEY_ATTEMPTS: u32 = 8;

/// Why one create attempt failed — only `Exists` is retryable
/// (another key may succeed; a real mapping error repeats).
#[derive(Debug)]
pub(crate) enum CreateFail {
    /// `os_id` already taken — a stale segment from a crashed host.
    Exists,
    /// Any other create/header failure.
    Other,
}

/// A created, header-initialized video segment owned by this host —
/// the `VideoL1Instance` facade's frame transport. `Drop` unlinks
/// the named object; the child's attach only borrows the `os_id`.
pub(crate) struct Segment {
    mem: Shmem,
    /// `in`-region byte length == `out`-region byte length —
    /// `frame_bytes(config)` at create (both header fields carry
    /// it; v1 canonical regions are always equal).
    region_bytes: usize,
    /// Total mapped bytes the child should expect to cover
    /// ([`vshm::segment_len`]); on Windows the reported mapping
    /// length may be rounded up to the allocation granularity, so
    /// this records the *declared* length.
    declared_len: usize,
}

impl Segment {
    /// Creates a uniquely-keyed segment for `slot`, sized from
    /// `config` — `HEADER_LEN + 2 × frame_bytes(config)` with the
    /// fixed header written — retrying [`MAX_KEY_ATTEMPTS`]
    /// distinct keys when a stale segment name collides
    /// (`ShmemError::MappingIdExists`). Returns `Err(())` on any
    /// mapping/bounds failure — the caller maps it to
    /// [`ruviform_plugin_core::HostError::Ipc`]. The `config`
    /// bound check (`frame_bytes ≤ WIRE_VIDEO_MAX_FRAME_BYTES`)
    /// is the caller's precondition — run it before paying the
    /// create.
    pub(crate) fn create(slot: u32, config: &VideoProcessConfig) -> Result<Self, ()> {
        Self::create_keyed(slot, config, &shm::unique_key)
    }

    /// The key-generation seam behind [`Segment::create`] — tests
    /// inject a generator to drive `MappingIdExists` collisions
    /// deterministically.
    pub(crate) fn create_keyed(
        slot: u32,
        config: &VideoProcessConfig,
        key: &dyn Fn(u32) -> String,
    ) -> Result<Self, ()> {
        for _ in 0..MAX_KEY_ATTEMPTS {
            match Self::create_named(&key(slot), config) {
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
    pub(crate) fn create_named(key: &str, config: &VideoProcessConfig) -> Result<Self, CreateFail> {
        let frame_bytes = vshm::frame_bytes(config);
        // The declared mapping length and the u32 header fields —
        // `frame_bytes` sits far below `u32::MAX` under the caller's
        // bound check, but an unbounded `VideoProcessConfig` can
        // still arrive here from a bypassed gate: refuse the create
        // rather than panic inside `header_bytes`.
        let declared_len =
            usize::try_from(vshm::segment_len(frame_bytes)).map_err(|_| CreateFail::Other)?;
        // `out_off = HEADER_LEN + in_bytes` must still fit `u32`:
        // values in `(u32::MAX - HEADER_LEN, u32::MAX]` pass the
        // `try_from` but panic inside `header_bytes` — refuse here.
        let header_len = u32::try_from(vshm::HEADER_LEN).map_err(|_| CreateFail::Other)?;
        let in_bytes = u32::try_from(frame_bytes)
            .ok()
            .filter(|&value| value <= u32::MAX - header_len)
            .ok_or(CreateFail::Other)?;
        let mut mem = ShmemConf::new()
            .os_id(key)
            .size(declared_len)
            .create()
            .map_err(|error| match error {
                ShmemError::MappingIdExists => CreateFail::Exists,
                _ => CreateFail::Other,
            })?;
        bytes_mut(&mut mem)[..vshm::HEADER_LEN]
            .copy_from_slice(&vshm::header_bytes(in_bytes, in_bytes));
        let region_bytes = usize::try_from(frame_bytes).map_err(|_| CreateFail::Other)?;
        Ok(Self {
            mem,
            region_bytes,
            declared_len,
        })
    }

    /// The `os_id` the child attaches with — the value carried by
    /// `CreateVideoInstance`'s `shm_key` field.
    pub(crate) fn key(&self) -> &str {
        self.mem.get_os_id()
    }

    /// Each region's byte length — `frame_bytes(config)` at create.
    /// The `Rendered.output` semantic gate bounds every plane
    /// `offset + len` by this.
    pub(crate) fn region_bytes(&self) -> usize {
        self.region_bytes
    }

    /// The declared segment length (header + both regions). On
    /// Windows the mapping's reported length can exceed this; the
    /// invariant is "the mapping covers it".
    // Test-support surface — reachable through the `#[path]` include
    // in `tests/l1_proxy_e2e.rs`, not through in-crate code.
    #[allow(dead_code)]
    pub(crate) fn declared_len(&self) -> usize {
        self.declared_len
    }

    /// The `in` region as a mutable byte slice — the facade's repack
    /// destination. Called only while no `VideoRender` is in flight
    /// (the request→reply window is the sole validity period — see
    /// module docs).
    pub(crate) fn in_region_mut(&mut self) -> &mut [u8] {
        let end = vshm::HEADER_LEN + self.region_bytes;
        &mut bytes_mut(&mut self.mem)[vshm::HEADER_LEN..end]
    }

    /// The `out` region as a read-only byte slice — read only after
    /// a `Rendered` reply and its `Acquire` fence (never on an `Err`
    /// reply — commit-on-success).
    pub(crate) fn out_region(&self) -> &[u8] {
        let start = vshm::HEADER_LEN + self.region_bytes;
        &bytes(&self.mem)[start..start + self.region_bytes]
    }

    /// Re-reads the header and checks every field against the
    /// values this segment was created with — `magic`, this
    /// revision's `wire_version`, the fixed offsets, zero `reserved`
    /// and `in_bytes == out_bytes == frame_bytes(config)` — the
    /// self-verification half of the header write/verify pair.
    // Test-support surface — reachable through the `#[path]` include
    // in `tests/l1_proxy_e2e.rs`, not through in-crate code.
    #[allow(dead_code)]
    pub(crate) fn verify_header(&self, config: &VideoProcessConfig) -> bool {
        let bytes = bytes(&self.mem);
        let Some((in_bytes, out_bytes)) = vshm::validate_header(bytes) else {
            return false;
        };
        let expected = u32::try_from(vshm::frame_bytes(config)).unwrap_or(u32::MAX);
        in_bytes == expected && out_bytes == expected
    }

    /// The `in_bytes` header field as written — the unit-test pin
    /// for the field placement table (read through
    /// [`wire::vshm`]'s offsets).
    // Test-support surface — reachable through the `#[path]` include
    // in `tests/l1_proxy_e2e.rs`, not through in-crate code.
    #[allow(dead_code)]
    pub(crate) fn header_fields(&self) -> (u32, u32) {
        let bytes = bytes(&self.mem);
        (
            get_u32(bytes, vshm::OFF_IN_BYTES),
            get_u32(bytes, vshm::OFF_OUT_BYTES),
        )
    }
}

/// The segment as a read-only byte slice.
fn bytes(mem: &Shmem) -> &[u8] {
    // SAFETY: `Shmem::as_slice` is `unsafe` because the crate cannot
    // prove the mapped range immutable — another process maps the same
    // pages. The vshm protocol supplies that guarantee for this read:
    // the `out` region is read only on the host control thread, after
    // a `Rendered` reply and its `Acquire` fence — the child touches
    // the region only while it serves a `VideoRender`, and the
    // synchronous request→reply window serializes those accesses
    // (header bytes themselves are written once at create).
    #[expect(unsafe_code)]
    unsafe {
        mem.as_slice()
    }
}

/// The segment as a mutable byte slice.
fn bytes_mut(mem: &mut Shmem) -> &mut [u8] {
    // SAFETY: same protocol argument as `bytes` — the `in` region is
    // written only on the host control thread while no render is in
    // flight (the child reads it only inside the request→reply
    // window), and the `&mut Shmem` borrow keeps the slice exclusive
    // within this call.
    #[expect(unsafe_code)]
    unsafe {
        mem.as_slice_mut()
    }
}

#[cfg(test)]
mod tests {
    //! The `vshm` create/header/region pins: the fixed header lands
    //! `wire::vshm`'s canonical field table, the regions slice to
    //! exactly `frame_bytes(config)`, a stale-name collision retries
    //! onto a fresh key, and an unbounded `config` refuses the
    //! create rather than panic or map a giant segment.

    use std::cell::Cell;

    use ruviform_media_core::FrameRate;
    use ruviform_plugin_core::wire::vshm;
    use ruviform_video_core::PixelFormat;

    use super::*;

    fn config(format: PixelFormat, width: u32, height: u32) -> VideoProcessConfig {
        VideoProcessConfig::new(format, width, height, FrameRate::new(30, 1).unwrap()).unwrap()
    }

    /// `create` writes the canonical header and slices the regions
    /// to `frame_bytes` — `verify_header`/`header_fields`/
    /// `declared_len` all read the same table.
    #[test]
    fn create_writes_the_canonical_header() {
        let config = config(PixelFormat::Yuv420p, 64, 48);
        let segment = Segment::create(0, &config).expect("segment creates");
        let frame_bytes = vshm::frame_bytes(&config);
        assert!(segment.verify_header(&config));
        assert_eq!(
            segment.header_fields(),
            (
                u32::try_from(frame_bytes).unwrap(),
                u32::try_from(frame_bytes).unwrap()
            )
        );
        assert_eq!(
            segment.region_bytes(),
            usize::try_from(frame_bytes).unwrap()
        );
        assert_eq!(
            segment.declared_len(),
            usize::try_from(vshm::segment_len(frame_bytes)).unwrap()
        );
        assert_ne!(segment.key(), "");
    }

    /// The `in`/`out` region slices cover exactly `frame_bytes`
    /// each and the `in` write is visible through the accessor —
    /// the repack/restore windows' extents.
    #[test]
    fn regions_slice_to_frame_bytes() {
        let config = config(PixelFormat::Yuv420p, 64, 48);
        let mut segment = Segment::create(0, &config).expect("segment creates");
        let frame_bytes = usize::try_from(vshm::frame_bytes(&config)).unwrap();
        segment.in_region_mut()[0] = 0xAB;
        segment.in_region_mut()[frame_bytes - 1] = 0xCD;
        assert_eq!(segment.out_region().len(), frame_bytes);
    }

    /// A `MappingIdExists` collision retries onto the next key —
    /// the stale-segment recovery `create_keyed` exists for. The
    /// injected generator collides once with a live segment's key,
    /// then serves a fresh name.
    #[test]
    fn create_keyed_retries_a_stale_name_collision() {
        let config = config(PixelFormat::Yuv420p, 64, 48);
        let staler = Segment::create_named("rvlv-test-stale", &config).expect("stale creates");
        let stale_key = staler.key().to_owned();
        let attempt = Cell::new(0_u32);
        let segment = Segment::create_keyed(0, &config, &|slot| {
            attempt.set(attempt.get() + 1);
            if attempt.get() == 1 {
                stale_key.clone()
            } else {
                format!("rvlv-test-fresh-{slot}")
            }
        })
        .expect("a fresh key succeeds after one collision");
        assert_eq!(attempt.get(), 2);
        assert_ne!(segment.key(), stale_key);
        assert!(segment.verify_header(&config));
    }

    /// A `VideoProcessConfig` `VideoProcessConfig::new` admits but
    /// the wire bounds reject (frame bytes past the `u32` header
    /// fields) refuses the create — `CreateFail::Other`, no panic,
    /// no giant mapping.
    #[test]
    fn create_named_refuses_an_unbounded_config() {
        let config = config(PixelFormat::Rgba8, u32::MAX, u32::MAX);
        assert!(matches!(
            Segment::create_named("rvlv-test-huge", &config),
            Err(CreateFail::Other)
        ));
    }
}
