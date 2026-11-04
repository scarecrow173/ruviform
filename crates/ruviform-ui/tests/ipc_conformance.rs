//! Issue #311 — fixture-driven `rvf.ipc.v1` / `rvf://data` / host
//! scenario conformance.
//!
//! The versioned corpus under `ui/guitest/fixtures` is the single
//! source of truth shared with the TypeScript lane (`ui/guitest`):
//! identical accept/reject shapes, identical stable error codes,
//! identical scenario semantics. The `rvf://data` URL/param surface
//! additionally runs in `ruviform-ui-webview::scheme`'s fixture lane.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ruviform_ui::{
    CallOutcome, DataRegistry, DataRequest, ErrorBody, Event, HostEvent, JobSpec, OpInfo,
    RoutedEvent, Service, ServiceContext, UiRuntime, VecDataSource, WindowId, codes, parse_intent,
};
use serde_json::{Value, json};

fn fixture(rel: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ui/guitest/fixtures")
        .join(rel);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read fixture {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("fixture {} is not JSON: {e}", path.display()))
}

fn utf8_len(s: &str) -> usize {
    s.len()
}

/// `bodyGen: {"kind":"pad-intent","padTo":N}` → a valid intent padded
/// to exactly N bytes via `params.pad`.
fn pad_intent(pad_to: usize) -> String {
    let head = r#"{"v":1,"kind":"intent","id":"i-pad","op":"sys.ping","params":{"pad":""#;
    let tail = r#""}}"#;
    let n = pad_to - utf8_len(head) - utf8_len(tail);
    format!("{head}{}{tail}", "x".repeat(n))
}

// ── intents ──────────────────────────────────────────────────────────

#[test]
fn ipc_intents_fixture_cases() {
    let doc = fixture("ipc/intents.v1.json");
    for case in doc["cases"].as_array().expect("cases") {
        let id = case["id"].as_str().unwrap();
        let body = match case["bodyGen"]["kind"].as_str() {
            Some("pad-intent") => {
                pad_intent(usize::try_from(case["bodyGen"]["padTo"].as_u64().unwrap()).unwrap())
            }
            _ => case["body"].as_str().expect("case body").to_string(),
        };
        let expect = &case["expect"];
        let r = parse_intent(&body);
        assert_eq!(
            r.is_ok(),
            expect["accept"].as_bool().unwrap(),
            "{id}: accept mismatch"
        );
        if expect["accept"].as_bool().unwrap() {
            let it = r.unwrap();
            if let Some(v) = expect["id"].as_str() {
                assert_eq!(it.id, v, "{id}: id");
            }
            if let Some(v) = expect["op"].as_str() {
                assert_eq!(it.op, v, "{id}: op");
            }
            if expect.get("params").is_some() {
                assert_eq!(it.params, expect["params"], "{id}: params");
            }
        } else {
            let e = r.unwrap_err();
            assert_eq!(e.code, expect["code"].as_str().unwrap(), "{id}: code");
        }
    }
}

// ── events ───────────────────────────────────────────────────────────
//
// The Rust side *produces* events (`Event` is Serialize-only +
// non-exhaustive): conformance here means the public constructors
// emit exactly the pinned wire shapes. Rejects are the decoder lane's
// job (TS `decodeEvent`); `notify` is a reserved wire shape with no
// public constructor — its meaning is pinned by the TS decode lane.

