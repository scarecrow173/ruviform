//! Output leg — scripted output device → production
//! [`ruviform_audio::Session`] playback path (Issue #628, 3rd slice).
//!
//! [`PlaybackPipeline::open`] hands the [`ScriptedDeviceHandle`] to
//! production [`Session::open`] as `&dyn Device` — the same call a
//! real backend's device object receives. `Session` internally
//! registers its RT `OutputDataCallback` that mints `RtContext` and
//! drives the active [`GraphPlan`]; the scripted backend's manual
//! [`ScriptedDeviceHandle::pump_output_next`] is the deterministic
//! stand-in for the backend RT thread, so every pump fires the real
//! production callback path (`plan.drive` → `Graph::pull` → node).
//!
//! The graph under test is caller-supplied; [`fixture_graph`] builds
//! the canonical single-node graph whose closure renders a
//! [`PcmFixture`] at absolute stream positions — the "what should the
//! output contain" side, written through the production
//! `AudioBufferViewMut` contract.
//!
//! ## Observed surface
//!
//! - [`PlaybackPipeline::captured`] — post-callback PCM the scripted
//!   backend recorded (`record_output`), as [`PcmBlock`]s.
//! - [`PlaybackPipeline::reports`] — per-pump [`PumpReport`]s
//!   (`frames`/`position`/`timestamps`/`errors_fired`/`invalidated`).
//!
//! ## Honest limits
//!
//! `Session::open` installs a **no-op error callback** — `DeviceError`
//! *values* are not observable through the production session surface;
//! only `PumpReport::errors_fired`/`invalidated` counts survive. That
//! is a production-surface limitation recorded as a finding, not
//! worked around here.
//!
//! **RT contract:** `open`/`pump`/`captured` are non-RT driver calls;
//! the alloc-free guarantee lives inside the callback the backend
//! fires (proven by `rt_pipeline_*` tests).

use std::sync::Arc;

use ruviform_audio::{Graph, GraphPlan, Session};
use ruviform_audio_core::AudioBufferViewMut;
use ruviform_audio_core::device::OutputStreamConfig;
use ruviform_rt::RtContext;

use crate::backend::{PumpReport, ScriptedDeviceHandle};
use crate::pcm::{PcmBlock, PcmFixture};
use crate::pipeline::PipelineError;

/// A live output pipeline: production `Session` over a scripted
/// device, plus the pump/capture observation surface.
///
/// Field order is irrelevant to correctness — the scripted stream
/// owns its callback state through `Arc`s independent of the handle.
pub struct PlaybackPipeline {
    session: Session,
    handle: ScriptedDeviceHandle,
    reports: Vec<PumpReport>,
}

impl PlaybackPipeline {
    /// Opens `handle`'s output stream through production
    /// [`Session::open`] and starts delivery.
    ///
    /// `graph` becomes the session's initial [`GraphPlan`]; further
    /// plans can be swapped via [`Session::swap`].
    ///
    /// # Errors
    ///
    /// [`PipelineError::Session`] when the device refuses the open or
    /// `start` — scripted open faults surface here unchanged.
    pub fn open(
        handle: &ScriptedDeviceHandle,
        config: OutputStreamConfig,
        graph: Graph,
    ) -> Result<Self, PipelineError> {
        let session = Session::open(handle, config, GraphPlan::new(graph))?;
        session.start()?;
        Ok(Self {
            session,
            handle: handle.clone(),
            // Diagnostic buffer preallocated so steady-state `pump`
            // stays allocation-free within the first 64 reports;
            // beyond that `pump` allocates — driver-side, non-RT.
            reports: Vec::with_capacity(64),
        })
    }

    /// Fires one output callback through the scripted backend — the
    /// deterministic stand-in for one backend RT tick. The production
    /// `Session` callback (`plan.drive`) runs inside.
    ///
    /// # Errors
    ///
    /// [`PipelineError::Device`] — `StreamInvalidated` after an
    /// `Invalidate` fault, `Backend("no_open_stream")`/vanish errors,
    /// or scripted start/stop faults.
    pub fn pump(&mut self) -> Result<PumpReport, PipelineError> {
        let report = self.handle.pump_output_next()?;
        self.reports.push(report);
        Ok(report)
    }

    /// Pumps `n` callbacks, collecting reports. Stops early on the
    /// first device error (returned).
    ///
    /// # Errors
    ///
    /// First [`PipelineError::Device`] encountered.
    pub fn pump_blocks(&mut self, n: usize) -> Result<Vec<PumpReport>, PipelineError> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(self.pump()?);
        }
        Ok(out)
    }

    /// Post-callback captured PCM blocks, oldest first — the observed
    /// side of the Expected/Observed comparison. Empty when the
    /// device spec had `record_output: false`.
    #[must_use]
    pub fn captured(&self) -> Vec<PcmBlock> {
        self.handle.captured_output()
    }

    /// Per-pump reports accumulated so far.
    #[must_use]
    pub fn reports(&self) -> &[PumpReport] {
        &self.reports
    }

    /// The production session (format negotiation, plan swap, retire
    /// drains).
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Stops the stream. Dropping also stops it (backend-defined).
    ///
    /// # Errors
    ///
    /// [`PipelineError::Session`] when the backend refuses.
    pub fn stop(&self) -> Result<(), PipelineError> {
        self.session.stop()?;
        Ok(())
    }
}

impl core::fmt::Debug for PlaybackPipeline {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PlaybackPipeline")
            .field("reports", &self.reports.len())
            .field("output_open", &self.handle.is_output_open())
            .finish_non_exhaustive()
    }
}

/// Builds the canonical output graph: one closure node that renders
/// `fixture` at absolute stream positions into the callback's
/// `AudioBufferViewMut`.
///
/// The node keeps a private cursor advanced by `out.frames()` per
/// pull. While the stream position advances contiguously by the same
/// frame counts, cursor == reported `position`; a `FramePosition`
/// fault would desynchronize them (documented input-only fault —
/// see the slice-2 backend notes).
///
/// On a channel-count mismatch (`fill_view` error — a configuration
/// bug the device open should have rejected) the node writes silence
/// rather than propagating stale scratch: deterministic, observable.
#[must_use]
pub fn fixture_graph(fixture: Arc<PcmFixture>) -> Graph {
    let mut graph = Graph::new();
    let mut cursor = 0u64;
    graph.push(Box::new(
        move |out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext| {
            let frames = u64::try_from(out.frames()).unwrap_or(u64::MAX);
            if fixture.fill_view(cursor, out).is_err() {
                for ch in 0..out.channels() {
                    if let Some(plane) = out.channel_mut(ch) {
                        plane.fill(0.0);
                    }
                }
            }
            cursor = cursor.wrapping_add(frames);
        },
    ));
    graph
}
