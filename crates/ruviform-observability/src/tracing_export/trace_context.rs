//! W3C Trace Context `traceparent` adapter — Issue #278 (OBS-03).
//!
//! [`TraceContext`] is a small `Copy` value holding the four fields of
//! a W3C `traceparent` header: version, trace-id, parent-id, and
//! trace-flags. It exists so hosts can carry an *external* trace
//! correlation handle into `rt_record` events without this crate taking
//! a dependency on an OpenTelemetry or `tracing-opentelemetry` stack.
//!
//! # Trust boundary
//!
//! A `traceparent` string is **untrusted input**: [`TraceContext::parse`]
//! validates syntax only — field count, lowercase hex, non-zero ids,
//! and the version `ff` ban — and never interprets the result as
//! authorization, identity, or permission. The emitted `trace_*`
//! fields are correlation metadata; nothing here generates spans,
//! parent/child relationships, or privilege from them.
//!
//! # Accepted grammar
//!
//! `version "-" trace-id "-" parent-id "-" trace-flags`, where hex is
//! lowercase only:
//!
//! - `version`: exactly 2 hex chars; `ff` is always invalid. Version
//!   `00` requires *exactly* four fields (the W3C rule for the only
//!   defined version). Other versions are read forward-compatibly: the
//!   first four fields are parsed and any trailing vendor-specific
//!   fields are ignored (they are not preserved by
//!   [`TraceContext::to_traceparent`]).
//! - `trace-id`: 32 hex chars, not all zero.
//! - `parent-id`: 16 hex chars, not all zero.
//! - `trace-flags`: 2 hex chars; bit `0x01` is the sampled flag.
//!
//! Anything else — wrong length, uppercase hex, non-hex characters,
//! zero ids, `ff` version, extra fields on version `00` — parses to
//! `None`, which callers treat as "no trace context" rather than an
//! error.

/// A parsed W3C `traceparent` — correlation metadata only.
///
/// `Copy` value type; construction is fallible because all-zero ids and
/// version `ff` are never valid. See the module docs for the accepted
/// grammar and trust boundary.
///
/// # Examples
///
/// ```
/// use ruviform_observability::tracing_export::TraceContext;
///
/// // The W3C specification's example header.
/// let ctx = TraceContext::parse(
///     "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
/// )
/// .expect("valid traceparent");
/// assert!(ctx.is_sampled());
/// assert_eq!(ctx.trace_id_hex(), "4bf92f3577b34da6a3ce929d0e0e4736");
/// assert_eq!(ctx.parent_span_id_hex(), "00f067aa0ba902b7");
///
/// // Malformed input is `None`, never an error or a partial context.
/// assert!(TraceContext::parse("ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01").is_none());
/// assert!(TraceContext::parse("00-00000000000000000000000000000000-00f067aa0ba902b7-01").is_none());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TraceContext {
    version: u8,
    trace_id: u128,
    parent_span_id: u64,
    flags: u8,
}

impl TraceContext {
    /// Builds a context from raw fields, or `None` when the values can
    /// never form a valid `traceparent` (version `ff`, all-zero
    /// trace-id, all-zero parent-id).
    ///
    /// Field *syntax* (hex width, lowercase) is a `parse` concern;
    /// `new` takes already-decoded values.
    #[must_use]
    pub const fn new(version: u8, trace_id: u128, parent_span_id: u64, flags: u8) -> Option<Self> {
        if version == 0xff || trace_id == 0 || parent_span_id == 0 {
            return None;
        }
        Some(Self {
            version,
            trace_id,
            parent_span_id,
            flags,
        })
    }

