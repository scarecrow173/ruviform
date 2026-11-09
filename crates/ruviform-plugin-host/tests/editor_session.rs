//! Contract tests for `ruviform_plugin_host::editor_session`
//! (`PluginEditorSession` / `PluginEditorSessions` /
//! `EditorSessionOps` / `InProcessEditorOps` / `L1EditorOps`).
//!
//! Fake ops pin the state machine, request-generation and terminal-once
//! invariants; the L0 `InProcessEditorOps` tests drive the real
//! `InProcessHost` `OpenGui`/`CloseGui` path behind a stub plugin. These
//! are contract-level results — **not** a real Dock PASS (no GPUI
//! window, OS parent or real plugin). The L1 adapter remains defined-
//! `Unsupported` on every op and never degrades to L0.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, GuiHandle, GuiParentToken, ParamInfo, PluginDescriptor,
    PrepareError, ProcessConfig, ProcessContext, ProcessResult, ProcessStatus, TransportInfo,
};
use ruviform_plugin_host::{
    AttachMode, EditorCapabilities, EditorOpError, EditorSessionError, EditorSessionEvent,
    EditorSessionOps, EditorSessionState, EditorSize, HostCommand, HostError, HostReply,
    InProcessEditorOps, InProcessHost, L1EditorOps, LeaseRedeemer, OpenOutcome, ParentSlotLease,
    Plugin, PluginEditorSessionId, PluginEditorSessions, PluginFactory, PluginHandle, SessionClock,
    SessionDeadlinePolicy, SessionFailCause, Support, erase,
};

// ---- shared fixtures -------------------------------------------------

const SIZE: EditorSize = EditorSize {
    width: 800.0,
    height: 600.0,
};

fn lease() -> ParentSlotLease {
    // The session side treats the payload as fully opaque — a fake
    // driver-encoded value stands in for a real `SlotLease::encode`.
    ParentSlotLease::new(0xA5A5_0001_0000_0000)
}

fn instance() -> PluginHandle {
    // `#[doc(hidden)]` constructor — tests mint handles directly.
    PluginHandle::new(3, 0)
}

// A controllable fake `EditorSessionOps` for state-machine tests.
struct FakeOps {
    open_result: Result<(), EditorOpError>,
    close_result: Result<(), EditorOpError>,
    size_result: Result<(), EditorOpError>,
    visibility_result: Result<(), EditorOpError>,
    reattach_result: Result<(), EditorOpError>,
    caps: EditorCapabilities,
    open_calls: usize,
}

impl FakeOps {
    fn ok() -> Self {
        Self {
            open_result: Ok(()),
            close_result: Ok(()),
            size_result: Ok(()),
            visibility_result: Ok(()),
            reattach_result: Ok(()),
            caps: EditorCapabilities {
                dock: Support::Supported,
                close: Support::Supported,
                ..EditorCapabilities::default()
            },
            open_calls: 0,
        }
    }

    fn open(mut self, result: Result<(), EditorOpError>) -> Self {
        self.open_result = result;
        self
    }
}

impl EditorSessionOps for FakeOps {
    fn capabilities(
        &self,
        _session: PluginEditorSessionId,
    ) -> Result<EditorCapabilities, EditorOpError> {
        Ok(self.caps)
    }

    fn open(
        &mut self,
        _session: PluginEditorSessionId,
        _lease: &ParentSlotLease,
        _size: EditorSize,
        _mode: AttachMode,
    ) -> Result<EditorCapabilities, EditorOpError> {
        self.open_calls += 1;
        self.open_result.map(|()| self.caps)
    }

    fn close(&mut self, _session: PluginEditorSessionId) -> Result<(), EditorOpError> {
        self.close_result
    }

    fn set_size(
        &mut self,
        _session: PluginEditorSessionId,
        _size: EditorSize,
    ) -> Result<(), EditorOpError> {
        self.size_result
    }

    fn set_visibility(
        &mut self,
        _session: PluginEditorSessionId,
        _visible: bool,
    ) -> Result<(), EditorOpError> {
        self.visibility_result
    }

    fn reattach(
        &mut self,
        _session: PluginEditorSessionId,
        _lease: &ParentSlotLease,
    ) -> Result<(), EditorOpError> {
        self.reattach_result
    }
}

// ---- state machine (fake ops) ---------------------------------------

#[test]
fn open_attaches_and_emits_attached() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok();
    sessions
        .session_mut(id)
        .unwrap()
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .expect("open");
    let session = sessions.session_mut(id).unwrap();
    assert_eq!(
        session.state(),
        EditorSessionState::Attached {
            mode: AttachMode::Docked
        }
    );
    assert_eq!(
        session.drain_events(),
        vec![EditorSessionEvent::Attached {
            mode: AttachMode::Docked
        }]
    );
}

#[test]
fn hide_show_cycles_between_attached_and_hidden() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok();
    let session = sessions.session_mut(id).unwrap();
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    session.drain_events();
    session.set_visibility(&mut ops, false).unwrap();
    assert!(matches!(session.state(), EditorSessionState::Hidden { .. }));
    session.set_visibility(&mut ops, true).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Attached { .. }
    ));
    assert_eq!(
        session.drain_events(),
        vec![
            EditorSessionEvent::Hidden,
            EditorSessionEvent::Shown {
                mode: AttachMode::Docked
            },
        ]
    );
}

#[test]
fn detach_and_reattach_roundtrip() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok();
    let session = sessions.session_mut(id).unwrap();
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    session.drain_events();
    session.reattach(&mut ops, &lease()).expect("reattach");
    assert!(matches!(
        session.state(),
        EditorSessionState::Attached { .. }
    ));
    // Sync and async paths share the `[Detached, Reattached]` vocabulary.
    assert_eq!(
        session.drain_events(),
        vec![
            EditorSessionEvent::Detached,
            EditorSessionEvent::Reattached {
                mode: AttachMode::Docked
            }
        ]
    );
}

