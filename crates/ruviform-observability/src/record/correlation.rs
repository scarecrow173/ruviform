//! Correlation — fixed-size tokens that link related records, and the
//! non-RT [`CorrelationBook`] that issues and classifies them.
//!
//! A [`CorrelationToken`] is a `NonZeroU64` minted before records enter
//! the RT path; a [`CorrelationLink`] pairs it with the token epoch that
//! was current at mint time, so a link is a small `Copy` value a record
//! header can carry unconditionally.
//!
//! Links are deliberately *not* Trace IDs, Grant IDs,
//! [`SourceId`](crate::source::SourceId)s or
//! [`SourceGeneration`](crate::source::SourceGeneration)s: those identify
//! registrations and re-mints of observation sources, while a link
//! correlates *records* with one another across a boundary the collector
//! defines. Re-minting or expiring a token never silently re-associates
//! records — stale links classify as [`CorrelationStatus::Unlinked`]
//! with an explicit [`UnlinkedReason`].
//!
//! [`CorrelationBook`] is a non-RT structure (heap maps). It is the
//! issuer/registry side; RT writers only *carry* links a domain prepared
//! beforehand — issuing, re-minting, expiring and classifying all happen
//! off the RT path.

use std::collections::{HashMap, HashSet};
use std::num::NonZeroU64;

/// A fixed-size correlation token — `NonZeroU64` so `0` can never be a
/// token and stays reserved for "no link".
///
/// Minted by [`CorrelationBook::issue`]; carried inside a
/// [`CorrelationLink`]. Must not be conflated with `SourceId`,
/// `SourceGeneration`, Trace IDs, or Grant IDs.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CorrelationToken(NonZeroU64);

impl CorrelationToken {
    /// Wraps `raw` as a [`CorrelationToken`], or `None` when `raw == 0`.
    ///
    /// Intended for book/issuer implementations; domains normally receive
    /// tokens through [`CorrelationBook::issue`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(raw: u64) -> Option<Self> {
        match NonZeroU64::new(raw) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// The raw 64-bit value (always non-zero).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// A token plus the epoch it was minted under.
///
/// The link a record header carries: `epoch` is the book's epoch for the
/// token at mint time, so a link minted before a [`CorrelationBook::remint`]
/// never silently aliases the new epoch — it classifies
/// [`UnlinkedReason::SupersededEpoch`].
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CorrelationLink {
    token: CorrelationToken,
    epoch: u64,
}

impl CorrelationLink {
    /// Pairs `token` and `epoch` into a link. Intended for issuers;
    /// domains normally receive links from [`CorrelationBook`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(token: CorrelationToken, epoch: u64) -> Self {
        Self { token, epoch }
    }

    /// The token this link carries.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn token(self) -> CorrelationToken {
        self.token
    }

    /// The token epoch this link was minted under.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn epoch(self) -> u64 {
        self.epoch
    }
}

/// How a link stands against the book that issued its token.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrelationStatus {
    /// The token is live and the link's epoch is current — the link is a
    /// valid correlation handle.
    Linked,
    /// The link must not be used to correlate — see [`UnlinkedReason`].
    Unlinked(UnlinkedReason),
}

/// Why a link is not usable for correlation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnlinkedReason {
    /// The book never issued this token (or it was minted elsewhere).
    UnknownToken,
    /// The token was re-minted after this link's epoch — the link is
    /// stale and must not alias the new epoch.
    SupersededEpoch,
    /// The token was explicitly expired — links to it stay unlinked
    /// forever.
    Expired,
}

/// Non-RT issuer and registry for [`CorrelationToken`]s.
///
/// `CorrelationBook` is constructed and operated off the RT path: it
/// holds heap maps and every method may allocate or rehash. The links it
/// returns are the small `Copy` values that travel inside records.
///
/// `issue` mints fresh tokens monotonically from 1 (zero is reserved);
/// `remint` bumps a live token's epoch so previously minted links go
/// [`UnlinkedReason::SupersededEpoch`]; `expire` tombstones a token so
/// its links classify [`UnlinkedReason::Expired`] rather than
/// `UnknownToken`.
///
/// **Non-RT only** — construction, `issue`, `remint`, `expire`,
/// `classify` all run off the RT path. Only the produced links are
/// RT-carryable.
#[derive(Debug, Default)]
pub struct CorrelationBook {
    /// Live tokens: token value -> current epoch.
    live: HashMap<u64, u64>,
    /// Expired tokens (tombstones) — distinguishes `Expired` from
    /// `UnknownToken`.
    expired: HashSet<u64>,
    /// Last issued token value; next issue is `next + 1` (0 skipped).
    next: u64,
}

