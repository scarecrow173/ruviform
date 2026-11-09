//! Inspection budget — the bounded cost contract every
//! [`DebugInspector::inspect`](crate::debug::DebugInspector::inspect)
//! call runs under (Issue #292, OBS-10).
//!
//! [`InspectionBudget`] bounds three independent dimensions:
//!
//! - `max_items` — maximum per-item entries collected **per section**.
//!   `0` disables per-item entry collection entirely: sections still
//!   report their aggregate counts (series totals, validity
//!   histograms, deck/probe counts) but `entries` stay empty. This is
//!   a deliberate mode, not an overflow — it is distinct from
//!   `InspectionDetail::Summary`, which is the *requested* detail
//!   level rather than a budget bound.
//! - `max_bytes` — approximate byte budget across all collected
//!   entries, estimated as `size_of` plus payload lengths (documented
//!   as approximate, not exact — the same convention as
//!   [`ReportLimits`](crate::diagnostics::ReportLimits)).
//!   `0` means unlimited. An entry that does not fit the remaining
//!   budget is skipped and counted; later smaller entries may still
//!   fit. The convention matches
//!   [`ReportLimits`](crate::diagnostics::ReportLimits).
//! - `timeout` — whole-inspect deadline measured from inspect start.
//!   Checked before each section and while collecting entries; on
//!   expiry, sections not yet started report
//!   [`SectionStatus::Unavailable`](crate::debug::SectionStatus)
//!   with [`UnavailableReason::DeadlineExceeded`](crate::debug::UnavailableReason),
//!   and a section interrupted mid-collection reports
//!   [`SectionStatus::Truncated`](crate::debug::SectionStatus) with
//!   `deadline_exceeded` set. `None` disables the bound.
//!
//! Every bound that actually removes data is reflected in
//! [`InspectionTruncation`](crate::debug::InspectionTruncation) —
//! truncation is always observable, never silent.

use std::time::Duration;

/// Bounds applied to one [`inspect`](crate::debug::DebugInspector::inspect)
/// call. See the module docs for the per-field `0`/`None` semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectionBudget {
    /// Maximum per-item entries collected per section; `0` disables
    /// entry collection (aggregates still reported).
    pub max_items: usize,
    /// Approximate byte budget across all collected entries;
    /// `0` = unlimited.
    pub max_bytes: usize,
    /// Whole-inspect deadline; `None` = unlimited.
    pub timeout: Option<Duration>,
}

impl Default for InspectionBudget {
    fn default() -> Self {
        Self {
            max_items: 64,
            max_bytes: 65_536,
            timeout: None,
        }
    }
}
