//! `UiRuntime` — the dispatcher that both entry points share.
//!
//! The runtime owns: registered [`Service`]s, the `rvf.ipc.v1` routing
//! table (built-in `sys.*` ops + `job.cancel`), the job registry and
//! the worker mailbox. It consumes normalized [`HostEvent`]s from an
//! adapter and produces [`Event`]s to push — identical semantics
//! whether the events arrive from a real webview (`HostEvent::Message`)
//! or a headless CLI calling [`UiRuntime::dispatch_op`] directly.
//!
//! Threading: the runtime itself is single-threaded UI-side. Job
//! workers report through a bounded mailbox drained on
//! [`HostEvent::Tick`] (GUI) or [`UiRuntime::poll`] (headless), so a
//! busy worker can never block the UI thread and a stalled UI can
//! never grow the mailbox unboundedly.
//!
//! **RT contract:** non-RT only.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};
use std::thread;
use std::time::Duration;

use ruviform_exec::CancellationToken;
use serde_json::{Value, json};

use crate::data::DataRegistry;
use crate::error::UiError;
use crate::event::HostEvent;
use crate::ipc::{
    ErrorBody, Event, Intent, LifecycleState, SCHEMA_VERSION, codes, parse_intent, valid_op,
};
use crate::service::{
    CallOutcome, JobContext, JobResult, JobSignal, JobSpec, OpInfo, Service, ServiceContext,
};

/// Bounded job→UI mailbox capacity. Progress is try-sent (dropped
/// under pressure); terminal signals block-send — bounded because a
/// wedged UI that never drains wedges the process anyway.
const JOB_MAILBOX: usize = 256;

const BUILTIN_OPS: &[OpInfo] = &[
    OpInfo {
        name: "sys.ping",
        summary: "Connectivity check (returns {\"pong\": true})",
    },
    OpInfo {
        name: "sys.capabilities",
        summary: "Schema version, registered ops, data-transfer limits",
    },
    OpInfo {
        name: "job.cancel",
        summary: "Request cancellation of a running job ({\"job\": id})",
    },
];

/// Ops owned by the runtime; services may not register them.
const RESERVED_OPS: &[&str] = &["job.cancel"];

struct JobRecord {
    cancel: CancellationToken,
}

/// The shared GUI/CLI dispatcher. See module docs.
///
/// **RT contract:** non-RT only — driven on the UI thread.
pub struct UiRuntime {
    data: DataRegistry,
    services: Vec<Box<dyn Service>>,
    jobs: HashMap<u64, JobRecord>,
    next_job: u64,
    job_tx: SyncSender<JobSignal>,
    job_rx: Receiver<JobSignal>,
}

impl UiRuntime {
    /// Empty runtime sharing `data` with the scheme handler and job
    /// workers.
    #[must_use]
    pub fn new(data: DataRegistry) -> Self {
        let (job_tx, job_rx) = sync_channel(JOB_MAILBOX);
        Self {
            data,
            services: Vec::new(),
            jobs: HashMap::new(),
            next_job: 1,
            job_tx,
            job_rx,
        }
    }

    /// Registers a service's ops.
    ///
    /// # Errors
    /// [`UiError::Internal`] when an op name is malformed, reserved
    /// (`sys.*`, `job.cancel`), or already registered.
    pub fn register_service(&mut self, service: impl Service + 'static) -> Result<(), UiError> {
        let mut seen = HashSet::new();
        for op in service.ops() {
            if !valid_op(op.name) {
                return Err(UiError::internal(format!(
                    "service registered malformed op {:?}",
                    op.name
                )));
            }
            if op.name.starts_with("sys.") || RESERVED_OPS.contains(&op.name) {
                return Err(UiError::internal(format!(
                    "op {:?} is reserved to the runtime",
                    op.name
                )));
            }
            if !seen.insert(op.name)
                || self
                    .services
                    .iter()
                    .any(|s| s.ops().iter().any(|o| o.name == op.name))
            {
                return Err(UiError::internal(format!("duplicate op {:?}", op.name)));
            }
        }
        self.services.push(Box::new(service));
        Ok(())
    }

