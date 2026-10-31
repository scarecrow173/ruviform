//! RIFF chunk plumbing for the AVI demuxer.
//!
//! AVI is a RIFF form: an 8-byte chunk header (`id` fourCC + little-endian
//! `u32` size, the size excluding the header) followed by a body padded to
//! an even boundary. `LIST`/`RIFF` chunks open with a 4-byte form type.
//! All structures here are byte-level facts — semantic interpretation of
//! `strh`/`strf` bodies lives in `crate::format`.

use std::io::SeekFrom;

use ruviform_media_io::{Error, ErrorKind, MediaSource};

/// The RIFF form type this backend owns (`RIFF....AVI `).
pub(crate) const FORM_AVI: [u8; 4] = *b"AVI ";
/// `LIST` chunk id.
pub(crate) const LIST: [u8; 4] = *b"LIST";
/// `RIFF` chunk id.
pub(crate) const RIFF: [u8; 4] = *b"RIFF";

/// A RIFF chunk header: `id` plus `size` — the body length in bytes,
/// excluding the 8-byte header and the odd-byte pad.
#[derive(Copy, Clone, Debug)]
pub(crate) struct ChunkHead {
    /// The fourCC chunk id.
    pub id: [u8; 4],
    /// Body length in bytes (header and pad excluded).
    pub size: u64,
}

impl ChunkHead {
    /// Byte offset one past the padded body — the next chunk header.
    pub(crate) const fn next_off(self, head_off: u64) -> u64 {
        head_off + 8 + self.size + (self.size & 1)
    }

    /// `true` when `id` begins with two ASCII digits — the `NNxx` media
    /// chunk id form (`00dc`, `01wb`, …) used inside `LIST movi`.
    pub(crate) const fn stream_chunk(self) -> Option<u32> {
        if self.id[0].is_ascii_digit() && self.id[1].is_ascii_digit() {
            Some((self.id[0] - b'0') as u32 * 10 + (self.id[1] - b'0') as u32)
        } else {
            None
        }
    }
}

/// Reads one chunk header at `src`'s current position, expecting it to lie
/// fully inside `region_end` (a byte offset one past the containing
/// region). Returns `Ok(None)` when exactly `region_end` is reached —
/// the region's natural end — and [`ErrorKind::InvalidData`] on a short or
/// overflowing header.
pub(crate) fn read_head_at(
    src: &mut dyn MediaSource,
    region_end: u64,
    pos: u64,
) -> Result<Option<ChunkHead>, Error> {
    if pos == region_end {
        return Ok(None);
    }
    if pos + 8 > region_end {
        return Err(crate::err(
            ErrorKind::InvalidData,
            "riff: chunk header extends past its containing region",
        ));
    }
    let mut buf = [0u8; 8];
    src.read_exact(&mut buf)
        .map_err(|_| crate::err(ErrorKind::Io, "riff: chunk header short read"))?;
    let size = u64::from(u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]));
    let head = ChunkHead {
        id: [buf[0], buf[1], buf[2], buf[3]],
        size,
    };
    // The padded body must fit inside the containing region.
    if head.next_off(pos) > region_end {
        return Err(crate::err(
            ErrorKind::InvalidData,
            "riff: chunk body extends past its containing region",
        ));
    }
    Ok(Some(head))
}

/// Advances `src` past `n` bytes, preferring a seek skip and falling back
/// to read-and-discard on non-seekable sources.
pub(crate) fn skip(src: &mut dyn MediaSource, n: u64) -> Result<(), Error> {
    if n == 0 {
        return Ok(());
    }
    if src
        .seek(SeekFrom::Current(i64::try_from(n).unwrap_or(i64::MAX)))
        .is_ok()
    {
        return Ok(());
    }
    // Non-seekable fallback: bounded discard reads.
    let mut left = n;
    let mut buf = [0u8; 4096];
    while left > 0 {
        let take = usize::try_from(left).unwrap_or(usize::MAX).min(buf.len());
        src.read_exact(&mut buf[..take])
            .map_err(|_| crate::err(ErrorKind::Io, "riff: skip short read"))?;
        left -= take as u64;
    }
    Ok(())
}

/// Reads a bounded body region into `buf`.
pub(crate) fn read_body(src: &mut dyn MediaSource, buf: &mut [u8]) -> Result<(), Error> {
    src.read_exact(buf)
        .map_err(|_| crate::err(ErrorKind::Io, "riff: chunk body short read"))
}
