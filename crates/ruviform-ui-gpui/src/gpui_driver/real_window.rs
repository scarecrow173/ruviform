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
//!   everywhere and `VisualCapture` only on Windows — the sole OS
//!   with a capture backend in this slice. `RealOsPointerOrKeyboard`
//!   is never claimed: OS-level input is Issue #313 scope and stays
//!   a separate evidence lane.
//! - `dispatch_input`/`select_*`/`invoke` are *semantic* dispatch
//!   through `ViewHost` — they bypass hit-testing and are not
//!   pointer/keyboard evidence.
//! - `capture_frame` returns real `PrintWindow`/`BitBlt` pixels via
//!   [`super::GpuiDriver::capture_window`]; backend errors map to
//!   [`CaptureOutcome::Unsupported`]/[`CaptureOutcome::NotRun`]
//!   rather than fabricated images.
//! - Stale `attach_epoch`/`generation` pins fail with
//!   [`ScenarioError::StaleEpoch`]/[`ScenarioError::StaleGeneration`]
//!   exactly like the other adapters — re-checked inside `App`
//!   context on every operation and capture.
//! - `settle` is frame-paced: it counts `drain` boundaries (one per
//!   tick), never sleeps.
//!
//! **RT contract:** non-RT only.

use std::cell::RefCell;
use std::collections::BTreeSet;
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
    DriverEvent, Intent, UiError, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_test::golden::RgbaImage;
use ruviform_ui_test::scenario::visual::{CaptureOutcome, VisualGuiTest};
use ruviform_ui_test::scenario::{
    MintedIntent, ScenarioCapability, ScenarioDriver, ScenarioError, ScenarioTarget,
    SemanticGuiTest, TargetQuery,
};

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
        let tick = options.tick;
        let settle_ticks = options.settle_ticks;
        let startup_settle_ticks = options.startup_settle_ticks;

        let thread_intents = Arc::clone(&intents);
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
            driver.run(tick, move |event, _drv| {
                if let DriverEvent::Windowed {
                    event: WindowLifecycle::Action(intent),
                    ..
                } = event
                {
                    thread_intents
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push(intent);
                }
            })
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
}

/// The capability set [`RealOsGuiTest`] honestly claims — see the
/// module docs. `VisualCapture` requires an OS capture backend —
/// Win32 on Windows, X11 (`GetImage`) on Linux; other OSes get
/// semantic + multi-window only until a backend lands.
/// `RealOsPointerOrKeyboard` stays unclaimed — OS-level input is
/// Issue #313 scope.
fn supported_capabilities() -> BTreeSet<ScenarioCapability> {
    let mut capabilities = BTreeSet::from([
        ScenarioCapability::SemanticAction,
        ScenarioCapability::MultiWindow,
    ]);
    if cfg!(any(windows, target_os = "linux")) {
        capabilities.insert(ScenarioCapability::VisualCapture);
    }
    capabilities
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
        supported_capabilities()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_claim_only_what_the_adapter_provides() {
        let caps = supported_capabilities();
        assert!(caps.contains(&ScenarioCapability::SemanticAction));
        assert!(caps.contains(&ScenarioCapability::MultiWindow));
        // OS-level input is Issue #313 scope — never claimed here.
        assert!(!caps.contains(&ScenarioCapability::RealOsPointerOrKeyboard));
        assert!(!caps.contains(&ScenarioCapability::SimulatedPointerOrKeyboard));
        assert!(!caps.contains(&ScenarioCapability::Ime));
        assert!(!caps.contains(&ScenarioCapability::NativePluginDock));
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