    /// Shared data registry (`rvf://data` sources).
    #[must_use]
    pub fn data(&self) -> &DataRegistry {
        &self.data
    }

    /// Whether any job is still running (headless poll loops wait on
    /// this).
    #[must_use]
    pub fn has_running_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Consumes one normalized host event; returns the events to push.
    ///
    /// - `Message` → parse + dispatch (malformed bodies yield an
    ///   uncorrelated `error` event).
    /// - `Tick` → drain the job mailbox.
    /// - `CloseRequested` → cancels running jobs and emits a
    ///   `lifecycle` `closing` event.
    /// - Everything else → no IPC traffic.
    pub fn handle(&mut self, ev: HostEvent) -> Vec<Event> {
        match ev {
            HostEvent::Message(body) => match parse_intent(&body) {
                Ok(intent) => self.dispatch(intent),
                Err(e) => vec![Event::error(e)],
            },
            HostEvent::Tick => self.drain_jobs(),
            HostEvent::CloseRequested => {
                for rec in self.jobs.values() {
                    rec.cancel.cancel();
                }
                vec![Event::lifecycle(LifecycleState::Closing)]
            }
            _ => Vec::new(),
        }
    }

    /// Headless intent entry point — the same routing a parsed
    /// [`HostEvent::Message`] takes (used by `demo_cli`, tests, and
    /// any non-GUI caller).
    pub fn dispatch_op(&mut self, id: impl Into<String>, op: &str, params: Value) -> Vec<Event> {
        let id = id.into();
        if !valid_op(op) {
            return vec![Event::result_err(
                id,
                ErrorBody::new(codes::MALFORMED_MESSAGE, format!("invalid op name {op:?}")),
            )];
        }
        self.dispatch(Intent {
            v: SCHEMA_VERSION,
            kind: "intent".to_string(),
            id,
            op: op.to_string(),
            params,
        })
    }

    /// Drains pending job signals into events (GUI path: call on every
    /// [`HostEvent::Tick`]).
    pub fn drain_jobs(&mut self) -> Vec<Event> {
        self.collect(None)
    }

    /// Headless wait loop: returns immediately with pending events, or
    /// blocks up to `wait` for the next job signal when jobs are
    /// running. With `wait == None`, never blocks.
    #[must_use]
    pub fn poll(&mut self, wait: Option<Duration>) -> Vec<Event> {
        match self.job_rx.try_recv() {
            Ok(sig) => return self.collect(Some(sig)),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
        }
        let Some(d) = wait else { return Vec::new() };
        if self.jobs.is_empty() {
            return Vec::new();
        }
        match self.job_rx.recv_timeout(d) {
            Ok(sig) => self.collect(Some(sig)),
            Err(_) => Vec::new(),
        }
    }

    fn dispatch(&mut self, intent: Intent) -> Vec<Event> {
        let Intent { id, op, params, .. } = intent;
        match op.as_str() {
            "sys.ping" => vec![Event::result_ok(id, json!({ "pong": true }))],
            "sys.capabilities" => vec![Event::result_ok(id, self.capabilities())],
            "job.cancel" => self.cancel_job(id, &params),
            _ => self.dispatch_service(id, &op, &params),
        }
    }

    fn dispatch_service(&mut self, id: String, op: &str, params: &Value) -> Vec<Event> {
        let Some(svc) = self
            .services
            .iter_mut()
            .find(|s| s.ops().iter().any(|o| o.name == op))
        else {
            return vec![Event::result_err(
                id,
                ErrorBody::new(codes::UNKNOWN_OP, format!("unregistered op {op:?}")),
            )];
        };
        let mut ctx = ServiceContext { data: &self.data };
        match svc.call(op, params, &mut ctx) {
            Ok(CallOutcome::Result(data)) => vec![Event::result_ok(id, data)],
            Ok(CallOutcome::Job(spec)) => vec![self.spawn_job(id, spec)],
            Err(e) => vec![Event::result_err(id, e)],
        }
    }