#[test]
fn close_is_terminal_and_idempotent_double_close_rejected() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok();
    let session = sessions.session_mut(id).unwrap();
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    session.close(&mut ops).expect("close");
    assert_eq!(session.state(), EditorSessionState::Closed);
    // Second close on a Closed session is a defined rejection.
    assert_eq!(
        session.close(&mut ops),
        Err(EditorSessionError::InvalidState)
    );
    // A concluded session is dead — it can never be re-opened (else the
    // registry could retire a live session and run two editors).
    assert_eq!(
        session.open(&mut ops, &lease(), SIZE, AttachMode::Docked),
        Err(EditorSessionError::InvalidState)
    );
    assert_eq!(
        session.begin_open(AttachMode::Docked),
        Err(EditorSessionError::InvalidState)
    );
    assert_eq!(
        session.drain_events(),
        vec![
            EditorSessionEvent::Attached {
                mode: AttachMode::Docked
            },
            EditorSessionEvent::Closed,
        ]
    );
}

#[test]
fn double_open_is_rejected() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok();
    let session = sessions.session_mut(id).unwrap();
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    // begin_open on a non-Closed session is a defined rejection — the
    // ops layer is never reached.
    assert_eq!(
        session.open(&mut ops, &lease(), SIZE, AttachMode::Floating),
        Err(EditorSessionError::InvalidState)
    );
    assert_eq!(ops.open_calls, 1, "ops was not re-invoked");
}

#[test]
fn stale_completion_request_is_ignored() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let session = sessions.session_mut(id).unwrap();
    // Drive the low-level API: begin a request, then complete with a
    // stale id — it is ignored and state is unchanged.
    let request = session.begin_open(AttachMode::Docked).unwrap();
    assert_eq!(
        session.complete_attach(request + 1, Ok(())),
        Err(EditorSessionError::StaleRequest)
    );
    assert!(matches!(
        session.state(),
        EditorSessionState::Creating { .. }
    ));
    // The real request still completes.
    session.complete_attach(request, Ok(())).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Attached { .. }
    ));
}

#[test]
fn notify_terminated_fires_once_and_late_callbacks_ignored() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok();
    let session = sessions.session_mut(id).unwrap();
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    session.drain_events();
    session.notify_terminated(SessionFailCause::HostLost);
    assert_eq!(session.state(), EditorSessionState::Closed);
    session.notify_terminated(SessionFailCause::WindowLost);
    assert_eq!(
        session.drain_events(),
        vec![EditorSessionEvent::Terminated {
            cause: SessionFailCause::HostLost
        }],
        "exactly one terminal event"
    );
    // A terminated session can never be re-opened.
    assert_eq!(
        session.open(&mut ops, &lease(), SIZE, AttachMode::Docked),
        Err(EditorSessionError::InvalidState)
    );
}

#[test]
fn terminate_during_inflight_open_fires_once_and_rejects_late_completion() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let session = sessions.session_mut(id).unwrap();
    let request = session.begin_open(AttachMode::Docked).unwrap();
    session.notify_terminated(SessionFailCause::HostLost);
    assert_eq!(
        session.drain_events(),
        vec![EditorSessionEvent::Terminated {
            cause: SessionFailCause::HostLost
        }]
    );
    // The driver's late attach callback must not revive the session.
    assert_eq!(
        session.complete_attach(request, Ok(())),
        Err(EditorSessionError::StaleRequest)
    );
    session.notify_terminated(SessionFailCause::HostLost);
    assert_eq!(session.state(), EditorSessionState::Closed);
    assert_eq!(session.drain_events(), vec![]);
    assert_eq!(
        session.begin_open(AttachMode::Docked),
        Err(EditorSessionError::InvalidState)
    );
}

#[test]
fn terminate_during_inflight_recover_fires_once_and_rejects_late_completion() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let session = sessions.session_mut(id).unwrap();
    let mut ops = FakeOps::ok().open(Err(EditorOpError::Timeout));
    let _ = session.open(&mut ops, &lease(), SIZE, AttachMode::Docked);
    let request = session.begin_recover(AttachMode::Docked).unwrap();
    session.drain_events();
    session.notify_terminated(SessionFailCause::WindowLost);
    assert_eq!(
        session.drain_events(),
        vec![EditorSessionEvent::Terminated {
            cause: SessionFailCause::WindowLost
        }]
    );
    assert_eq!(
        session.complete_attach(request, Ok(())),
        Err(EditorSessionError::StaleRequest)
    );
    assert_eq!(session.state(), EditorSessionState::Closed);
    assert_eq!(session.drain_events(), vec![]);
}

#[test]
fn failed_reattach_keeps_live_editor_and_closes_through_adapter() {
    // A refused re-attach may leave the editor alive, so the close must
    // still reach the adapter (and its refusal surfaces, not a silent
    // conclusion).
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let session = sessions.session_mut(id).unwrap();
    let mut ops = FakeOps {
        reattach_result: Err(EditorOpError::Plugin),
        close_result: Err(EditorOpError::HostLost),
        ..FakeOps::ok()
    };
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    assert_eq!(
        session.reattach(&mut ops, &lease()),
        Err(EditorSessionError::Ops(EditorOpError::Plugin))
    );
    assert_eq!(
        session.close(&mut ops),
        Err(EditorSessionError::Ops(EditorOpError::HostLost))
    );
    assert_eq!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::HostLost
        }
    );
}

#[test]
fn ops_unsupported_maps_to_failed_cause() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok().open(Err(EditorOpError::Unsupported));
    let session = sessions.session_mut(id).unwrap();
    assert_eq!(
        session.open(&mut ops, &lease(), SIZE, AttachMode::Docked),
        Err(EditorSessionError::Ops(EditorOpError::Unsupported))
    );
    assert_eq!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::Unsupported
        }
    );
}

