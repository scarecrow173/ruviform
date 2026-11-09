//! Finding vocabulary — stable, classified diagnostic findings that
//! cite [`EvidenceRef`]s instead of asserting causes.
//!
//! A [`DiagnosticFinding`] pairs a domain-owned [`DiagnosticCode`]
//! (stable `name` + `schema_version`) with a [`FindingSeverity`], a
//! [`FindingCertainty`] (`Confirmed` / `Inferred` / `NotEvaluable`),
//! a [`FindingStatus`] lifetime, and the [`EvidenceRef`]s that explain
//! it. Codes are deliberately *not* a central enum: domain `Error`
//! types and existing registries (e.g. the plugin host's
//! `DiagRegistry`) remain the authoritative failure channel, and
//! diagnostics only references them — never duplicates them.
//!
//! # Definitive vs non-definitive
//!
//! [`FindingCertainty::Confirmed`] is reserved for findings grounded
//! in observed facts plus domain-declared rules (e.g. a threshold
//! evaluation). [`FindingCertainty::Inferred`] requires a non-empty
//! `evidence_refs` basis and never asserts a cause;
//! [`FindingCertainty::NotEvaluable`] records that assessment was
//! impossible (missing/stale/unobservable input). Unsupported causal
//! claims cannot be expressed as `Confirmed`.

use std::error::Error;
use std::fmt;
use std::time::Instant;

use crate::source::{Confidentiality, SourceIdentity};

use super::evidence::EvidenceRef;

/// Stable identifier of a diagnostic finding — domain-owned, never a
/// central enum.
///
/// `name` is a `&'static str` bound by the declaring domain;
/// `schema_version` versions the code's meaning/payload contract so a
/// meaning change is explicit rather than a silent reuse. `Ord` is
/// derived so reports can sort deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiagnosticCode {
    /// Stable code name declared by the domain.
    pub name: &'static str,
    /// Schema version of this code's meaning.
    pub schema_version: u16,
}

/// Attention level of a [`DiagnosticFinding`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FindingSeverity {
    /// Informational — recorded for context.
    Info,
    /// Warning — degraded condition worth attention.
    Warning,
    /// Critical — severe condition.
    Critical,
}

impl FindingSeverity {
    /// Ordering rank — higher is more severe. Crate-internal so
    /// [`#[non_exhaustive]`](Self) variants added later force a
    /// compile-visible review inside this crate.
    pub(crate) const fn rank(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Critical => 2,
        }
    }
}

/// How certain a [`DiagnosticFinding`] is.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FindingCertainty {
    /// Verified against observed facts and declared rules.
    Confirmed,
    /// Reasoned from evidence — non-definitive by construction.
    Inferred,
    /// Assessment was not possible (missing/stale/unobservable input).
    NotEvaluable,
}

impl FindingCertainty {
    /// Certainty rank — higher is more certain. Used when merging
    /// repeated observations of the same open finding.
    pub(crate) const fn rank(self) -> u8 {
        match self {
            Self::NotEvaluable => 0,
            Self::Inferred => 1,
            Self::Confirmed => 2,
        }
    }
}

/// Lifetime state of a [`DiagnosticFinding`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FindingStatus {
    /// Currently observed.
    Open,
    /// Condition cleared — retained for report history.
    Resolved,
}

/// One diagnostic finding.
///
/// Fields are private so the [`FindingCertainty::Inferred`] basis
/// invariant cannot be bypassed after construction; read through the
/// accessors. `summary` is free text for humans — it is `String`
/// (non-RT construction only) and deliberately has no dedicated
/// fields for paths, byte blobs, tokens, or stderr.
#[derive(Debug, Clone)]
pub struct DiagnosticFinding {
    code: DiagnosticCode,
    domain: &'static str,
    source: Option<SourceIdentity>,
    severity: FindingSeverity,
    certainty: FindingCertainty,
    status: FindingStatus,
    confidentiality: Confidentiality,
    summary: String,
    evidence_refs: Box<[EvidenceRef]>,
    first_seen_at: Instant,
    last_seen_at: Instant,
}