    fn spawn_job(&mut self, id: String, spec: JobSpec) -> Event {
        let job = self.next_job;
        self.next_job += 1;
        let cancel = CancellationToken::new();
        let label = spec.label;
        let ctx = JobContext {
            job,
            cancel: cancel.clone(),
            progress_tx: self.job_tx.clone(),
            data: self.data.clone(),
        };
        let spawn = thread::Builder::new()
            .name(format!("rvf-job-{job}"))
            .spawn(move || {
                let result = match catch_unwind(AssertUnwindSafe(|| spec.execute(&ctx))) {
                    Ok(Ok(v)) => JobResult::Ok(v),
                    Ok(Err(e)) => JobResult::Err(e),
                    Err(_) => JobResult::Panic,
                };
                let _ = ctx.progress_tx.send(JobSignal::Finished { job, result });
            });
        match spawn {
            Ok(_) => {
                self.jobs.insert(job, JobRecord { cancel });
                Event::result_ok(id, json!({ "job": job, "label": label }))
            }
            Err(e) => Event::result_err(
                id,
                ErrorBody::new(
                    codes::SERVICE_ERROR,
                    format!("failed to spawn job worker: {e}"),
                ),
            ),
        }
    }

    fn cancel_job(&mut self, id: String, params: &Value) -> Vec<Event> {
        let Some(job) = params.get("job").and_then(Value::as_u64) else {
            return vec![Event::result_err(
                id,
                ErrorBody::new(
                    codes::INVALID_PARAMS,
                    "job.cancel requires a numeric \"job\" param",
                ),
            )];
        };
        match self.jobs.get(&job) {
            Some(rec) => {
                rec.cancel.cancel();
                vec![Event::result_ok(
                    id,
                    json!({ "job": job, "cancelling": true }),
                )]
            }
            None => vec![Event::result_err(
                id,
                ErrorBody::new(codes::UNKNOWN_JOB, format!("job {job} is not running")),
            )],
        }
    }

    fn collect(&mut self, first: Option<JobSignal>) -> Vec<Event> {
        let mut events = Vec::new();
        // Latest progress per job within this drain batch (BTreeMap for
        // deterministic ordering across drains).
        let mut latest: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
        let mut pending = first;
        loop {
            let sig = match pending.take() {
                Some(s) => Some(s),
                None => self.job_rx.try_recv().ok(),
            };
            let Some(sig) = sig else { break };
            match sig {
                JobSignal::Progress { job, done, total } => {
                    latest.insert(job, (done, total));
                }
                JobSignal::Finished { job, result } => {
                    if let Some((d, t)) = latest.remove(&job) {
                        events.push(Event::job_progress(job, d, t));
                    }
                    self.jobs.remove(&job);
                    events.push(match result {
                        JobResult::Ok(v) => Event::job_finished(job, v),
                        JobResult::Err(e) if e.is_cancelled() => Event::job_cancelled(job),
                        JobResult::Err(e) => Event::job_failed(job, e),
                        JobResult::Panic => Event::job_failed(
                            job,
                            ErrorBody::new(codes::WORKER_FAILED, "job worker panicked"),
                        ),
                    });
                }
            }
        }
        for (job, (done, total)) in latest {
            events.push(Event::job_progress(job, done, total));
        }
        events
    }

    fn capabilities(&self) -> Value {
        let mut ops: Vec<Value> = BUILTIN_OPS
            .iter()
            .map(|o| json!({ "name": o.name, "summary": o.summary }))
            .collect();
        for svc in &self.services {
            for op in svc.ops() {
                ops.push(json!({ "name": op.name, "summary": op.summary }));
            }
        }
        json!({
            "schema": SCHEMA_VERSION,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "ops": ops,
            "data": {
                "max_chunk": self.data.max_chunk(),
                "max_sources": self.data.max_sources(),
                "sources": self.data.names(),
            },
            "jobs": { "running": self.jobs.len() },
        })
    }
}
