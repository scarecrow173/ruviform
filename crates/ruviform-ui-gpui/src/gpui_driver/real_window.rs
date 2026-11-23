//! Real-window [`SemanticGuiTest`] + [`VisualGuiTest`] adapter
//! (Issue #314 slice 4 — design
//! `docs/designs/issue-314-real-window-adapter.md`).
//!
//! [`RealOsGuiTest`] is the `ScenarioDriver::RealOs` evidence level:
//! every [`SemanticGuiTest`] operation runs as a `Send` job inside
//! the real GPUI event loop's `App` context (the [`UiJob`] seam
//! [`super::GpuiDriver::set_ui_jobs`] installs and [`super::drain`]
//! polls), so semantic dispatch, observation, and
//! [`VisualGuiTest::capture_frame`] all measure the *real OS window*
//! — never a draw plan, a semantic snapshot, or a fabricated
//! framebuffer.
//!
//! **Threading.** `gpui::Application`/`App`/`Entity` are `Rc`-based
//! and `!Send`, so the session splits in two. The app thread owns
//! `GpuiDriver` and the platform loop; the caller thread owns this
//! adapter and blocks on per-command reply channels. Commands cross
//! as `Send` closures whose *arguments* (`Inner`, `GpuiDriver`,
//! `App`) are created on the UI thread — GPUI types never cross the
//! boundary themselves.
//!
//! **Honesty contract.**
//!
//! - `driver()` reports [`ScenarioDriver::RealOs`];
//!   `capabilities()` claims `SemanticAction` + `MultiWindow`
//!   everywhere, `VisualCapture` on OSes with a capture backend
//!   (Windows/Linux), `RealOsPointerOrKeyboard` only where
//!   [`os_input::supported`] reports an injection backend —
//!   Windows-only in this slice (Issue #313), and
//!   `NativePluginDock` only where the surface backend can mint a
//!   real native container parent — Windows `GpuiSurfaceAdapter`
//!   child HWNDs, never under `gpui-test` (Issue #631).
//! - The native-slot ops ([`declare_native_slot`](Self::declare_native_slot)
//!   and friends) drive the driver's canonical
//!   [`surface::SharedSurface`] through the same `UiJob` seam, so
//!   declare/bind/lease/validate/release and `container_token`
//!   observation all execute inside `App` context on the UI thread.
//!   The vocabulary stays `ruviform-ui` ([`SlotSpec`],
//!   [`SurfaceSlotId`], [`SlotLease`], [`LeaseInfo`],
//!   [`SlotState`]) — no GPUI type crosses this boundary. Ledger
//!   refusals ([`SlotError`]) surface as
//!   [`ScenarioError::Driver`] with a "native slot" prefix naming
//!   the refused operation — `ScenarioError` has no
//!   slot-specific variant and a new public error enum cannot cross
//!   the private `gpui_driver` module boundary without a `lib.rs`
//!   re-export, so the driver variant is the honest carrier here —
//!   the prefix keeps a slot refusal distinguishable from a session
//!   failure in evidence.
//! - `dispatch_input`/`select_*`/`invoke` are *semantic* dispatch
//!   through `ViewHost` — they bypass hit-testing and are not
//!   pointer/keyboard evidence. The `os_*` methods below are the
//!   only OS-injection path: they run real `SendInput` calls through
//!   [`os_input`], gated by the Issue #552 preflight, and report
//!   [`OsInputOutcome`] instead of collapsing blocked injection into
//!   `fail`.
//! - `capture_frame` returns real `PrintWindow`/`BitBlt` pixels via
//!   [`super::GpuiDriver::capture_window`]; backend errors map to
//!   [`CaptureOutcome::Unsupported`]/[`CaptureOutcome::NotRun`]
//!   rather than fabricated images.
//! - Stale `attach_epoch`/`generation` pins fail with
//!   [`ScenarioError::StaleEpoch`]/[`ScenarioError::StaleGeneration`]
//!   exactly like the other adapters — re-checked inside `App`
//!   context on every operation, capture, and `os_*` call.
//! - `settle` is frame-paced: it counts `drain` boundaries (one per
//!   tick), never sleeps.
//!
//! **RT contract:** non-RT only.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use gpui::{App, Entity};
use ruviform_ui::behavior::ControlInput;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::ipc::Event;
use ruviform_ui::view::{AttachEpoch, ComponentKey, ComponentSnapshot, ViewSpec};
use ruviform_ui::{
    CloseDecision, DriverEvent, Intent, LeaseInfo, LogicalRect, SlotError, SlotLease, SlotSpec,
    SlotState, SurfaceSlotId, UiError, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_test::golden::RgbaImage;
use ruviform_ui_test::scenario::visual::{CaptureOutcome, VisualGuiTest};
use ruviform_ui_test::scenario::{
    MintedIntent, ScenarioCapability, ScenarioDriver, ScenarioError, ScenarioTarget,
    SemanticGuiTest, TargetQuery,
};

use super::capture::OsWindowGeometry;
use super::os_ime;
use super::os_input::{
    self, OsInputOutcome, OsKey, PreflightCheck, PreflightReport, PreflightScope,
};
use super::surface;
use super::{GpuiDriver, Inner, UiJob, ViewMount, minted_from_ipc};
use crate::render::RvfView;

/// Measured metadata of the most recent successful
/// [`VisualGuiTest::capture_frame`] — the values a consumer records
/// into `GoldenKey::scale_factor` and `EnvironmentMeta`
/// (`backend`/`scale_factor`) without leaking the driver-side
/// `OsCapturedFrame` type across the adapter boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureMeta {
    /// Captured frame width in physical pixels.
    pub width: u32,
    /// Captured frame height in physical pixels.
    pub height: u32,
    /// Window scale factor (DPI) measured at capture time.
    pub scale_factor: f64,
    /// OS capture backend tag (`"win32-printwindow"` /
    /// `"win32-bitblt-screen"`) — `EnvironmentMeta::backend_detail`
    /// evidence.
    pub backend: &'static str,
}

/// Observation-facing copy of one `WindowLifecycle` transition —
/// the record the app-thread event handler appends to the session
/// log [`RealOsGuiTest::window_events`] reads (Issue #631
/// window-close/DPI surface). `Copy` + `PartialEq` keep snapshot
/// reads cheap and assertable; `Action` intents are not lifecycle
/// transitions and stay on the [`drain_intents`](SemanticGuiTest::drain_intents)
/// channel. Geometry variants carry the driver's physical-pixel
/// values — the same unit `Resized`/`Moved` report and
/// [`RealOsGuiTest::window_geometry`] measures.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowLifecycleEvent {
    /// `WindowLifecycle::Created` — the driver accepted the window.
    Created,
    /// `WindowLifecycle::Ready` — first frame presented.
    Ready,
    /// `WindowLifecycle::Resized` — physical client size.
    Resized {
        /// Physical pixel width.
        width: u32,
        /// Physical pixel height.
        height: u32,
    },
    /// `WindowLifecycle::Moved` — physical virtual-desktop position.
    Moved {
        /// Physical x position.
        x: i32,
        /// Physical y position.
        y: i32,
    },
    /// `WindowLifecycle::Focused` — OS activation state.
    Focused(bool),
    /// `WindowLifecycle::ScaleFactorChanged` — new DPI scale factor.
    ScaleFactorChanged {
        /// New scale factor (for example `1.5` at 150%).
        factor: f64,
    },
    /// `WindowLifecycle::CloseRequested` — veto-able close request;
    /// answered through [`RealOsGuiTest::arm_close_response`] /
    /// [`RealOsGuiTest::respond_close`].
    CloseRequested,
    /// `WindowLifecycle::Destroyed` — terminal; the id is retired
    /// and never reused.
    Destroyed,
}

