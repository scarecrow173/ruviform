//! Evidence vocabulary — typed, classified observations that separate
//! verifiable facts from derived, reported, and inferred claims.
//!
//! Every [`Evidence`] records *what kind of claim* it is
//! ([`EvidenceKind`]), *which registered source* it concerns
//! ([`SourceIdentity`] — id **and** generation), *when* it was observed
//! ([`EvidenceTime`]: host monotonic [`Instant`] or a domain
//! [`ClockContext`]), *how trustworthy* it is
//! ([`MeasurementValidity`]), *how confidential* it is
//! ([`Confidentiality`]), and — for [`EvidenceKind::Derived`] /
//! [`EvidenceKind::Inferred`] — the non-empty [`EvidenceRef`] basis it
//! rests on. Observed is not automatically complete, exact, or true:
//! validity and confidentiality are carried, never assumed.
//!
//! # Non-leakage
//!
//! [`EvidencePayload`] is a closed vocabulary — it cannot carry raw
//! byte buffers, filesystem paths, prompt text, credentials/tokens, or
//! child stderr. Those must never be attached to evidence; export
//! decisions stay with the host's authorization boundary (modeled as
//! [`Confidentiality`] plus the report disclosure policy in
//! [`crate::diagnostics::report`]).
//!
//! # Lifetime
//!
//! [`EvidenceRef`]s are minted once by the engine and never reused.
//! Evidence recorded against a [`SourceIdentity`] keeps that identity
//! forever — when the source retires, is removed, or re-registers under
//! a new generation, the engine marks the entry `superseded` rather
//! than rebinding it to a different source.
//!
//! [`Instant`]: std::time::Instant
//! [`ClockContext`]: crate::record::ClockContext

use std::error::Error;
use std::fmt;
use std::time::Instant;

use crate::metrics::{Measurement, MeasurementValidity, MetricValue};
use crate::record::ClockContext;
use crate::source::{Confidentiality, SourceIdentity};

/// Immutable reference to one stored [`Evidence`] entry, minted by the
/// diagnostics engine.
///
/// References are monotonically increasing and never reused — a
/// [`DiagnosticFinding`](crate::diagnostics::DiagnosticFinding) citing
/// a ref stays explainable for the engine's retention lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EvidenceRef(u64);

impl EvidenceRef {
    /// The raw reference value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Mints the next reference — engine-internal.
    pub(crate) const fn mint(raw: u64) -> Self {
        Self(raw)
    }
}

/// What kind of claim an [`Evidence`] is.
///
/// The variants never promote into each other: a [`Reported`](Self::Reported)
/// claim stays reported even when convenient, and only
/// [`Derived`](Self::Derived) / [`Inferred`](Self::Inferred) carry a
/// mandatory basis.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceKind {
    /// Directly observed fact — e.g. a collector [`Measurement`].
    /// Carries no basis; its trustworthiness is expressed through
    /// `validity`/`confidentiality`, not through promotion.
    Observed,
    /// Deterministically derived from other evidence — requires a
    /// non-empty `basis` of [`EvidenceRef`]s.
    Derived,
    /// Claimed by another party (domain, peer, external report).
    /// Unverifiable here — it is never silently upgraded to
    /// [`Observed`](Self::Observed).
    Reported,
    /// Reasoned guess from evidence — requires a non-empty `basis`.
    /// Findings built on inferred evidence keep
    /// [`FindingCertainty::Inferred`](crate::diagnostics::FindingCertainty::Inferred)
    /// and never assert a definitive cause.
    Inferred,
}

/// The clock reading an [`Evidence`] carries, if any.
///
/// `Monotonic` is the collector's host `Instant` (observation commit
/// time). `Clock` is a domain-native [`ClockContext`] — ticks, timebase,
/// epoch and provenance are preserved verbatim with no conversion.
/// `Unstamped` means the evidence legitimately claims no time; it is
/// distinct from an estimated clock reading.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceTime {
    /// Host monotonic instant (e.g. `Measurement::observed_at`).
    Monotonic(Instant),
    /// Domain-native clock-domain stamp.
    Clock(ClockContext),
    /// No time claim.
    Unstamped,
}

/// The payload an [`Evidence`] may carry — a closed vocabulary that
/// cannot transport arbitrary bytes.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum EvidencePayload {
    /// No payload — the claim is purely classificatory.
    None,
    /// A typed metric value cloned from a [`Measurement`].
    Metric(MetricValue),
    /// A statically-bound domain claim code (e.g. `"queue-full"`).
    Claim(&'static str),
}

/// Descriptive fields shared by every [`Evidence`] kind.
///
/// `domain` is the provenance label (typically the descriptor's
/// `origin`); `subject` statically identifies what the evidence is
/// about (typically a `MetricDescriptor::key`). Both are `&'static
/// str`, bounding cardinality at compile time.
#[derive(Debug, Clone)]
pub struct EvidenceMeta {
    /// Provenance label — which domain/crate/system the claim comes
    /// from.
    pub domain: &'static str,
    /// Static subject identifier (e.g. a metric key), if any.
    pub subject: Option<&'static str>,
    /// When the claim was observed, if known.
    pub time: EvidenceTime,
    /// Trustworthiness of the underlying observation.
    pub validity: MeasurementValidity,
    /// Confidentiality classification — drives report disclosure.
    pub confidentiality: Confidentiality,
    /// The closed-vocabulary payload.
    pub payload: EvidencePayload,
}

/// One classified piece of diagnostic evidence.
///
/// Construct through the per-kind constructors — they enforce the
/// basis invariants: [`Evidence::derived`] and [`Evidence::inferred`]
/// reject an empty basis, while [`Evidence::observed`] and
/// [`Evidence::reported`] cannot carry one at all.
#[derive(Debug, Clone)]
pub struct Evidence {
    kind: EvidenceKind,
    source: Option<SourceIdentity>,
    meta: EvidenceMeta,
    basis: Box<[EvidenceRef]>,
}

