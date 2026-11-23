//! Contract tests for the plugin→host GUI request channel (Issue
//! #873): `GuiRequest` delivery through `InProcessHost` +
//! `InProcessEditorOps` — the synchronous sink path (the VST3
//! `resizeView` shape) and the queued drain→adjudicate path (the CLAP
//! `request_*` shape). A scripted `Plugin` plus a scripted arbiter
//! drive the real L0 machinery headless; these are contract-level
//! results — **not** a real Dock PASS (no GPUI window, OS parent or
//! real plugin). The L1 adapter stays defined-`Unsupported` on the
//! whole channel.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, GuiHandle, GuiRequest, GuiRequestRefusal, GuiRequestSink,
    GuiRequestVerdict, GuiSize, ParamInfo, ParentLease, PluginDescriptor, PrepareError,
    ProcessConfig, ProcessContext, ProcessResult, ProcessStatus, TransportInfo,
};
use ruviform_plugin_host::{
    AttachMode, EditorOpError, EditorSessionOps, EditorSessionState, GuiRequestArbiter,
    GuiRequestArbiterHandle, HostCommand, HostError, HostReply, InProcessEditorOps, InProcessHost,
    L1EditorOps, OutOfProcessSpec, ParentSlotLease, Plugin, PluginEditorSessionId,
    PluginEditorSessions, PluginFactory, PluginHandle, PluginHost, erase,
};

// ---- shared fixtures -------------------------------------------------

const SIZE: ruviform_plugin_host::EditorSize = ruviform_plugin_host::EditorSize {
    width: 800.0,
    height: 600.0,
};

fn lease() -> ParentSlotLease {
    ParentSlotLease::new(0xA5A5_0001_0000_0000)
}

/// One plugin-side GUI verb call recorded by [`RequestPlugin`].
#[derive(Debug, PartialEq)]
enum Verb {
    /// `set_gui_size` — the logical size the plugin received.
    Size(GuiSize),
    /// `set_gui_visibility` — the show/hide flag.
    Visibility(bool),
    /// `set_gui_request_sink` — `true` on install, `false` on detach.
    Sink(bool),
}

/// A `Plugin` whose GUI verbs record into `verbs`, whose request sink
/// is captured into `sink` for the test to drive (the VST3
/// `resizeView` shape), and whose `pending` queue feeds
/// `drain_gui_requests` (the CLAP `request_*` shape).
struct RequestPlugin {
    gui_supported: bool,
    verbs: Rc<RefCell<Vec<Verb>>>,
    sink: Rc<RefCell<Option<Box<GuiRequestSink>>>>,
    pending: Rc<RefCell<VecDeque<GuiRequest>>>,
}

impl ruviform_plugin_core::PluginProcessor for RequestPlugin {
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

impl Plugin for RequestPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        DESCRIPTOR
    }

    fn params(&self) -> &[ParamInfo] {
        &[]
    }

    fn open_gui(&mut self, _parent: ParentLease) -> Result<GuiHandle, HostError> {
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

    fn set_gui_size(&mut self, size: GuiSize) -> Result<(), HostError> {
        self.verbs.borrow_mut().push(Verb::Size(size));
        Ok(())
    }

    fn set_gui_visibility(&mut self, visible: bool) -> Result<(), HostError> {
        self.verbs.borrow_mut().push(Verb::Visibility(visible));
        Ok(())
    }

    fn set_gui_request_sink(&mut self, sink: Option<Box<GuiRequestSink>>) {
        self.verbs.borrow_mut().push(Verb::Sink(sink.is_some()));
        *self.sink.borrow_mut() = sink;
    }

    fn drain_gui_requests(&mut self, out: &mut Vec<GuiRequest>) {
        out.extend(self.pending.borrow_mut().drain(..));
    }
}

struct RequestFactory {
    gui_supported: bool,
    verbs: Rc<RefCell<Vec<Verb>>>,
    sink: Rc<RefCell<Option<Box<GuiRequestSink>>>>,
    pending: Rc<RefCell<VecDeque<GuiRequest>>>,
}