#[test]
fn ops_lease_rejected_maps_to_lease_invalid() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok().open(Err(EditorOpError::LeaseRejected));
    let session = sessions.session_mut(id).unwrap();
    let _ = session.open(&mut ops, &lease(), SIZE, AttachMode::Docked);
    assert_eq!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::LeaseInvalid
        }
    );
}

#[test]
fn ops_timeout_maps_to_timeout_cause() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok().open(Err(EditorOpError::Timeout));
    let session = sessions.session_mut(id).unwrap();
    let _ = session.open(&mut ops, &lease(), SIZE, AttachMode::Docked);
    assert_eq!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        }
    );
}

#[test]
fn failed_session_recovers_to_attached() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok().open(Err(EditorOpError::Timeout));
    let session = sessions.session_mut(id).unwrap();
    let _ = session.open(&mut ops, &lease(), SIZE, AttachMode::Docked);
    assert!(matches!(session.state(), EditorSessionState::Failed { .. }));
    // Recovery re-opens under the same ops (now succeeding).
    let mut ok_ops = FakeOps::ok();
    session
        .recover(&mut ok_ops, &lease(), SIZE, AttachMode::Floating)
        .expect("recover");
    assert_eq!(
        session.state(),
        EditorSessionState::Attached {
            mode: AttachMode::Floating
        }
    );
    assert_eq!(
        session.drain_events(),
        vec![
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Timeout
            },
            EditorSessionEvent::Recovered {
                mode: AttachMode::Floating
            },
        ]
    );
}

#[test]
fn resize_rejection_is_non_fatal_and_observable() {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    let mut ops = FakeOps::ok();
    ops.size_result = Err(EditorOpError::Plugin);
    let session = sessions.session_mut(id).unwrap();
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    assert_eq!(
        session.set_size(
            &mut ops,
            EditorSize {
                width: 640.0,
                height: 480.0
            }
        ),
        Err(EditorSessionError::Ops(EditorOpError::Plugin))
    );
    // Session survives the rejected resize.
    assert!(matches!(
        session.state(),
        EditorSessionState::Attached { .. }
    ));
}

#[test]
fn second_live_session_on_instance_is_session_busy() {
    let mut sessions = PluginEditorSessions::new();
    let instance = instance();
    let first = sessions.begin_session(instance).unwrap();
    // A second live session for the same instance is refused.
    assert_eq!(
        sessions.begin_session(instance),
        Err(EditorSessionError::SessionBusy)
    );
    // Once the first closes, a new session may begin.
    let mut ops = FakeOps::ok();
    let session = sessions.session_mut(first).unwrap();
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    session.close(&mut ops).unwrap();
    // A concluded session is dead — the re-open attack that would have
    // let `begin_session` retire a live session is refused.
    assert_eq!(
        session.open(&mut ops, &lease(), SIZE, AttachMode::Docked),
        Err(EditorSessionError::InvalidState)
    );
    let second = sessions.begin_session(instance).expect("after close");
    assert_ne!(first, second);
    // The recycled slot bumps its generation.
    assert_eq!(first.slot(), second.slot());
    assert!(second.generation() > first.generation());
}

#[test]
fn end_session_refuses_live_sessions_and_retires_closed_ones() {
    let mut sessions = PluginEditorSessions::new();
    let instance = instance();
    let id = sessions.begin_session(instance).unwrap();
    let mut ops = FakeOps::ok();
    sessions
        .session_mut(id)
        .unwrap()
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    // Retiring a live Attached session would orphan the plugin editor —
    // it must be closed or converged first.
    assert_eq!(
        sessions.end_session(id),
        Err(EditorSessionError::InvalidState)
    );
    assert!(sessions.session(id).is_some());
    sessions.session_mut(id).unwrap().close(&mut ops).unwrap();
    sessions.end_session(id).expect("retire closed session");
    assert!(sessions.session(id).is_none());
    assert!(sessions.session_for_instance(instance).is_none());
}

#[test]
fn terminate_on_never_opened_session_poisons_it() {
    let mut sessions = PluginEditorSessions::new();
    let instance = instance();
    let id = sessions.begin_session(instance).unwrap();
    let mut ops = FakeOps::ok();
    let session = sessions.session_mut(id).unwrap();
    // Born-Closed: nothing live to terminate — no event, but the
    // session is poisoned so it can never be opened later.
    session.notify_terminated(SessionFailCause::HostLost);
    assert_eq!(session.drain_events(), Vec::<EditorSessionEvent>::new());
    assert_eq!(
        session.open(&mut ops, &lease(), SIZE, AttachMode::Docked),
        Err(EditorSessionError::InvalidState)
    );
    // The registry may retire it and mint a fresh session.
    sessions.end_session(id).unwrap();
    assert!(sessions.begin_session(instance).is_ok());
}

#[test]
fn notify_terminated_on_registry_converges_instance_sessions() {
    let mut sessions = PluginEditorSessions::new();
    let instance = instance();
    let id = sessions.begin_session(instance).unwrap();
    let mut ops = FakeOps::ok();
    sessions
        .session_mut(id)
        .unwrap()
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    sessions.session_mut(id).unwrap().drain_events();
    sessions.notify_terminated(instance, SessionFailCause::HostLost);
    let session = sessions.session_mut(id).unwrap();
    assert_eq!(session.state(), EditorSessionState::Closed);
    assert_eq!(
        session.drain_events(),
        vec![EditorSessionEvent::Terminated {
            cause: SessionFailCause::HostLost
        }]
    );
    // The instance is freed for a fresh session.
    assert!(sessions.session_for_instance(instance).is_none());
    assert!(sessions.begin_session(instance).is_ok());
}

