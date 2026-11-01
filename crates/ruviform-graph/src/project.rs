//! `GraphProject`: the non-RT container that owns the requested spec,
//! issues revisions, and publishes validated snapshots.
//!
//! This is **not** an RT publication/swap mechanism — it is a plain
//! `&mut` container for edit-side code. RT-facing hand-off (snapshot
//! swap, retire) belongs to the M2+ consumer.

use crate::graph::GraphSpec;
use crate::ids::GraphRevision;
use crate::validate::{ValidatedGraph, ValidationErrors};

/// A successfully validated, published snapshot with its revision.
#[derive(Clone, Debug)]
pub struct PublishedGraph {
    revision: GraphRevision,
    graph: ValidatedGraph,
}

impl PublishedGraph {
    /// Revision this snapshot was published under.
    #[must_use]
    pub const fn revision(&self) -> GraphRevision {
        self.revision
    }

    /// The validated snapshot.
    #[must_use]
    pub const fn graph(&self) -> &ValidatedGraph {
        &self.graph
    }
}

/// Failure of [`GraphProject::prepare`].
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum PrepareError {
    /// Validation failed — the attempted revision is returned with
    /// the full error set. Published state is unchanged.
    Validation {
        /// The revision consumed by the failed attempt.
        revision: GraphRevision,
        /// Every detected failure.
        errors: ValidationErrors,
    },
}

impl core::fmt::Display for PrepareError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Validation { revision, errors } => {
                write!(f, "prepare of revision {} failed: {errors}", revision.get())
            }
        }
    }
}

impl core::error::Error for PrepareError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Validation { errors, .. } => Some(errors),
        }
    }
}

/// Non-RT container owning the last *requested* spec and the last
/// successfully *published* graph, plus monotonically increasing
/// project-local revisions.
///
/// `prepare` always consumes a revision number (requested), validates,
/// and only replaces the published snapshot on success — a failed
/// prepare leaves the previous published graph intact while
/// `requested_revision() > published_revision()`.
#[derive(Debug, Default)]
pub struct GraphProject {
    next_revision: u64,
    requested: Option<(GraphRevision, GraphSpec)>,
    published: Option<PublishedGraph>,
}

impl GraphProject {
    /// Empty project — no requested or published state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Revision of the last `prepare` attempt, if any.
    #[must_use]
    pub const fn requested_revision(&self) -> Option<GraphRevision> {
        match &self.requested {
            Some((rev, _)) => Some(*rev),
            None => None,
        }
    }

    /// The last requested (attempted) spec, if any.
    #[must_use]
    pub const fn requested(&self) -> Option<&GraphSpec> {
        match &self.requested {
            Some((_, spec)) => Some(spec),
            None => None,
        }
    }

    /// Revision of the last *successful* prepare, if any.
    #[must_use]
    pub const fn published_revision(&self) -> Option<GraphRevision> {
        match &self.published {
            Some(p) => Some(p.revision),
            None => None,
        }
    }

    /// The last published validated snapshot, if any.
    #[must_use]
    pub const fn published(&self) -> Option<&PublishedGraph> {
        self.published.as_ref()
    }

    /// Attempts to validate and publish `spec`.
    ///
    /// Always records `spec` as the requested state under a fresh
    /// monotonically increasing revision — even on failure. On
    /// success the [`ValidatedGraph`] becomes the new published
    /// snapshot and its revision is returned. On failure the
    /// previously published snapshot is preserved untouched.
    ///
    /// # Errors
    /// Returns [`PrepareError::Validation`] with the consumed
    /// revision and all detected failures; published state is
    /// unchanged.
    pub fn prepare(&mut self, spec: GraphSpec) -> Result<GraphRevision, PrepareError> {
        self.next_revision += 1;
        let revision = GraphRevision::from_issued(self.next_revision);

        match crate::validate::Validator::validate(&spec) {
            Ok(graph) => {
                self.requested = Some((revision, spec));
                self.published = Some(PublishedGraph { revision, graph });
                Ok(revision)
            }
            Err(errors) => {
                self.requested = Some((revision, spec));
                Err(PrepareError::Validation { revision, errors })
            }
        }
    }
}
