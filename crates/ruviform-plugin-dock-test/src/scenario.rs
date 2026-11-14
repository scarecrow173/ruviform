//! Plugin-dock scenario cases (Issue #631 first slice, Issue #389
//! session-lifecycle cases).
//!
//! [`run_all`] executes the eight cases of the `rf631.plugin-dock`
//! scenario and returns one [`ScenarioReport`] per case:
//!
//! | case                | driver            | honest outcome today                        |
//! |---------------------|-------------------|---------------------------------------------|
//! | `view-mount`        | `Fake`            | `Pass` — semantic `NativeSlot` spec mounts  |
//! | `artifact`          | `PluginHostProbe` | `Pass`/`NotRun`/`Fail` — resolve+hash+load  |
//! | `host-lifecycle`    | `PluginHostProbe` | `Pass`/`NotRun` — Scan→…→Activate→Destroy   |
//! | `editor-attach`     | `PluginHostProbe` | `Unsupported` — `createView` null today     |
//! | `session-lifecycle` | `PluginHostProbe` | `Pass`/`NotRun` — begin/busy/refused-open/  |
//! |                     |                   | close/end/re-begin on the real binary       |
//! | `host-terminate`    | `PluginHostProbe` | `Pass`/`NotRun` — Destroy→notify→          |
//! |                     |                   | `Terminated{HostLost}` exactly once         |
//! | `open-or-degrade`   | `PluginHostProbe` | `Pass`/`NotRun` — Docked+Floating double    |
//! |                     |                   | refusal, zero `Degraded` events             |
//! | `dock-pixels`       | `PluginHostProbe` | `NotRun` — no surface, no `VisualCapture`   |
//!
//! `view-mount` proves the driver-neutral `NativeSlot` [`ViewSpec`]
//! mounts under the semantic fake — contract conformance only, never
//! dock evidence. The probe cases gate on artifact presence inside
//! the body: a missing binary is `NotRun` (fail-closed), an unloadable
//! one is `Fail`, and every step that really ran against the binary
//! may claim `EnvironmentKind::RealAbiTestPlugin`.
//!
//! **RT contract:** non-RT only — dlopen, host dispatch, filesystem.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ruviform_audio_core::SampleRate;
use ruviform_audio_test::EnvironmentKind;
use ruviform_plugin_core::{PluginFactory, ProcessConfig};
use ruviform_plugin_host::{
    AttachMode, EditorCapabilities, EditorOpError, EditorSessionError, EditorSessionEvent,
    EditorSessionOps, EditorSessionState, EditorSize, HostCommand, HostError, HostReply,
    InProcessEditorOps, InProcessHost, ParentLease, ParentSlotLease, PluginEditorSession,
    PluginEditorSessionId, PluginEditorSessions, PluginHandle, SessionFailCause,
};
use ruviform_plugin_vst3::host::Vst3Library;
use ruviform_ui::WindowId;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::native_slot::{
    LogicalRect, NativeSurfaceSlot, SlotCapabilities, SlotGeometry, SlotLease, SlotSpec,
    SlotVisibility, Support, SurfaceSlotId,
};
use ruviform_ui_test::scenario::plugin_dock::{
    PLUGIN_DOCK_SCENARIO, PLUGIN_DOCK_SLOT_KEY, PluginArtifactEvidence, plugin_dock_view_spec,
};
use ruviform_ui_test::{
    CaseId, FakeGuiTest, FakeNativeSlot, RunId, ScenarioCapability, ScenarioId, ScenarioOutcome,
    ScenarioReport, ScenarioSpec, ScenarioTarget, ScenarioVerdict, SemanticGuiTest, TargetQuery,
    run_scenario,
};
use serde_json::{Map, Value, json};

use crate::artifact;
use crate::probe::PluginHostProbe;

/// The run id shared by every case of this lane.
pub const RUN_ID: &str = "rf631-plugin-dock";

/// Lane-level inputs the binary fills from the environment.
#[derive(Debug, Clone, Default)]
pub struct LaneEnvironment {
    /// Repository commit the evidence was produced at (`git rev-parse
    /// HEAD` / `GITHUB_SHA`); `None` records an honest `null`.
    pub repo_sha: Option<String>,
    /// Explicit artifact path override — the [`artifact::PLUGIN_PATH_ENV`]
    /// value. `None` falls back to `current_exe`-relative auto lookup.
    pub plugin_path: Option<PathBuf>,
}

/// Runs every plugin-dock case and returns the reports in case order.
///
/// The artifact is resolved/hashed/loaded once and shared: the
/// `artifact` case performs the load (its verdict covers
/// resolve/hash/load/discover) and later cases reuse the library —
/// a load failure leaves `library_slot` empty and dependents report
/// `NotRun` (prerequisite absent), never a fabricated pass.
#[must_use]
pub fn run_all(env: &LaneEnvironment) -> Vec<ScenarioReport> {
    let run_id = RunId::new(RUN_ID);
    let mut reports = Vec::with_capacity(8);

    reports.push(run_view_mount(&run_id, env));

    let mut library_slot = None;
    let mut artifact_slot = None;
    reports.push(run_artifact_case(
        &run_id,
        env,
        &mut library_slot,
        &mut artifact_slot,
    ));

    reports.push(run_host_lifecycle(
        &run_id,
        env,
        library_slot.as_ref(),
        artifact_slot.as_ref(),
    ));
    reports.push(run_editor_attach(
        &run_id,
        env,
        library_slot.as_ref(),
        artifact_slot.as_ref(),
    ));
    reports.push(run_session_lifecycle(
        &run_id,
        env,
        library_slot.as_ref(),
        artifact_slot.as_ref(),
    ));
    reports.push(run_host_terminate(
        &run_id,
        env,
        library_slot.as_ref(),
        artifact_slot.as_ref(),
    ));
    reports.push(run_open_or_degrade(
        &run_id,
        env,
        library_slot.as_ref(),
        artifact_slot.as_ref(),
    ));
    reports.push(run_dock_pixels(&run_id, env));

    reports
}

// ---------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------

/// The `environment` object recorded on every report: host facts,
/// the honest `environment_kind`, optional plugin-artifact identity
/// and per-case extras.
fn case_environment(
    env: &LaneEnvironment,
    kind: EnvironmentKind,
    artifact: Option<&PluginArtifactEvidence>,
    extra: Map<String, Value>,
) -> Value {
    let mut object = Map::from_iter([
        ("os".to_string(), json!(std::env::consts::OS)),
        ("arch".to_string(), json!(std::env::consts::ARCH)),
        // Headless probe — no real window exists, so no meaningful
        // scale factor; `null` is the honest record.
        ("scale_factor".to_string(), Value::Null),
        (
            "repo_sha".to_string(),
            env.repo_sha.clone().map_or(Value::Null, Value::String),
        ),
        ("environment_kind".to_string(), json!(kind.as_str())),
    ]);
    if let Some(artifact) = artifact {
        object.insert(
            "plugin_artifact".to_string(),
            artifact.environment_value()["plugin_artifact"].clone(),
        );
    }
    object.extend(extra);
    Value::Object(object)
}

