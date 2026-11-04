//! Source identity — the registrar-assigned, registration-unique
//! identity of an observation source.
//!
//! A [`SourceId`] uniquely identifies one observation *registration*.
//! It is deliberately not a domain instance ID, not a graph revision,
//! and not interchangeable with a [`SourceGeneration`]: re-minting the
//! same logical source produces a new registration whose identity
//! differs in `id` and/or `generation`.

use std::fmt;
use std::num::NonZeroU64;

/// Unique identifier of one observation registration, minted by the
/// registrar.
///
/// `SourceId` is never zero and is never reused while a registrar is
/// alive — re-registrations get a fresh `id` and/or a bumped
/// [`SourceGeneration`]. It must not be conflated with domain
/// instance IDs, graph revisions, or generation/re-mint counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceId(NonZeroU64);

impl SourceId {
    /// Wraps `raw` as a [`SourceId`], or `None` when `raw == 0`.
    ///
    /// Intended for registrar implementations (Issue #289's
    /// registry); domains receive `SourceId` values only through
    /// [`RegistrationLease`](crate::source::RegistrationLease).
    #[must_use]
    pub const fn new(raw: u64) -> Option<Self> {
        match NonZeroU64::new(raw) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// The raw 64-bit value (always non-zero).
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Generation of a logical source — distinguishes re-mints
/// (re-registrations) of what the domain considers "the same" source
/// from each other and from a fresh [`SourceId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceGeneration(u64);

impl SourceGeneration {
    /// Wraps `raw` as a [`SourceGeneration`]. Intended for registrar
    /// implementations.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw 64-bit value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SourceGeneration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The registration-unique identity pair assigned by the registrar:
/// a unique [`SourceId`] plus the [`SourceGeneration`] of the logical
/// source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceIdentity {
    id: SourceId,
    generation: SourceGeneration,
}

impl SourceIdentity {
    /// Pairs `id` and `generation` into a [`SourceIdentity`]. Intended
    /// for registrar implementations.
    #[must_use]
    pub const fn new(id: SourceId, generation: SourceGeneration) -> Self {
        Self { id, generation }
    }

    /// The unique registration id.
    #[must_use]
    pub const fn id(self) -> SourceId {
        self.id
    }

    /// The logical-source generation of this registration.
    #[must_use]
    pub const fn generation(self) -> SourceGeneration {
        self.generation
    }
}

impl fmt::Display for SourceIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.id, self.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_id_is_never_zero() {
        assert!(SourceId::new(0).is_none());
        assert_eq!(SourceId::new(7).unwrap().get(), 7);
    }

    #[test]
    fn identity_pairs_id_and_generation() {
        let id = SourceId::new(3).unwrap();
        let ident = SourceIdentity::new(id, SourceGeneration::new(2));
        assert_eq!(ident.id(), id);
        assert_eq!(ident.generation().get(), 2);
        assert_eq!(ident.to_string(), "3#2");
    }
}