impl PluginFactory for RequestFactory {
    fn descriptor(&self) -> PluginDescriptor {
        DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ruviform_plugin_host::ErasedPlugin> {
        erase(RequestPlugin {
            gui_supported: self.gui_supported,
            verbs: Rc::clone(&self.verbs),
            sink: Rc::clone(&self.sink),
            pending: Rc::clone(&self.pending),
        })
    }
}

const PLUGIN_ID: &str = "org.ruviform.test.gui-request";

const DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: PLUGIN_ID,
    name: "Request Plugin",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// One wired-up fixture: the host plus the observable plugin state.
struct Fixture {
    host: InProcessHost,
    verbs: Rc<RefCell<Vec<Verb>>>,
    sink: Rc<RefCell<Option<Box<GuiRequestSink>>>>,
    pending: Rc<RefCell<VecDeque<GuiRequest>>>,
}

fn fixture() -> Fixture {
    let verbs = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::new(RefCell::new(None));
    let pending = Rc::new(RefCell::new(VecDeque::new()));
    let mut host = InProcessHost::new();
    host.register_factory(Box::new(RequestFactory {
        gui_supported: true,
        verbs: Rc::clone(&verbs),
        sink: Rc::clone(&sink),
        pending: Rc::clone(&pending),
    }));
    Fixture {
        host,
        verbs,
        sink,
        pending,
    }
}

fn instantiate(host: &mut InProcessHost) -> PluginHandle {
    match host
        .execute(HostCommand::Instantiate {
            plugin_id: PLUGIN_ID.to_owned(),
        })
        .unwrap()
    {
        HostReply::Instantiated { handle } => handle,
        _ => panic!("unexpected reply"),
    }
}

fn token_redeemer() -> Box<ruviform_plugin_host::LeaseRedeemer<'static>> {
    Box::new(|lease: &ParentSlotLease| Ok(ParentLease::new(0, lease.raw())))
}

/// The `(session, request)` sequence a scripted arbiter observed.
type RequestLog = Arc<Mutex<Vec<(PluginEditorSessionId, GuiRequest)>>>;

/// A scripted arbiter: records every `(session, request)` pair and
/// answers the scripted verdict.
fn recording_arbiter(verdict: GuiRequestVerdict) -> (GuiRequestArbiterHandle, RequestLog) {
    let seen: RequestLog = Arc::new(Mutex::new(Vec::new()));
    let seen_requests = Arc::clone(&seen);
    let arbiter: Box<GuiRequestArbiter> = Box::new(move |session, request| {
        seen_requests.lock().unwrap().push((session, request));
        verdict
    });
    (Arc::new(Mutex::new(arbiter)), seen)
}

const ACCEPTED: GuiRequestVerdict = GuiRequestVerdict::Accepted { granted: None };

fn resize(width: f64, height: f64) -> GuiRequest {
    GuiRequest::Resize {
        size: GuiSize { width, height },
    }
}

// ---- synchronous sink path (the VST3 `resizeView` shape) --------------

