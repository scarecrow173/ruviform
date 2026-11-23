//! E2E: `L1EditorOps` over the real `ruviform-plugin-l1-runner`
//! child, routed through the composite `PluginHost` — spawn →
//! `Hello` (the `GUI_FLOATING` advertisement) →
//! `Instantiate`(`dev.ruviform.mock.gui`) → `Initialize` → session
//! `open(Floating)` → the floating verb family → `close`, plus the
//! defined refusals (`Docked` attach, `reattach`, the request
//! channel, `SetGuiParent`) and `pump_notifications`'s
//! `InstanceFailed` → `Terminated{HostLost}` delivery.
//!
//! The runner claims `WIRE_CAPABILITY_GUI_FLOATING` only where its
//! child-owned UI-thread mechanism exists (Windows): the advertised
//! path is `#[cfg(windows)]` and the unadvertised arm pins the same
//! verbs answering the negotiated gate's defined `Unsupported`
//! elsewhere. `GuiPlugin` is window-independent — what these tests
//! pin is the routing + reply vocabulary end to end, not a real
//! HWND (the Windows lane owns that).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use ruviform_plugin_core::{
    GuiKeyDirection, GuiKeyEvent, GuiModifiers, GuiRequest, HostError, PluginDescriptor,
};
use ruviform_plugin_host::{
    AttachMode, EditorOpError, EditorSessionOps, EditorSize, HostCommand, HostReply, L1EditorOps,
    OutOfProcessSpec, ParentSlotLease, PluginEditorSessions, PluginHandle, PluginHost,
};

const SIZE: EditorSize = EditorSize {
    width: 320.0,
    height: 240.0,
};

/// The descriptor `ruviform-plugin-l1-runner`'s built-in `GuiPlugin`
/// reports — the `OutOfProcessSpec` oracle must field-match it.
const GUI_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.gui",
    name: "Gui",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["test-fixture"],
};

