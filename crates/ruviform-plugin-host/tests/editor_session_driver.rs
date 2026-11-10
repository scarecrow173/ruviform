//! Contract tests for `ruviform_plugin_host::editor_session` driven
//! through a fake host driver (`FakeHostDriver` + `ScriptedOps`).
//!
//! The existing `editor_session.rs` suite pins the state machine by
//! hand-driving `begin_*`/`complete_*`/`tick` per test. This file adds
//! the missing driver-side fixture: a reusable fake GUI driver that
//! owns the sessions registry, a scripted `EditorSessionOps`
//! adapter, a shared fake `SessionClock`, and a pending-completion
//! queue — reproducing the documented async driver loop
//! (`begin_*` → dispatch `ops.*` → `complete_*` → periodic
//! `tick`/`tick_all`). SM-xx cases verify terminal-event-once,
//! stale session/request rejection, double close, request timeouts,
//! OS `Unsupported`, explicit degrade, and Plugin/GPU host crash
//! convergence — **contract-level results, not a real Dock PASS**
//! (no GPUI window, OS parent, or real plugin; real-device
//! acceptance stays `NotRun` under #388 / Synvelis #176).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

use ruviform_plugin_core::{ParentSlotLease, PluginEditorSessionId, PluginHandle};
use ruviform_plugin_host::{
    AttachMode, DEFAULT_REQUEST_TIMEOUT, EditorCapabilities, EditorOpError, EditorSessionError,
    EditorSessionEvent, EditorSessionOps, EditorSessionState, EditorSize, PluginEditorSession,
    PluginEditorSessions, SessionClock, SessionDeadlinePolicy, SessionFailCause, Support,
};

// ---- shared fixtures -------------------------------------------------

const SIZE: EditorSize = EditorSize {
    width: 800.0,
    height: 600.0,
};

const SIZE_LARGE: EditorSize = EditorSize {
    width: 1024.0,
    height: 768.0,
};

fn lease() -> ParentSlotLease {
    // The session side treats the payload as fully opaque — a fake
    // driver-encoded value stands in for a real `SlotLease::encode`.
    ParentSlotLease::new(0xA5A5_0002_0000_0000)
}

fn instance(slot: u32) -> PluginHandle {
    // `#[doc(hidden)]` constructor — tests mint handles directly.
    PluginHandle::new(slot, 0)
}

// ---- fake host driver fixture ----------------------------------------

/// Op kinds the driver can withhold (`hung`) to model an adapter
/// that never produces a result. `Detach` gates the platform-detach
/// half of a reparent (there is no `ops.detach` verb — detach is
/// driver/platform work, scripted separately).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum OpKind {
    Open,
    Detach,
    Reattach,
    Close,
}

/// One recorded adapter invocation, for call-order assertions.
#[derive(Clone, Debug, PartialEq)]
enum OpCall {
    Capabilities,
    Open { mode: AttachMode },
    Close,
    SetSize,
    SetVisibility { visible: bool },
    Reattach,
}

/// Scripted `EditorSessionOps`: canned per-op results, per-mode open
/// results, a hang set, and a call log. `hung` kinds are never
/// dispatched — `pump` defers them so the session deadline (`tick`)
/// is what converges the request; the late dispatch then lands as
/// `StaleRequest`.
struct ScriptedOps {
    open_by_mode: HashMap<AttachMode, Result<(), EditorOpError>>,
    open_default: Result<(), EditorOpError>,
    close_result: Result<(), EditorOpError>,
    reattach_result: Result<(), EditorOpError>,
    size_result: Result<(), EditorOpError>,
    visibility_result: Result<(), EditorOpError>,
    caps: EditorCapabilities,
    hung: HashSet<OpKind>,
    calls: RefCell<Vec<OpCall>>,
}

impl ScriptedOps {
    fn ok() -> Self {
        Self {
            open_by_mode: HashMap::new(),
            open_default: Ok(()),
            close_result: Ok(()),
            reattach_result: Ok(()),
            size_result: Ok(()),
            visibility_result: Ok(()),
            caps: EditorCapabilities {
                dock: Support::Supported,
                float: Support::Supported,
                resize: Support::Supported,
                hide_show: Support::Supported,
                reparent: Support::Supported,
                close: Support::Supported,
                ..EditorCapabilities::default()
            },
            hung: HashSet::new(),
            calls: RefCell::new(Vec::new()),
        }
    }

    /// Every lifecycle op refuses with `Unsupported` and caps report
    /// all-`Unsupported` — a defined-refusal adapter. Note this is
    /// *not* `L1EditorOps`-shaped: the real L1 port also refuses
    /// `capabilities` with `Err(Unsupported)`, while this fake returns
    /// `Ok(caps)` (matching `InProcessEditorOps` pre-open).
    fn unsupported() -> Self {
        Self {
            open_default: Err(EditorOpError::Unsupported),
            close_result: Err(EditorOpError::Unsupported),
            reattach_result: Err(EditorOpError::Unsupported),
            size_result: Err(EditorOpError::Unsupported),
            visibility_result: Err(EditorOpError::Unsupported),
            caps: EditorCapabilities::default(),
            ..Self::ok()
        }
    }

    fn hang(&mut self, kind: OpKind) {
        self.hung.insert(kind);
    }

    fn unhang(&mut self, kind: OpKind) {
        self.hung.remove(&kind);
    }

    fn is_hung(&self, kind: OpKind) -> bool {
        self.hung.contains(&kind)
    }

    fn calls(&self) -> Vec<OpCall> {
        self.calls.borrow().clone()
    }

    fn open_call_count(&self) -> usize {
        self.calls
            .borrow()
            .iter()
            .filter(|call| matches!(call, OpCall::Open { .. }))
            .count()
    }

    fn close_call_count(&self) -> usize {
        self.calls
            .borrow()
            .iter()
            .filter(|call| matches!(call, OpCall::Close))
            .count()
    }

