//! `ruviform-ui-gpui` — GPUI adapter implementing the
//! `ruviform-ui` driver-neutral window/host contract (Issue #386).
//!
//! This crate is the **standard Driver backend** for the neutral
//! contract in [`ruviform_ui::driver`]. All `gpui` types
//! (`Application`, `App`, `Window`, `Entity`, `Context`,
//! `WindowHandle`) stay inside this crate — nothing in the public
//! surface leaks them, so consumer code binds only
//! [`ruviform_ui::WindowDriver`]/[`ruviform_ui::DriverEvent`].
//!
//! The `gpui` dependency is behind the non-default `gpui` feature so
//! `cargo check --workspace` stays a headless verification path
//! (same precedent as `ruviform-ui-webview`'s `webview` feature).
//! With the feature off, this crate compiles to nothing but
//! [`version`].
//!
//! **Capability honesty:** the minimal fixture wires window
//! lifecycle (create/resize/focus/close-flow/multi-window/monitors)
//! but declares `ime` and `native_surface_slot`
//! [`ruviform_ui::Capability::Unsupported`] — real IME routing and
//! the `NativeSurfaceSlot` embedding contract belong to Issues
//! #388/#389 and must not be faked here.
//!
//! **RT contract:** non-RT only — all entry points run on the UI
//! thread.

#![warn(missing_docs)]

/// Returns the crate version string.
///
/// **RT contract:** non-RT only.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(all(test, feature = "gpui-test"))]
mod lab_binding;
#[cfg(all(test, feature = "gpui-test"))]
mod lifecycle_fixture;

#[cfg(feature = "gpui")]
mod gpui_driver {
    /// The `NativeSurfaceSlot` port implementation for this backend
    /// (Issue #388) — slot ledger plus real OS surface containers.
    pub(super) mod surface;

    use std::cell::RefCell;
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    use std::rc::Rc;
    use std::time::Duration;

    use gpui::{
        AnyWindowHandle, App, AppContext, Application, Bounds, Context, Entity, IntoElement,
        ParentElement, Render, Styled, Subscription, TitlebarOptions, Window,
        WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, point, px, size,
    };
    use ruviform_ui::view::ComponentKey;
    use ruviform_ui::{
        Capability, CloseDecision, DriverEvent, Event, Intent, LogicalSize, MonitorInfo, SlotState,
        SurfaceSlotId, UiError, WindowCapabilities, WindowDriver, WindowId, WindowLifecycle,
        WindowSpec,
    };

    /// Default periodic tick (`DriverEvent::Tick` cadence) when the
    /// consumer does not override it.
    const DEFAULT_TICK: Duration = Duration::from_millis(16);

    /// Bounded `post_event` backlog per not-yet-`Ready` window —
    /// same bound as `ruviform-ui-test`'s `MAX_EVENT_BACKLOG`.
    const MAX_INBOX: usize = 16;

    /// Consumer event handler stored in [`Inner`].
    type Handler = Box<dyn FnMut(DriverEvent, &mut dyn WindowDriver)>;

