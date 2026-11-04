//! Private shared machinery for the format modules.
//!
//! `flate2`'s low-level `Compress`/`Decompress` provide the raw DEFLATE
//! body engine. All format framing (gzip header/trailer), byte-exact
//! consumption accounting, state machines and limit enforcement live in
//! this module or the format modules — no `flate2` type escapes to the
//! public API.

use std::io::{self, BufRead, Write};

use flate2::{Compress, Compression, Decompress, FlushCompress, FlushDecompress, Status};

use crate::error::{CompressionError, CompressionErrorKind};
use crate::options::EncodeLimits;
use crate::receipt::EncodeReceipt;

/// Output scratch size for the encode pump (bounded heap buffer).
const ENCODE_CHUNK: usize = 32 * 1024;

/// Maps a backend compression/decompression error to `CorruptData`.
fn backend_error(error: impl std::error::Error + Send + Sync + 'static) -> CompressionError {
    CompressionError::with_source(CompressionErrorKind::CorruptData, error)
}

/// `io::Error` for operations attempted in a terminal encoder/decoder
/// state, keeping the [`CompressionError`] reachable via `source()`.
pub(crate) fn invalid_state_io() -> io::Error {
    CompressionError::from_kind(CompressionErrorKind::InvalidState).into_io_error()
}

/// `u64` -> `usize` for backend byte deltas. Real deltas are bounded
/// by the slice lengths involved; saturating (instead of truncating)
/// keeps an absurd backend value from silently wrapping.
fn to_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// Reads exactly `buf.len()` bytes from `reader`, adding them to
/// `consumed` and enforcing the `compressed` byte cap. Short input
/// maps to [`CompressionErrorKind::TruncatedInput`].
#[cfg(feature = "gzip")]
pub(crate) fn take_exact<R: BufRead + ?Sized>(
    reader: &mut R,
    buf: &mut [u8],
    consumed: &mut u64,
    limit: u64,
) -> Result<(), CompressionError> {
    reader.read_exact(buf).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            CompressionError::from_kind(CompressionErrorKind::TruncatedInput)
        } else {
            CompressionError::io(error)
        }
    })?;
    *consumed += buf.len() as u64;
    if *consumed > limit {
        return Err(CompressionError::from_kind(
            CompressionErrorKind::LimitExceeded,
        ));
    }
    Ok(())
}

/// Limit-checked physical write: never emits bytes that would cross
/// `max` and tracks `written` as physical bytes delivered to `writer`.
fn checked_emit<W: Write + ?Sized>(
    writer: &mut W,
    written: &mut u64,
    max: u64,
    chunk: &[u8],
) -> Result<(), CompressionError> {
    if written.saturating_add(chunk.len() as u64) > max {
        return Err(CompressionError::from_kind(
            CompressionErrorKind::LimitExceeded,
        ));
    }
    writer.write_all(chunk).map_err(CompressionError::io)?;
    *written += chunk.len() as u64;
    Ok(())
}

/// Deflate-body encode pump. Owns the backend compressor and a bounded
/// output scratch; produced bytes are handed to a caller-supplied sink
/// so callers can count and bound physical writes.
pub(crate) struct DeflateOut {
    compress: Compress,
    out: Vec<u8>,
}