    fn reattach_call_count(&self) -> usize {
        self.calls
            .borrow()
            .iter()
            .filter(|call| matches!(call, OpCall::Reattach))
            .count()
    }
}

impl EditorSessionOps for ScriptedOps {
    fn capabilities(
        &self,
        _session: PluginEditorSessionId,
    ) -> Result<EditorCapabilities, EditorOpError> {
        self.calls.borrow_mut().push(OpCall::Capabilities);
        Ok(self.caps)
    }

    fn open(
        &mut self,
        _session: PluginEditorSessionId,
        _lease: &ParentSlotLease,
        _size: EditorSize,
        mode: AttachMode,
    ) -> Result<EditorCapabilities, EditorOpError> {
        self.calls.borrow_mut().push(OpCall::Open { mode });
        self.open_by_mode
            .get(&mode)
            .copied()
            .unwrap_or(self.open_default)
            .map(|()| self.caps)
    }

    fn close(&mut self, _session: PluginEditorSessionId) -> Result<(), EditorOpError> {
        self.calls.borrow_mut().push(OpCall::Close);
        self.close_result
    }

    fn set_size(
        &mut self,
        _session: PluginEditorSessionId,
        _size: EditorSize,
    ) -> Result<(), EditorOpError> {
        self.calls.borrow_mut().push(OpCall::SetSize);
        self.size_result
    }

    fn set_visibility(
        &mut self,
        _session: PluginEditorSessionId,
        visible: bool,
    ) -> Result<(), EditorOpError> {
        self.calls
            .borrow_mut()
            .push(OpCall::SetVisibility { visible });
        self.visibility_result
    }

    fn reattach(
        &mut self,
        _session: PluginEditorSessionId,
        _lease: &ParentSlotLease,
    ) -> Result<(), EditorOpError> {
        self.calls.borrow_mut().push(OpCall::Reattach);
        self.reattach_result
    }
}

/// A `SessionClock` the test advances manually — the shared clock every
/// session deadline policy is armed against.
#[derive(Clone, Default)]
struct FakeClock {
    now: Rc<Cell<Duration>>,
}

impl FakeClock {
    fn advance(&self, by: Duration) {
        self.now.set(self.now.get() + by);
    }
}

impl SessionClock for FakeClock {
    fn now(&self) -> Duration {
        self.now.get()
    }
}

/// What a pending queue item does when `pump` delivers it.
#[derive(Copy, Clone)]
enum PendingKind {
    /// `ops.open` → `complete_attach` (open / recover / degrade).
    Open {
        request: u64,
        lease: ParentSlotLease,
        size: EditorSize,
        mode: AttachMode,
    },
    /// Platform detach (scripted result) → `complete_detach`; on `Ok`
    /// the driver enqueues `Reattach` under the fresh request id.
    Detach {
        request: u64,
        result: Result<(), EditorOpError>,
        lease: ParentSlotLease,
    },
    /// `ops.reattach` → `complete_attach` (the re-attach half).
    Reattach {
        request: u64,
        lease: ParentSlotLease,
    },
    /// `ops.close` → `complete_close` on `Ok`; a refusal is withheld —
    /// async `complete_close` has no error channel, so the request
    /// expires via deadline (`Failed{Timeout}`) and a later `close`
    /// retries through the adapter path.
    Close { request: u64 },
}

impl PendingKind {
    fn tag(&self) -> OpKind {
        match self {
            Self::Open { .. } => OpKind::Open,
            Self::Detach { .. } => OpKind::Detach,
            Self::Reattach { .. } => OpKind::Reattach,
            Self::Close { .. } => OpKind::Close,
        }
    }
}

struct PendingOp {
    session: PluginEditorSessionId,
    kind: PendingKind,
}

/// Fake host driver: the GUI-thread side of the async session
/// protocol. Owns the sessions registry, the scripted adapter, the
/// shared fake clock, and the pending-completion queue. Verbs mint
/// requests via `begin_*` and enqueue work; `pump` executes ready
/// ops and delivers `complete_*`; `advance` + `tick_all` drive the
/// deadline policy. Only public session API is used — the driver's
/// `live_editors` ledger tracks *adapter-side* truth: an editor
/// exists iff `ops.open` returned `Ok` and no `ops.close` `Ok` /
/// termination converged since — deliberately independent of
/// whether `complete_attach` accepted the result, so a stale-but-
/// successful open still records the orphan the adapter now holds.
struct FakeHostDriver {
    sessions: PluginEditorSessions,
    ops: ScriptedOps,
    clock: FakeClock,
    timeout: Duration,
    pending: VecDeque<PendingOp>,
    instances: HashMap<PluginEditorSessionId, PluginHandle>,
    live_editors: HashSet<PluginEditorSessionId>,
    detach_results: VecDeque<Result<(), EditorOpError>>,
    stale_drops: usize,
    stale_rejections: usize,
}

impl FakeHostDriver {
    fn new() -> Self {
        Self {
            sessions: PluginEditorSessions::new(),
            ops: ScriptedOps::ok(),
            clock: FakeClock::default(),
            timeout: DEFAULT_REQUEST_TIMEOUT,
            pending: VecDeque::new(),
            instances: HashMap::new(),
            live_editors: HashSet::new(),
            detach_results: VecDeque::new(),
            stale_drops: 0,
            stale_rejections: 0,
        }
    }

    fn ops_mut(&mut self) -> &mut ScriptedOps {
        &mut self.ops
    }

    /// Begins a session for `instance` and arms every in-flight
    /// request deadline against the shared fake clock.
    fn begin_instance(
        &mut self,
        instance: PluginHandle,
    ) -> Result<PluginEditorSessionId, EditorSessionError> {
        let id = self.sessions.begin_session(instance)?;
        let policy = SessionDeadlinePolicy {
            timeout: self.timeout,
            clock: Box::new(self.clock.clone()),
        };
        if let Some(session) = self.sessions.session_mut(id) {
            session.set_deadline_policy(Some(policy));
        }
        self.instances.insert(id, instance);
        Ok(id)
    }

