//! C1 identifiers/registry: backend descriptors, consumer
//! requirements, and the fail-closed [`ScriptBackendRegistry`].
//!
//! Resolution is **fail-closed**: unknown languages/backends, zero
//! candidates, version incompatibility, missing capabilities, and
//! ambiguity all produce typed errors. There is no nearest-match
//! fallback, no lazy loading, and no runtime feature exclusion.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::CONTRACT_VERSION;
use crate::capability::BackendCapabilities;
use crate::id::{CapabilityId, ScriptBackendId, ScriptBackendVersion, ScriptLanguageId};
use crate::session::ScriptBackend;
use crate::value::ArtifactKind;

/// Static description of a registered [`ScriptBackend`].
#[derive(Clone, Debug, PartialEq)]
pub struct BackendDescriptor {
    /// Unique backend identifier.
    pub backend_id: ScriptBackendId,
    /// Language this backend implements.
    pub language: ScriptLanguageId,
    /// Implementation/contract version pair.
    pub version: ScriptBackendVersion,
    /// Advertised capabilities.
    pub capabilities: BackendCapabilities,
}

/// What a consumer needs from a backend.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptRequirement {
    /// Required language.
    pub language: ScriptLanguageId,
    /// Optional specific backend hint. `None` = any backend for the
    /// language.
    pub backend: Option<ScriptBackendId>,
    /// Capability IDs that must be advertised.
    pub required_capabilities: BTreeSet<CapabilityId>,
    /// Minimum acceptable `contract_version`.
    pub min_contract_version: u32,
    /// Artifact kinds the backend must be able to produce.
    pub artifact_kinds: BTreeSet<ArtifactKind>,
}

/// Errors from [`ScriptBackendRegistry::register`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistryError {
    /// A backend with this ID is already registered.
    DuplicateBackend {
        /// The duplicated backend ID.
        backend_id: ScriptBackendId,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateBackend { backend_id } => {
                write!(f, "duplicate backend id: {backend_id}")
            }
        }
    }
}

impl std::error::Error for RegistryError {}

/// Why a [`ScriptRequirement`] matched no backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResolveErrorReason {
    /// No registered backend implements the required language.
    UnknownLanguage,
    /// The hinted backend ID is not registered.
    UnknownBackend,
    /// Language/backend filters matched nothing (e.g. a known backend
    /// hint for a different language).
    NoCandidates,
    /// Candidates exist but none speaks a compatible contract version.
    UnsupportedContractVersion,
    /// Candidates exist but none advertises the required capabilities
    /// or artifact kinds.
    MissingCapability,
}

/// Fail-closed resolution errors.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ResolveError {
    /// No backend satisfied the requirement.
    NoMatchingBackend {
        /// Distinguishing reason.
        reason: ResolveErrorReason,
        /// Human-readable detail.
        detail: String,
    },
    /// More than one backend satisfied the requirement — the consumer
    /// must disambiguate (the registry never picks arbitrarily).
    AmbiguousBackend {
        /// IDs of the matching candidates, sorted for determinism.
        candidates: Vec<ScriptBackendId>,
    },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMatchingBackend { reason, detail } => {
                write!(f, "no matching backend ({reason:?}): {detail}")
            }
            Self::AmbiguousBackend { candidates } => {
                write!(f, "ambiguous backend candidates: {candidates:?}")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// Registry of [`ScriptBackend`] implementations.
///
/// Backends are registered explicitly by the consumer — the registry
/// ships empty and contains no fake or default backend.
#[derive(Default)]
pub struct ScriptBackendRegistry {
    backends: BTreeMap<ScriptBackendId, Box<dyn ScriptBackend>>,
}

impl ScriptBackendRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `backend` under its descriptor's `backend_id`.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::DuplicateBackend`] if the ID is taken.
    pub fn register(&mut self, backend: Box<dyn ScriptBackend>) -> Result<(), RegistryError> {
        let id = backend.descriptor().backend_id.clone();
        if self.backends.contains_key(&id) {
            return Err(RegistryError::DuplicateBackend { backend_id: id });
        }
        self.backends.insert(id, backend);
        Ok(())
    }

    /// Resolves `requirement` to exactly one backend, fail-closed.
    ///
    /// Filter order: language/backend identity → candidate existence →
    /// contract-version compatibility → capability/artifact coverage →
    /// uniqueness. See the module docs and
    /// `docs/designs/issue-558-script-contract-design.md` for the
    /// fail-closed table.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError::NoMatchingBackend`] with a distinguishing
    /// [`ResolveErrorReason`], or [`ResolveError::AmbiguousBackend`]
    /// when multiple backends satisfy the requirement.
    pub fn resolve(
        &self,
        requirement: &ScriptRequirement,
    ) -> Result<&dyn ScriptBackend, ResolveError> {
        // Identity checks first so the reason distinguishes "unknown
        // language/backend" from "known IDs that contradict".
        let language_known = self
            .backends
            .values()
            .any(|b| b.descriptor().language == requirement.language);
        if !language_known {
            return Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::UnknownLanguage,
                detail: format!(
                    "no registered backend implements language {:?}",
                    requirement.language
                ),
            });
        }
        if let Some(backend_id) = &requirement.backend
            && !self.backends.contains_key(backend_id)
        {
            return Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::UnknownBackend,
                detail: format!("backend {backend_id} is not registered"),
            });
        }

        let mut candidates: Vec<&dyn ScriptBackend> = self
            .backends
            .values()
            .map(Box::as_ref)
            .filter(|b| {
                let d = b.descriptor();
                d.language == requirement.language
                    && requirement
                        .backend
                        .as_ref()
                        .is_none_or(|id| &d.backend_id == id)
            })
            .collect();

        if candidates.is_empty() {
            return Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::NoCandidates,
                detail: format!(
                    "language {:?} and backend hint are known but no backend satisfies both",
                    requirement.language
                ),
            });
        }

        candidates.retain(|b| {
            let v = &b.descriptor().version;
            v.contract_version == CONTRACT_VERSION
                && v.contract_version >= requirement.min_contract_version
        });
        if candidates.is_empty() {
            return Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::UnsupportedContractVersion,
                detail: format!(
                    "no candidate speaks contract version {CONTRACT_VERSION} with min {}",
                    requirement.min_contract_version
                ),
            });
        }

        candidates.retain(|b| {
            let caps = &b.descriptor().capabilities;
            caps.supports_capabilities(&requirement.required_capabilities)
                && requirement
                    .artifact_kinds
                    .iter()
                    .all(|kind| caps.artifact_kinds.contains(kind))
        });
        if candidates.is_empty() {
            return Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::MissingCapability,
                detail: "no candidate advertises the required capabilities/artifact kinds"
                    .to_string(),
            });
        }

        match candidates.as_slice() {
            [single] => Ok(*single),
            _ => Err(ResolveError::AmbiguousBackend {
                candidates: candidates
                    .iter()
                    .map(|b| b.descriptor().backend_id.clone())
                    .collect(),
            }),
        }
    }

    /// Lists all registered backends in backend-ID order.
    #[must_use]
    pub fn list(&self) -> Vec<&dyn ScriptBackend> {
        self.backends.values().map(Box::as_ref).collect()
    }

    /// Number of registered backends.
    #[must_use]
    pub fn len(&self) -> usize {
        self.backends.len()
    }

    /// Returns `true` if no backends are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }
}
