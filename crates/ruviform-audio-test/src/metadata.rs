//! Run metadata using the Case Schema v1 vocabulary (Issue #628,
//! contract from `scarecrow173/ruviform#627` / POLICIES.md
//! 2026-11-15 virtual-first terms).
//!
//! [`RunMetadata`] is a typed record with JSON serialization
//! ([`RunMetadata::to_json`]/[`RunMetadata::from_json`]/
//! [`RunMetadata::save_json`]/[`RunMetadata::load_json`]) — the
//! non-RT test-driver API that persists the data needed to reproduce a
//! run. Every deterministic-backend run must be reproducible from
//! `(seed, fixture_version, clock_config)` alone; these fields are
//! what a consumer needs to re-run and compare.
//!
//! **Honesty rule:** `environment_kind = DeterministicBackend` +
//! `level = OsL1` records deterministic-core evidence only. It is never
//! a claim of real OS API, GUI, or plugin ABI success — those live at
//! EXT-L3 / GUI-L2 with their own `environment_kind`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::clock::VirtualClockConfig;

/// Whether a case is required for the current acceptance scope.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy. Serialized as the Case Schema v1 `kebab-case` vocabulary
/// (`required`/`deferred`/`not-applicable`).
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
/// inferred or claimed. Serialized as the Case Schema v1 `kebab-case`
/// vocabulary (`pass`/`fail`/`not-run`/`unsupported`).
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
/// policy. Serialized as the Case Schema v1 `kebab-case` vocabulary
/// (`os-l1`/`gui-l2`/`ext-l3`).
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
/// policy. `DeterministicBackend` remains the only kind this crate's
/// own runtime emits; the real kinds below are vocabulary for
/// consumer harnesses (e.g. `ruviform-plugin-dock-test`) that actually
/// execute the named environment — a run may claim a real kind only
/// when it really ran there (POLICIES.md 2026-11-15 virtual-first
/// terms; a mock/headless success never promotes to a real kind).
/// Serialized as the Case Schema v1 `kebab-case` vocabulary — the
/// same spelling [`EnvironmentKind::as_str`] returns.
///
/// Known vocabulary gap: `RealAbiTestPlugin` keeps its historical
/// `real-abi-test-plugin` wire spelling, which differs from the Case
/// Schema v1 `test-plugin` value. Aligning it is a breaking serde
/// change (consumers already emit the current spelling in evidence)
/// and is deferred to a later slice together with a serde migration
/// decision.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnvironmentKind {
    /// Deterministic virtual backend (this crate's domain).
    DeterministicBackend,
    /// A real plugin-ABI binary was resolved, loaded and driven
    /// through the production host path (e.g. the Ruviform Test
    /// Plugin suite via `Vst3Library`/`InProcessHost`). Only a run
    /// that actually executed the binary may claim this — mock or
    /// headless plugin success stays `DeterministicBackend`.
    RealAbiTestPlugin,
    /// A real OS window / GUI session produced the evidence (GUI-L2
    /// lanes). Reserved for real-window driver runs — a headless or
    /// simulated run must not claim it.
    RealWindow,
    /// A virtual OS endpoint that the production OS audio/MIDI API
    /// actually traversed (e.g. a provisioned WASAPI/WinMM virtual
    /// render↔capture pair driven through the production backend).
    /// Only a run that really streamed through the endpoint may claim
    /// this — enumeration-only, mock, or headless success stays
    /// `DeterministicBackend`, and the kind never implies
    /// physical-device parity.
    VirtualOsEndpoint,
    /// A physical audio/MIDI device or commercial plugin produced the
    /// evidence. Reserved for opt-in cases that formally declare
    /// support for that specific device/plugin — never part of
    /// first-pass acceptance.
    PhysicalDevice,
}

impl EnvironmentKind {
    /// The Case Schema v1 wire spelling (`kebab-case`).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeterministicBackend => "deterministic-backend",
            Self::RealAbiTestPlugin => "real-abi-test-plugin",
            Self::RealWindow => "real-window",
            Self::VirtualOsEndpoint => "virtual-os-endpoint",
            Self::PhysicalDevice => "physical-device",
        }
    }
}

/// Reproducibility parameters of a scenario run — the minimal set that
/// must reproduce identical PCM/MIDI traces.
///
/// **RT contract:** RT-suitable — `Copy` value type. `Serialize`/
/// `Deserialize` are derived for run-metadata JSON only; they are
/// never invoked on RT paths.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
/// needed. JSON persistence is provided by
/// [`RunMetadata::to_json`]/[`RunMetadata::from_json`]/
/// [`RunMetadata::save_json`]/[`RunMetadata::load_json`] — non-RT
/// test-driver APIs (they allocate and perform I/O; never call them
/// from an RT path).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

    /// Serializes this record to a JSON document — the Case Schema v1
    /// `kebab-case` wire vocabulary.
    ///
    /// **Non-RT:** allocates; never call from an RT path.
    ///
    /// # Errors
    ///
    /// [`serde_json::Error`] when serialization fails (cannot happen for
    /// this record's field types in practice).
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Deserializes a [`RunMetadata`] from a JSON document produced by
    /// [`Self::to_json`] (or an equivalent Case Schema v1 record).
    ///
    /// **Non-RT:** allocates; never call from an RT path.
    ///
    /// # Errors
    ///
    /// [`serde_json::Error`] on malformed JSON or a vocabulary value
    /// outside the `kebab-case` enum spellings.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    /// Writes this record as JSON to `path` (UTF-8, no trailing
    /// newline added beyond the serialized document).
    ///
    /// **Non-RT:** performs file I/O; never call from an RT path.
    ///
    /// # Errors
    ///
    /// [`std::io::Error`] on serialization or write failure.
    pub fn save_json(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let json = self
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Reads a JSON document from `path` and deserializes it — the
    /// counterpart of [`Self::save_json`].
    ///
    /// **Non-RT:** performs file I/O; never call from an RT path.
    ///
    /// # Errors
    ///
    /// [`std::io::Error`] on read failure, or `InvalidData` when the
    /// file is not a valid [`RunMetadata`] JSON document.
    pub fn load_json(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