/// Post-run report finalization: environment + artifact evidence ref.
fn finalize(
    mut report: ScenarioReport,
    env: &LaneEnvironment,
    kind: EnvironmentKind,
    artifact: Option<&PluginArtifactEvidence>,
    extra: Map<String, Value>,
) -> ScenarioReport {
    report.environment = Some(case_environment(env, kind, artifact, extra));
    if let Some(artifact) = artifact {
        report.evidence.push(artifact.evidence_ref());
    }
    report
}

/// A `NotRun` outcome with `detail` — the fail-closed shape for
/// absent prerequisites.
fn not_run(detail: impl Into<String>) -> ScenarioOutcome {
    let mut outcome = ScenarioOutcome::new(ScenarioVerdict::NotRun);
    outcome.detail = Some(detail.into());
    outcome
}

/// A `Fail` outcome carrying `detail`.
fn fail(detail: impl Into<String>) -> ScenarioOutcome {
    let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Fail);
    outcome.detail = Some(detail.into());
    outcome
}

/// Maps a host-command error to an honest outcome: the defined
/// `Unsupported` refusal stays `Unsupported`; everything else is a
/// real `Fail`.
fn host_error_outcome(step: &str, error: HostError) -> ScenarioOutcome {
    let verdict = match error {
        HostError::Unsupported => ScenarioVerdict::Unsupported,
        _ => ScenarioVerdict::Fail,
    };
    let mut outcome = ScenarioOutcome::new(verdict);
    outcome.detail = Some(format!("host step `{step}`: {error}"));
    outcome
}

/// The plugin-dock [`ScenarioSpec`] for `case` on the probe driver —
/// `required` stays empty because the probe honestly claims no
/// capability; artifact presence gates inside the body. `claim`
/// records what evidence level the case asserts.
fn probe_spec(run_id: &RunId, case: &str, claim: &str) -> ScenarioSpec {
    let mut spec = ScenarioSpec::new(run_id.clone(), ScenarioId::new(PLUGIN_DOCK_SCENARIO))
        .case(CaseId::new(case))
        .required(BTreeSet::new());
    spec.environment = Some(json!({ "claim": claim }));
    spec
}

/// The defined process configuration for `Activate`: 48 kHz, block
/// range 1..=512.
fn test_gain_config() -> Option<ProcessConfig> {
    let sample_rate = SampleRate::new(48_000).ok()?;
    ProcessConfig::new(sample_rate, 1, 512).ok()
}

/// Scan → Instantiate → Initialize → Activate(48 kHz, 1..=512) for
/// `plugin_id` — the defined process configuration. The caller owns
/// Deactivate/Destroy teardown. Each step's error propagates as
/// [`HostError`]; reply-shape violations map to `PluginFailed`.
fn activate_test_gain(
    host: &mut InProcessHost,
    plugin_id: &str,
) -> Result<PluginHandle, HostError> {
    match host.execute(HostCommand::Scan)? {
        HostReply::Scanned(descriptors) => {
            if !descriptors.iter().any(|d| d.id == plugin_id) {
                return Err(HostError::UnknownPluginId);
            }
        }
        _ => return Err(HostError::PluginFailed),
    }
    let HostReply::Instantiated { handle } = host.execute(HostCommand::Instantiate {
        plugin_id: plugin_id.to_string(),
    })?
    else {
        return Err(HostError::PluginFailed);
    };
    host.execute(HostCommand::Initialize { handle })?;
    let config = test_gain_config().ok_or(HostError::InvalidValue)?;
    host.execute(HostCommand::Activate { handle, config })?;
    Ok(handle)
}

/// Registers every factory `library` scanned with `host`.
fn register_factories(host: &mut InProcessHost, library: &Vst3Library) {
    for factory in library.factories() {
        host.register_factory(Box::new(factory) as Box<dyn PluginFactory>);
    }
}

/// The defined editor size every session case requests — matches the
/// fake dock slot's 640×480 region.
fn editor_size() -> EditorSize {
    EditorSize {
        width: 640.0,
        height: 480.0,
    }
}

/// `InProcessEditorOps` redeeming session leases against `slot`'s
/// ledger — the issuer-side seam every attach attempt shares: the
/// session-facing `ParentSlotLease` payload decodes to a `SlotLease`,
/// `validate_lease` proves it live, and the minted `ParentLease`
/// carries the registry sequence as its generation. A stale/forged
/// lease never reaches the plugin.
fn editor_ops<'a>(
    host: &'a mut InProcessHost,
    handle: PluginHandle,
    slot: &'a FakeNativeSlot,
) -> InProcessEditorOps<'a> {
    InProcessEditorOps::with_redeemer(
        host,
        handle,
        Box::new(move |session_lease| {
            let raw = SlotLease::new(session_lease.raw());
            slot.validate_lease(raw)
                .map(|info| ParentLease::new(info.seq, raw.raw()))
                .map_err(|_| EditorOpError::LeaseRejected)
        }),
    )
}

/// `InProcessEditorOps` whose redeemer always refuses — for converge
/// paths (`close`) where the lease must never be consulted.
fn rejecting_editor_ops(host: &mut InProcessHost, handle: PluginHandle) -> InProcessEditorOps<'_> {
    InProcessEditorOps::with_redeemer(
        host,
        handle,
        Box::new(|_| Err(EditorOpError::LeaseRejected)),
    )
}

/// `EditorSessionOps` decorator counting `open`/`close` dispatches —
/// lets a case prove a converge path never reached the adapter: a
/// refused open leaves `editor_live == false`, so `close` must not
/// dispatch `CloseGui` at all.
struct CountingEditorOps<'a> {
    inner: InProcessEditorOps<'a>,
    opens: usize,
    closes: usize,
}

impl<'a> CountingEditorOps<'a> {
    /// Wraps [`editor_ops`] — the validating-lease redeemer.
    fn new(host: &'a mut InProcessHost, handle: PluginHandle, slot: &'a FakeNativeSlot) -> Self {
        Self {
            inner: editor_ops(host, handle, slot),
            opens: 0,
            closes: 0,
        }
    }
}

impl EditorSessionOps for CountingEditorOps<'_> {
    fn capabilities(
        &self,
        session: PluginEditorSessionId,
    ) -> Result<EditorCapabilities, EditorOpError> {
        self.inner.capabilities(session)
    }

    fn open(
        &mut self,
        session: PluginEditorSessionId,
        lease: &ParentSlotLease,
        size: EditorSize,
        mode: AttachMode,
    ) -> Result<EditorCapabilities, EditorOpError> {
        self.opens += 1;
        self.inner.open(session, lease, size, mode)
    }

    fn close(&mut self, session: PluginEditorSessionId) -> Result<(), EditorOpError> {
        self.closes += 1;
        self.inner.close(session)
    }

    fn set_size(
        &mut self,
        session: PluginEditorSessionId,
        size: EditorSize,
    ) -> Result<(), EditorOpError> {
        self.inner.set_size(session, size)
    }

    fn set_visibility(
        &mut self,
        session: PluginEditorSessionId,
        visible: bool,
    ) -> Result<(), EditorOpError> {
        self.inner.set_visibility(session, visible)
    }

    fn reattach(
        &mut self,
        session: PluginEditorSessionId,
        lease: &ParentSlotLease,
    ) -> Result<(), EditorOpError> {
        self.inner.reattach(session, lease)
    }
}

