//! Shared test fixtures for the compression contract tests.

#![cfg(any(feature = "gzip", feature = "deflate", feature = "zstd"))]
#![allow(dead_code)]

use std::io::{self, BufRead, Read, Write};

use ruviform_io_compression::{CompressionError, CompressionErrorKind};

/// Extracts the normalized kind from an `io::Error` produced by our
/// `Read`/`Write` impls. `None` means the error is a raw I/O error
/// carrying no `CompressionError` payload.
pub fn compression_kind(error: &io::Error) -> Option<CompressionErrorKind> {
    error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<CompressionError>())
        .map(CompressionError::kind)
}

/// Asserts an `io::Error` carries the expected `CompressionErrorKind`.
pub fn assert_io_kind(error: &io::Error, expected: CompressionErrorKind) {
    assert_eq!(
        compression_kind(error),
        Some(expected),
        "expected {expected:?}, got {error:?}"
    );
}

/// Asserts a `Result<T, CompressionError>` carries the expected kind.
pub fn assert_kind<T>(result: Result<T, CompressionError>, expected: CompressionErrorKind) {
    match result {
        Ok(_) => panic!("expected {expected:?}, got Ok"),
        Err(error) => assert_eq!(error.kind(), expected, "{error:?}"),
    }
}

/// `BufRead` that caps `fill_buf` to `step` bytes per call, simulating
/// a source that never hands the decoder a large lookahead window.
/// Pinned by `decoder_contract::short_fill_bufread_roundtrip` — a
/// 1-byte `fill_buf` must not corrupt member-boundary detection.
pub struct ShortFill<R> {
    inner: R,
    step: usize,
    buffered: Vec<u8>,
    pos: usize,
}

impl<R: Read> ShortFill<R> {
    pub fn new(inner: R, step: usize) -> Self {
        Self {
            inner,
            step: step.max(1),
            buffered: Vec::new(),
            pos: 0,
        }
    }
}

impl<R: Read> Read for ShortFill<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let avail = self.fill_buf()?;
        let n = avail.len().min(buf.len());
        buf[..n].copy_from_slice(&avail[..n]);
        self.consume(n);
        Ok(n)
    }
}

impl<R: Read> BufRead for ShortFill<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos >= self.buffered.len() {
            self.buffered.clear();
            self.pos = 0;
            self.buffered.resize(self.step, 0);
            let n = self.inner.read(&mut self.buffered)?;
            self.buffered.truncate(n);
        }
        Ok(&self.buffered[self.pos..])
    }

    fn consume(&mut self, amt: usize) {
        self.pos += amt;
    }
}

/// `Write` that fails after `budget` successful bytes, for encoder
/// failure-path tests. Accepted bytes are kept in `inner` so tests can
/// prove what the caller retains after a terminal `Failed` state.
pub struct FailAfter {
    remaining: usize,
    pub attempts: usize,
    pub inner: Vec<u8>,
}

impl FailAfter {
    pub fn new(budget: usize) -> Self {
        Self {
            remaining: budget,
            attempts: 0,
            inner: Vec::new(),
        }
    }
}

impl Write for FailAfter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.attempts += 1;
        if self.remaining == 0 {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "injected failure",
            ));
        }
        let n = buf.len().min(self.remaining);
        self.remaining -= n;
        self.inner.extend_from_slice(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// `Write` that accepts every byte into `inner` but always fails
/// `flush` — drives the finish-time underlying-flush failure path
/// (`finish` ends with the caller's `flush`).
#[derive(Default)]
pub struct FailFlush {
    pub inner: Vec<u8>,
}

impl Write for FailFlush {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "injected flush failure",
        ))
    }
}

/// `Write` that fails its first `fail_calls` `write` calls, then
/// accepts everything into `inner` — models a transient downstream
/// failure so tests can prove the caller regains a usable writer
/// after the encoder's terminal `Failed` state.
pub struct RecoveringWriter {
    ok_calls: usize,
    fail_calls: usize,
    calls: usize,
    pub inner: Vec<u8>,
}

impl RecoveringWriter {
    /// Fail the first `fail_calls` `write` calls.
    pub fn new(fail_calls: usize) -> Self {
        Self::after_ok(0, fail_calls)
    }

    /// Succeed the first `ok_calls` writes, then fail `fail_calls`
    /// writes, then accept everything — models a writer that recovers
    /// after the encoder's terminal `Failed` state.
    pub fn after_ok(ok_calls: usize, fail_calls: usize) -> Self {
        Self {
            ok_calls,
            fail_calls,
            calls: 0,
            inner: Vec::new(),
        }
    }
}

impl Write for RecoveringWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        let failing = self.calls > self.ok_calls && self.calls <= self.ok_calls + self.fail_calls;
        if failing {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "injected failure",
            ));
        }
        self.inner.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// `BufRead` over a byte slice that starts failing once `fail_at`
/// bytes have been handed out — models a mid-stream source error for
/// decoder `Io` propagation tests. `fail_at` must be `< data.len()`.
pub struct FailingReader<'a> {
    data: &'a [u8],
    pos: usize,
    fail_at: usize,
}

impl<'a> FailingReader<'a> {
    pub fn new(data: &'a [u8], fail_at: usize) -> Self {
        Self {
            data,
            pos: 0,
            fail_at,
        }
    }
}

impl Read for FailingReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let avail = self.fill_buf()?;
        let n = avail.len().min(buf.len());
        buf[..n].copy_from_slice(&avail[..n]);
        self.consume(n);
        Ok(n)
    }
}

impl BufRead for FailingReader<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos >= self.fail_at {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "injected read failure",
            ));
        }
        let end = self.data.len().min(self.fail_at);
        Ok(&self.data[self.pos..end])
    }

    fn consume(&mut self, amt: usize) {
        self.pos += amt;
    }
}