    /// Parses a W3C `traceparent` header value — syntax validation
    /// only, per the module docs. Returns `None` for any malformed or
    /// forbidden input.
    ///
    /// Non-RT only in the trivial sense that it allocates nothing and
    /// is pure; it is safe to call anywhere parsing is allowed, but the
    /// intended use is the non-RT export path.
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let mut parts = input.split('-');
        let version = u8::try_from(parse_hex::<2>(parts.next()?)?).ok()?;
        if version == 0xff {
            return None;
        }
        let trace_id = parse_hex::<32>(parts.next()?)?;
        let parent_span_id = u64::try_from(parse_hex::<16>(parts.next()?)?).ok()?;
        let flags = u8::try_from(parse_hex::<2>(parts.next()?)?).ok()?;
        if trace_id == 0 || parent_span_id == 0 {
            return None;
        }
        // Version `00` is the only defined version and must carry
        // exactly four fields; later versions may append vendor fields,
        // which are ignored (not preserved by `to_traceparent`).
        if version == 0x00 && parts.next().is_some() {
            return None;
        }
        Some(Self {
            version,
            trace_id,
            parent_span_id,
            flags,
        })
    }

    /// The `traceparent` version byte (`00`–`fe`; `ff` is rejected).
    #[must_use]
    pub const fn version(self) -> u8 {
        self.version
    }

    /// The 128-bit trace id.
    #[must_use]
    pub const fn trace_id(self) -> u128 {
        self.trace_id
    }

    /// The 64-bit parent span id.
    #[must_use]
    pub const fn parent_span_id(self) -> u64 {
        self.parent_span_id
    }

    /// The raw trace-flags byte.
    #[must_use]
    pub const fn flags(self) -> u8 {
        self.flags
    }

    /// Whether the sampled flag (`0x01`) is set.
    #[must_use]
    pub const fn is_sampled(self) -> bool {
        self.flags & 0x01 != 0
    }

    /// The trace id as 32 lowercase hex chars — the `trace_id` field
    /// value emitted on `rt_record` events.
    #[must_use]
    pub fn trace_id_hex(self) -> String {
        format!("{:032x}", self.trace_id)
    }

    /// The parent span id as 16 lowercase hex chars — the
    /// `parent_span_id` field value emitted on `rt_record` events.
    #[must_use]
    pub fn parent_span_id_hex(self) -> String {
        format!("{:016x}", self.parent_span_id)
    }

    /// Re-formats the canonical four-field `traceparent` value.
    ///
    /// For versions other than `00`, trailing vendor-specific fields
    /// seen at `parse` time are not preserved — the output is always
    /// the canonical `version-trace-id-parent-id-flags` form.
    #[must_use]
    pub fn to_traceparent(&self) -> String {
        format!(
            "{:02x}-{:032x}-{:016x}-{:02x}",
            self.version, self.trace_id, self.parent_span_id, self.flags
        )
    }
}

/// Strict lowercase-hex check — W3C `traceparent` requires lowercase;
/// uppercase is rejected rather than normalized so malformed input
/// never silently becomes a different trace id.
fn is_lower_hex(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Parses exactly `N` lowercase hex chars into a `u128`. `N <= 32`
/// keeps the value inside `u128`; callers narrow with `try_from`.
fn parse_hex<const N: usize>(s: &str) -> Option<u128> {
    if s.len() != N || !is_lower_hex(s) {
        return None;
    }
    u128::from_str_radix(s, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    #[test]
    fn parses_w3c_example() {
        let ctx = TraceContext::parse(VALID).expect("valid");
        assert_eq!(ctx.version(), 0x00);
        assert_eq!(ctx.trace_id(), 0x4bf9_2f35_77b3_4da6_a3ce_929d_0e0e_4736);
        assert_eq!(ctx.parent_span_id(), 0x00f0_67aa_0ba9_02b7);
        assert_eq!(ctx.flags(), 0x01);
        assert!(ctx.is_sampled());
        assert_eq!(ctx.to_traceparent(), VALID);
    }

    #[test]
    fn rejects_version_ff() {
        assert!(
            TraceContext::parse("ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
                .is_none()
        );
    }

    #[test]
    fn rejects_zero_ids() {
        assert!(
            TraceContext::parse("00-00000000000000000000000000000000-00f067aa0ba902b7-01")
                .is_none()
        );
        assert!(
            TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01")
                .is_none()
        );
    }

    #[test]
    fn rejects_bad_lengths_and_hex() {
        // Short trace id.
        assert!(
            TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e473-00f067aa0ba902b7-01").is_none()
        );
        // Long parent id.
        assert!(
            TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b70-01")
                .is_none()
        );
        // Non-hex character.
        assert!(
            TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e473g-00f067aa0ba902b7-01")
                .is_none()
        );
        // Uppercase hex is not normalized — it is rejected.
        assert!(
            TraceContext::parse("00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-01")
                .is_none()
        );
        // Missing field / empty input.
        assert!(
            TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7").is_none()
        );
        assert!(TraceContext::parse("").is_none());
    }

    #[test]
    fn version_00_rejects_extra_fields() {
        assert!(
            TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01-extra")
                .is_none()
        );
    }

    #[test]
    fn later_versions_read_first_four_fields() {
        let ctx =
            TraceContext::parse("01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-02-vendor")
                .expect("forward-compatible parse");
        assert_eq!(ctx.version(), 0x01);
        assert_eq!(ctx.flags(), 0x02);
        assert!(!ctx.is_sampled()); // bit 0x01 is clear
        // Re-formatting emits the canonical four-field form.
        assert_eq!(
            ctx.to_traceparent(),
            "01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-02"
        );
    }

    #[test]
    fn new_rejects_invalid_combinations() {
        assert!(TraceContext::new(0xff, 1, 1, 0).is_none());
        assert!(TraceContext::new(0x00, 0, 1, 0).is_none());
        assert!(TraceContext::new(0x00, 1, 0, 0).is_none());
        assert!(TraceContext::new(0x00, 1, 1, 0).is_some());
    }

    #[test]
    fn sampled_flag_is_bit_zero_only() {
        let ctx = TraceContext::new(0x00, 1, 1, 0x02).unwrap();
        assert!(!ctx.is_sampled());
        let ctx = TraceContext::new(0x00, 1, 1, 0x03).unwrap();
        assert!(ctx.is_sampled());
    }
}
