//! Input-side I/O port for the media-I/O boundary: [`MediaSource`].
//!
//! [`MediaSource`] is the common byte-source contract a backend demuxer
//! is built on — a `Send`-able [`std::io::Read`] + [`std::io::Seek`]
//! object. The trait is deliberately minimal: the media-I/O boundary
//! owns the packet/frame model, not the storage backend, so files,
//! in-memory cursors and custom sources all satisfy the same port.
//! `Read`/`Seek` failures surface through the normalized
//! [`ErrorKind::Io`](crate::ErrorKind) classification.
//!
//! A `Box<dyn MediaSource>` is `Send` (the trait has `Send` as a
//! supertrait), so a source may be handed to a backend living on the
//! I/O/decode thread (design baseline, thread-model section).
//!
//! Seekability is intentionally not a separate capability flag: the
//! port requires [`Seek`], and a backend discovers at probe or seek
//! time whether the implementation actually honours it (an unseekable
//! implementation surfaces its seek failure through
//! [`ErrorKind::Unsupported`](crate::ErrorKind), per the
//! [`Demuxer::seek`](crate::Demuxer::seek) contract).

use std::io::{Read, Seek};

/// Byte-source port for demux-side input: `Read` + `Seek` + `Send`.
///
/// Implemented automatically for every `T: Read + Seek + Send` —
/// `std::io::Cursor<Vec<u8>>`, `std::fs::File` and
/// `Box<dyn MediaSource>` all satisfy the bound without an explicit
/// impl.
///
/// `Send` is part of the contract: backend I/O objects are owned by the
/// I/O/decode thread and cross the thread boundary from the thread that
/// opened the source.
///
/// **RT contract:** non-RT only — `Read`/`Seek` may block on I/O.
///
/// # Examples
///
/// ```
/// use ruviform_media_io::MediaSource;
/// use std::io::Cursor;
///
/// // `Cursor` implements `Read` + `Seek` + `Send`, so it is a
/// // `MediaSource` through the blanket impl.
/// fn takes_source(source: impl MediaSource) {}
///
/// takes_source(Cursor::new(vec![0_u8; 8]));
///
/// // Object form works too — `Box<dyn MediaSource>` is `Send`.
/// let boxed: Box<dyn MediaSource> = Box::new(Cursor::new(vec![1_u8; 4]));
/// takes_source(boxed);
/// ```
pub trait MediaSource: Read + Seek + Send {}

impl<T: Read + Seek + Send> MediaSource for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn check_send<T: Send>() {}

    #[test]
    fn cursor_is_a_media_source() {
        fn accepts<S: MediaSource>(_source: S) {}
        accepts(Cursor::new(Vec::<u8>::new()));
        // `Box<dyn MediaSource>` itself satisfies the bound — a backend
        // can take `impl MediaSource` and callers pass a boxed object.
        accepts(Box::new(Cursor::new(Vec::<u8>::new())) as Box<dyn MediaSource>);
    }

    #[test]
    fn boxed_source_crosses_threads() {
        check_send::<Box<dyn MediaSource>>();
    }
}