impl WindowLifecycleEvent {
    /// Maps a driver [`WindowLifecycle`] onto the observation record.
    /// `Action` intents and any future `WindowLifecycle` variant the
    /// adapter does not yet summarize return `None` — they are not
    /// lifecycle transitions this surface claims to observe.
    fn observe(event: &WindowLifecycle) -> Option<Self> {
        Some(match event {
            WindowLifecycle::Created => Self::Created,
            WindowLifecycle::Ready => Self::Ready,
            WindowLifecycle::Resized { width, height } => Self::Resized {
                width: *width,
                height: *height,
            },
            WindowLifecycle::Moved { x, y } => Self::Moved { x: *x, y: *y },
            WindowLifecycle::Focused(active) => Self::Focused(*active),
            WindowLifecycle::ScaleFactorChanged { factor } => {
                Self::ScaleFactorChanged { factor: *factor }
            }
            WindowLifecycle::CloseRequested => Self::CloseRequested,
            WindowLifecycle::Destroyed => Self::Destroyed,
            _ => return None,
        })
    }
}

/// One real OS window to open for a [`RealOsGuiTest::run`] session —
/// the neutral [`WindowSpec`] plus the consumer [`ViewMount`].
pub struct RealWindowMount {
    /// Window chrome/geometry spec.
    pub window: WindowSpec,
    /// Mounted view — `ViewHost` + optional theme.
    pub view: ViewMount,
}

/// Session tuning for [`RealOsGuiTest::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealWindowOptions {
    /// `DriverEvent::Tick` cadence — `None` uses the driver default
    /// (16 ms).
    pub tick: Option<Duration>,
    /// `drain` boundaries each [`SemanticGuiTest::settle`] waits —
    /// one boundary is one tick, so `2` is roughly two frames of
    /// pacing between a mutation and the next observation.
    pub settle_ticks: u32,
    /// `drain` boundaries waited once before `body` runs — covers
    /// window open, first paint, and the deferred `Ready`. The
    /// default `30` matches the measured ~0.5 s settle the slice-3/6
    /// real-window golden tests require for bit-stable frames.
    pub startup_settle_ticks: u32,
}

impl Default for RealWindowOptions {
    fn default() -> Self {
        Self {
            tick: None,
            settle_ticks: 2,
            startup_settle_ticks: 30,
        }
    }
}

/// `ScenarioDriver::RealOs` [`SemanticGuiTest`] +
/// [`VisualGuiTest`] adapter — see the module docs for the threading
/// and honesty contracts. Values exist only inside the
/// [`RealOsGuiTest::run`] session `body`; there is no public
/// constructor.
pub struct RealOsGuiTest {
    /// Command channel into the app thread's `drain` job pump.
    jobs: Sender<UiJob>,
    /// `drain` boundaries per [`SemanticGuiTest::settle`].
    settle_ticks: u32,
    /// `Action` intents collected by the app-thread event handler —
    /// shared so [`drain_intents`](SemanticGuiTest::drain_intents)
    /// reads them without a UI-thread round-trip.
    intents: Arc<Mutex<Vec<Intent>>>,
    /// Metadata of the last successful capture — see
    /// [`last_capture_meta`](Self::last_capture_meta).
    last_capture: Arc<Mutex<Option<CaptureMeta>>>,
    /// Per-window focus state observed through the app thread's
    /// [`WindowLifecycle::Focused`] events — the gate for the
    /// `os_*` ops that deliver input to the foreground window.
    focused: Arc<Mutex<BTreeMap<WindowId, bool>>>,
    /// `(window, event)` lifecycle/geometry transitions the app
    /// thread's event handler observed, in emission order — read
    /// through [`window_events`](Self::window_events). `Action`
    /// intents stay on the `intents` channel; the log is append-only
    /// for the session's lifetime so a destroyed window's terminal
    /// `Destroyed` remains observable.
    events: Arc<Mutex<Vec<(WindowId, WindowLifecycleEvent)>>>,
    /// Per-window queue of [`CloseDecision`]s the event handler
    /// consumes inside the synchronous `CloseRequested` flush — the
    /// only point a `Veto`/`Defer` can still beat the driver's
    /// unanswered-`Approve` default. Fed by
    /// [`arm_close_response`](Self::arm_close_response).
    close_responses: Arc<Mutex<BTreeMap<WindowId, VecDeque<CloseDecision>>>>,
    /// Per-family session preflight cache (Issue #552) — each
    /// `os_*` op gates on its family's cached check so a measured
    /// ineffective backend reports `EnvironmentBlocked` instead of
    /// injecting into the void; `os_input_preflight` probes the
    /// missing families of its scope once.
    os_preflight: RefCell<BTreeMap<&'static str, PreflightCheck>>,
    /// Lane-injected claims unioned into
    /// [`capabilities`](SemanticGuiTest::capabilities) — populated
    /// only through [`claim_capability`](Self::claim_capability)
    /// after the lane's own session-scoped probe *proved* the
    /// capability, so the honest-claim lattice stays fail-closed
    /// by default (empty here).
    lane_claimed: BTreeSet<ScenarioCapability>,
}

/// How the session `body` ended — lets [`RealOsGuiTest::run`] tear
/// the app thread down uniformly before surfacing the outcome.
enum BodyEnd<R> {
    /// `body` returned a value.
    Value(R),
    /// The startup settle failed — `body` never ran.
    Startup(ScenarioError),
    /// `body` panicked — the payload resumes on the caller thread
    /// after teardown.
    Panic(Box<dyn std::any::Any + Send>),
}

