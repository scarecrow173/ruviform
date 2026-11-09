//! Host-facing observability configuration — Issue #293 (OBS-11).
//!
//! [`ObservabilityConfig`] groups the per-subsystem settings the
//! [`ObservationRunner`](crate::integration::ObservationRunner)
//! composes: collection/metrics budgets, diagnostics, debug
//! inspection, and (with the `tracing-export` feature) the tracing
//! exposure / export policy. Each section is independent — `None`
//! disables that subsystem and the runner still operates as a
//! Collector-only pipeline.
//!
//! `LoggingConfig` (Issue #276, OBS-01) is deliberately **not** part of
//! this config: logging stays host-owned and is attached to the runner
//! explicitly via
//! [`attach_logging`](crate::integration::ObservationRunner::attach_logging)
//! under the `logging` feature. The runner never installs a global
//! `tracing` dispatcher and never wraps the host's logging policy.

use std::sync::Arc;

use crate::collector::CollectorConfig;
use crate::debug::{DebugAuthority, DebugConfig, DebugInspector};
use crate::diagnostics::{DiagnosticsConfig, DiagnosticsConfigError};

/// Debug inspection settings for [`ObservabilityConfig::debug`].
///
/// `authority` is an optional shared [`DebugAuthority`]: when present,
/// the runner builds the inspector with
/// [`DebugInspector::with_authority`] so the host can mint
/// [`SessionPermit`](crate::debug::SessionPermit)s for debug-session
/// control. The `Arc` lets the host keep its own handle for permit
/// minting while the inspector holds the cloned capability token.
/// `authority: None` builds a plain inspector — session operations are
/// then rejected with `NoAuthority` while inspection and report
/// generation keep working.
#[derive(Debug, Clone)]
pub struct DebugSettings {
    /// Inspector configuration (budgets, retention, disclosure
    /// ceilings).
    pub config: DebugConfig,
    /// Optional shared session authority.
    pub authority: Option<Arc<DebugAuthority>>,
}

impl DebugSettings {
    /// Settings with no session authority.
    #[must_use]
    pub fn new(config: DebugConfig) -> Self {
        Self {
            config,
            authority: None,
        }
    }

    /// Settings sharing `authority` with the host for permit minting.
    #[must_use]
    pub fn with_authority(config: DebugConfig, authority: Arc<DebugAuthority>) -> Self {
        Self {
            config,
            authority: Some(authority),
        }
    }
}

/// Typed configuration failure — delegates to the owning subsystem's
/// validator; the integration layer invents no new rules.
#[non_exhaustive]
#[derive(Debug)]
pub enum ConfigError {
    /// [`CollectorConfig::validate`] rejected the collection settings.
    Collector(crate::collector::CollectorConfigError),
    /// [`DiagnosticsConfig::validate`] rejected the diagnostics
    /// settings.
    Diagnostics(DiagnosticsConfigError),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Collector(err) => write!(f, "invalid collector config: {err}"),
            Self::Diagnostics(err) => write!(f, "invalid diagnostics config: {err}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<crate::collector::CollectorConfigError> for ConfigError {
    fn from(err: crate::collector::CollectorConfigError) -> Self {
        Self::Collector(err)
    }
}

impl From<DiagnosticsConfigError> for ConfigError {
    fn from(err: DiagnosticsConfigError) -> Self {
        Self::Diagnostics(err)
    }
}

/// Unified non-RT observability configuration.
///
/// Every subsystem section is independent:
///
/// - `collection` — the OBS-07 [`CollectorConfig`] authority: poll
///   interval, source/series/batch budgets, history budget (the
///   metrics retention policy), and the stale threshold. Metrics has
///   no separate engine — its budgets live here, so no second metrics
///   config is invented.
/// - `diagnostics` — `None` disables the diagnostics engine entirely.
/// - `debug` — `None` disables debug inspection entirely.
/// - `export` — present only with the `tracing-export` feature;
///   `dispatch: None` disables tracing export while keeping the rest
///   of the pipeline intact.
///
/// There is no unconditional logging section: `LoggingConfig` stays
/// host-owned (OBS-01) and attaches via
/// [`ObservationRunner::attach_logging`](crate::integration::ObservationRunner::attach_logging).
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Collection and metrics budgets — validated by
    /// [`CollectorConfig::validate`].
    pub collection: CollectorConfig,
    /// Diagnostics engine settings; `None` disables diagnostics.
    pub diagnostics: Option<DiagnosticsConfig>,
    /// Debug inspector settings; `None` disables debug inspection.
    pub debug: Option<DebugSettings>,
    /// Tracing exposure / export policy — `tracing-export` feature only.
    #[cfg(feature = "tracing-export")]
    pub export: ExportPolicy,
}

/// Tracing exposure / export policy (`tracing-export` feature only).
///
/// The runner emits each poll's [`ObservationBatch`](crate::collector::ObservationBatch)
/// through a [`TracingExporter`](crate::tracing_export::TracingExporter)
/// bound to `dispatch` — a [`tracing::Dispatch`] the **host** built and
/// owns. The library never calls `set_global_default`/`init`; the
/// exporter scopes every emit to the injected dispatch, so ambient or
/// global subscribers are unaffected unless the host chooses them.
#[cfg(feature = "tracing-export")]
#[derive(Debug, Clone)]
pub struct ExportPolicy {
    /// Dispatch observation events are emitted on. `None` disables
    /// tracing export entirely — the runner then owns no exporter.
    pub dispatch: Option<tracing::Dispatch>,
    /// Per-poll cap on measurement events, forwarded to
    /// [`TracingExporter::with_max_emissions_per_poll`]. `0` emits no
    /// measurement events (policy/snapshot bookkeeping still runs).
    pub max_emissions_per_poll: usize,
    /// When `true` and diagnostics is enabled with a dispatch present,
    /// `prepare` installs a
    /// [`TracingFindingSink`](crate::tracing_export::TracingFindingSink)
    /// on the diagnostics engine so health transitions reach the same
    /// dispatch. The host may still replace the sink via
    /// [`diagnostics_mut`](crate::integration::ObservationRunner::diagnostics_mut).
    pub diagnostics_events: bool,
}

#[cfg(feature = "tracing-export")]
impl Default for ExportPolicy {
    fn default() -> Self {
        Self {
            dispatch: None,
            max_emissions_per_poll: crate::tracing_export::DEFAULT_MAX_EMISSIONS_PER_POLL,
            diagnostics_events: false,
        }
    }
}

impl ObservabilityConfig {
    /// Validates every enabled subsystem by delegating to the owning
    /// validator — no new rules are invented here.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Collector`] or [`ConfigError::Diagnostics`]
    /// when the corresponding section fails its own `validate`.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.collection.validate()?;
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.validate()?;
        }
        Ok(())
    }

    /// Builds the debug inspector described by `debug`, if any.
    /// Crate-internal — the runner calls this during `prepare`.
    pub(crate) fn build_debug(&self) -> Option<DebugInspector> {
        self.debug
            .as_ref()
            .map(|settings| match &settings.authority {
                Some(authority) => {
                    DebugInspector::with_authority(settings.config.clone(), authority)
                }
                None => DebugInspector::new(settings.config.clone()),
            })
    }
}
