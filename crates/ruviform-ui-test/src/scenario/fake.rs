//! Fake [`SemanticGuiTest`] adapter — `ViewHost` instances keyed by
//! `WindowId`/`ViewInstanceId` (Issue #464).
//!
//! The adapter owns no product logic: every operation delegates to the
//! mounted [`ViewHost`] and every wire [`Event`] goes through
//! [`ViewHost::deliver_event`], the same decode path the GPUI adapter
//! uses. `settle` is a no-op beyond draining recorded events because
//! `ViewHost` is synchronous — there is no hidden queue to flush.
//!
//! **RT contract:** non-RT only.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ruviform_ui::WindowId;
use ruviform_ui::behavior::ControlInput;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::ipc::Event;
use ruviform_ui::view::{
    AttachEpochAllocator, ComponentKey, ComponentSnapshot, ViewEvent, ViewHost, ViewSpec,
};

use super::{
    MintedIntent, ScenarioCapability, ScenarioDriver, ScenarioError, ScenarioTarget,
    SemanticGuiTest, TargetQuery,
};

/// `ViewHost`-backed fake implementation of [`SemanticGuiTest`].
///
/// Windows are opened with [`open_window`](Self::open_window) and
/// views mounted with [`mount_view`](Self::mount_view); both mint
/// stable ids the consumer then addresses through [`TargetQuery`].
/// The adapter claims `SemanticAction` + `MultiWindow` only — it is
/// semantic evidence, never pointer/OS/pixel proof.
pub struct FakeGuiTest {
    /// `WindowId → ViewInstanceId → ViewHost`.
    windows: BTreeMap<WindowId, BTreeMap<ViewInstanceId, ViewHost>>,
    /// Session-scoped epoch authority — one per adapter, mirroring the
    /// consumer session contract on `ViewHost::new`.
    epochs: AttachEpochAllocator,
    /// Next window id to mint (ids start at 1, never reused).
    next_window: u64,
    /// `ViewEvent`s recorded by hosts since the last drain. Intents
    /// are partitioned out by [`SemanticGuiTest::drain_intents`]; the
    /// remainder stays available through
    /// [`drain_view_events`](Self::drain_view_events).
    pending: VecDeque<ViewEvent>,
}

impl Default for FakeGuiTest {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeGuiTest {
    /// An empty adapter with no windows.
    #[must_use]
    pub fn new() -> Self {
        Self {
            windows: BTreeMap::new(),
            epochs: AttachEpochAllocator::new(),
            next_window: 1,
            pending: VecDeque::new(),
        }
    }

    /// Opens a fresh window and returns its id.
    ///
    /// # Panics
    /// Never in practice — the counter starts at 1 and only grows.
    pub fn open_window(&mut self) -> WindowId {
        let id = WindowId::new(self.next_window).expect("window ids start at 1");
        self.next_window += 1;
        self.windows.entry(id).or_default();
        id
    }

    /// Mounts `spec` as `view` in `window`, minting a fresh
    /// [`AttachEpoch`] from the session allocator.
    ///
    /// # Errors
    /// [`ScenarioError::NoSuchWindow`] for an unknown window;
    /// [`ScenarioError::View`] when the spec fails validation.
    pub fn mount_view(
        &mut self,
        window: WindowId,
        view: ViewInstanceId,
        spec: ViewSpec,
    ) -> Result<(), ScenarioError> {
        let Some(slot) = self.windows.get_mut(&window) else {
            return Err(ScenarioError::NoSuchWindow { window });
        };
        let mut host = ViewHost::new(view, self.epochs.mint());
        host.mount(spec)?;
        self.pending.extend(host.drain_events());
        slot.insert(view, host);
        Ok(())
    }

