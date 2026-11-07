//! End-to-end contract tests for the Issue #156 message surface:
//! intent parsing, result/error events, job lifecycle
//! (progress/cancel/panic), bounded data transfer, asset path safety
//! and GUI/CLI parity (same service, same event shapes through both
//! entry points).

use std::sync::Arc;
use std::time::{Duration, Instant};

use ruviform_ui::{
    AssetSource, CallOutcome, DataRegistry, DataRequest, DemoService, ErrorBody, Event, HostEvent,
    JobSpec, JobState, MapAssetSource, OpInfo, Service, ServiceContext, UiRuntime, VecDataSource,
    codes, dispatch_script, parse_intent, resolve_path,
};
use serde_json::{Value, json};

fn rt() -> (UiRuntime, DataRegistry) {
    let registry = DataRegistry::new();
    let mut rt = UiRuntime::new(registry.clone());
    rt.register_service(DemoService::new())
        .expect("demo service registration");
    (rt, registry)
}

fn wait_terminal(rt: &mut UiRuntime, timeout: Duration) -> Vec<Event> {
    let deadline = Instant::now() + timeout;
    let mut all = Vec::new();
    while rt.has_running_jobs() && Instant::now() < deadline {
        all.extend(rt.poll(Some(Duration::from_millis(50))));
    }
    all.extend(rt.drain_jobs());
    all
}

