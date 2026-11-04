//! Shared test fixtures for the compression contract tests.

#![cfg(any(feature = "gzip", feature = "deflate"))]
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
/// failure-path tests.
pub struct FailAfter {
    remaining: usize,
    pub attempts: usize,
}

impl FailAfter {
    pub fn new(budget: usize) -> Self {
        Self {
            remaining: budget,
            attempts: 0,
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
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