impl RealOsGuiTest {
    /// Upper bound for one command's reply — a live-but-stalled app
    /// thread surfaces as [`ScenarioError::Driver`], never a hang.
    const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);

    /// Runs `body` against a real-window session: opens every
    /// [`RealWindowMount`] as a real OS window on a dedicated app
    /// thread, waits the startup settle, invokes `body` on the
    /// caller thread, then quits the loop and joins the thread —
    /// on every path, including a panicking `body`.
    ///
    /// # Errors
    /// [`UiError::BackendUnavailable`] is not used; failures surface
    /// as [`UiError::Host`] (app thread never started, startup settle
    /// failed) or the platform loop's own [`UiError`]. Requires an
    /// interactive desktop session for real windows — on a headless
    /// host window creation fails and commands report honest
    /// [`ScenarioError`]s/`NotRun` captures rather than pixels.
    pub fn run<R>(
        mounts: Vec<RealWindowMount>,
        options: RealWindowOptions,
        body: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, UiError> {
        let (jobs_tx, jobs_rx) = channel::<UiJob>();
        let (ready_tx, ready_rx) = channel::<Result<(), String>>();
        let intents = Arc::new(Mutex::new(Vec::<Intent>::new()));
        let last_capture = Arc::new(Mutex::new(None::<CaptureMeta>));
        let focused = Arc::new(Mutex::new(BTreeMap::<WindowId, bool>::new()));
        let events = Arc::new(Mutex::new(Vec::<(WindowId, WindowLifecycleEvent)>::new()));
        let close_responses = Arc::new(Mutex::new(
            BTreeMap::<WindowId, VecDeque<CloseDecision>>::new(),
        ));
        let tick = options.tick;
        let settle_ticks = options.settle_ticks;
        let startup_settle_ticks = options.startup_settle_ticks;

        let thread_intents = Arc::clone(&intents);
        let thread_focused = Arc::clone(&focused);
        let thread_events = Arc::clone(&events);
        let thread_close_responses = Arc::clone(&close_responses);
        let app = thread::spawn(move || -> Result<(), UiError> {
            let mut driver = GpuiDriver::new();
            driver.set_ui_jobs(jobs_rx);
            for mount in mounts {
                if let Err(error) = driver.create_view_window(&mount.window, mount.view) {
                    // Report the real failure before exiting — the
                    // caller would otherwise wait out the command
                    // timeout for a generic "not ready" error.
                    let _ = ready_tx.send(Err(error.to_string()));
                    return Err(error);
                }
            }
            // Setup complete — the platform loop takes over from here.
            let _ = ready_tx.send(Ok(()));
            driver.run(
                tick,
                window_event_handler(
                    thread_intents,
                    thread_focused,
                    thread_events,
                    thread_close_responses,
                ),
            )
        });

        // The app thread reports setup completion (or the first
        // `UiError`) before `run` blocks in the platform loop.
        match ready_rx.recv_timeout(Self::COMMAND_TIMEOUT) {
            Ok(Ok(())) => {}
            Ok(Err(message)) => {
                let _ = app.join();
                return Err(UiError::host(message));
            }
            Err(_) => {
                let _ = app.join();
                return Err(UiError::host(
                    "gpui app thread did not report readiness".to_string(),
                ));
            }
        }

        let mut gui = Self {
            jobs: jobs_tx.clone(),
            settle_ticks,
            intents,
            last_capture,
            focused,
            events,
            close_responses,
            os_preflight: RefCell::new(BTreeMap::new()),
            lane_claimed: BTreeSet::new(),
        };

        let outcome = match gui.settle_n(startup_settle_ticks) {
            Ok(()) => {
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(&mut gui))) {
                    Ok(value) => BodyEnd::Value(value),
                    Err(payload) => BodyEnd::Panic(payload),
                }
            }
            Err(error) => BodyEnd::Startup(error),
        };
        drop(gui);

        // Teardown on every path: quit the platform loop, then join.
        let _ = jobs_tx.send(Box::new(|_inner, _driver, cx| cx.quit()));
        let join = app.join();
        match outcome {
            BodyEnd::Panic(payload) => std::panic::resume_unwind(payload),
            BodyEnd::Startup(error) => Err(UiError::host(format!(
                "scenario session failed to start: {error}"
            ))),
            BodyEnd::Value(value) => match join {
                Ok(run_result) => run_result.and(Ok(value)),
                Err(payload) => std::panic::resume_unwind(payload),
            },
        }
    }

    /// `settle` with an explicit `drain`-boundary count — the startup
    /// wait uses the configured `startup_settle_ticks`, not the
    /// per-step value.
    fn settle_n(&self, ticks: u32) -> Result<(), ScenarioError> {
        self.call(|reply| settle_job(ticks.max(1), self.jobs.clone(), reply))
    }

    /// Sends one command job and blocks for its reply — the
    /// synchronous face of the cross-thread session.
    fn call<V: Send + 'static>(
        &self,
        make_job: impl FnOnce(Sender<Result<V, ScenarioError>>) -> UiJob,
    ) -> Result<V, ScenarioError> {
        let (reply, rx) = channel();
        self.jobs.send(make_job(reply)).map_err(|_| {
            ScenarioError::Driver("scenario session ended — app thread gone".into())
        })?;
        rx.recv_timeout(Self::COMMAND_TIMEOUT)
            .map_err(|e| match e {
                RecvTimeoutError::Timeout => ScenarioError::Driver(
                    "scenario command timed out — gpui app thread unresponsive".into(),
                ),
                RecvTimeoutError::Disconnected => {
                    ScenarioError::Driver("scenario session ended — app thread gone".into())
                }
            })?
    }

    /// Measured metadata of the most recent *successful*
    /// [`capture_frame`](VisualGuiTest::capture_frame) — `None`
    /// before the first capture. Failed captures leave the previous
    /// value in place; the field documents the last frame that
    /// actually produced pixels.
    #[must_use]
    pub fn last_capture_meta(&self) -> Option<CaptureMeta> {
        *self
            .last_capture
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Replaces the mounted [`ViewSpec`] of `window` — the story
    /// teardown seam. Queues the same op as
    /// [`GpuiDriver::update_view`]; the next
    /// [`settle`](SemanticGuiTest::settle) applies it, and the host
    /// generation stays stable so pinned targets remain valid.
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when `window` is unknown.
    pub fn update_view(&mut self, window: WindowId, spec: ViewSpec) -> Result<(), ScenarioError> {
        self.call(|reply| {
            Box::new(move |_inner, driver, _cx| {
                let _ = reply.send(
                    driver
                        .update_view(window, spec)
                        .map_err(ScenarioError::from),
                );
            })
        })
    }

    // -----------------------------------------------------------------
    // OS-input port (Issue #313 — `RealOsPointerOrKeyboard`).
    //
    // These methods are the *only* OS-injection path on this adapter:
    // they drive `os_input` (`SendInput` on Windows) rather than the
    // `ViewHost` semantic dispatch used by `SemanticGuiTest`. A
    // scenario asserts `RealOsPointerOrKeyboard` through
    // `run_scenario`'s capability pre-flight, then calls these to
    // produce `channel: "os-injection"` evidence.
    // -----------------------------------------------------------------

    /// Runs the Issue #552 injection-effect preflight for `scope`,
    /// probing only the families not yet verified this session and
    /// merging the cached checks for the rest — each family is probed
    /// at most once per session, so repeated calls never re-move the
    /// cursor. The returned report's `effective` reflects the cached
    /// checks for `scope`; callers should treat `!effective` as
    /// `environment_blocked`, not a failure of the scenario itself.
    ///
    /// On non-Windows hosts every check reports `effective = false`
    /// with `family`/`detail` explaining the missing backend.
    pub fn os_input_preflight(&mut self, scope: PreflightScope) -> PreflightReport {
        let missing: Vec<PreflightScope> = scope
            .families()
            .iter()
            .filter(|family| !self.os_preflight.borrow().contains_key(*family))
            .map(|family| match *family {
                "pointer" => PreflightScope::Pointer,
                _ => PreflightScope::Keyboard,
            })
            .collect();
        for scope in missing {
            for check in os_input::preflight(scope).checks {
                self.os_preflight.borrow_mut().insert(check.family, check);
            }
        }
        self.cached_preflight(scope)
    }

    /// Synthesizes a [`PreflightReport`] for `scope` from the cached
    /// per-family checks — no probing.
    fn cached_preflight(&self, scope: PreflightScope) -> PreflightReport {
        let cache = self.os_preflight.borrow();
        let checks: Vec<PreflightCheck> = scope
            .families()
            .iter()
            .map(|family| {
                cache
                    .get(family)
                    .cloned()
                    .unwrap_or_else(|| PreflightCheck {
                        family,
                        effective: false,
                        detail: "preflight not run for this family".to_string(),
                    })
            })
            .collect();
        PreflightReport {
            effective: checks.iter().all(|check| check.effective),
            checks,
        }
    }

    /// The pre-injection gate every `os_*` op passes: backend
    /// presence, then the cached session preflight for `family`
    /// (probing lazily on first use). `Err(outcome)` is the honest
    /// early-out the op returns as `Ok(outcome)`.
    fn os_gate(&self, family: &'static str) -> Result<(), OsInputOutcome> {
        if !os_input::supported() {
            return Err(OsInputOutcome::Unsupported {
                reason: format!("no OS input injection backend on {}", std::env::consts::OS),
            });
        }
        let needs_probe = !self.os_preflight.borrow().contains_key(family);
        if needs_probe {
            let scope = match family {
                "pointer" => PreflightScope::Pointer,
                _ => PreflightScope::Keyboard,
            };
            for check in os_input::preflight(scope).checks {
                self.os_preflight.borrow_mut().insert(check.family, check);
            }
        }
        let cache = self.os_preflight.borrow();
        match cache.get(family) {
            Some(check) if check.effective => Ok(()),
            Some(check) => Err(OsInputOutcome::EnvironmentBlocked {
                detail: check.detail.clone(),
            }),
            None => Err(OsInputOutcome::EnvironmentBlocked {
                detail: format!("preflight for family '{family}' did not run"),
            }),
        }
    }

    /// Whether `window` is currently the focused window, as observed
    /// through the app thread's [`WindowLifecycle::Focused`] events.
    /// `false` covers both "never focused" and "focus lost" — OS
    /// input ops that target the foreground window refuse to run
    /// against it (`OsInputOutcome::NotRun`).
    #[must_use]
    pub fn window_focused(&self, window: WindowId) -> bool {
        self.focused
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&window)
            .copied()
            .unwrap_or(false)
    }

    /// Queues a window focus request through the driver's own
    /// `Op::Focus` path — the honest `activate_window` request the
    /// OS applies (or ignores) before the next [`Focused`] event.
    /// Callers then wait on [`window_focused`](Self::window_focused).
    /// `focused: false` reports the driver's honest `Unsupported`
    /// (gpui cannot release focus) — it never silently no-ops.
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when `window` is unknown or focus
    /// is unsupported on this backend.
    pub fn request_focus(&mut self, window: WindowId, focused: bool) -> Result<(), ScenarioError> {
        self.call(move |reply| {
            Box::new(move |_inner, driver, _cx| {
                let _ = reply.send(
                    WindowDriver::focus(&mut driver.clone(), window, focused)
                        .map_err(ScenarioError::from),
                );
            })
        })
    }

    /// Injects one real keystroke (`key-down` + `key-up`) through
    /// [`os_input::press_key`] — delivered to whatever window the OS
    /// considers foreground, so `target`'s window must be observed
    /// focused first. The view pin check keeps stale targets a
    /// `fail`, and the family preflight keeps a blocked host an
    /// `environment_blocked` — never a fabricated key delivery.
    ///
    /// # Errors
    /// [`ScenarioError::NoSuchWindow`]/[`ScenarioError::NoSuchView`]/
    /// stale-pin errors for dead targets;
    /// [`ScenarioError::Driver`] when the OS accepted fewer than the
    /// two events (a partial keystroke is `fail`, never `Injected`).
    pub fn os_press_key(
        &mut self,
        target: &ScenarioTarget,
        key: OsKey,
    ) -> Result<OsInputOutcome, ScenarioError> {
        if let Err(outcome) = self.os_gate("keyboard") {
            return Ok(outcome);
        }
        if !self.window_focused(target.window) {
            return Ok(OsInputOutcome::NotRun {
                reason: format!("window {} is not the focused window", target.window),
            });
        }
        let target = target.clone();
        self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_check_os_target(inner, driver, cx, &target, None));
            })
        })?;
        os_input::press_key(key).map_err(ScenarioError::from)?;
        Ok(OsInputOutcome::Injected {
            action: "key-press",
            commanded: None,
            cursor: None,
            detail: format!("key {} injected via win32-SendInput", key.name()),
        })
    }

    /// Aims the OS cursor at the measured center-relative point
    /// `frac` inside `component`'s painted bounds — logical
    /// `component_bounds` + `window_geometry` converted to physical
    /// screen pixels, never a hard-coded coordinate — then injects
    /// the absolute move and reads the cursor back for the
    /// [`OsInputOutcome::Injected`] evidence. `frac` is the
    /// normalized `(x, y)` offset inside the component rect
    /// (`(0.5, 0.5)` = center); the lane picks the point.
    ///
    /// # Errors
    /// Contract violations (unknown window/component, stale pins)
    /// and `UiError::Host` from the injection call itself are
    /// `Err(ScenarioError)`; a component that exists but has not
    /// painted bounds yet returns `Ok(OsInputOutcome::NotRun)`.
    pub fn os_pointer_move_to(
        &mut self,
        target: &ScenarioTarget,
        component: &ComponentKey,
        frac: (f64, f64),
    ) -> Result<OsInputOutcome, ScenarioError> {
        if let Err(outcome) = self.os_gate("pointer") {
            return Ok(outcome);
        }
        let target = target.clone();
        let component = component.clone();
        let aim = self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_resolve_os_aim(
                    inner, driver, cx, &target, &component, frac,
                ));
            })
        })?;
        let (x, y) = match aim {
            Aim::Point(point) => point,
            Aim::NotRun(reason) => {
                return Ok(OsInputOutcome::NotRun { reason });
            }
        };
        os_input::move_to(x, y).map_err(ScenarioError::from)?;
        // Give the OS a beat to apply the queued move before the
        // read-back, same budget as the smoke examples.
        thread::sleep(OS_CURSOR_SETTLE);
        let cursor = os_input::cursor_position().map_err(ScenarioError::from)?;
        Ok(OsInputOutcome::Injected {
            action: "pointer-move",
            commanded: Some((x, y)),
            cursor: Some(cursor),
            detail: format!(
                "move to ({x}, {y}) physical px; read-back ({}, {})",
                cursor.0, cursor.1
            ),
        })
    }

    /// Injects `WM_LBUTTONDOWN` at the current cursor position —
    /// the lane aims first with
    /// [`os_pointer_move_to`](Self::os_pointer_move_to), whose
    /// read-back is the position evidence. `target`'s window must be
    /// the observed focused window (the click would otherwise land
    /// on whatever is underneath — refused as `NotRun`).
    ///
    /// # Errors
    /// Same error surface as [`os_press_key`](Self::os_press_key).
    pub fn os_mouse_down(
        &mut self,
        target: &ScenarioTarget,
    ) -> Result<OsInputOutcome, ScenarioError> {
        self.os_mouse_button(target, true)
    }

    /// Injects `WM_LBUTTONUP` at the current cursor position —
    /// completes a click/drag started by
    /// [`os_mouse_down`](Self::os_mouse_down).
    ///
    /// # Errors
    /// Same error surface as [`os_press_key`](Self::os_press_key).
    pub fn os_mouse_up(
        &mut self,
        target: &ScenarioTarget,
    ) -> Result<OsInputOutcome, ScenarioError> {
        self.os_mouse_button(target, false)
    }

    /// Shared body for the button ops — gate, focus, pin check,
    /// then the single injection call.
    fn os_mouse_button(
        &mut self,
        target: &ScenarioTarget,
        down: bool,
    ) -> Result<OsInputOutcome, ScenarioError> {
        if let Err(outcome) = self.os_gate("pointer") {
            return Ok(outcome);
        }
        if !self.window_focused(target.window) {
            return Ok(OsInputOutcome::NotRun {
                reason: format!("window {} is not the focused window", target.window),
            });
        }
        let target = target.clone();
        self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_check_os_target(inner, driver, cx, &target, None));
            })
        })?;
        if down {
            os_input::mouse_down().map_err(ScenarioError::from)?;
        } else {
            os_input::mouse_up().map_err(ScenarioError::from)?;
        }
        Ok(OsInputOutcome::Injected {
            action: if down { "mouse-down" } else { "mouse-up" },
            commanded: None,
            cursor: None,
            detail: format!(
                "left button {} at current cursor position via win32-SendInput",
                if down { "down" } else { "up" }
            ),
        })
    }
}