    /// Overrides the request deadline for sessions already begun —
    /// re-installs the policy (which re-arms any in-flight request).
    fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
        let ids: Vec<PluginEditorSessionId> = self.instances.keys().copied().collect();
        for id in ids {
            if let Some(session) = self.sessions.session_mut(id) {
                session.set_deadline_policy(Some(SessionDeadlinePolicy {
                    timeout,
                    clock: Box::new(self.clock.clone()),
                }));
            }
        }
    }

    fn session(&self, id: PluginEditorSessionId) -> Option<&PluginEditorSession> {
        self.sessions.session(id)
    }

    fn session_mut(
        &mut self,
        id: PluginEditorSessionId,
    ) -> Result<&mut PluginEditorSession, EditorSessionError> {
        self.sessions
            .session_mut(id)
            .ok_or(EditorSessionError::UnknownSession)
    }

    fn state(&self, id: PluginEditorSessionId) -> Option<EditorSessionState> {
        self.sessions.session(id).map(PluginEditorSession::state)
    }

    fn drain_events(&mut self, id: PluginEditorSessionId) -> Vec<EditorSessionEvent> {
        self.sessions
            .session_mut(id)
            .map_or_else(Vec::new, PluginEditorSession::drain_events)
    }

    fn session_for_instance(&self, instance: PluginHandle) -> Option<PluginEditorSessionId> {
        self.sessions.session_for_instance(instance)
    }

    fn end_session(&mut self, id: PluginEditorSessionId) -> Result<(), EditorSessionError> {
        self.sessions.end_session(id)
    }

    // ---- driver verbs (mint request + enqueue work) ----

    /// `Closed → Creating` via `begin_open`; `ops.open` runs on `pump`.
    fn open(
        &mut self,
        id: PluginEditorSessionId,
        lease: ParentSlotLease,
        size: EditorSize,
        mode: AttachMode,
    ) -> Result<(), EditorSessionError> {
        let request = self.session_mut(id)?.begin_open(mode)?;
        self.pending.push_back(PendingOp {
            session: id,
            kind: PendingKind::Open {
                request,
                lease,
                size,
                mode,
            },
        });
        Ok(())
    }

    /// `Failed → Recovering` via `begin_recover`; `ops.open` runs on
    /// `pump` and lands `Recovered` on success.
    fn recover(
        &mut self,
        id: PluginEditorSessionId,
        lease: ParentSlotLease,
        size: EditorSize,
        mode: AttachMode,
    ) -> Result<(), EditorSessionError> {
        let request = self.session_mut(id)?.begin_recover(mode)?;
        self.pending.push_back(PendingOp {
            session: id,
            kind: PendingKind::Open {
                request,
                lease,
                size,
                mode,
            },
        });
        Ok(())
    }

    /// `Failed → Recovering` marked degrade via `begin_degrade`;
    /// `ops.open` runs on `pump` and lands `Degraded` on success —
    /// the async counterpart of `open_or_degrade`, used when the
    /// driver explicitly falls back (e.g. Docked → Floating).
    fn degrade(
        &mut self,
        id: PluginEditorSessionId,
        lease: ParentSlotLease,
        size: EditorSize,
        mode: AttachMode,
    ) -> Result<(), EditorSessionError> {
        let request = self.session_mut(id)?.begin_degrade(mode)?;
        self.pending.push_back(PendingOp {
            session: id,
            kind: PendingKind::Open {
                request,
                lease,
                size,
                mode,
            },
        });
        Ok(())
    }

    /// `{Attached|Hidden} → Detaching` via `begin_detach`; the
    /// platform-detach half delivers on `pump` (scripted via
    /// [`Self::script_detach`], default `Ok`), then `ops.reattach`
    /// delivers the re-attach half under the fresh request id.
    fn reparent(
        &mut self,
        id: PluginEditorSessionId,
        lease: ParentSlotLease,
    ) -> Result<(), EditorSessionError> {
        let request = self.session_mut(id)?.begin_detach()?;
        let result = self.detach_results.pop_front().unwrap_or(Ok(()));
        self.pending.push_back(PendingOp {
            session: id,
            kind: PendingKind::Detach {
                request,
                result,
                lease,
            },
        });
        Ok(())
    }

    /// Scripts the platform-detach outcome consumed by the next
    /// `reparent` (default `Ok` when the queue is empty).
    fn script_detach(&mut self, result: Result<(), EditorOpError>) {
        self.detach_results.push_back(result);
    }

    /// `{Attached|Hidden|Detaching|Failed} → Closing` via
    /// `begin_close`. A session whose editor never went live
    /// converges immediately without an adapter call; a live one
    /// enqueues `ops.close` for `pump`.
    fn close(&mut self, id: PluginEditorSessionId) -> Result<(), EditorSessionError> {
        let request = self.session_mut(id)?.begin_close()?;
        if self.live_editors.contains(&id) {
            self.pending.push_back(PendingOp {
                session: id,
                kind: PendingKind::Close { request },
            });
        } else {
            self.session_mut(id)?.complete_close(request)?;
        }
        Ok(())
    }

    /// Synchronous `Attached ↔ Hidden` (`ops.set_visibility` inline).
    fn set_visible(
        &mut self,
        id: PluginEditorSessionId,
        visible: bool,
    ) -> Result<(), EditorSessionError> {
        let session = self
            .sessions
            .session_mut(id)
            .ok_or(EditorSessionError::UnknownSession)?;
        session.set_visibility(&mut self.ops, visible)
    }

    /// Synchronous resize (`ops.set_size` inline) — a plugin refusal
    /// is non-fatal and leaves the session `Attached`/`Hidden`.
    fn resize(
        &mut self,
        id: PluginEditorSessionId,
        size: EditorSize,
    ) -> Result<(), EditorSessionError> {
        let session = self
            .sessions
            .session_mut(id)
            .ok_or(EditorSessionError::UnknownSession)?;
        session.set_size(&mut self.ops, size)
    }

    /// Adapter capability probe (`ops.capabilities` inline).
    fn probe(&self, id: PluginEditorSessionId) -> Result<EditorCapabilities, EditorOpError> {
        self.ops.capabilities(id)
    }

    // ---- driver loop ----

    /// Advances the shared fake clock — deadlines are evaluated
    /// against it, never wall-clock.
    fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }

    /// Executes every ready pending op once (snapshot semantics —
    /// items enqueued during this pass, e.g. a re-attach, wait for
    /// the next `pump`). `hung` items stay queued. Returns the number
    /// of ops executed.
    fn pump(&mut self) -> usize {
        let batch: Vec<PendingOp> = self.pending.drain(..).collect();
        let mut deferred: Vec<PendingOp> = Vec::new();
        let mut executed = 0;
        for op in batch {
            if self.ops.is_hung(op.kind.tag()) {
                deferred.push(op);
                continue;
            }
            self.execute(&op);
            executed += 1;
        }
        self.pending.extend(deferred);
        executed
    }

    /// Runs one deadline evaluation across every session.
    fn tick_all(&mut self) -> usize {
        self.sessions.tick_all()
    }

    /// Instance destroy / host crash / window loss — converges the
    /// instance's live session to `Closed` + `Terminated{cause}` once
    /// and frees its instance binding and live-editor ledger entry.
    fn crash(&mut self, instance: PluginHandle, cause: SessionFailCause) {
        self.sessions.notify_terminated(instance, cause);
        let dead: Vec<PluginEditorSessionId> = self
            .instances
            .iter()
            .filter(|(_, bound)| **bound == instance)
            .map(|(id, _)| *id)
            .collect();
        for id in dead {
            self.live_editors.remove(&id);
        }
    }

    /// Test escape hatch: delivers an attach completion directly,
    /// bypassing the pending queue — models a stale/mismatched async
    /// callback arriving driver-side. `StaleRequest` results are
    /// counted like queue-delivered ones.
    fn inject_attach_completion(
        &mut self,
        id: PluginEditorSessionId,
        request: u64,
        result: Result<(), EditorOpError>,
    ) -> Result<(), EditorSessionError> {
        let outcome = self.session_mut(id)?.complete_attach(request, result);
        if matches!(outcome, Err(EditorSessionError::StaleRequest)) {
            self.stale_rejections += 1;
        }
        if outcome.is_ok() {
            self.live_editors.insert(id);
        }
        outcome
    }

    // ---- inspection ----

    /// Adapter-side liveness ledger: `true` iff `ops.open` returned
    /// `Ok` and no `ops.close` `Ok` / termination converged since.
    fn is_live(&self, id: PluginEditorSessionId) -> bool {
        self.live_editors.contains(&id)
    }

    fn stale_drops(&self) -> usize {
        self.stale_drops
    }

    fn stale_rejections(&self) -> usize {
        self.stale_rejections
    }

    fn ops_calls(&self) -> Vec<OpCall> {
        self.ops.calls()
    }

    fn open_call_count(&self) -> usize {
        self.ops.open_call_count()
    }

    fn close_call_count(&self) -> usize {
        self.ops.close_call_count()
    }

    fn reattach_call_count(&self) -> usize {
        self.ops.reattach_call_count()
    }

    // ---- pending delivery ----

    fn execute(&mut self, op: &PendingOp) {
        match op.kind {
            PendingKind::Open {
                request,
                lease,
                size,
                mode,
            } => {
                let Some(session) = self.sessions.session_mut(op.session) else {
                    self.stale_drops += 1;
                    return;
                };
                let result = self.ops.open(op.session, &lease, size, mode);
                if result.is_ok() {
                    // Adapter-side truth: the editor exists even if
                    // `complete_attach` below rejects the reply as
                    // stale — the orphan must stay visible to `close`.
                    self.live_editors.insert(op.session);
                }
                if let Err(EditorSessionError::StaleRequest) =
                    session.complete_attach(request, result.map(|_| ()))
                {
                    self.stale_rejections += 1;
                }
            }
            PendingKind::Detach {
                request,
                result,
                lease,
            } => {
                let Some(session) = self.sessions.session_mut(op.session) else {
                    self.stale_drops += 1;
                    return;
                };
                match session.complete_detach(request, result) {
                    Ok(reattach_request) => {
                        self.pending.push_back(PendingOp {
                            session: op.session,
                            kind: PendingKind::Reattach {
                                request: reattach_request,
                                lease,
                            },
                        });
                    }
                    Err(EditorSessionError::StaleRequest) => self.stale_rejections += 1,
                    Err(_) => {}
                }
            }
            PendingKind::Reattach { request, lease } => {
                let Some(session) = self.sessions.session_mut(op.session) else {
                    self.stale_drops += 1;
                    return;
                };
                let result = self.ops.reattach(op.session, &lease);
                if let Err(EditorSessionError::StaleRequest) =
                    session.complete_attach(request, result)
                {
                    self.stale_rejections += 1;
                }
            }
            PendingKind::Close { request } => {
                let Some(session) = self.sessions.session_mut(op.session) else {
                    self.stale_drops += 1;
                    return;
                };
                if let Ok(()) = self.ops.close(op.session) {
                    // The adapter confirmed the close — the editor is
                    // gone even if `complete_close` turns out stale.
                    self.live_editors.remove(&op.session);
                    if let Err(EditorSessionError::StaleRequest) = session.complete_close(request) {
                        self.stale_rejections += 1;
                    }
                } else {
                    // No error channel on `complete_close` — the
                    // completion is withheld; the deadline
                    // converges the request to `Failed{Timeout}`.
                }
            }
        }
    }
}

