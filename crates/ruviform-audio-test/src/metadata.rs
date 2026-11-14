//! Run metadata using the Case Schema v1 vocabulary (Issue #628,
//! contract from `scarecrow173/ruviform#627` / POLICIES.md
//! 2026-11-15 virtual-first terms).
//!
//! [`RunMetadata`] is a typed record — JSON serialization is deferred
//! to a later slice. Every deterministic-backend run must be
//! reproducible from `(seed, fixture_version, clock_config)` alone;
//! these fields are what a consumer needs to re-run and compare.
//!
//! **Honesty rule:** `environment_kind = DeterministicBackend` +
//! `level = OsL1` records deterministic-core evidence only. It is never
//! a claim of real OS API, GUI, or plugin ABI success — those live at
//! EXT-L3 / GUI-L2 with their own `environment_kind`.

use crate::clock::VirtualClockConfig;

/// Whether a case is required for the current acceptance scope.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CaseRequirement {
    /// Must pass for acceptance at this level.
    Required,
    /// Scheduled for a later slice/level — not blocking now.
    Deferred,
    /// Does not apply to this environment/level.
    NotApplicable,
}

/// Observed result state of a case.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy. Anything not actually executed must be `NotRun` — never
/// inferred or claimed.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CaseResult {
    /// Executed and matched expectations.
    Pass,
    /// Executed and did not match expectations.
    Fail,
    /// Not executed (capability absent, lane not run, fixture missing).
    NotRun,
    /// Environment cannot express this case.
    Unsupported,
}

/// Verification depth level from POLICIES.md.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VerificationLevel {
    /// OS-L1: deterministic/virtual backends, native build+headless.
    OsL1,
    /// GUI-L2: real windowing surface required.
    GuiL2,
    /// EXT-L3: real OS APIs/endpoints/ABI required.
    ExtL3,
}

/// Environment kind from Case Schema v1 — what produced the evidence.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy. This slice only emits [`Self::DeterministicBackend`]; real
/// endpoint/plugin/GUI kinds arrive with their own slices.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnvironmentKind {
    /// Deterministic virtual backend (this crate's domain).
    DeterministicBackend,
}

/// Reproducibility parameters of a scenario run — the minimal set that
/// must reproduce identical PCM/MIDI traces.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RunParameters {
    /// Scenario seed.
    pub seed: u64,
    /// Fixture/scenario format version — bump when generation semantics
    /// change so old traces remain attributable.
    pub fixture_version: u32,
    /// Clock configuration the run used.
    pub clock_config: VirtualClockConfig,
}

/// One case's run record: identity, scope, environment, result and the
/// reproducibility parameters needed to re-run it.
///
/// All fields are public data — consumers serialize or extend as
/// needed; this crate intentionally provides no JSON writer yet.
#[derive(Clone, Debug, PartialEq)]
pub struct RunMetadata {
    /// Case identifier (free-form, e.g. `rf628.pcm.sine.48k`).
    pub case_id: String,
    /// Requirement classification for this case.
    pub requirement: CaseRequirement,
    /// Observed result — `NotRun` unless actually executed.
    pub result: CaseResult,
    /// Verification level the evidence belongs to.
    pub level: VerificationLevel,
    /// Environment that produced the evidence.
    pub environment_kind: EnvironmentKind,
    /// Oracle used for expected values (e.g. `oracle::pcm_reference`).
    pub oracle: String,
    /// Reproducibility parameters.
    pub parameters: RunParameters,
    /// Repository identity at run time (`owner/repo` or worktree path).
    pub repo: String,
    /// Source revision (commit SHA) the run was executed against.
    pub sha: String,
    /// OS description when relevant (empty for pure-Rust runs).
    pub os: String,
    /// Architecture when relevant (empty for pure-Rust runs).
    pub arch: String,
    /// Expected-result digest/summary for comparison.
    pub expected: String,
    /// Observed-result digest/summary for comparison.
    pub observed: String,
    /// Evidence references (file paths, CI run URLs, log digests).
    pub evidence: Vec<String>,
}

impl RunMetadata {
    /// Creates a deterministic-backend OS-L1 run record — the only
    /// combination this slice produces.
    #[must_use]
    pub fn deterministic(
        case_id: impl Into<String>,
        oracle: impl Into<String>,
        parameters: RunParameters,
        repo: impl Into<String>,
        sha: impl Into<String>,
    ) -> Self {
        Self {
            case_id: case_id.into(),
            requirement: CaseRequirement::Required,
            result: CaseResult::NotRun,
            level: VerificationLevel::OsL1,
            environment_kind: EnvironmentKind::DeterministicBackend,
            oracle: oracle.into(),
            parameters,
            repo: repo.into(),
            sha: sha.into(),
            os: String::new(),
            arch: String::new(),
            expected: String::new(),
            observed: String::new(),
            evidence: Vec::new(),
        }
    }

    /// Marks the case executed with `result` and records observed
    /// output.
    #[must_use]
    pub fn with_result(mut self, result: CaseResult, observed: impl Into<String>) -> Self {
        self.result = result;
        self.observed = observed.into();
        self
    }

    /// Sets the expected-result digest/summary.
    #[must_use]
    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = expected.into();
        self
    }

    /// Sets OS/arch fields.
    #[must_use]
    pub fn with_platform(mut self, os: impl Into<String>, arch: impl Into<String>) -> Self {
        self.os = os.into();
        self.arch = arch.into();
        self
    }

    /// Appends an evidence reference.
    #[must_use]
    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }

    /// Reproducibility key: `(seed, fixture_version)` — identical keys
    /// + identical `clock_config` must reproduce identical traces.
    #[must_use]
    pub fn reproducibility_key(&self) -> (u64, u32) {
        (self.parameters.seed, self.parameters.fixture_version)
    }
}