impl DiagnosticFinding {
    /// Records a new open finding.
    ///
    /// `certainty` + `evidence_refs` enforce the definitiveness
    /// contract: [`FindingCertainty::Inferred`] requires a non-empty
    /// `evidence_refs`.
    ///
    /// # Errors
    ///
    /// Returns [`FindingError::MissingBasis`] when `certainty` is
    /// `Inferred` and `evidence_refs` is empty.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        code: DiagnosticCode,
        domain: &'static str,
        source: Option<SourceIdentity>,
        severity: FindingSeverity,
        certainty: FindingCertainty,
        confidentiality: Confidentiality,
        summary: impl Into<String>,
        evidence_refs: impl Into<Box<[EvidenceRef]>>,
        now: Instant,
    ) -> Result<Self, FindingError> {
        let evidence_refs: Box<[EvidenceRef]> = evidence_refs.into();
        if certainty == FindingCertainty::Inferred && evidence_refs.is_empty() {
            return Err(FindingError::MissingBasis);
        }
        Ok(Self {
            code,
            domain,
            source,
            severity,
            certainty,
            status: FindingStatus::Open,
            confidentiality,
            summary: summary.into(),
            evidence_refs,
            first_seen_at: now,
            last_seen_at: now,
        })
    }

    /// The stable code.
    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// The domain that owns this finding.
    #[must_use]
    pub const fn domain(&self) -> &'static str {
        self.domain
    }

    /// The subject source, if bound.
    #[must_use]
    pub const fn source(&self) -> Option<SourceIdentity> {
        self.source
    }

    /// Attention level.
    #[must_use]
    pub const fn severity(&self) -> FindingSeverity {
        self.severity
    }

    /// Certainty classification.
    #[must_use]
    pub const fn certainty(&self) -> FindingCertainty {
        self.certainty
    }

    /// Lifetime state.
    #[must_use]
    pub const fn status(&self) -> FindingStatus {
        self.status
    }

    /// Confidentiality classification — drives report disclosure.
    #[must_use]
    pub const fn confidentiality(&self) -> Confidentiality {
        self.confidentiality
    }

    /// Human-readable summary.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Evidence references explaining this finding.
    #[must_use]
    pub fn evidence_refs(&self) -> &[EvidenceRef] {
        &self.evidence_refs
    }

    /// When the finding was first observed.
    #[must_use]
    pub const fn first_seen_at(&self) -> Instant {
        self.first_seen_at
    }

    /// When the finding was last observed.
    #[must_use]
    pub const fn last_seen_at(&self) -> Instant {
        self.last_seen_at
    }

    /// Marks the finding resolved at `at`.
    pub fn resolve(&mut self, at: Instant) {
        self.status = FindingStatus::Resolved;
        self.last_seen_at = at;
    }

    /// Merges a repeated observation into this open finding:
    /// keeps `first_seen_at`, takes the worse severity, the more
    /// certain certainty, the stricter confidentiality, and replaces
    /// summary/refs/source/last-seen with the latest observation.
    pub(crate) fn observe_again(&mut self, newer: &DiagnosticFinding) {
        if newer.severity.rank() > self.severity.rank() {
            self.severity = newer.severity;
        }
        if newer.certainty.rank() > self.certainty.rank() {
            self.certainty = newer.certainty;
        }
        if confidentiality_rank(newer.confidentiality) > confidentiality_rank(self.confidentiality)
        {
            self.confidentiality = newer.confidentiality;
        }
        self.source = newer.source;
        self.summary.clone_from(&newer.summary);
        self.evidence_refs.clone_from(&newer.evidence_refs);
        self.last_seen_at = newer.last_seen_at;
    }
}

/// Ordering rank of [`Confidentiality`] — higher is more restrictive.
/// Shared with the report disclosure filter.
pub(crate) const fn confidentiality_rank(c: Confidentiality) -> u8 {
    match c {
        Confidentiality::Public => 0,
        Confidentiality::Internal => 1,
        Confidentiality::Confidential => 2,
    }
}

/// Errors constructing or recording a [`DiagnosticFinding`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingError {
    /// `Inferred` certainty was supplied with an empty evidence basis.
    MissingBasis,
    /// The engine's finding retention is disabled
    /// (`DiagnosticsConfig::max_findings == 0`).
    RetentionDisabled,
}

impl fmt::Display for FindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBasis => {
                write!(f, "inferred findings require a non-empty evidence basis")
            }
            Self::RetentionDisabled => write!(f, "finding retention is disabled"),
        }
    }
}

impl Error for FindingError {}
