//! [`InputLimits`] — the shared input resource limits enforced by every
//! format module's `from_reader`.
//!
//! Only limits that are *actually enforced* live here: `max_bytes` is a
//! hard ceiling on the bytes pulled from the caller's reader — rejected
//! before the parser can consume past it. A byte limit does **not**
//! bound CPU time or the heap a `Deserialize` implementation allocates;
//! those stay advisory and are never claimed.

/// Default input byte ceiling: 16 MiB.
///
/// Chosen for config/manifest-scale documents; raise per call site for
/// larger payloads. The default is deliberately finite.
pub const DEFAULT_MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

/// Hard input limits for `from_reader`.
///
/// `#[non_exhaustive]` — new limits may be added; construct with
/// [`InputLimits::new`] or [`Default`], then adjust fields as needed.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// use ruviform_io_structured::InputLimits;
///
/// let limits = InputLimits::new(1024);
/// assert_eq!(limits.max_bytes, 1024);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputLimits {
    /// Maximum bytes the reader may pull from the input stream,
    /// inclusive. An input of exactly `max_bytes` bytes may succeed;
    /// one byte more is rejected with
    /// [`DocumentErrorKind::LimitExceeded`](crate::DocumentErrorKind::LimitExceeded).
    pub max_bytes: u64,
}

impl InputLimits {
    /// Limits with [`DEFAULT_MAX_INPUT_BYTES`].
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new(max_bytes: u64) -> Self {
        Self { max_bytes }
    }
}

impl Default for InputLimits {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_INPUT_BYTES)
    }
}