// ---- L0 `InProcessEditorOps` (real InProcessHost path) ---------------

const GAIN_ID: &str = "org.ruviform.test.gain";

const GAIN_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: GAIN_ID,
    name: "Test Gain",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// Minimal `PluginProcessor` + `Plugin` whose `open_gui` is switchable.
struct GuiPlugin {
    gui_supported: bool,
}

impl ruviform_plugin_core::PluginProcessor for GuiPlugin {
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        Ok(())
    }

    fn process<I, O>(
        &mut self,
        _inputs: &[I],
        _outputs: &mut [O],
        _events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        ProcessResult::new(ProcessStatus::Continue, ProcessResult::TAIL_NONE)
    }
}

impl Plugin for GuiPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        GAIN_DESCRIPTOR
    }

    fn params(&self) -> &[ParamInfo] {
        &[]
    }

    fn open_gui(&mut self, _parent: GuiParentToken) -> Result<GuiHandle, HostError> {
        if self.gui_supported {
            Ok(GuiHandle::new(7))
        } else {
            Err(HostError::Unsupported)
        }
    }

    fn close_gui(&mut self) -> Result<(), HostError> {
        if self.gui_supported {
            Ok(())
        } else {
            Err(HostError::Unsupported)
        }
    }
}

struct GuiFactory {
    gui_supported: bool,
}

impl PluginFactory for GuiFactory {
    fn descriptor(&self) -> PluginDescriptor {
        GAIN_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ruviform_plugin_host::ErasedPlugin> {
        erase(GuiPlugin {
            gui_supported: self.gui_supported,
        })
    }
}

fn host_with_plugin(gui_supported: bool) -> InProcessHost {
    let mut host = InProcessHost::new();
    host.register_factory(Box::new(GuiFactory { gui_supported }));
    host
}

/// Stands in for the OS-adapter issuer: maps the lease payload to the
/// raw parent token the plugin receives. The real issuer validates the
/// lease first (see `l0_redeemer_refusal_blocks_open_and_never_issues_opengui`).
fn token_redeemer() -> Box<LeaseRedeemer<'static>> {
    Box::new(|lease: &ParentSlotLease| Ok(GuiParentToken::new(lease.raw())))
}

fn instantiate(host: &mut InProcessHost) -> PluginHandle {
    match host
        .execute(HostCommand::Instantiate {
            plugin_id: GAIN_ID.to_owned(),
        })
        .unwrap()
    {
        HostReply::Instantiated { handle } => handle,
        _ => panic!("unexpected reply"),
    }
}

