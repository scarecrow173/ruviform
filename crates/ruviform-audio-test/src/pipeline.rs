//! Deterministic pipeline wiring (Issue #628, 3rd slice).
//!
//! Feeds the scripted port-trait [`crate::backend`] devices into
//! **production consumer contracts** and compares Expected vs
//! Observed at the pipeline boundary:
//!
//! - [`output::PlaybackPipeline`] — scripted output device opened
//!   through production [`ruviform_audio::Session::open`]; manual
//!   `pump_output_next` drives the production RT `plan.drive`
//!   callback; captured PCM is compared against the independent
//!   [`crate::oracle`] reference.
//! - [`input::InputPipeline`] — scripted input stream whose data
//!   callback hands bounded PCM blocks through a preallocated SPSC
//!   capture into the production typed runtime
//!   (`RtRuntime::input_writer` → `process` → `output_reader`).
//! - [`midi::MidiLeg`] — `MidiFeed`/`MockPort` events through the
//!   production `InputStage` staging/pump/`collect_block` path
//!   (`ClockAnchor` + `map_stamp_to_frame`).
//!
//! ## Honesty boundary
//!
//! Everything here runs on the test driver's thread against the
//! deterministic scripted backend. A Pass is **OS-L1 deterministic
//! evidence only** — it is not evidence of WASAPI/WinMM, real
//! windows, plugin ABI, or real-thread RT behavior. Timestamp/capture
//! provenance is observed as metadata; surfaces the production
//! consumer cannot carry (e.g. `Session`'s no-op error callback)
//! are recorded as findings, not papered over.
//!
//! ## RT contract
//!
//! The callback-side pieces ([`input::InputCapture`] offer path,
//! `GainNode`/typed `process`, `InputStage::collect_block`) are
//! allocation-free and proven by `rt_pipeline_*` tests under
//! `assert_no_alloc`. All driver orchestration (`pump_*_next`,
//! handoff drain, expected-value computation) is non-RT.

mod common;
pub mod input;
pub mod midi;
pub mod output;

pub use common::{
    compare_pcm_block, compare_planar, expected_block, expected_fixture_block, scale_expected,
};
pub use input::{
    CaptureBlock, CaptureCounters, InputCapture, InputPipeline, ObservedBlock,
    pipeline_compiler_config,
};
pub use midi::{BlockObservation, CollectedBlock, InputPump, InputPumpStats, MidiLeg};
pub use output::{PlaybackPipeline, fixture_graph};

use ruviform_audio::SessionError;
use ruviform_audio::typed::{ActivationError, CompileError, ProcessError};
use ruviform_audio_core::device::DeviceError;
use ruviform_graph::ValidationErrors;

/// Failure of a pipeline leg — wraps the production error type at
/// the boundary where it surfaced so callers see *which* contract
/// rejected, not a flattened string.
///
/// **RT contract:** non-RT only — error reporting.
#[non_exhaustive]
#[derive(Debug)]
pub enum PipelineError {
    /// The scripted device port rejected the call (`DeviceError`).
    Device(DeviceError),
    /// `Session::open`/`start`/`stop` rejected (output leg).
    Session(SessionError),
    /// Graph build/validation failed before compile.
    Graph(ValidationErrors),
    /// The typed `Compiler` rejected the prepared graph.
    Compile(CompileError),
    /// `GraphRuntime::publish` could not queue the install delta.
    Activation(ActivationError),
    /// `RtRuntime::process` rejected the block (e.g. frames budget).
    Process(ProcessError),
    /// Pipeline-internal configuration inconsistency (channel/layout
    /// mismatch the production contract cannot express).
    Config(&'static str),
}

impl core::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Device(e) => write!(f, "device error: {e:?}"),
            Self::Session(e) => write!(f, "session error: {e}"),
            Self::Graph(e) => write!(f, "graph validation error: {e}"),
            Self::Compile(e) => write!(f, "compile error: {e}"),
            Self::Activation(e) => write!(f, "activation error: {e}"),
            Self::Process(e) => write!(f, "process error: {e}"),
            Self::Config(what) => write!(f, "pipeline config error: {what}"),
        }
    }
}

impl std::error::Error for PipelineError {}

impl From<DeviceError> for PipelineError {
    fn from(e: DeviceError) -> Self {
        Self::Device(e)
    }
}

impl From<SessionError> for PipelineError {
    fn from(e: SessionError) -> Self {
        Self::Session(e)
    }
}

impl From<CompileError> for PipelineError {
    fn from(e: CompileError) -> Self {
        Self::Compile(e)
    }
}

impl From<ActivationError> for PipelineError {
    fn from(e: ActivationError) -> Self {
        Self::Activation(e)
    }
}

impl From<ProcessError> for PipelineError {
    fn from(e: ProcessError) -> Self {
        Self::Process(e)
    }
}