impl DeflateOut {
    /// Creates a raw-deflate (no zlib wrapper) compressor at `level`
    /// (`0..=9`; rejects out-of-range with `InvalidInput`).
    pub(crate) fn new(level: u32) -> Result<Self, CompressionError> {
        if level > 9 {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::InvalidInput,
            ));
        }
        Ok(Self {
            compress: Compress::new(Compression::new(level), false),
            out: vec![0; ENCODE_CHUNK],
        })
    }

    /// Compresses `input` with `FlushCompress::None`, emitting every
    /// produced chunk to `emit`. Consumes the whole input or fails.
    pub(crate) fn feed(
        &mut self,
        input: &[u8],
        emit: &mut dyn FnMut(&[u8]) -> Result<(), CompressionError>,
    ) -> Result<(), CompressionError> {
        let mut consumed = 0;
        while consumed < input.len() {
            let (in0, out0) = (self.compress.total_in(), self.compress.total_out());
            self.compress
                .compress(&input[consumed..], &mut self.out, FlushCompress::None)
                .map_err(backend_error)?;
            let used = to_usize(self.compress.total_in() - in0);
            let made = to_usize(self.compress.total_out() - out0);
            emit(&self.out[..made])?;
            consumed += used;
            if used == 0 && made == 0 {
                // No forward progress with non-empty input.
                return Err(CompressionError::from_kind(
                    CompressionErrorKind::CorruptData,
                ));
            }
        }
        Ok(())
    }

    /// Emits a deflate sync flush (`00 00 ff ff` block boundary) —
    /// pushes pending bits downstream without ending the stream.
    ///
    /// Every `Sync` call appends at least an empty stored block, so
    /// "call until nothing is produced" never terminates. The loop
    /// stops once a call does not fill the scratch — pending output is
    /// then drained (a fresh marker is ~5 bytes, far below the
    /// scratch size).
    pub(crate) fn flush_sync(
        &mut self,
        emit: &mut dyn FnMut(&[u8]) -> Result<(), CompressionError>,
    ) -> Result<(), CompressionError> {
        loop {
            let out0 = self.compress.total_out();
            self.compress
                .compress(&[], &mut self.out, FlushCompress::Sync)
                .map_err(backend_error)?;
            let made = to_usize(self.compress.total_out() - out0);
            emit(&self.out[..made])?;
            if made < self.out.len() {
                return Ok(());
            }
        }
    }

    /// Drives the compressor to `StreamEnd` (deflate final block).
    pub(crate) fn finish(
        &mut self,
        emit: &mut dyn FnMut(&[u8]) -> Result<(), CompressionError>,
    ) -> Result<(), CompressionError> {
        loop {
            let out0 = self.compress.total_out();
            let status = self
                .compress
                .compress(&[], &mut self.out, FlushCompress::Finish)
                .map_err(backend_error)?;
            let made = to_usize(self.compress.total_out() - out0);
            emit(&self.out[..made])?;
            match status {
                Status::StreamEnd => return Ok(()),
                _ if made > 0 => {}
                _ => {
                    return Err(CompressionError::from_kind(
                        CompressionErrorKind::CorruptData,
                    ));
                }
            }
        }
    }
}

/// Outcome of a single decompress step from [`DeflateIn::pump`].
pub(crate) struct PumpStep {
    /// Bytes consumed from the `BufRead` (for compressed counters).
    pub(crate) consumed: usize,
    /// Bytes produced into the output slice.
    pub(crate) produced: usize,
    /// The deflate stream reached its end (byte-aligned; following
    /// bytes belong to the caller/next unit).
    pub(crate) end: bool,
    /// `fill_buf` returned empty — the caller decides truncated-vs-end.
    pub(crate) starved: bool,
}

/// Deflate-body decode pump over a borrowed `BufRead`. Consumption is
/// byte-exact: only the `total_in` delta is `consume()`d, so bytes
/// after a unit's end stay logically unread for the caller.
pub(crate) struct DeflateIn {
    decompress: Decompress,
    ended: bool,
}

impl DeflateIn {
    /// Creates a raw-deflate (no zlib wrapper) decompressor.
    pub(crate) fn new() -> Self {
        Self {
            decompress: Decompress::new(false),
            ended: false,
        }
    }

    /// Resets for a subsequent unit (concatenated gzip members).
    #[cfg(feature = "gzip")]
    pub(crate) fn reset(&mut self) {
        self.decompress.reset(false);
        self.ended = false;
    }

    /// Pulls input from `reader` and decompresses into `out`.
    ///
    /// `Err(CorruptData)` on backend errors; `step.end` marks the
    /// deflate-stream end; `step.starved` reports an empty `fill_buf`
    /// without consuming state.
    pub(crate) fn pump<R: BufRead + ?Sized>(
        &mut self,
        reader: &mut R,
        out: &mut [u8],
    ) -> Result<PumpStep, CompressionError> {
        if self.ended {
            return Ok(PumpStep {
                consumed: 0,
                produced: 0,
                end: true,
                starved: false,
            });
        }
        // The decompressor may hold buffered output even after all
        // input was consumed — it must be offered output space before
        // an empty `fill_buf` can be reported as starvation.
        let input = reader.fill_buf().map_err(CompressionError::io)?;
        let starved = input.is_empty();
        let (in0, out0) = (self.decompress.total_in(), self.decompress.total_out());
        let status = self
            .decompress
            .decompress(input, out, FlushDecompress::None)
            .map_err(backend_error)?;
        let consumed = to_usize(self.decompress.total_in() - in0);
        let produced = to_usize(self.decompress.total_out() - out0);
        reader.consume(consumed);
        if status == Status::StreamEnd {
            self.ended = true;
        }
        if consumed == 0 && produced == 0 && !self.ended && !starved {
            // Non-empty input and output space yet no progress.
            return Err(CompressionError::from_kind(
                CompressionErrorKind::CorruptData,
            ));
        }
        Ok(PumpStep {
            consumed,
            produced,
            end: self.ended,
            // Starved = the source is empty *and* the decompressor had
            // nothing pending to emit.
            starved: starved && produced == 0 && !self.ended,
        })
    }
}

/// Encoder lifecycle — `Writing -> Finishing -> Finished | Failed`.
#[derive(Debug, Clone)]
pub(crate) enum EncState {
    Writing,
    Finishing,
    Finished(EncodeReceipt),
    Failed,
}

