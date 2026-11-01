//! The video-frame shared-memory window a `CreateVideoInstance`
//! command names: the host owns the segment and the child attaches
//! through it, so the layout lives in `plugin-core`'s
//! [`wire::vshm`](ruviform_plugin_core::wire::vshm) module as wire
//! vocabulary (its own documentation covers the layout and the
//! capacity arithmetic the `u64` values carry).
//!
//! `Segment::attach` validates two layers, mirroring the audio
//! `shm::Segment::attach` contract:
//!
//! * **structural** — the fixed header's `magic`/`wire_version`/
//!   `reserved`/`in_off`/`out_off` fields and the mapped length, via
//!   [`wire::vshm::validate_header`];
//! * **config agreement** — both declared region sizes equal the
//!   `frame_bytes(config)` the segment was created for, and that
//!   frame size is inside [`WIRE_VIDEO_MAX_FRAME_BYTES`]. The host
//!   already applies the same bound at create time, but the child
//!   re-checks symmetrically: a corrupted or mismatched segment is
//!   rejected here, never trusted.
//!
//! `run`/`run_video` only reach `unsafe` in this module's two scoped
//! accessors (the same audited pattern as `shm.rs`) —
//! `#[expect(unsafe_code)]` is the enforced pairing of the lint with
//! its SAFETY comment.

use ruviform_plugin_core::VideoProcessConfig;
use ruviform_plugin_core::wire;
use ruviform_plugin_core::wire::vshm;
use shared_memory::{Shmem, ShmemConf};

/// One attached video-frame segment — the symmetric `shm::Segment`
/// for the video kind. The declared `in`/`out` region sizes are
/// equal (one frame each way) and pinned to the config the create
/// command carried.
pub(crate) struct Segment {
    mem: Shmem,
    in_off: usize,
    in_bytes: usize,
    out_off: usize,
    out_bytes: usize,
}

impl Segment {
    /// Attaches to the host-owned segment `key` names and validates
    /// it end to end: structurally through
    /// [`wire::vshm::validate_header`], then for config agreement —
    /// both regions must be exactly the `frame_bytes(config)` the
    /// create command carried, bounded by
    /// [`WIRE_VIDEO_MAX_FRAME_BYTES`]. Any mismatch is a plain `Err`:
    /// the caller maps it to `HostError::Ipc`, mirroring how a failed
    /// audio attach surfaces.
    pub(crate) fn attach(key: &str, config: &VideoProcessConfig) -> Result<Self, &'static str> {
        let mem = ShmemConf::new()
            .os_id(key)
            .open()
            .map_err(|_| "shared-memory segment open failed")?;
        let bytes = bytes(&mem);
        let (in_bytes, out_bytes) =
            vshm::validate_header(bytes).ok_or("shared-memory header mismatch")?;
        let frame_bytes = vshm::frame_bytes(config);
        if frame_bytes > wire::WIRE_VIDEO_MAX_FRAME_BYTES {
            return Err("frame size exceeds the video wire bound");
        }
        if u64::from(in_bytes) != frame_bytes || u64::from(out_bytes) != frame_bytes {
            return Err("shared-memory region size differs from the config frame size");
        }
        Ok(Self {
            mem,
            in_off: vshm::HEADER_LEN,
            in_bytes: in_bytes as usize,
            out_off: vshm::HEADER_LEN + in_bytes as usize,
            out_bytes: out_bytes as usize,
        })
    }

    /// The input region — the frame the host wrote for `VideoRender`
    /// — as a byte slice.
    pub(crate) fn in_region(&self) -> &[u8] {
        &bytes(&self.mem)[self.in_off..self.in_off + self.in_bytes]
    }

    /// The input region's declared capacity in bytes — the bound a
    /// `VideoRender` input descriptor is region-fit checked against.
    pub(crate) fn in_bytes(&self) -> usize {
        self.in_bytes
    }

    /// The output region — the frame the child writes back for
    /// `Rendered` — as a mutable byte slice.
    pub(crate) fn out_region_mut(&mut self) -> &mut [u8] {
        &mut bytes_mut(&mut self.mem)[self.out_off..self.out_off + self.out_bytes]
    }
}