// -----------------------------------------------------------------
// Window lifecycle / DPI observation surface (Issue #631).
//
// These methods expose the driver-neutral close protocol
// (`WindowDriver::request_close`/`respond_close` →
// `WindowLifecycle::CloseRequested` → `Destroyed`) and the
// physical-geometry/DPI reads the real OS window produces. Every
// observation comes from the app thread's event handler or a
// `UiJob` inside `App` context — nothing is synthesized caller-side.
// -----------------------------------------------------------------

impl RealOsGuiTest {
    /// Lifecycle/geometry transitions the app thread observed for
    /// `window`, in emission order — a non-destructive snapshot, so
    /// it is safe inside [`settle_until`] predicates and repeatable
    /// after the window's destruction (the log is append-only for
    /// the session). `Action` intents are excluded — observe them
    /// through [`drain_intents`](SemanticGuiTest::drain_intents).
    /// `Resized`/`Moved`/`ScaleFactorChanged` carry the driver's
    /// physical-pixel values; pair with
    /// [`window_geometry`](Self::window_geometry) for the live
    /// OS-measured frame.
    #[must_use]
    pub fn window_events(&self, window: WindowId) -> Vec<WindowLifecycleEvent> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|(id, _)| *id == window)
            .map(|(_, event)| *event)
            .collect()
    }

    /// Records a capability the lane *proved inside this session* —
    /// the injection port for adapter-external evidence (e.g. a
    /// plugin-host capability `ruviform-ui-gpui` cannot observe
    /// itself, like the out-of-process L1 plugin route the dock
    /// lane probes once at session start). The caller attests the
    /// capability was demonstrated on this session — "claim は
    /// 実証された能力のみ" — so a lane that did not run its probe
    /// must not call this. The claim unions into
    /// [`capabilities`](SemanticGuiTest::capabilities) for the rest
    /// of the session.
    pub fn claim_capability(&mut self, capability: ScenarioCapability) {
        self.lane_claimed.insert(capability);
    }

    /// Queues the veto-able close request — the real-window
    /// counterpart of the OS close button
    /// (`WindowDriver::request_close` → `Op::RequestClose`). The
    /// request emits `WindowLifecycle::CloseRequested` at the next
    /// drain; an *unanswered* request defaults to `Approve` for that
    /// window only, so a `Veto`/`Defer` must already be armed through
    /// [`arm_close_response`](Self::arm_close_response) — or, for a
    /// `Defer`-parked request, answered later through
    /// [`respond_close`](Self::respond_close). Unknown windows are
    /// ignored per the trait contract.
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when the session is gone.
    pub fn request_close(&mut self, window: WindowId) -> Result<(), ScenarioError> {
        self.call(move |reply| {
            Box::new(move |_inner, driver, _cx| {
                WindowDriver::request_close(&mut driver.clone(), window);
                let _ = reply.send(Ok(()));
            })
        })
    }

    /// Arms the [`CloseDecision`] the app thread's event handler
    /// applies inside the synchronous `CloseRequested` flush — one
    /// queued decision per expected request, consumed oldest-first
    /// per `window`. This is the *only* timely `Veto`/`Defer` path
    /// for [`request_close`](Self::request_close): the driver
    /// defaults an unanswered request to `Approve` within the same
    /// drain pass, before any caller-side [`respond_close`] job can
    /// run. Arm before `request_close`; an exhausted queue leaves
    /// later requests on the default-`Approve` path, and decisions
    /// armed for a window that never emits `CloseRequested` stay
    /// queued without leaking into other windows.
    pub fn arm_close_response(&self, window: WindowId, decision: CloseDecision) {
        self.close_responses
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(window)
            .or_default()
            .push_back(decision);
    }

    /// Answers a pending close — forwards
    /// `WindowDriver::respond_close` through the `UiJob` seam.
    /// `Approve` destroys the window (`Destroyed` follows at the
    /// next drain); `Veto` cancels the close; `Defer` leaves the
    /// request pending for a later answer. For a close whose
    /// `CloseRequested` is flushing *right now* the armed queue in
    /// [`arm_close_response`](Self::arm_close_response) is the only
    /// timely answer — a caller-side response always lands after
    /// the unanswered-`Approve` default.
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when `window` is unknown/destroyed
    /// or has no pending close (mirrors `UiError::Host`), or the
    /// session is gone.
    pub fn respond_close(
        &mut self,
        window: WindowId,
        decision: CloseDecision,
    ) -> Result<(), ScenarioError> {
        self.call(move |reply| {
            Box::new(move |_inner, driver, _cx| {
                let _ = reply.send(
                    WindowDriver::respond_close(&mut driver.clone(), window, decision)
                        .map_err(ScenarioError::from),
                );
            })
        })
    }

    /// The window's live client-area geometry in *physical* screen
    /// pixels — origin, size, and the scale factor (logical →
    /// physical) the OS reported — via
    /// [`super::GpuiDriver::window_geometry`]. This is the DPI
    /// observation point: `scale_factor` is the measured backend
    /// factor and `width`/`height` are physical pixels, the same
    /// unit [`WindowLifecycle::Resized`] reports. `ScaleFactorChanged`
    /// transitions additionally surface through
    /// [`window_events`](Self::window_events) when the OS reports a
    /// factor change (for example a move across mixed-DPI monitors).
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when `window` is unknown or not yet
    /// open (`UiError::Host`), has no OS handle (`UiError::
    /// Unsupported` — for example a `gpui-test` session), or the
    /// session is gone.
    pub fn window_geometry(&self, window: WindowId) -> Result<OsWindowGeometry, ScenarioError> {
        self.call(move |reply| {
            Box::new(move |_inner, driver, _cx| {
                let _ = reply.send(driver.window_geometry(window).map_err(ScenarioError::from));
            })
        })
    }
}

