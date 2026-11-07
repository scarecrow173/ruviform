//! Output-side I/O port for the media-I/O boundary: [`MediaSink`].
//!
//! [`MediaSink`] is the common byte-sink contract a backend muxer is
//! built on — a `Send`-able [`std::io::Write`] + [`std::io::Seek`]
//! object, the mirror image of [`MediaSource`](crate::MediaSource).
//! The trait is deliberately minimal: the media-I/O boundary owns the
//! packet/frame model, not the storage backend, so files, in-memory
//! cursors and custom sinks all satisfy the same port.
//! `Write`/`Seek` failures surface through the normalized
//! [`ErrorKind::Io`](crate::ErrorKind) classification.
//!
//! Seekability is required because container finalization backpatches
//! sizes the writer could not know up front (RIFF chunk lengths). A
//! sink that cannot honour a seek surfaces the failure as
//! [`ErrorKind::Io`](crate::ErrorKind) at finalize time — there is no
//! separate capability flag.
//!
//! Durability is intentionally *not* part of this port: `Write::flush`
//! reaches only userspace buffers. The checked sync + atomic publish +
//! recovery contract lives in `ruviform-io-file`
//! (`StagedFile`/`SealedFile`/`PublishFailure`), which owns the staged
//! file through the whole write → finalize → publish sequence.
//!
//! A `Box<dyn MediaSink>` is `Send` (the trait has `Send` as a
//! supertrait), so a sink may be handed to a backend living on the
//! I/O thread (design baseline, thread-model section).

use std::io::{Seek, Write};

/// Byte-sink port for mux-side output: `Write` + `Seek` + `Send`.
///
/// Implemented automatically for every `T: Write + Seek + Send` —
/// `std::fs::File`, `&mut File`, `std::io::Cursor<Vec<u8>>` and
/// `Box<dyn MediaSink>` all satisfy the bound without an explicit
/// impl.
///
/// `Send` is part of the contract: backend I/O objects are owned by the
/// I/O thread and cross the thread boundary from the thread that opened
/// the sink.
///
/// **RT contract:** non-RT only — `Write`/`Seek` may block on I/O.
///
/// # Examples
///
/// ```
/// use ruviform_media_io::MediaSink;
/// use std::io::Cursor;
///
/// // `Cursor` implements `Write` + `Seek` + `Send`, so it is a
/// // `MediaSink` through the blanket impl.
/// fn takes_sink(sink: impl MediaSink) {}
///
/// takes_sink(Cursor::new(Vec::<u8>::new()));
///
/// // Object form works too — `Box<dyn MediaSink>` is `Send`.
/// let boxed: Box<dyn MediaSink> = Box::new(Cursor::new(vec![1_u8; 4]));
/// takes_sink(boxed);
/// ```
pub trait MediaSink: Write + Seek + Send {}

impl<T: Write + Seek + Send> MediaSink for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn check_send<T: Send>() {}

    #[test]
    fn cursor_is_a_media_sink() {
        fn accepts<S: MediaSink>(_sink: S) {}
        accepts(Cursor::new(Vec::<u8>::new()));
        // `Box<dyn MediaSink>` itself satisfies the bound — a backend
        // can take `impl MediaSink` and callers pass a boxed object.
        accepts(Box::new(Cursor::new(Vec::<u8>::new())) as Box<dyn MediaSink>);
    }

    #[test]
    fn boxed_sink_crosses_threads() {
        check_send::<Box<dyn MediaSink>>();
    }
}
