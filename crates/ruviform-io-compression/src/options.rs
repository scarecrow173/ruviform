//! Shared option and scope vocabulary for the format modules.
//!
//! [`DecodeScope`] selects how much of a concatenated/input stream a
//! decoder is responsible for. [`EncodeLimits`] and [`DecodeLimits`]
//! carry cumulative byte/unit budgets. All three are format-agnostic so
//! a future `zstd` module (IO-IMP-07) can reuse the same vocabulary
//! instead of inventing parallel types.

/// How far a decoder must consume/verify the provided input.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DecodeScope {
    /// Decode every unit (gzip member / deflate stream) in order and
    /// require the input to end exactly at the last unit's end.
    ///
    /// Concatenated units are cumulative for limits and receipt
    /// counting. Bytes after the final unit that are not another unit
    /// are rejected (`TrailingData`); a unit that never terminates is
    /// `TruncatedInput`. This is the default and the recommended mode
    /// for standalone `.gz` / raw-deflate blobs.
    #[default]
    WholeInput,
    /// Decode only the first unit and stop at its format end. Bytes
    /// after that end stay unconsumed in the caller's [`std::io::BufRead`]
    /// and remain readable by the caller — the embedded-stream case.
    ///
    /// `Decoder::finish` verifies the single unit's end/trailer/checksum
    /// but does not require input EOF.
    SingleUnit,
}

/// Cumulative budgets for an encoder: input bytes accepted through
/// `Write` and output bytes written to the underlying writer (including
/// format framing — gzip header/trailer bytes count as output).
///
/// Limits are cumulative across the whole stream. Exceeding a bound
/// fails the encoder (`LimitExceeded`, terminal `Failed` state); the
/// encoder never performs a physical write that would cross
/// `max_output_bytes`.
///
/// These are byte budgets only. CPU time, backend-internal memory and
/// downstream-sink memory are **not** hard-limited by this type.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodeLimits {
    input_bytes: u64,
    output_bytes: u64,
}

impl EncodeLimits {
    /// Limits with no bound on either axis.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            input_bytes: u64::MAX,
            output_bytes: u64::MAX,
        }
    }

    /// Caps total uncompressed bytes accepted by the encoder.
    #[must_use]
    pub const fn with_max_input_bytes(mut self, limit: u64) -> Self {
        self.input_bytes = limit;
        self
    }

    /// Caps total bytes written to the underlying writer.
    #[must_use]
    pub const fn with_max_output_bytes(mut self, limit: u64) -> Self {
        self.output_bytes = limit;
        self
    }

    /// Configured input byte cap (`u64::MAX` = unbounded).
    #[must_use]
    pub const fn max_input_bytes(&self) -> u64 {
        self.input_bytes
    }

    /// Configured output byte cap (`u64::MAX` = unbounded).
    #[must_use]
    pub const fn max_output_bytes(&self) -> u64 {
        self.output_bytes
    }
}

impl Default for EncodeLimits {
    fn default() -> Self {
        Self::new()
    }
}

/// Cumulative budgets for a decoder, applied across the *whole
/// concatenated* stream when [`DecodeScope::WholeInput`] is selected.
///
/// - `max_compressed_bytes` bounds logical bytes consumed from the
///   caller's [`std::io::BufRead`].
/// - `max_decoded_bytes` bounds bytes delivered to the consumer through
///   `Read`. Reaching it exactly at a unit's verified end is allowed;
///   any additional decoded byte fails with `LimitExceeded` — hitting
///   the bound is never silently treated as EOF.
/// - `max_units` bounds the number of decoded units (gzip members /
///   deflate streams), counted cumulatively across the stream.
///
/// These are byte/unit budgets only. CPU time, backend-internal memory
/// and consumer-side memory are **not** hard-limited by this type.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeLimits {
    compressed_bytes: u64,
    decoded_bytes: u64,
    units: u64,
}

impl DecodeLimits {
    /// Limits with no bound on any axis.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            compressed_bytes: u64::MAX,
            decoded_bytes: u64::MAX,
            units: u64::MAX,
        }
    }

    /// Caps cumulative logical bytes consumed from the source `BufRead`.
    #[must_use]
    pub const fn with_max_compressed_bytes(mut self, limit: u64) -> Self {
        self.compressed_bytes = limit;
        self
    }

    /// Caps cumulative decoded bytes delivered to the consumer.
    #[must_use]
    pub const fn with_max_decoded_bytes(mut self, limit: u64) -> Self {
        self.decoded_bytes = limit;
        self
    }

    /// Caps cumulative decoded units (members/streams).
    #[must_use]
    pub const fn with_max_units(mut self, limit: u64) -> Self {
        self.units = limit;
        self
    }

    /// Configured compressed-byte cap (`u64::MAX` = unbounded).
    #[must_use]
    pub const fn max_compressed_bytes(&self) -> u64 {
        self.compressed_bytes
    }

    /// Configured decoded-byte cap (`u64::MAX` = unbounded).
    #[must_use]
    pub const fn max_decoded_bytes(&self) -> u64 {
        self.decoded_bytes
    }

    /// Configured unit-count cap (`u64::MAX` = unbounded).
    #[must_use]
    pub const fn max_units(&self) -> u64 {
        self.units
    }
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self::new()
    }
}
