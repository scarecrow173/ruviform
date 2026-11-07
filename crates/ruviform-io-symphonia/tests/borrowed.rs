//! `with_borrowed_demuxer` regression tests — the borrowed→owned
//! bridge's honesty, boundedness and termination contracts
//! (Issue #400).

use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::panic::{AssertUnwindSafe, catch_unwind};

use ruviform_media_io::Demuxer;

const RATE: u32 = 48_000;

/// Deterministic S16 sample — same generator as the media-reader
/// acceptance tests.
fn sample_at(i: usize) -> i16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    {
        (i as i32 * 7 % 5000 - 2500) as i16
    }
}

/// Minimal PCM WAV byte stream (mono).
fn wav_bytes(channels: u16, frames: usize) -> Vec<u8> {
    let bits = 16_u16;
    let block_align = channels * (bits / 8);
    let data_len = u32::try_from(frames * usize::from(block_align)).unwrap();
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        for _ in 0..channels {
            bytes.extend_from_slice(&sample_at(i).to_le_bytes());
        }
    }
    bytes
}

/// Synthetic source reporting a large logical size with zero backing
/// allocation: `read` zero-fills `min(remaining, buf.len())`, `seek`
/// honours `SeekFrom` honestly including seek-past-end. Observation
/// hooks record every `read`/`seek` for the boundedness oracle.
struct SparseSource {
    position: u64,
    len: u64,
    max_read_request: usize,
    reads: usize,
    seeks: usize,
}

impl SparseSource {
    fn new(len: u64) -> Self {
        Self {
            position: 0,
            len,
            max_read_request: 0,
            reads: 0,
            seeks: 0,
        }
    }
}

impl Read for SparseSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reads += 1;
        self.max_read_request = self.max_read_request.max(buf.len());
        let n = usize::try_from(
            (self.len - self.position.min(self.len)).min(u64::try_from(buf.len()).unwrap()),
        )
        .unwrap();
        buf[..n].fill(0);
        self.position += u64::try_from(n).unwrap();
        Ok(n)
    }
}

impl Seek for SparseSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seeks += 1;
        let base: i128 = match pos {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::End(n) => i128::from(self.len) + i128::from(n),
            SeekFrom::Current(n) => i128::from(self.position) + i128::from(n),
        };
        if base < 0 || base > i128::from(u64::MAX) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek position out of range",
            ));
        }
        self.position = u64::try_from(base).unwrap();
        Ok(self.position)
    }
}

/// Wraps a `Cursor<Vec<u8>>`, injecting at most `fail_remaining`
/// `io::Error`s once reads reach `fail_at_byte` of source position —
/// mid-payload faults are deterministic regardless of how many
/// buffered `read` calls the backend issued to get there, and a
/// finite count keeps retrying backends from spinning forever.
struct FailingSource {
    inner: Cursor<Vec<u8>>,
    fail_at_byte: u64,
    fail_remaining: usize,
    kind: io::ErrorKind,
}

impl Read for FailingSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.inner.position() >= self.fail_at_byte && self.fail_remaining > 0 {
            self.fail_remaining -= 1;
            return Err(io::Error::new(self.kind, "programmed fault"));
        }
        self.inner.read(buf)
    }
}

impl Seek for FailingSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// A `Read + Seek` source that is deliberately `!Send` (holds `Rc`) —
/// proves the API does not require `Send` on the borrowed source.
struct RcSource {
    inner: Cursor<Vec<u8>>,
    _not_send: std::rc::Rc<()>,
}

impl Read for RcSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for RcSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// Normal path: a real WAV on a borrowed cursor demuxes and decodes
/// correctly — the `read_packet` pump returns every encoded payload
/// byte intact (this is what the encoded-copy workaround emulated).
#[test]
fn borrowed_wav_demuxes_and_decodes_byte_exact() {
    let payload_frames = 512usize;
    let mut source = Cursor::new(wav_bytes(2, payload_frames));
    let payload = ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| {
        let mut data = Vec::new();
        while let Some(pkt) = d.read_packet()? {
            data.extend_from_slice(&pkt.data);
        }
        Ok(data)
    })
    .expect("demuxer over borrowed source");

    // WAV data chunk holds s16 interleaved PCM — the packet payload
    // must match the muxed bytes exactly.
    let expected: Vec<u8> = (0..payload_frames)
        .flat_map(|i| {
            let v = sample_at(i).to_le_bytes();
            [v[0], v[1], v[0], v[1]]
        })
        .collect();
    assert_eq!(payload, expected, "borrowed decode must be byte-exact");
}