    /// Replaces the mounted spec of `view` — bumps the host
    /// generation, so previously resolved [`ScenarioTarget`]s go stale.
    ///
    /// # Errors
    /// [`ScenarioError::NoSuchWindow`]/[`ScenarioError::NoSuchView`]
    /// for unknown ids; [`ScenarioError::View`] when the spec fails
    /// validation or the host rejects the update.
    pub fn update_view(
        &mut self,
        window: WindowId,
        view: ViewInstanceId,
        spec: ViewSpec,
    ) -> Result<(), ScenarioError> {
        let generation = self.host(window, view)?.generation();
        self.host_mut(window, view)?.update(spec, generation)?;
        let events = self
            .host_mut(window, view)
            .map(ViewHost::drain_events)
            .unwrap_or_default();
        self.pending.extend(events);
        Ok(())
    }

    /// Detaches `view` (dock transfer / window hide) — bumps the host
    /// generation and ends in-flight gestures per detach policy.
    ///
    /// # Errors
    /// [`ScenarioError::NoSuchWindow`]/[`ScenarioError::NoSuchView`]
    /// for unknown ids.
    pub fn detach_view(
        &mut self,
        window: WindowId,
        view: ViewInstanceId,
        reason: &str,
    ) -> Result<u64, ScenarioError> {
        let generation = self.host_mut(window, view)?.detach(reason);
        let events = self
            .host_mut(window, view)
            .map(ViewHost::drain_events)
            .unwrap_or_default();
        self.pending.extend(events);
        Ok(generation)
    }

    /// Drains recorded non-intent [`ViewEvent`]s (gestures, projection
    /// ops, unsupported notices) — the Fake adapter's equivalent of
    /// inspecting the host event stream directly.
    pub fn drain_view_events(&mut self) -> Vec<ViewEvent> {
        self.pending.drain(..).collect()
    }

    fn host(&self, window: WindowId, view: ViewInstanceId) -> Result<&ViewHost, ScenarioError> {
        self.windows
            .get(&window)
            .ok_or(ScenarioError::NoSuchWindow { window })?
            .get(&view)
            .ok_or(ScenarioError::NoSuchView { window, view })
    }

    fn host_mut(
        &mut self,
        window: WindowId,
        view: ViewInstanceId,
    ) -> Result<&mut ViewHost, ScenarioError> {
        self.windows
            .get_mut(&window)
            .ok_or(ScenarioError::NoSuchWindow { window })?
            .get_mut(&view)
            .ok_or(ScenarioError::NoSuchView { window, view })
    }

    /// Re-checks a resolved target's pins against the live host — the
    /// stale gate every operation passes through.
    fn check_target(&self, target: &ScenarioTarget) -> Result<(), ScenarioError> {
        let host = self.host(target.window, target.view)?;
        let got_epoch = host.attach_epoch();
        if got_epoch != target.attach_epoch {
            return Err(ScenarioError::StaleEpoch {
                expected: target.attach_epoch,
                got: got_epoch,
            });
        }
        let got_generation = host.generation();
        if got_generation != target.generation {
            return Err(ScenarioError::StaleGeneration {
                expected: target.generation,
                got: got_generation,
            });
        }
        Ok(())
    }

    /// Moves freshly recorded host events into `pending`.
    fn absorb(&mut self, window: WindowId, view: ViewInstanceId) {
        let events = self
            .host_mut(window, view)
            .map(ViewHost::drain_events)
            .unwrap_or_default();
        self.pending.extend(events);
    }
}

impl SemanticGuiTest for FakeGuiTest {
    fn driver(&self) -> ScenarioDriver {
        ScenarioDriver::Fake
    }

    fn capabilities(&self) -> BTreeSet<ScenarioCapability> {
        BTreeSet::from([
            ScenarioCapability::SemanticAction,
            ScenarioCapability::MultiWindow,
        ])
    }

    fn windows(&self) -> Vec<WindowId> {
        self.windows.keys().copied().collect()
    }

