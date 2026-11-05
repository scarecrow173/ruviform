//! Raw DEFLATE (RFC 1951) stream encode/decode.
//!
//! This is the bare DEFLATE bitstream — **not** the zlib container
//! (RFC 1950) and not gzip (RFC 1952). There is no header and no
//! checksum; integrity is exactly what the format provides, which the
//! receipt records via [`IntegritySummary::unchecked_units`].
//!
//! Stream concatenation is not implicit: a second DEFLATE stream is
//! only distinguishable if the caller knows the boundary, so
//! `WholeInput` treats bytes after the stream end as `TrailingData`.
//! Use [`DecodeScope::SingleUnit`] for embedded streams where bytes
//! after the end belong to the caller.

use std::io::{self, BufRead, Write};

use crate::error::{CompressionError, CompressionErrorKind};
use crate::options::{DecodeLimits, DecodeScope, EncodeLimits};
use crate::receipt::{DecodeReceipt, EncodeReceipt, IntegritySummary};
use crate::stream::{DeflateIn, DeflateOut, EncoderCore, invalid_state_io};

/// Encode options for [`Encoder`]: DEFLATE compression level plus the
/// shared [`EncodeLimits`] byte budgets.
///
/// `level` follows DEFLATE semantics (`0` = stored, `9` = maximum,
/// default `6`). It is deliberately *not* a cross-format level type —
/// other formats define their own options.
#[derive(Debug, Clone, Copy)]
pub struct EncodeOptions {
    level: u32,
    limits: EncodeLimits,
}

impl EncodeOptions {
    /// Options with an explicit `level` and `limits`. `Encoder::new`
    /// rejects `level > 9` with `InvalidInput`.
    #[must_use]
    pub const fn new(level: u32, limits: EncodeLimits) -> Self {
        Self { level, limits }
    }

    /// DEFLATE level (`0..=9`; values outside are rejected by
    /// `Encoder::new`).
    #[must_use]
    pub const fn level(&self) -> u32 {
        self.level
    }

    /// Configured encoder budgets.
    #[must_use]
    pub const fn limits(&self) -> EncodeLimits {
        self.limits
    }
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self::new(6, EncodeLimits::new())
    }
}

/// Decode options for [`Decoder`]: the input [`DecodeScope`] plus the
/// shared [`DecodeLimits`] cumulative budgets.
#[derive(Debug, Clone, Copy)]
pub struct DecodeOptions {
    scope: DecodeScope,
    limits: DecodeLimits,
}

impl DecodeOptions {
    /// Options with an explicit `scope` and `limits`.
    #[must_use]
    pub const fn new(scope: DecodeScope, limits: DecodeLimits) -> Self {
        Self { scope, limits }
    }

    /// Input scope (`WholeInput` or `SingleUnit`).
    #[must_use]
    pub const fn scope(&self) -> DecodeScope {
        self.scope
    }

    /// Configured decoder budgets.
    #[must_use]
    pub const fn limits(&self) -> DecodeLimits {
        self.limits
    }
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self::new(DecodeScope::WholeInput, DecodeLimits::new())
    }
}

/// Raw-DEFLATE stream encoder over a borrowed `W`.
///
/// Lifecycle: `Writing -> Finishing -> Finished | Failed`. Only an
/// explicit successful [`Encoder::finish`] completes the stream
/// (writes the deflate final block); `flush` is a deflate sync flush,
/// not completion. `Drop` performs **no** writes.
///
/// **RT contract:** non-RT only — `Write` may block and allocate.
///
/// # Examples
///
/// ```
/// use ruviform_io_compression::deflate::{Encoder, EncodeOptions};
/// use std::io::Write;
///
/// let mut out = Vec::new();
/// {
///     let mut enc = Encoder::new(&mut out, EncodeOptions::default())?;
///     enc.write_all(b"hello")?;
///     enc.finish()?;
/// }
/// assert!(!out.is_empty());
/// // No zlib/gzip framing: the first byte is a DEFLATE block header.
/// assert_ne!(out[0], 0x1f);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Encoder<'w, W: Write + ?Sized> {
    core: EncoderCore<'w, W, DeflateOut>,
}

