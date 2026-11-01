//! Typed Graph IR → prepared audio plan → single-owner RT runtime
//! (Issue #195 / Migration M2).
//!
//! The non-RT side compiles a [`ruviform_graph::ValidatedGraph`] into
//! an immutable [`PreparedPlan`] ([`Compiler`]): deterministic
//! schedule, bounded buffer slots (max count / frames / alignment at
//! prepare), fan-in merge bindings and merge-point PDC delay lines.
//! The RT side ([`RtRuntime`]) is the single owner of the stable
//! processor slot registry and drives the schedule inside the RT
//! contract — no allocation, no blocking lock, no OS I/O, no
//! destructors; retired objects travel [`ruviform_rt::Retirer`]
//! channels to non-RT drains owned by [`GraphRuntime`].
//!
//! See `docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md`.

mod buffer;
mod compile;
mod io;
pub mod probe;
mod runtime;

pub use buffer::{AlignedSamples, DEFAULT_ALIGNMENT, DelayLine, Slot, SlotSpec};
pub use compile::{
    CompileError, CompiledPlan, Compiler, CompilerConfig, DelaySpec, EdgeBinding, FactoryError,
    InputBinding, Install, InstalledInfo, NodeFactory, PrepareOutcome, PreparedPlan, ScheduledNode,
    SlotAllocator,
};
pub use io::{InputView, NodeIo, OutputView};
pub use runtime::{
    ActivationError, GraphProcessor, GraphRuntime, NodeEvent, ProcessError, RtRuntime,
    RuntimeCommand, RuntimeFault, SharedStatus, SharedStatusSnapshot,
};