#[test]
fn l0_inprocess_ops_opens_and_closes_real_gui_path() {
    let mut host = host_with_plugin(true);
    let instance = instantiate(&mut host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    {
        let mut ops = InProcessEditorOps::with_redeemer(&mut host, instance, token_redeemer());
        session
            .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
            .expect("open via InProcessHost");
        assert!(matches!(
            session.state(),
            EditorSessionState::Attached { .. }
        ));
        session.close(&mut ops).expect("close via InProcessHost");
        assert_eq!(session.state(), EditorSessionState::Closed);
    }
}

#[test]
fn l0_floating_open_is_explicitly_unsupported_not_faked() {
    // The L0 command surface carries a plain docked open only — a
    // Floating request must be a defined refusal, never a docked open
    // recorded as a float.
    let mut host = host_with_plugin(true);
    let instance = instantiate(&mut host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    {
        let mut ops = InProcessEditorOps::with_redeemer(&mut host, instance, token_redeemer());
        assert_eq!(
            session.open(&mut ops, &lease(), SIZE, AttachMode::Floating),
            Err(EditorSessionError::Ops(EditorOpError::Unsupported))
        );
        assert!(matches!(session.state(), EditorSessionState::Failed { .. }));
    }
}

#[test]
fn l0_unsupported_open_gui_maps_to_failed_not_pass() {
    let mut host = host_with_plugin(false);
    let instance = instantiate(&mut host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    {
        let mut ops = InProcessEditorOps::with_redeemer(&mut host, instance, token_redeemer());
        assert_eq!(
            session.open(&mut ops, &lease(), SIZE, AttachMode::Docked),
            Err(EditorSessionError::Ops(EditorOpError::Unsupported))
        );
        assert_eq!(
            session.state(),
            EditorSessionState::Failed {
                cause: SessionFailCause::Unsupported
            }
        );
    }
}

#[test]
fn l0_advertises_no_capability_before_an_accepted_open() {
    // The L0 surface cannot query plugin GUI support, so nothing may be
    // claimed before an open proves dock/close.
    let mut host = host_with_plugin(true);
    let instance = instantiate(&mut host);
    let ops = InProcessEditorOps::with_redeemer(&mut host, instance, token_redeemer());
    assert_eq!(
        ops.capabilities(PluginEditorSessionId::new(0, 0)),
        Ok(EditorCapabilities::default())
    );
}

#[test]
fn refused_open_closes_without_adapter_op_and_frees_instance() {
    // A refused open never produced a plugin editor, so the session
    // must conclude without calling `CloseGui` (which the plugin would
    // refuse too). Otherwise the `Failed` session pins the instance:
    // `begin_session` would return `SessionBusy` for every later editor.
    let mut host = host_with_plugin(false);
    let instance = instantiate(&mut host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    {
        let mut ops = InProcessEditorOps::with_redeemer(&mut host, instance, token_redeemer());
        let session = sessions.session_mut(id).unwrap();
        assert!(
            session
                .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
                .is_err()
        );
        session
            .close(&mut ops)
            .expect("closing a never-attached session needs no adapter op");
        assert_eq!(session.state(), EditorSessionState::Closed);
    }
    sessions.end_session(id).expect("concluded session retires");
    sessions
        .begin_session(instance)
        .expect("instance is free for a new editor session");
}

#[test]
fn l0_redeemer_refusal_blocks_open_and_never_issues_opengui() {
    // The adapter-backed redeemer is the staleness boundary: a rejected
    // lease must fail the open before `OpenGui` ever reaches the plugin.
    let mut host = host_with_plugin(true);
    let instance = instantiate(&mut host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    {
        let mut ops = InProcessEditorOps::with_redeemer(
            &mut host,
            instance,
            Box::new(|_lease| Err(EditorOpError::LeaseRejected)),
        );
        assert_eq!(
            session.open(&mut ops, &lease(), SIZE, AttachMode::Docked),
            Err(EditorSessionError::Ops(EditorOpError::LeaseRejected))
        );
        assert_eq!(
            session.state(),
            EditorSessionState::Failed {
                cause: SessionFailCause::LeaseInvalid
            }
        );
    }
}

#[test]
fn l0_redeemer_token_reaches_plugin_open_gui() {
    // The adapter-backed redeemer maps the opaque lease payload to the
    // raw parent token the plugin sees — distinct from pass-through.
    use std::cell::Cell;
    use std::rc::Rc;

    struct TokenProbe {
        seen: Rc<Cell<Option<u64>>>,
    }
    impl ruviform_plugin_core::PluginProcessor for TokenProbe {
        fn prepare(
            &mut self,
            _config: &ProcessConfig,
            _context: &mut ProcessContext,
        ) -> Result<(), PrepareError> {
            Ok(())
        }
        fn process<I, O>(
            &mut self,
            _inputs: &[I],
            _outputs: &mut [O],
            _events: EventList<'_>,
            _transport: Option<TransportInfo>,
            _context: &mut ProcessContext,
        ) -> ProcessResult
        where
            I: BufferView,
            O: BufferViewMut,
        {
            ProcessResult::new(ProcessStatus::Continue, ProcessResult::TAIL_NONE)
        }
    }
    impl Plugin for TokenProbe {
        fn descriptor(&self) -> PluginDescriptor {
            GAIN_DESCRIPTOR
        }
        fn params(&self) -> &[ParamInfo] {
            &[]
        }
        fn open_gui(&mut self, parent: GuiParentToken) -> Result<GuiHandle, HostError> {
            self.seen.set(Some(parent.raw()));
            Ok(GuiHandle::new(9))
        }
        fn close_gui(&mut self) -> Result<(), HostError> {
            Ok(())
        }
    }
    struct ProbeFactory {
        seen: Rc<Cell<Option<u64>>>,
    }
    impl PluginFactory for ProbeFactory {
        fn descriptor(&self) -> PluginDescriptor {
            GAIN_DESCRIPTOR
        }
        fn create(&self) -> Box<dyn ruviform_plugin_host::ErasedPlugin> {
            erase(TokenProbe {
                seen: self.seen.clone(),
            })
        }
    }

    let seen = Rc::new(Cell::new(None));
    let mut host = InProcessHost::new();
    host.register_factory(Box::new(ProbeFactory { seen: seen.clone() }));
    let instance = instantiate(&mut host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    {
        // The adapter redeems lease 0xAAAA… into token 0xBBBB… — not
        // the pass-through value.
        let mut ops = InProcessEditorOps::with_redeemer(
            &mut host,
            instance,
            Box::new(|_lease| Ok(GuiParentToken::new(0xBBBB))),
        );
        session
            .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
            .expect("open via redeemer");
        assert!(matches!(
            session.state(),
            EditorSessionState::Attached { .. }
        ));
    }
    assert_eq!(seen.get(), Some(0xBBBB));
}

#[test]
fn l1_editor_ops_refuses_every_op_and_never_degrades() {
    let mut ops = L1EditorOps::new();
    let session_id = PluginEditorSessionId::new(0, 0);
    assert_eq!(
        ops.capabilities(session_id),
        Err(EditorOpError::Unsupported)
    );
    assert_eq!(
        ops.open(session_id, &lease(), SIZE, AttachMode::Docked),
        Err(EditorOpError::Unsupported)
    );
    assert_eq!(ops.close(session_id), Err(EditorOpError::Unsupported));
    assert_eq!(
        ops.set_size(session_id, SIZE),
        Err(EditorOpError::Unsupported)
    );
    assert_eq!(
        ops.set_visibility(session_id, false),
        Err(EditorOpError::Unsupported)
    );
    assert_eq!(
        ops.reattach(session_id, &lease()),
        Err(EditorOpError::Unsupported)
    );
}

#[test]
fn session_surface_names_no_gui_toolkit_or_os_window_handle() {
    // The session module is the driver-neutral surface: it must not name
    // a GUI toolkit or an OS window handle. The raw parent token exists
    // only behind the adapter-side `LeaseRedeemer` seam.
    let source = include_str!("../src/editor_session.rs");
    for banned in [
        "gpui",
        "wry",
        "winit",
        "HWND",
        "NSView",
        "XID",
        "raw_window_handle",
    ] {
        assert!(
            !source.contains(banned),
            "editor_session names `{banned}` on its public surface"
        );
    }
}

// ---- hardening: deadline timeout (fake clock) ------------------------
//
// SH-xx cases pin `docs/test-specs/issue-389-session-hardening.md`:
// bounded request deadlines via an injected `SessionClock`, explicit
// `Degraded` outcomes, and the distinct `GraphicsLost` cause. All
// contract-level — no real wall-clock, plugin, GPU or OS surface.

/// A manually advanced clock — `Rc<Cell>` so one clock can be shared
/// across sessions and policies.
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

fn fake_policy(clock: &FakeClock, timeout: Duration) -> SessionDeadlinePolicy {
    SessionDeadlinePolicy {
        timeout,
        clock: Box::new(clock.clone()),
    }
}

/// A session in `sessions` bound to `handle` with `timeout` armed
/// against `clock`.
fn timed_session(
    sessions: &mut PluginEditorSessions,
    handle: PluginHandle,
    clock: &FakeClock,
    timeout: Duration,
) -> PluginEditorSessionId {
    let id = sessions.begin_session(handle).unwrap();
    sessions
        .session_mut(id)
        .unwrap()
        .set_deadline_policy(Some(fake_policy(clock, timeout)));
    id
}

fn bare_session() -> (PluginEditorSessions, PluginEditorSessionId) {
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance()).unwrap();
    (sessions, id)
}

#[test]
fn sh01_open_request_times_out_to_failed_timeout() {
    let mut sessions = PluginEditorSessions::new();
    let clock = FakeClock::default();
    let id = timed_session(&mut sessions, instance(), &clock, Duration::from_secs(5));
    let request = sessions
        .session_mut(id)
        .unwrap()
        .begin_open(AttachMode::Docked)
        .unwrap();
    clock.advance(Duration::from_secs(6));
    let session = sessions.session_mut(id).unwrap();
    assert!(session.tick());
    assert!(matches!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        }
    ));
    let events = session.drain_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        EditorSessionEvent::Failed {
            cause: SessionFailCause::Timeout
        }
    ));
    // A late completion is stale — the timeout already terminated
    // the request; no duplicate terminal event.
    assert!(matches!(
        session.complete_attach(request, Ok(())),
        Err(EditorSessionError::StaleRequest)
    ));
    assert_eq!(session.drain_events().len(), 0);
}

#[test]
fn sh02_tick_before_deadline_is_a_noop() {
    let mut sessions = PluginEditorSessions::new();
    let clock = FakeClock::default();
    let id = timed_session(&mut sessions, instance(), &clock, Duration::from_secs(5));
    let session = sessions.session_mut(id).unwrap();
    let request = session.begin_open(AttachMode::Docked).unwrap();
    clock.advance(Duration::from_secs(4));
    assert!(!session.tick());
    assert!(matches!(
        session.state(),
        EditorSessionState::Creating { .. }
    ));
    assert_eq!(session.drain_events().len(), 0);
    // The request still completes normally after the early tick.
    session.complete_attach(request, Ok(())).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Attached {
            mode: AttachMode::Docked
        }
    ));
}