impl<'w, W: Write + ?Sized> Encoder<'w, W> {
    /// Builds an encoder. Writes nothing yet — the first bytes appear
    /// on `write`/`flush`/`finish`.
    pub fn new(writer: &'w mut W, options: EncodeOptions) -> Result<Self, CompressionError> {
        Ok(Self {
            core: EncoderCore::new(writer, DeflateOut::new(options.level())?, options.limits()),
        })
    }

    /// Completes the stream: deflate `Finish` block, then the
    /// underlying `flush`. Idempotent — a second call returns the
    /// stored [`EncodeReceipt`]. After `Failed`, reports
    /// `InvalidState`.
    pub fn finish(&mut self) -> Result<EncodeReceipt, CompressionError> {
        self.core.finish(&[])
    }
}

impl<W: Write + ?Sized> Write for Encoder<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.core.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.core.flush()
    }
}

/// Internal phases — `Body -> Boundary -> Done`. `Boundary` only runs
/// under `WholeInput`; `Done` is `VerifiedEnd`.
#[derive(Debug)]
enum Phase {
    /// Inside the deflate stream body.
    Body,
    /// Stream end reached under `WholeInput`: require input EOF.
    Boundary,
    /// Verification complete (`VerifiedEnd`).
    Done,
}

/// Raw-DEFLATE stream decoder over a borrowed `R`.
///
/// `impl Read` yields decoded bytes; `Ok(0)` on a non-empty buffer is
/// returned only once the stream end is reached *and* — under
/// `WholeInput` — the input is confirmed empty. `read(&mut [])` never
/// changes state and is not evidence of completion.
/// [`Decoder::finish`] inspects state only: `Ok` exactly in
/// `VerifiedEnd`, `NotExhausted` while output or verification may
/// remain, `InvalidState` once `Failed`.
///
/// `SingleUnit` leaves bytes after the stream end unconsumed inside the
/// caller's `BufRead`. The format carries no checksum, so the receipt
/// counts the unit as [`IntegritySummary::unchecked_units`].
///
/// **RT contract:** non-RT only — `Read` may block and allocate.
///
/// # Examples
///
/// ```
/// use ruviform_io_compression::deflate::{Decoder, DecodeOptions};
/// use std::io::{BufReader, Read};
///
/// # let mut encoded = Vec::new();
/// # {
/// #   use ruviform_io_compression::deflate::{Encoder, EncodeOptions};
/// #   use std::io::Write;
/// #   let mut enc = Encoder::new(&mut encoded, EncodeOptions::default())?;
/// #   enc.write_all(b"payload")?;
/// #   enc.finish()?;
/// # }
/// let mut source = BufReader::new(&encoded[..]);
/// let mut dec = Decoder::new(&mut source, DecodeOptions::default());
/// let mut out = String::new();
/// dec.read_to_string(&mut out)?;
/// let receipt = dec.finish()?;
/// assert_eq!(out, "payload");
/// assert_eq!(receipt.units_decoded(), 1);
/// // Raw deflate provides no checksum to verify.
/// assert_eq!(receipt.integrity().unchecked_units(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Decoder<'r, R: BufRead + ?Sized> {
    reader: &'r mut R,
    options: DecodeOptions,
    phase: Phase,
    decomp: DeflateIn,
    /// Logical bytes consumed from `reader`.
    compressed: u64,
    /// Bytes delivered to the consumer through `read`.
    delivered: u64,
    /// `0` until the single stream's end is verified, then `1`.
    units: u64,
    /// Terminal `Failed` latch.
    failed: bool,
}