#[test]
fn intent_parsing_rejects_bad_shapes() {
    assert!(parse_intent("not json").is_err());
    assert!(parse_intent(r#"{"v":2,"kind":"intent","id":"x","op":"a.b"}"#).is_err());
    assert!(parse_intent(r#"{"v":1,"kind":"other","id":"x","op":"a.b"}"#).is_err());
    assert!(parse_intent(r#"{"v":1,"kind":"intent","id":"x","op":"UPPER.Case"}"#).is_err());
    assert!(parse_intent(r#"{"v":1,"kind":"intent","id":"x","op":"nodots"}"#).is_err());
    assert!(
        parse_intent(&format!(
            "{{\"v\":1,\"kind\":\"intent\",\"id\":\"x\",\"op\":\"a.b\",\"params\":{}}}",
            "x".repeat(2 * 1024 * 1024)
        ))
        .is_err()
    );
    let ok = parse_intent(r#"{"v":1,"kind":"intent","id":"i-1","op":"sys.ping","params":{}}"#)
        .expect("valid intent");
    assert_eq!(ok.op, "sys.ping");
}

#[test]
fn malformed_message_yields_uncorrelated_error_event() {
    let (mut rt, _) = rt();
    let events = rt.handle(HostEvent::Message("garbage".to_string()));
    assert!(
        matches!(events.as_slice(), [Event::Error { error }] if error.code == codes::MALFORMED_MESSAGE)
    );
}

#[test]
fn unknown_op_returns_stable_error_code() {
    let (mut rt, _) = rt();
    let events = rt.dispatch_op("i-9", "no.such_op", json!({}));
    assert!(
        matches!(events.as_slice(), [Event::Result { id, ok: false, error: Some(e), .. }] if id == "i-9" && e.code == codes::UNKNOWN_OP)
    );
}

#[test]
fn sys_ops_answer_immediately() {
    let (mut rt, _) = rt();
    for ev in rt.dispatch_op("p", "sys.ping", json!({})) {
        if let Event::Result {
            ok: true,
            data: Some(d),
            ..
        } = ev
        {
            assert_eq!(d["pong"], true);
        } else {
            panic!("sys.ping did not return a result: {}", ev.to_json());
        }
    }
    for ev in rt.dispatch_op("c", "sys.capabilities", json!({})) {
        if let Event::Result {
            ok: true,
            data: Some(d),
            ..
        } = ev
        {
            assert_eq!(d["schema"], 1);
            let ops: Vec<&str> = d["ops"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|o| o["name"].as_str())
                .collect();
            for want in [
                "sys.ping",
                "sys.capabilities",
                "job.cancel",
                "demo.synthesize",
            ] {
                assert!(ops.contains(&want), "missing op {want}");
            }
        } else {
            panic!("sys.capabilities failed");
        }
    }
}

#[test]
fn invalid_params_is_rejected_not_panicked() {
    let (mut rt, _) = rt();
    for params in [
        json!({"seconds": 0}),
        json!({"seconds": 9999}),
        json!({"seconds": -1}),
        json!({"seconds": "abc"}),
        json!({"sample_rate": 1}),
    ] {
        let events = rt.dispatch_op("v", "demo.synthesize", params);
        assert!(
            matches!(events.as_slice(), [Event::Result { ok: false, error: Some(e), .. }] if e.code == codes::INVALID_PARAMS),
            "params should be rejected"
        );
    }
}

#[test]
fn job_progress_finished_and_data_source_registered() {
    let (mut rt, registry) = rt();
    let events = rt.dispatch_op(
        "j",
        "demo.synthesize",
        json!({ "seconds": 2, "sample_rate": 8000, "chunk_frames": 800 }),
    );
    let job = match &events[..] {
        [
            Event::Result {
                ok: true,
                data: Some(d),
                ..
            },
        ] => d["job"].as_u64().unwrap(),
        other => panic!("unexpected ack: {other:?}"),
    };
    let all = wait_terminal(&mut rt, Duration::from_secs(30));
    assert!(all.iter().any(|e| matches!(
        e,
        Event::Job {
            state: JobState::Progress,
            ..
        }
    )));
    let finished = all.iter().find_map(|e| match e {
        Event::Job {
            job: j,
            state: JobState::Finished,
            data: Some(d),
            ..
        } if *j == job => Some(d.clone()),
        _ => None,
    });
    let data = finished.expect("job did not finish");
    assert_eq!(data["source"], "demo/peaks");
    let bytes = data["bytes"].as_u64().unwrap();
    assert_eq!(bytes, data["points"].as_u64().unwrap() * 8);
    // Registered source is pullable within bounds.
    assert!(registry.names().contains(&"demo/peaks".to_string()));
    let chunk = registry
        .read(&DataRequest {
            name: "demo/peaks".into(),
            offset: 0,
            len: Some(8),
        })
        .unwrap();
    assert_eq!(chunk.bytes.len(), 8);
    assert_eq!(chunk.total, bytes);
}

/// Minimal service exercising both progress report shapes
/// (Issue #172): a job that ends on a detail-labelled report and a
/// job that only sends detail-less reports.
struct StageService;

impl Service for StageService {
    fn ops(&self) -> &'static [OpInfo] {
        &[
            OpInfo {
                name: "test.stages",
                summary: "Progress reports carrying a detail label",
            },
            OpInfo {
                name: "test.plain",
                summary: "Progress reports without a detail label",
            },
        ]
    }

    fn call(
        &mut self,
        op: &str,
        _params: &Value,
        _ctx: &mut ServiceContext<'_>,
    ) -> Result<CallOutcome, ErrorBody> {
        let spec = match op {
            "test.stages" => JobSpec::new("test.stages", |ctx| {
                ctx.progress(1, 2);
                // The last progress report carries the stage label —
                // latest-wins coalescing can never supersede it.
                ctx.progress_detail(2, 2, "peaks");
                Ok(json!({ "stages": 2 }))
            }),
            "test.plain" => JobSpec::new("test.plain", |ctx| {
                ctx.progress(1, 1);
                Ok(json!({ "plain": true }))
            }),
            _ => unreachable!("StageService registers only test.stages/test.plain"),
        };
        Ok(CallOutcome::Job(spec))
    }
}

#[test]
fn job_progress_detail_reaches_the_wire() {
    let (mut rt, _) = rt();
    rt.register_service(StageService)
        .expect("stage service registration");

    // The detail'd report is the job's last progress signal, so the
    // drain emits it either standalone or in the terminal batch —
    // deterministically observable.
    let events = rt.dispatch_op("j", "test.stages", json!({}));
    assert!(matches!(
        events.as_slice(),
        [Event::Result { ok: true, .. }]
    ));
    let all = wait_terminal(&mut rt, Duration::from_secs(10));
    let detailed = all
        .iter()
        .filter_map(|e| match e {
            Event::Job {
                state: JobState::Progress,
                detail,
                ..
            } => Some(detail.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        detailed.contains(&Some("peaks".to_string())),
        "detail label did not reach a job progress event: {detailed:?}"
    );
    // The detail'd progress event precedes the terminal event.
    let last_progress = all
        .iter()
        .rposition(|e| {
            matches!(
                e,
                Event::Job {
                    state: JobState::Progress,
                    ..
                }
            )
        })
        .expect("no progress event");
    let finished = all
        .iter()
        .position(|e| {
            matches!(
                e,
                Event::Job {
                    state: JobState::Finished,
                    ..
                }
            )
        })
        .expect("job did not finish");
    assert!(last_progress < finished);

    // Detail-less reports keep emitting progress events with no
    // detail — the pre-Issue-172 shape.
    rt.dispatch_op("j2", "test.plain", json!({}));
    let all = wait_terminal(&mut rt, Duration::from_secs(10));
    let plain = all.iter().find_map(|e| match e {
        Event::Job {
            state: JobState::Progress,
            detail,
            ..
        } => Some(detail.clone()),
        _ => None,
    });
    assert_eq!(plain, Some(None), "plain reports must not carry detail");

    // Wire shape: `detail` serializes when present and is absent
    // (not null) for detail-less reports — additive rvf.ipc.v1 field.
    let with: Value =
        serde_json::from_str(&Event::job_progress_detail(7, 2, 2, Some("peaks")).to_json())
            .unwrap();
    assert_eq!(with["kind"], "job");
    assert_eq!(with["state"], "progress");
    assert_eq!(with["detail"], "peaks");
    let without: Value = serde_json::from_str(&Event::job_progress(7, 2, 2).to_json()).unwrap();
    assert!(without.get("detail").is_none());
}

#[test]
fn job_cancel_reaches_terminal_state() {
    let (mut rt, _) = rt();
    let events = rt.dispatch_op("j", "demo.sleep", json!({ "ms": 60_000 }));
    let job = match &events[..] {
        [
            Event::Result {
                ok: true,
                data: Some(d),
                ..
            },
        ] => d["job"].as_u64().unwrap(),
        other => panic!("unexpected ack: {other:?}"),
    };
    let cancel_ack = rt.dispatch_op("c", "job.cancel", json!({ "job": job }));
    assert!(matches!(
        cancel_ack.as_slice(),
        [Event::Result { ok: true, .. }]
    ));
    let all = wait_terminal(&mut rt, Duration::from_secs(10));
    assert!(
        all.iter().any(|e| matches!(
            e,
            Event::Job {
                job: j,
                state: JobState::Cancelled,
                ..
            } if *j == job
        )),
        "job did not reach cancelled state"
    );
    // Cancelling a finished/unknown job reports unknown_job.
    let ev = rt.dispatch_op("c2", "job.cancel", json!({ "job": job }));
    assert!(matches!(
        ev.as_slice(),
        [Event::Result {
            ok: false,
            error: Some(e),
            ..
        }] if e.code == codes::UNKNOWN_JOB
    ));
}

#[test]
fn worker_panic_maps_to_worker_failed() {
    let (mut rt, _) = rt();
    rt.dispatch_op("p", "demo.panic", json!({}));
    let all = wait_terminal(&mut rt, Duration::from_secs(10));
    assert!(
        all.iter().any(|e| matches!(
            e,
            Event::Job {
                state: JobState::Failed,
                error: Some(err),
                ..
            } if err.code == codes::WORKER_FAILED
        )),
        "panic did not surface as worker_failed"
    );
}

#[test]
fn service_error_is_in_band() {
    let (mut rt, _) = rt();
    let ev = rt.dispatch_op("f", "demo.fail", json!({}));
    assert!(matches!(
        ev.as_slice(),
        [Event::Result {
            ok: false,
            error: Some(e),
            ..
        }] if e.code == codes::SERVICE_ERROR
    ));
}

#[test]
fn reserved_ops_cannot_be_registered() {
    struct Bad;
    impl Service for Bad {
        fn ops(&self) -> &'static [OpInfo] {
            &[OpInfo {
                name: "sys.evil",
                summary: "x",
            }]
        }
        fn call(
            &mut self,
            _op: &str,
            _params: &Value,
            _ctx: &mut ServiceContext<'_>,
        ) -> Result<CallOutcome, ErrorBody> {
            unreachable!()
        }
    }
    let (mut rt, _) = rt();
    assert!(rt.register_service(Bad).is_err());
}

#[test]
fn bounded_data_transfer_contract() {
    let registry = DataRegistry::with_max_chunk(16);
    registry.register("blob", Arc::new(VecDataSource::new(vec![7u8; 100])));

    // full range
    let c = registry
        .read(&DataRequest {
            name: "blob".into(),
            offset: 0,
            len: Some(100),
        })
        .unwrap();
    assert_eq!(c.bytes.len(), 16, "len capped at max_chunk");
    assert_eq!(c.total, 100);

    // tail partial
    let c = registry
        .read(&DataRequest {
            name: "blob".into(),
            offset: 90,
            len: None,
        })
        .unwrap();
    assert_eq!(c.bytes.len(), 10);

    // boundary
    assert!(
        registry
            .read(&DataRequest {
                name: "blob".into(),
                offset: 101,
                len: None,
            })
            .is_err()
    );
    let c = registry
        .read(&DataRequest {
            name: "blob".into(),
            offset: 100,
            len: None,
        })
        .unwrap();
    assert_eq!(c.bytes.len(), 0);

    assert!(matches!(
        registry.read(&DataRequest {
            name: "nope".into(),
            offset: 0,
            len: None,
        }),
        Err(ruviform_ui::DataError::NotFound { .. })
    ));
}

#[test]
fn asset_paths_are_sanitized() {
    assert_eq!(resolve_path("/").as_deref(), Some("index.html"));
    assert_eq!(resolve_path("").as_deref(), Some("index.html"));
    assert_eq!(resolve_path("/a/b.js").as_deref(), Some("a/b.js"));
    assert_eq!(resolve_path("a/./b.js").as_deref(), Some("a/b.js"));
    for bad in ["../x", "a/../../x", "a\\b", "c:/win", "a//b", "//x"] {
        assert_eq!(resolve_path(bad), None, "{bad} must be rejected");
    }
    let assets = MapAssetSource::new().with_asset("index.html", None, b"<h/>".as_slice());
    assert!(assets.get("index.html").is_some());
    assert!(assets.get("missing.js").is_none());
}

#[test]
fn event_serialization_and_push_script() {
    let ev = Event::result_ok("i-1", json!({"x": "a\"b</script>"}));
    let json = ev.to_json();
    let v: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["v"], 1);
    assert_eq!(v["kind"], "result");
    assert_eq!(v["id"], "i-1");
    assert_eq!(v["data"]["x"], "a\"b</script>");
    let script = dispatch_script(&ev);
    assert!(script.contains("__rvf.dispatch(JSON.parse("));
    // The embedded literal must decode back to the same JSON text.
    let lit = script
        .strip_prefix("window.__rvf && window.__rvf.dispatch(JSON.parse(")
        .unwrap()
        .strip_suffix("))")
        .unwrap();
    let inner: String = serde_json::from_str(lit).unwrap();
    assert_eq!(inner, json);
}

#[test]
fn gui_and_cli_paths_produce_identical_event_shapes() {
    // GUI path: intents arrive as raw HostEvent::Message bodies.
    // CLI path: dispatch_op. Both must produce the same event kinds —
    // the shared-boundary acceptance criterion.
    let gui_body = r#"{"v":1,"kind":"intent","id":"a","op":"demo.echo","params":{"k":1}}"#;
    let (mut gui_rt, _) = rt();
    let (mut cli_rt, _) = rt();
    let gui = gui_rt.handle(HostEvent::Message(gui_body.to_string()));
    let cli = cli_rt.dispatch_op("a", "demo.echo", json!({ "k": 1 }));
    let gui_json: Vec<String> = gui.iter().map(Event::to_json).collect();
    let cli_json: Vec<String> = cli.iter().map(Event::to_json).collect();
    assert_eq!(gui_json, cli_json);

    // Malformed GUI bodies never panic and yield the protocol error.
    let gui = gui_rt.handle(HostEvent::Message("{".to_string()));
    assert!(matches!(gui.as_slice(), [Event::Error { .. }]));

    // Tick drains jobs identically regardless of entry point.
    let body =
        "{\"v\":1,\"kind\":\"intent\",\"id\":\"j\",\"op\":\"demo.sleep\",\"params\":{\"ms\":50}}";
    gui_rt.handle(HostEvent::Message(body.to_string()));
    cli_rt.dispatch_op("j", "demo.sleep", json!({ "ms": 50 }));
    let deadline = Instant::now() + Duration::from_secs(10);
    let (mut g, mut c) = (Vec::new(), Vec::new());
    while (gui_rt.has_running_jobs() || cli_rt.has_running_jobs()) && Instant::now() < deadline {
        g.extend(gui_rt.handle(HostEvent::Tick));
        c.extend(cli_rt.poll(Some(Duration::from_millis(50))));
    }
    g.extend(gui_rt.drain_jobs());
    c.extend(cli_rt.drain_jobs());
    let kind_of = |e: &Event| match e {
        Event::Job { state, .. } => format!("job/{state:?}"),
        _ => "other".to_string(),
    };
    let gk: Vec<String> = g.iter().map(kind_of).collect();
    let ck: Vec<String> = c.iter().map(kind_of).collect();
    assert_eq!(gk.last().map(String::as_str), Some("job/Finished"));
    assert_eq!(ck.last().map(String::as_str), Some("job/Finished"));
}

#[test]
fn close_requested_cancels_jobs_and_emits_lifecycle() {
    let (mut rt, _) = rt();
    rt.dispatch_op("j", "demo.sleep", json!({ "ms": 60_000 }));
    let events = rt.handle(HostEvent::CloseRequested);
    assert!(matches!(events.as_slice(), [Event::Lifecycle { .. }]));
    // The job observes cancellation and terminates.
    let all = wait_terminal(&mut rt, Duration::from_secs(10));
    assert!(all.iter().any(|e| matches!(
        e,
        Event::Job {
            state: JobState::Cancelled,
            ..
        }
    )));
}

/// Issue #386 connection test: the typed `Action(Intent)` path —
/// `dispatch_window_intent` — produces the same dispatch as the
/// legacy `HostEvent::Message` raw-string path, with the window
/// recorded as routing owner.
#[test]
fn dispatch_window_intent_parity_with_message_path() {
    use ruviform_ui::{Intent, WindowId};

    let (mut rt_msg, _) = rt();
    let (mut rt_typed, _) = rt();
    let window = WindowId::new(7).expect("nonzero");
    let intent = parse_intent(r#"{"v":1,"kind":"intent","id":"i-1","op":"sys.ping","params":{}}"#)
        .expect("valid intent");

    // Legacy path: raw string through HostEvent::Message → parse →
    // dispatch_for.
    let legacy = rt_msg.handle_window(
        window,
        HostEvent::Message(
            r#"{"v":1,"kind":"intent","id":"i-1","op":"sys.ping","params":{}}"#.to_string(),
        ),
    );
    // Typed path: driver constructed the Intent directly.
    let typed = rt_typed.dispatch_window_intent(
        window,
        Intent::new(intent.id.clone(), intent.op.clone(), intent.params.clone()),
    );
    assert_eq!(legacy.len(), typed.len());
    for (l, t) in legacy.iter().zip(typed.iter()) {
        assert_eq!(l.window, Some(window));
        assert_eq!(t.window, Some(window));
        assert_eq!(l.event.to_json(), t.event.to_json());
    }
}

/// Issue #386: a malformed `op` on the typed path reports the same
/// stable `malformed_message` code the parse path produces — the
/// error surface must not depend on the entry point.
#[test]
fn dispatch_window_intent_malformed_op_matches_parse_path() {
    use ruviform_ui::{Intent, WindowId};

    let (mut rt_msg, _) = rt();
    let (mut rt_typed, _) = rt();
    let window = WindowId::new(7).expect("nonzero");

    let legacy = rt_msg.handle_window(
        window,
        HostEvent::Message(
            r#"{"v":1,"kind":"intent","id":"i-1","op":"Bad Op","params":{}}"#.to_string(),
        ),
    );
    let typed = rt_typed.dispatch_window_intent(window, Intent::new("i-1", "Bad Op", json!({})));
    assert_eq!(legacy.len(), 1);
    assert_eq!(typed.len(), 1);
    // Same stable error code on both paths; the shapes differ
    // legitimately — the parse path fails before the intent id is
    // known (kind "error"), the typed path can correlate (kind
    // "result" carrying the id).
    assert!(legacy[0].event.to_json().contains("malformed_message"));
    assert!(typed[0].event.to_json().contains("malformed_message"));
}