#[test]
fn sh03_disarmed_policy_leaves_requests_unbounded() {
    let mut sessions = PluginEditorSessions::new();
    let clock = FakeClock::default();
    let id = timed_session(&mut sessions, instance(), &clock, Duration::from_secs(5));
    let session = sessions.session_mut(id).unwrap();
    session.begin_open(AttachMode::Docked).unwrap();
    // Disarm mid-flight — the in-flight request loses its deadline.
    session.set_deadline_policy(None);
    assert!(session.request_deadline().is_none());
    clock.advance(Duration::from_secs(3600));
    assert!(!session.tick());
    assert!(matches!(
        session.state(),
        EditorSessionState::Creating { .. }
    ));
}

#[test]
fn sh04_close_request_timeout_keeps_adapter_close_path() {
    let mut sessions = PluginEditorSessions::new();
    let clock = FakeClock::default();
    let id = timed_session(&mut sessions, instance(), &clock, Duration::from_secs(5));
    let mut ops = FakeOps::ok();
    sessions
        .session_mut(id)
        .unwrap()
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    sessions.session_mut(id).unwrap().begin_close().unwrap();
    clock.advance(Duration::from_secs(6));
    let session = sessions.session_mut(id).unwrap();
    assert!(session.tick());
    // `complete_close` has no error channel — a timed-out close
    // fails the session, but `editor_live` stays set so a later
    // `close` still reaches the adapter.
    assert!(matches!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        }
    ));
    session.close(&mut ops).unwrap();
    assert!(matches!(session.state(), EditorSessionState::Closed));
}

#[test]
fn sh05_detach_request_timeout_emits_no_detached() {
    let mut sessions = PluginEditorSessions::new();
    let clock = FakeClock::default();
    let id = timed_session(&mut sessions, instance(), &clock, Duration::from_secs(5));
    let mut ops = FakeOps::ok();
    sessions
        .session_mut(id)
        .unwrap()
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    sessions.session_mut(id).unwrap().begin_detach().unwrap();
    sessions.session_mut(id).unwrap().drain_events();
    clock.advance(Duration::from_secs(6));
    let session = sessions.session_mut(id).unwrap();
    assert!(session.tick());
    assert!(matches!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        }
    ));
    // The detach never completed — no `Detached` event, only the
    // single `Failed` terminal.
    let events = session.drain_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        EditorSessionEvent::Failed {
            cause: SessionFailCause::Timeout
        }
    ));
}

#[test]
fn sh06_recover_request_timeout_refails_with_timeout() {
    let mut sessions = PluginEditorSessions::new();
    let clock = FakeClock::default();
    let id = timed_session(&mut sessions, instance(), &clock, Duration::from_secs(5));
    let mut ops = FakeOps::ok().open(Err(EditorOpError::Plugin));
    assert!(
        sessions
            .session_mut(id)
            .unwrap()
            .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
            .is_err()
    );
    let session = sessions.session_mut(id).unwrap();
    session.begin_recover(AttachMode::Docked).unwrap();
    clock.advance(Duration::from_secs(6));
    assert!(session.tick());
    assert!(matches!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        }
    ));
}

