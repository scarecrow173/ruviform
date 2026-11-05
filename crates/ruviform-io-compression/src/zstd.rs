//! Zstd (RFC 8878) frame encode/decode.
//!
//! The Ruviform side owns all frame-boundary policy — magic and frame
//! header parsing, skippable frames, window-size limits and the
//! unit/checksum accounting that feeds the receipt — while the frame
//! body uses the shared private backend pump (`zstd_safe` `DCtx`). That
//! split is what makes `units_decoded`, `compressed_bytes_consumed` and
//! `IntegritySummary`'s checked/unchecked split exact even for mixed
//! checksum and skippable-frame streams.
//!
//! Frame capabilities that the borrowed-stream contract cannot verify
//! are reported as `Unsupported` rather than silently approximated:
//! dictionary-referencing frames (`Dictionary_ID` present) are
//! rejected, and skippable frames follow the explicit
//! [`SkippableFramePolicy`]. Magic-less frames and preset/raw-content
//! dictionaries are not supported.

use std::io::{self, BufRead, Write};

use crate::error::{CompressionError, CompressionErrorKind};
use crate::options::{DecodeLimits, DecodeScope, EncodeLimits};
use crate::receipt::{DecodeReceipt, EncodeReceipt, IntegritySummary};
use crate::stream::{EncoderCore, ZstdIn, ZstdOut, invalid_state_io, take_exact};

/// Zstd data-frame magic number (RFC 8878 `0xFD2FB528`, little-endian
/// bytes `28 b5 2f fd`).
const ZSTD_MAGIC: u32 = 0xFD2F_B528;
/// First skippable-frame magic (`0x184D2A50` + `0..=0xF`).
const SKIPPABLE_MAGIC_LOW: u32 = 0x184D_2A50;
/// Last skippable-frame magic.
const SKIPPABLE_MAGIC_HIGH: u32 = 0x184D_2A5F;
/// Maximum zstd frame header size: magic (4) + FHD (1) +
/// `Window_Descriptor` (1) + `Dictionary_ID` (4) + `Frame_Content_Size` (8).
const FRAME_HEADER_MAX: usize = 18;
/// Default decoder window cap, matching the zstd reference decoder's
/// `ZSTD_WINDOWLOG_LIMIT_DEFAULT` (window size <= 128 MiB).
const DEFAULT_WINDOW_LOG_MAX: u32 = 27;

fn invalid_format() -> CompressionError {
    CompressionError::from_kind(CompressionErrorKind::InvalidFormat)
}

/// Policy for skippable frames (`0x184D2A50..=0x184D2A5F`) encountered
/// while decoding.
///
/// Skippable frames carry user metadata and no compressed payload.
/// They are never counted as decoded units — [`DecodeReceipt`] unit
/// and integrity counts cover data frames only — though their bytes do
/// count toward `compressed_bytes_consumed` and `max_compressed_bytes`.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SkippableFramePolicy {
    /// Skip the frame's declared payload and continue decoding. This is
    /// the default: skippable frames are transparently ignored.
    #[default]
    Accept,
    /// Reject any skippable frame with [`CompressionErrorKind::Unsupported`].
    Reject,
}

/// Encode options for [`Encoder`]: zstd compression level, the content
/// checksum flag, plus the shared [`EncodeLimits`] byte budgets.
///
/// `level` follows zstd semantics (`0` = backend default, negative =
/// faster; the representable range is the backend's
/// `min_c_level()..=max_c_level()` — an out-of-range level is rejected
/// by `Encoder::new` with `InvalidInput`). It is deliberately *not* a
/// cross-format level type.
#[derive(Debug, Clone, Copy)]
pub struct EncodeOptions {
    level: i32,
    checksum: bool,
    limits: EncodeLimits,
}

impl EncodeOptions {
    /// Options with explicit `level`/`checksum`/`limits`.
    #[must_use]
    pub const fn new(level: i32, checksum: bool, limits: EncodeLimits) -> Self {
        Self {
            level,
            checksum,
            limits,
        }
    }

    /// Zstd level (`0` = backend default).
    #[must_use]
    pub const fn level(&self) -> i32 {
        self.level
    }

    /// Whether the frame carries a 4-byte content checksum (verified
    /// by the decoder, counted as a checked unit).
    #[must_use]
    pub const fn checksum(&self) -> bool {
        self.checksum
    }

    /// Configured encoder budgets.
    #[must_use]
    pub const fn limits(&self) -> EncodeLimits {
        self.limits
    }
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self::new(0, false, EncodeLimits::new())
    }
}