/// Format-agnostic encoder core: owns the borrowed writer, the deflate
/// pump, the state machine and both budgets. Format modules wrap it
/// with their own header/trailer emission.
pub(crate) struct EncoderCore<'w, W: Write + ?Sized> {
    writer: &'w mut W,
    engine: DeflateOut,
    state: EncState,
    limits: EncodeLimits,
    /// Physical bytes written to `writer` (framing included).
    written: u64,
    /// Uncompressed bytes accepted through `write`.
    input_bytes: u64,
}

impl<'w, W: Write + ?Sized> EncoderCore<'w, W> {
    /// Builds the core; `level`/`limits` come from the format options.
    pub(crate) fn new(
        writer: &'w mut W,
        level: u32,
        limits: EncodeLimits,
    ) -> Result<Self, CompressionError> {
        Ok(Self {
            writer,
            engine: DeflateOut::new(level)?,
            state: EncState::Writing,
            limits,
            written: 0,
            input_bytes: 0,
        })
    }

    /// Limit-checked raw write for format framing (gzip header).
    #[cfg(feature = "gzip")]
    pub(crate) fn emit(&mut self, chunk: &[u8]) -> Result<(), CompressionError> {
        checked_emit(
            self.writer,
            &mut self.written,
            self.limits.max_output_bytes(),
            chunk,
        )
    }

    /// `Write::write`: accepts the whole buffer or fails. Terminal
    /// states are rejected with `InvalidState`; backend/I/O/limit
    /// failures move the encoder to `Failed`.
    pub(crate) fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.state {
            EncState::Writing => {}
            _ => return Err(invalid_state_io()),
        }
        if self.input_bytes.saturating_add(buf.len() as u64) > self.limits.max_input_bytes() {
            self.state = EncState::Failed;
            return Err(
                CompressionError::from_kind(CompressionErrorKind::LimitExceeded).into_io_error(),
            );
        }
        let max_out = self.limits.max_output_bytes();
        let Self {
            writer,
            engine,
            written,
            ..
        } = self;
        match engine.feed(buf, &mut |chunk| {
            checked_emit(*writer, written, max_out, chunk)
        }) {
            Ok(()) => {
                self.input_bytes += buf.len() as u64;
                Ok(buf.len())
            }
            Err(error) => {
                self.state = EncState::Failed;
                Err(error.into_io_error())
            }
        }
    }

    /// `Write::flush`: deflate sync flush plus underlying `flush`.
    /// A no-op `Ok(())` on `Finished` so stacked writers can flush
    /// unconditionally; `Failed`/`Finishing` report `InvalidState`.
    pub(crate) fn flush(&mut self) -> io::Result<()> {
        match self.state {
            EncState::Finished(_) => return Ok(()),
            EncState::Writing => {}
            _ => return Err(invalid_state_io()),
        }
        let max_out = self.limits.max_output_bytes();
        let Self {
            writer,
            engine,
            written,
            ..
        } = self;
        let result = engine
            .flush_sync(&mut |chunk| checked_emit(*writer, written, max_out, chunk))
            .and_then(|()| writer.flush().map_err(CompressionError::io));
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                self.state = EncState::Failed;
                Err(error.into_io_error())
            }
        }
    }

    /// `Encoder::finish`: idempotent — returns the stored receipt once
    /// `Finished`, `InvalidState` once `Failed`. `tail` carries the
    /// format trailer (gzip CRC32/ISIZE; empty for raw deflate).
    pub(crate) fn finish(&mut self, tail: &[u8]) -> Result<EncodeReceipt, CompressionError> {
        match &self.state {
            EncState::Finished(receipt) => return Ok(*receipt),
            EncState::Failed => {
                return Err(CompressionError::from_kind(
                    CompressionErrorKind::InvalidState,
                ));
            }
            _ => {}
        }
        self.state = EncState::Finishing;
        match self.finish_pipeline(tail) {
            Ok(receipt) => {
                self.state = EncState::Finished(receipt);
                Ok(receipt)
            }
            Err(error) => {
                self.state = EncState::Failed;
                Err(error)
            }
        }
    }

    /// Deflate `Finish` pump -> format trailer -> underlying `flush`.
    fn finish_pipeline(&mut self, tail: &[u8]) -> Result<EncodeReceipt, CompressionError> {
        let max_out = self.limits.max_output_bytes();
        let Self {
            writer,
            engine,
            written,
            ..
        } = self;
        engine.finish(&mut |chunk| checked_emit(*writer, written, max_out, chunk))?;
        checked_emit(*writer, written, max_out, tail)?;
        writer.flush().map_err(CompressionError::io)?;
        Ok(EncodeReceipt::new(self.input_bytes, *written, 1))
    }
}
