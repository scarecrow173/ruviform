//! Gzip (RFC 1952) member encode/decode.
//!
//! The Ruviform side owns all member framing — the 10-byte header
//! (with FEXTRA/FNAME/FCOMMENT/FHCRC fields) and the 8-byte CRC32/ISIZE
//! trailer — while the DEFLATE body uses the shared private backend
//! pump. That split is what makes member-boundary byte accounting,
//! `SingleUnit` leftovers and "Drop writes nothing" guarantees
//! backend-independent.
//!
//! Encoded headers are fixed and reproducible: `1f 8b 08 00` +
//! `mtime = 0` + `xfl` (level-derived) + `os = 0xff` — no filename,
//! no timestamp, no platform-dependent bytes.

use std::io::{self, BufRead, Write};

use flate2::Crc;

use crate::error::{CompressionError, CompressionErrorKind};
use crate::options::{DecodeLimits, DecodeScope, EncodeLimits};
use crate::receipt::{DecodeReceipt, EncodeReceipt, IntegritySummary};
use crate::stream::{DeflateIn, DeflateOut, EncoderCore, invalid_state_io, take_exact};

/// Fixed gzip header length (magic + method + flags + mtime + xfl + os).
const HEADER_LEN: usize = 10;
/// Member trailer: CRC32 (LE) + ISIZE (LE).
const TRAILER_LEN: usize = 8;
/// Defensive cap on one member's total header bytes (fixed + FEXTRA +
/// FNAME + FCOMMENT + FHCRC). RFC 1952 does not bound optional fields;
/// a header larger than this is reported `InvalidFormat`.
const MEMBER_HEADER_MAX: usize = 64 * 1024;

const FLAG_FHCRC: u8 = 0x02;
const FLAG_FEXTRA: u8 = 0x04;
const FLAG_FNAME: u8 = 0x08;
const FLAG_FCOMMENT: u8 = 0x10;
const FLAG_RESERVED: u8 = 0xE0;

const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

fn invalid_format() -> CompressionError {
    CompressionError::from_kind(CompressionErrorKind::InvalidFormat)
}

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

/// Gzip stream encoder over a borrowed `W`.
///
/// Lifecycle: `Writing -> Finishing -> Finished | Failed`. Only an
/// explicit successful [`Encoder::finish`] completes the member
/// (writes the CRC32/ISIZE trailer); `flush` is a deflate sync flush,
/// not completion. `Drop` performs **no** writes — dropping an
/// unfinished encoder leaves whatever bytes were already emitted and
/// never appends a trailer.
///
/// **RT contract:** non-RT only — `Write` may block and allocate.
///
/// # Examples
///
/// ```
/// use ruviform_io_compression::gzip::{Encoder, EncodeOptions};
/// use std::io::Write;
///
/// let mut out = Vec::new();
/// {
///     let mut enc = Encoder::new(&mut out, EncodeOptions::default())?;
///     enc.write_all(b"hello")?;
///     let receipt = enc.finish()?;
///     assert_eq!(receipt.input_bytes(), 5);
///     // `finish` is idempotent — the same receipt comes back.
///     assert_eq!(enc.finish()?, receipt);
/// } // borrow of `out` ends here
/// assert_eq!(&out[..4], &[0x1f, 0x8b, 0x08, 0x00]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Encoder<'w, W: Write + ?Sized> {
    core: EncoderCore<'w, W, DeflateOut>,
    crc: Crc,
}

impl<'w, W: Write + ?Sized> Encoder<'w, W> {
    /// Builds an encoder and immediately writes the fixed 10-byte
    /// member header to `writer` (an I/O failure here fails `new` —
    /// `writer` remains owned by the caller either way).
    pub fn new(writer: &'w mut W, options: EncodeOptions) -> Result<Self, CompressionError> {
        let mut encoder = Self {
            core: EncoderCore::new(writer, DeflateOut::new(options.level())?, options.limits()),
            crc: Crc::new(),
        };
        encoder.core.emit(&header_bytes(options.level()))?;
        Ok(encoder)
    }

