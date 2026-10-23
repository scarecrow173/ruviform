//! `ruviform-edit` — the non-destructive edit data model (NLE
//! timeline / track / clip) of the Ruviform media foundation.
//!
//! ## Responsibility
//!
//! Data + evaluation only: a [`Timeline`] owns a single [`TimeBase`]
//! and an ordered [`Track`] list holding [`Clip`]s — source reference +
//! timeline occupancy + source in/out + kind attributes (ADR
//! `second-target-editing-model` D1-a). All mutations funnel through
//! `Timeline`, which normalizes every stored `Clip::range` into
//! `timeline.base()` ([`Rounding::NearestEven`]) and re-runs the full
//! validation rule set; the evaluation projection (`evaluate` →
//! `Evaluation`/`ActiveClip`, D3-a) stays engine-neutral data.
//!
//! undo/redo, UI, media-asset management and engine construction are
//! the application layer's job — engine structs (`Graph`,
//! `FramePipeline`, …) are built by consumers, not here
//! (second-target boundary rule).
//!
//! ## Allowed dependency layers
//!
//! `ruviform-media-core` only (`MediaTime` / `MediaDuration` /
//! `MediaRange` / `TimeBase` / `Rounding` — `convert` / `checked_end` /
//! `checked_add`, exact cross-base comparison). Engine crates
//! (`ruviform-audio` / `ruviform-video`) and other foundations must
//! not be depended on — the projection's neutrality is guaranteed by
//! the dependency structure itself.
//!
//! ## RT contract
//!
//! **Non-RT only.** Every API may allocate (`Vec` tracks/clips and
//! the evaluation output included), so calling any of it from an RT
//! path is a contract violation. `Timeline` is not required to be
//! `Send` — concurrent access is the caller's responsibility
//! (`InProcessHost` convention). Per-API labels read
//! `**RT contract:** non-RT only`.
//!
//! Contract source: `docs/designs/edit-data-model.md` (Issue #96) and
//! `docs/adr/second-target-editing-model.md` (D1-a / D3-a).

// The public names and shapes below are fixed API decisions of the
// data model: they deliberately echo their module (`model::Timeline`,
// `error::Error`, …) while being re-exported at the crate root, and as
// a `publish = false` workspace-internal crate the invariant-free
// types (`ClipSpec`, `Evaluation`/`ActiveClip`) rely on literal
// construction — see `docs/adr/public-enum-non-exhaustive-policy.md`.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::exhaustive_structs)]

mod error;
mod eval;
mod model;

pub use error::{Error, Result};
pub use eval::{ActiveClip, Evaluation};
pub use model::{
    Clip, ClipAttributes, ClipId, ClipSource, ClipSpec, Timeline, Track, TrackId, TrackKind,
};

/// Returns the crate version string.
///
/// **RT contract:** non-RT only — trivial pure function, but part of
/// a non-RT crate.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_edit::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
