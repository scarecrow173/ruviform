//! Verified completion evidence for encode/decode operations.
//!
//! Receipts are returned only by `finish()` — the explicit success
//! path. A receipt records what was *observed* (bytes in/out, units,
//! integrity checks performed), never a guarantee about durability or
//! downstream publication — those belong to the caller and to
//! `ruviform-io-file`.

/// Evidence for a completed encode operation.
///
/// `units_encoded` is the number of format units written (gzip members
/// or deflate streams). Current encoders write exactly one unit per
/// `Encoder`, so it is always `1`.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodeReceipt {
    input_bytes: u64,
    output_bytes: u64,
    units_encoded: u64,
}

impl EncodeReceipt {
    #[cfg(any(feature = "gzip", feature = "deflate"))]
    pub(crate) const fn new(input_bytes: u64, output_bytes: u64, units_encoded: u64) -> Self {
        Self {
            input_bytes,
            output_bytes,
            units_encoded,
        }
    }

    /// Uncompressed bytes accepted through `Write`.
    #[must_use]
    pub const fn input_bytes(&self) -> u64 {
        self.input_bytes
    }

    /// Total bytes written to the underlying writer, including format
    /// framing (gzip header + trailer).
    #[must_use]
    pub const fn output_bytes(&self) -> u64 {
        self.output_bytes
    }

    /// Format units completed (always `1` for the current encoders).
    #[must_use]
    pub const fn units_encoded(&self) -> u64 {
        self.units_encoded
    }
}

/// Per-unit integrity accounting for a completed decode.
///
/// Checksums are format-dependent: every gzip member carries a verified
/// CRC32/ISIZE trailer (counted as *checked*), while raw deflate carries
/// no checksum at all (counted as *unchecked*). A `ChecksumVerified` /
/// `NoChecksumPresent` single value would misrepresent mixed streams
/// (e.g. zstd frames with optional checksums, IO-IMP-07), so the counts
/// are kept separate.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegritySummary {
    checked_units: u64,
    unchecked_units: u64,
}

impl IntegritySummary {
    #[cfg(any(feature = "gzip", feature = "deflate"))]
    pub(crate) const fn new(checked_units: u64, unchecked_units: u64) -> Self {
        Self {
            checked_units,
            unchecked_units,
        }
    }

    /// Units whose format checksum was verified (gzip: CRC32 + ISIZE).
    #[must_use]
    pub const fn checked_units(&self) -> u64 {
        self.checked_units
    }

    /// Units the format provides no checksum for (raw deflate).
    #[must_use]
    pub const fn unchecked_units(&self) -> u64 {
        self.unchecked_units
    }
}

/// Evidence for a verified decode operation.
///
/// A `DecodeReceipt` exists only after the decoder observed the
/// format-defined end (member trailer / stream end), verified every
/// checksum the format provides, satisfied the [`crate::DecodeScope`]
/// input-boundary contract and stayed within [`crate::DecodeLimits`].
///
/// `compressed_bytes_consumed` counts bytes this decoder logically
/// consumed from the caller's [`std::io::BufRead`]. It deliberately does
/// not include bytes the caller's `BufRead` may have *prefetched* from
/// its own source — that physical-read count is not observable at this
/// layer and is never conflated with logical consumption.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeReceipt {
    compressed_bytes_consumed: u64,
    decoded_bytes_delivered: u64,
    units_decoded: u64,
    integrity: IntegritySummary,
}

impl DecodeReceipt {
    #[cfg(any(feature = "gzip", feature = "deflate"))]
    pub(crate) const fn new(
        compressed_bytes_consumed: u64,
        decoded_bytes_delivered: u64,
        units_decoded: u64,
        integrity: IntegritySummary,
    ) -> Self {
        Self {
            compressed_bytes_consumed,
            decoded_bytes_delivered,
            units_decoded,
            integrity,
        }
    }

    /// Bytes logically consumed from the provided `BufRead` (see type
    /// docs for the consumed-vs-prefetched distinction).
    #[must_use]
    pub const fn compressed_bytes_consumed(&self) -> u64 {
        self.compressed_bytes_consumed
    }

    /// Decoded bytes delivered to the consumer through `Read`.
    #[must_use]
    pub const fn decoded_bytes_delivered(&self) -> u64 {
        self.decoded_bytes_delivered
    }

    /// Units fully decoded and verified (gzip members / deflate streams).
    #[must_use]
    pub const fn units_decoded(&self) -> u64 {
        self.units_decoded
    }

    /// Per-unit integrity accounting (checked vs checksum-less units).
    #[must_use]
    pub const fn integrity(&self) -> IntegritySummary {
        self.integrity
    }
}