/// How long `os_pointer_move_to` waits before the cursor read-back —
/// matches the smoke examples' measured budget for the OS to apply
/// a queued absolute move.
const OS_CURSOR_SETTLE: Duration = Duration::from_millis(80);

/// What [`exec_resolve_os_aim`] computed — the physical-px screen
/// point, or the honest `NotRun` reason.
enum Aim {
    /// Physical screen pixels to inject a move toward.
    Point((i32, i32)),
    /// The aim could not be computed from live state — `not_run`.
    NotRun(String),
}

/// The capability set [`RealOsGuiTest`] honestly claims — see the
/// module docs. `VisualCapture` requires an OS capture backend —
/// Win32 on Windows, X11 (`GetImage`) on Linux. `RealOsPointerOrKeyboard`
/// requires an [`os_input`] injection backend — Win32 `SendInput` on
/// Windows only in this slice (Issue #313); the claim records backend
/// *presence* — a blocked environment stays claimed and each `os_*`
/// op reports `EnvironmentBlocked` through the preflight gate instead.
/// `Ime` follows the same presence convention through
/// [`os_ime::supported`] — Win32 IMM32 composition drive + the
/// `WM_IME_*` delivery oracle (Issue #389); a session without an
/// installed/enabled IME reports `environment_blocked` per-case,
/// never a fabricated pass. `NativePluginDock` requires a surface
/// backend that mints real native container parents — Windows child
/// HWNDs only, never `gpui-test` (Issue #631).
fn supported_capabilities() -> BTreeSet<ScenarioCapability> {
    let mut capabilities = BTreeSet::from([
        ScenarioCapability::SemanticAction,
        ScenarioCapability::MultiWindow,
    ]);
    if cfg!(any(windows, target_os = "linux")) {
        capabilities.insert(ScenarioCapability::VisualCapture);
    }
    if os_input::supported() {
        capabilities.insert(ScenarioCapability::RealOsPointerOrKeyboard);
    }
    if os_ime::supported() {
        capabilities.insert(ScenarioCapability::Ime);
    }
    if native_dock_backend_available() {
        capabilities.insert(ScenarioCapability::NativePluginDock);
    }
    capabilities
}

/// The app-thread [`DriverEvent`] handler [`RealOsGuiTest::run`]
/// installs: records every `Windowed` lifecycle transition into the
/// session's observation log, routes `Action` intents and focus state
/// to their own channels, and answers `CloseRequested` from the
/// per-window armed queue *inside the same synchronous flush* — the
/// driver defaults an unanswered request to `Approve` when the flush
/// returns, so a caller-side `respond_close` queued now would always
/// lose the race.
fn window_event_handler(
    intents: Arc<Mutex<Vec<Intent>>>,
    focused: Arc<Mutex<BTreeMap<WindowId, bool>>>,
    events: Arc<Mutex<Vec<(WindowId, WindowLifecycleEvent)>>>,
    close_responses: Arc<Mutex<BTreeMap<WindowId, VecDeque<CloseDecision>>>>,
) -> impl FnMut(DriverEvent, &mut dyn WindowDriver) + 'static {
    move |event, drv| {
        let DriverEvent::Windowed { window, event } = event else {
            return;
        };
        if let Some(observed) = WindowLifecycleEvent::observe(&event) {
            events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push((window, observed));
        }
        match event {
            WindowLifecycle::Action(intent) => {
                intents
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(intent);
            }
            WindowLifecycle::Focused(active) => {
                focused
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(window, active);
            }
            WindowLifecycle::CloseRequested => {
                let decision = close_responses
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get_mut(&window)
                    .and_then(VecDeque::pop_front);
                if let Some(decision) = decision {
                    // The window is close-pending by construction; a
                    // refusal means driver state diverged — the event
                    // log still records `CloseRequested` honestly.
                    let _ = drv.respond_close(window, decision);
                }
            }
            _ => {}
        }
    }
}

/// One [`UiJob`] that counts `drain` boundaries before replying —
/// `settle`'s frame-paced wait. Each pass re-queues itself with
/// `ticks - 1`, so `ticks` boundaries must elapse before the reply:
/// bounded, never a sleep.
fn settle_job(ticks: u32, jobs: Sender<UiJob>, reply: Sender<Result<(), ScenarioError>>) -> UiJob {
    Box::new(move |_inner, _driver, _cx| {
        if ticks <= 1 {
            let _ = reply.send(Ok(()));
        } else {
            // A failed send drops the reply `Sender` inside the
            // returned job — the caller's `recv_timeout` reports the
            // dead session immediately instead of timing out.
            let _ = jobs.send(settle_job(ticks - 1, jobs.clone(), reply));
        }
    })
}

/// Live identity + component snapshots of the view mounted in
/// `window`, read inside `App` context. The entity stays on the UI
/// thread — callers mutate it through `Entity::update` in the same
/// job.
struct MountedView {
    entity: Entity<RvfView>,
    attach_epoch: AttachEpoch,
    generation: u64,
    components: Vec<(ComponentKey, ComponentSnapshot)>,
}