fn event_from_wire(wire: &Value) -> Option<Event> {
    Some(match wire["kind"].as_str().unwrap() {
        "result" => {
            if wire["ok"].as_bool().unwrap() {
                Event::result_ok(
                    wire["id"].as_str().unwrap(),
                    wire.get("data").cloned().unwrap_or(Value::Null),
                )
            } else {
                let e = &wire["error"];
                let mut body =
                    ErrorBody::new(e["code"].as_str().unwrap(), e["message"].as_str().unwrap());
                body.details = e.get("details").cloned();
                Event::result_err(wire["id"].as_str().unwrap(), body)
            }
        }
        "job" => {
            let job = wire["job"].as_u64().unwrap();
            match wire["state"].as_str().unwrap() {
                "progress" => Event::job_progress_detail(
                    job,
                    wire["done"].as_u64().unwrap(),
                    wire["total"].as_u64().unwrap(),
                    wire.get("detail").and_then(Value::as_str),
                ),
                "finished" => {
                    Event::job_finished(job, wire.get("data").cloned().unwrap_or(Value::Null))
                }
                "cancelled" => Event::job_cancelled(job),
                "failed" => {
                    let e = &wire["error"];
                    Event::job_failed(
                        job,
                        ErrorBody::new(e["code"].as_str().unwrap(), e["message"].as_str().unwrap()),
                    )
                }
                _ => return None,
            }
        }
        "lifecycle" => Event::lifecycle(match wire["state"].as_str().unwrap() {
            "closing" => ruviform_ui::LifecycleState::Closing,
            _ => return None,
        }),
        "error" => {
            let e = &wire["error"];
            Event::error(ErrorBody::new(
                e["code"].as_str().unwrap(),
                e["message"].as_str().unwrap(),
            ))
        }
        _ => return None, // `notify` — no public constructor (see above)
    })
}

#[test]
fn ipc_events_fixture_cases() {
    let doc = fixture("ipc/events.v1.json");
    for case in doc["cases"].as_array().expect("cases") {
        let id = case["id"].as_str().unwrap();
        let expect = &case["expect"];
        if !expect["accept"].as_bool().unwrap() {
            continue;
        }
        let Some(ev) = event_from_wire(&case["wire"]) else {
            continue;
        };
        let produced: Value = serde_json::from_str(&ev.to_json()).unwrap();
        assert_eq!(produced["v"], 1, "{id}: envelope version");
        assert_eq!(produced["kind"], expect["class"], "{id}: kind");
        if let Some(c) = expect.get("correlation") {
            let corr = match expect["class"].as_str().unwrap() {
                "result" => &produced["id"],
                "job" => &produced["job"],
                "notify" => &produced["seq"],
                "lifecycle" => &produced["state"],
                _ => &Value::Null,
            };
            assert_eq!(corr, c, "{id}: correlation");
        }
        if let Some(ok) = expect["ok"].as_bool() {
            assert_eq!(produced["ok"], ok, "{id}: ok");
        }
        if let Some(s) = expect["state"].as_str() {
            assert_eq!(produced["state"], s, "{id}: state");
        }
        if let Some(code) = expect["code"].as_str() {
            assert_eq!(produced["error"]["code"], code, "{id}: error code");
        }
    }
}

// ── rvf://data (registry range contract) ─────────────────────────────

fn data_case_registry(reg: &Value) -> DataRegistry {
    let registry = DataRegistry::with_max_chunk(reg["maxChunk"].as_u64().unwrap());
    for s in reg["sources"].as_array().unwrap() {
        let size = usize::try_from(s["size"].as_u64().unwrap()).unwrap();
        let byte = u8::try_from(s["byte"].as_u64().unwrap()).unwrap();
        registry.register(
            s["name"].as_str().unwrap(),
            Arc::new(VecDataSource::new(vec![byte; size])),
        );
    }
    registry
}

/// `rawQuery` cases carry the params as a query string — split it the
/// same way `serve_data`'s `parse_query` does (unknown keys ignored,
/// `k=v` pairs only).
fn data_case_request(req: &Value) -> DataRequest {
    let mut offset = req["offset"].as_u64().unwrap_or(0);
    let mut len = req["len"].as_u64();
    if let Some(q) = req["rawQuery"].as_str() {
        offset = 0;
        len = None;
        for pair in q.split('&') {
            if pair.is_empty() {
                continue;
            }
            let Some((k, v)) = pair.split_once('=') else {
                continue;
            };
            match k {
                "offset" => offset = v.parse().unwrap(),
                "len" => len = Some(v.parse().unwrap()),
                _ => {}
            }
        }
    }
    DataRequest {
        name: req["name"].as_str().unwrap().to_string(),
        offset,
        len,
    }
}

