//! Issue #243 — Window/Job invariants through the Fake/Headless
//! driver-neutral path.
//!
//! Composes `UiRuntime` + `FakeDriver` exactly the way the consumer
//! wiring does (`dummy_app`, GPUI `windowed_test_app`): `Action`
//! intents dispatch via `dispatch_window_intent`, `Tick` drains via
//! `drain_jobs_routed`, `CloseRequested` answers through
//! `respond_close`, and every produced [`RoutedEvent`] is delivered
//! through `post_event`. The harness additionally records every
//! [`RoutedEvent`] *before* delivery so tests can assert where the
//! runtime routed an event independently of whether the (possibly
//! dead) destination window accepted it.
//!
//! These fixtures pin the invariants existing coverage does not:
//!
//! - window-scoped job cancellation is exact — it never touches
//!   another window's jobs nor ownerless (application/project)
//!   jobs;
//! - a job that outlives its owner window still terminates, and its
//!   terminal `job` event plus command receipt stay routed to the
//!   recorded (now stale) `WindowId` — the delivery layer drops them
//!   instead of leaking them to a sibling window;
//! - a stale `WindowId` still identifies exactly its orphaned jobs
//!   for cleanup, never a different window's;
//! - inbound intents for a destroyed window never reach dispatch.
//!
//! `FakeDriver` success is contract conformance only — it is not
//! real native window, real input, or Dock acceptance (OS-L1, not
//! GUI-L2/EXT-L3).

use std::time::{Duration, Instant};