/// The `ruviform-plugin-l1-runner` executable — a
/// `ruviform-plugin-export` package bin, so `CARGO_BIN_EXE_*` cannot
/// name it here. `RUVIFORM_L1_RUNNER` wins verbatim; otherwise the
/// binary is located relative to this test binary (the
/// `l1_proxy_e2e` convention — each integration test crate needs
/// its own copy).
fn runner_program() -> OsString {
    if let Some(path) = std::env::var_os("RUVIFORM_L1_RUNNER") {
        return path;
    }
    let exe = std::env::current_exe().expect("current_exe");
    let file_name = format!("ruviform-plugin-l1-runner{}", std::env::consts::EXE_SUFFIX);
    let candidates: Vec<PathBuf> = exe
        .parent()
        .into_iter()
        .chain(exe.parent().and_then(Path::parent))
        .map(|dir| dir.join(&file_name))
        .collect();
    for path in &candidates {
        if path.is_file() {
            return path.clone().into_os_string();
        }
    }
    panic!(
        "ruviform-plugin-l1-runner missing — searched {}; run \
         `mise run test` (workspace test builds all member bins) \
         or set RUVIFORM_L1_RUNNER",
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// A `PluginHost` with one `OutOfProcess` `dev.ruviform.mock.gui`
/// instance, spawned and `Initialize`d on the real runner child.
fn gui_host() -> (PluginHost, PluginHandle) {
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(GUI_DESCRIPTOR, runner_program()))
        .unwrap();
    let handle = match host
        .execute(HostCommand::Instantiate {
            plugin_id: GUI_DESCRIPTOR.id.to_owned(),
        })
        .unwrap()
    {
        HostReply::Instantiated { handle } => handle,
        other => panic!("unexpected reply: {other:?}"),
    };
    host.execute(HostCommand::Initialize { handle }).unwrap();
    (host, handle)
}

fn lease() -> ParentSlotLease {
    ParentSlotLease::new(0xFEED)
}

fn key() -> GuiKeyEvent {
    GuiKeyEvent {
        direction: GuiKeyDirection::Down,
        key: 'a',
        key_code: 0x41,
        modifiers: GuiModifiers::NONE,
    }
}

/// The request channel has no wire verbs on any platform — the
/// ops' defined refusals hold even where `GUI_FLOATING` is
/// advertised.
#[test]
fn l1_editor_ops_keeps_the_request_channel_refused() {
    let (mut host, instance) = gui_host();
    let session = PluginEditorSessions::new().begin_session(instance).unwrap();
    let mut ops = L1EditorOps::new(&mut host, instance);
    let mut drained = Vec::new();
    assert_eq!(
        ops.drain_gui_requests(session, &mut drained),
        Err(EditorOpError::Unsupported)
    );
    assert_eq!(drained.len(), 0);
    assert_eq!(
        ops.adjudicate_gui_request(session, GuiRequest::Show),
        Err(EditorOpError::Unsupported)
    );
    assert_eq!(
        ops.reattach(session, &lease()),
        Err(EditorOpError::Unsupported)
    );
}

/// Off-Windows the runner advertises `0` — the negotiated gate
/// holds the defined `Unsupported` on the whole family and the ops
/// layer reports it verbatim.
#[cfg(not(windows))]
#[test]
fn l1_editor_ops_holds_the_gate_where_the_mechanism_is_absent() {
    let (mut host, instance) = gui_host();
    let mut sessions = PluginEditorSessions::new();
    let session = sessions.begin_session(instance).unwrap();
    {
        let mut ops = L1EditorOps::new(&mut host, instance);

        assert_eq!(ops.capabilities(session), Err(EditorOpError::Unsupported));
        for mode in [AttachMode::Docked, AttachMode::Floating] {
            assert_eq!(
                ops.open(session, &lease(), SIZE, mode),
                Err(EditorOpError::Unsupported)
            );
        }
        assert_eq!(ops.close(session), Err(EditorOpError::Unsupported));
        assert_eq!(ops.set_size(session, SIZE), Err(EditorOpError::Unsupported));
        assert_eq!(
            ops.set_visibility(session, true),
            Err(EditorOpError::Unsupported)
        );
        assert_eq!(
            ops.set_focus(session, true),
            Err(EditorOpError::Unsupported)
        );
        assert_eq!(
            ops.send_key(session, key()),
            Err(EditorOpError::Unsupported)
        );
    }
    // The gate refused before the wire — the child still answers.
    assert_eq!(
        host.execute(HostCommand::SetGuiVisibility {
            handle: instance,
            visible: true,
        }),
        Err(HostError::Unsupported)
    );
}

#[cfg(windows)]
mod windows {
    use std::thread;
    use std::time::{Duration, Instant};

    use ruviform_plugin_core::ParentLease;
    use ruviform_plugin_host::{
        EditorSessionEvent, EditorSessionState, HostNotification, SessionFailCause, Support,
        pump_notifications,
    };

    use super::*;

    const NOTIFY_TIMEOUT: Duration = Duration::from_secs(10);

    /// The advertised path, end to end: the runner's `Hello` claims
    /// `GUI_FLOATING`, so the negotiated gate admits every floating
    /// verb and `OpenGui`'s `Opened.capabilities` (`0x82` — `float`
    /// + `close`) becomes the session's `EditorCapabilities`.
    #[test]
    fn l1_editor_ops_drives_the_floating_session_over_the_wire() {
        let (mut host, instance) = gui_host();
        let mut sessions = PluginEditorSessions::new();
        let session = sessions.begin_session(instance).unwrap();
        let mut ops = L1EditorOps::new(&mut host, instance);

        // Pre-open: the negotiated advertisement claims the
        // mechanism — `float` + `close` — and nothing else.
        let caps = ops.capabilities(session).unwrap();
        assert_eq!(caps.float, Support::Supported);
        assert_eq!(caps.close, Support::Supported);
        assert_eq!(caps.dock, Support::Unsupported);
        assert_eq!(caps.resize, Support::Unsupported);

        // `open` rides `OpenGui{Floating}` to the child's UI thread;
        // the `Opened.capabilities` reply supersedes the
        // advertisement.
        let caps = ops
            .open(session, &lease(), SIZE, AttachMode::Floating)
            .unwrap();
        assert_eq!(caps.float, Support::Supported);
        assert_eq!(caps.close, Support::Supported);
        assert_eq!(caps.dock, Support::Unsupported);
        assert_eq!(ops.capabilities(session).unwrap(), caps);

        // The rest of the floating family round-trips to `Ok` while
        // the surface is live — each verb passed the negotiated
        // gate and the runner's surface bookkeeping.
        ops.set_size(session, SIZE).unwrap();
        ops.set_visibility(session, false).unwrap();
        ops.set_visibility(session, true).unwrap();
        ops.set_focus(session, true).unwrap();
        ops.send_key(session, key()).unwrap();
        ops.close(session).unwrap();

        // The runner-side surface vocabulary reaches the session
        // layer: every verb but `open` against a closed surface is
        // the defined refusal (`InvalidValue` — the settled
        // no-surface answer, generalized to the family).
        for result in [
            ops.set_size(session, SIZE),
            ops.set_visibility(session, true),
            ops.set_focus(session, true),
            ops.send_key(session, key()),
            ops.close(session),
        ] {
            assert_eq!(result, Err(EditorOpError::InvalidValue));
        }
    }

    /// `SetGuiParent` has no wire verb — the composite refuses it
    /// `Unsupported` on an `OutOfProcess` slot without touching the
    /// child, which keeps answering afterwards.
    #[test]
    fn composite_set_gui_parent_refuses_on_l1_without_wire_traffic() {
        let (mut host, instance) = gui_host();
        assert_eq!(
            host.execute(HostCommand::SetGuiParent {
                handle: instance,
                parent: ParentLease::new(0, 0),
            }),
            Err(HostError::Unsupported)
        );
        // Not forwarded — the child still serves the GUI family
        // (its surface-less `InvalidValue` is a live reply, not a
        // gate hit).
        assert_eq!(
            host.execute(HostCommand::SetGuiVisibility {
                handle: instance,
                visible: true,
            }),
            Err(HostError::InvalidValue)
        );
    }

    /// `pump_notifications` bridges the composite's `InstanceFailed`
    /// fan-out onto the registry's `Terminated{HostLost}` — open a
    /// floating session, kill the child, poll the pump, and the
    /// bound session converges exactly once (a second pump delivers
    /// nothing).
    #[test]
    fn pump_notifications_converges_the_session_on_child_death() {
        let (mut host, instance) = gui_host();
        let mut sessions = PluginEditorSessions::new();
        let session = sessions.begin_session(instance).unwrap();
        // A live (`Attached`) session — `Terminated` only fires on a
        // session that was open; a never-opened `Closed` one just
        // concludes silently.
        let request = sessions
            .session_mut(session)
            .unwrap()
            .begin_open(AttachMode::Floating)
            .unwrap();
        let result = {
            let mut ops = L1EditorOps::new(&mut host, instance);
            ops.open(session, &lease(), SIZE, AttachMode::Floating)
                .map(|_| ())
        };
        sessions
            .session_mut(session)
            .unwrap()
            .complete_attach(request, result)
            .unwrap();

        host.kill_child(instance).unwrap();
        let deadline = Instant::now() + NOTIFY_TIMEOUT;
        let delivered = loop {
            let delivered = pump_notifications(&mut host, &mut sessions);
            if delivered > 0 {
                break delivered;
            }
            assert!(
                Instant::now() < deadline,
                "InstanceFailed notification never arrived"
            );
            thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(delivered, 1);
        let session = sessions.session_mut(session).unwrap();
        assert_eq!(session.state(), EditorSessionState::Closed);
        assert!(
            session
                .drain_events()
                .contains(&EditorSessionEvent::Terminated {
                    cause: SessionFailCause::HostLost
                })
        );
        // Exactly-once: the notification was consumed by the first
        // pump.
        assert_eq!(pump_notifications(&mut host, &mut sessions), 0);
    }

    /// The notification itself carries the public handle — the
    /// pump's `handle → session` correlation is what routes
    /// `HostLost` to the right session (and only that one).
    #[test]
    fn pump_notifications_reports_the_failed_handle() {
        let (mut host, instance) = gui_host();
        host.kill_child(instance).unwrap();
        let deadline = Instant::now() + NOTIFY_TIMEOUT;
        loop {
            if let Some(HostNotification::InstanceFailed { handle, .. }) =
                host.try_recv_notification()
            {
                assert_eq!(handle, instance);
                break;
            }
            assert!(
                Instant::now() < deadline,
                "InstanceFailed notification never arrived"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }
}
