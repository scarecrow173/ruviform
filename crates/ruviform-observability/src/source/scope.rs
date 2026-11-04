//! Observation scope — where a metric source belongs in the running
//! system, expressed as a closed enum so scope cardinality is
//! statically bounded.

/// The scope a metric source belongs to.
///
/// A closed enum bounds scope cardinality by construction; adding a
/// new scope kind is a deliberate, compile-visible contract change.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservationScope {
    /// Process-wide singleton source (e.g. a global submit queue).
    Process,
    /// Per domain instance (plugin instance, graph node, device).
    Instance,
    /// Per run/session incarnation (reset at session boundaries).
    Session,
}