/// A shared view of the raw segment bytes. SAFETY: `as_slice` is the
/// crate's read-only view over the mapped region; the mapping stays
/// pinned for `mem`'s lifetime, and a shared `&[u8]` exposes no
/// mutable aliasing beyond the inter-process window the protocol
/// already treats as concurrently shared (only the header's atomic
/// fields carry cross-process synchronization for the audio shape;
/// the video shape is message-driven, so reads and writes are
/// separated by the command/reply round trip).
#[expect(unsafe_code)]
fn bytes(mem: &Shmem) -> &[u8] {
    unsafe { mem.as_slice() }
}

/// An exclusive view of the raw segment bytes. SAFETY: `as_slice_mut`
/// yields `&mut [u8]` over the mapped region, which is sound under
/// `&mut Shmem` here for the same reason `bytes` is — plus the fact
/// that the wire protocol serializes access: the child touches the
/// out region only between reading `VideoRender` and writing
/// `Rendered`, so no host reader aliases it.
#[expect(unsafe_code)]
fn bytes_mut(mem: &mut Shmem) -> &mut [u8] {
    unsafe { mem.as_slice_mut() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::FrameRate;
    use ruviform_plugin_core::wire::vshm;
    use ruviform_video_core::PixelFormat;
    use shared_memory::ShmemConf;

    /// A small RGBA8 config — 64×64 × 4 B = 16 KiB per frame.
    fn config() -> VideoProcessConfig {
        VideoProcessConfig::new(PixelFormat::Rgba8, 64, 64, FrameRate::new(30, 1).unwrap())
            .expect("valid config")
    }

    fn unique_key(tag: &str) -> String {
        // macOS `shm_open` caps the name at `PSHMNAMLEN` (~30 bytes
        // including the leading `/`; the production key hit the same
        // limit in Issue #207). Keep the test key ≤ 30 under any pid
        // width — `ENAMETOOLONG` then can never fire (Issue #220).
        let key = format!("/rvpe-{}-{tag}", std::process::id());
        assert!(
            key.len() <= 30,
            "shm test key exceeds macOS name limit: {key}"
        );
        key
    }

    /// `vshm::segment_len` as a usize — always fits (bounded by
    /// `WIRE_VIDEO_MAX_FRAME_BYTES`).
    fn segment_len(frame_bytes: u64) -> usize {
        usize::try_from(vshm::segment_len(frame_bytes)).expect("segment len fits usize")
    }

    /// `vshm::HEADER_LEN` as a u32 — the header is 28 bytes.
    fn header_len_u32() -> u32 {
        u32::try_from(vshm::HEADER_LEN).expect("header len fits u32")
    }

    /// Creates a host-side segment of `size` bytes and writes
    /// `header` into its first [`vshm::HEADER_LEN`] bytes — the
    /// minimal-host counterpart of the child's attach.
    fn create(key: &str, size: usize, header: &[u8; vshm::HEADER_LEN]) -> Shmem {
        let mut mem = ShmemConf::new()
            .os_id(key)
            .size(size)
            .create()
            .expect("create shared-memory segment");
        bytes_mut(&mut mem)[..vshm::HEADER_LEN].copy_from_slice(header);
        mem
    }

    #[test]
    fn attach_accepts_a_matching_segment() {
        let config = config();
        let frame_bytes = vshm::frame_bytes(&config);
        let key = unique_key("accept");
        let header = vshm::header_bytes(
            u32::try_from(frame_bytes).expect("frame bytes fit u32"),
            u32::try_from(frame_bytes).expect("frame bytes fit u32"),
        );
        let _mem = create(&key, segment_len(frame_bytes), &header);
        let mut segment = Segment::attach(&key, &config).expect("valid segment attaches");
        assert_eq!(segment.in_bytes() as u64, frame_bytes);
        // Region accessors slice the two halves exactly.
        assert_eq!(segment.in_region().len() as u64, frame_bytes);
        assert_eq!(
            segment.out_region_mut().len() as u64,
            frame_bytes,
            "out region is one frame"
        );
    }

    #[test]
    fn attach_rejects_a_missing_segment() {
        assert!(Segment::attach(&unique_key("missing"), &config()).is_err());
    }

    #[test]
    fn attach_rejects_every_structural_mismatch() {
        let config = config();
        let frame_bytes = vshm::frame_bytes(&config);
        let fb32 = u32::try_from(frame_bytes).expect("frame bytes fit u32");
        let cases: Vec<(String, [u8; vshm::HEADER_LEN], usize)> = {
            let valid = || vshm::header_bytes(fb32, fb32);
            let mut cases = Vec::new();
            // magic mismatch.
            let mut h = valid();
            h[0..4].copy_from_slice(b"BAD!");
            cases.push(("magic".to_owned(), h, segment_len(frame_bytes)));
            // wire_version mismatch.
            let mut h = valid();
            h[vshm::OFF_WIRE_VERSION..vshm::OFF_WIRE_VERSION + 4]
                .copy_from_slice(&2_u32.to_le_bytes());
            cases.push(("version".to_owned(), h, segment_len(frame_bytes)));
            // in_off != HEADER_LEN.
            let mut h = valid();
            h[vshm::OFF_IN_OFF..vshm::OFF_IN_OFF + 4].copy_from_slice(&0_u32.to_le_bytes());
            cases.push(("in_off".to_owned(), h, segment_len(frame_bytes)));
            // out_off != HEADER_LEN + in_bytes.
            let mut h = valid();
            h[vshm::OFF_OUT_OFF..vshm::OFF_OUT_OFF + 4]
                .copy_from_slice(&(header_len_u32() + fb32 + 4).to_le_bytes());
            cases.push(("out_off".to_owned(), h, segment_len(frame_bytes)));
            // reserved != 0.
            let mut h = valid();
            h[vshm::OFF_RESERVED..vshm::OFF_RESERVED + 4].copy_from_slice(&1_u32.to_le_bytes());
            cases.push(("reserved".to_owned(), h, segment_len(frame_bytes)));
            // mapped len shorter than the declared out_off +
            // out_bytes — the OS rounds the mapping up to the
            // allocation granularity, so the violation is the
            // declared layout (out_bytes inflated past any rounding
            // of a valid-size segment), not the requested size.
            cases.push((
                "len".to_owned(),
                vshm::header_bytes(fb32, u32::MAX),
                segment_len(frame_bytes),
            ));
            // in_bytes != frame_bytes(config) — structurally
            // consistent (out_off tracks in_bytes) but the config
            // agreement check must fire.
            let smaller = fb32 / 2;
            cases.push((
                "in_bytes".to_owned(),
                vshm::header_bytes(smaller, smaller),
                vshm::HEADER_LEN + 2 * smaller as usize,
            ));
            // out_bytes != frame_bytes(config).
            cases.push((
                "out_bytes".to_owned(),
                vshm::header_bytes(fb32, smaller),
                usize::try_from(vshm::HEADER_LEN as u64 + frame_bytes + u64::from(smaller))
                    .expect("segment len fits usize"),
            ));
            cases
        };
        for (index, (tag, header, size)) in cases.into_iter().enumerate() {
            let key = unique_key(&format!("reject-{index}"));
            let _mem = create(&key, size, &header);
            assert!(
                Segment::attach(&key, &config).is_err(),
                "{tag} must be rejected"
            );
        }
    }

    #[test]
    fn attach_rechecks_the_frame_size_bound() {
        // 8192×8192 × RgbaF32(16 B) = 1 GiB > WIRE_VIDEO_MAX_FRAME_BYTES
        // — the config-derived bound the child re-checks
        // symmetrically even though the segment is small and valid.
        let oversized = VideoProcessConfig::new(
            PixelFormat::RgbaF32,
            8_192,
            8_192,
            FrameRate::new(30, 1).unwrap(),
        )
        .expect("valid config");
        assert!(
            vshm::frame_bytes(&oversized) > wire::WIRE_VIDEO_MAX_FRAME_BYTES,
            "fixture must exceed the frame bound"
        );
        let config = config();
        let frame_bytes = vshm::frame_bytes(&config);
        let key = unique_key("cap");
        let header = vshm::header_bytes(
            u32::try_from(frame_bytes).expect("frame bytes fit u32"),
            u32::try_from(frame_bytes).expect("frame bytes fit u32"),
        );
        let _mem = create(&key, segment_len(frame_bytes), &header);
        assert!(Segment::attach(&key, &oversized).is_err());
    }
}