// ---- assertion helpers ------------------------------------------------

/// Asserts the event log ends in exactly one terminal event
/// (`Closed` or `Terminated`) and contains no duplicate terminal.
fn assert_single_terminal(events: &[EditorSessionEvent]) {
    let terminals = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                EditorSessionEvent::Closed | EditorSessionEvent::Terminated { .. }
            )
        })
        .count();
    assert_eq!(
        terminals, 1,
        "expected exactly one terminal event: {events:?}"
    );
}

fn attached_events(mode: AttachMode) -> EditorSessionEvent {
    EditorSessionEvent::Attached { mode }
}

// ---- SM-xx scenario tests --------------------------------------------

/// SM-01: full driver-driven lifecycle — open/attach, hide/show,
/// resize, two-phase reparent, close — with the ops call order and
/// the complete event log asserted.
#[test]
fn sm01_driver_lifecycle_open_hide_show_resize_reparent_close() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");

    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Creating { request: 1 })
    );
    assert_eq!(driver.pump(), 1);
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Attached {
            mode: AttachMode::Docked
        })
    );

    driver.set_visible(id, false).expect("hide");
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Hidden {
            mode: AttachMode::Docked
        })
    );
    driver.set_visible(id, true).expect("show");
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Attached {
            mode: AttachMode::Docked
        })
    );

    driver.resize(id, SIZE_LARGE).expect("resize");
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Attached {
            mode: AttachMode::Docked
        })
    );

    // Two-phase reparent: detach half on one pump, re-attach half on
    // the next — the intermediate `Creating` request is observable.
    driver.reparent(id, lease()).expect("reparent");
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Detaching { request: 2 })
    );
    assert_eq!(driver.pump(), 1);
    assert!(matches!(
        driver.state(id),
        Some(EditorSessionState::Creating { .. })
    ));
    assert_eq!(driver.pump(), 1);
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Attached {
            mode: AttachMode::Docked
        })
    );

    driver.close(id).expect("close");
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Closing { request: 4 })
    );
    assert_eq!(driver.pump(), 1);
    assert_eq!(driver.state(id), Some(EditorSessionState::Closed));

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            attached_events(AttachMode::Docked),
            EditorSessionEvent::Hidden,
            EditorSessionEvent::Shown {
                mode: AttachMode::Docked
            },
            EditorSessionEvent::Detached,
            EditorSessionEvent::Reattached {
                mode: AttachMode::Docked
            },
            EditorSessionEvent::Closed,
        ]
    );
    assert_eq!(
        driver.ops_calls(),
        vec![
            OpCall::Open {
                mode: AttachMode::Docked
            },
            OpCall::SetVisibility { visible: false },
            OpCall::SetVisibility { visible: true },
            OpCall::SetSize,
            OpCall::Reattach,
            OpCall::Close,
        ]
    );
    assert_single_terminal(&events);
}