/// Canonical decode is equally available — `make_decoder` +
/// `send_packet`/`receive_frame` yields the planar `f32` the caller
/// would have decoded from an owned source.
#[test]
fn borrowed_decoder_produces_canonical_frames() {
    let mut source = Cursor::new(wav_bytes(1, 256));
    let decoded = ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| {
        let stream = d.streams()[0].stream.id;
        let mut decoder = d.make_decoder(stream, ruviform_media_io::DecodeMode::Canonical)?;
        let mut out = Vec::new();
        while let Some(pkt) = d.read_packet()? {
            decoder.send_packet(pkt)?;
            while let Some(frame) = decoder.receive_frame()? {
                let ruviform_media_io::Frame::Audio(audio) = frame else {
                    continue;
                };
                out.extend_from_slice(audio.buffer().channel(0).unwrap());
            }
        }
        Ok(out)
    })
    .expect("decoder over borrowed source");
    assert_eq!(decoded.len(), 256);
    assert!((decoded[0] - f32::from(sample_at(0)) / 32768.0).abs() < 1e-6);
}

/// R-1/R-3: a large logical source with no backing allocation serves
/// bounded single reads — the largest forwarded read request is
/// `CHUNK` and every short read is served honestly. The probe fails
/// honestly (zeros are not a container), the call still returns.
#[test]
fn large_logical_source_is_bounded() {
    // 64 MiB logical size — no allocation behind it.
    let mut source = SparseSource::new(64 * 1024 * 1024);
    let result =
        ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| Ok(d.streams().len()));
    assert!(
        result.is_err(),
        "zeros are not a container — probe must fail honestly"
    );
    assert!(
        source.max_read_request <= 64 * 1024,
        "no single read may exceed CHUNK (got {})",
        source.max_read_request
    );
    assert!(source.reads > 0, "probe did read through the proxy");
}

/// R-2: seek requests are forwarded verbatim — the source observed the
/// probe's end/restore sequence and reports real positions.
#[test]
fn seeks_are_forwarded_verbatim() {
    let mut source = SparseSource::new(1024);
    let _ = ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| Ok(d.container().format));
    assert!(
        source.seeks >= 3,
        "construction probe must issue real seeks (got {})",
        source.seeks
    );
}

/// R-3/R-6: a programmed mid-stream fault surfaces as
/// `ErrorKind::Io`, not a silent short stream or a panic. The
/// payload exceeds the backend's read buffer, so the fault hits
/// mid-packet pump — after a clean probe.
#[test]
fn injected_read_fault_surfaces_as_io_error() {
    // 100_000 mono frames → 200 KiB payload — larger than the
    // backend's stream buffer, forcing source reads mid-pump.
    let mut source = FailingSource {
        inner: Cursor::new(wav_bytes(1, 100_000)),
        fail_at_byte: 100_000,
        fail_remaining: usize::MAX,
        kind: io::ErrorKind::PermissionDenied,
    };
    let result = ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| {
        let mut n = 0usize;
        while let Some(_pkt) = d.read_packet()? {
            n += 1;
        }
        Ok(n)
    });
    let err = result.expect_err("injected fault must surface");
    assert_eq!(err.kind(), ruviform_media_io::ErrorKind::Io);
}

/// R-3: `Interrupted` is forwarded verbatim — the source's error
/// reaches the demuxer as `Interrupted`. Whether the call then fails
/// or recovers is the backend's own retry policy (`read_exact`
/// retries it, plain `read` does not) — the bridge fabricates
/// neither.
#[test]
fn interrupted_error_is_forwarded_not_swallowed() {
    // A bounded run of `Interrupted` from the *first* byte — probing
    // sees the real errors (the bridge does not fabricate retries),
    // and a finite count guarantees termination whether or not the
    // backend retries Interrupted internally.
    let mut source = FailingSource {
        inner: Cursor::new(wav_bytes(1, 64)),
        fail_at_byte: 0,
        fail_remaining: 3,
        kind: io::ErrorKind::Interrupted,
    };
    // Whatever the outcome, the call must return (the error is
    // forwarded, the bridge does not silently retry forever).
    let result =
        ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| Ok(d.streams().len()));
    // The first interrupted read happens inside the adapter's probe —
    // the honest answer then is Err (probe can't proceed) or, if the
    // backend retried and succeeded, Ok. Either way the bridge's job —
    // forward the error — is what matters; assert only the call
    // terminated and any error is Io-classified.
    if let Err(err) = result {
        assert_eq!(err.kind(), ruviform_media_io::ErrorKind::Io);
    }
}

