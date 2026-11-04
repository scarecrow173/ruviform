//! [`PublishOptions`] — the three independent axes of a publish
//! request: mode, visibility requirement, durability.
//!
//! These axes are deliberately separate: replacing a destination,
//! making the replacement atomic, and making it durable are different
//! guarantees that must not be conflated. A platform that cannot meet
//! a requested axis reports `Unsupported`/failure — it never silently
//! degrades the request.

use std::fmt;

/// How [`SealedFile::publish`](crate::SealedFile::publish) treats the
/// destination.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PublishMode {
    /// Publish only when the destination does not already exist.
    ///
    /// An existing destination is refused (the publish reports
    /// [`PublishFailureKind::DestinationExists`](crate::PublishFailureKind::DestinationExists)
    /// with [`PublishEffect::NotPublished`](crate::PublishEffect::NotPublished));
    /// the existing destination stays byte-for-byte untouched.
    ///
    /// This must be implemented with a genuinely atomic
    /// no-replace primitive by the platform adapter —
    /// `exists()`-then-`rename()` does not provide that guarantee
    /// and is never used as a fallback.
    CreateNew,
    /// Create-or-replace: publish whether or not the destination
    /// exists. This is *unconditional* replacement — it carries no
    /// revision/CAS or multi-file transaction semantics.
    Replace,
}

impl fmt::Display for PublishMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CreateNew => "create-new (fail if destination exists)",
            Self::Replace => "replace (create-or-replace)",
        };
        f.write_str(message)
    }
}

/// The visibility guarantee the publish must provide.
///
/// These levels are a *requirement*, not a description: an adapter
/// that cannot meet [`RequireAtomic`](Self::RequireAtomic) for the
/// requested mode must fail the publish
/// ([`PublishFailureKind::Unsupported`](crate::PublishFailureKind::Unsupported))
/// rather than silently fall back to a weaker mechanism (e.g.
/// copy-and-delete). [`BestEffort`](Self::BestEffort) explicitly
/// relaxes that requirement for callers that accept platform-dependent
/// visibility.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisibilityRequirement {
    /// The publish operation must be atomic: observers either see the
    /// complete staged content at the destination or see the previous
    /// state — never a partially written file.
    RequireAtomic,
    /// Atomic visibility is not required — the adapter may use its
    /// cheapest publish path. The destination may transiently expose
    /// intermediate states to observers.
    BestEffort,
}

impl fmt::Display for VisibilityRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::RequireAtomic => "require atomic visibility",
            Self::BestEffort => "best-effort visibility",
        };
        f.write_str(message)
    }
}

/// The durability guarantee the publish must achieve before
/// reporting success.
///
/// `Write::flush`, file sync, directory sync, atomic visibility and
/// power-failure durability are *distinct* guarantees — this enum
/// only names the sync levels the publish pipeline is asked to
/// achieve; it is not a claim about total hardware-failure immunity.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Durability {
    /// No sync requirement — the publish reports success once the
    /// publish operation itself is applied. Cheapest, least durable.
    BestEffort,
    /// The staged file's contents must be synced to storage
    /// (`sync_file`) *before* the publish. The directory entry itself
    /// is not synced — a crash may still lose the new name.
    FileSync,
    /// `FileSync` plus a directory sync *after* the publish, so the
    /// destination name itself is durable. Only reported when the
    /// platform actually completed it; if directory sync fails after
    /// a successful publish the result is `Err` with
    /// [`PublishEffect::Published`](crate::PublishEffect::Published).
    FileAndDirectorySync,
}

impl fmt::Display for Durability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::BestEffort => "best-effort",
            Self::FileSync => "file sync",
            Self::FileAndDirectorySync => "file and directory sync",
        };
        f.write_str(message)
    }
}

/// The publish request: three independent axes.
///
/// `#[non_exhaustive]` — new axes may be added; construct with
/// [`PublishOptions::new`], then adjust fields as needed.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// use ruviform_io_file::{Durability, PublishMode, PublishOptions};
///
/// let options = PublishOptions::new(PublishMode::CreateNew, Durability::FileSync);
/// assert_eq!(options.mode, PublishMode::CreateNew);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct PublishOptions {
    /// How an existing destination is treated.
    pub mode: PublishMode,
    /// The required visibility guarantee.
    pub visibility: VisibilityRequirement,
    /// The required durability level.
    pub durability: Durability,
}

impl PublishOptions {
    /// Options with [`VisibilityRequirement::RequireAtomic`] — the only
    /// visibility level this crate currently supports.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new(mode: PublishMode, durability: Durability) -> Self {
        Self {
            mode,
            visibility: VisibilityRequirement::RequireAtomic,
            durability,
        }
    }

    /// Overrides the visibility requirement.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_visibility(mut self, visibility: VisibilityRequirement) -> Self {
        self.visibility = visibility;
        self
    }
}