#[test]
fn ipc_data_registry_cases() {
    let doc = fixture("ipc/data.v1.json");
    for case in doc["cases"].as_array().expect("cases") {
        let id = case["id"].as_str().unwrap();
        // `call`/`url` cases exercise the JS bridge's URL builder —
        // not a Rust surface. Scheme-level name/param validation runs
        // in the webview crate's fixture lane.
        if case.get("registry").is_none() {
            continue;
        }
        let expect = &case["expect"];
        // `bad_name`/`bad_params` are scheme-layer checks in
        // `serve_data` — covered by the webview lane; the registry
        // itself only answers range errors.
        if expect["class"].as_str() == Some("bad_name")
            || expect["class"].as_str() == Some("bad_params")
        {
            continue;
        }
        let registry = data_case_registry(&case["registry"]);
        let r = registry.read(&data_case_request(&case["request"]));
        let status = expect["status"].as_u64().unwrap();
        match (status, r) {
            (200, Ok(chunk)) => {
                assert_eq!(
                    chunk.bytes.len() as u64,
                    expect["bodyLen"].as_u64().unwrap(),
                    "{id}: bodyLen"
                );
                assert_eq!(
                    chunk.total,
                    expect["total"].as_u64().unwrap(),
                    "{id}: total"
                );
            }
            (404, Err(ruviform_ui::DataError::NotFound { .. })) => {
                assert_eq!(expect["class"].as_str().unwrap(), "not_found");
            }
            (400, Err(ruviform_ui::DataError::OutOfRange { .. })) => {
                assert_eq!(expect["class"].as_str().unwrap(), "out_of_range");
            }
            (s, r) => panic!("{id}: status {s} vs read result {r:?}"),
        }
    }
}

// ── host scenario (window-lifecycle) ─────────────────────────────────

/// Scripted services declared by `setup.services`: `echo` answers the
/// params verbatim; `job-slow` runs until cancelled.
struct ScriptedServices;

impl Service for ScriptedServices {
    fn ops(&self) -> &'static [OpInfo] {
        &[
            OpInfo {
                name: "proj.echo",
                summary: "echo params verbatim",
            },
            OpInfo {
                name: "proj.job",
                summary: "cancellable long-running job",
            },
        ]
    }

    fn call(
        &mut self,
        op: &str,
        params: &Value,
        _ctx: &mut ServiceContext<'_>,
    ) -> Result<CallOutcome, ErrorBody> {
        match op {
            "proj.echo" => Ok(CallOutcome::Result(params.clone())),
            "proj.job" => Ok(CallOutcome::Job(JobSpec::new("proj.job", |ctx| {
                loop {
                    ctx.check_cancel()?;
                    ctx.progress(1, 0);
                    std::thread::sleep(Duration::from_millis(10));
                }
            }))),
            _ => Err(ErrorBody::new(
                codes::UNKNOWN_OP,
                format!("unknown op {op}"),
            )),
        }
    }
}

fn ev_fields(re: &RoutedEvent, names: &HashMap<WindowId, String>) -> Value {
    let wire: Value = serde_json::from_str(&re.event.to_json()).unwrap();
    let window = re
        .window
        .and_then(|w| names.get(&w).cloned())
        .unwrap_or_default();
    let mut out = json!({ "window": window, "kind": wire["kind"] });
    for k in ["id", "ok", "state", "job"] {
        if let Some(v) = wire.get(k) {
            out[k] = v.clone();
        }
    }
    if let Some(c) = wire["error"]["code"].as_str() {
        out["code"] = json!(c);
    }
    out
}

fn ev_matches(actual: &Value, pin: &Value) -> bool {
    pin.as_object()
        .unwrap()
        .iter()
        .all(|(k, v)| &actual[k] == v)
}

/// Scenario runner state: named windows → runtime ids; liveness models
/// the delivery layer (events produced for a destroyed window are
/// dropped, never an error — `drain_jobs_routed` keeps producing them).
struct HostState {
    names: HashMap<WindowId, String>,
    ids: HashMap<String, WindowId>,
    alive: HashSet<WindowId>,
    actual: Vec<Value>,
}