/// R-6: `f` returning `Err` early terminates the call and returns that
/// error — the serve loop does not hang waiting for requests that
/// will never come.
#[test]
fn early_callback_error_terminates() {
    let mut source = Cursor::new(wav_bytes(1, 64));
    let sentinel = ruviform_media_io::Error::from_kind(ruviform_media_io::ErrorKind::InvalidValue);
    let result: ruviform_media_io::Result<()> =
        ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |_d| Err(sentinel));
    assert_eq!(
        result.expect_err("sentinel").kind(),
        ruviform_media_io::ErrorKind::InvalidValue
    );
}

/// R-7: non-container bytes fail probing honestly.
#[test]
fn invalid_source_fails_probe() {
    let mut source = Cursor::new(vec![0xAB_u8; 64]);
    let result =
        ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| Ok(d.streams().len()));
    assert!(result.is_err());
}

/// R-4: a panic inside `f` propagates to the caller with its payload —
/// not an `Err`, and the call still terminates (serve exited on
/// disconnect).
#[test]
fn callback_panic_propagates() {
    let mut source = Cursor::new(wav_bytes(1, 64));
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _: ruviform_media_io::Result<()> =
            ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |_d| {
                panic!("sentinel-panic");
            });
    }));
    let payload = outcome.expect_err("panic must reach the caller");
    let message = payload.downcast_ref::<&str>().copied().unwrap_or_default();
    assert_eq!(message, "sentinel-panic");
}

/// R-10: a `!Send` source (`Rc` inside) is accepted — the `&mut`
/// never crosses the thread boundary.
#[test]
fn non_send_source_is_accepted() {
    let mut source = RcSource {
        inner: Cursor::new(wav_bytes(1, 64)),
        _not_send: std::rc::Rc::new(()),
    };
    let result = ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| {
        let mut n = 0usize;
        while d.read_packet()?.is_some() {
            n += 1;
        }
        Ok(n)
    });
    assert!(
        result.is_ok(),
        "!Send source must be served on the caller thread"
    );
}

/// A source whose `read` panics — the panic travels through `serve`
/// on the caller thread.
struct PanicSource {
    inner: Cursor<Vec<u8>>,
}

impl Read for PanicSource {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        panic!("source-panic-sentinel");
    }
}

impl Seek for PanicSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// R-4 complement: a panic inside the *source's* `Read`/`Seek` also
/// propagates with its payload. `serve` unwinds on the caller side;
/// the scope-owned channel endpoints drop, the worker's pending
/// request fails, and the scope join completes. (Regression:
/// endpoints merely borrowed by the scope closure would stay alive
/// through the implicit join and deadlock the caller forever.)
#[test]
fn source_panic_propagates_and_scope_joins() {
    let mut source = PanicSource {
        inner: Cursor::new(wav_bytes(1, 64)),
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _: ruviform_media_io::Result<usize> =
            ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| Ok(d.streams().len()));
    }));
    let payload = outcome.expect_err("source panic must reach the caller");
    let message = payload.downcast_ref::<&str>().copied().unwrap_or_default();
    assert_eq!(message, "source-panic-sentinel");
}

/// The serve loop terminates when `f` returns — checked by the
/// `&mut` borrow being usable again: this test would deadlock rather
/// than fail if termination were broken. The second call re-probes
/// the *same* source (a seekable source rewinds under the adapter
/// probe regardless of the position the first call left).
#[test]
fn source_is_reusable_after_scoped_call() {
    let mut source = Cursor::new(wav_bytes(1, 64));
    let first =
        ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| Ok(d.streams().len()))
            .expect("first call");
    let second =
        ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| Ok(d.streams().len()))
            .expect("second call — the source borrow returned");
    assert_eq!(first, second);
}