/// Decode options for [`Decoder`]: the input [`DecodeScope`], the
/// shared [`DecodeLimits`] cumulative budgets, the maximum accepted
/// window log and the skippable-frame policy.
///
/// `max_window_log` bounds the memory a frame may declare: a frame
/// whose computed `Window_Size` exceeds `1 << max_window_log` is
/// rejected with `LimitExceeded` before the backend allocates for it
/// (and the backend's `ZSTD_d_windowLogMax` enforces the same bound).
/// Values outside the representable range `10..=41` are clamped to the
/// nearest bound. The default `27` matches the zstd reference
/// decoder's own limit.
#[derive(Debug, Clone, Copy)]
pub struct DecodeOptions {
    scope: DecodeScope,
    limits: DecodeLimits,
    max_window_log: u32,
    skippable_frames: SkippableFramePolicy,
}

impl DecodeOptions {
    /// Options with an explicit `scope` and `limits`.
    #[must_use]
    pub const fn new(scope: DecodeScope, limits: DecodeLimits) -> Self {
        Self {
            scope,
            limits,
            max_window_log: DEFAULT_WINDOW_LOG_MAX,
            skippable_frames: SkippableFramePolicy::Accept,
        }
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

    /// Maximum accepted frame window log (clamped to `10..=41` when
    /// applied). Default `27`.
    #[must_use]
    pub const fn max_window_log(&self) -> u32 {
        self.max_window_log
    }

    /// Overrides `max_window_log`.
    #[must_use]
    pub const fn with_max_window_log(mut self, max_window_log: u32) -> Self {
        self.max_window_log = max_window_log;
        self
    }

    /// Configured skippable-frame policy (default
    /// [`SkippableFramePolicy::Accept`]).
    #[must_use]
    pub const fn skippable_frames(&self) -> SkippableFramePolicy {
        self.skippable_frames
    }

    /// Overrides the skippable-frame policy.
    #[must_use]
    pub const fn with_skippable_frames(mut self, policy: SkippableFramePolicy) -> Self {
        self.skippable_frames = policy;
        self
    }
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self::new(DecodeScope::WholeInput, DecodeLimits::new())
    }
}

/// Zstd frame encoder over a borrowed `W`. One `Encoder` writes exactly
/// one zstd frame (concatenated frames are produced by finishing one
/// encoder and starting another on the same writer).
///
/// Lifecycle: `Writing -> Finishing -> Finished | Failed`. Only an
/// explicit successful [`Encoder::finish`] completes the frame (the
/// `ZSTD_e_end` epilogue plus the optional content checksum); `flush`
/// is a zstd block flush, not completion. `Drop` performs **no**
/// writes.
///
/// **RT contract:** non-RT only — `Write` may block and allocate.
///
/// # Examples
///
/// ```
/// use ruviform_io_compression::zstd::{Encoder, EncodeOptions};
/// use std::io::Write;
///
/// let mut out = Vec::new();
/// {
///     let mut enc = Encoder::new(&mut out, EncodeOptions::default())?;
///     enc.write_all(b"hello")?;
///     let receipt = enc.finish()?;
///     assert_eq!(receipt.input_bytes(), 5);
///     assert_eq!(enc.finish()?, receipt); // idempotent
/// } // borrow of `out` ends here
/// assert_eq!(&out[..4], &[0x28, 0xb5, 0x2f, 0xfd]); // zstd magic
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Encoder<'w, W: Write + ?Sized> {
    core: EncoderCore<'w, W, ZstdOut>,
}

impl<'w, W: Write + ?Sized> Encoder<'w, W> {
    /// Builds an encoder. Writes nothing yet — the frame header is
    /// emitted by the backend on the first `write`/`flush`/`finish`.
    /// An out-of-range `level` fails `new` with `InvalidInput`.
    pub fn new(writer: &'w mut W, options: EncodeOptions) -> Result<Self, CompressionError> {
        Ok(Self {
            core: EncoderCore::new(
                writer,
                ZstdOut::new(options.level(), options.checksum())?,
                options.limits(),
            ),
        })
    }