impl HostState {
    fn new(doc: &Value) -> Self {
        let mut st = Self {
            names: HashMap::new(),
            ids: HashMap::new(),
            alive: HashSet::new(),
            actual: Vec::new(),
        };
        for (i, w) in doc["windows"].as_array().unwrap().iter().enumerate() {
            let id = WindowId::new((i + 1) as u64).expect("nonzero");
            st.names.insert(id, w.as_str().unwrap().to_string());
            st.ids.insert(w.as_str().unwrap().to_string(), id);
            st.alive.insert(id);
        }
        st
    }

    fn collect(&mut self, evs: Vec<RoutedEvent>) {
        for re in evs {
            // Delivery-layer drop: events for destroyed windows never
            // reach the wire (still produced by the runtime).
            if re.window.is_some_and(|w| !self.alive.contains(&w)) {
                continue;
            }
            self.actual.push(ev_fields(&re, &self.names));
        }
    }

    fn run_op(&mut self, rt: &mut UiRuntime, op: &Value) {
        let name = op["op"].as_str().unwrap();
        let w = op["window"].as_str().map(|s| self.ids[s]);
        match name {
            "ready" => {
                let evs = rt.handle_window(
                    w.unwrap(),
                    HostEvent::Ready {
                        url: "rvf://app".to_string(),
                    },
                );
                self.collect(evs);
            }
            "message" => {
                let w = w.unwrap();
                // A destroyed window can't emit messages — the input
                // itself doesn't exist (no WebView to postMessage).
                if self.alive.contains(&w) {
                    let evs = rt.handle_window(
                        w,
                        HostEvent::Message(op["body"].as_str().unwrap().to_string()),
                    );
                    self.collect(evs);
                }
            }
            "tick" => {
                let evs = rt.drain_jobs_routed();
                self.collect(evs);
            }
            "close" => {
                let evs = rt.handle_window(w.unwrap(), HostEvent::CloseRequested);
                self.collect(evs);
            }
            "destroy" => {
                let w = w.unwrap();
                let evs = rt.handle_window(w, HostEvent::Destroyed);
                self.collect(evs);
                self.alive.remove(&w);
            }
            "waitJobs" => {
                let timeout = Duration::from_millis(op["timeoutMs"].as_u64().unwrap_or(5000));
                let deadline = Instant::now() + timeout;
                while rt.has_running_jobs() && Instant::now() < deadline {
                    let evs = rt.drain_jobs_routed();
                    if evs.is_empty() {
                        std::thread::sleep(Duration::from_millis(5));
                    } else {
                        self.collect(evs);
                    }
                }
                let evs = rt.drain_jobs_routed();
                self.collect(evs);
            }
            other => panic!("unknown scenario op {other}"),
        }
    }
}

#[test]
fn host_window_lifecycle_scenario() {
    let doc = fixture("scenarios/host/window-lifecycle.v1.json");
    let registry = DataRegistry::new();
    let mut rt = UiRuntime::new(registry);
    rt.register_service(ScriptedServices)
        .expect("scripted services");

    let mut st = HostState::new(&doc);
    for op in doc["in"].as_array().unwrap() {
        st.run_op(&mut rt, op);
    }

    // Ordered-subsequence match — fault-tolerant against interleaved
    // progress reports etc. (the pin list asserts contract events only).
    let expect = doc["expect"]["events"].as_array().unwrap();
    let mut cursor = 0usize;
    for (i, pin) in expect.iter().enumerate() {
        let j = st
            .actual
            .iter()
            .enumerate()
            .skip(cursor)
            .find(|(_, a)| ev_matches(a, pin))
            .map_or_else(
                || {
                    panic!(
                        "expect.events[{i}] {pin} not found after index {cursor}; actual: {:?}",
                        st.actual
                    )
                },
                |(j, _)| j,
            );
        cursor = j + 1;
    }
}
