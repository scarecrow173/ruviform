//! Fixed-size record vocabulary for the RT record bridge — Issue #280
//! (OBS-05).
//!
//! - [`clock`]: [`ClockContext`], the bounded clock-domain stamp a record
//!   carries (domain + epoch + position + timebase + provenance).
//! - [`correlation`]: fixed-size correlation tokens/links and the non-RT
//!   [`CorrelationBook`] that classifies them.
//! - [`event`]: the generic [`EventRecord`] — [`RecordHeader`] metadata
//!   plus a `u64` payload and reserved space.

mod clock;
mod correlation;
mod event;

pub use clock::ClockContext;
pub use correlation::{
    CorrelationBook, CorrelationLink, CorrelationStatus, CorrelationToken, UnlinkedReason,
};
pub use event::{EventId, EventRecord, RecordHeader};