#[test]
fn sh07_post_detach_reattach_request_times_out() {
    let mut sessions = PluginEditorSessions::new();
    let clock = FakeClock::default();
    let id = timed_session(&mut sessions, instance(), &clock, Duration::from_secs(5));
    let mut ops = FakeOps::ok();
    let session = sessions.session_mut(id).unwrap();
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    let detach = session.begin_detach().unwrap();
    // `complete_detach(Ok)` mints the re-attach request — it carries
    // its own deadline.
    let reattach = session.complete_detach(detach, Ok(())).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Creating { request } if request == reattach
    ));
    clock.advance(Duration::from_secs(6));
    assert!(session.tick());
    assert!(matches!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        }
    ));
}

#[test]
fn sh08_tick_all_times_out_only_expired_sessions() {
    let mut sessions = PluginEditorSessions::new();
    let clock = FakeClock::default();
    let expired = timed_session(
        &mut sessions,
        PluginHandle::new(3, 0),
        &clock,
        Duration::from_secs(5),
    );
    let healthy = timed_session(
        &mut sessions,
        PluginHandle::new(4, 0),
        &clock,
        Duration::from_secs(50),
    );
    let mut ops = FakeOps::ok();
    sessions
        .session_mut(expired)
        .unwrap()
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    sessions
        .session_mut(healthy)
        .unwrap()
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    // Both sessions now have a detach request in flight.
    sessions
        .session_mut(expired)
        .unwrap()
        .begin_detach()
        .unwrap();
    sessions
        .session_mut(healthy)
        .unwrap()
        .begin_detach()
        .unwrap();
    clock.advance(Duration::from_secs(10));
    assert_eq!(sessions.tick_all(), 1);
    let session = sessions.session_mut(expired).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        }
    ));
    // The healthy session's deadline (50s) has not elapsed.
    assert!(matches!(
        sessions.session(healthy).unwrap().state(),
        EditorSessionState::Detaching { .. }
    ));
    assert_eq!(sessions.tick_all(), 0);
}

// ---- hardening: explicit degrade --------------------------------------

/// Fake ops whose `open` result depends on the requested `AttachMode`
/// and records every mode it was asked to open — pins degrade paths.
struct ModeOps {
    results: HashMap<AttachMode, Result<(), EditorOpError>>,
    open_modes: Vec<AttachMode>,
    caps: EditorCapabilities,
}

impl ModeOps {
    fn new() -> Self {
        Self {
            results: HashMap::new(),
            open_modes: Vec::new(),
            caps: EditorCapabilities {
                dock: Support::Supported,
                float: Support::Supported,
                ..EditorCapabilities::default()
            },
        }
    }

    /// Refuses `open` for `mode` with `error`.
    fn refusing(mut self, mode: AttachMode, error: EditorOpError) -> Self {
        self.results.insert(mode, Err(error));
        self
    }
}

impl EditorSessionOps for ModeOps {
    fn capabilities(
        &self,
        _session: PluginEditorSessionId,
    ) -> Result<EditorCapabilities, EditorOpError> {
        Ok(self.caps)
    }

    fn open(
        &mut self,
        _session: PluginEditorSessionId,
        _lease: &ParentSlotLease,
        _size: EditorSize,
        mode: AttachMode,
    ) -> Result<EditorCapabilities, EditorOpError> {
        self.open_modes.push(mode);
        self.results
            .get(&mode)
            .copied()
            .unwrap_or(Ok(()))
            .map(|()| self.caps)
    }

    fn close(&mut self, _session: PluginEditorSessionId) -> Result<(), EditorOpError> {
        Ok(())
    }

    fn set_size(
        &mut self,
        _session: PluginEditorSessionId,
        _size: EditorSize,
    ) -> Result<(), EditorOpError> {
        Ok(())
    }

    fn set_visibility(
        &mut self,
        _session: PluginEditorSessionId,
        _visible: bool,
    ) -> Result<(), EditorOpError> {
        Ok(())
    }

    fn reattach(
        &mut self,
        _session: PluginEditorSessionId,
        _lease: &ParentSlotLease,
    ) -> Result<(), EditorOpError> {
        Ok(())
    }
}

#[test]
fn sh09_preferred_unsupported_degrades_to_floating_explicitly() {
    let (mut sessions, id) = bare_session();
    let mut ops = ModeOps::new().refusing(AttachMode::Docked, EditorOpError::Unsupported);
    let outcome = sessions
        .session_mut(id)
        .unwrap()
        .open_or_degrade(
            &mut ops,
            &lease(),
            SIZE,
            AttachMode::Docked,
            AttachMode::Floating,
        )
        .unwrap();
    assert!(matches!(
        outcome,
        OpenOutcome::Degraded {
            mode: AttachMode::Floating,
            ..
        }
    ));
    let session = sessions.session_mut(id).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Attached {
            mode: AttachMode::Floating
        }
    ));
    // The degrade is its own terminal — `Attached`/`Recovered` never
    // fire, so it cannot be mistaken for a requested-mode attach.
    let events = session.drain_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        EditorSessionEvent::Degraded {
            mode: AttachMode::Floating
        }
    ));
    assert_eq!(ops.open_modes, [AttachMode::Docked, AttachMode::Floating]);
}

#[test]
fn sh10_preferred_supported_never_retries() {
    let (mut sessions, id) = bare_session();
    let mut ops = ModeOps::new();
    let outcome = sessions
        .session_mut(id)
        .unwrap()
        .open_or_degrade(
            &mut ops,
            &lease(),
            SIZE,
            AttachMode::Docked,
            AttachMode::Floating,
        )
        .unwrap();
    assert!(matches!(outcome, OpenOutcome::Attached(_)));
    let session = sessions.session_mut(id).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Attached {
            mode: AttachMode::Docked
        }
    ));
    let events = session.drain_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        EditorSessionEvent::Attached {
            mode: AttachMode::Docked
        }
    ));
    assert_eq!(ops.open_modes, [AttachMode::Docked]);
}