#[test]
fn l0_open_installs_sink_and_delivers_resize_to_arbiter() {
    let mut fixture = fixture();
    let instance = instantiate(&mut fixture.host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    let (arbiter, seen) = recording_arbiter(ACCEPTED);
    {
        let mut ops = InProcessEditorOps::with_redeemer_and_arbiter(
            &mut fixture.host,
            instance,
            token_redeemer(),
            arbiter,
        );
        session
            .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
            .expect("open via InProcessHost");
        // The install landed through the plugin hook — the captured
        // sink is live.
        assert_eq!(fixture.verbs.borrow().as_slice(), &[Verb::Sink(true)]);
        // The VST3 `resizeView` shape: the plugin drives the sink and
        // gets the arbiter's verdict back synchronously.
        let verdict = {
            let mut sink = fixture.sink.borrow_mut();
            sink.as_mut().expect("installed sink")(resize(640.0, 480.0))
        };
        assert_eq!(verdict, ACCEPTED);
        // Arbiter saw the session id + the verbatim request.
        assert_eq!(
            seen.lock().unwrap().as_slice(),
            &[(id, resize(640.0, 480.0))]
        );
        // The session is still Attached — a request never moves state.
        assert_eq!(
            session.state(),
            EditorSessionState::Attached {
                mode: AttachMode::Docked
            }
        );
    }
    // `open` ⇒ route live: detach on close is the symmetric half.
}

#[test]
fn l0_sink_refusal_maps_arbiter_verdict() {
    let mut fixture = fixture();
    let instance = instantiate(&mut fixture.host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    let refused = GuiRequestVerdict::Refused {
        reason: GuiRequestRefusal::Rejected,
    };
    let (arbiter, _seen) = recording_arbiter(refused);
    let mut ops = InProcessEditorOps::with_redeemer_and_arbiter(
        &mut fixture.host,
        instance,
        token_redeemer(),
        arbiter.clone(),
    );
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .expect("open via InProcessHost");
    let verdict = {
        let mut sink = fixture.sink.borrow_mut();
        sink.as_mut().expect("installed sink")(resize(4096.0, 4096.0))
    };
    // The refusal is delivered verbatim — the plugin reads its
    // format's defined decline (`kResultFalse`), never a fake accept.
    assert_eq!(verdict, refused);
    // A busy arbiter is the defined `Busy` refusal, not a block: hold
    // the lock and the in-flight request cannot re-enter it.
    let _guard = arbiter.lock().unwrap();
    let verdict = {
        let mut sink = fixture.sink.borrow_mut();
        sink.as_mut().expect("installed sink")(resize(1.0, 1.0))
    };
    assert_eq!(
        verdict,
        GuiRequestVerdict::Refused {
            reason: GuiRequestRefusal::Busy
        }
    );
}

#[test]
fn l0_open_without_arbiter_installs_no_sink() {
    let mut fixture = fixture();
    let instance = instantiate(&mut fixture.host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    {
        let mut ops =
            InProcessEditorOps::with_redeemer(&mut fixture.host, instance, token_redeemer());
        session
            .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
            .expect("open via InProcessHost");
    }
    // No arbiter ⇒ no route: `set_gui_request_sink` was never called
    // and `adjudicate_gui_request` answers `Unrouted`.
    assert!(fixture.verbs.borrow().is_empty());
    assert!(fixture.sink.borrow().is_none());
    let mut ops = InProcessEditorOps::with_redeemer(&mut fixture.host, instance, token_redeemer());
    assert_eq!(
        ops.adjudicate_gui_request(id, GuiRequest::Show).unwrap(),
        GuiRequestVerdict::Refused {
            reason: GuiRequestRefusal::Unrouted
        }
    );
}

#[test]
fn l0_close_detaches_the_request_route() {
    let mut fixture = fixture();
    let instance = instantiate(&mut fixture.host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    let (arbiter, _seen) = recording_arbiter(ACCEPTED);
    let mut ops = InProcessEditorOps::with_redeemer_and_arbiter(
        &mut fixture.host,
        instance,
        token_redeemer(),
        arbiter,
    );
    session
        .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
        .expect("open via InProcessHost");
    session.close(&mut ops).expect("close via InProcessHost");
    // Detach was delivered — the plugin's stored sink is gone, so a
    // stale frame meets the defined refusal, never a dead arbiter.
    assert_eq!(
        fixture.verbs.borrow().as_slice(),
        &[Verb::Sink(true), Verb::Sink(false)]
    );
    assert!(fixture.sink.borrow().is_none());
}

// ---- queued drain→adjudicate path (the CLAP `request_*` shape) --------

#[test]
fn l0_dispatch_drains_queue_and_adjudicates_in_order() {
    let mut fixture = fixture();
    let instance = instantiate(&mut fixture.host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    // CLAP `request_*` enqueue ack shape: the plugin queued three
    // requests from its own thread side.
    fixture.pending.borrow_mut().extend([
        resize(1024.0, 768.0),
        GuiRequest::Show,
        GuiRequest::Closed {
            was_destroyed: true,
        },
    ]);
    let (arbiter, seen) = recording_arbiter(ACCEPTED);
    let mut ops = InProcessEditorOps::with_redeemer_and_arbiter(
        &mut fixture.host,
        instance,
        token_redeemer(),
        arbiter,
    );
    let mut dispatched = Vec::new();
    ops.dispatch_gui_requests(id, &mut dispatched).unwrap();
    // Every request reached the arbiter in queue order, and each
    // verdict is recorded next to its request.
    assert_eq!(
        dispatched,
        [
            (resize(1024.0, 768.0), ACCEPTED),
            (GuiRequest::Show, ACCEPTED),
            (
                GuiRequest::Closed {
                    was_destroyed: true
                },
                ACCEPTED
            ),
        ]
    );
    assert_eq!(seen.lock().unwrap().len(), 3);
    // A drained queue stays drained.
    assert!(fixture.pending.borrow().is_empty());
    let mut dispatched = Vec::new();
    ops.dispatch_gui_requests(id, &mut dispatched).unwrap();
    assert_eq!(dispatched, Vec::new());
    // State non-destructive — session never moved.
    assert_eq!(session.state(), EditorSessionState::Closed);
}

#[test]
fn l0_accepted_resize_roundtrips_through_session_reply() {
    let mut fixture = fixture();
    let instance = instantiate(&mut fixture.host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    {
        let mut ops =
            InProcessEditorOps::with_redeemer(&mut fixture.host, instance, token_redeemer());
        session
            .open(&mut ops, &lease(), SIZE, AttachMode::Docked)
            .expect("open via InProcessHost");
    }
    fixture
        .pending
        .borrow_mut()
        .push_back(resize(1024.0, 768.0));
    // The arbiter accepts with a corrected size — the container's
    // clamp is `granted`.
    let granted = GuiSize {
        width: 960.0,
        height: 720.0,
    };
    let (arbiter, _seen) = recording_arbiter(GuiRequestVerdict::Accepted {
        granted: Some(granted),
    });
    let mut ops = InProcessEditorOps::with_redeemer_and_arbiter(
        &mut fixture.host,
        instance,
        token_redeemer(),
        arbiter,
    );
    let mut dispatched = Vec::new();
    ops.dispatch_gui_requests(id, &mut dispatched).unwrap();
    assert_eq!(
        dispatched,
        [(
            resize(1024.0, 768.0),
            GuiRequestVerdict::Accepted {
                granted: Some(granted)
            }
        )]
    );
    // The driver-side reply composition: the arbiter confirmed the
    // container effect, and `session.set_size` notifies the plugin —
    // the full request→verdict→reply roundtrip.
    session
        .set_size(
            &mut ops,
            ruviform_plugin_host::EditorSize {
                width: granted.width,
                height: granted.height,
            },
        )
        .expect("reply set_size");
    assert_eq!(fixture.verbs.borrow().as_slice(), &[Verb::Size(granted)]);
    assert_eq!(
        session.state(),
        EditorSessionState::Attached {
            mode: AttachMode::Docked
        }
    );
}

#[test]
fn l0_refused_request_notifies_nothing() {
    let mut fixture = fixture();
    let instance = instantiate(&mut fixture.host);
    let mut sessions = PluginEditorSessions::new();
    let id = sessions.begin_session(instance).unwrap();
    let session = sessions.session_mut(id).unwrap();
    fixture
        .pending
        .borrow_mut()
        .push_back(resize(8192.0, 8192.0));
    let refused = GuiRequestVerdict::Refused {
        reason: GuiRequestRefusal::Rejected,
    };
    let (arbiter, _seen) = recording_arbiter(refused);
    let mut ops = InProcessEditorOps::with_redeemer_and_arbiter(
        &mut fixture.host,
        instance,
        token_redeemer(),
        arbiter,
    );
    let mut dispatched = Vec::new();
    ops.dispatch_gui_requests(id, &mut dispatched).unwrap();
    assert_eq!(dispatched, [(resize(8192.0, 8192.0), refused)]);
    // The refusal delivered the verdict only — no plugin-side verb
    // fired (nothing faked an effect).
    assert!(fixture.verbs.borrow().is_empty());
    // The session never moved — a refused request is not a failure.
    assert_eq!(session.state(), EditorSessionState::Closed);
}

// ---- L1 defined refusal ----------------------------------------------

/// The descriptor `l1-test-worker` reports for `Scan` — it backs the
/// `OutOfProcess` slot the L1 ops drives here.
const L1_WORKER_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.l1-test-worker",
    name: "L1 Test Worker",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// A `PluginHost` with one `OutOfProcess` instance on
/// `l1-test-worker`.
fn l1_host() -> (PluginHost, PluginHandle) {
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(
        L1_WORKER_DESCRIPTOR,
        std::ffi::OsString::from(env!("CARGO_BIN_EXE_l1-test-worker")),
    ))
    .unwrap();
    match host
        .execute(HostCommand::Instantiate {
            plugin_id: L1_WORKER_DESCRIPTOR.id.to_owned(),
        })
        .unwrap()
    {
        HostReply::Instantiated { handle } => (host, handle),
        other => panic!("unexpected reply: {other:?}"),
    }
}

/// The request channel has no wire verbs — every L1 request op
/// stays the defined `Unsupported` (a refused drain propagates
/// through the `dispatch_gui_requests` default).
#[test]
fn l1_ops_refuse_the_whole_request_channel() {
    let (mut host, instance) = l1_host();
    let mut ops = L1EditorOps::new(&mut host, instance);
    let session = PluginEditorSessionId::new(0, 0);
    let mut drained = Vec::new();
    assert_eq!(
        ops.drain_gui_requests(session, &mut drained),
        Err(EditorOpError::Unsupported)
    );
    assert_eq!(
        ops.adjudicate_gui_request(session, GuiRequest::Show),
        Err(EditorOpError::Unsupported)
    );
    let mut dispatched = Vec::new();
    assert_eq!(
        ops.dispatch_gui_requests(session, &mut dispatched),
        Err(EditorOpError::Unsupported)
    );
}