/// SM-02: reparent abort on the detach half — `Failed{PluginFailed}`,
/// the re-attach op is never dispatched, and `recover` rolls the
/// session back to `Attached` (`Recovered`).
#[test]
fn sm02_reparent_detach_abort_fails_then_recovers() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();

    driver.script_detach(Err(EditorOpError::Plugin));
    driver.reparent(id, lease()).expect("reparent");
    driver.pump();

    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Failed {
            cause: SessionFailCause::PluginFailed
        })
    );
    assert_eq!(driver.reattach_call_count(), 0, "attach half must not run");

    driver
        .recover(id, lease(), SIZE, AttachMode::Docked)
        .expect("recover");
    driver.pump();
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Attached {
            mode: AttachMode::Docked
        })
    );

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            attached_events(AttachMode::Docked),
            EditorSessionEvent::Failed {
                cause: SessionFailCause::PluginFailed
            },
            EditorSessionEvent::Recovered {
                mode: AttachMode::Docked
            },
        ]
    );
}

/// SM-03: reparent abort on the re-attach half — `Detached` fired,
/// `ops.reattach` refused with `LeaseRejected` → `Failed{LeaseInvalid}`;
/// the live editor still closes through the adapter.
#[test]
fn sm03_reparent_reattach_abort_fails_then_closes() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();
    driver.ops_mut().reattach_result = Err(EditorOpError::LeaseRejected);

    driver.reparent(id, lease()).expect("reparent");
    driver.pump();
    assert!(matches!(
        driver.state(id),
        Some(EditorSessionState::Creating { .. })
    ));
    driver.pump();

    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Failed {
            cause: SessionFailCause::LeaseInvalid
        })
    );
    assert_eq!(driver.reattach_call_count(), 1);

    driver.close(id).expect("close");
    assert_eq!(driver.pump(), 1);
    assert_eq!(driver.state(id), Some(EditorSessionState::Closed));
    assert_eq!(
        driver.close_call_count(),
        1,
        "live editor closes via adapter"
    );

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            attached_events(AttachMode::Docked),
            EditorSessionEvent::Detached,
            EditorSessionEvent::Failed {
                cause: SessionFailCause::LeaseInvalid
            },
            EditorSessionEvent::Closed,
        ]
    );
    assert_single_terminal(&events);
}