/// Mutable state the session cases fill in — the registry, the fake
/// slot, every begun session id and the leased surface — so
/// [`teardown_editor_case`] can retire everything on any exit path.
#[derive(Default)]
struct SessionCaseState {
    sessions: PluginEditorSessions,
    begun: Vec<PluginEditorSessionId>,
    slot: FakeNativeSlot,
    surface: Option<SurfaceSlotId>,
    lease: Option<SlotLease>,
}

/// JSON view of a `PluginEditorSessionId` — `{slot, generation}`.
fn session_id_json(id: PluginEditorSessionId) -> Value {
    json!({ "slot": id.slot(), "generation": id.generation() })
}

/// Debug rendering of drained session events for `observed` — the
/// event vocabulary is small and stable, so `Debug` is the honest
/// record (the session types carry no serde impl).
fn events_debug(events: &[EditorSessionEvent]) -> Vec<String> {
    events.iter().map(|event| format!("{event:?}")).collect()
}

/// `true` when `id`'s session sits in `Failed{Unsupported}` — the
/// honest refused-open landing for the editor-less `test_gain`.
fn session_failed_unsupported(sessions: &PluginEditorSessions, id: PluginEditorSessionId) -> bool {
    sessions.session(id).is_some_and(|session| {
        matches!(
            session.state(),
            EditorSessionState::Failed {
                cause: SessionFailCause::Unsupported
            }
        )
    })
}

/// `true` when `id`'s session sits in `Closed` and concluded.
fn session_closed_concluded(sessions: &PluginEditorSessions, id: PluginEditorSessionId) -> bool {
    sessions.session(id).is_some_and(|session| {
        matches!(session.state(), EditorSessionState::Closed) && session.concluded()
    })
}

/// `begin_session` for a case — records the id in `state.begun` so
/// teardown can retire it, and logs the step.
fn begin_case_session(
    state: &mut SessionCaseState,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    step_name: &str,
) -> Result<PluginEditorSessionId, String> {
    let id = state
        .sessions
        .begin_session(handle)
        .map_err(|error| format!("{step_name}: {error}"))?;
    state.begun.push(id);
    steps.push(json!({
        "step": step_name,
        "status": "ok",
        "session": session_id_json(id),
    }));
    Ok(id)
}

/// Declares the fake dock slot and mints the parent lease — the
/// issuer-side half every session case shares. Records both steps and
/// returns the session-facing [`ParentSlotLease`].
fn declare_fake_dock_lease(
    state: &mut SessionCaseState,
    steps: &mut Vec<Value>,
) -> Result<ParentSlotLease, String> {
    let surface = state
        .slot
        .declare_slot(fake_dock_slot_spec())
        .map_err(|error| format!("declare_slot: {error}"))?;
    state.surface = Some(surface);
    steps.push(json!({ "step": "declare_slot", "status": "ok" }));
    let lease = state
        .slot
        .lease_parent(surface)
        .map_err(|error| format!("lease_parent: {error}"))?;
    state.lease = Some(lease);
    steps.push(json!({ "step": "lease_parent", "status": "ok" }));
    Ok(ParentSlotLease::new(lease.raw()))
}

/// Drains `id`'s events and records the step — the caller asserts on
/// the returned list. Takes the registry directly so it can run while
/// an [`CountingEditorOps`] still borrows `state.slot`.
fn drain_case_events(
    sessions: &mut PluginEditorSessions,
    id: PluginEditorSessionId,
    steps: &mut Vec<Value>,
    step_name: &str,
) -> Vec<EditorSessionEvent> {
    let events = sessions
        .session_mut(id)
        .map_or_else(Vec::new, PluginEditorSession::drain_events);
    let rendered = events_debug(&events);
    steps.push(json!({
        "step": step_name,
        "status": "ok",
        "events": rendered,
    }));
    events
}

/// Asserts a second `begin_session` on the same live-bound instance
/// is refused with `SessionBusy` — the registry enforces at most one
/// non-concluded session per plugin instance. Records the step.
fn assert_begin_session_busy(
    sessions: &mut PluginEditorSessions,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> Result<(), String> {
    match sessions.begin_session(handle) {
        Err(EditorSessionError::SessionBusy) => {
            steps.push(json!({
                "step": "begin_session_again",
                "status": "ok",
                "result": "session-busy",
            }));
            Ok(())
        }
        Ok(unexpected) => {
            steps.push(json!({
                "step": "begin_session_again",
                "status": "error",
                "result": "unexpected-success",
                "session": session_id_json(unexpected),
            }));
            Err("second begin_session on a live-bound instance succeeded — \
                 SessionBusy contract violated"
                .to_string())
        }
        Err(error) => {
            steps.push(json!({
                "step": "begin_session_again",
                "status": "error",
                "error": error.to_string(),
            }));
            Err(format!("second begin_session: {error}"))
        }
    }
}

/// Asserts a refused open landed honestly against the editor-less
/// `test_gain`: `Err(Ops(Unsupported))`, `Failed{Unsupported}` state
/// and exactly `[Failed{Unsupported}]` drained events. Records the
/// open/state/drain steps. Takes the registry directly so it can run
/// while an [`CountingEditorOps`] still borrows `state.slot`.
fn assert_refused_open(
    sessions: &mut PluginEditorSessions,
    id: PluginEditorSessionId,
    ops: &mut CountingEditorOps<'_>,
    open_result: Result<(), EditorSessionError>,
    steps: &mut Vec<Value>,
    step_name: &str,
) -> Result<(), String> {
    match open_result {
        Err(EditorSessionError::Ops(EditorOpError::Unsupported)) => {
            steps.push(json!({
                "step": step_name,
                "status": "ok",
                "result": "refused-unsupported",
                "ops_opens": ops.opens,
            }));
        }
        other => {
            steps.push(json!({
                "step": step_name,
                "status": "error",
                "result": format!("{other:?}"),
            }));
            return Err(format!(
                "{step_name}: expected Ops(Unsupported), got {other:?}"
            ));
        }
    }
    if !session_failed_unsupported(sessions, id) {
        return Err(format!(
            "session did not land Failed{{Unsupported}} after {step_name}"
        ));
    }
    steps.push(json!({
        "step": "state_after_refused_open",
        "status": "ok",
        "state": "failed-unsupported",
    }));
    let events = sessions
        .session_mut(id)
        .map_or_else(Vec::new, PluginEditorSession::drain_events);
    let rendered = events_debug(&events);
    steps.push(json!({
        "step": "drain_events_after_open",
        "status": "ok",
        "events": rendered,
    }));
    let failed_once = events.len() == 1
        && matches!(
            events[0],
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Unsupported
            }
        );
    if !failed_once {
        return Err(format!(
            "expected exactly [Failed{{Unsupported}}] after {step_name}, got {rendered:?}"
        ));
    }
    Ok(())
}