    fn resolve_target(&mut self, query: &TargetQuery) -> Result<ScenarioTarget, ScenarioError> {
        let host = self.host(query.window, query.view)?;
        if let Some(expected) = query.attach_epoch
            && host.attach_epoch() != expected
        {
            return Err(ScenarioError::StaleEpoch {
                expected,
                got: host.attach_epoch(),
            });
        }
        if let Some(expected) = query.generation
            && host.generation() != expected
        {
            return Err(ScenarioError::StaleGeneration {
                expected,
                got: host.generation(),
            });
        }
        if let Some(key) = &query.component
            && host.observe(key).is_none()
        {
            return Err(ScenarioError::UnknownComponent { key: key.clone() });
        }
        Ok(ScenarioTarget {
            window: query.window,
            view: query.view,
            attach_epoch: host.attach_epoch(),
            generation: host.generation(),
            component: query.component.clone(),
        })
    }

    fn dispatch_input(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        input: ControlInput,
    ) -> Result<(), ScenarioError> {
        self.check_target(target)?;
        let generation = self.host(target.window, target.view)?.generation();
        self.host_mut(target.window, target.view)?
            .dispatch_input(generation, key, input)?;
        self.absorb(target.window, target.view);
        Ok(())
    }

    fn select_row(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        row: usize,
    ) -> Result<(), ScenarioError> {
        self.check_target(target)?;
        let generation = self.host(target.window, target.view)?.generation();
        self.host_mut(target.window, target.view)?
            .select_row(generation, key, row)?;
        self.absorb(target.window, target.view);
        Ok(())
    }

    fn select_tab(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        index: usize,
    ) -> Result<(), ScenarioError> {
        self.check_target(target)?;
        let generation = self.host(target.window, target.view)?.generation();
        self.host_mut(target.window, target.view)?
            .select_tab(generation, key, index)?;
        self.absorb(target.window, target.view);
        Ok(())
    }

    fn invoke(&mut self, target: &ScenarioTarget, key: &ComponentKey) -> Result<(), ScenarioError> {
        self.check_target(target)?;
        let generation = self.host(target.window, target.view)?.generation();
        self.host_mut(target.window, target.view)?
            .invoke(generation, key)?;
        self.absorb(target.window, target.view);
        Ok(())
    }

    fn observe(
        &self,
        target: &ScenarioTarget,
        key: &ComponentKey,
    ) -> Result<ComponentSnapshot, ScenarioError> {
        self.check_target(target)?;
        self.host(target.window, target.view)?
            .observe(key)
            .ok_or_else(|| ScenarioError::UnknownComponent { key: key.clone() })
    }

    fn settle(&mut self) -> Result<(), ScenarioError> {
        // `ViewHost` is synchronous — every mutation already recorded
        // its events, which `absorb` moved into `pending`. Settle only
        // guarantees nothing host-side is left undrained.
        for window in self.windows.keys().copied().collect::<Vec<_>>() {
            let views: Vec<ViewInstanceId> = self
                .windows
                .get(&window)
                .map(|slot| slot.keys().copied().collect())
                .unwrap_or_default();
            for view in views {
                self.absorb(window, view);
            }
        }
        Ok(())
    }

    fn drain_intents(&mut self) -> Vec<MintedIntent> {
        let mut intents = Vec::new();
        self.pending.retain(|event| {
            let Some(minted) = minted_intent(event) else {
                return true;
            };
            intents.push(minted);
            false
        });
        intents
    }

    fn post_event(&mut self, target: &ScenarioTarget, event: &Event) -> Result<(), ScenarioError> {
        self.check_target(target)?;
        self.host_mut(target.window, target.view)?
            .deliver_event(event);
        self.absorb(target.window, target.view);
        Ok(())
    }
}

/// Extracts the [`MintedIntent`] from a [`ViewEvent::Intent`]; `None`
/// for every other event kind (kept pending for `drain_view_events`).
fn minted_intent(event: &ViewEvent) -> Option<MintedIntent> {
    match event {
        ViewEvent::Intent {
            intent,
            command_id,
            expect_receipt,
        } => Some(MintedIntent {
            command_id: command_id.clone(),
            expect_receipt: *expect_receipt,
            intent: intent.clone(),
        }),
        _ => None,
    }
}
