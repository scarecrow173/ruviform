//! C6 host API descriptors: the neutral declaration of host bindings
//! a script may call.
//!
//! The contract carries **descriptors only** — no OS/FS/Network/Shell
//! binding implementations, no handler execution seam (deferred to
//! the real-backend slice). [`HostApiSurface::validate_neutral`]
//! rejects binding names in reserved prefixes so the neutral contract
//! cannot smuggle product- or OS-specific surfaces.
//!
//! `effect` is advisory metadata for consumers and analyzers — it is
//! **not** an authorization grant. Every host call is reauthorized by
//! the consumer at call time.

use std::collections::BTreeSet;
use std::fmt;
use std::time::Duration;

use crate::capability::ValueKind;

/// Schema version for [`HostApiSurface`].
pub const HOST_SURFACE_SCHEMA_VERSION: u32 = 1;

/// Reserved binding-name prefixes the neutral contract forbids.
///
/// These name OS/process/environment/system surfaces that belong to a
/// specific backend or consumer, not to the neutral contract.
const FORBIDDEN_PREFIXES: &[&str] = &[
    "os.", "fs.", "net.", "network.", "shell.", "process.", "env.", "sys.", "danger.",
];

/// Advisory effect class of a host binding.
///
/// Informational only — never an authorization decision. The consumer
/// reauthorizes every host call regardless of declared effect.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HostEffect {
    /// No observable side effects (e.g. pure computation, lookups).
    #[default]
    Pure,
    /// Reads external state that may change between calls.
    Query,
    /// Causes side effects (mutation, I/O, calls outward).
    Effect,
}

/// Typed signature of a host binding: argument kinds and return kind
/// in the neutral [`ValueKind`] vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostSignature {
    /// Argument types, in order.
    pub args: Vec<ValueKind>,
    /// Return type.
    pub returns: ValueKind,
}

/// Description of one host binding a session may call.
#[derive(Clone, Debug, PartialEq)]
pub struct HostBinding {
    /// Binding name as seen by scripts (e.g. `"action.invoke"`).
    /// Must not use a reserved OS/FS/Network/Shell prefix.
    pub name: String,
    /// Typed signature.
    pub signature: HostSignature,
    /// Advisory effect class.
    pub effect: HostEffect,
    /// Schema version of this binding description.
    pub schema_version: u32,
    /// Optional timeout hint for the call, if the backend supports it.
    pub timeout_hint: Option<Duration>,
}

/// Errors from [`HostApiSurface`] construction/validation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostSurfaceError {
    /// Binding name is empty.
    EmptyBindingName,
    /// Binding name uses a reserved OS/FS/Network/Shell prefix.
    ForbiddenBindingName {
        /// The offending name.
        name: String,
    },
    /// Two bindings share the same name.
    DuplicateBindingName {
        /// The duplicated name.
        name: String,
    },
}

impl fmt::Display for HostSurfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBindingName => f.write_str("host binding name is empty"),
            Self::ForbiddenBindingName { name } => {
                write!(f, "host binding name {name:?} uses a reserved prefix")
            }
            Self::DuplicateBindingName { name } => {
                write!(f, "duplicate host binding name {name:?}")
            }
        }
    }
}

impl std::error::Error for HostSurfaceError {}

/// The declared host API surface for a session: a set of neutral
/// binding descriptors plus a schema version.
///
/// Construct with [`HostApiSurface::new`], which validates neutrality
/// and uniqueness; [`HostApiSurface::validate_neutral`] re-checks an
/// existing (possibly mutated) surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HostApiSurface {
    /// Declared bindings.
    pub bindings: Vec<HostBinding>,
    /// Schema version of this surface envelope.
    pub schema_version: u32,
}

impl HostApiSurface {
    /// An empty surface — no host bindings.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            bindings: Vec::new(),
            schema_version: HOST_SURFACE_SCHEMA_VERSION,
        }
    }

    /// Creates a surface, validating binding names and uniqueness.
    ///
    /// # Errors
    ///
    /// Returns [`HostSurfaceError`] on empty, forbidden, or duplicate
    /// binding names.
    pub fn new(bindings: Vec<HostBinding>) -> Result<Self, HostSurfaceError> {
        let surface = Self {
            bindings,
            schema_version: HOST_SURFACE_SCHEMA_VERSION,
        };
        surface.validate_neutral()?;
        Ok(surface)
    }

    /// Validates that every binding name is neutral (non-empty, no
    /// reserved prefix) and unique.
    ///
    /// # Errors
    ///
    /// Returns [`HostSurfaceError`] describing the first violation.
    pub fn validate_neutral(&self) -> Result<(), HostSurfaceError> {
        let mut seen = BTreeSet::new();
        for binding in &self.bindings {
            let name = binding.name.as_str();
            if name.is_empty() {
                return Err(HostSurfaceError::EmptyBindingName);
            }
            let lower = name.to_ascii_lowercase();
            if FORBIDDEN_PREFIXES.iter().any(|p| lower.starts_with(p)) {
                return Err(HostSurfaceError::ForbiddenBindingName {
                    name: name.to_string(),
                });
            }
            if !seen.insert(name) {
                return Err(HostSurfaceError::DuplicateBindingName {
                    name: name.to_string(),
                });
            }
        }
        Ok(())
    }
}