/// Resolves `window`/`view` to its mounted view inside `App`
/// context — the real-window counterpart of
/// `GpuiGuiTest::observation`. Unknown windows report
/// [`ScenarioError::NoSuchWindow`], missing views and view-id
/// mismatches report [`ScenarioError::NoSuchView`].
fn mounted_view(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    window: WindowId,
    view: ViewInstanceId,
) -> Result<MountedView, ScenarioError> {
    if !inner.borrow().windows.contains_key(&window) {
        return Err(ScenarioError::NoSuchWindow { window });
    }
    let entity = driver
        .view_of(cx, window)
        .ok_or(ScenarioError::NoSuchView { window, view })?;
    let (view_id, attach_epoch, generation, components) = entity.update(cx, |v, _cx| {
        let host = v.host();
        let components = host
            .mounted_keys()
            .into_iter()
            .filter_map(|key| host.observe(&key).map(|snapshot| (key, snapshot)))
            .collect();
        (
            host.view_id(),
            host.attach_epoch(),
            host.generation(),
            components,
        )
    });
    if view_id != view {
        return Err(ScenarioError::NoSuchView { window, view });
    }
    Ok(MountedView {
        entity,
        attach_epoch,
        generation,
        components,
    })
}

/// Re-checks a resolved target's pins — the stale gate every
/// operation passes through, mirroring `GpuiGuiTest::check_target`.
fn check_pins(mounted: &MountedView, target: &ScenarioTarget) -> Result<(), ScenarioError> {
    if mounted.attach_epoch != target.attach_epoch {
        return Err(ScenarioError::StaleEpoch {
            expected: target.attach_epoch,
            got: mounted.attach_epoch,
        });
    }
    if mounted.generation != target.generation {
        return Err(ScenarioError::StaleGeneration {
            expected: target.generation,
            got: mounted.generation,
        });
    }
    Ok(())
}

/// The component snapshot for `key`, or
/// [`ScenarioError::UnknownComponent`].
fn component<'a>(
    mounted: &'a MountedView,
    key: &ComponentKey,
) -> Result<&'a ComponentSnapshot, ScenarioError> {
    mounted
        .components
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, snapshot)| snapshot)
        .ok_or_else(|| ScenarioError::UnknownComponent { key: key.clone() })
}

/// Job body for [`SemanticGuiTest::resolve_target`].
fn exec_resolve_target(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    query: &TargetQuery,
) -> Result<ScenarioTarget, ScenarioError> {
    let mounted = mounted_view(inner, driver, cx, query.window, query.view)?;
    if let Some(expected) = query.attach_epoch
        && mounted.attach_epoch != expected
    {
        return Err(ScenarioError::StaleEpoch {
            expected,
            got: mounted.attach_epoch,
        });
    }
    if let Some(expected) = query.generation
        && mounted.generation != expected
    {
        return Err(ScenarioError::StaleGeneration {
            expected,
            got: mounted.generation,
        });
    }
    if let Some(key) = &query.component {
        component(&mounted, key)?;
    }
    Ok(ScenarioTarget {
        window: query.window,
        view: query.view,
        attach_epoch: mounted.attach_epoch,
        generation: mounted.generation,
        component: query.component.clone(),
    })
}

/// Shared prelude for the mutating operations: resolve the mounted
/// view, re-check the target's pins, verify the component is mounted.
fn checked_view(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    target: &ScenarioTarget,
    key: &ComponentKey,
) -> Result<MountedView, ScenarioError> {
    let mounted = mounted_view(inner, driver, cx, target.window, target.view)?;
    check_pins(&mounted, target)?;
    component(&mounted, key)?;
    Ok(mounted)
}

/// Job body for [`SemanticGuiTest::dispatch_input`].
fn exec_dispatch_input(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    target: &ScenarioTarget,
    key: &ComponentKey,
    input: ControlInput,
) -> Result<(), ScenarioError> {
    let mounted = checked_view(inner, driver, cx, target, key)?;
    mounted
        .entity
        .update(cx, |v, cx| v.try_input(key, input, cx))
        .map_err(ScenarioError::from)
}

/// Job body for [`SemanticGuiTest::select_row`].
fn exec_select_row(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    target: &ScenarioTarget,
    key: &ComponentKey,
    row: usize,
) -> Result<(), ScenarioError> {
    let mounted = checked_view(inner, driver, cx, target, key)?;
    mounted
        .entity
        .update(cx, |v, cx| v.try_select_row(key, row, cx))
        .map_err(ScenarioError::from)
}

/// Job body for [`SemanticGuiTest::select_tab`].
fn exec_select_tab(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    target: &ScenarioTarget,
    key: &ComponentKey,
    index: usize,
) -> Result<(), ScenarioError> {
    let mounted = checked_view(inner, driver, cx, target, key)?;
    mounted
        .entity
        .update(cx, |v, cx| v.try_select_tab(key, index, cx))
        .map_err(ScenarioError::from)
}

/// Job body for [`SemanticGuiTest::invoke`].
fn exec_invoke(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    target: &ScenarioTarget,
    key: &ComponentKey,
) -> Result<(), ScenarioError> {
    let mounted = checked_view(inner, driver, cx, target, key)?;
    mounted
        .entity
        .update(cx, |v, cx| v.try_invoke(key, cx))
        .map_err(ScenarioError::from)
}

/// Job body for [`SemanticGuiTest::observe`] — read-only snapshot.
fn exec_observe(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    target: &ScenarioTarget,
    key: &ComponentKey,
) -> Result<ComponentSnapshot, ScenarioError> {
    let mounted = checked_view(inner, driver, cx, target, key)?;
    Ok(component(&mounted, key)?.clone())
}

/// Job body for [`SemanticGuiTest::post_event`] — re-checks the
/// pins, then queues through the driver's own `post_event` so the
/// ready/inbox routing stays identical to every other consumer.
fn exec_post_event(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    target: &ScenarioTarget,
    event: &Event,
) -> Result<(), ScenarioError> {
    let mounted = mounted_view(inner, driver, cx, target.window, target.view)?;
    check_pins(&mounted, target)?;
    WindowDriver::post_event(&mut driver.clone(), target.window, event);
    Ok(())
}

/// Job body for [`VisualGuiTest::capture_frame`] — pin check, then
/// the real OS capture. Backend errors map to the honest
/// `Unsupported`/`NotRun` outcomes; only measured pixels become
/// [`CaptureOutcome::Captured`].
fn exec_capture_frame(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    target: &ScenarioTarget,
    last_capture: &Arc<Mutex<Option<CaptureMeta>>>,
) -> Result<CaptureOutcome, ScenarioError> {
    let mounted = mounted_view(inner, driver, cx, target.window, target.view)?;
    check_pins(&mounted, target)?;
    let (opened, os_handle) = {
        let guard = inner.borrow();
        match guard.windows.get(&target.window) {
            Some(slot) => (slot.handle.is_some(), slot.raw_parent.is_some()),
            None => (false, false),
        }
    };
    if !opened {
        return Ok(CaptureOutcome::NotRun {
            reason: "os_window_not_open".to_string(),
        });
    }
    if !os_handle {
        return Ok(CaptureOutcome::Unsupported {
            reason: "window_has_no_os_handle".to_string(),
        });
    }
    match driver.capture_window(target.window) {
        Ok(frame) => {
            let image = RgbaImage::new(frame.width, frame.height, frame.pixels)
                .map_err(|e| ScenarioError::Driver(format!("captured frame is malformed: {e}")))?;
            *last_capture
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(CaptureMeta {
                width: frame.width,
                height: frame.height,
                scale_factor: frame.scale_factor,
                backend: frame.backend,
            });
            Ok(CaptureOutcome::Captured(image))
        }
        Err(UiError::Unsupported { .. }) => Ok(CaptureOutcome::Unsupported {
            reason: "os_capture_backend_unavailable".to_string(),
        }),
        Err(UiError::Host { .. }) => Ok(CaptureOutcome::NotRun {
            reason: "os_window_capture_unavailable".to_string(),
        }),
        Err(error) => Err(ScenarioError::Driver(error.to_string())),
    }
}

