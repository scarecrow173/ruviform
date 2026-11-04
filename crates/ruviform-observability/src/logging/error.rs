//! Typed errors for [`build_logging`](crate::logging::build_logging) /
//! [`build_logging_layers`](crate::logging::build_logging_layers).

use std::error::Error;
use std::fmt;

use super::health::SinkKind;

/// Build-time failure of a single sink — used both as the payload of
/// [`LoggingBuildError::RequiredSinkFailed`] (one failing required sink) and
/// [`LoggingBuildError::AllSinksFailed`] (every best-effort sink failed).
#[derive(Debug)]
pub struct SinkFailure {
    /// Index of the failed sink in the `sinks` list of the supplied
    /// `LoggingConfig`.
    pub sink_index: usize,
    /// Which destination kind was being initialized.
    pub kind: SinkKind,
    /// The underlying initialization failure.
    pub source: std::io::Error,
}

impl fmt::Display for SinkFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "sink #{sink_index} ({kind}) failed to initialize: {source}",
            sink_index = self.sink_index,
            kind = self.kind,
            source = self.source,
        )
    }
}

/// Errors returned by [`build_logging`](crate::logging::build_logging) /
/// [`build_logging_layers`](crate::logging::build_logging_layers).
///
/// Every variant names a distinct build-time failure class. Runtime sink
/// failures (after a successful build) never surface through this type —
/// they are counted in the health snapshot instead.
#[non_exhaustive]
#[derive(Debug)]
pub enum LoggingBuildError {
    /// `LoggingConfig::sinks` was empty. A no-sink build would silently
    /// produce a dead pipeline, so it is rejected explicitly.
    NoSinks,
    /// A sink's `filter` string is not a valid `EnvFilter` directive.
    InvalidFilter {
        /// Index of the sink in the `sinks` list.
        sink_index: usize,
        /// The offending filter string, echoed back for the host to log or
        /// surface.
        filter: String,
        /// Human-readable parse failure detail from the filter parser.
        reason: String,
    },
    /// A sink configured [`SinkFailureMode::Required`] failed to
    /// initialize. The build is aborted; any already-built sinks are
    /// shut down before this error is returned.
    RequiredSinkFailed {
        /// Which sink failed and why.
        failure: SinkFailure,
    },
    /// Every configured sink failed to initialize. All were
    /// [`SinkFailureMode::BestEffort`] (a required failure returns
    /// [`Self::RequiredSinkFailed`] first), so instead of returning an
    /// all-dead pipeline as success the build reports the collected
    /// failures.
    AllSinksFailed {
        /// One [`SinkFailure`] per configured sink, in configuration order.
        failures: Vec<SinkFailure>,
    },
}

impl fmt::Display for LoggingBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSinks => f.write_str("logging config contains no sinks"),
            Self::InvalidFilter {
                sink_index,
                filter,
                reason,
            } => write!(
                f,
                "sink #{sink_index} has an invalid filter {filter:?}: {reason}"
            ),
            Self::RequiredSinkFailed { failure } => {
                write!(f, "required {failure}")
            }
            Self::AllSinksFailed { failures } => {
                write!(f, "all {} configured sink(s) failed:", failures.len())?;
                for failure in failures {
                    write!(f, " {failure};")?;
                }
                Ok(())
            }
        }
    }
}

impl Error for LoggingBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RequiredSinkFailed { failure } => Some(&failure.source),
            Self::AllSinksFailed { failures } => failures
                .first()
                .map(|failure| &failure.source as &dyn Error),
            Self::NoSinks | Self::InvalidFilter { .. } => None,
        }
    }
}