#[test]
fn sh11_non_unsupported_preferred_error_never_retries() {
    let (mut sessions, id) = bare_session();
    let mut ops = ModeOps::new().refusing(AttachMode::Docked, EditorOpError::Plugin);
    let result = sessions.session_mut(id).unwrap().open_or_degrade(
        &mut ops,
        &lease(),
        SIZE,
        AttachMode::Docked,
        AttachMode::Floating,
    );
    assert!(matches!(
        result,
        Err(EditorSessionError::Ops(EditorOpError::Plugin))
    ));
    let session = sessions.session_mut(id).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::PluginFailed
        }
    ));
    assert_eq!(ops.open_modes, [AttachMode::Docked]);
}

#[test]
fn sh12_fallback_failure_terminates_failed() {
    let (mut sessions, id) = bare_session();
    let mut ops = ModeOps::new()
        .refusing(AttachMode::Docked, EditorOpError::Unsupported)
        .refusing(AttachMode::Floating, EditorOpError::HostLost);
    let result = sessions.session_mut(id).unwrap().open_or_degrade(
        &mut ops,
        &lease(),
        SIZE,
        AttachMode::Docked,
        AttachMode::Floating,
    );
    assert!(matches!(
        result,
        Err(EditorSessionError::Ops(EditorOpError::HostLost))
    ));
    let session = sessions.session_mut(id).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::HostLost
        }
    ));
    assert_eq!(ops.open_modes, [AttachMode::Docked, AttachMode::Floating]);
}

#[test]
fn sh13_equal_modes_degenerate_to_plain_open() {
    let (mut sessions, id) = bare_session();
    let mut ops = ModeOps::new();
    let outcome = sessions
        .session_mut(id)
        .unwrap()
        .open_or_degrade(
            &mut ops,
            &lease(),
            SIZE,
            AttachMode::Docked,
            AttachMode::Docked,
        )
        .unwrap();
    assert!(matches!(outcome, OpenOutcome::Attached(_)));
    assert_eq!(ops.open_modes, [AttachMode::Docked]);
}

#[test]
fn sh14_async_degrade_lands_degraded_not_recovered() {
    let (mut sessions, id) = bare_session();
    let mut failing = FakeOps::ok().open(Err(EditorOpError::Unsupported));
    assert!(
        sessions
            .session_mut(id)
            .unwrap()
            .open(&mut failing, &lease(), SIZE, AttachMode::Docked)
            .is_err()
    );
    let session = sessions.session_mut(id).unwrap();
    let request = session.begin_degrade(AttachMode::Floating).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Recovering { .. }
    ));
    session.complete_attach(request, Ok(())).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Attached {
            mode: AttachMode::Floating
        }
    ));
    let events = session.drain_events();
    // `[Failed{Unsupported}, Degraded{Floating}]` — no `Recovered`.
    assert_eq!(events.len(), 2);
    assert!(matches!(
        events[1],
        EditorSessionEvent::Degraded {
            mode: AttachMode::Floating
        }
    ));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, EditorSessionEvent::Recovered { .. }))
    );
}

#[test]
fn sh15_failed_degrade_does_not_poison_later_recover() {
    let (mut sessions, id) = bare_session();
    let mut failing = FakeOps::ok().open(Err(EditorOpError::Unsupported));
    assert!(
        sessions
            .session_mut(id)
            .unwrap()
            .open(&mut failing, &lease(), SIZE, AttachMode::Docked)
            .is_err()
    );
    let session = sessions.session_mut(id).unwrap();
    let request = session.begin_degrade(AttachMode::Floating).unwrap();
    assert!(matches!(
        session.complete_attach(request, Err(EditorOpError::Timeout)),
        Err(EditorSessionError::Ops(EditorOpError::Timeout))
    ));
    assert!(matches!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::Timeout
        }
    ));
    // The degrade flag was cleared by the failure — a plain recover
    // emits `Recovered`, not `Degraded`.
    let mut ops = FakeOps::ok();
    session
        .recover(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    let events = session.drain_events();
    assert!(matches!(
        events.last(),
        Some(EditorSessionEvent::Recovered {
            mode: AttachMode::Docked
        })
    ));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, EditorSessionEvent::Degraded { .. }))
    );
}

// ---- hardening: graphics-lost cause ------------------------------------

#[test]
fn sh16_graphics_lost_maps_to_distinct_fail_cause() {
    let (mut sessions, id) = bare_session();
    let mut ops = FakeOps::ok().open(Err(EditorOpError::GraphicsLost));
    let result =
        sessions
            .session_mut(id)
            .unwrap()
            .open(&mut ops, &lease(), SIZE, AttachMode::Docked);
    assert!(matches!(
        result,
        Err(EditorSessionError::Ops(EditorOpError::GraphicsLost))
    ));
    let session = sessions.session_mut(id).unwrap();
    assert!(matches!(
        session.state(),
        EditorSessionState::Failed {
            cause: SessionFailCause::GraphicsLost
        }
    ));
    let events = session.drain_events();
    assert!(matches!(
        events.as_slice(),
        [EditorSessionEvent::Failed {
            cause: SessionFailCause::GraphicsLost
        }]
    ));
}

#[test]
fn sh17_notify_terminated_graphics_lost_fires_once() {
    let (mut sessions, id) = bare_session();
    let mut ops = FakeOps::ok();
    sessions
        .session_mut(id)
        .unwrap()
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .unwrap();
    let session = sessions.session_mut(id).unwrap();
    session.notify_terminated(SessionFailCause::GraphicsLost);
    session.notify_terminated(SessionFailCause::GraphicsLost);
    let events = session.drain_events();
    assert!(matches!(
        events.as_slice(),
        [
            EditorSessionEvent::Attached { .. },
            EditorSessionEvent::Terminated {
                cause: SessionFailCause::GraphicsLost
            }
        ]
    ));
}
