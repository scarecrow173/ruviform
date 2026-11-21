//! Private input plumbing shared by every format module:
//! [`BoundedRead`] — the single authoritative implementation of the
//! `InputLimits::max_bytes` hard limit.
//!
//! Semantics: a document of *exactly* `max_bytes` may succeed; the
//! first byte beyond the limit trips [`BudgetExceeded`]. The trip is
//! recorded on `was_limit_exceeded` so the caller can reclassify a
//! backend-wrapped `io::Error` into
//! [`DocumentErrorKind::LimitExceeded`](crate::DocumentErrorKind::LimitExceeded).

use std::io::{self, Read};

/// Marker error produced when the input byte budget is exceeded.
/// Surfaced to the parser as an `io::Error`; the owning `BoundedRead`
/// is the source of truth (`was_limit_exceeded`).
#[derive(Debug)]
struct BudgetExceeded;

impl std::fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("input byte budget exceeded")
    }
}

impl std::error::Error for BudgetExceeded {}

/// A `Read` wrapper that counts bytes and hard-fails past `max_bytes`.
///
/// When the budget is spent, the next `read` performs a one-byte probe:
/// a real end-of-stream is reported honestly (`Ok(0)`), while any
/// remaining byte trips `exceeded` and returns an error — so "document
/// exactly at the budget" can succeed and nothing beyond it is silently
/// treated as EOF.
#[derive(Debug)]
pub(crate) struct BoundedRead<R> {
    inner: R,
    remaining: u64,
    exceeded: bool,
    eof: bool,
}

impl<R: Read> BoundedRead<R> {
    pub(crate) fn new(inner: R, max_bytes: u64) -> Self {
        Self {
            inner,
            remaining: max_bytes,
            exceeded: false,
            eof: false,
        }
    }

    /// Whether the byte budget was tripped. Checked by the caller
    /// *after* an error so a backend-flattened I/O failure can be
    /// reclassified as `LimitExceeded`.
    pub(crate) fn was_limit_exceeded(&self) -> bool {
        self.exceeded
    }
}

impl<R: Read> Read for BoundedRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.exceeded {
            return Err(io::Error::other(BudgetExceeded));
        }
        if buf.is_empty() || self.eof {
            return Ok(0);
        }
        if self.remaining == 0 {
            // Budget spent: probe whether the underlying reader is
            // genuinely at EOF before reporting one.
            let mut probe = [0u8; 1];
            return match self.inner.read(&mut probe) {
                Ok(0) => {
                    self.eof = true;
                    Ok(0)
                }
                Ok(_) => {
                    self.exceeded = true;
                    Err(io::Error::other(BudgetExceeded))
                }
                Err(e) => Err(e),
            };
        }
        let allowed = usize::try_from(self.remaining)
            .unwrap_or(usize::MAX)
            .min(buf.len());
        match self.inner.read(&mut buf[..allowed]) {
            Ok(0) => {
                self.eof = true;
                Ok(0)
            }
            Ok(n) => {
                self.remaining -= n as u64;
                Ok(n)
            }
            Err(e) => Err(e),
        }
    }
}

/// Reads the entire input under `limits` into a `String`, classifying
/// failures as [`DocumentError`](crate::DocumentError):
/// budget trip → `LimitExceeded`, other I/O → `Io`,
/// non-UTF-8 → `InvalidEncoding` (position = first invalid byte).
#[cfg(feature = "toml")]
pub(crate) fn read_to_string(
    reader: &mut impl Read,
    limits: crate::InputLimits,
    format: crate::DocumentFormat,
) -> Result<String, crate::DocumentError> {
    use crate::{DocumentError, DocumentErrorKind, DocumentLimit, DocumentPosition};

    let mut bounded = BoundedRead::new(reader, limits.max_bytes);
    let mut buf = Vec::new();
    if let Err(e) = bounded.read_to_end(&mut buf) {
        return Err(if bounded.was_limit_exceeded() {
            DocumentError::from_kind(DocumentErrorKind::LimitExceeded, format).with_limit(
                DocumentLimit {
                    name: "max_bytes",
                    max: limits.max_bytes,
                },
            )
        } else {
            DocumentError::with_source(DocumentErrorKind::Io, format, e)
        });
    }
    String::from_utf8(buf).map_err(|e| {
        DocumentError::with_parts(
            DocumentErrorKind::InvalidEncoding,
            format,
            Some(DocumentPosition {
                byte: Some(e.utf8_error().valid_up_to() as u64),
                line: None,
                column: None,
                record: None,
            }),
            None,
            Some(Box::new(e)),
        )
    })
}

/// Kani proofs of the `BoundedRead` contract — the shared byte-budget
/// gate every format reader drives through. `BoundedRead` is
/// `pub(crate)`, so the harness lives in-source; `cargo kani --tests`
/// covers lib-target proofs (run via `mise run test-formal-kani`).
///
/// Slice-1 lesson applied: no symbolic streams — a fixed 6-byte input
/// plus a symbolic budget keeps the model tiny while still covering
/// every branch of the gate (in-budget, boundary, probe, sticky).
#[cfg(kani)]
mod proofs {
    use super::BoundedRead;
    use std::io::Read;

    /// Delivered bytes never exceed `min(len, max_bytes)`; `Ok(0)` is
    /// reported only when everything deliverable was delivered (a byte
    /// past the budget can never masquerade as EOF); the trip flag is
    /// set iff the stream held a byte beyond the budget; and both the
    /// honest EOF and the budget error are sticky across reads.
    #[kani::proof]
    #[kani::unwind(16)]
    fn bounded_read_budget_gate() {
        const INPUT: &[u8] = b"abcdef"; // 6 bytes
        let max_bytes: u64 = kani::any();
        let want = usize::try_from((INPUT.len() as u64).min(max_bytes)).unwrap_or(usize::MAX);
        let mut bounded = BoundedRead::new(INPUT, max_bytes);
        let mut buf = [0u8; 2];
        let mut delivered = 0usize;
        let mut terminal = false;
        for _ in 0..8 {
            match bounded.read(&mut buf) {
                Ok(0) => {
                    // Honest EOF: all `want` bytes were delivered, the
                    // flag stayed clear, and the next read is `Ok(0)`
                    // again.
                    assert!(delivered == want);
                    assert!(!bounded.was_limit_exceeded());
                    assert!(bounded.read(&mut buf).unwrap_or(1) == 0);
                    terminal = true;
                    break;
                }
                Ok(n) => {
                    assert!(n <= buf.len());
                    delivered += n;
                    assert!(delivered <= want);
                }
                Err(_) => {
                    // Budget trip: a byte lay past the budget, the flag
                    // is set, and the next read errors again.
                    assert!((INPUT.len() as u64) > max_bytes);
                    assert!(delivered == want);
                    assert!(bounded.was_limit_exceeded());
                    assert!(bounded.read(&mut buf).is_err());
                    terminal = true;
                    break;
                }
            }
        }
        // The stream always terminates: `want <= 6` bytes at 2 per read
        // plus one probe is well inside the loop bound.
        assert!(terminal);
        assert!(delivered == want);
        assert!(bounded.was_limit_exceeded() == ((INPUT.len() as u64) > max_bytes));
    }
}