/// SM-04: a pending completion addressed to a retired session id
/// (slot generation bumped by `end_session`) is dropped without
/// touching the recycled session.
#[test]
fn sm04_stale_session_id_delivery_is_dropped_after_generation_recycle() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id_a = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id_a, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();

    // Hold a detach completion for id_a across terminate + retire.
    driver.reparent(id_a, lease()).expect("reparent");
    driver.crash(inst, SessionFailCause::HostLost);
    assert_eq!(driver.state(id_a), Some(EditorSessionState::Closed));
    driver.end_session(id_a).expect("end_session");
    assert!(driver.session(id_a).is_none());

    // The recycled slot mints a new generation for the same instance.
    let id_b = driver.begin_instance(inst).expect("begin_instance");
    assert_eq!(id_b.slot(), id_a.slot());
    assert!(id_b.generation() > id_a.generation());
    driver
        .open(id_b, lease(), SIZE, AttachMode::Docked)
        .expect("open");

    // One pump delivers the stale detach (dropped) then id_b's open.
    driver.pump();
    assert_eq!(driver.stale_drops(), 1);
    assert_eq!(
        driver.state(id_b),
        Some(EditorSessionState::Attached {
            mode: AttachMode::Docked
        })
    );

    let events_b = driver.drain_events(id_b);
    assert_eq!(events_b, vec![attached_events(AttachMode::Docked)]);
    assert!(driver.session(id_a).is_none());
}

/// SM-05: a completion carrying a mismatched request id is rejected
/// `StaleRequest` — state and events are untouched, and the real
/// request still completes normally.
#[test]
fn sm05_stale_request_completion_is_rejected() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");

    let request = match driver.state(id) {
        Some(EditorSessionState::Creating { request }) => request,
        other => panic!("expected Creating, got {other:?}"),
    };
    let outcome = driver.inject_attach_completion(id, request + 1, Ok(()));
    assert_eq!(outcome, Err(EditorSessionError::StaleRequest));
    assert_eq!(driver.stale_rejections(), 1);
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Creating { request })
    );
    assert_eq!(driver.drain_events(id), Vec::new());

    driver.pump();
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Attached {
            mode: AttachMode::Docked
        })
    );
    assert_eq!(
        driver.drain_events(id),
        vec![attached_events(AttachMode::Docked)]
    );
}

/// SM-06: double close — the second `close` is refused `InvalidState`,
/// `ops.close` is not re-dispatched, and `Closed` fires exactly once.
#[test]
fn sm06_double_close_is_rejected_with_single_terminal() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();

    driver.close(id).expect("close");
    driver.pump();
    assert_eq!(driver.state(id), Some(EditorSessionState::Closed));

    let outcome = driver.close(id);
    assert_eq!(outcome, Err(EditorSessionError::InvalidState));
    assert_eq!(
        driver.close_call_count(),
        1,
        "adapter close must not re-dispatch"
    );

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            attached_events(AttachMode::Docked),
            EditorSessionEvent::Closed,
        ]
    );
    assert_single_terminal(&events);
    driver.end_session(id).expect("end_session");
}

/// SM-07: a hung open expires via the deadline — `Failed{Timeout}` —
/// and the late adapter result is `StaleRequest`, never a second
/// terminal or a fake attach. The stale-but-successful `ops.open`
/// still records an adapter-side orphan that a later `close`
/// dispatches `ops.close` for.
#[test]
fn sm07_hung_open_times_out_and_late_completion_is_stale() {
    let mut driver = FakeHostDriver::new();
    driver.set_timeout(Duration::from_secs(5));
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");

    driver.ops_mut().hang(OpKind::Open);
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    // A pre-deadline `tick` is a no-op and a `pump` while hung defers
    // dispatch — the request stays in flight.
    assert_eq!(driver.tick_all(), 0);
    assert_eq!(driver.pump(), 0);
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Creating { request: 1 })
    );

    driver.advance(Duration::from_secs(6));
    assert_eq!(driver.tick_all(), 1);

    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        })
    );
    assert_eq!(
        driver.open_call_count(),
        0,
        "hung op never reached the adapter"
    );

    driver.ops_mut().unhang(OpKind::Open);
    driver.pump();
    assert_eq!(
        driver.open_call_count(),
        1,
        "late dispatch still runs the op"
    );
    assert_eq!(driver.stale_rejections(), 1);
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        })
    );
    assert!(
        driver.is_live(id),
        "adapter-side orphan: ops.open succeeded even though the completion was stale"
    );

    // The orphan stays visible to the driver ledger — `close`
    // dispatches `ops.close` and converges the session to `Closed`.
    driver.close(id).expect("close");
    driver.pump();
    assert_eq!(driver.state(id), Some(EditorSessionState::Closed));
    assert_eq!(driver.close_call_count(), 1);
    assert!(!driver.is_live(id));

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Timeout
            },
            EditorSessionEvent::Closed,
        ]
    );
    assert_single_terminal(&events);
}

/// SM-08: a hung close expires via the deadline — `Failed{Timeout}`
/// with the editor still live — and the late `ops.close` `Ok`
/// clears the adapter ledger even though `complete_close` is stale;
/// a later `close` then converges the session without re-dispatching
/// the adapter op.
#[test]
fn sm08_hung_close_times_out_then_retry_converges() {
    let mut driver = FakeHostDriver::new();
    driver.set_timeout(Duration::from_secs(5));
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();

    driver.ops_mut().hang(OpKind::Close);
    driver.close(id).expect("close");
    driver.advance(Duration::from_secs(6));
    assert_eq!(driver.tick_all(), 1);
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        })
    );

    // The withheld close never completed — releasing the hang lets the
    // adapter run; `ops.close` `Ok` clears the ledger while
    // `complete_close` is stale against `Failed`.
    driver.ops_mut().unhang(OpKind::Close);
    driver.pump();
    assert_eq!(driver.stale_rejections(), 1);
    assert!(!driver.is_live(id), "adapter confirmed the close");

    // `Failed` still allows `begin_close`; the adapter editor is
    // already gone, so the session converges to `Closed` without a
    // second `ops.close` dispatch.
    driver.close(id).expect("close retry");
    assert_eq!(driver.state(id), Some(EditorSessionState::Closed));
    assert_eq!(driver.close_call_count(), 1);

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            attached_events(AttachMode::Docked),
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Timeout
            },
            EditorSessionEvent::Closed,
        ]
    );
    assert_single_terminal(&events);
}

