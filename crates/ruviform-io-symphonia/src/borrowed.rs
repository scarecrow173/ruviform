//! Borrowed-source bridge: a borrowed `Read + Seek` stream driving an
//! owned, `'static` demuxer inside a closed scope.
//!
//! [`open_demuxer`] requires `impl MediaSource + 'static` — an owned,
//! `Send` byte source — while callers such as
//! `ruviform_io_file::SealedFile::open_readback` lend a *borrowed*,
//! `!Send` `Read + Seek` whose lifetime bounds `publish`/`abort`
//! exclusivity. Copying the stream into an owned buffer just to
//! satisfy the bound costs one full encoded-size allocation.
//!
//! [`with_borrowed_demuxer`] bridges the two requirements without that
//! copy: the calling thread runs a `serve` loop that owns `&mut
//! source` (the `!Send` borrow never crosses a thread boundary) while
//! exactly one scoped worker owns a channel-backed proxy satisfying
//! `MediaSource`. Every proxy `read`/`seek` is forwarded verbatim over
//! a `sync_channel(1)` request/response pair, so in-flight encoded
//! bytes stay bounded by one chunk per direction.
//!
//! The port handed to the callback is deliberately `&mut dyn Demuxer`:
//! a concrete or `Sized` handout could be swapped out from under the
//! proxy (a fabricated demuxer via `mem::replace`) and keep the
//! request channel alive forever — a reachable deadlock. The `&mut
//! dyn` borrow cannot be boxed, replaced, or returned (the result type
//! cannot name its scope lifetime), so the serving loop always exits
//! when the worker finishes.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::mpsc::{self, Receiver, SyncSender};

use ruviform_media_io::{Demuxer, Error, Result};

use crate::open_demuxer;

/// Largest single `read` request the proxy forwards — the per-request
/// encoded-byte bound. `read` calls larger than this are split into
/// honest short reads, which `Read` permits and `MediaSourceStream`
/// handles.
const CHUNK: usize = 64 * 1024;

/// Worker thread name for diagnostics.
const WORKER_NAME: &str = "rf-borrowed-source";

/// Request the proxy sends to the calling thread's serve loop.
enum Req {
    /// Read at most `len` bytes at the source's current position.
    Read {
        /// Requested byte count (`<= CHUNK`, `> 0`).
        len: usize,
    },
    /// Seek exactly as [`Seek::seek`] specifies.
    Seek(SeekFrom),
}

/// Response the serve loop returns — the real `io::Result` from the
/// source, verbatim.
enum Resp {
    /// Read result: the bytes actually read (truncated to the real
    /// count — short read and `Ok(0)` EOF are honest) or the real error.
    Read(io::Result<Vec<u8>>),
    /// Seek result: the real new position or the real error.
    Seek(io::Result<u64>),
}

/// Error the proxy reports when the serve loop is gone — a dead
/// channel is a broken pipe, never a clean EOF: reporting `Ok(0)`
/// would let a decode look "successfully truncated" and mask the real
/// failure (lost responses are genuine I/O loss, not stream end).
fn serve_closed() -> io::Error {
    io::Error::new(
        io::ErrorKind::BrokenPipe,
        "borrowed-source serve terminated",
    )
}

/// The worker-side byte source: a channel endpoint pair, `Send +
/// 'static`, that forwards every `Read`/`Seek` call to the calling
/// thread's serve loop.
///
/// It holds the only request `SyncSender` — when the demuxer owning it
/// drops (worker closure end, `open_demuxer` failure, or unwind), the
/// request channel disconnects and the serve loop exits. The sender is
/// never cloned, which is what makes disconnect a reliable termination
/// signal.
struct ReadProxy {
    /// Requests to the serve loop.
    req: SyncSender<Req>,
    /// Responses from the serve loop.
    resp: Receiver<Resp>,
}

impl ReadProxy {
    /// Sends one request and waits for its response; the serve loop
    /// alternates strictly request→response so `resp.recv` is the only
    /// blocking point here.
    fn roundtrip(&self, req: Req) -> io::Result<Resp> {
        self.req.send(req).map_err(|_| serve_closed())?;
        self.resp.recv().map_err(|_| serve_closed())
    }
}

impl Read for ReadProxy {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let len = buf.len().min(CHUNK);
        match self.roundtrip(Req::Read { len })? {
            Resp::Read(Ok(bytes)) => {
                if bytes.len() > buf.len() {
                    // Serve-side contract violation — the source cannot
                    // legally return more than requested; report rather
                    // than truncate silently.
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "borrowed-source serve returned more bytes than requested",
                    ));
                }
                buf[..bytes.len()].copy_from_slice(&bytes);
                Ok(bytes.len())
            }
            Resp::Read(Err(err)) => Err(err),
            Resp::Seek(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "borrowed-source protocol violation: seek response to read request",
            )),
        }
    }
}

impl Seek for ReadProxy {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match self.roundtrip(Req::Seek(pos))? {
            Resp::Seek(result) => result,
            Resp::Read(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "borrowed-source protocol violation: read response to seek request",
            )),
        }
    }
}