use ruviform_ui::{
    CloseDecision, DataRegistry, DemoService, DriverEvent, Event, Intent, JobState, LifecycleState,
    RoutedEvent, UiRuntime, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_test::{FakeDriver, deliver_routed};
use serde_json::{Value, json};

/// How the harness answers `CloseRequested` — the consumer's close
/// policy, mirroring `dummy_app`'s fixture-controlled knob.
#[derive(Clone, Copy)]
enum CloseAnswer {
    /// `respond_close(Approve)` — destroy now.
    Approve,
    /// `respond_close(Veto)` — the window survives.
    Veto,
}

/// `UiRuntime` + `FakeDriver` composed through the production
/// consumer wiring, plus a pre-delivery routed-event journal.
struct Harness {
    driver: FakeDriver,
    rt: UiRuntime,
    /// Close decision applied on `CloseRequested`.
    close_answer: CloseAnswer,
    /// Whether `Destroyed` triggers the consumer's window-job
    /// cleanup (`cancel_window_jobs`) — `dummy_app` policy parity.
    cancel_jobs_on_destroy: bool,
    /// Every [`RoutedEvent`] the runtime produced, before delivery.
    routed: Vec<RoutedEvent>,
}

impl Harness {
    fn new(cancel_jobs_on_destroy: bool) -> Self {
        let mut rt = UiRuntime::new(DataRegistry::new());
        rt.register_service(DemoService::new())
            .expect("demo service registers");
        Self {
            driver: FakeDriver::new(),
            rt,
            close_answer: CloseAnswer::Approve,
            cancel_jobs_on_destroy,
            routed: Vec::new(),
        }
    }

    /// Creates a window and marks it `Ready` — the state a real
    /// window reaches before the app accepts its traffic.
    fn open(&mut self, title: &str) -> WindowId {
        let w = self
            .driver
            .create_window(&WindowSpec {
                title: title.to_string(),
                ..WindowSpec::default()
            })
            .expect("create_window");
        self.driver.inject(DriverEvent::Windowed {
            window: w,
            event: WindowLifecycle::Ready,
        });
        self.pump();
        w
    }

    /// One `driver.pump` cycle with the consumer wiring: normalize →
    /// dispatch → deliver. Job drains ride `DriverEvent::Tick`.
    fn pump(&mut self) {
        let rt = &mut self.rt;
        let routed = &mut self.routed;
        let answer = self.close_answer;
        let cancel_on_destroy = self.cancel_jobs_on_destroy;
        self.driver.pump(&mut |dev, drv| match dev {
            DriverEvent::Windowed { window, event } => match event {
                WindowLifecycle::Action(intent) => {
                    for re in rt.dispatch_window_intent(window, intent) {
                        routed.push(re.clone());
                        deliver_routed(drv, &re);
                    }
                }
                WindowLifecycle::CloseRequested => {
                    let decision = match answer {
                        CloseAnswer::Approve => CloseDecision::Approve,
                        CloseAnswer::Veto => CloseDecision::Veto,
                    };
                    let _ = drv.respond_close(window, decision);
                    let re = RoutedEvent {
                        window: Some(window),
                        event: Event::lifecycle(LifecycleState::Closing),
                    };
                    routed.push(re.clone());
                    deliver_routed(drv, &re);
                }
                // The consumer's window-job policy — never the
                // host's. `dummy_app` cancels on destroy; tests
                // opt out to observe orphaned-job behavior.
                WindowLifecycle::Destroyed if cancel_on_destroy => {
                    rt.cancel_window_jobs(window);
                }
                _ => {}
            },
            DriverEvent::Tick => {
                for re in rt.drain_jobs_routed() {
                    routed.push(re.clone());
                    deliver_routed(drv, &re);
                }
            }
            _ => {}
        });
    }

    /// Injects a `Tick` and pumps — the job-drain wakeup.
    fn tick(&mut self) {
        self.driver.inject(DriverEvent::Tick);
        self.pump();
    }

    /// Sends a typed `Action` intent to `window` and pumps — the
    /// driver-neutral equivalent of a parsed `postMessage` body.
    fn send_intent(&mut self, window: WindowId, id: &str, op: &str, params: Value) {
        self.driver.inject(DriverEvent::Windowed {
            window,
            event: WindowLifecycle::Action(Intent::new(id, op, params)),
        });
        self.pump();
    }

    /// Spawns `demo.sleep` from `window`; returns the issued job id
    /// (read back from the delivered ack `result`).
    fn spawn_sleep(&mut self, window: WindowId, command: &str, ms: u64) -> u64 {
        self.send_intent(window, command, "demo.sleep", json!({ "ms": ms }));
        self.job_id_of(window, command)
    }

    /// Spawns `demo.sleep` through the headless `dispatch_op` path —
    /// the application/project-owned (ownerless) job analog. Its
    /// events are unrouted (`window: None`); the consumer picks the
    /// sink.
    fn spawn_headless(&mut self, command: &str, ms: u64) -> u64 {
        let events = self
            .rt
            .dispatch_op(command, "demo.sleep", json!({ "ms": ms }));
        let job = events
            .iter()
            .find_map(|e| match e {
                Event::Result { data: Some(d), .. } => d.get("job").and_then(Value::as_u64),
                _ => None,
            })
            .expect("sleep ack carries a job id");
        for event in events {
            self.routed.push(RoutedEvent {
                window: None,
                event,
            });
        }
        job
    }

    /// Job id from the ack `result` delivered to `window` for
    /// `command`.
    fn job_id_of(&self, window: WindowId, command: &str) -> u64 {
        self.driver
            .events_for(window)
            .iter()
            .find_map(|e| match e {
                Event::Result {
                    id, data: Some(d), ..
                } if id.as_str() == command => d.get("job").and_then(Value::as_u64),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no job ack for {command} on {window}"))
    }

    /// Pumps `Tick`s until `pred` holds or the deadline expires.
    fn drain_until(&mut self, pred: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !pred(self) {
            assert!(
                Instant::now() < deadline,
                "drain_until timed out; routed so far: {:?}",
                self.routed
            );
            self.tick();
            std::thread::sleep(Duration::from_millis(5));
        }
        self.tick();
    }

    /// Routed events with no window destination (unrouted/app-owned).
    fn routed_unrouted(&self) -> Vec<&RoutedEvent> {
        self.routed
            .iter()
            .filter(|re| re.window.is_none())
            .collect()
    }
}

/// The most recent `job` lifecycle state `job` reached in the routed
/// journal (progress signals precede the terminal state per job).
fn last_job_state(h: &Harness, job: u64) -> Option<JobState> {
    h.routed
        .iter()
        .filter_map(|re| match &re.event {
            Event::Job { job: j, state, .. } if *j == job => Some(*state),
            _ => None,
        })
        .next_back()
}

/// Whether the routed journal holds `job`'s terminal event routed to
/// `window`.
fn terminal_routed_to(h: &Harness, job: u64, window: Option<WindowId>) -> bool {
    h.routed.iter().any(|re| {
        re.window == window
            && matches!(
                &re.event,
                Event::Job { job: j, state, .. }
                    if *j == job
                        && matches!(
                            state,
                            JobState::Finished | JobState::Cancelled | JobState::Failed
                        )
            )
    })
}

/// Whether the command receipt for `command` was produced routed to
/// `window` (`ok: true` = applied, `ok: false` = rejected — the
/// Issue #456 terminal receipt under the spawning intent's id).
fn receipt_routed_to(h: &Harness, command: &str, window: Option<WindowId>, ok: bool) -> bool {
    h.routed.iter().any(|re| {
        re.window == window
            && matches!(
                &re.event,
                Event::Result { id, ok: o, .. } if id.as_str() == command && *o == ok
            )
    })
}

/// Window-scoped cancellation is exact: `cancel_window_jobs(w)`
/// signals only the jobs recorded with `owner == w`. Jobs owned by
/// another window and ownerless (application/project-dispatched)
/// jobs survive — and their events stay unrouted.
#[test]
fn window_job_cancel_is_exact_and_spares_unowned_jobs() {
    let mut h = Harness::new(true);
    let w1 = h.open("main");
    let w2 = h.open("panel");

    let j1 = h.spawn_sleep(w1, "cmd-w1", 60_000);
    let j2 = h.spawn_sleep(w2, "cmd-w2", 60_000);
    let ja = h.spawn_headless("cmd-app", 60_000);
    assert!(h.rt.has_running_jobs());

    // Cancelling w1 signals exactly its own job.
    assert_eq!(h.rt.cancel_window_jobs(w1), 1);
    h.drain_until(|h| terminal_routed_to(h, j1, Some(w1)));
    assert!(receipt_routed_to(&h, "cmd-w1", Some(w1), false));
    // w2's and the app-owned job are untouched and still running.
    assert!(h.rt.has_running_jobs());
    assert!(!terminal_routed_to(&h, j2, Some(w2)));
    assert!(!terminal_routed_to(&h, ja, None));

    // Same precision on the second window.
    assert_eq!(h.rt.cancel_window_jobs(w2), 1);
    h.drain_until(|h| terminal_routed_to(h, j2, Some(w2)));
    assert!(receipt_routed_to(&h, "cmd-w2", Some(w2), false));
    assert!(h.rt.has_running_jobs(), "app-owned job survives");
    assert!(!terminal_routed_to(&h, ja, None));

    // The ownerless job's terminal events stay unrouted — the
    // consumer picks the sink; nothing leaks to a window.
    h.send_intent(w1, "cmd-cancel-app", "job.cancel", json!({ "job": ja }));
    h.drain_until(|h| terminal_routed_to(h, ja, None));
    assert!(receipt_routed_to(&h, "cmd-app", None, false));
    assert!(h.routed_unrouted().iter().any(|re| matches!(
        re.event,
        Event::Job {
            state: JobState::Cancelled,
            ..
        }
    )));
    assert!(!h.rt.has_running_jobs());
}

/// Window close is not job cancellation — but the consumer's close
/// policy (`cancel_window_jobs` on `Destroyed`, the `dummy_app`
/// wiring) scopes cleanup to the dying window only. A vetoed close
/// cancels nothing; an approved close retires the window and its
/// jobs while the sibling window and the app-owned job survive.
#[test]
fn close_policy_cancels_only_the_closed_windows_jobs() {
    let mut h = Harness::new(true);
    let w1 = h.open("main");
    let w2 = h.open("panel");

    let j1 = h.spawn_sleep(w1, "cmd-w1", 60_000);
    let j2 = h.spawn_sleep(w2, "cmd-w2", 60_000);
    let ja = h.spawn_headless("cmd-app", 60_000);

    // Veto: the close request is answered but the window survives —
    // no job is touched.
    h.close_answer = CloseAnswer::Veto;
    h.driver.request_close(w2);
    h.pump();
    assert!(h.driver.windows().contains(&w2), "veto keeps w2 open");
    assert!(h.rt.has_running_jobs());
    assert!(!terminal_routed_to(&h, j2, Some(w2)));

    // Approve: CloseRequested → Approve → Destroyed → the consumer's
    // destroy policy cancels w2's jobs — and only w2's.
    h.close_answer = CloseAnswer::Approve;
    h.driver.request_close(w2);
    h.pump();
    assert!(!h.driver.windows().contains(&w2), "approve retires w2");
    assert!(h.driver.destroyed().contains(&w2));
    h.drain_until(|h| terminal_routed_to(h, j2, Some(w2)));
    assert!(receipt_routed_to(&h, "cmd-w2", Some(w2), false));
    // w1's and the app-owned jobs are still running.
    assert!(h.rt.has_running_jobs());
    assert!(!terminal_routed_to(&h, j1, Some(w1)));
    assert!(!terminal_routed_to(&h, ja, None));

    // w1 still receives its own traffic after w2's teardown.
    h.send_intent(w1, "cmd-ping", "sys.ping", json!({}));
    assert!(
        h.driver
            .events_for(w1)
            .iter()
            .any(|e| matches!(e, Event::Result { id, ok: true, .. } if id.as_str() == "cmd-ping"))
    );

    // Cleanup: scoped cancels leave only expected terminals.
    assert_eq!(h.rt.cancel_window_jobs(w1), 1);
    h.send_intent(w1, "cmd-cancel-app", "job.cancel", json!({ "job": ja }));
    h.drain_until(|h| terminal_routed_to(h, j1, Some(w1)) && terminal_routed_to(h, ja, None));
    assert!(!h.rt.has_running_jobs());
}

/// A job that outlives its owner window still terminates: the
/// runtime keeps producing its terminal `job` event and the
/// spawning command's receipt routed to the recorded (now stale)
/// `WindowId`. `post_event` drops them at the dead destination —
/// they never leak to a sibling window — while a stale inbound
/// `Action` never reaches dispatch.
#[test]
fn job_outliving_its_window_routes_terminal_to_the_dead_window_only() {
    // Consumer does not auto-cancel on destroy — the job is orphaned.
    let mut h = Harness::new(false);
    let w1 = h.open("doomed");
    let w2 = h.open("stable");

    let j1 = h.spawn_sleep(w1, "cmd-doom", 200);
    // Destroy w1 before the job finishes; the runtime does not
    // implicitly cancel it.
    h.driver.destroy_window(w1).expect("destroy w1");
    h.pump();
    assert!(h.driver.destroyed().contains(&w1));
    assert!(h.rt.has_running_jobs(), "job outlives its window");

    // A stale inbound intent for the dead window is dropped by the
    // driver before dispatch — no result is ever produced for it.
    h.driver.inject(DriverEvent::Windowed {
        window: w1,
        event: WindowLifecycle::Action(Intent::new("stale-intent", "demo.echo", json!({}))),
    });
    h.pump();
    assert!(
        !h.routed.iter().any(
            |re| matches!(&re.event, Event::Result { id, .. } if id.as_str() == "stale-intent")
        ),
        "stale-window intent must not produce a result"
    );

    // The sibling window keeps working — its own job is the control.
    let j2 = h.spawn_sleep(w2, "cmd-stable", 200);
    let w1_delivered_before = h.driver.events_for(w1).len();

    // Both jobs terminate; each terminal + receipt stays routed to
    // its recorded owner — including the dead w1.
    h.drain_until(|h| terminal_routed_to(h, j1, Some(w1)) && terminal_routed_to(h, j2, Some(w2)));
    assert!(receipt_routed_to(&h, "cmd-doom", Some(w1), true));
    assert!(receipt_routed_to(&h, "cmd-stable", Some(w2), true));

    // Routing isolation: j1's events went only to Some(w1), j2's only
    // to Some(w2) — nothing crossed, nothing unrouted.
    for re in h
        .routed
        .iter()
        .filter(|re| matches!(&re.event, Event::Job { job, .. } if *job == j1 || *job == j2))
    {
        let expected = if matches!(&re.event, Event::Job { job, .. } if *job == j1) {
            Some(w1)
        } else {
            Some(w2)
        };
        assert_eq!(re.window, expected, "job event leaked: {re:?}");
    }

    // Delivery-layer drop: w1 accepted nothing after its death, while
    // w2 received its own terminal events.
    assert_eq!(h.driver.events_for(w1).len(), w1_delivered_before);
    assert!(
        h.driver
            .events_for(w2)
            .iter()
            .any(|e| matches!(e, Event::Job { job, state: JobState::Finished, .. } if *job == j2))
    );

    // No double destruction: a second destroy of the retired id is
    // rejected, and the id is never reused for a new window.
    assert!(h.driver.destroy_window(w1).is_err());
    let w3 = h.open("reborn");
    assert_ne!(w3, w1);
    assert_ne!(w3, w2);
}

/// A stale `WindowId` still identifies exactly its orphaned jobs:
/// `cancel_window_jobs` matches the owner recorded at spawn, so a
/// late consumer cleanup reaches the dead window's jobs — and,
/// because ids are never reused, can never hit a different window's.
#[test]
fn cancel_window_jobs_reaches_orphans_via_stale_window_id() {
    let mut h = Harness::new(false);
    let w1 = h.open("doomed");
    let w2 = h.open("stable");

    let j1 = h.spawn_sleep(w1, "cmd-doom", 60_000);
    let j2 = h.spawn_sleep(w2, "cmd-stable", 60_000);

    h.driver.destroy_window(w1).expect("destroy w1");
    h.pump();
    assert!(h.rt.has_running_jobs());

    // The recorded owner survives window death: the stale id cancels
    // exactly j1 — j2 keeps running.
    assert_eq!(h.rt.cancel_window_jobs(w1), 1);
    h.drain_until(|h| terminal_routed_to(h, j1, Some(w1)));
    assert_eq!(last_job_state(&h, j1), Some(JobState::Cancelled));
    assert!(receipt_routed_to(&h, "cmd-doom", Some(w1), false));
    assert!(h.rt.has_running_jobs(), "w2 job survives");
    assert!(!terminal_routed_to(&h, j2, Some(w2)));
    // The cancelled orphan's terminal events were produced for the
    // dead window and dropped at delivery — w2 saw none of them.
    assert!(
        !h.driver
            .events_for(w2)
            .iter()
            .any(|e| matches!(e, Event::Job { job, .. } if *job == j1))
    );

    assert_eq!(h.rt.cancel_window_jobs(w2), 1);
    h.drain_until(|h| terminal_routed_to(h, j2, Some(w2)));
    assert!(!h.rt.has_running_jobs());
}