/// SM-09a: a hung platform detach expires the `Detaching` request —
/// `Failed{Timeout}` with no `Detached` event and no re-attach op.
#[test]
fn sm09a_hung_detach_times_out_without_detached_event() {
    let mut driver = FakeHostDriver::new();
    driver.set_timeout(Duration::from_secs(5));
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();

    driver.ops_mut().hang(OpKind::Detach);
    driver.reparent(id, lease()).expect("reparent");
    driver.pump();
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Detaching { request: 2 })
    );

    driver.advance(Duration::from_secs(6));
    assert_eq!(driver.tick_all(), 1);
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        })
    );

    driver.ops_mut().unhang(OpKind::Detach);
    driver.pump();
    assert_eq!(driver.stale_rejections(), 1);
    assert_eq!(driver.reattach_call_count(), 0);

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            attached_events(AttachMode::Docked),
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Timeout
            },
        ]
    );
    assert!(!events.contains(&EditorSessionEvent::Detached));
}

/// SM-09b: the re-attach request minted by `complete_detach` carries
/// its own deadline — a hung `ops.reattach` expires it to
/// `Failed{Timeout}` after `Detached` already fired.
#[test]
fn sm09b_hung_reattach_times_out_the_reattach_request() {
    let mut driver = FakeHostDriver::new();
    driver.set_timeout(Duration::from_secs(5));
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();

    driver.reparent(id, lease()).expect("reparent");
    driver.ops_mut().hang(OpKind::Reattach);
    driver.pump();
    assert!(matches!(
        driver.state(id),
        Some(EditorSessionState::Creating { .. })
    ));

    driver.advance(Duration::from_secs(6));
    assert_eq!(driver.tick_all(), 1);
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        })
    );

    driver.ops_mut().unhang(OpKind::Reattach);
    driver.pump();
    assert_eq!(driver.reattach_call_count(), 1);
    assert_eq!(driver.stale_rejections(), 1);

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            attached_events(AttachMode::Docked),
            EditorSessionEvent::Detached,
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Timeout
            },
        ]
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, EditorSessionEvent::Reattached { .. }))
    );
}

/// SM-10: OS/adapter `Unsupported` on the preferred mode — the driver
/// explicitly degrades to `Floating` via `begin_degrade`; the session
/// lands `Attached{Floating}` emitting `Degraded`, never `Attached`.
#[test]
fn sm10_unsupported_preferred_mode_degrades_explicitly_to_floating() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .ops_mut()
        .open_by_mode
        .insert(AttachMode::Docked, Err(EditorOpError::Unsupported));

    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Failed {
            cause: SessionFailCause::Unsupported
        })
    );

    driver
        .degrade(id, lease(), SIZE, AttachMode::Floating)
        .expect("degrade");
    driver.pump();
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Attached {
            mode: AttachMode::Floating
        })
    );

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Unsupported
            },
            EditorSessionEvent::Degraded {
                mode: AttachMode::Floating
            },
        ]
    );
    assert!(
        !events.iter().any(|event| matches!(
            event,
            EditorSessionEvent::Attached { .. }
                | EditorSessionEvent::Recovered { .. }
                | EditorSessionEvent::Reattached { .. }
        )),
        "a degrade must never emit a requested-mode attach event: {events:?}"
    );
    assert_eq!(driver.open_call_count(), 2, "preferred then fallback open");
}

/// SM-11: an adapter that refuses everything (L1-shaped) reports all
/// capabilities `Unsupported`, fails open with `Failed{Unsupported}`,
/// and closes a never-live editor without an adapter call — defined
/// refusals never become fake success.
#[test]
fn sm11_all_unsupported_adapter_fails_defined_without_fake_success() {
    let mut driver = FakeHostDriver::new();
    *driver.ops_mut() = ScriptedOps::unsupported();
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");

    let caps = driver.probe(id).expect("capabilities");
    assert_eq!(caps, EditorCapabilities::default());

    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Failed {
            cause: SessionFailCause::Unsupported
        })
    );

    driver.close(id).expect("close");
    assert_eq!(driver.state(id), Some(EditorSessionState::Closed));
    assert_eq!(
        driver.close_call_count(),
        0,
        "no adapter editor — no close op"
    );

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Unsupported
            },
            EditorSessionEvent::Closed,
        ]
    );
    assert_single_terminal(&events);
}

/// SM-12: plugin host crash (`HostLost`) converges a live session to
/// `Closed` + `Terminated` exactly once; a held reparent completion
/// delivered afterwards is stale, and a repeated crash adds nothing.
#[test]
fn sm12_host_crash_terminates_once_and_late_completion_is_stale() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();

    driver.reparent(id, lease()).expect("reparent");
    driver.crash(inst, SessionFailCause::HostLost);
    assert_eq!(driver.state(id), Some(EditorSessionState::Closed));
    assert_eq!(
        driver.session_for_instance(inst),
        None,
        "binding freed on crash"
    );

    // The held detach completion arrives after termination — stale.
    driver.pump();
    assert_eq!(driver.stale_rejections(), 1);

    driver.crash(inst, SessionFailCause::HostLost);
    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            attached_events(AttachMode::Docked),
            EditorSessionEvent::Terminated {
                cause: SessionFailCause::HostLost
            },
        ]
    );
    assert_single_terminal(&events);
}