impl<'r, R: BufRead + ?Sized> Decoder<'r, R> {
    /// Builds a decoder. Parsing is lazy — no bytes are consumed until
    /// the first `read`, so `new` itself is infallible.
    pub fn new(reader: &'r mut R, options: DecodeOptions) -> Self {
        Self {
            reader,
            options,
            phase: Phase::Body,
            decomp: DeflateIn::new(),
            compressed: 0,
            delivered: 0,
            units: 0,
            failed: false,
        }
    }

    /// Verifies completion. `Ok` exactly in `VerifiedEnd` — the
    /// deflate stream end observed and, under `WholeInput`, the input
    /// confirmed exhausted. Undelivered output is never discarded to
    /// reach this state; a mid-stream call reports `NotExhausted`.
    pub fn finish(&mut self) -> Result<DecodeReceipt, CompressionError> {
        if self.failed {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::InvalidState,
            ));
        }
        if !matches!(self.phase, Phase::Done) {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::NotExhausted,
            ));
        }
        Ok(DecodeReceipt::new(
            self.compressed,
            self.delivered,
            self.units,
            IntegritySummary::new(0, self.units),
        ))
    }

    /// Compressed bytes consumed so far (diagnostic; the authoritative
    /// figure is on the receipt after `finish`).
    #[must_use]
    pub const fn compressed_bytes_consumed(&self) -> u64 {
        self.compressed
    }

    /// State-machine driver: loops until output is produced, the
    /// verified end is reached (`Ok(0)`), or an error occurs.
    fn drive(&mut self, out: &mut [u8]) -> Result<usize, CompressionError> {
        let mut scratch = [0u8; 1];
        loop {
            match self.phase {
                Phase::Done => return Ok(0),
                Phase::Body => {
                    let remaining = self
                        .options
                        .limits()
                        .max_decoded_bytes()
                        .saturating_sub(self.delivered);
                    let allowed =
                        usize::try_from(remaining.min(out.len() as u64)).unwrap_or(usize::MAX);
                    let (target, probing) = if allowed == 0 {
                        (&mut scratch[..], true)
                    } else {
                        (&mut out[..allowed], false)
                    };
                    let step = self.decomp.pump(self.reader, target)?;
                    self.compressed += step.consumed as u64;
                    if self.compressed > self.options.limits().max_compressed_bytes() {
                        return Err(CompressionError::from_kind(
                            CompressionErrorKind::LimitExceeded,
                        ));
                    }
                    if step.produced > 0 {
                        if probing {
                            // At the decoded cap, a further byte proves
                            // the budget was exceeded — it is *not*
                            // delivered to the consumer.
                            return Err(CompressionError::from_kind(
                                CompressionErrorKind::LimitExceeded,
                            ));
                        }
                        self.delivered += step.produced as u64;
                        return Ok(step.produced);
                    }
                    if step.starved {
                        return Err(CompressionError::from_kind(
                            CompressionErrorKind::TruncatedInput,
                        ));
                    }
                    if step.end {
                        self.units = 1;
                        self.phase = match self.options.scope() {
                            DecodeScope::SingleUnit => Phase::Done,
                            _ => Phase::Boundary,
                        };
                    }
                }
                Phase::Boundary => {
                    let avail = self.reader.fill_buf().map_err(CompressionError::io)?;
                    if avail.is_empty() {
                        self.phase = Phase::Done;
                        return Ok(0);
                    }
                    return Err(CompressionError::from_kind(
                        CompressionErrorKind::TrailingData,
                    ));
                }
            }
        }
    }
}

impl<R: BufRead + ?Sized> io::Read for Decoder<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            // A zero-length buffer never proves or advances anything.
            return Ok(0);
        }
        if self.failed {
            return Err(invalid_state_io());
        }
        match self.drive(out) {
            Ok(produced) => Ok(produced),
            Err(error) => {
                self.failed = true;
                Err(error.into_io_error())
            }
        }
    }
}
