//! Opaque identifier newtypes and version carriers for the neutral
//! Script/Analyzer contract.
//!
//! All IDs are opaque strings: they carry no semantics beyond equality
//! and ordering, and no vendor or product type leaks into them.

use std::fmt;

/// Language identifier for a script source (e.g. `"luau"`, `"rhai"`).
///
/// The neutral contract does not enumerate languages; consumers and
/// backends agree on the vocabulary out of band.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptLanguageId(String);

impl ScriptLanguageId {
    /// Creates an ID from its opaque string form.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the opaque string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ScriptLanguageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Backend identifier for a [`crate::ScriptBackend`] implementation.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptBackendId(String);

impl ScriptBackendId {
    /// Creates an ID from its opaque string form.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the opaque string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ScriptBackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Analyzer identifier for a [`crate::ScriptAnalyzer`] implementation.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnalyzerId(String);

impl AnalyzerId {
    /// Creates an ID from its opaque string form.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the opaque string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AnalyzerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Capability identifier advertised by a backend or analyzer
/// descriptor and required by a consumer requirement.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(String);

impl CapabilityId {
    /// Creates an ID from its opaque string form.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the opaque string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Domain identifier qualifying a [`crate::ScriptValue::Time`] value's time
/// domain (e.g. `"audio.transport"`, `"project.tick"`).
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeDomainId(String);

impl TimeDomainId {
    /// Creates an ID from its opaque string form.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the opaque string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TimeDomainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Domain identifier qualifying a [`crate::ScriptValue::ResourceId`] value's
/// resource domain (e.g. `"track"`, `"plugin.instance"`).
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceDomainId(String);

impl ResourceDomainId {
    /// Creates an ID from its opaque string form.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the opaque string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceDomainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Session identifier assigned by a backend when a session is opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(pub u64);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "session:{}", self.0)
    }
}

/// Version pair advertised by backend and analyzer descriptors.
///
/// `contract_version` is the RF-SCRIPT-00 contract version this
/// implementation speaks; it must equal
/// [`crate::CONTRACT_VERSION`] for registry resolution to succeed.
/// `impl_version` is an opaque implementation version string (semver
/// or vendor-specific) carried for diagnostics and provenance only —
/// resolution never compares it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptBackendVersion {
    /// Opaque implementation version (semver or vendor-specific).
    pub impl_version: String,
    /// RF-SCRIPT-00 contract version implemented.
    pub contract_version: u32,
}

impl ScriptBackendVersion {
    /// Creates a version pair.
    #[must_use]
    pub fn new(impl_version: impl Into<String>, contract_version: u32) -> Self {
        Self {
            impl_version: impl_version.into(),
            contract_version,
        }
    }
}