/// Calling-thread loop: forwards every request verbatim to `src` until
/// the request channel disconnects (the proxy dropped inside the
/// worker) or the worker stopped receiving responses. Returns without
/// value — source errors travel to the worker through the response
/// channel, never through this loop.
fn serve<S: Read + Seek + ?Sized>(src: &mut S, req: &Receiver<Req>, resp: &SyncSender<Resp>) {
    while let Ok(request) = req.recv() {
        let response = match request {
            Req::Read { len } => {
                let mut buf = vec![0_u8; len];
                let result = src.read(&mut buf).map(|n| {
                    buf.truncate(n);
                    buf
                });
                Resp::Read(result)
            }
            Req::Seek(pos) => Resp::Seek(src.seek(pos)),
        };
        if resp.send(response).is_err() {
            // The worker died between request and response — nothing
            // left to serve.
            break;
        }
    }
}

/// Runs `f` with a probed [`Demuxer`] whose byte source is the
/// borrowed `source`, keeping additional encoded memory bounded.
///
/// `source` may be any `Read + Seek` stream — including `!Send`
/// borrows such as `ruviform_io_file`'s `ReadbackFile` — because it
/// stays on the calling thread: an internal worker thread owns a
/// channel-backed proxy that satisfies [`MediaSource`]'s `Send +
/// 'static` bound, and every `read`/`seek` the demuxer issues is
/// forwarded to the calling thread verbatim (short reads, `Ok(0)`
/// EOF, `Interrupted`, seek-past-end results and arbitrary `io::Error`
/// kinds all reach the demuxer exactly as the source produced them).
/// In-flight encoded bytes are bounded by one `≤64 KiB` chunk per
/// direction; the callback's own buffering decides all other memory.
///
/// `f` runs on the worker thread and receives `&mut dyn Demuxer` —
/// a borrow that cannot escape the call: boxing it yields only a
/// `Box<dyn Demuxer + '_>` still bound by the scope's lifetime, so it
/// cannot be owned, swapped out for a fabricated demuxer, or returned
/// through `R` — and the serving loop always terminates when `f` (or
/// probing) ends. Owned values produced inside `f` — `Packet`s,
/// `Frame`s, `Box<dyn Decoder>`s, decoded output — may leave through
/// `R` only if they carry no borrow of the demuxer or proxy.
///
/// **RT contract:** non-RT only — both threads perform blocking
/// channel and source I/O plus unbounded demux work; the calling
/// thread is dedicated to serving until `f` returns.
///
/// # Caveats
///
/// * `f` executes on a *different* thread: it sees worker
///   thread-locals and may capture only `Send` data.
/// * `f` must not block on the calling thread (e.g. wait on a channel
///   only the caller could drain) — that is a real deadlock.
/// * The source's final position after return is unspecified
///   (probing seeks to the end and back).
/// * Escape-freedom relies on `&mut dyn` opacity: if [`Demuxer`] ever
///   gains a downcast port or a `Box<dyn Demuxer>` factory, this
///   guarantee must be re-audited.
///
/// # Errors
///
/// Everything [`open_demuxer`] reports (probe/probe-time I/O) plus any
/// error `f` returns — the `Err` cannot be distinguished by origin,
/// matching the boundary's error model.
///
/// # Panics
///
/// Panics in `f` propagate to the caller via [`resume_unwind`]; panics
/// in the source's `Read`/`Seek` propagate directly (the worker's
/// channel calls then fail and it is joined).
///
/// [`MediaSource`]: ruviform_media_io::MediaSource
/// [`resume_unwind`]: std::panic::resume_unwind
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
/// use ruviform_media_io::Demuxer;
///
/// let bytes: Vec<u8> = b"not a container".to_vec();
/// let mut source = Cursor::new(bytes);
/// // Probing fails honestly for non-container input.
/// let result = ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| {
///     Ok(d.streams().len())
/// });
/// assert!(result.is_err());
/// ```
///
/// The borrowed demuxer cannot be smuggled out of `f`:
///
/// ```compile_fail
/// use std::io::Cursor;
/// use ruviform_media_io::{Demuxer, Result};
///
/// let mut source = Cursor::new(Vec::new());
/// let leaked: Result<Box<dyn Demuxer>> = ruviform_io_symphonia::with_borrowed_demuxer(
///     &mut source,
///     |d| Ok(Box::new(d)), // cannot name the borrow's lifetime — does not compile
/// );
/// ```
pub fn with_borrowed_demuxer<R>(
    source: &mut (impl Read + Seek + ?Sized),
    f: impl FnOnce(&mut dyn Demuxer) -> Result<R> + Send,
) -> Result<R>
where
    R: Send,
{
    let (req_tx, req_rx) = mpsc::sync_channel::<Req>(1);
    let (resp_tx, resp_rx) = mpsc::sync_channel::<Resp>(1);
    // `move` is load-bearing: the endpoints (and `source`/`f`) must be
    // owned by the closure so they drop when it unwinds — before
    // `thread::scope` implicitly joins the worker. If `serve` panics
    // (a panicking `Read`/`Seek`), dropping `req_rx`/`resp_tx` here
    // disconnects the channels, the worker's next `send`/`recv`
    // fails and it exits — the join then completes and the panic
    // propagates. Borrowed endpoints in the outer frame would stay
    // alive through the join and deadlock it.
    std::thread::scope(move |scope| {
        let worker = std::thread::Builder::new()
            .name(WORKER_NAME.to_owned())
            .spawn_scoped(scope, move || {
                let proxy = ReadProxy {
                    req: req_tx,
                    resp: resp_rx,
                };
                let mut demuxer = open_demuxer(proxy)?;
                f(&mut demuxer)
            });
        let worker = match worker {
            Ok(worker) => worker,
            Err(err) => return Err(Error::from(err)),
        };
        serve(source, &req_rx, &resp_tx);
        match worker.join() {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    })
}
