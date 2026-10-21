//! The [`RtContext`] capability token — proof that the caller is inside
//! an RT contract section.

/// Capability token marking that the caller is inside an RT contract
/// section.
///
/// `RtContext` is the `prepare` / `process` boundary of the
/// [`Processor`](crate::Processor) contract: `prepare(setup)` runs in the
/// non-RT section (allocation allowed) and produces the processing object
/// whose [`process`](crate::RtProcessor::process) API requires this
/// token. Code that does not hold an `RtContext` cannot call `process`.
///
/// Issuance rule: `RtContext` can only be minted by a driver that has
/// started an RT contract section — the RT driver (`ruviform-audio`,
/// inside the `ruviform-audio-device` device callback) or the offline
/// driver (`ruviform-exec`'s non-RT driver, where no-alloc constraints do
/// not apply). Application code must not be able to construct it: the
/// private field blocks literal construction, and the only constructor —
/// [`mint`](Self::mint) — is `#[doc(hidden)]` and reserved for driver
/// entry points by convention (enforcement relies on code review, as
/// documented in the mechanism ADR).
///
/// `RtContext` is a single opaque type: it carries no runtime mode flag
/// or variant distinguishing offline from RT issuance — the "no `if rt`
/// branching inside the same API" principle. The relaxation of RT
/// constraints under an offline-issued context is a contract-level
/// property bound to the issuing driver, not runtime state.
///
/// It is a reference token only: no clone/copy, no data, no behavior.
///
/// **RT contract:** RT-suitable — the token is zero-sized; holding and
/// passing `&RtContext` never allocates or blocks.
///
/// # Examples
///
/// ```
/// use ruviform_rt::RtContext;
///
/// // APIs on the RT path take `&RtContext` as proof of the contract
/// // section. Only driver entry points can mint the token — see
/// // `RtProcessor::process` for how the token is consumed.
/// fn needs_context(_ctx: &RtContext) {}
/// ```
#[derive(Debug)]
pub struct RtContext {
    _private: (),
}

impl RtContext {
    /// Mints an `RtContext` token.
    ///
    /// **RT contract:** RT-suitable — pure token construction at contract
    /// section start. No allocation, no blocking.
    ///
    /// Reserved for driver entry points (the RT driver's device callback
    /// and the offline driver). The restriction is enforced at the
    /// convention level — `#[doc(hidden)]` plus a drivers-only usage
    /// contract plus code review — and is not a complete type-level
    /// prevention (recorded in
    /// `docs/adr/rt-foundation-primitives-implementation.md` as a
    /// documented weakening of the baseline wording).
    #[doc(hidden)]
    #[must_use]
    pub fn mint() -> Self {
        Self { _private: () }
    }
}