    /// Completes the member: deflate `Finish`, then the CRC32/ISIZE
    /// trailer, then the underlying `flush`. Idempotent — a second
    /// call returns the stored [`EncodeReceipt`] without touching the
    /// backend. After `Failed`, reports `InvalidState`.
    pub fn finish(&mut self) -> Result<EncodeReceipt, CompressionError> {
        let trailer = self.trailer_bytes();
        self.core.finish(&trailer)
    }

    fn trailer_bytes(&self) -> [u8; TRAILER_LEN] {
        let mut trailer = [0u8; TRAILER_LEN];
        trailer[..4].copy_from_slice(&self.crc.sum().to_le_bytes());
        trailer[4..].copy_from_slice(&self.crc.amount().to_le_bytes());
        trailer
    }
}

impl<W: Write + ?Sized> Write for Encoder<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let accepted = self.core.write(buf)?;
        // Whole-or-error acceptance makes `accepted == buf.len()` on
        // success; the member CRC covers delivered input only.
        self.crc.update(&buf[..accepted]);
        Ok(accepted)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.core.flush()
    }
}

fn header_bytes(level: u32) -> [u8; HEADER_LEN] {
    // RFC 1952 XFL: 2 = maximum compression, 4 = fastest algorithm.
    let xfl = if level >= 9 {
        2
    } else if level <= 1 {
        4
    } else {
        0
    };
    [0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, xfl, 0xff]
}

/// Internal member phases — `Start -> Header -> Body -> Trailer ->
/// Between`. `Between` only runs under `WholeInput`; `Done` is
/// `VerifiedEnd`.
#[derive(Debug)]
enum Phase {
    /// Stream start: the first member's magic is not yet consumed.
    Start,
    /// Magic consumed; the rest of the member header is expected.
    Header,
    /// Inside the member's deflate body.
    Body,
    /// Reading the member's 8-byte trailer.
    Trailer,
    /// Member ended under `WholeInput`: peek for another member or EOF.
    Between,
    /// Verification complete (`VerifiedEnd`).
    Done,
}