impl CorrelationBook {
    /// An empty book — non-RT only.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mints a fresh token and returns its first-epoch link.
    ///
    /// Token values increase monotonically from 1; `0` is never issued.
    /// Wrapping past `u64::MAX` re-uses `1` — unreachable in practice.
    ///
    /// **Non-RT only** — allocates.
    pub fn issue(&mut self) -> CorrelationLink {
        self.next = self.next.wrapping_add(1);
        if self.next == 0 {
            self.next = 1;
        }
        // `next` is non-zero by the skip above.
        let token = CorrelationToken(NonZeroU64::new(self.next).unwrap_or(NonZeroU64::MIN));
        self.live.insert(token.get(), 0);
        CorrelationLink::new(token, 0)
    }

    /// Re-mints `token`: bumps its epoch and returns the new link. Old
    /// links classify [`UnlinkedReason::SupersededEpoch`].
    ///
    /// Returns `None` when the token is unknown or expired.
    ///
    /// **Non-RT only** — map mutation.
    pub fn remint(&mut self, token: CorrelationToken) -> Option<CorrelationLink> {
        let epoch = self.live.get_mut(&token.get())?;
        *epoch = epoch.wrapping_add(1);
        Some(CorrelationLink::new(token, *epoch))
    }

    /// Tombstones `token`: its links classify [`UnlinkedReason::Expired`].
    /// Returns `true` when a live token was expired, `false` for an
    /// unknown or already-expired token.
    ///
    /// **Non-RT only** — map mutation.
    pub fn expire(&mut self, token: CorrelationToken) -> bool {
        if self.live.remove(&token.get()).is_some() {
            self.expired.insert(token.get());
            true
        } else {
            false
        }
    }

    /// Classifies `link` against the book — non-RT only.
    ///
    /// A live token whose current epoch matches the link's is
    /// [`CorrelationStatus::Linked`]; any other link is
    /// [`CorrelationStatus::Unlinked`] with the explicit reason.
    #[must_use]
    pub fn classify(&self, link: CorrelationLink) -> CorrelationStatus {
        match self.live.get(&link.token.get()) {
            Some(&epoch) if epoch == link.epoch => CorrelationStatus::Linked,
            Some(_) => CorrelationStatus::Unlinked(UnlinkedReason::SupersededEpoch),
            None if self.expired.contains(&link.token.get()) => {
                CorrelationStatus::Unlinked(UnlinkedReason::Expired)
            }
            None => CorrelationStatus::Unlinked(UnlinkedReason::UnknownToken),
        }
    }

    /// `classify` as a boolean — non-RT only.
    #[must_use]
    pub fn is_linked(&self, link: CorrelationLink) -> bool {
        matches!(self.classify(link), CorrelationStatus::Linked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_never_zero() {
        assert!(CorrelationToken::new(0).is_none());
        assert_eq!(CorrelationToken::new(9).unwrap().get(), 9);
    }

    #[test]
    fn issued_link_is_linked() {
        let mut book = CorrelationBook::new();
        let link = book.issue();
        assert_eq!(link.token().get(), 1);
        assert_eq!(link.epoch(), 0);
        assert_eq!(book.classify(link), CorrelationStatus::Linked);
        assert!(book.is_linked(link));
    }

    #[test]
    fn remint_supersedes_old_epoch() {
        let mut book = CorrelationBook::new();
        let old = book.issue();
        let new = book.remint(old.token()).unwrap();
        assert_eq!(new.token(), old.token());
        assert_eq!(new.epoch(), 1);
        assert_eq!(
            book.classify(old),
            CorrelationStatus::Unlinked(UnlinkedReason::SupersededEpoch)
        );
        assert_eq!(book.classify(new), CorrelationStatus::Linked);
    }

    #[test]
    fn expire_unlinks_forever() {
        let mut book = CorrelationBook::new();
        let link = book.issue();
        assert!(book.expire(link.token()));
        assert_eq!(
            book.classify(link),
            CorrelationStatus::Unlinked(UnlinkedReason::Expired)
        );
        // A tombstoned token cannot be re-minted back to life.
        assert!(book.remint(link.token()).is_none());
        // Re-expiring is a no-op.
        assert!(!book.expire(link.token()));
    }

    #[test]
    fn unknown_token_is_unlinked_not_linked() {
        let book = CorrelationBook::new();
        let link = CorrelationLink::new(CorrelationToken::new(42).unwrap(), 0);
        assert_eq!(
            book.classify(link),
            CorrelationStatus::Unlinked(UnlinkedReason::UnknownToken)
        );
        assert!(!book.is_linked(link));
    }

    #[test]
    fn remint_unknown_token_returns_none() {
        let mut book = CorrelationBook::new();
        assert!(book.remint(CorrelationToken::new(7).unwrap()).is_none());
    }
}