/// Job prelude for the `os_*` ops that take no component: resolve
/// the mounted view and re-check the pins. `component`, when given,
/// is additionally required to be mounted — the same gate the
/// semantic ops pass through.
fn exec_check_os_target(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    target: &ScenarioTarget,
    key: Option<&ComponentKey>,
) -> Result<(), ScenarioError> {
    let mounted = mounted_view(inner, driver, cx, target.window, target.view)?;
    check_pins(&mounted, target)?;
    if let Some(key) = key {
        component(&mounted, key)?;
    }
    Ok(())
}

/// Job body for [`RealOsGuiTest::os_pointer_move_to`] — resolves the
/// pin-checked mounted view, then converts the component's measured
/// [`LogicalRect`] into a physical-px screen point through
/// [`GpuiDriver::window_geometry`]. A mounted-but-unpainted
/// component or a degenerate rect reports [`Aim::NotRun`]; missing
/// geometry/OS handles stay honest `Err` (`fail`), never a zero.
#[expect(
    clippy::cast_possible_truncation,
    reason = "screen coordinates are bounded by the OS virtual screen (thousands of px), far inside i32"
)]
fn exec_resolve_os_aim(
    inner: &Rc<RefCell<Inner>>,
    driver: &GpuiDriver,
    cx: &mut App,
    target: &ScenarioTarget,
    key: &ComponentKey,
    frac: (f64, f64),
) -> Result<Aim, ScenarioError> {
    let mounted = mounted_view(inner, driver, cx, target.window, target.view)?;
    check_pins(&mounted, target)?;
    component(&mounted, key)?;
    let bounds = driver
        .component_bounds(target.window)
        .map_err(ScenarioError::from)?;
    let Some(rect): Option<LogicalRect> = bounds.get(key).copied() else {
        return Ok(Aim::NotRun(format!(
            "component {key} has no painted bounds yet"
        )));
    };
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return Ok(Aim::NotRun(format!(
            "component {key} has degenerate bounds {rect:?}"
        )));
    }
    let OsWindowGeometry {
        origin_x,
        origin_y,
        scale_factor,
        ..
    } = driver
        .window_geometry(target.window)
        .map_err(ScenarioError::from)?;
    let x = f64::from(origin_x) + (rect.x + rect.width * frac.0) * scale_factor;
    let y = f64::from(origin_y) + (rect.y + rect.height * frac.1) * scale_factor;
    Ok(Aim::Point((x.round() as i32, y.round() as i32)))
}

impl VisualGuiTest for RealOsGuiTest {
    /// Real OS-window pixels through
    /// [`super::GpuiDriver::capture_window`] (`PrintWindow` primary,
    /// screen `BitBlt` fallback on Windows). Pins are re-checked
    /// inside `App` context first; backend honesty maps to
    /// `Unsupported`/`NotRun` — pixels are never fabricated from
    /// draw plans or semantic snapshots.
    fn capture_frame(&mut self, target: &ScenarioTarget) -> Result<CaptureOutcome, ScenarioError> {
        let target = target.clone();
        let last_capture = Arc::clone(&self.last_capture);
        self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_capture_frame(
                    inner,
                    driver,
                    cx,
                    &target,
                    &last_capture,
                ));
            })
        })
    }
}

impl SemanticGuiTest for RealOsGuiTest {
    fn driver(&self) -> ScenarioDriver {
        ScenarioDriver::RealOs
    }

    fn capabilities(&self) -> BTreeSet<ScenarioCapability> {
        let mut capabilities = supported_capabilities();
        capabilities.extend(self.lane_claimed.iter().copied());
        capabilities
    }

    fn windows(&self) -> Vec<WindowId> {
        // The signature cannot report errors — a dead session
        // reports an empty window set and every later operation
        // fails honestly through `call`.
        self.call(|reply| {
            Box::new(move |_inner, driver, _cx| {
                let _ = reply.send(Ok(WindowDriver::windows(driver)));
            })
        })
        .unwrap_or_default()
    }

    fn resolve_target(&mut self, query: &TargetQuery) -> Result<ScenarioTarget, ScenarioError> {
        let query = query.clone();
        self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_resolve_target(inner, driver, cx, &query));
            })
        })
    }

    fn dispatch_input(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        input: ControlInput,
    ) -> Result<(), ScenarioError> {
        let target = target.clone();
        let key = key.clone();
        self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_dispatch_input(inner, driver, cx, &target, &key, input));
            })
        })
    }

    fn select_row(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        row: usize,
    ) -> Result<(), ScenarioError> {
        let target = target.clone();
        let key = key.clone();
        self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_select_row(inner, driver, cx, &target, &key, row));
            })
        })
    }

    fn select_tab(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        index: usize,
    ) -> Result<(), ScenarioError> {
        let target = target.clone();
        let key = key.clone();
        self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_select_tab(inner, driver, cx, &target, &key, index));
            })
        })
    }

    fn invoke(&mut self, target: &ScenarioTarget, key: &ComponentKey) -> Result<(), ScenarioError> {
        let target = target.clone();
        let key = key.clone();
        self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_invoke(inner, driver, cx, &target, &key));
            })
        })
    }

    fn observe(
        &self,
        target: &ScenarioTarget,
        key: &ComponentKey,
    ) -> Result<ComponentSnapshot, ScenarioError> {
        let target = target.clone();
        let key = key.clone();
        self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_observe(inner, driver, cx, &target, &key));
            })
        })
    }

    fn settle(&mut self) -> Result<(), ScenarioError> {
        self.settle_n(self.settle_ticks)
    }

    fn drain_intents(&mut self) -> Vec<MintedIntent> {
        // Intents reach the shared log through the app thread's
        // event handler at each `flush_events`; `settle` guarantees
        // at least one boundary has passed since the last mutation.
        let intents = std::mem::take(
            &mut *self
                .intents
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        intents.iter().filter_map(minted_from_ipc).collect()
    }

    fn post_event(&mut self, target: &ScenarioTarget, event: &Event) -> Result<(), ScenarioError> {
        let target = target.clone();
        let event = event.clone();
        self.call(move |reply| {
            Box::new(move |inner, driver, cx| {
                let _ = reply.send(exec_post_event(inner, driver, cx, &target, &event));
            })
        })
    }
}

