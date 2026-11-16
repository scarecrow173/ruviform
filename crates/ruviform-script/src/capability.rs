//! Capability advertisement: what a backend or host binding can do,
//! expressed in the neutral vocabulary.

use std::collections::BTreeSet;

use crate::budget::CancelGranularity;
use crate::id::CapabilityId;
use crate::value::ArtifactKind;

/// Neutral type vocabulary for host binding signatures.
///
/// Describes the shape of host call arguments and return values
/// without exposing any vendor or product type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueKind {
    /// Boolean.
    Bool,
    /// Integer (contract `Integer` — strict `i64` semantics).
    Integer,
    /// Floating-point number.
    Number,
    /// String.
    String,
    /// Raw bytes.
    Bytes,
    /// Array of any value kind.
    Array,
    /// Record of any value kind.
    Record,
    /// Domain-qualified time value.
    Time,
    /// Domain-qualified resource reference.
    ResourceId,
    /// Any value kind (escape hatch — prefer concrete kinds).
    #[default]
    Any,
}

/// Capabilities a [`crate::ScriptBackend`] advertises in its
/// [`crate::BackendDescriptor`].
///
/// Used by [`crate::ScriptBackendRegistry::resolve`] to filter
/// candidates: a requirement's `artifact_kinds` must be a subset of
/// `artifact_kinds`, and `required_capabilities` must be a subset of
/// `capability_ids`.
///
/// The boolean flags describe session-placement and isolation
/// properties, not authorization: a consumer still reauthorizes every
/// host call and enforces its own policy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "capability advertisement is inherently a set of independent boolean flags"
)]
pub struct BackendCapabilities {
    /// Artifact kinds this backend can produce.
    pub artifact_kinds: BTreeSet<ArtifactKind>,
    /// Well-known and backend-defined capability IDs this backend
    /// supports (e.g. `"session.evaluate"`, `"host.call"`). Matched
    /// against [`crate::ScriptRequirement::required_capabilities`].
    pub capability_ids: BTreeSet<CapabilityId>,
    /// Sessions must be opened and evaluated on the owning thread.
    pub owner_thread_only: bool,
    /// Session objects may be sent across threads.
    pub send: bool,
    /// Session objects may be shared across threads (`Sync`).
    pub sync: bool,
    /// The backend shares VM state across sessions of the same
    /// backend instance — consumers requiring isolation
    /// (F-16 `IsolationViolation`) must not select such backends.
    pub shared_vm_state: bool,
    /// How promptly the backend observes [`crate::CancelToken`].
    pub cancel_granularity: CancelGranularity,
}

impl BackendCapabilities {
    /// Returns `true` if `required` capability IDs are all advertised.
    #[must_use]
    pub fn supports_capabilities(&self, required: &BTreeSet<CapabilityId>) -> bool {
        required.is_subset(&self.capability_ids)
    }
}