    /// `u32` logical pixel → gpui `f32` pixel — exact at all
    /// realistic window coordinates (far below 2^24).
    #[expect(
        clippy::cast_precision_loss,
        reason = "window coordinates stay far below the f32 exact-integer range"
    )]
    fn f32_u(v: u32) -> f32 {
        v as f32
    }

    /// `i32` logical pixel → gpui `f32` pixel — same bound.
    #[expect(
        clippy::cast_precision_loss,
        reason = "window coordinates stay far below the f32 exact-integer range"
    )]
    fn f32_i(v: i32) -> f32 {
        v as f32
    }

    /// gpui `f32` pixel → `u32` physical pixel — the intended
    /// round-down of OS-reported bounds.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "OS-reported pixel bounds are positive and fit u32; truncation is the intended round-down"
    )]
    fn u32_of(v: f32) -> u32 {
        v as u32
    }

    /// gpui `f32` pixel → `i32` physical position — same
    /// round-down; virtual-desktop coordinates fit `i32`.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "virtual-desktop coordinates fit i32; truncation is the intended round-down"
    )]
    fn i32_of(v: f32) -> i32 {
        v as i32
    }

    /// Operations queued by [`WindowDriver`] calls, applied inside
    /// `App` context by [`drain`].
    enum Op {
        Create {
            id: WindowId,
            spec: WindowSpec,
            mount: Option<Box<ViewMount>>,
        },
        Resize {
            id: WindowId,
            size: LogicalSize,
        },
        Focus {
            id: WindowId,
            focused: bool,
        },
        RequestClose {
            id: WindowId,
        },
        Destroy {
            id: WindowId,
        },
        PostEvent {
            id: WindowId,
            event: Box<Event>,
        },
        Exit,
    }

    /// Per-window close bookkeeping — mirrors the contract in
    /// `FakeHost`/`FakeDriver`.
    #[derive(Default)]
    struct CloseState {
        pending: bool,
        awaiting_response: bool,
        approved: bool,
    }

    /// Consumer content mounted as a window's root view — the Issue
    /// #387 seam. Backend-free: the struct carries only
    /// `ruviform-ui`/`ruviform-ui-theme` types; the driver projects
    /// it onto its internal GPUI entity at window open.
    ///
    /// Intents minted by the view land on the window's `Action`
    /// driver-event channel through [`IntentSink`]; `post_event`
    /// `Result`/`Notify` payloads are projected back into `host`
    /// (receipts route by `command_id`, projections by
    /// `control_id`).
    pub struct ViewMount {
        /// Authoritative projection host for the view — spec tree,
        /// interaction dispatchers, and command bindings. The host's
        /// [`AttachEpoch`](ruviform_ui::view::AttachEpoch) must be
        /// minted by the consumer's session-owned
        /// [`AttachEpochAllocator`](ruviform_ui::view::AttachEpochAllocator)
        /// — one allocator per session, shared across every host it
        /// creates — so a recreated host (layout restore, move) can
        /// never reuse a dead host's command-id namespace.
        pub host: ruviform_ui::view::ViewHost,
        /// One-way [`ResolvedTheme`] consumption for the palette;
        /// `None` yields the literal fallback palette.
        pub theme: Option<ruviform_ui_theme::ResolvedTheme>,
    }

    /// Live window slot. `handle` is `None` between `create_window`
    /// and the deferred `open_window` application.
    struct Slot {
        handle: Option<AnyWindowHandle>,
        /// Raw OS window identifier captured at open — the surface
        /// adapter's parent of record for surface-slot containers.
        /// `usize` keeps it pointer-width without naming OS types.
        raw_parent: Option<usize>,
        root: Option<Entity<AppRoot>>,
        inbox: Rc<RefCell<Vec<Event>>>,
        /// `ComponentKey → SurfaceSlotId` associations installed by
        /// [`GpuiDriver::bind_native_slot`] — the mounted view reads
        /// the shared map each frame to project laid-out bounds onto
        /// bound slots (Issue #387 × #388 glue).
        surface_bindings: Rc<RefCell<BTreeMap<ComponentKey, SurfaceSlotId>>>,
        ready: bool,
        pending_focus: Option<bool>,
        scale_factor: f32,
        /// Last physical size reported via `Resized` — events fire
        /// on actual transitions only.
        last_size: Option<(u32, u32)>,
        /// Last physical position reported via `Moved`.
        last_pos: Option<(i32, i32)>,
        close: CloseState,
    }

    struct Inner {
        ops: VecDeque<Op>,
        events: VecDeque<DriverEvent>,
        windows: BTreeMap<WindowId, Slot>,
        /// Retired window ids — append-only for the driver's
        /// lifetime: the surface adapter's owner-death detection
        /// relies on entries never being dropped (`IsWindow` on the
        /// captured raw parent is the fallback for unrecorded
        /// teardown routes). `WindowId`s are never reused, so growth
        /// is bounded by total window creations.
        destroyed: BTreeSet<WindowId>,
        monitors: Vec<MonitorInfo>,
        capabilities: WindowCapabilities,
        handler: Option<Handler>,
        app_subs: Vec<Subscription>,
        next: u64,
        started: bool,
        exited: bool,
    }

    /// GPUI-backed implementation of [`WindowDriver`] — cloneable
    /// handle over shared driver state so `App`-context callbacks
    /// can hand `&mut dyn WindowDriver` to the consumer.
    ///
    /// Operations update driver state synchronously and queue
    /// backend work applied inside `App` context at the next drain
    /// (each tick and each window callback). `create_window`
    /// reserves the [`WindowId`] immediately; if OS-level creation
    /// later fails, the window terminates with `Destroyed`.
    ///
    /// `surface` is the driver's canonical [`SharedSurface`] handle:
    /// every clone shares the same slot ledger, and mounted views
    /// project laid-out `NativeSlot` component bounds onto it.
    ///
    /// **RT contract:** non-RT only — UI thread.
    #[derive(Clone)]
    pub struct GpuiDriver {
        inner: Rc<RefCell<Inner>>,
        surface: surface::SharedSurface,
    }

    /// Typed-intent ingress for views inside a GPUI window (Issue
    /// #386 `Action` channel): pushes [`WindowLifecycle::Action`]
    /// driver events from Rust-side view code — the GPUI-side
    /// counterpart of the `WebView` adapter's `postMessage` decode.
    #[derive(Clone)]
    pub struct IntentSink {
        inner: Rc<RefCell<Inner>>,
        window: WindowId,
    }

    impl IntentSink {
        /// Queues `intent` as an `Action` event on the bound window.
        /// Delivery follows the driver event flush — fire-and-forget;
        /// sends to unknown or destroyed windows are dropped.
        pub fn send(&self, intent: Intent) {
            let mut inner = self.inner.borrow_mut();
            if inner.windows.contains_key(&self.window) {
                inner.events.push_back(DriverEvent::Windowed {
                    window: self.window,
                    event: WindowLifecycle::Action(intent),
                });
            }
        }
    }

    impl Default for GpuiDriver {
        fn default() -> Self {
            Self::new()
        }
    }

    impl GpuiDriver {
        /// Creates a driver with no windows.
        #[must_use]
        pub fn new() -> Self {
            let inner = Rc::new(RefCell::new(Inner {
                ops: VecDeque::new(),
                events: VecDeque::new(),
                windows: BTreeMap::new(),
                destroyed: BTreeSet::new(),
                monitors: Vec::new(),
                capabilities: WindowCapabilities {
                    resize: Capability::Supported,
                    focus: Capability::Supported,
                    scale_factor: Capability::Supported,
                    multi_window: Capability::Supported,
                    // IME routing stays unwired — Issue #389 owns
                    // that contract. `native_surface_slot` is
                    // implemented by `GpuiSurfaceAdapter` on the
                    // OSes that have a container-surface backend;
                    // honest per-OS reporting, never fake support.
                    ime: Capability::Unsupported,
                    native_surface_slot: if surface::BACKEND_AVAILABLE {
                        Capability::Supported
                    } else {
                        Capability::Unsupported
                    },
                },
                handler: None,
                app_subs: Vec::new(),
                next: 1,
                started: false,
                exited: false,
            }));
            Self {
                surface: surface::SharedSurface::wrap(surface::GpuiSurfaceAdapter::bound(&inner)),
                inner,
            }
        }

        /// The driver's canonical [`SharedSurface`] handle — the one
        /// slot ledger mounted views project onto. Declare slots
        /// through this handle (not a fresh `GpuiSurfaceAdapter::new`,
        /// which would own a separate ledger) so `bind_native_slot`'s
        /// runtime check and the renderer's
        /// `set_geometry`/`set_clip`/`set_visibility` calls see them.
        /// Borrow discipline is internal to the handle — a consumer
        /// cannot hold a guard across frames, so it cannot starve the
        /// renderer's pushes.
        ///
        /// **RT contract:** non-RT only — UI-thread only.
        #[must_use]
        pub fn surface(&self) -> surface::SharedSurface {
            self.surface.clone()
        }

        /// Binds a mounted view's `NativeSlot` component (`key`) to a
        /// live [`SurfaceSlotId`] — after this, the view projects its
        /// laid-out bounds, ancestor clip, and painted/suppressed
        /// state onto `slot` each frame.
        ///
        /// The slot must already be declared on [`Self::surface`]
        /// (`spec.window == window`) and not released — binding a
        /// stale or unknown id fails here rather than silently
        /// no-oping in the paint loop.
        ///
        /// # Errors
        /// [`UiError::Host`] when `window` is unknown, `key` is
        /// already bound, or `slot` is not a live slot on the
        /// canonical adapter.
        pub fn bind_native_slot(
            &mut self,
            window: WindowId,
            key: ComponentKey,
            slot: SurfaceSlotId,
        ) -> Result<(), UiError> {
            {
                let inner = self.inner.borrow();
                if !inner.windows.contains_key(&window) {
                    return Err(UiError::host(format!("no window {window}")));
                }
            }
            {
                match self
                    .surface
                    .state(slot)
                    .map_err(|e| UiError::host(format!("surface borrow: {e:?}")))?
                {
                    Some(SlotState::Declared | SlotState::Leased) => {}
                    Some(SlotState::Released) => {
                        return Err(UiError::host(format!("surface slot {slot:?} is released")));
                    }
                    None => {
                        return Err(UiError::host(format!("unknown surface slot {slot:?}")));
                    }
                    Some(state) => {
                        return Err(UiError::host(format!(
                            "surface slot {slot:?} in unbindable state {state:?}"
                        )));
                    }
                }
                if self
                    .surface
                    .spec(slot)
                    .map_err(|e| UiError::host(format!("surface borrow: {e:?}")))?
                    .is_some_and(|s| s.window != window)
                {
                    return Err(UiError::host(format!(
                        "surface slot {slot:?} belongs to a different window"
                    )));
                }
            }
            let bindings = self
                .slot_binding_map(window)
                .ok_or_else(|| UiError::host(format!("no window {window}")))?;
            {
                let map = bindings.borrow();
                if map.contains_key(&key) {
                    return Err(UiError::host(format!(
                        "component key {key} already bound to a surface slot"
                    )));
                }
                // One slot paints into one region — binding the same
                // slot to a second key would race its geometry.
                if map.values().any(|s| *s == slot) {
                    return Err(UiError::host(format!(
                        "surface slot {slot:?} already bound to another key"
                    )));
                }
            }
            bindings.borrow_mut().insert(key, slot);
            Ok(())
        }

        /// Removes `key`'s slot binding. The next painted frame
        /// suppresses the slot (`Hidden`) — releasing the slot itself
        /// stays the consumer's call on [`Self::surface`].
        ///
        /// # Errors
        /// [`UiError::Host`] when `window` is unknown or `key` has no
        /// binding.
        pub fn unbind_native_slot(
            &mut self,
            window: WindowId,
            key: &ComponentKey,
        ) -> Result<(), UiError> {
            let bindings = self
                .slot_binding_map(window)
                .ok_or_else(|| UiError::host(format!("no window {window}")))?;
            if bindings.borrow_mut().remove(key).is_none() {
                return Err(UiError::host(format!(
                    "component key {key} has no surface-slot binding"
                )));
            }
            Ok(())
        }

        /// The shared `key → slot` map for `window`, if live — the
        /// mounted view reads it each frame.
        pub(crate) fn slot_binding_map(
            &self,
            window: WindowId,
        ) -> Option<Rc<RefCell<BTreeMap<ComponentKey, SurfaceSlotId>>>> {
            self.inner
                .borrow()
                .windows
                .get(&window)
                .map(|s| s.surface_bindings.clone())
        }

        /// An [`IntentSink`] bound to `window` for view-side
        /// `Action` intent production.
        #[must_use]
        pub fn intent_sink(&self, window: WindowId) -> IntentSink {
            IntentSink {
                inner: self.inner.clone(),
                window,
            }
        }

        /// Opens a window whose root is the consumer's
        /// [`ViewSpec`] renderer instead of the fixture surface —
        /// the Issue #387 mount seam. Same reservation contract as
        /// [`WindowDriver::create_window`]: the id is minted now,
        /// the OS window appears at the next `run` drain.
        ///
        /// # Errors
        /// Currently infallible — kept `Result` for parity with
        /// `create_window` and future backend failures.
        pub fn create_view_window(
            &mut self,
            spec: &WindowSpec,
            mount: ViewMount,
        ) -> Result<WindowId, UiError> {
            Ok(reserve_window(&self.inner, spec, Some(Box::new(mount))))
        }

        /// Ids the driver recorded as destroyed.
        #[must_use]
        pub fn destroyed(&self) -> BTreeSet<WindowId> {
            self.inner.borrow().destroyed.clone()
        }

        /// Whether `exit`/`quit` was requested.
        #[must_use]
        pub fn exited(&self) -> bool {
            self.inner.borrow().exited
        }

        /// Runs the GPUI application event loop, delivering
        /// [`DriverEvent`]s to `on_event` — the driver-neutral
        /// counterpart of `ruviform-ui-webview`'s `run_multi`.
        ///
        /// `tick` sets the `Tick` cadence and the maximum latency
        /// before queued operations are applied. `None` uses
        /// [`DEFAULT_TICK`]. This call blocks until [`exit`] runs
        /// and the platform loop returns.
        ///
        /// # Errors
        /// [`UiError::BackendUnavailable`] when the loop was already
        /// started.
        pub fn run(
            &mut self,
            tick: Option<Duration>,
            on_event: impl FnMut(DriverEvent, &mut dyn WindowDriver) + 'static,
        ) -> Result<(), UiError> {
            {
                let mut inner = self.inner.borrow_mut();
                if inner.started {
                    return Err(UiError::BackendUnavailable {
                        message: "gpui driver loop already started".to_string(),
                    });
                }
                inner.started = true;
                inner.handler = Some(Box::new(on_event));
            }
            let tick = tick.unwrap_or(DEFAULT_TICK);
            let inner = self.inner.clone();
            let driver = self.clone();
            let app = Application::new();
            app.run(move |cx| {
                // Global teardown hook: any window close sweeps
                // stale handles into the terminal `Destroyed` path.
                let closed_sub = cx.on_window_closed({
                    let inner = inner.clone();
                    let driver = driver.clone();
                    move |cx| {
                        sweep_destroyed(&inner, cx);
                        flush_events(&inner, &driver);
                    }
                });
                inner.borrow_mut().app_subs.push(closed_sub);
                // Periodic pump: applies queued WindowDriver ops and
                // emits Tick — the loop-global job-drain wakeup.
                cx.spawn({
                    let inner = inner.clone();
                    let driver = driver.clone();
                    async move |cx: &mut gpui::AsyncApp| {
                        loop {
                            cx.background_executor().timer(tick).await;
                            if cx
                                .update(|cx| {
                                    drain(&inner, &driver, cx);
                                    inner.borrow_mut().events.push_back(DriverEvent::Tick);
                                    flush_events(&inner, &driver);
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                })
                .detach();
                drain(&inner, &driver, cx);
                flush_events(&inner, &driver);
            });
            self.inner.borrow_mut().exited = true;
            Ok(())
        }
    }

    /// Test-harness seam (`cfg(test)` + `gpui-test` only — not a
    /// production entry point). `run` owns the platform loop; these
    /// hooks let a `TestAppContext` drive the same queue and event
    /// delivery one turn at a time for the shared lifecycle fixture
    /// (Issue #386).
    #[cfg(all(test, feature = "gpui-test"))]
    impl GpuiDriver {
        /// One harness turn: applies queued window ops inside an App
        /// update, lets deferred work park (the first-frame `Ready`),
        /// applies follow-up ops, then delivers the emitted events to
        /// `on_event`. The handler may call back into the driver.
        pub(crate) fn settle_for_test(
            &mut self,
            cx: &mut gpui::TestAppContext,
            on_event: impl FnMut(DriverEvent, &mut dyn WindowDriver) + 'static,
        ) {
            let this = self.clone();
            this.inner.borrow_mut().handler = Some(Box::new(on_event));
            cx.update(|app| drain(&this.inner, &this, app));
            cx.run_until_parked();
            cx.update(|app| drain(&this.inner, &this, app));
            flush_events(&this.inner, &this);
        }

        /// Events queued in `window`'s inbox and not yet consumed by
        /// its root view.
        #[must_use]
        pub(crate) fn inbox_len_for_test(&self, window: WindowId) -> usize {
            self.inner
                .borrow()
                .windows
                .get(&window)
                .map_or(0, |slot| slot.inbox.borrow().len())
        }

        /// The mounted view of a window opened through
        /// [`GpuiDriver::create_view_window`]; `None` for fixture windows.
        #[must_use]
        pub(crate) fn view_for_test(
            &self,
            cx: &mut gpui::TestAppContext,
            window: WindowId,
        ) -> Option<Entity<crate::render::RvfView>> {
            let root = self.inner.borrow().windows.get(&window)?.root.clone()?;
            root.update(cx, |root, _cx| {
                root.content
                    .clone()
                    .downcast::<crate::render::RvfView>()
                    .ok()
            })
        }
    }

    impl WindowDriver for GpuiDriver {
        fn capabilities(&self) -> WindowCapabilities {
            self.inner.borrow().capabilities.clone()
        }

        fn create_window(&mut self, spec: &WindowSpec) -> Result<WindowId, UiError> {
            Ok(reserve_window(&self.inner, spec, None))
        }

        fn resize(&mut self, window: WindowId, size: LogicalSize) -> Result<(), UiError> {
            let mut inner = self.inner.borrow_mut();
            if inner.capabilities.resize != Capability::Supported {
                return Err(UiError::unsupported("resize", "gpui driver cannot resize"));
            }
            if !inner.windows.contains_key(&window) {
                return Err(UiError::host(format!("no window {window}")));
            }
            inner.ops.push_back(Op::Resize { id: window, size });
            Ok(())
        }

        fn focus(&mut self, window: WindowId, focused: bool) -> Result<(), UiError> {
            let mut inner = self.inner.borrow_mut();
            if inner.capabilities.focus != Capability::Supported {
                return Err(UiError::unsupported("focus", "gpui driver cannot focus"));
            }
            let Some(slot) = inner.windows.get_mut(&window) else {
                return Err(UiError::host(format!("no window {window}")));
            };
            if !focused {
                // gpui 0.2.2 exposes window activation but no OS-level
                // deactivation — report honestly rather than silently
                // no-op.
                return Err(UiError::unsupported(
                    "focus",
                    "gpui 0.2.2 cannot release window focus",
                ));
            }
            // Focus requests stay meaningful before the OS window
            // exists — applied again at open.
            slot.pending_focus = Some(focused);
            inner.ops.push_back(Op::Focus {
                id: window,
                focused,
            });
            Ok(())
        }

        fn request_close(&mut self, window: WindowId) {
            let mut inner = self.inner.borrow_mut();
            if inner.windows.contains_key(&window) {
                inner.ops.push_back(Op::RequestClose { id: window });
            }
        }

        fn respond_close(
            &mut self,
            window: WindowId,
            decision: CloseDecision,
        ) -> Result<(), UiError> {
            let mut inner = self.inner.borrow_mut();
            let Some(slot) = inner.windows.get_mut(&window) else {
                return Err(UiError::host(format!("no window {window}")));
            };
            if !slot.close.pending {
                return Err(UiError::host(format!(
                    "window {window} has no pending close"
                )));
            }
            slot.close.awaiting_response = false;
            match decision {
                CloseDecision::Approve => {
                    slot.close.approved = true;
                    inner.ops.push_back(Op::Destroy { id: window });
                }
                CloseDecision::Veto => slot.close.pending = false,
                // Defer leaves the close pending; unknown future
                // variants conservatively keep the window open.
                _ => {}
            }
            Ok(())
        }

        fn destroy_window(&mut self, window: WindowId) -> Result<(), UiError> {
            let mut inner = self.inner.borrow_mut();
            if !inner.windows.contains_key(&window) {
                return Err(UiError::host(format!("no window {window}")));
            }
            inner.ops.push_back(Op::Destroy { id: window });
            Ok(())
        }

        fn post_event(&mut self, window: WindowId, event: &Event) {
            let mut inner = self.inner.borrow_mut();
            let Some(slot) = inner.windows.get(&window) else {
                return; // unknown/destroyed — silent drop per contract
            };
            let (ready, inbox) = (slot.ready, slot.inbox.clone());
            if ready {
                inner.ops.push_back(Op::PostEvent {
                    id: window,
                    event: Box::new(event.clone()),
                });
            } else {
                // Not-ready delivery: buffer into the window inbox —
                // the same channel `Op::PostEvent` uses once live.
                let mut inbox = inbox.borrow_mut();
                if inbox.len() >= MAX_INBOX {
                    inbox.remove(0);
                }
                inbox.push(event.clone());
            }
        }

        fn windows(&self) -> Vec<WindowId> {
            let inner = self.inner.borrow();
            inner.windows.keys().copied().collect()
        }

        fn monitors(&self) -> Vec<MonitorInfo> {
            // Snapshot refreshed at each drain — empty before `run`.
            self.inner.borrow().monitors.clone()
        }

        fn exit(&mut self) {
            let mut inner = self.inner.borrow_mut();
            inner.exited = true;
            inner.ops.push_back(Op::Exit);
        }
    }

    /// Reserves a [`WindowId`], installs the slot, and queues the
    /// create op — shared by `create_window` (fixture root) and
    /// `create_view_window` (consumer mount).
    fn reserve_window(
        inner: &Rc<RefCell<Inner>>,
        spec: &WindowSpec,
        mount: Option<Box<ViewMount>>,
    ) -> WindowId {
        let mut inner = inner.borrow_mut();
        let id = WindowId::new(inner.next).expect("counter nonzero");
        inner.next += 1;
        inner.windows.insert(
            id,
            Slot {
                handle: None,
                raw_parent: None,
                root: None,
                inbox: Rc::new(RefCell::new(Vec::new())),
                surface_bindings: Rc::new(RefCell::new(BTreeMap::new())),
                ready: false,
                pending_focus: None,
                scale_factor: 1.0,
                last_size: None,
                last_pos: None,
                close: CloseState::default(),
            },
        );
        inner.ops.push_back(Op::Create {
            id,
            spec: spec.clone(),
            mount,
        });
        id
    }

    /// Delivers queued driver→consumer events to the handler.
    /// The handler is taken out while invoked so it may call back
    /// into the driver (which borrows `inner`) without a `RefCell`
    /// re-entry panic.
    fn flush_events(inner: &Rc<RefCell<Inner>>, driver: &GpuiDriver) {
        loop {
            let (ev, handler) = {
                let mut guard = inner.borrow_mut();
                match guard.events.pop_front() {
                    Some(ev) => (ev, guard.handler.take()),
                    None => return,
                }
            };
            let mut handler = handler;
            let mut drv = driver.clone();
            if let Some(h) = handler.as_mut() {
                h(ev, &mut drv);
            }
            inner.borrow_mut().handler = handler;
        }
    }

    /// Detects windows whose OS teardown bypassed the close flow
    /// (backend loss, external kill) — any slot whose handle no
    /// longer resolves is retired and emits `Destroyed`.
    fn sweep_destroyed(inner: &Rc<RefCell<Inner>>, cx: &mut App) {
        // Probe handles guard-free: `update` runs arbitrary gpui
        // post-update work whose callbacks may borrow `inner`.
        let handles: Vec<(WindowId, AnyWindowHandle)> = {
            let guard = inner.borrow();
            guard
                .windows
                .iter()
                .filter_map(|(id, s)| s.handle.map(|h| (*id, h)))
                .collect()
        };
        for (id, h) in handles {
            if h.update(cx, |_, _, _| ()).is_err() {
                retire(inner, id);
            }
        }
    }

    /// Applies queued [`Op`]s inside `App` context — window ops
    /// cannot run without `cx`, so `WindowDriver` calls defer here.
    ///
    /// Ordering invariant: `sweep_destroyed` retires stale slots
    /// *before* the op loop so ops never touch released entities or
    /// dead window handles.
    fn drain(inner: &Rc<RefCell<Inner>>, driver: &GpuiDriver, cx: &mut App) {
        refresh_monitors(inner, cx);
        sweep_destroyed(inner, cx);
        loop {
            let op = inner.borrow_mut().ops.pop_front();
            let Some(op) = op else { break };
            match op {
                Op::Create { id, spec, mount } => {
                    open_window(inner, driver, cx, id, &spec, mount);
                }
                Op::Resize { id, size } => {
                    let handle = inner.borrow().windows.get(&id).and_then(|s| s.handle);
                    if let Some(h) = handle {
                        let _ = h.update(cx, |_, window, _| {
                            window
                                .resize(gpui::size(px(f32_u(size.width)), px(f32_u(size.height))));
                        });
                    }
                }
                Op::Focus { id, focused } => {
                    let handle = inner.borrow().windows.get(&id).and_then(|s| s.handle);
                    if let Some(h) = handle {
                        let _ = h.update(cx, |_, window, _| {
                            if focused {
                                window.activate_window();
                            }
                        });
                    }
                }
                Op::RequestClose { id } => {
                    // Programmatic close request = the same
                    // veto-able `CloseRequested` the user close
                    // button produces; the unanswered default is
                    // Approve for this window only. Dead windows are
                    // dropped silently — same as `post_event`.
                    let known = {
                        let mut guard = inner.borrow_mut();
                        match guard.windows.get_mut(&id) {
                            Some(slot) => {
                                slot.close.pending = true;
                                slot.close.awaiting_response = true;
                                guard.events.push_back(DriverEvent::Windowed {
                                    window: id,
                                    event: WindowLifecycle::CloseRequested,
                                });
                                true
                            }
                            None => false,
                        }
                    };
                    if !known {
                        continue;
                    }
                    flush_events(inner, driver);
                    let approved = {
                        let mut guard = inner.borrow_mut();
                        match guard.windows.get_mut(&id) {
                            Some(slot) if slot.close.awaiting_response => {
                                slot.close.approved = true;
                                true
                            }
                            Some(slot) => slot.close.approved,
                            None => false,
                        }
                    };
                    if approved {
                        apply_destroy(inner, id, cx);
                    }
                }
                Op::Destroy { id } => apply_destroy(inner, id, cx),
                Op::PostEvent { id, event } => {
                    let (root, inbox) = {
                        let guard = inner.borrow();
                        match guard.windows.get(&id) {
                            Some(s) => (s.root.clone(), s.inbox.clone()),
                            None => continue,
                        }
                    };
                    // Same bounded drop-oldest bound as the pre-`Ready`
                    // backlog — the fixture inbox must not grow forever.
                    {
                        let mut inbox = inbox.borrow_mut();
                        if inbox.len() >= MAX_INBOX {
                            inbox.remove(0);
                        }
                        inbox.push(*event);
                    }
                    // Safe while `drain` runs `sweep_destroyed` first —
                    // a dead window's slot/root are retired before ops.
                    if let Some(root) = root {
                        root.update(cx, |_, cx| cx.notify());
                    }
                }
                Op::Exit => cx.quit(),
            }
        }
    }

    /// Maps `WindowSpec` → `WindowOptions` — the whole point of the
    /// neutral spec: title, logical size/position, resizable, nothing
    /// backend-specific.
    fn spec_options(spec: &WindowSpec, cx: &App) -> WindowOptions {
        let size = size(
            px(f32_u(spec.logical_size.width)),
            px(f32_u(spec.logical_size.height)),
        );
        // `None` = platform default placement — center on the
        // primary display rather than pinning origin (0,0).
        let bounds = spec.position.map_or_else(
            || Bounds::centered(None, size, cx),
            |p| Bounds {
                origin: point(px(f32_i(p.x)), px(f32_i(p.y))),
                size,
            },
        );
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some(spec.title.clone().into()),
                appears_transparent: false,
                traffic_light_position: None,
            }),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            is_resizable: spec.resizable,
            is_minimizable: true,
            display_id: None,
            window_background: WindowBackgroundAppearance::Opaque,
            app_id: None,
            window_min_size: None,
            window_decorations: None,
            tabbing_identifier: None,
        }
    }

    /// The user/OS close hook — the veto-able `CloseRequested`
    /// contract. Returns the `should_close` verdict: an approved or
    /// unanswered close proceeds; `Veto`/`Defer` keeps the window
    /// open. `respond_close` decisions taken later apply through the
    /// `Op::Destroy` drain.
    fn on_should_close(inner: &Rc<RefCell<Inner>>, driver: &GpuiDriver, id: WindowId) -> bool {
        {
            let mut guard = inner.borrow_mut();
            match guard.windows.get_mut(&id) {
                Some(slot) if slot.close.approved => return true,
                Some(slot) => {
                    slot.close.pending = true;
                    slot.close.awaiting_response = true;
                }
                None => return true,
            }
        }
        inner.borrow_mut().events.push_back(DriverEvent::Windowed {
            window: id,
            event: WindowLifecycle::CloseRequested,
        });
        flush_events(inner, driver);
        // Contract: unanswered defaults to Approve for this window
        // only; Veto/Defer keep it open.
        let mut guard = inner.borrow_mut();
        match guard.windows.get_mut(&id) {
            Some(slot) if slot.close.approved => true,
            Some(slot) if slot.close.awaiting_response => {
                slot.close.approved = true;
                true
            }
            _ => false,
        }
    }

    /// OS parent handle for surface containers. gpui's test platform
    /// has no OS handle and its `window_handle` panics rather than
    /// erroring, so test builds with `gpui-test` keep the parent unset —
    /// the lifecycle fixture embeds no surface.
    #[cfg(not(all(test, feature = "gpui-test")))]
    fn capture_raw_parent(window: &Window) -> Option<usize> {
        surface::raw_parent_of(window)
    }

    #[cfg(all(test, feature = "gpui-test"))]
    fn capture_raw_parent(_window: &Window) -> Option<usize> {
        None
    }

    /// Maps `WindowSpec` → `WindowOptions` and opens the OS window.
    /// `mount` selects the root content: `Some` projects the
    /// consumer's [`ViewSpec`] through the Issue #387 renderer;
    /// `None` keeps the fixture surface.
    fn open_window(
        inner: &Rc<RefCell<Inner>>,
        driver: &GpuiDriver,
        cx: &mut App,
        id: WindowId,
        spec: &WindowSpec,
        mount: Option<Box<ViewMount>>,
    ) {
        let options = spec_options(spec, cx);
        let inner2 = inner.clone();
        let driver2 = driver.clone();
        let inbox = inner
            .borrow()
            .windows
            .get(&id)
            .map(|s| s.inbox.clone())
            .unwrap_or_default();
        let created = cx.open_window(options, move |window, cx| {
            let scale = window.scale_factor();
            {
                let mut guard = inner2.borrow_mut();
                if let Some(slot) = guard.windows.get_mut(&id) {
                    slot.scale_factor = scale;
                    // Capture the OS parent handle while `&mut Window`
                    // is in scope — the surface adapter redeems leases
                    // into containers parented under this handle.
                    slot.raw_parent = capture_raw_parent(window);
                }
            }
            // User/OS close → veto-able CloseRequested contract.
            window.on_window_should_close(cx, {
                let inner = inner2.clone();
                let driver = driver2.clone();
                move |_window, _cx| on_should_close(&inner, &driver, id)
            });
            let content: gpui::AnyView = match mount {
                Some(m) => {
                    let mount = *m;
                    let sink = IntentSink {
                        inner: inner2.clone(),
                        window: id,
                    };
                    let slots = crate::render::SlotMount {
                        adapter: driver2.surface.clone(),
                        bindings: inner2
                            .borrow()
                            .windows
                            .get(&id)
                            .map(|s| s.surface_bindings.clone())
                            .unwrap_or_default(),
                    };
                    cx.new(|cx| {
                        crate::render::RvfView::mounted(
                            mount.host,
                            mount.theme.as_ref(),
                            &sink,
                            inbox,
                            slots,
                            window,
                            cx,
                        )
                    })
                    .into()
                }
                None => cx.new(|_| RvfRoot::new(spec.title.clone(), inbox)).into(),
            };
            let entity =
                cx.new(|cx| AppRoot::new(id, content, inner2.clone(), driver2, window, cx));
            {
                let mut guard = inner2.borrow_mut();
                if let Some(slot) = guard.windows.get_mut(&id) {
                    slot.root = Some(entity.clone());
                }
            }
            entity
        });
        match created {
            Ok(handle) => {
                let any: AnyWindowHandle = handle.into();
                {
                    let mut guard = inner.borrow_mut();
                    if let Some(slot) = guard.windows.get_mut(&id) {
                        slot.handle = Some(any);
                    }
                    guard.events.push_back(DriverEvent::Windowed {
                        window: id,
                        event: WindowLifecycle::Created,
                    });
                }
                // Ready = first applied frame — defer one cycle.
                let inner3 = inner.clone();
                let _ = any.update(cx, |_, window, cx| {
                    window.defer(cx, move |window, _cx| {
                        let mut guard = inner3.borrow_mut();
                        if let Some(slot) = guard.windows.get_mut(&id) {
                            slot.ready = true;
                            slot.scale_factor = window.scale_factor();
                            if slot.pending_focus.take() == Some(true) {
                                window.activate_window();
                            }
                            guard.events.push_back(DriverEvent::Windowed {
                                window: id,
                                event: WindowLifecycle::Ready,
                            });
                        }
                    });
                });
            }
            Err(_err) => {
                // Creation failure terminates as Destroyed — the id
                // is retired and never reused.
                retire(inner, id);
            }
        }
    }

    /// Applies a destroy op inside `App` context: removes the OS
    /// window; the resulting `on_window_closed` (or the next
    /// `sweep_destroyed`) then retires the slot and emits
    /// `Destroyed`. If the handle update fails the window is
    /// already gone — retire inline.
    fn apply_destroy(inner: &Rc<RefCell<Inner>>, id: WindowId, cx: &mut App) {
        let handle = inner.borrow().windows.get(&id).and_then(|s| s.handle);
        match handle {
            Some(h) => {
                if h.update(cx, |_, window, _| window.remove_window()).is_err() {
                    retire(inner, id);
                }
            }
            None => retire(inner, id),
        }
    }

    /// Marks a window destroyed in driver state and emits
    /// `Destroyed` — the terminal retire path shared by the release
    /// observer, failed updates, and failed creation.
    fn retire(inner: &Rc<RefCell<Inner>>, id: WindowId) {
        let mut guard = inner.borrow_mut();
        if guard.windows.remove(&id).is_some() {
            guard.destroyed.insert(id);
            guard.events.push_back(DriverEvent::Windowed {
                window: id,
                event: WindowLifecycle::Destroyed,
            });
        }
    }

    fn refresh_monitors(inner: &Rc<RefCell<Inner>>, cx: &App) {
        // gpui 0.2.2 `PlatformDisplay` exposes logical-pixel bounds
        // only — no per-display scale factor. `MonitorInfo` asks for
        // physical pixels, so the estimate uses the scale factor
        // observed on a live window (typically the primary display);
        // heterogeneous-DPI precision is an extension point for the
        // real native-surface work (Issue #388).
        let sf = {
            let guard = inner.borrow();
            guard
                .windows
                .values()
                .next()
                .map_or(1.0, |s| s.scale_factor)
        };
        let monitors: Vec<MonitorInfo> = cx
            .displays()
            .iter()
            .map(|d| {
                let b = d.bounds();
                MonitorInfo {
                    name: d.uuid().ok().map(|u| u.to_string()),
                    width: u32_of(f32::from(b.size.width) * sf),
                    height: u32_of(f32::from(b.size.height) * sf),
                    x: i32_of(f32::from(b.origin.x) * sf),
                    y: i32_of(f32::from(b.origin.y) * sf),
                    scale_factor: f64::from(sf),
                }
            })
            .collect();
        inner.borrow_mut().monitors = monitors;
    }

    /// Emits geometry/scale events read from a live `Window` —
    /// shared by the bounds observer.
    fn emit_window_state(inner: &Rc<RefCell<Inner>>, id: WindowId, window: &Window) {
        let sf = window.scale_factor();
        let vp = window.viewport_size();
        let b = window.bounds();
        // Geometry events fire on actual transitions only — the
        // legacy contract does not emit a `Resized` for a pure move.
        // Slot state is mutated under the borrow, events pushed
        // after it ends.
        let (scale, size, pos) = {
            let mut guard = inner.borrow_mut();
            let Some(slot) = guard.windows.get_mut(&id) else {
                return;
            };
            let scale = ((slot.scale_factor - sf).abs() > f32::EPSILON).then(|| {
                slot.scale_factor = sf;
                f64::from(sf)
            });
            let size = (
                u32_of(f32::from(vp.width) * sf),
                u32_of(f32::from(vp.height) * sf),
            );
            let size = (slot.last_size != Some(size)).then(|| {
                slot.last_size = Some(size);
                size
            });
            let pos = (
                i32_of(f32::from(b.origin.x) * sf),
                i32_of(f32::from(b.origin.y) * sf),
            );
            let pos = (slot.last_pos != Some(pos)).then(|| {
                slot.last_pos = Some(pos);
                pos
            });
            (scale, size, pos)
        };
        let mut guard = inner.borrow_mut();
        if let Some(factor) = scale {
            guard.events.push_back(DriverEvent::Windowed {
                window: id,
                event: WindowLifecycle::ScaleFactorChanged { factor },
            });
        }
        if let Some((width, height)) = size {
            guard.events.push_back(DriverEvent::Windowed {
                window: id,
                event: WindowLifecycle::Resized { width, height },
            });
        }
        if let Some((x, y)) = pos {
            guard.events.push_back(DriverEvent::Windowed {
                window: id,
                event: WindowLifecycle::Moved { x, y },
            });
        }
    }

    /// Window-level root entity — owns the lifecycle subscriptions
    /// (bounds → `Resized`/`Moved`/`ScaleFactorChanged`, activation →
    /// `Focused`) once per window, and renders the selected content
    /// view: the fixture [`RvfRoot`] or a consumer
    /// [`render::RvfView`] mounted through [`GpuiDriver::create_view_window`].
    struct AppRoot {
        rvf_id: WindowId,
        content: gpui::AnyView,
        inner: Rc<RefCell<Inner>>,
        driver: GpuiDriver,
        _subscriptions: Vec<Subscription>,
    }

    impl AppRoot {
        fn new(
            rvf_id: WindowId,
            content: gpui::AnyView,
            inner: Rc<RefCell<Inner>>,
            driver: GpuiDriver,
            window: &mut Window,
            cx: &mut Context<Self>,
        ) -> Self {
            let subscriptions = vec![
                cx.observe_window_bounds(window, |this, window, _cx| {
                    emit_window_state(&this.inner, this.rvf_id, window);
                    flush_events(&this.inner, &this.driver);
                }),
                cx.observe_window_activation(window, |this, window, _cx| {
                    let active = window.is_window_active();
                    this.inner
                        .borrow_mut()
                        .events
                        .push_back(DriverEvent::Windowed {
                            window: this.rvf_id,
                            event: WindowLifecycle::Focused(active),
                        });
                    flush_events(&this.inner, &this.driver);
                }),
            ];
            Self {
                rvf_id,
                content,
                inner,
                driver,
                _subscriptions: subscriptions,
            }
        }
    }

    impl Render for AppRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            self.content.clone()
        }
    }

    /// Minimal fixture content — renders the window title and the
    /// count of typed [`Event`]s delivered via `post_event` so the
    /// typed push path is observable on the real backend.
    struct RvfRoot {
        title: String,
        inbox: Rc<RefCell<Vec<Event>>>,
    }

    impl RvfRoot {
        fn new(title: String, inbox: Rc<RefCell<Vec<Event>>>) -> Self {
            Self { title, inbox }
        }
    }

    impl Render for RvfRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(format!(
                "ruviform gpui fixture — {} (events: {})",
                self.title,
                self.inbox.borrow().len()
            ))
        }
    }
}

#[cfg(feature = "gpui")]
pub use gpui_driver::surface::{GpuiSurfaceAdapter, SharedSurface};
#[cfg(feature = "gpui")]
pub use gpui_driver::{GpuiDriver, IntentSink, ViewMount};

/// Issue #387 — GPUI Controls/Viz renderer: `ViewSpec` → element
/// projection, pointer/key → `ControlInput` normalization, and
/// `CommandIntent` egress through `rvf.ui.command.v1` intents.
/// Adapter-internal: `gpui` types never cross the crate surface —
/// consumers mount a view through [`ViewMount`] +
/// [`GpuiDriver::create_view_window`].
#[cfg(feature = "gpui")]
mod render;

#[cfg(feature = "gpui")]
pub use render::COMMAND_INTENT_OP;