impl Evidence {
    /// Observed fact bound to a registered [`SourceIdentity`].
    ///
    /// Observed evidence asserts *that a measurement exists* — never
    /// that it is complete or exact; `meta.validity` carries the
    /// collector's own trust classification.
    #[must_use]
    pub fn observed(source: SourceIdentity, meta: EvidenceMeta) -> Self {
        Self {
            kind: EvidenceKind::Observed,
            source: Some(source),
            meta,
            basis: Box::new([]),
        }
    }

    /// A claim reported by another party — `source` is the subject
    /// source when known. Reported evidence carries no basis and is
    /// never promoted to [`EvidenceKind::Observed`].
    #[must_use]
    pub fn reported(source: Option<SourceIdentity>, meta: EvidenceMeta) -> Self {
        Self {
            kind: EvidenceKind::Reported,
            source,
            meta,
            basis: Box::new([]),
        }
    }

    /// Deterministically derived claim — `basis` must be non-empty.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceError::MissingBasis`] when `basis` is empty:
    /// a derived claim without explainable references is rejected.
    pub fn derived(
        basis: impl Into<Box<[EvidenceRef]>>,
        source: Option<SourceIdentity>,
        meta: EvidenceMeta,
    ) -> Result<Self, EvidenceError> {
        Self::with_basis(EvidenceKind::Derived, basis, source, meta)
    }

    /// Inferred claim — `basis` must be non-empty. Inferred evidence
    /// supports only non-definitive findings; it never asserts a cause.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceError::MissingBasis`] when `basis` is empty.
    pub fn inferred(
        basis: impl Into<Box<[EvidenceRef]>>,
        source: Option<SourceIdentity>,
        meta: EvidenceMeta,
    ) -> Result<Self, EvidenceError> {
        Self::with_basis(EvidenceKind::Inferred, basis, source, meta)
    }

    fn with_basis(
        kind: EvidenceKind,
        basis: impl Into<Box<[EvidenceRef]>>,
        source: Option<SourceIdentity>,
        meta: EvidenceMeta,
    ) -> Result<Self, EvidenceError> {
        let basis: Box<[EvidenceRef]> = basis.into();
        if basis.is_empty() {
            return Err(EvidenceError::MissingBasis);
        }
        Ok(Self {
            kind,
            source,
            meta,
            basis,
        })
    }

    /// Builds [`EvidenceKind::Observed`] evidence from a collector
    /// [`Measurement`], preserving identity (id + generation),
    /// observation instant, validity, confidentiality, metric key and
    /// the typed value verbatim — no reinterpretation.
    #[must_use]
    pub fn from_measurement(m: &Measurement) -> Self {
        let payload = match &m.value {
            Some(v) => EvidencePayload::Metric(v.clone()),
            None => EvidencePayload::None,
        };
        Self::observed(
            m.identity,
            EvidenceMeta {
                domain: m.descriptor.semantics.origin,
                subject: Some(m.descriptor.key),
                time: EvidenceTime::Monotonic(m.observed_at),
                validity: m.validity,
                confidentiality: m.descriptor.semantics.confidentiality,
                payload,
            },
        )
    }

    /// The claim classification.
    #[must_use]
    pub const fn kind(&self) -> EvidenceKind {
        self.kind
    }

    /// The registered source this evidence concerns, if bound.
    #[must_use]
    pub const fn source(&self) -> Option<SourceIdentity> {
        self.source
    }

    /// Provenance label.
    #[must_use]
    pub const fn domain(&self) -> &'static str {
        self.meta.domain
    }

    /// Static subject identifier.
    #[must_use]
    pub const fn subject(&self) -> Option<&'static str> {
        self.meta.subject
    }

    /// The clock reading, if any.
    #[must_use]
    pub const fn time(&self) -> EvidenceTime {
        self.meta.time
    }

    /// Trustworthiness classification.
    #[must_use]
    pub const fn validity(&self) -> MeasurementValidity {
        self.meta.validity
    }

    /// Confidentiality classification.
    #[must_use]
    pub const fn confidentiality(&self) -> Confidentiality {
        self.meta.confidentiality
    }

    /// The payload.
    #[must_use]
    pub const fn payload(&self) -> &EvidencePayload {
        &self.meta.payload
    }

    /// The explainable basis — empty for `Observed`/`Reported`,
    /// guaranteed non-empty for `Derived`/`Inferred`.
    #[must_use]
    pub fn basis(&self) -> &[EvidenceRef] {
        &self.basis
    }
}

/// One stored evidence entry — the engine's retention form.
#[derive(Debug, Clone)]
pub struct EvidenceEntry {
    /// The minted reference other records cite.
    pub reference: EvidenceRef,
    /// The evidence itself.
    pub evidence: Evidence,
    /// `true` when the evidence's recorded [`SourceIdentity`] is no
    /// longer present in the latest evaluated snapshot — the source
    /// retired, was removed, or re-registered under a new generation.
    /// Superseded evidence is never rebound to a different source.
    pub superseded: bool,
}

/// Errors constructing or recording [`Evidence`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    /// `Derived`/`Inferred` evidence was built with an empty basis.
    MissingBasis,
    /// The engine's evidence retention is disabled
    /// (`DiagnosticsConfig::max_evidence == 0`).
    RetentionDisabled,
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBasis => {
                write!(f, "derived/inferred evidence requires a non-empty basis")
            }
            Self::RetentionDisabled => write!(f, "evidence retention is disabled"),
        }
    }
}

impl Error for EvidenceError {}