/// SM-13: GPU host crash (`GraphicsLost`) during an in-flight open —
/// `Terminated{GraphicsLost}` once, and the late attach completion
/// cannot produce a second terminal.
#[test]
fn sm13_gpu_host_crash_during_open_terminates_once() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");

    driver.ops_mut().hang(OpKind::Open);
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.crash(inst, SessionFailCause::GraphicsLost);
    assert_eq!(driver.state(id), Some(EditorSessionState::Closed));

    driver.ops_mut().unhang(OpKind::Open);
    driver.pump();
    assert_eq!(driver.stale_rejections(), 1);

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![EditorSessionEvent::Terminated {
            cause: SessionFailCause::GraphicsLost
        }]
    );
    assert_single_terminal(&events);
}

/// SM-14: window close (`Closed`) and DSP instance destroy
/// (`Terminated`) are distinct terminals — a graceful close keeps the
/// instance binding until `end_session`, while a crash frees it
/// immediately.
#[test]
fn sm14_window_close_and_instance_destroy_are_distinct_terminals() {
    let mut driver = FakeHostDriver::new();
    let inst_a = instance(3);
    let inst_b = instance(4);
    let id_a = driver.begin_instance(inst_a).expect("begin_instance");
    let id_b = driver.begin_instance(inst_b).expect("begin_instance");
    driver
        .open(id_a, lease(), SIZE, AttachMode::Docked)
        .expect("open a");
    driver
        .open(id_b, lease(), SIZE, AttachMode::Docked)
        .expect("open b");
    driver.pump();

    driver.close(id_a).expect("close a");
    driver.pump();
    assert_eq!(driver.state(id_a), Some(EditorSessionState::Closed));
    assert_eq!(
        driver.session_for_instance(inst_a),
        Some(id_a),
        "graceful close keeps the binding until end_session"
    );

    driver.crash(inst_b, SessionFailCause::HostLost);
    assert_eq!(driver.state(id_b), Some(EditorSessionState::Closed));
    assert_eq!(driver.session_for_instance(inst_b), None);

    let events_a = driver.drain_events(id_a);
    let events_b = driver.drain_events(id_b);
    assert_eq!(events_a.last(), Some(&EditorSessionEvent::Closed));
    assert_eq!(
        events_b.last(),
        Some(&EditorSessionEvent::Terminated {
            cause: SessionFailCause::HostLost
        })
    );
    assert!(
        !events_a
            .iter()
            .any(|event| matches!(event, EditorSessionEvent::Terminated { .. }))
    );
}

/// SM-15: recreate — after close + `end_session`, `begin_instance`
/// mints the same slot with a bumped generation; the new session
/// opens cleanly and the retired id accepts nothing.
#[test]
fn sm15_recreate_after_close_mints_new_generation() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id_a = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id_a, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();
    driver.close(id_a).expect("close");
    driver.pump();
    driver.end_session(id_a).expect("end_session");

    let id_b = driver.begin_instance(inst).expect("begin_instance");
    assert_eq!(id_b.slot(), id_a.slot());
    assert!(id_b.generation() > id_a.generation());
    assert!(driver.session(id_a).is_none());

    driver
        .open(id_b, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();
    assert_eq!(
        driver.state(id_b),
        Some(EditorSessionState::Attached {
            mode: AttachMode::Docked
        })
    );
    assert_eq!(
        driver.drain_events(id_b),
        vec![attached_events(AttachMode::Docked)]
    );

    // The retired id is unknown to the registry — driver verbs refuse.
    assert_eq!(
        driver.open(id_a, lease(), SIZE, AttachMode::Docked),
        Err(EditorSessionError::UnknownSession)
    );
}

/// SM-16: a plugin-side resize refusal is non-fatal — `ops.set_size`
/// errors surface to the driver while the session stays `Attached`
/// and emits nothing.
#[test]
fn sm16_resize_refusal_is_non_fatal_and_observable() {
    let mut driver = FakeHostDriver::new();
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();
    driver.drain_events(id);

    driver.ops_mut().size_result = Err(EditorOpError::Plugin);
    let outcome = driver.resize(id, SIZE_LARGE);
    assert_eq!(outcome, Err(EditorSessionError::Ops(EditorOpError::Plugin)));
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Attached {
            mode: AttachMode::Docked
        })
    );
    assert_eq!(
        driver.drain_events(id),
        Vec::new(),
        "resize refusal emits no event"
    );
}

/// SM-17: an `ops.close` *refusal* (not a hang) has no
/// `complete_close` error channel — the completion is withheld, the
/// deadline converges the request to `Failed{Timeout}` with the
/// adapter editor still live, and a later `close` retries through
/// the adapter to a single `Closed`.
#[test]
fn sm17_close_refusal_is_withheld_then_timeout_and_retry_converges() {
    let mut driver = FakeHostDriver::new();
    driver.set_timeout(Duration::from_secs(5));
    let inst = instance(3);
    let id = driver.begin_instance(inst).expect("begin_instance");
    driver
        .open(id, lease(), SIZE, AttachMode::Docked)
        .expect("open");
    driver.pump();

    driver.ops_mut().close_result = Err(EditorOpError::Plugin);
    driver.close(id).expect("close");
    driver.pump();
    // `ops.close` was dispatched and refused — `complete_close` is
    // withheld, so the request stays in flight.
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Closing { request: 2 })
    );
    assert!(driver.is_live(id));

    driver.advance(Duration::from_secs(6));
    assert_eq!(driver.tick_all(), 1);
    assert_eq!(
        driver.state(id),
        Some(EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        })
    );
    assert!(
        driver.is_live(id),
        "a refused close leaves the adapter editor live"
    );

    driver.ops_mut().close_result = Ok(());
    driver.close(id).expect("close retry");
    driver.pump();
    assert_eq!(driver.state(id), Some(EditorSessionState::Closed));
    assert_eq!(driver.close_call_count(), 2);
    assert!(!driver.is_live(id));

    let events = driver.drain_events(id);
    assert_eq!(
        events,
        vec![
            attached_events(AttachMode::Docked),
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Timeout
            },
            EditorSessionEvent::Closed,
        ]
    );
    assert_single_terminal(&events);
}