/// Gzip stream decoder over a borrowed `R`.
///
/// `impl Read` yields decoded bytes; `Ok(0)` on a non-empty buffer is
/// returned only once the *verified* end is reached — after the last
/// member's trailer checksum and the scope's input-boundary contract
/// both hold. `read(&mut [])` never changes state and is not evidence
/// of completion. [`Decoder::finish`] inspects state only: `Ok`
/// exactly in `VerifiedEnd`, `NotExhausted` while output or
/// verification may remain, `InvalidState` once `Failed`.
///
/// `SingleUnit` leaves bytes after the first member's end unconsumed
/// inside the caller's `BufRead` (readable by the caller afterwards).
/// `compressed_bytes_consumed` counts logical consumption only — a
/// caller-side `BufReader`'s physical prefetch is not included.
///
/// **RT contract:** non-RT only — `Read` may block and allocate.
///
/// # Examples
///
/// ```
/// use ruviform_io_compression::gzip::{Decoder, DecodeOptions};
/// use std::io::{BufReader, Read};
///
/// # let mut encoded = Vec::new();
/// # {
/// #   use ruviform_io_compression::gzip::{Encoder, EncodeOptions};
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
/// assert_eq!(receipt.integrity().checked_units(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Decoder<'r, R: BufRead + ?Sized> {
    reader: &'r mut R,
    options: DecodeOptions,
    phase: Phase,
    decomp: DeflateIn,
    crc: Crc,
    /// Logical bytes consumed from `reader`.
    compressed: u64,
    /// Bytes delivered to the consumer through `read`.
    delivered: u64,
    /// Member count (started and verified — `VerifiedEnd` implies all
    /// started members verified, so this is also the receipt count).
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
            phase: Phase::Start,
            decomp: DeflateIn::new(),
            crc: Crc::new(),
            compressed: 0,
            delivered: 0,
            units: 0,
            failed: false,
        }
    }

    /// Verifies completion. `Ok` exactly in `VerifiedEnd` —
    /// the format end, trailer and checksum verification, and the
    /// scope's input-boundary condition all confirmed. Undelivered
    /// output is never discarded to reach this state; a mid-stream
    /// call reports `NotExhausted`.
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
            IntegritySummary::new(self.units, 0),
        ))
    }

    /// Compressed bytes consumed so far (diagnostic; the authoritative
    /// figure is on the receipt after `finish`).
    #[must_use]
    pub const fn compressed_bytes_consumed(&self) -> u64 {
        self.compressed
    }

    /// Reads `buf.len()` bytes as part of a member header, tracking the
    /// header cap and compressed budget.
    fn take_header_bytes(&mut self, buf: &mut [u8]) -> Result<(), CompressionError> {
        take_exact(
            self.reader,
            buf,
            &mut self.compressed,
            self.options.limits().max_compressed_bytes(),
        )
    }

    /// Reads the 2-byte member magic. At the stream's first member an
    /// empty `fill_buf` maps to `TruncatedInput` (a member was expected
    /// but the input ended); wrong bytes map to `InvalidFormat`.
    fn read_member_magic(&mut self) -> Result<(), CompressionError> {
        let mut magic = [0u8; 2];
        self.take_header_bytes(&mut magic)?;
        if magic != GZIP_MAGIC {
            return Err(invalid_format());
        }
        Ok(())
    }

    /// Parses one member header after the magic: the remaining 8 fixed
    /// bytes, optional FEXTRA / FNAME / FCOMMENT, optional FHCRC
    /// verification.
    fn read_member_header(&mut self) -> Result<(), CompressionError> {
        if self.units >= self.options.limits().max_units() {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::LimitExceeded,
            ));
        }
        let mut fixed = [0u8; HEADER_LEN];
        fixed[..2].copy_from_slice(&GZIP_MAGIC);
        self.take_header_bytes(&mut fixed[2..])?;
        if fixed[2] != 0x08 {
            return Err(invalid_format());
        }
        let flags = fixed[3];
        if flags & FLAG_RESERVED != 0 {
            return Err(invalid_format());
        }

        // FHCRC needs every header byte before the CRC field; the
        // accumulation buffer exists only when the flag is set.
        let need_hcrc = flags & FLAG_FHCRC != 0;
        let mut header: Option<Vec<u8>> = need_hcrc.then(|| fixed.to_vec());
        let mut header_len = HEADER_LEN;

        if flags & FLAG_FEXTRA != 0 {
            let mut xlen = [0u8; 2];
            self.take_header_bytes(&mut xlen)?;
            accumulate(&mut header, &xlen);
            header_len += 2;
            let extra_len = usize::from(u16::from_le_bytes(xlen));
            let mut extra = vec![0u8; extra_len];
            self.take_header_bytes(&mut extra)?;
            accumulate(&mut header, &extra);
            header_len += extra_len;
            if header_len > MEMBER_HEADER_MAX {
                return Err(invalid_format());
            }
        }
        if flags & FLAG_FNAME != 0 {
            self.read_terminated(&mut header, &mut header_len)?;
        }
        if flags & FLAG_FCOMMENT != 0 {
            self.read_terminated(&mut header, &mut header_len)?;
        }
        if need_hcrc {
            let mut field = [0u8; 2];
            self.take_header_bytes(&mut field)?;
            header_len += 2;
            if header_len > MEMBER_HEADER_MAX {
                return Err(invalid_format());
            }
            let mut crc = Crc::new();
            crc.update(header.as_deref().unwrap_or_default());
            if u16::from_le_bytes(field) != (crc.sum() & 0xFFFF) as u16 {
                return Err(CompressionError::from_kind(
                    CompressionErrorKind::ChecksumMismatch,
                ));
            }
        }

        self.units += 1;
        self.phase = Phase::Body;
        Ok(())
    }

    /// Reads a NUL-terminated header field (FNAME/FCOMMENT). The
    /// terminator is included in the FHCRC material per RFC 1952.
    fn read_terminated(
        &mut self,
        header: &mut Option<Vec<u8>>,
        header_len: &mut usize,
    ) -> Result<(), CompressionError> {
        loop {
            let mut byte = [0u8; 1];
            self.take_header_bytes(&mut byte)?;
            *header_len += 1;
            if let Some(material) = header.as_mut() {
                material.push(byte[0]);
            }
            if byte[0] == 0 {
                return Ok(());
            }
            if *header_len > MEMBER_HEADER_MAX {
                return Err(invalid_format());
            }
        }
    }

    /// Reads and verifies the member trailer (`CRC32 LE + ISIZE LE`).
    fn read_member_trailer(&mut self) -> Result<(), CompressionError> {
        let mut trailer = [0u8; TRAILER_LEN];
        self.take_header_bytes(&mut trailer)?;
        let expected_crc = u32::from_le_bytes(trailer[..4].try_into().expect("4 bytes"));
        let expected_isize = u32::from_le_bytes(trailer[4..].try_into().expect("4 bytes"));
        if expected_crc != self.crc.sum() || expected_isize != self.crc.amount() {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::ChecksumMismatch,
            ));
        }
        self.crc.reset();
        self.decomp.reset();
        self.phase = match self.options.scope() {
            DecodeScope::WholeInput => Phase::Between,
            DecodeScope::SingleUnit => Phase::Done,
        };
        Ok(())
    }

    /// `WholeInput` boundary: empty `fill_buf` is the only clean end
    /// (`Done`); anything else must be another member's magic.
    ///
    /// A `BufRead` may legally report a 1-byte buffer, so `[0x1f]` is
    /// consumed and the second magic byte demanded explicitly —
    /// short fills can never masquerade as `TrailingData` or
    /// `TruncatedInput`.
    fn read_member_boundary(&mut self) -> Result<(), CompressionError> {
        let limit = self.options.limits().max_compressed_bytes();
        let avail = self.reader.fill_buf().map_err(CompressionError::io)?;
        if avail.is_empty() {
            self.phase = Phase::Done;
            return Ok(());
        }
        if avail[0] != GZIP_MAGIC[0] {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::TrailingData,
            ));
        }
        if avail.len() >= 2 && avail[1] != GZIP_MAGIC[1] {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::TrailingData,
            ));
        }
        // `avail` is `1f` followed by `8b`, or a lone `1f` (short fill
        // or EOF — the second byte decides which).
        let known = avail.len().min(2);
        self.reader.consume(known);
        self.compressed += known as u64;
        if self.compressed > limit {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::LimitExceeded,
            ));
        }
        if known == 2 {
            self.phase = Phase::Header;
            return Ok(());
        }
        let mut second = [0u8; 1];
        // `TruncatedInput` if the source ends after a lone `1f`.
        take_exact(self.reader, &mut second, &mut self.compressed, limit)?;
        if second[0] != GZIP_MAGIC[1] {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::TrailingData,
            ));
        }
        self.phase = Phase::Header;
        Ok(())
    }

    /// State-machine driver: loops until output is produced, the
    /// verified end is reached (`Ok(0)`), or an error occurs.
    fn drive(&mut self, out: &mut [u8]) -> Result<usize, CompressionError> {
        let mut scratch = [0u8; 1];
        loop {
            match self.phase {
                Phase::Done => return Ok(0),
                Phase::Start => {
                    self.read_member_magic()?;
                    self.phase = Phase::Header;
                }
                Phase::Header => self.read_member_header()?,
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
                        self.crc.update(&target[..step.produced]);
                        self.delivered += step.produced as u64;
                        return Ok(step.produced);
                    }
                    if step.starved {
                        return Err(CompressionError::from_kind(
                            CompressionErrorKind::TruncatedInput,
                        ));
                    }
                    if step.end {
                        self.phase = Phase::Trailer;
                    }
                }
                Phase::Trailer => self.read_member_trailer()?,
                Phase::Between => {
                    self.read_member_boundary()?;
                    if matches!(self.phase, Phase::Done) {
                        return Ok(0);
                    }
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

fn accumulate(header: &mut Option<Vec<u8>>, bytes: &[u8]) {
    if let Some(material) = header.as_mut() {
        material.extend_from_slice(bytes);
    }
}