    /// Completes the frame: `ZSTD_e_end` epilogue (plus the content
    /// checksum when configured), then the underlying `flush`.
    /// Idempotent — a second call returns the stored [`EncodeReceipt`].
    /// After `Failed`, reports `InvalidState`.
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

/// Internal phases — `Start -> Header -> Body -> Between -> Done`.
/// `Between` only runs under `WholeInput`; `Done` is `VerifiedEnd`.
#[derive(Debug)]
enum Phase {
    /// Stream start: the first unit's magic is not yet consumed.
    Start,
    /// Magic consumed; the rest of the frame header is expected.
    Header,
    /// Inside the frame body (pending header bytes drain first, then
    /// reader input).
    Body,
    /// Frame ended under `WholeInput`: peek for another frame, a
    /// skippable frame, or clean EOF.
    Between,
    /// Verification complete (`VerifiedEnd`).
    Done,
}

/// Zstd frame decoder over a borrowed `R`.
///
/// `impl Read` yields decoded bytes; `Ok(0)` on a non-empty buffer is
/// returned only once the *verified* end is reached — after the last
/// frame's end (and content checksum where present) and the scope's
/// input-boundary contract both hold. `read(&mut [])` never changes
/// state and is not evidence of completion. [`Decoder::finish`]
/// inspects state only: `Ok` exactly in `VerifiedEnd`, `NotExhausted`
/// while output or verification may remain, `InvalidState` once
/// `Failed`.
///
/// `SingleUnit` leaves bytes after the first frame's end unconsumed
/// inside the caller's `BufRead`. `compressed_bytes_consumed` counts
/// logical consumption only — a caller-side `BufReader`'s physical
/// prefetch is not included.
///
/// Frames with a `Dictionary_ID` are rejected (`Unsupported`) — no
/// dictionary surface exists. Frames declaring a window larger than
/// [`DecodeOptions::max_window_log`] are rejected (`LimitExceeded`)
/// before the backend allocates for them.
///
/// **RT contract:** non-RT only — `Read` may block and allocate.
///
/// # Examples
///
/// ```
/// use ruviform_io_compression::zstd::{Decoder, DecodeOptions};
/// use std::io::{BufReader, Read};
///
/// # let mut encoded = Vec::new();
/// # {
/// #   use ruviform_io_compression::zstd::{Encoder, EncodeOptions};
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
/// // Default options emit no checksum: the unit counts as unchecked.
/// assert_eq!(receipt.integrity().unchecked_units(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Decoder<'r, R: BufRead + ?Sized> {
    reader: &'r mut R,
    options: DecodeOptions,
    phase: Phase,
    decomp: ZstdIn,
    /// Logical bytes consumed from `reader`.
    compressed: u64,
    /// Bytes delivered to the consumer through `read`.
    delivered: u64,
    /// Data frames accepted (started and verified — `VerifiedEnd`
    /// implies all started frames verified).
    units: u64,
    /// Accepted frames carrying a content checksum (verified by the
    /// backend at the frame end).
    checked: u64,
    /// Current frame's complete header (magic included), fed to the
    /// backend before live reader input so header bytes are consumed
    /// exactly once — counted here, not re-counted.
    pending: [u8; FRAME_HEADER_MAX],
    pending_len: usize,
    pending_pos: usize,
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
            decomp: ZstdIn::new(options.max_window_log()),
            compressed: 0,
            delivered: 0,
            units: 0,
            checked: 0,
            pending: [0; FRAME_HEADER_MAX],
            pending_len: 0,
            pending_pos: 0,
            failed: false,
        }
    }

    /// Verifies completion. `Ok` exactly in `VerifiedEnd` — the format
    /// end, content-checksum verification where present, and the
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
            IntegritySummary::new(self.checked, self.units - self.checked),
        ))
    }

    /// Compressed bytes consumed so far (diagnostic; the authoritative
    /// figure is on the receipt after `finish`).
    #[must_use]
    pub const fn compressed_bytes_consumed(&self) -> u64 {
        self.compressed
    }

    /// Reads `buf.len()` bytes as part of a frame header, tracking the
    /// compressed budget.
    fn take_header_bytes(&mut self, buf: &mut [u8]) -> Result<(), CompressionError> {
        take_exact(
            self.reader,
            buf,
            &mut self.compressed,
            self.options.limits().max_compressed_bytes(),
        )
    }

    /// Boundary scan shared by `Start` (`first = true`) and `Between`
    /// (`false`): decide whether the input ends here or another unit
    /// follows.
    ///
    /// - Empty input: `Done` between units, `TruncatedInput` at start.
    /// - Skippable magic: policy applies (`Accept` skips the payload
    ///   and stays in the same boundary phase; `Reject` fails
    ///   `Unsupported`).
    /// - Zstd magic: the `max_units` budget is checked and the magic is
    ///   staged into `pending` for `Header`.
    /// - Anything else: `InvalidFormat` at start, `TrailingData`
    ///   between units.
    fn read_frame_boundary(&mut self, first: bool) -> Result<(), CompressionError> {
        let limit = self.options.limits().max_compressed_bytes();
        let unexpected = if first {
            CompressionErrorKind::InvalidFormat
        } else {
            CompressionErrorKind::TrailingData
        };
        let avail = self.reader.fill_buf().map_err(CompressionError::io)?;
        if avail.is_empty() {
            if first {
                return Err(CompressionError::from_kind(
                    CompressionErrorKind::TruncatedInput,
                ));
            }
            self.phase = Phase::Done;
            return Ok(());
        }
        // First byte must be able to start a zstd or skippable magic
        // (`28` / `50..=5f` little-endian); anything else is junk.
        match avail[0] {
            0x28 | 0x50..=0x5f => {}
            _ => return Err(CompressionError::from_kind(unexpected)),
        }
        let mut magic = [0u8; 4];
        // A short source after a magic-looking first byte is truncated,
        // not trailing — the format boundary is mid-word.
        take_exact(self.reader, &mut magic, &mut self.compressed, limit)?;
        let magic = u32::from_le_bytes(magic);
        if (SKIPPABLE_MAGIC_LOW..=SKIPPABLE_MAGIC_HIGH).contains(&magic) {
            self.skip_skippable_frame()?;
            // A skippable frame is consumed input: subsequent EOF is a
            // clean boundary (`Between`), not a truncated stream —
            // including a stream that is skippable frames only.
            self.phase = Phase::Between;
            return Ok(());
        }
        if magic != ZSTD_MAGIC {
            return Err(CompressionError::from_kind(unexpected));
        }
        if self.units >= self.options.limits().max_units() {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::LimitExceeded,
            ));
        }
        self.pending[..4].copy_from_slice(&magic.to_le_bytes());
        self.pending_len = 4;
        self.pending_pos = 0;
        self.phase = Phase::Header;
        Ok(())
    }

    /// Skips one skippable frame after its magic: the 4-byte
    /// little-endian `Frame_Size` field, then that many payload bytes.
    /// All skipped bytes count toward the compressed budget; the frame
    /// never counts as a decoded unit.
    fn skip_skippable_frame(&mut self) -> Result<(), CompressionError> {
        match self.options.skippable_frames() {
            SkippableFramePolicy::Accept => {}
            // `#[non_exhaustive]` is exhaustive inside this crate — a
            // future variant fails to compile here until its policy is
            // decided deliberately.
            SkippableFramePolicy::Reject => {
                return Err(CompressionError::from_kind(
                    CompressionErrorKind::Unsupported,
                ));
            }
        }
        let limit = self.options.limits().max_compressed_bytes();
        let mut size = [0u8; 4];
        take_exact(self.reader, &mut size, &mut self.compressed, limit)?;
        let mut remaining = u64::from(u32::from_le_bytes(size));
        while remaining > 0 {
            let avail = self.reader.fill_buf().map_err(CompressionError::io)?;
            if avail.is_empty() {
                return Err(CompressionError::from_kind(
                    CompressionErrorKind::TruncatedInput,
                ));
            }
            let n = usize::try_from(remaining.min(avail.len() as u64)).unwrap_or(usize::MAX);
            self.reader.consume(n);
            self.compressed += n as u64;
            if self.compressed > limit {
                return Err(CompressionError::from_kind(
                    CompressionErrorKind::LimitExceeded,
                ));
            }
            remaining -= n as u64;
        }
        Ok(())
    }

    /// Parses the frame header after the magic: the FHD byte,
    /// `Window_Descriptor` (absent for single-segment frames),
    /// `Dictionary_ID` and `Frame_Content_Size` fields. Reserved-bit,
    /// dictionary and window violations are rejected deterministically
    /// before the backend sees the frame.
    fn read_frame_header(&mut self) -> Result<(), CompressionError> {
        let mut fhd = [0u8; 1];
        self.take_header_bytes(&mut fhd)?;
        self.pending[4] = fhd[0];
        self.pending_len = 5;
        let fhd = fhd[0];
        let fcs_flag = fhd >> 6;
        let single_segment = fhd & 0x20 != 0;
        let checksum = fhd & 0x04 != 0;
        let did_flag = fhd & 0x03;
        if fhd & 0x08 != 0 {
            // Reserved FHD bit must be zero (RFC 8878 §3.1.1.1.1.2).
            return Err(invalid_format());
        }
        if did_flag != 0 {
            // Dictionary frames require a dictionary surface this crate
            // does not provide — an explicit Unsupported, never a guess.
            return Err(CompressionError::from_kind(
                CompressionErrorKind::Unsupported,
            ));
        }
        let did_len = match did_flag {
            1 => 1usize,
            2 => 2,
            3 => 4,
            _ => 0,
        };
        let fcs_len = match fcs_flag {
            0 if single_segment => 1,
            0 => 0,
            1 => 2,
            2 => 4,
            _ => 8,
        };
        let window_desc_len = usize::from(!single_segment);
        let rest = window_desc_len + did_len + fcs_len;
        let start = self.pending_len;
        let mut rest_buf = [0u8; 13];
        self.take_header_bytes(&mut rest_buf[..rest])?;
        self.pending[start..start + rest].copy_from_slice(&rest_buf[..rest]);
        self.pending_len = start + rest;

        let fields = &self.pending[start..start + rest];
        let mut offset = 0usize;
        let window_descriptor = if single_segment {
            None
        } else {
            offset = 1;
            Some(fields[0])
        };
        offset += did_len;
        let fcs = {
            let field = &fields[offset..offset + fcs_len];
            let mut padded = [0u8; 8];
            padded[..fcs_len].copy_from_slice(field);
            let raw = u64::from_le_bytes(padded);
            if fcs_len == 2 { raw + 256 } else { raw }
        };
        let window_size: u128 = match window_descriptor {
            Some(descriptor) => {
                let exponent = u32::from(descriptor >> 3);
                let mantissa = u128::from(descriptor & 0x07);
                let base = 1u128 << (10 + exponent);
                base + (base / 8) * mantissa
            }
            // Single-segment: the window is the content itself, with a
            // 1 KiB floor per RFC 8878.
            None => u128::from(fcs.max(1024)),
        };
        let max_log = self.options.max_window_log().clamp(10, 41);
        if window_size > (1u128 << max_log) {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::LimitExceeded,
            ));
        }

        self.units += 1;
        if checksum {
            self.checked += 1;
        }
        self.pending_pos = 0;
        self.phase = Phase::Body;
        Ok(())
    }

    /// Transition at a verified frame end: `SingleUnit` stops here;
    /// `WholeInput` scans the boundary for another unit.
    fn frame_done(&mut self) {
        self.pending_len = 0;
        self.pending_pos = 0;
        self.phase = match self.options.scope() {
            DecodeScope::SingleUnit => Phase::Done,
            _ => Phase::Between,
        };
    }

    /// Pumps one decompress step into `target`, draining the staged
    /// frame header before pulling from `reader`. Returns
    /// `(produced, frame_end)`.
    fn pump_body(&mut self, target: &mut [u8]) -> Result<(usize, bool), CompressionError> {
        if self.pending_pos < self.pending_len {
            let (consumed, produced, ended) = self
                .decomp
                .pump_slice(&self.pending[self.pending_pos..self.pending_len], target)?;
            self.pending_pos += consumed;
            if consumed == 0 && produced == 0 && !ended {
                // The complete header was offered but the backend made
                // no progress.
                return Err(CompressionError::from_kind(
                    CompressionErrorKind::CorruptData,
                ));
            }
            return Ok((produced, ended));
        }
        let input = self.reader.fill_buf().map_err(CompressionError::io)?;
        let starved = input.is_empty();
        let (consumed, produced, ended) = self.decomp.pump_slice(input, target)?;
        self.reader.consume(consumed);
        self.compressed += consumed as u64;
        if self.compressed > self.options.limits().max_compressed_bytes() {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::LimitExceeded,
            ));
        }
        if consumed == 0 && produced == 0 && !ended && !starved {
            // Non-empty input and output space yet no progress.
            return Err(CompressionError::from_kind(
                CompressionErrorKind::CorruptData,
            ));
        }
        if starved && produced == 0 && !ended {
            return Err(CompressionError::from_kind(
                CompressionErrorKind::TruncatedInput,
            ));
        }
        Ok((produced, ended))
    }

    /// State-machine driver: loops until output is produced, the
    /// verified end is reached (`Ok(0)`), or an error occurs.
    fn drive(&mut self, out: &mut [u8]) -> Result<usize, CompressionError> {
        let mut scratch = [0u8; 1];
        loop {
            match self.phase {
                Phase::Done => return Ok(0),
                Phase::Start => self.read_frame_boundary(true)?,
                Phase::Header => self.read_frame_header()?,
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
                    let (produced, ended) = self.pump_body(target)?;
                    // A pump may deliver the last body bytes and the
                    // frame end in the same step — the end must be
                    // latched even when output is still being returned.
                    if ended {
                        self.frame_done();
                    }
                    if produced > 0 {
                        if probing {
                            // At the decoded cap, a further byte proves
                            // the budget was exceeded — it is *not*
                            // delivered to the consumer.
                            return Err(CompressionError::from_kind(
                                CompressionErrorKind::LimitExceeded,
                            ));
                        }
                        self.delivered += produced as u64;
                        return Ok(produced);
                    }
                }
                Phase::Between => {
                    self.read_frame_boundary(false)?;
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
