//! Internal source adapter: the media-io [`MediaSource`] byte-source
//! contract → the backend's distinct `MediaSource` trait.
//!
//! The two crates define unrelated `MediaSource` traits:
//!
//! - media-io's [`MediaSource`] (`Read + Seek + Send`, blanket-implemented
//!   for `Box<dyn MediaSource>`) is the boundary's input port — the
//!   proof-of-concept's "任意バイトストリーム" surface.
//! - the backend's `MediaSource` is `Read + Seek + Send + Sync`, and
//!   expects runtime answers for `is_seekable` and `byte_len`.
//!
//! This adapter is the only place the two spellings meet. It is crate-
//! internal: exposing it would leak a backend trait into the public API.
//!
//! The boundary port cannot answer those queries directly — it only
//! promises the `Seek` trait's presence, not that seeking works — so the
//! adapter probes the source once at construction: save the position,
//! seek to the end for the length, restore. A source whose `Seek` fails
//! is reported as unseekable, which the backend surfaces as
//! `SeekError::Unseekable` →
//! [`ErrorKind::Unsupported`](ruviform_media_io::ErrorKind::Unsupported) —
//! matching the [`Demuxer::seek`](ruviform_media_io::Demuxer::seek)
//! contract.
//!
//! The `Sync` gap is closed by a `Mutex`. The lock is never contended
//! across the boundary (the demuxer owns the adapter exclusively), and
//! [`Read`]/[`Seek`] use `Mutex::get_mut` — the zero-cost path that
//! cannot block. `is_seekable`/`byte_len` read the cached probe result
//! and never touch the lock.

use std::io::{Read, Seek, SeekFrom};
use std::sync::Mutex;

use ruviform_media_io::MediaSource;
use symphonia::core::io::MediaSource as BackendSource;

/// Wraps a media-io [`MediaSource`] as the backend's `MediaSource`.
///
/// `Sync` is supplied by the `Mutex`; the port contract guarantees `Send`
/// only, so a shared reference to the inner source is impossible without
/// the lock. `seekable`/`len` hold the one-time probe result.
pub(crate) struct SourceAdapter {
    inner: Mutex<Box<dyn MediaSource>>,
    seekable: bool,
    len: Option<u64>,
}

impl SourceAdapter {
    /// Adapts an owned media-io source for the backend probe, testing
    /// seekability and byte length once (see the module docs).
    pub(crate) fn new(mut inner: Box<dyn MediaSource>) -> Self {
        let (seekable, len) = probe(&mut *inner);
        Self {
            inner: Mutex::new(inner),
            seekable,
            len,
        }
    }
}

/// Probes `source` for real seek support and total byte length.
///
/// `Seek::stream_len` is not used: it was stabilized in Rust 1.87, above
/// the workspace `rust-version` (1.85), so the save → `End` → restore
/// sequence is written out by hand. Any step failing marks the source
/// unseekable — a `Seek` that cannot move or restore cannot serve the
/// backend's bisection seeks anyway. The position is restored on the
/// success path; when the restore itself fails the source is already
/// declared unseekable.
fn probe(source: &mut dyn MediaSource) -> (bool, Option<u64>) {
    let Ok(saved) = source.stream_position() else {
        return (false, None);
    };
    let Ok(len) = source.seek(SeekFrom::End(0)) else {
        let _ = source.seek(SeekFrom::Start(saved));
        return (false, None);
    };
    if source.seek(SeekFrom::Start(saved)).is_err() {
        return (false, None);
    }
    (true, Some(len))
}

impl Read for SourceAdapter {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner
            .get_mut()
            .map_err(|_| std::io::Error::other("media source lock poisoned"))?
            .read(buf)
    }
}

impl Seek for SourceAdapter {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner
            .get_mut()
            .map_err(|_| std::io::Error::other("media source lock poisoned"))?
            .seek(pos)
    }
}

impl BackendSource for SourceAdapter {
    fn is_seekable(&self) -> bool {
        self.seekable
    }

    fn byte_len(&self) -> Option<u64> {
        self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// `Read + Seek + Send` stub whose `Seek` always fails — stands in
    /// for a stream that satisfies the port's bounds but cannot seek.
    struct BrokenSeek {
        cursor: Cursor<Vec<u8>>,
    }

    impl Read for BrokenSeek {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.cursor.read(buf)
        }
    }

    impl Seek for BrokenSeek {
        fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
            Err(std::io::Error::other("seek unsupported"))
        }
    }

    #[test]
    fn seekable_source_reports_real_length() {
        let adapter = SourceAdapter::new(Box::new(Cursor::new(vec![0_u8; 11])));
        assert!(adapter.is_seekable());
        assert_eq!(adapter.byte_len(), Some(11));
    }

    #[test]
    fn unseekable_source_is_reported_honestly() {
        let adapter = SourceAdapter::new(Box::new(BrokenSeek {
            cursor: Cursor::new(vec![0_u8; 7]),
        }));
        assert!(!adapter.is_seekable());
        assert_eq!(adapter.byte_len(), None);
    }

    #[test]
    fn probe_preserves_position() {
        let mut cursor = Cursor::new(vec![0_u8; 16]);
        cursor.seek(SeekFrom::Start(4)).unwrap();
        let mut adapter = SourceAdapter::new(Box::new(cursor));
        assert_eq!(adapter.stream_position().unwrap(), 4);
    }
}
