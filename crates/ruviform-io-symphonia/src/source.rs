//! Internal source adapter: the media-io [`MediaSource`] byte-source
//! contract → the backend's distinct `MediaSource` trait.
//!
//! The two crates define unrelated `MediaSource` traits:
//!
//! - media-io's [`MediaSource`] (`Read + Seek + Send`, blanket-implemented
//!   for `Box<dyn MediaSource>`) is the boundary's input port — the
//!   proof-of-concept's "任意バイトストリーム" surface.
//! - the backend's `MediaSource` is `Read + Seek + Send + Sync`, with an
//!   extra `byte_len` hint and `is_seekable` flag.
//!
//! This adapter is the only place the two spellings meet. It is crate-
//! internal: exposing it would leak a backend trait into the public API.
//!
//! The `Sync` gap is closed by a `Mutex`. The lock is never contended
//! across the boundary (the demuxer owns the adapter exclusively), and
//! [`Read`]/[`Seek`]/[`Write`] use `Mutex::get_mut` — the zero-cost path
//! that cannot block. Only `byte_len(&self)` takes the lock; poisoning is
//! handled gracefully (`None` = "length unknown") rather than by panic.

use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Mutex;

use ruviform_media_io::MediaSource;
use symphonia::core::io::MediaSource as BackendSource;

/// Wraps a media-io [`MediaSource`] as the backend's `MediaSource`.
///
/// `Sync` is supplied by the `Mutex`; the port contract guarantees `Send`
/// only, so a shared reference to the inner source is impossible without
/// the lock.
pub(crate) struct SourceAdapter {
    inner: Mutex<Box<dyn MediaSource>>,
}

impl SourceAdapter {
    /// Adapts an owned media-io source for the backend probe.
    pub(crate) fn new(inner: Box<dyn MediaSource>) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }
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

impl Write for SourceAdapter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("media source is not writable"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("media source is not writable"))
    }
}

impl BackendSource for SourceAdapter {
    fn is_seekable(&self) -> bool {
        // The port contract requires `Seek`, so the capability is
        // reported as seekable; a non-seekable implementation surfaces
        // `ErrorKind::Io` through the seek call itself.
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.inner.lock().ok()?.byte_len()
    }
}