/// Runs `session.open(Docked)` through `ops` and asserts the honest
/// refused-open landing via [`assert_refused_open`] — shared by the
/// cases whose fixture refuses every attach mode.
fn refused_docked_open(
    sessions: &mut PluginEditorSessions,
    id: PluginEditorSessionId,
    ops: &mut CountingEditorOps<'_>,
    lease: ParentSlotLease,
    steps: &mut Vec<Value>,
) -> Result<(), String> {
    let open_result = sessions
        .session_mut(id)
        .map(|session| {
            session
                .open(ops, &lease, editor_size(), AttachMode::Docked)
                .map(|_| ())
        })
        .ok_or_else(|| "editor session vanished before open".to_string())?;
    assert_refused_open(sessions, id, ops, open_result, steps, "open_docked")
}

/// Asserts `close` converges `id` to `Closed`+`concluded` without an
/// adapter `CloseGui` dispatch (`ops.closes == 0` — `editor_live`
/// stayed false after the refused open) and emits exactly `[Closed]`.
/// Records the steps.
fn assert_close_converge(
    sessions: &mut PluginEditorSessions,
    id: PluginEditorSessionId,
    ops: &mut CountingEditorOps<'_>,
    steps: &mut Vec<Value>,
) -> Result<(), String> {
    let close_result = sessions
        .session_mut(id)
        .map(|session| session.close(ops))
        .ok_or_else(|| "editor session vanished before close".to_string())?;
    close_result.map_err(|error| format!("session.close: {error}"))?;
    steps.push(json!({
        "step": "close_converge",
        "status": "ok",
        "ops_closes": ops.closes,
    }));
    if ops.closes != 0 {
        return Err(format!(
            "adapter close dispatched {} time(s) for a refused open",
            ops.closes
        ));
    }
    if !session_closed_concluded(sessions, id) {
        return Err("session did not converge to Closed+concluded".to_string());
    }
    steps.push(json!({
        "step": "state_after_close",
        "status": "ok",
        "state": "closed-concluded",
    }));
    let events = sessions
        .session_mut(id)
        .map_or_else(Vec::new, PluginEditorSession::drain_events);
    let rendered = events_debug(&events);
    steps.push(json!({
        "step": "drain_events_after_close",
        "status": "ok",
        "events": rendered,
    }));
    if !(events.len() == 1 && matches!(events[0], EditorSessionEvent::Closed)) {
        return Err(format!(
            "expected exactly [Closed] after close, got {rendered:?}"
        ));
    }
    Ok(())
}

/// `end_session` + instance-binding-release assertion — after `end`
/// the registry must hold no session for `handle`. Records both
/// steps; `end_step` names the end step in the log.
fn end_case_session(
    sessions: &mut PluginEditorSessions,
    id: PluginEditorSessionId,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    end_step: &str,
) -> Result<(), String> {
    sessions
        .end_session(id)
        .map_err(|error| format!("{end_step}: {error}"))?;
    steps.push(json!({ "step": end_step, "status": "ok" }));
    if sessions.session_for_instance(handle).is_some() {
        return Err("instance binding still present after end_session".to_string());
    }
    steps.push(json!({
        "step": "session_for_instance",
        "status": "ok",
        "result": "none",
    }));
    Ok(())
}

/// Best-effort teardown shared by the session cases: converges any
/// still-live session for `handle` via `notify_terminated` (the
/// registry-level host-loss path), retires every begun session id,
/// releases the fake slot lease/surface and destroys the instance.
/// Runs on every exit path; the notes land in `observed.cleanup`.
fn teardown_editor_case(
    host: &mut InProcessHost,
    handle: PluginHandle,
    state: &mut SessionCaseState,
) -> Vec<String> {
    state
        .sessions
        .notify_terminated(handle, SessionFailCause::HostLost);
    for &id in &state.begun {
        let _ = state.sessions.end_session(id);
    }
    if let Some(lease) = state.lease {
        let _ = state.slot.release_lease(lease);
    }
    if let Some(surface) = state.surface {
        let _ = state.slot.release_slot(surface);
    }
    let _ = host.execute(HostCommand::Deactivate { handle });
    let _ = host.execute(HostCommand::Destroy { handle });
    vec![
        "sessions-retire-attempted".to_string(),
        "slot-release-attempted".to_string(),
        "instance-destroy-attempted".to_string(),
    ]
}

// ---------------------------------------------------------------------
// case: view-mount (semantic fake — contract conformance only)
// ---------------------------------------------------------------------

/// `view-mount`: mounts [`plugin_dock_view_spec`] — a `Panel` holding
/// one `NativeSlot` — on the `Fake` adapter and observes the slot
/// component. `required = {SemanticAction}` only: this case proves
/// the declarative spec mounts and is observable; it is *not* dock,
/// window, input, or pixel evidence.
fn run_view_mount(run_id: &RunId, env: &LaneEnvironment) -> ScenarioReport {
    let mut gui = FakeGuiTest::new();
    let spec = ScenarioSpec::new(run_id.clone(), ScenarioId::new(PLUGIN_DOCK_SCENARIO))
        .case(CaseId::new("view-mount"))
        .required(BTreeSet::from([ScenarioCapability::SemanticAction]));
    let report = run_scenario(&mut gui, &spec, |gui| {
        let window = gui.open_window();
        let view = ViewInstanceId::new(1).expect("view id 1 is non-zero");
        gui.mount_view(window, view, plugin_dock_view_spec())?;

        let slot_key = ruviform_ui::view::ComponentKey(PLUGIN_DOCK_SLOT_KEY.to_string());
        let mut query = TargetQuery::new(window, view);
        query.component = Some(slot_key.clone());
        let target: ScenarioTarget = gui.resolve_target(&query)?;
        gui.observe(&target, &slot_key)?;

        let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
        outcome
            .evidence
            .push(format!("semantic-mount:native_slot:{PLUGIN_DOCK_SLOT_KEY}"));
        outcome.detail = Some(
            "semantic view mount only — contract conformance, not dock/window/pixel evidence"
                .to_string(),
        );
        Ok(outcome)
    });
    finalize(
        report,
        env,
        EnvironmentKind::DeterministicBackend,
        None,
        Map::from_iter([("claim".to_string(), json!("semantic-view-mount-only"))]),
    )
}

// ---------------------------------------------------------------------
// case: artifact (resolve → hash → load → discover)
// ---------------------------------------------------------------------