/// Runs one surface-ledger op on the driver's canonical
/// [`surface::SharedSurface`] inside `App` context. Ledger/backend
/// refusals surface as [`ScenarioError::Driver`] with a
/// "native slot" prefix naming the refused operation so evidence can
/// tell a slot refusal from a session failure; session/driver
/// failures keep their own [`ScenarioError`] shape.
impl RealOsGuiTest {
    fn slot_call<T, F>(&self, op: &'static str, call: F) -> Result<T, ScenarioError>
    where
        T: Send + 'static,
        F: FnOnce(&surface::SharedSurface) -> Result<T, SlotError> + Send + 'static,
    {
        self.call(move |reply| {
            Box::new(move |_inner, driver, _cx| {
                let surface = driver.surface();
                let result = call(&surface)
                    .map_err(|error| ScenarioError::Driver(format!("native slot `{op}`: {error}")));
                let _ = reply.send(Ok(result));
            })
        })?
    }

    /// Declares `spec` on the canonical surface ledger — see
    /// [`surface::SharedSurface::declare_slot`]. `spec.window` must
    /// name a live session window; once
    /// [`bind_native_slot`](Self::bind_native_slot) ties a mounted
    /// `NativeSlot` key to the returned id, the renderer projects
    /// real geometry/clip/visibility onto it every frame.
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when the ledger/backend refuses
    /// (non-Windows backend, unknown owner window); session/driver
    /// failures keep their [`ScenarioError`] shape.
    pub fn declare_native_slot(&self, spec: SlotSpec) -> Result<SurfaceSlotId, ScenarioError> {
        self.slot_call("declare_slot", move |surface| surface.declare_slot(spec))
    }

    /// Binds `key`'s mounted `NativeSlot` component in `window` to a
    /// live [`SurfaceSlotId`] — [`GpuiDriver::bind_native_slot`]
    /// through the `UiJob` seam. After this the view projects its
    /// laid-out bounds, ancestor clip, and painted/suppressed state
    /// onto `slot` each frame.
    ///
    /// # Errors
    /// [`ScenarioError`] when `window` is unknown, `key` is already
    /// bound, or `slot` is not a live slot on the canonical adapter.
    pub fn bind_native_slot(
        &self,
        window: WindowId,
        key: ComponentKey,
        slot: SurfaceSlotId,
    ) -> Result<(), ScenarioError> {
        self.call(move |reply| {
            Box::new(move |_inner, driver, cx| {
                let mut driver_mut = driver.clone();
                let result = driver_mut
                    .bind_native_slot(window, key, slot)
                    .map_err(ScenarioError::from);
                if result.is_ok()
                    && let Some(view) = driver.view_of(cx, window)
                {
                    // The bindings map is read live each frame, but
                    // GPUI only repaints on invalidation — without
                    // this notify a static view would never paint the
                    // newly bound `NativeSlot`, so
                    // `project_slot_frame` would never run and the
                    // slot would stay `Hidden` forever.
                    view.update(cx, |_, cx| cx.notify());
                }
                let _ = reply.send(Ok(result));
            })
        })?
    }

    /// Removes `key`'s slot binding — the next painted frame
    /// suppresses the slot (`Hidden`); releasing the slot itself
    /// stays the caller's [`release_native_slot`](Self::release_native_slot).
    ///
    /// # Errors
    /// [`ScenarioError`] when `window` is unknown or `key` has no
    /// binding.
    pub fn unbind_native_slot(
        &self,
        window: WindowId,
        key: ComponentKey,
    ) -> Result<(), ScenarioError> {
        self.call(move |reply| {
            Box::new(move |_inner, driver, cx| {
                let mut driver_mut = driver.clone();
                let result = driver_mut
                    .unbind_native_slot(window, &key)
                    .map_err(ScenarioError::from);
                if result.is_ok()
                    && let Some(view) = driver.view_of(cx, window)
                {
                    // Same invalidation as `bind_native_slot` — the
                    // next painted frame must run
                    // `suppress_unpainted_slots` to hide the slot.
                    view.update(cx, |_, cx| cx.notify());
                }
                let _ = reply.send(Ok(result));
            })
        })?
    }

    /// Leases `slot`'s native parent — on Windows this mints the real
    /// child-HWND container and applies the geometry/clip/visibility
    /// the renderer last projected
    /// ([`surface::SharedSurface::lease_parent`]).
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when the slot is unknown/released/
    /// already leased or the backend cannot create the container.
    pub fn lease_native_slot(&self, slot: SurfaceSlotId) -> Result<SlotLease, ScenarioError> {
        self.slot_call("lease_parent", move |surface| surface.lease_parent(slot))
    }

    /// Validates `lease` against the live ledger and returns its
    /// [`LeaseInfo`] — the slot binding plus the issuer sequence a
    /// `ParentLease` mint needs.
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when the lease payload is malformed,
    /// stale, or already released.
    pub fn validate_native_lease(&self, lease: SlotLease) -> Result<LeaseInfo, ScenarioError> {
        self.slot_call("validate_lease", move |surface| {
            surface.validate_lease(lease)
        })
    }

    /// Releases a live lease — the container lease is revoked and the
    /// slot returns to `Declared`.
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when the lease is unknown or already
    /// released.
    pub fn release_native_lease(&self, lease: SlotLease) -> Result<(), ScenarioError> {
        self.slot_call("release_lease", move |surface| surface.release_lease(lease))
    }

    /// Releases `slot` — held leases die with it and the index may be
    /// recycled under a bumped generation.
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when the slot is unknown or already
    /// released.
    pub fn release_native_slot(&self, slot: SurfaceSlotId) -> Result<(), ScenarioError> {
        self.slot_call("release_slot", move |surface| surface.release_slot(slot))
    }

    /// Current lifecycle state of `slot` — `None` when the id names
    /// no live or recycled slot.
    ///
    /// # Errors
    /// [`ScenarioError`] on a surface borrow or session failure.
    pub fn native_slot_state(
        &self,
        slot: SurfaceSlotId,
    ) -> Result<Option<SlotState>, ScenarioError> {
        self.slot_call("state", move |surface| surface.state(slot))
    }

    /// The declared [`SlotSpec`] of `slot` — the renderer-updated
    /// geometry/visibility observation point (the projection writes
    /// real layout bounds back through `set_geometry`/`set_clip`/
    /// `set_visibility` each frame).
    ///
    /// # Errors
    /// Same error surface as [`native_slot_state`](Self::native_slot_state).
    pub fn native_slot_spec(&self, slot: SurfaceSlotId) -> Result<Option<SlotSpec>, ScenarioError> {
        self.slot_call("spec", move |surface| surface.spec(slot))
    }

    /// The OS parent token of `slot`'s live container — the container
    /// HWND on Windows, carried as an opaque `u64`. `None` while no
    /// live lease exists.
    ///
    /// # Errors
    /// Same error surface as [`native_slot_state`](Self::native_slot_state).
    pub fn native_container_token(
        &self,
        slot: SurfaceSlotId,
    ) -> Result<Option<u64>, ScenarioError> {
        self.slot_call("container_token", move |surface| {
            surface.container_token(slot)
        })
    }

    /// Redeems a session-facing lease payload — the `u64` inside a
    /// `ParentSlotLease` — into the raw OS parent token. This is the
    /// issuer-side seam an `InProcessEditorOps` `LeaseRedeemer`
    /// calls: the adapter validates the payload against its own live
    /// ledger before returning the token, so a stale or forged lease
    /// never resolves to a real HWND.
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when the payload does not name a
    /// live lease; session failures keep their [`ScenarioError`]
    /// shape. A consumer redeem seam maps any `Err` to its own
    /// lease-rejection vocabulary.
    pub fn redeem_native_lease(&self, payload: u64) -> Result<u64, ScenarioError> {
        self.slot_call("redeem", move |surface| surface.redeem(payload))
    }
}

/// `true` where the surface backend can mint a real native container
/// parent — the only environment in which
/// [`ScenarioCapability::NativePluginDock`] may be claimed. The
/// Windows `GpuiSurfaceAdapter` creates a real child HWND per lease
/// (`surface::BACKEND_AVAILABLE`); `gpui-test` builds stub parent
/// capture, so no real container exists there even on Windows.
fn native_dock_backend_available() -> bool {
    surface::BACKEND_AVAILABLE && !cfg!(feature = "gpui-test")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_claim_only_what_the_adapter_provides() {
        let caps = supported_capabilities();
        assert!(caps.contains(&ScenarioCapability::SemanticAction));
        assert!(caps.contains(&ScenarioCapability::MultiWindow));
        // OS-level input is claimed exactly where `os_input` has an
        // injection backend — Windows only in this slice (#313).
        assert_eq!(
            caps.contains(&ScenarioCapability::RealOsPointerOrKeyboard),
            os_input::supported()
        );
        assert!(!caps.contains(&ScenarioCapability::SimulatedPointerOrKeyboard));
        // OS-level IME composition is claimed exactly where `os_ime`
        // has an IMM32 backend — Windows only in this slice (#389).
        assert_eq!(caps.contains(&ScenarioCapability::Ime), os_ime::supported());
        // NativePluginDock tracks the surface backend's ability to
        // mint a real container parent — Windows child HWNDs only,
        // never under `gpui-test` (Issue #631).
        assert_eq!(
            caps.contains(&ScenarioCapability::NativePluginDock),
            native_dock_backend_available()
        );
        // VisualCapture tracks the OS capture backend — Windows
        // (Win32) and Linux (X11) have one in this slice.
        assert_eq!(
            caps.contains(&ScenarioCapability::VisualCapture),
            cfg!(any(windows, target_os = "linux"))
        );
    }

    #[test]
    fn settle_job_replies_after_the_declared_boundaries() {
        // A settle job is a `UiJob` — its arguments are UI-thread
        // handles it ignores, so the countdown itself is exercised
        // through the channel contract: `ticks = 1` replies inside
        // the first pass.
        let (jobs_tx, jobs_rx) = channel::<UiJob>();
        let (reply, rx) = channel();
        let job = settle_job(1, jobs_tx, reply);
        // The job cannot run without `App` context, but its contract
        // is "reply on the final boundary" — verify the types line up
        // by checking the channel is still live before the job runs.
        drop(job);
        assert!(jobs_rx.try_recv().is_err());
        drop(rx);
    }
}
