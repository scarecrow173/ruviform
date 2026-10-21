//! The processing contract: [`Processor`] (non-RT preparation) and
//! [`RtProcessor`] (RT processing) — the type-level split of the
//! `prepare` / `process` lifecycle.

use crate::context::RtContext;

/// Non-RT construction / reconfiguration phase of a processor. Never
/// called from the RT path.
///
/// The two-trait split is the type-level realization of the design
/// baseline's "`prepare(setup)` produces a *separate* processing object
/// holding `process`" rule: `prepare` consumes `Setup` input and returns
/// the `Rt` object that is driven inside the RT contract section. With a
/// single trait, "has been prepared" could not be expressed at the type
/// level.
///
/// Associated types keep the contract generic — upper crates concretize
/// `Setup`/`Rt`/`Error`, and object-safety needs are resolved on the
/// consumer side rather than fixing a `dyn`-oriented shape in this
/// bottom-layer contract crate.
///
/// **RT contract:** the trait itself is non-RT machinery; see
/// [`prepare`](Self::prepare) for the callable API's contract.
///
/// # Examples
///
/// ```
/// use core::fmt;
/// use ruviform_rt::{Processor, RtContext, RtProcessor};
///
/// // The RT-side processing object.
/// struct Gain;
///
/// impl RtProcessor for Gain {
///     type Io = [f32; 4];
///
///     fn process(&mut self, io: &mut Self::Io, _ctx: &RtContext) {
///         io.fill(0.5);
///     }
/// }
///
/// // A non-RT processor that prepares `Gain` objects.
/// struct GainStage;
///
/// #[derive(Debug)]
/// struct Unsupported;
///
/// impl fmt::Display for Unsupported {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         f.write_str("unsupported setup")
///     }
/// }
///
/// impl core::error::Error for Unsupported {}
///
/// impl Processor for GainStage {
///     type Setup = f32;
///     type Rt = Gain;
///     type Error = Unsupported;
///
///     fn prepare(&mut self, _setup: Self::Setup) -> Result<Self::Rt, Self::Error> {
///         Ok(Gain)
///     }
/// }
///
/// let mut stage = GainStage;
/// let _processing = stage.prepare(0.5).unwrap();
/// ```
pub trait Processor {
    /// Setup input handed to `prepare` (configuration, layout, …).
    /// Defined by the caller.
    type Setup;

    /// The RT processing object produced by `prepare`.
    type Rt: RtProcessor;

    /// Failure classification of `prepare`: each processor binds its own
    /// error type via `core::error::Error` rather than the crate-wide
    /// [`Error`](crate::Error).
    type Error: core::error::Error;

    /// Builds or rebuilds the RT processing object from `setup`.
    ///
    /// **RT contract:** non-RT only — the construction/reconfiguration
    /// path may allocate, perform I/O, and block. Each call returns a new
    /// RT processing object; re-`prepare` is permitted at the type level
    /// and retiring the displaced `Rt` object is the caller's
    /// responsibility (e.g. via a retire channel).
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_rt::{Processor, RtContext, RtProcessor};
    ///
    /// struct Silence;
    ///
    /// impl RtProcessor for Silence {
    ///     type Io = ();
    ///     fn process(&mut self, _io: &mut Self::Io, _ctx: &RtContext) {}
    /// }
    ///
    /// struct Stage;
    ///
    /// impl Processor for Stage {
    ///     type Setup = ();
    ///     type Rt = Silence;
    ///     type Error = core::convert::Infallible;
    ///
    ///     fn prepare(&mut self, _setup: Self::Setup) -> Result<Self::Rt, Self::Error> {
    ///         Ok(Silence)
    ///     }
    /// }
    ///
    /// let mut stage = Stage;
    /// let _silence = stage.prepare(()).unwrap();
    /// ```
    fn prepare(&mut self, setup: Self::Setup) -> Result<Self::Rt, Self::Error>;
}

/// RT processing object driven inside an RT contract section.
///
/// Produced by [`Processor::prepare`] and driven per block via
/// [`process`](Self::process), which demands a `&RtContext` — code that
/// does not hold the capability token cannot call `process`, so
/// execution outside a contract section is prevented by capability
/// gating (`RtContext::mint` is `#[doc(hidden)]` — enforcement is
/// convention plus review, per the mechanism ADR).
///
/// `Io` stays generic: `ruviform-rt` is the bottom-layer crate and does
/// not depend on media types, so the buffer/event-block shape is
/// concretized by upper crates.
///
/// **RT contract:** see [`process`](Self::process) — the only method —
/// for the callable API's contract.
///
/// # Examples
///
/// ```
/// use ruviform_rt::{RtContext, RtProcessor};
///
/// struct Meter;
///
/// impl RtProcessor for Meter {
///     type Io = [f32; 8];
///
///     fn process(&mut self, io: &mut Self::Io, _ctx: &RtContext) {
///         io.fill(0.0);
///     }
/// }
///
/// // The driver owns the `RtContext` and calls `process` once per block.
/// fn drive(meter: &mut Meter, block: &mut [f32; 8], ctx: &RtContext) {
///     meter.process(block, ctx);
/// }
/// ```
pub trait RtProcessor {
    /// Block-level I/O type (buffers, event lists, …), concretized by the
    /// implementing processor.
    type Io;

    /// Processes one block.
    ///
    /// **RT contract:** RT-suitable — implementations must not
    /// allocate/deallocate, block, wait without bound, or perform
    /// unbounded work. Infallible by contract: processing failures are
    /// expressed as `io`-domain state, never as a returned `Result`.
    ///
    /// `ctx` is the proof of the RT contract section — an `RtContext`
    /// minted by the driver that started the section.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_rt::{RtContext, RtProcessor};
    ///
    /// struct Meter;
    ///
    /// impl RtProcessor for Meter {
    ///     type Io = [f32; 8];
    ///
    ///     fn process(&mut self, io: &mut Self::Io, _ctx: &RtContext) {
    ///         io.fill(0.0);
    ///     }
    /// }
    ///
    /// // `ctx: &RtContext` is required — the driver supplies it per
    /// // block inside the contract section.
    /// fn drive(meter: &mut Meter, block: &mut [f32; 8], ctx: &RtContext) {
    ///     meter.process(block, ctx);
    /// }
    /// ```
    fn process(&mut self, io: &mut Self::Io, ctx: &RtContext);
}