/// `artifact`: resolves the `test_gain` cdylib (explicit env path →
/// `current_exe` auto lookup), records its SHA-256, loads it through
/// `Vst3Library::load`, and identifies the expected descriptor —
/// path/digest/id/name/vendor/version land in evidence. A missing
/// file is `NotRun` (fail-closed); hash/load/discover failures are
/// `Fail`. On success the loaded library and artifact evidence are
/// handed to the later cases through the out-params.
fn run_artifact_case(
    run_id: &RunId,
    env: &LaneEnvironment,
    library_slot: &mut Option<Vst3Library>,
    artifact_slot: &mut Option<PluginArtifactEvidence>,
) -> ScenarioReport {
    let mut probe = PluginHostProbe::new();
    let spec = probe_spec(run_id, "artifact", "real-abi-artifact-identity");
    let report = run_scenario(&mut probe, &spec, |_gui| {
        let path = match artifact::resolve_artifact(env.plugin_path.as_deref()) {
            Ok(path) => path,
            Err(reason) => return Ok(not_run(reason)),
        };
        let sha256 = match artifact::sha256_hex(&path) {
            Ok(digest) => digest,
            Err(error) => {
                return Ok(fail(format!(
                    "artifact hash failed for {}: {error}",
                    path.display()
                )));
            }
        };
        let library = match Vst3Library::load(&path) {
            Ok(library) => library,
            Err(error) => {
                return Ok(fail(format!(
                    "Vst3Library::load failed for {}: {error}",
                    path.display()
                )));
            }
        };
        let Some(descriptor) = library
            .descriptors()
            .iter()
            .find(|d| d.name == "test_gain")
            .or_else(|| library.descriptors().first())
        else {
            return Ok(fail(format!(
                "loaded {} but no plugin descriptor was discovered",
                path.display()
            )));
        };
        let evidence = PluginArtifactEvidence {
            path: path.display().to_string(),
            sha256,
            plugin_id: descriptor.id.to_string(),
            name: descriptor.name.to_string(),
            vendor: descriptor.vendor.to_string(),
            version: descriptor.version.to_string(),
        };
        let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
        outcome
            .evidence
            .push(format!("artifact:{}", evidence.evidence_ref()));
        outcome.observed = Some(json!({
            "plugin_id": evidence.plugin_id,
            "name": evidence.name,
            "vendor": evidence.vendor,
            "version": evidence.version,
            "sha256": evidence.sha256,
        }));
        *artifact_slot = Some(evidence);
        *library_slot = Some(library);
        Ok(outcome)
    });
    finalize(
        report,
        env,
        if library_slot.is_some() {
            EnvironmentKind::RealAbiTestPlugin
        } else {
            EnvironmentKind::DeterministicBackend
        },
        artifact_slot.as_ref(),
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: host-lifecycle (Scan → Instantiate → Initialize → Activate → Destroy)
// ---------------------------------------------------------------------

/// `host-lifecycle`: registers the scanned factories with a fresh
/// `InProcessHost` and drives the real command path — Scan,
/// Instantiate, Initialize, Activate (48 kHz, block 1..=512), then
/// Deactivate + Destroy teardown. Every step that ran is recorded in
/// `observed`; the first error maps to `Unsupported`/`Fail` honestly.
fn run_host_lifecycle(
    run_id: &RunId,
    env: &LaneEnvironment,
    library: Option<&Vst3Library>,
    artifact: Option<&PluginArtifactEvidence>,
) -> ScenarioReport {
    let mut probe = PluginHostProbe::new();
    let spec = probe_spec(run_id, "host-lifecycle", "real-abi-host-lifecycle");
    let report = run_scenario(&mut probe, &spec, |_gui| {
        let (Some(library), Some(artifact)) = (library, artifact) else {
            return Ok(not_run(
                "plugin artifact unavailable — see the `artifact` case verdict",
            ));
        };
        let plugin_id = artifact.plugin_id.clone();
        let mut host = InProcessHost::new();
        register_factories(&mut host, library);

        let mut steps = Vec::new();
        macro_rules! step {
            ($name:literal, $result:expr) => {
                match $result {
                    Ok(value) => {
                        steps.push(json!({ "step": $name, "status": "ok" }));
                        value
                    }
                    Err(error) => {
                        steps.push(json!({
                            "step": $name,
                            "status": "error",
                            "error": error.to_string(),
                        }));
                        let mut outcome = host_error_outcome($name, error);
                        outcome.observed = Some(json!({ "steps": steps }));
                        return Ok(outcome);
                    }
                }
            };
        }

        let scanned = step!("scan", host.execute(HostCommand::Scan));
        let HostReply::Scanned(descriptors) = scanned else {
            let mut outcome = fail("scan returned an unexpected reply shape");
            outcome.observed = Some(json!({ "steps": steps }));
            return Ok(outcome);
        };
        if !descriptors.iter().any(|d| d.id == plugin_id) {
            let mut outcome = fail(format!(
                "scan did not report expected plugin id `{plugin_id}`"
            ));
            outcome.observed = Some(json!({ "steps": steps }));
            return Ok(outcome);
        }

        let instantiated = step!(
            "instantiate",
            host.execute(HostCommand::Instantiate {
                plugin_id: plugin_id.clone(),
            })
        );
        let HostReply::Instantiated { handle } = instantiated else {
            let mut outcome = fail("instantiate returned an unexpected reply shape");
            outcome.observed = Some(json!({ "steps": steps }));
            return Ok(outcome);
        };

        step!(
            "initialize",
            host.execute(HostCommand::Initialize { handle })
        );
        let Some(config) = test_gain_config() else {
            let mut outcome = fail("test_gain ProcessConfig (48kHz, 1..=512) rejected");
            outcome.observed = Some(json!({ "steps": steps }));
            return Ok(outcome);
        };
        step!(
            "activate",
            host.execute(HostCommand::Activate { handle, config })
        );

        // Teardown — recorded like every other step; failures land in
        // `cleanup` on the report, not silently dropped.
        let _ = host.execute(HostCommand::Deactivate { handle });
        let _ = host.execute(HostCommand::Destroy { handle });
        steps.push(json!({ "step": "deactivate+destroy", "status": "ok" }));

        let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
        outcome.observed = Some(json!({
            "plugin_id": plugin_id,
            "sample_rate": 48000,
            "min_frames": 1,
            "max_frames": 512,
            "steps": steps,
        }));
        outcome
            .evidence
            .push("host:scan/instantiate/initialize/activate".to_string());
        Ok(outcome)
    });
    finalize(
        report,
        env,
        if library.is_some() {
            EnvironmentKind::RealAbiTestPlugin
        } else {
            EnvironmentKind::DeterministicBackend
        },
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: editor-attach (session + fake slot lease → honest Unsupported)
// ---------------------------------------------------------------------

/// The declarative fake slot the attach attempt leases: one 640×480
/// visible region with `attach_parent` support — the dock region the
/// `plugin_dock_view_spec` `NativeSlot` reserves.
fn fake_dock_slot_spec() -> SlotSpec {
    SlotSpec {
        window: WindowId::new(1).expect("window id 1 is non-zero"),
        view: ViewInstanceId::new(1).expect("view id 1 is non-zero"),
        geometry: SlotGeometry {
            rect: LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 480.0,
            },
            scale_factor: 1.0,
        },
        visibility: SlotVisibility::Visible,
        capabilities: SlotCapabilities {
            attach_parent: Support::Supported,
            ..SlotCapabilities::default()
        },
    }
}

/// What one editor-attach attempt produced.
enum AttachAttempt {
    /// `session.open` returned this.
    Open(Result<EditorCapabilities, EditorSessionError>),
    /// Setup failed before `open` — the detail string.
    Setup(String),
}

/// Runs the attach attempt — session begin, fake-slot declare/lease,
/// `ParentSlotLease` → issuer-side redeem → `ParentLease`, then
/// `session.open(Docked)` — and always tears down (session
/// close/end, lease/slot release, instance destroy). Returns the
/// open outcome (or the setup failure detail) plus cleanup notes.
fn attempt_editor_attach(
    host: &mut InProcessHost,
    handle: PluginHandle,
) -> (AttachAttempt, Vec<String>) {
    let mut cleanup = Vec::new();
    let mut sessions = PluginEditorSessions::new();
    let session_id = match sessions.begin_session(handle) {
        Ok(id) => id,
        Err(error) => {
            let _ = host.execute(HostCommand::Destroy { handle });
            return (
                AttachAttempt::Setup(format!("begin_session: {error}")),
                cleanup,
            );
        }
    };

    // Fake slot ledger: declare → lease → session-side
    // ParentSlotLease. The lease payload is the registry's opaque
    // encoding — never an OS handle.
    let mut slot = FakeNativeSlot::new();
    let surface = match slot.declare_slot(fake_dock_slot_spec()) {
        Ok(id) => id,
        Err(error) => {
            let _ = sessions.end_session(session_id);
            let _ = host.execute(HostCommand::Destroy { handle });
            return (
                AttachAttempt::Setup(format!("declare_slot: {error}")),
                cleanup,
            );
        }
    };
    let lease = match slot.lease_parent(surface) {
        Ok(lease) => lease,
        Err(error) => {
            let _ = slot.release_slot(surface);
            let _ = sessions.end_session(session_id);
            let _ = host.execute(HostCommand::Destroy { handle });
            return (
                AttachAttempt::Setup(format!("lease_parent: {error}")),
                cleanup,
            );
        }
    };
    let parent_slot_lease = ParentSlotLease::new(lease.raw());

    // The issuer-side redeem seam lives in `editor_ops`: validate the
    // session lease against the slot ledger, then mint the
    // plugin-side ParentLease.
    let open_result = {
        let mut ops = editor_ops(host, handle, &slot);
        sessions.session_mut(session_id).map(|session| {
            session.open(
                &mut ops,
                &parent_slot_lease,
                editor_size(),
                AttachMode::Docked,
            )
        })
    };

    // Teardown: converge the session (close is a no-op adapter call
    // when the open was refused — `editor_live` stays false), end it,
    // release the lease/slot, destroy the instance. The close reuses
    // `InProcessEditorOps`; its redeemer is never consulted on close.
    if let Some(session) = sessions.session_mut(session_id) {
        let mut close_ops = rejecting_editor_ops(host, handle);
        let _ = session.close(&mut close_ops);
    }
    let _ = sessions.end_session(session_id);
    let _ = slot.release_lease(lease);
    let _ = slot.release_slot(surface);
    let _ = host.execute(HostCommand::Destroy { handle });
    cleanup.push("session-ended/lease-released/instance-destroyed".to_string());

    let attempt = match open_result {
        Some(result) => AttachAttempt::Open(result),
        None => AttachAttempt::Setup("editor session vanished before open".to_string()),
    };
    (attempt, cleanup)
}

/// `editor-attach`: drives the real attach path — `PluginEditorSessions`
/// → `InProcessEditorOps` → `OpenGui` — against a `FakeNativeSlot`
/// lease redeemed through `validate_lease`. The current `test_gain`
/// fixture has no editor (`createView` → null → `HostError::Unsupported`
/// → `EditorOpError::Unsupported`), so the honest verdict is
/// `Unsupported` with that detail — never `Pass`. A future editor
/// fixture flipping this to `Pass` is contract-path evidence only,
/// still not a real-Dock claim (that stays `dock-pixels`).
fn run_editor_attach(
    run_id: &RunId,
    env: &LaneEnvironment,
    library: Option<&Vst3Library>,
    artifact: Option<&PluginArtifactEvidence>,
) -> ScenarioReport {
    let mut probe = PluginHostProbe::new();
    let spec = probe_spec(run_id, "editor-attach", "real-abi-editor-attach-attempt");
    let report = run_scenario(&mut probe, &spec, |_gui| {
        let (Some(library), Some(artifact)) = (library, artifact) else {
            return Ok(not_run(
                "plugin artifact unavailable — see the `artifact` case verdict",
            ));
        };
        let plugin_id = artifact.plugin_id.clone();
        let mut host = InProcessHost::new();
        register_factories(&mut host, library);

        let handle = match activate_test_gain(&mut host, &plugin_id) {
            Ok(handle) => handle,
            Err(error) => return Ok(host_error_outcome("activate", error)),
        };
        let (attempt, cleanup) = attempt_editor_attach(&mut host, handle);

        let mut outcome = match attempt {
            AttachAttempt::Open(Ok(_capabilities)) => {
                // The plugin accepted OpenGui through the fake-slot
                // lease — contract-path conformance only. No real
                // child HWND exists behind the fake slot, so this is
                // still not a Dock PASS.
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome.detail = Some(
                    "editor open accepted through fake-slot lease — contract \
                     conformance only, not real dock evidence"
                        .to_string(),
                );
                outcome
                    .evidence
                    .push("editor-attach:contract-path".to_string());
                outcome
            }
            AttachAttempt::Open(Err(EditorSessionError::Ops(EditorOpError::Unsupported))) => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
                outcome.detail = Some(
                    "plugin declined OpenGui: createView returned null → \
                     HostError::Unsupported → EditorOpError::Unsupported \
                     (test_gain ships no editor surface)"
                        .to_string(),
                );
                outcome
                    .evidence
                    .push("editor-attach:createView-null".to_string());
                outcome
            }
            AttachAttempt::Open(Err(error)) => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Fail);
                outcome.detail = Some(format!("session.open: {error}"));
                outcome
            }
            AttachAttempt::Setup(detail) => fail(detail),
        };
        outcome.observed = Some(json!({ "cleanup": cleanup }));
        Ok(outcome)
    });
    finalize(
        report,
        env,
        if library.is_some() {
            EnvironmentKind::RealAbiTestPlugin
        } else {
            EnvironmentKind::DeterministicBackend
        },
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: session-lifecycle (begin/busy/refused-open/close/end/re-begin)
// ---------------------------------------------------------------------

/// The session-lifecycle step sequence — every assertion runs against
/// the live registry and the real binary's refused `OpenGui`. Returns
/// `Err(detail)` on the first violated expectation; `steps` records
/// every step that ran.
fn session_lifecycle_steps(
    host: &mut InProcessHost,
    handle: PluginHandle,
    state: &mut SessionCaseState,
    steps: &mut Vec<Value>,
) -> Result<(), String> {
    let first = begin_case_session(state, handle, steps, "begin_session")?;
    assert_begin_session_busy(&mut state.sessions, handle, steps)?;
    let parent_slot_lease = declare_fake_dock_lease(state, steps)?;

    // Refused open: test_gain `createView` null -> the session lands
    // `Failed{Unsupported}` with `editor_live == false`.
    let mut ops = CountingEditorOps::new(host, handle, &state.slot);
    refused_docked_open(
        &mut state.sessions,
        first,
        &mut ops,
        parent_slot_lease,
        steps,
    )?;
    if ops.opens != 1 {
        return Err(format!("expected 1 open dispatch, got {}", ops.opens));
    }

    // Converge: `editor_live == false` means `close` must not dispatch
    // an adapter `CloseGui` — the counting ops prove it directly.
    assert_close_converge(&mut state.sessions, first, &mut ops, steps)?;
    // `ops` still borrows `state.slot`/`host` — drop it before the
    // whole-state helpers and `Destroy` run.
    drop(ops);

    end_case_session(&mut state.sessions, first, handle, steps, "end_session")?;

    // Slot reuse + generation bump: the freed slot index mints a new
    // id at generation + 1.
    let second = begin_case_session(state, handle, steps, "rebegin_session")?;
    if second.slot() != first.slot() || second.generation() != first.generation().wrapping_add(1) {
        return Err(format!(
            "re-begun session {second:?} did not reuse slot {} at generation {}",
            first.slot(),
            first.generation().wrapping_add(1)
        ));
    }
    steps.push(json!({
        "step": "rebegin_session_check",
        "status": "ok",
        "slot_reused": true,
        "generation_bumped": true,
    }));
    end_case_session(
        &mut state.sessions,
        second,
        handle,
        steps,
        "end_session_second",
    )?;

    host.execute(HostCommand::Deactivate { handle })
        .map_err(|error| format!("deactivate: {error}"))?;
    steps.push(json!({ "step": "deactivate", "status": "ok" }));
    host.execute(HostCommand::Destroy { handle })
        .map_err(|error| format!("destroy: {error}"))?;
    steps.push(json!({ "step": "destroy", "status": "ok" }));
    Ok(())
}

/// `session-lifecycle`: the full registry lifecycle against the real
/// binary — `begin_session` → `SessionBusy` refusal → refused
/// `open(Docked)` (`Ops(Unsupported)`, `Failed{Unsupported}`) →
/// adapter-free `close` converge → `end_session` → instance binding
/// release → slot/generation-bumped re-`begin_session` → `Destroy`.
/// Every step that ran is recorded in `observed`; the first violated
/// expectation maps to `Fail`.
fn run_session_lifecycle(
    run_id: &RunId,
    env: &LaneEnvironment,
    library: Option<&Vst3Library>,
    artifact: Option<&PluginArtifactEvidence>,
) -> ScenarioReport {
    let mut probe = PluginHostProbe::new();
    let spec = probe_spec(run_id, "session-lifecycle", "real-abi-session-lifecycle");
    let report = run_scenario(&mut probe, &spec, |_gui| {
        let (Some(library), Some(artifact)) = (library, artifact) else {
            return Ok(not_run(
                "plugin artifact unavailable — see the `artifact` case verdict",
            ));
        };
        let plugin_id = artifact.plugin_id.clone();
        let mut host = InProcessHost::new();
        register_factories(&mut host, library);
        let handle = match activate_test_gain(&mut host, &plugin_id) {
            Ok(handle) => handle,
            Err(error) => return Ok(host_error_outcome("activate", error)),
        };

        let mut state = SessionCaseState::default();
        let mut steps = Vec::new();
        let result = session_lifecycle_steps(&mut host, handle, &mut state, &mut steps);
        let cleanup = teardown_editor_case(&mut host, handle, &mut state);

        let mut outcome = match result {
            Ok(()) => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome
                    .evidence
                    .push("session:begin/busy/refused-open/close/end/rebegin".to_string());
                outcome
            }
            Err(detail) => fail(detail),
        };
        outcome.observed = Some(json!({
            "plugin_id": plugin_id,
            "steps": steps,
            "cleanup": cleanup,
        }));
        Ok(outcome)
    });
    finalize(
        report,
        env,
        if library.is_some() {
            EnvironmentKind::RealAbiTestPlugin
        } else {
            EnvironmentKind::DeterministicBackend
        },
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: host-terminate (Destroy → notify_terminated → exactly-once)
// ---------------------------------------------------------------------

/// The host-terminate step sequence — refused `open` leaves a live
/// `Failed{Unsupported}` session, then `Destroy` + registry-level
/// `notify_terminated(HostLost)` must emit `Terminated{HostLost}`
/// exactly once and converge to `Closed`.
fn host_terminate_steps(
    host: &mut InProcessHost,
    handle: PluginHandle,
    state: &mut SessionCaseState,
    steps: &mut Vec<Value>,
) -> Result<(), String> {
    let id = begin_case_session(state, handle, steps, "begin_session")?;
    let parent_slot_lease = declare_fake_dock_lease(state, steps)?;

    // Refused open → live `Failed{Unsupported}` (non-`Closed`) — the
    // only honest way to get a `Terminated` event today: a
    // born-`Closed` session emits none.
    {
        let mut ops = CountingEditorOps::new(host, handle, &state.slot);
        refused_docked_open(&mut state.sessions, id, &mut ops, parent_slot_lease, steps)?;
    }

    // Host loss: deactivate + destroy the instance (Destroy is not
    // legal from `Activated`), then the registry-level termination
    // path converges the session — `Terminated` fires exactly once,
    // the instance binding is removed, and a repeated notify is a
    // no-op.
    host.execute(HostCommand::Deactivate { handle })
        .map_err(|error| format!("deactivate: {error}"))?;
    steps.push(json!({ "step": "deactivate", "status": "ok" }));
    host.execute(HostCommand::Destroy { handle })
        .map_err(|error| format!("destroy: {error}"))?;
    steps.push(json!({ "step": "destroy", "status": "ok" }));

    state
        .sessions
        .notify_terminated(handle, SessionFailCause::HostLost);
    steps.push(json!({
        "step": "notify_terminated",
        "status": "ok",
        "cause": "host-lost",
    }));

    let terminated_events =
        drain_case_events(&mut state.sessions, id, steps, "drain_events_terminated");
    let terminated_once = terminated_events.len() == 1
        && matches!(
            terminated_events[0],
            EditorSessionEvent::Terminated {
                cause: SessionFailCause::HostLost
            }
        );
    if !terminated_once {
        return Err(format!(
            "expected exactly [Terminated{{HostLost}}], got {:?}",
            events_debug(&terminated_events)
        ));
    }

    state
        .sessions
        .notify_terminated(handle, SessionFailCause::HostLost);
    let after = drain_case_events(
        &mut state.sessions,
        id,
        steps,
        "drain_events_after_second_notify",
    );
    if !after.is_empty() {
        return Err(format!(
            "second notify_terminated produced events: {:?}",
            events_debug(&after)
        ));
    }
    steps.push(json!({
        "step": "notify_terminated_again",
        "status": "ok",
        "result": "no-op",
    }));

    if !session_closed_concluded(&state.sessions, id) {
        return Err("session did not converge to Closed+concluded after Terminated".to_string());
    }
    steps.push(json!({
        "step": "state_after_terminated",
        "status": "ok",
        "state": "closed-concluded",
    }));

    end_case_session(&mut state.sessions, id, handle, steps, "end_session")?;
    Ok(())
}

/// `host-terminate`: `Destroy` on a live `Failed{Unsupported}`
/// session's instance followed by `PluginEditorSessions::
/// notify_terminated(HostLost)` — asserts `Terminated{HostLost}`
/// fires exactly once (a repeated notify is a no-op), the session
/// converges to `Closed`, and the instance binding is released.
fn run_host_terminate(
    run_id: &RunId,
    env: &LaneEnvironment,
    library: Option<&Vst3Library>,
    artifact: Option<&PluginArtifactEvidence>,
) -> ScenarioReport {
    let mut probe = PluginHostProbe::new();
    let spec = probe_spec(run_id, "host-terminate", "real-abi-host-terminate");
    let report = run_scenario(&mut probe, &spec, |_gui| {
        let (Some(library), Some(artifact)) = (library, artifact) else {
            return Ok(not_run(
                "plugin artifact unavailable — see the `artifact` case verdict",
            ));
        };
        let plugin_id = artifact.plugin_id.clone();
        let mut host = InProcessHost::new();
        register_factories(&mut host, library);
        let handle = match activate_test_gain(&mut host, &plugin_id) {
            Ok(handle) => handle,
            Err(error) => return Ok(host_error_outcome("activate", error)),
        };

        let mut state = SessionCaseState::default();
        let mut steps = Vec::new();
        let result = host_terminate_steps(&mut host, handle, &mut state, &mut steps);
        let cleanup = teardown_editor_case(&mut host, handle, &mut state);

        let mut outcome = match result {
            Ok(()) => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome
                    .evidence
                    .push("session:terminated{host-lost}:exactly-once".to_string());
                outcome
            }
            Err(detail) => fail(detail),
        };
        outcome.observed = Some(json!({
            "plugin_id": plugin_id,
            "steps": steps,
            "cleanup": cleanup,
        }));
        Ok(outcome)
    });
    finalize(
        report,
        env,
        if library.is_some() {
            EnvironmentKind::RealAbiTestPlugin
        } else {
            EnvironmentKind::DeterministicBackend
        },
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: open-or-degrade (Docked→Floating double refusal, zero Degraded)
// ---------------------------------------------------------------------

/// The open-or-degrade step sequence — `open_or_degrade(Docked →
/// Floating)` against the editor-less `test_gain` must refuse both
/// modes (`Err(Ops(Unsupported))`, `Failed{Unsupported}`) and emit
/// zero `Degraded`/`Attached`-family events: a degrade is only ever
/// reported when the fallback actually attached.
fn open_or_degrade_steps(
    host: &mut InProcessHost,
    handle: PluginHandle,
    state: &mut SessionCaseState,
    steps: &mut Vec<Value>,
) -> Result<(), String> {
    let id = begin_case_session(state, handle, steps, "begin_session")?;
    let parent_slot_lease = declare_fake_dock_lease(state, steps)?;

    let mut ops = CountingEditorOps::new(host, handle, &state.slot);
    let result = state
        .sessions
        .session_mut(id)
        .map(|session| {
            session
                .open_or_degrade(
                    &mut ops,
                    &parent_slot_lease,
                    editor_size(),
                    AttachMode::Docked,
                    AttachMode::Floating,
                )
                .map(|_| ())
        })
        .ok_or_else(|| "editor session vanished before open_or_degrade".to_string())?;
    // Exactly `[Failed{Unsupported}]` events (asserted inside) also
    // proves zero `Degraded`/attach-family events — a degrade is only
    // ever reported when the fallback actually attached.
    assert_refused_open(
        &mut state.sessions,
        id,
        &mut ops,
        result,
        steps,
        "open_or_degrade",
    )?;
    // Docked attempt + Floating retry = two adapter dispatches; the
    // Floating refusal lands before any lease redemption.
    if ops.opens != 2 {
        return Err(format!("expected 2 open dispatches, got {}", ops.opens));
    }

    // Same `editor_live == false` converge as `session-lifecycle`.
    assert_close_converge(&mut state.sessions, id, &mut ops, steps)?;
    drop(ops);

    end_case_session(&mut state.sessions, id, handle, steps, "end_session")?;

    host.execute(HostCommand::Deactivate { handle })
        .map_err(|error| format!("deactivate: {error}"))?;
    steps.push(json!({ "step": "deactivate", "status": "ok" }));
    host.execute(HostCommand::Destroy { handle })
        .map_err(|error| format!("destroy: {error}"))?;
    steps.push(json!({ "step": "destroy", "status": "ok" }));
    Ok(())
}

/// `open-or-degrade`: `open_or_degrade(Docked → Floating)` against
/// the editor-less `test_gain` — asserts the double refusal lands
/// `Failed{Unsupported}` with zero `Degraded`/attach-family events
/// (a degrade is never fabricated), then the same adapter-free close
/// converge and registry teardown as `session-lifecycle`.
fn run_open_or_degrade(
    run_id: &RunId,
    env: &LaneEnvironment,
    library: Option<&Vst3Library>,
    artifact: Option<&PluginArtifactEvidence>,
) -> ScenarioReport {
    let mut probe = PluginHostProbe::new();
    let spec = probe_spec(run_id, "open-or-degrade", "real-abi-open-or-degrade");
    let report = run_scenario(&mut probe, &spec, |_gui| {
        let (Some(library), Some(artifact)) = (library, artifact) else {
            return Ok(not_run(
                "plugin artifact unavailable — see the `artifact` case verdict",
            ));
        };
        let plugin_id = artifact.plugin_id.clone();
        let mut host = InProcessHost::new();
        register_factories(&mut host, library);
        let handle = match activate_test_gain(&mut host, &plugin_id) {
            Ok(handle) => handle,
            Err(error) => return Ok(host_error_outcome("activate", error)),
        };

        let mut state = SessionCaseState::default();
        let mut steps = Vec::new();
        let result = open_or_degrade_steps(&mut host, handle, &mut state, &mut steps);
        let cleanup = teardown_editor_case(&mut host, handle, &mut state);

        let mut outcome = match result {
            Ok(()) => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome
                    .evidence
                    .push("session:open-or-degrade:double-refusal:no-degraded-event".to_string());
                outcome
            }
            Err(detail) => fail(detail),
        };
        outcome.observed = Some(json!({
            "plugin_id": plugin_id,
            "steps": steps,
            "cleanup": cleanup,
        }));
        Ok(outcome)
    });
    finalize(
        report,
        env,
        if library.is_some() {
            EnvironmentKind::RealAbiTestPlugin
        } else {
            EnvironmentKind::DeterministicBackend
        },
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: dock-pixels (honest NotRun — no surface, no capture driver)
// ---------------------------------------------------------------------

/// `dock-pixels`: pixel verification of a docked editor. With no
/// attached editor surface (see `editor-attach`) and no
/// `VisualCapture`-capable driver, the honest verdict is `NotRun` —
/// pixels were never captured and nothing here claims they were.
fn run_dock_pixels(run_id: &RunId, env: &LaneEnvironment) -> ScenarioReport {
    let mut probe = PluginHostProbe::new();
    let spec = probe_spec(run_id, "dock-pixels", "real-window-pixels-not-applicable");
    let report = run_scenario(&mut probe, &spec, |_gui| {
        let mut outcome = not_run(
            "no docked editor surface exists (editor-attach: unsupported) and \
             no VisualCapture driver is present — pixel checks not executed",
        );
        outcome.evidence.push("pixels:not-captured".to_string());
        Ok(outcome)
    });
    finalize(
        report,
        env,
        EnvironmentKind::DeterministicBackend,
        None,
        Map::new(),
    )
}
