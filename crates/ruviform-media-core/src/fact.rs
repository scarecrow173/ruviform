//! Availability-aware fact values: [`Fact`] pairs a value with the reason
//! it could not be supplied, so a missing fact is never collapsed into a
//! fabricated default such as `0`.

use core::fmt;

/// An observed fact, or the reason it could not be supplied.
///
/// Observation contracts distinguish *three* absence causes that `Option`
/// collapses into one `None`:
///
/// - [`Fact::NotDeclared`] — the container/stream does not carry this
///   fact (a normal state, not an error),
/// - [`Fact::Unsupported`] — the backend cannot supply the fact,
/// - [`Fact::Malformed`] — the fact was declared but is not usable
///   (corrupt, out of range, or unrepresentable).
///
/// `Fact` is `Copy` whenever `T` is `Copy`, so it can be embedded in
/// RT-suitable descriptors.
///
/// **RT contract:** RT-suitable — `Copy` when `T: Copy`, allocation-free.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::Fact;
///
/// let known: Fact<u32> = Fact::known(48_000);
/// let missing: Fact<u32> = Fact::NotDeclared;
/// assert_eq!(known.get(), Some(&48_000));
/// assert!(missing.is_unavailable());
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Fact<T> {
    /// The fact was observed and carries a value.
    Known(T),
    /// The container/stream does not declare this fact. Not an error —
    /// most containers simply omit facts they do not model.
    NotDeclared,
    /// The backend cannot supply this fact (capability limitation).
    Unsupported,
    /// The fact was declared but is unusable (corrupt, out of range, or
    /// unrepresentable at the boundary).
    Malformed,
}

impl<T> Fact<T> {
    /// Wraps a known value.
    pub const fn known(value: T) -> Self {
        Self::Known(value)
    }

    /// Maps an `Option` into a [`Fact`]: `Some` becomes
    /// [`Fact::Known`], `None` becomes [`Fact::NotDeclared`]. Use
    /// [`Fact::Unsupported`] / [`Fact::Malformed`] explicitly when the
    /// stronger classification is known.
    pub fn from_option(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Known(value),
            None => Self::NotDeclared,
        }
    }

    /// Returns `true` when the fact carries a value.
    pub const fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    /// Returns `true` when the fact is absent for any reason.
    pub const fn is_unavailable(&self) -> bool {
        !self.is_known()
    }

    /// Returns the value reference when known, else `None`. Collapses the
    /// unavailability reason — keep the [`Fact`] itself when the reason
    /// matters.
    pub const fn get(&self) -> Option<&T> {
        match self {
            Self::Known(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value when known, else `None`. Same caveat as
    /// [`Fact::get`].
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Known(value) => Some(value),
            _ => None,
        }
    }

    /// Maps the contained value, preserving the absence classification.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Fact<U> {
        match self {
            Self::Known(value) => Fact::Known(f(value)),
            Self::NotDeclared => Fact::NotDeclared,
            Self::Unsupported => Fact::Unsupported,
            Self::Malformed => Fact::Malformed,
        }
    }
}

impl<T: Copy> Fact<T> {
    /// Returns the value when known, else `None`.
    pub const fn value(&self) -> Option<T> {
        match self {
            Self::Known(value) => Some(*value),
            _ => None,
        }
    }
}

impl<T> Default for Fact<T> {
    /// The empty default is `NotDeclared` — a fact that has not been
    /// supplied and was never claimed.
    fn default() -> Self {
        Self::NotDeclared
    }
}

impl<T: fmt::Display> fmt::Display for Fact<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Known(value) => write!(f, "{value}"),
            Self::NotDeclared => f.write_str("not declared"),
            Self::Unsupported => f.write_str("unsupported"),
            Self::Malformed => f.write_str("malformed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_value_round_trips() {
        let fact = Fact::known(42_u32);
        assert!(fact.is_known());
        assert!(!fact.is_unavailable());
        assert_eq!(fact.get(), Some(&42));
        assert_eq!(fact.value(), Some(42));
        assert_eq!(fact.into_option(), Some(42));
        assert_eq!(fact.map(|v| v + 1), Fact::Known(43));
    }

    #[test]
    fn absence_kinds_are_distinct_and_preserved_by_map() {
        for fact in [
            Fact::<u32>::NotDeclared,
            Fact::<u32>::Unsupported,
            Fact::<u32>::Malformed,
        ] {
            assert!(!fact.is_known());
            assert!(fact.is_unavailable());
            assert_eq!(fact.get(), None);
            assert_eq!(fact.value(), None);
            assert_eq!(fact.map(|v| v + 1), fact);
        }
        assert_ne!(
            Fact::<u32>::NotDeclared,
            Fact::<u32>::Unsupported,
            "absence reasons must not collapse"
        );
    }

    #[test]
    fn from_option_maps_none_to_not_declared() {
        assert_eq!(Fact::from_option(Some(1_u32)), Fact::Known(1));
        assert_eq!(Fact::from_option(None::<u32>), Fact::NotDeclared);
    }

    #[test]
    fn display_distinguishes_known_from_absent() {
        assert_eq!(Fact::known(7_u32).to_string(), "7");
        assert_eq!(Fact::<u32>::NotDeclared.to_string(), "not declared");
        assert_eq!(Fact::<u32>::Unsupported.to_string(), "unsupported");
        assert_eq!(Fact::<u32>::Malformed.to_string(), "malformed");
    }

    /// `Fact<T: Copy>` stays `Copy` so RT-suitable descriptors may embed
    /// it without losing their own `Copy` contract.
    #[test]
    fn fact_of_copy_is_copy() {
        let fact = Fact::known(1_u32);
        let copy = fact;
        assert_eq!(fact, copy);
    }
}
