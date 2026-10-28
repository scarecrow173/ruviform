//! `supervisor` — the opt-in L1 crash-recovery util layer (Issue
//! #116; `docs/designs/issue-116-supervisor.md` is the authoritative
//! contract).
//!
//! The core [`PluginHost`] stays restart-free (AC4: it reports
//! [`HostNotification::InstanceFailed`] and leaves the decision to the
//! caller). [`Supervisor`] is the opt-in machinery a caller chooses to
//! drive for a watched handle: [`Supervisor::watch`] adopts the
//! handle, [`Supervisor::poll`] — called from the caller's non-RT
//! control loop with an injected `now` — drains the notification
//! queue, destroys the dead instance, replays `Instantiate` →
//! `Initialize` → `RestoreState` (checkpoint) → `Activate` →
//! `StartProcessing` per the entry's [`RestartRecipe`], and reports
//! [`SupervisionEvent`]s. Bounded retries (`RestartPolicy` exponential
//! backoff + `stable_window` episode reset + `max_attempts_per_poll`
//! burst cap) end in [`SupervisionEvent::GaveUp`], a separate
//! vocabulary from [`HostError::Failed`].
//!
//! **Only the public `PluginHost` API is used** — `execute`,
//! `try_recv_notification` and `lifecycle_state`. No `pub(crate)` L1
//! machinery (`l1::`, `Route`/`Slot`/pool internals) is touched: that
//! boundary is a design contract, not a compiler-enforced one, so this
//! module's imports and its tests stay on the public surface.
//!
//! **Drive contract:** the supervisor owns no host, thread, timer or
//! async runtime — `poll(&mut host, now)` does all work synchronously
//! on the caller's thread, so it composes with `PluginHost`'s
//! `!Send`/`!Sync`. **`poll` is the sole `try_recv_notification`
//! drain point** for the bound host — see its doc for the WARNING.
//! One `Supervisor` binds to one `PluginHost` (first host-receiving
//! call records the pointer as a `usize`); every later foreign-host
//! call is the defined rejection `WrongHost` without draining or
//! attempting — it must not steal another supervisor's notifications.

use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, Instant};

use ruviform_plugin_core::{LifecycleState, ProcessConfig, wire};

use crate::{
    EndpointBounds, HostCommand, HostError, HostNotification, HostReply, PluginHandle, PluginHost,
};

/// Byte overhead one wire command frame adds around its payload —
/// `seq` 8 + `instance` 4 + `verb` 1 + `len` prefix 4 = 17 B. The
/// `set_checkpoint` bound exists so a `RestoreState` command built
/// from a registered blob always fits a single
/// `wire::WIRE_MAX_PAYLOAD` frame: the encoder does not check
/// `WIRE_MAX_*` bounds itself, and an oversized frame fails the
/// runner's `read_frame`/`decode` and kills the whole child
/// (child-scope `Failed` + fan-out — a repeated kill every attempt).
const WIRE_COMMAND_ENVELOPE: usize = 17;

/// The opt-in crash-recovery driver — watches [`PluginHandle`]s on one
/// bound [`PluginHost`] and replays their recovery.
///
/// A `Supervisor` holds only sendable bookkeeping (`entries`, an id
/// counter, the [`RestartPolicy`] and the `usize` host binding) — it
/// never owns the host, so `Send`/`Sync` stay intact while
/// `PluginHost` remains `!Send`/`!Sync`. The host is borrowed per
/// call; drive [`Supervisor::poll`] from the same non-RT control loop
/// that owns the host.
///
/// **RT contract:** every operation is non-RT — `poll` drains
/// notifications, destroys dead instances and replays
/// spawn/handshake/lifecycle commands with reply waits.
pub struct Supervisor {
    /// Per-supervised logical instance bookkeeping, keyed by the
    /// supervisor-minted [`SupervisedId`] (stable across handle
    /// remints — restart always mints a fresh handle).
    entries: BTreeMap<SupervisedId, Entry>,
    /// Id allocator — monotonically increasing, wraps only after
    /// 2^64 `watch` calls (unreachable in practice).
    next_id: u64,
    /// The supervisor-wide restart policy — per-entry overrides are
    /// a deferred concern.
    policy: RestartPolicy,
    /// The bound host's address (`host as *const PluginHost as
    /// usize`), set on the first host-receiving call — `usize`, not a
    /// raw pointer, so `Send`/`Sync` stay derivable. Address identity
    /// is deliberately strict: moving the host value reads as a
    /// foreign host and is rejected, the safe direction.
    bound_host: Option<usize>,
}

/// Opaque logical key for one watched instance — stable across the
/// `PluginHandle` remint every restart performs.
///
/// `Ord` is part of the contract: attempt issuance sorts ready
/// entries by `(next_attempt_at, id)` for determinism.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SupervisedId(u64);

/// Bounded-retry policy shared by every entry on one [`Supervisor`].
///
/// `failures` counts attempts consumed inside one episode — a
/// successful `Restarted` does **not** reset it; an entry that stays
/// alive `stable_window` after its last restart resets `failures`
/// (and `last_error`) to zero, so sporadic crashes recover unbounded
/// while crash-loops stay bounded. The same predicate also runs at
/// drain time, before the give-up check — a crash landing past the
/// window resets the episode first instead of giving up on stale
/// `failures`.
#[derive(Clone, Debug)]
pub struct RestartPolicy {
    /// Total recovery attempts an episode may consume — `0` means
    /// "report failures but never attempt": a watched `InstanceFailed`
    /// still drains/destroys, then gives up immediately
    /// (`GaveUp { attempts: 0, last_error: None }`).
    pub max_attempts: u32,
    /// First backoff delay after an attempt failure —
    /// `delay(k) = min(base_delay * 2^(k-1), max_delay)` for `k`
    /// consumed attempts.
    pub base_delay: Duration,
    /// Backoff ceiling.
    pub max_delay: Duration,
    /// Episode boundary: `failures`/`last_error` reset once a
    /// `Restarted` instance stays alive this long. `0` resets on every
    /// restart — crash-loops then retry forever (documented edge).
    pub stable_window: Duration,
    /// Attempts issued per `poll` — bounds a bundle-crash burst's
    /// spawn concentration; overflow `Pending` entries carry to the
    /// next poll (they emit no event until their attempt runs). `0`
    /// makes the supervisor inert, so [`Supervisor::new`] rejects it
    /// with [`SupervisionError::InertPolicy`] rather than silently
    /// clamping the caller's intent.
    pub max_attempts_per_poll: u32,
}

impl Default for RestartPolicy {
    /// `max_attempts` 3, `base_delay` 250 ms, `max_delay` 30 s,
    /// `stable_window` 60 s, `max_attempts_per_poll` 8.
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(30),
            stable_window: Duration::from_secs(60),
            max_attempts_per_poll: 8,
        }
    }
}

/// The recovery declaration one watched instance replays —
/// registered at [`Supervisor::watch`], fully replaceable via
/// [`Supervisor::set_recipe`].
#[derive(Clone, Debug)]
pub struct RestartRecipe {
    /// The `plugin_id` handed to `Instantiate` — the caller's
    /// registration-time claim, verified only as "registered" by
    /// `Scan` at `watch`/`set_recipe`. Whether the *watched handle*
    /// really belongs to that plugin is unverifiable caller
    /// responsibility (the host exposes no `handle→plugin_id` lookup):
    /// misdeclaration resurrects a different plugin
    /// (cross-plugin resurrection).
    pub plugin_id: String,
    /// `Some` replays `Activate { config }` after `Initialize`;
    /// `None` stops the recovered instance at `Initialized`.
    pub config: Option<ProcessConfig>,
    /// Replays `StartProcessing` after a successful `Activate` — the
    /// effective guard is `config.is_some() && start_processing`:
    /// with `config: None` no `Activate` runs, `StartProcessing` is
    /// not issued (`Initialized` rejects it with `Err(Lifecycle)`),
    /// and the flag is ignored.
    pub start_processing: bool,
    /// Caller-side endpoint re-attach bounds, echoed verbatim into
    /// [`SupervisionEvent::Restarted`]'s `bounds` — the supervisor
    /// never calls `prepare_endpoint` (a `PluginEndpoint` is `Send`
    /// and must move to the RT-side owner; no hand-off path exists).
    /// Meaningful only when `config` is `Some` — an `Initialized`
    /// instance rejects `prepare_endpoint` with `Err(Lifecycle)`, so
    /// the echo is dead data there.
    pub endpoint_bounds: Option<EndpointBounds>,
}

impl RestartRecipe {
    /// The minimal recipe: recover `plugin_id` up to `Initialized`
    /// (`config: None`, `start_processing: false`,
    /// `endpoint_bounds: None`). Set `config`/`start_processing` for
    /// a `Processing` recovery.
    #[must_use]
    pub fn new(plugin_id: impl Into<String>) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            config: None,
            start_processing: false,
            endpoint_bounds: None,
        }
    }
}

/// The checkpoint application result echoed by
/// [`SupervisionEvent::Restarted`]'s `restored` — emitted only on a
/// successful attempt.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RestoreOutcome {
    /// No checkpoint was registered — `RestoreState` was not issued;
    /// the instance recovered on its plugin-default state.
    NotRequested,
    /// `RestoreState` returned `Ok` — the registered blob applied.
    Applied,
    /// `RestoreState` returned `Err(Unsupported)` — the plugin has no
    /// state support, so the restore was skipped **without consuming
    /// the attempt**; the checkpoint stays registered for the next
    /// attempt.
    Skipped,
}

/// One recovery fact surfaced by [`Supervisor::poll`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy; each struct variant is `#[non_exhaustive]` too, so new
/// fields stay additive for downstream matchers.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum SupervisionEvent {
    /// A watched entry recovered: `old` is the dead handle (now
    /// `Err(InvalidHandle)`-stale — generation-bumped on remint),
    /// `new` the freshly minted one. The new instance has **no
    /// endpoint attached** — call `prepare_endpoint` with `bounds`
    /// (the recipe's `endpoint_bounds` echo) and re-wire handle
    /// bookkeeping yourself.
    #[non_exhaustive]
    Restarted {
        /// The entry's stable logical key.
        id: SupervisedId,
        /// The dead instance's handle.
        old: PluginHandle,
        /// The replacement instance's handle.
        new: PluginHandle,
        /// Recipe `endpoint_bounds` echo — caller's `prepare_endpoint`
        /// input (`None` unless declared).
        bounds: Option<EndpointBounds>,
        /// Checkpoint application result of this attempt.
        restored: RestoreOutcome,
    },
    /// One recovery attempt failed — `attempt` is the episode's
    /// consumed-attempt count after this failure. A backoff delay was
    /// scheduled (or `GaveUp` follows in this same event batch when
    /// the episode budget just exhausted).
    #[non_exhaustive]
    AttemptFailed {
        /// The entry's stable logical key.
        id: SupervisedId,
        /// Consumed attempts including this one.
        attempt: u32,
        /// Why the attempt failed — the first `Err` of the replay.
        error: HostError,
    },
    /// The episode's attempt budget is exhausted — terminal until
    /// the caller `unwatch`es (entries are kept, never auto-removed).
    /// Any minted residue was unwound/`Destroy`ed or is exposed via
    /// `current_handle` as lingering. A re-emit (same `attempts` and
    /// `last_error`) reports a lingering handle's late death —
    /// dedupe on `(id, attempts)` if the first `GaveUp` and that
    /// report must be told apart.
    #[non_exhaustive]
    GaveUp {
        /// The entry's stable logical key.
        id: SupervisedId,
        /// Attempts the episode consumed.
        attempts: u32,
        /// The episode's last attempt error — `None` when give-up
        /// happened with no attempt issued (`max_attempts` 0).
        last_error: Option<HostError>,
    },
    /// A notification for a handle no entry watches — pass-through,
    /// since `poll` is the sole drain point and unwatched instances
    /// stay the caller's manual-recovery responsibility.
    #[non_exhaustive]
    Unsupervised {
        /// The drained notification, unmodified.
        notification: HostNotification,
    },
    /// `poll` was called on a host different from the bound one —
    /// nothing was drained, reset or attempted (the foreign host's
    /// notifications stay queued for their own supervisor). An
    /// event rather than an `Err` because `poll`'s signature is
    /// `-> Vec<SupervisionEvent>`.
    WrongHost,
}

/// The defined rejections of [`Supervisor`] calls.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy and stays `Copy` — future payloads are limited to `Copy`
/// types; dropping `Copy` for a richer payload is a separate
/// breaking-change decision.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SupervisionError {
    /// `lifecycle_state` returned `None` — a stale or unknown handle.
    InvalidHandle,
    /// An entry already watches a matching handle — the match set is
    /// the entry's current `Live` handle, `Pending.failed` and any
    /// state's `lingering` residue.
    AlreadyWatched,
    /// `recipe.plugin_id` is not in `Scan`'s registered descriptors
    /// (the only check possible — registration, not ownership).
    UnknownPluginId,
    /// The `SupervisedId` names no entry.
    NotWatched,
    /// The call targeted a host different from the bound one — one
    /// `Supervisor` serves one `PluginHost`.
    WrongHost,
    /// `set_checkpoint`'s blob cannot fit one `RestoreState` wire
    /// frame (`bytes.len() + WIRE_COMMAND_ENVELOPE >
    /// wire::WIRE_MAX_PAYLOAD`). Rejected before the wire: an
    /// oversized frame kills the whole child (`Failed` + fan-out),
    /// turning recovery into a repeated kill loop.
    CheckpointTooLarge,
    /// `max_attempts_per_poll == 0` — an inert policy that can never
    /// issue an attempt. Rejected rather than clamped: rewriting `0`
    /// to a default changes the caller's intent.
    InertPolicy,
}

impl fmt::Display for SupervisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle => f.write_str("stale or unknown plugin handle"),
            Self::AlreadyWatched => f.write_str("handle is already watched"),
            Self::UnknownPluginId => f.write_str("plugin id is not registered"),
            Self::NotWatched => f.write_str("supervised id is not watched"),
            Self::WrongHost => f.write_str("supervisor is bound to a different plugin host"),
            Self::CheckpointTooLarge => f.write_str("checkpoint blob exceeds the wire frame limit"),
            Self::InertPolicy => f.write_str("policy allows no attempts per poll"),
        }
    }
}

impl std::error::Error for SupervisionError {}

/// Per-entry bookkeeping shared by every state — the recipe the next
/// attempt replays, the caller-pushed checkpoint, the episode
/// counters (`failures`/`last_error`) and the cleanup residue
/// (`lingering`) all live across state transitions.
struct Entry {
    /// What the next attempt replays (`watch`/`set_recipe` owned).
    recipe: RestartRecipe,
    /// Caller-registered pre-failure `SaveState` blob — `id`-keyed,
    /// so it applies to every remint unconditionally. Always inside
    /// the `set_checkpoint` bound, so the replayed `RestoreState`
    /// frame always fits `wire::WIRE_MAX_PAYLOAD`.
    checkpoint: Option<Vec<u8>>,
    /// Attempts consumed inside this episode — survives `Restarted`
    /// (restart is not a reset) and clears only on `stable_window`
    /// survival. Give-up fires at `failures >= max_attempts`.
    failures: u32,
    /// The episode's most recent attempt error — the payload source
    /// for `AttemptFailed`/`GaveUp.last_error`.
    last_error: Option<HostError>,
    /// A live minted handle the unwind could not destroy — kept so
    /// it neither leaks nor orphans: `current_handle` exposes it and
    /// every `poll` retries its cleanup (Phase 2). Carried over a
    /// `Pending → Live` transition; cleared by `Ok`/`Err(InvalidHandle)`
    /// (externally destroyed) cleanup or a matching late-death
    /// notification.
    lingering: Option<PluginHandle>,
    /// The state machine position.
    state: EntryState,
}

/// The per-entry recovery state machine: `Live → Pending → (Live |
/// GaveUp)`; `GaveUp` is terminal until `unwatch`.
#[derive(Copy, Clone, Debug)]
enum EntryState {
    /// Watched and alive. `healthy_since` is `Some(now)` of the last
    /// `Restarted` success — `None` after `watch` (which takes no
    /// `Instant`; `failures` can only grow through a `Restarted`, so
    /// the reset predicate never evaluates on `None`).
    Live {
        /// The current minted handle — the match set's "current".
        handle: PluginHandle,
        /// Last `Restarted` timestamp, for the `stable_window` check.
        healthy_since: Option<Instant>,
    },
    /// Dead, waiting for its retry window. `next_attempt_at` `None`
    /// means "ready immediately" — the `watch`-adopt path, which
    /// receives no `Instant`; notification-driven entries get
    /// `Some(now)` (first attempt immediate) or `Some(now + delay)`
    /// (post-failure backoff). `failed` is the dead handle —
    /// `Restarted.old`'s echo source — and the match set's "failed".
    Pending {
        /// Earliest `poll`-time the next attempt may issue.
        next_attempt_at: Option<Instant>,
        /// The dead handle this entry is recovering.
        failed: PluginHandle,
    },
    /// Episode budget exhausted — terminal bookkeeping (`unwatch` to
    /// remove). `failures`/`last_error`/`lingering` stay live for
    /// `GaveUp` re-emits and `current_handle` exposure.
    GaveUp,
}

/// Which match-set member `find_watched` hit — the drain handler
/// needs the distinction: only a `Live` current-handle death runs the
/// give-up check, and only a `Lingering` hit clears `lingering`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum MatchKind {
    /// `Live.handle` — the watched current instance died.
    Current,
    /// `Pending.failed` — the already-dead handle reported again
    /// (unreachable under exactly-once delivery, harmless anyway).
    Failed,
    /// `lingering` residue of any state — the cleanup survivor died.
    Lingering,
}

/// Why one attempt failed, plus the minted handle it left for
/// cleanup (`None` when `Instantiate` itself failed — nothing to
/// unwind).
struct AttemptFailure {
    /// The first `Err` of the replay.
    error: HostError,
    /// The handle `Instantiate` minted before the failure, if any.
    minted: Option<PluginHandle>,
}

/// `now >= since + window` with overflow-safe arithmetic — an
/// unrepresentable deadline reads as "not yet past" (the window check
/// stays conservative; `checked_add` never panics on outlier input).
fn deadline_passed(since: Instant, window: Duration, now: Instant) -> bool {
    since
        .checked_add(window)
        .is_some_and(|deadline| now >= deadline)
}

/// `delay(k) = min(base_delay * 2^(k-1), max_delay)` — `k` is the
/// consumed-attempt count (`failures` after increment), so attempt 1
/// waits `base_delay`. The exponent saturates at 31; a
/// `Duration::checked_mul` overflow clamps to `max_delay`.
fn attempt_delay(policy: &RestartPolicy, failures: u32) -> Duration {
    let shift = failures.saturating_sub(1).min(31);
    // `shift <= 31 < 32` by construction — the shift cannot overflow.
    let factor = 1u32 << shift;
    policy
        .base_delay
        .checked_mul(factor)
        .map_or(policy.max_delay, |delay| delay.min(policy.max_delay))
}

impl Supervisor {
    /// Creates a supervisor under `policy`.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`SupervisionError::InertPolicy`] when
    /// `policy.max_attempts_per_poll` is `0` — an attempt can never
    /// issue, so the supervisor would watch-and-drain forever without
    /// recovering anything. Rejected rather than clamped.
    pub fn new(policy: RestartPolicy) -> Result<Self, SupervisionError> {
        if policy.max_attempts_per_poll == 0 {
            return Err(SupervisionError::InertPolicy);
        }
        Ok(Self {
            entries: BTreeMap::new(),
            next_id: 0,
            policy,
            bound_host: None,
        })
    }

    /// Starts watching `handle` and returns its stable
    /// [`SupervisedId`].
    ///
    /// Validation order: live check (`lifecycle_state` `None` →
    /// `InvalidHandle`), registration check (`Scan` has
    /// `recipe.plugin_id`, else `UnknownPluginId` — proof of
    /// registration only; whether the handle belongs to that plugin
    /// is unverifiable caller responsibility), duplicate check (the
    /// handle already appears in some entry's match set →
    /// `AlreadyWatched`).
    ///
    /// A live handle registers `Live` — note an L0 handle is legal
    /// but inert (L0 never notifies; only the `Failed`-adopt path
    /// below can still recover one). A `Failed` handle is **adopted**
    /// instead: `Destroy` frees its slot (dropping any still-queued
    /// `InstanceFailed` with it — no double recovery), then the entry
    /// registers `Pending { next_attempt_at: None }` so the next
    /// `poll` attempts immediately even if the caller already
    /// consumed the notification. A `Destroy` error is absorbed —
    /// adoption continues, the slot release is best-effort.
    ///
    /// The first host-receiving call binds `bound_host`; later calls
    /// on a different host are rejected `Err(WrongHost)`.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`SupervisionError`] — `WrongHost`/`InvalidHandle`/
    /// `UnknownPluginId`/`AlreadyWatched` per the checks above.
    pub fn watch(
        &mut self,
        host: &mut PluginHost,
        handle: PluginHandle,
        recipe: RestartRecipe,
    ) -> Result<SupervisedId, SupervisionError> {
        self.bind_host(host)?;
        let state = host
            .lifecycle_state(handle)
            .ok_or(SupervisionError::InvalidHandle)?;
        Self::scan_plugin_id(host, &recipe.plugin_id)?;
        if self.find_watched(handle).is_some() {
            return Err(SupervisionError::AlreadyWatched);
        }
        let state = if state == LifecycleState::Failed {
            // Adopt: free the failed slot — its still-queued
            // `InstanceFailed` drops with it, so no later drain can
            // double-recover this handle. `Err` absorbed (e.g. an
            // external `Destroy` already freed it) — the caller's
            // intent is recovery, freeing is best-effort.
            let _ = host.execute(HostCommand::Destroy { handle });
            EntryState::Pending {
                next_attempt_at: None,
                failed: handle,
            }
        } else {
            EntryState::Live {
                handle,
                healthy_since: None,
            }
        };
        let id = SupervisedId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        self.entries.insert(
            id,
            Entry {
                recipe,
                checkpoint: None,
                failures: 0,
                last_error: None,
                lingering: None,
                state,
            },
        );
        Ok(id)
    }

    /// Removes the entry and returns the handle the caller should
    /// keep managing — `Some(current)` for `Live`, `Some(lingering)`
    /// for `Pending`/`GaveUp` carrying cleanup residue, `None` for an
    /// unknown id or a residue-free entry. The returned instance is
    /// **not** destroyed (supervision stops, the instance lives on —
    /// `Destroy`/salvage is the caller's call). `None` alone cannot
    /// distinguish "unknown id" from "no live handle" — query
    /// [`Supervisor::current_handle`] first if that matters.
    ///
    /// A `Live` entry can still carry lingering cleanup residue —
    /// returning the current handle abandons it to the caller's host
    /// management (only one handle can be returned).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn unwatch(&mut self, id: SupervisedId) -> Option<PluginHandle> {
        let entry = self.entries.remove(&id)?;
        match entry.state {
            EntryState::Live { handle, .. } => Some(handle),
            _ => entry.lingering,
        }
    }

    /// Replaces `id`'s recipe wholesale — the `plugin_id` passes the
    /// same `Scan` registration check `watch` performs.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`SupervisionError::WrongHost`] on a foreign host,
    /// [`SupervisionError::NotWatched`] for an unknown id,
    /// [`SupervisionError::UnknownPluginId`] for an unregistered id.
    pub fn set_recipe(
        &mut self,
        host: &mut PluginHost,
        id: SupervisedId,
        recipe: RestartRecipe,
    ) -> Result<(), SupervisionError> {
        self.bind_host(host)?;
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or(SupervisionError::NotWatched)?;
        Self::scan_plugin_id(host, &recipe.plugin_id)?;
        entry.recipe = recipe;
        Ok(())
    }

    /// Registers the caller's pre-failure `SaveState` blob for `id` —
    /// applied by `RestoreState` on every subsequent recovery (`id`-
    /// keyed, so reminted handles inherit it). `Failed` makes
    /// `SaveState` impossible, so checkpointing is push-only: the
    /// caller decides save timing (the supervisor never runs
    /// `SaveState` inside `poll`).
    ///
    /// **RT contract:** non-RT only — stores the blob.
    ///
    /// # Errors
    ///
    /// [`SupervisionError::CheckpointTooLarge`] when the blob cannot
    /// fit one `RestoreState` frame — rejected before the wire so an
    /// oversized frame never kills the child; `SaveState` output
    /// itself already fits the shared codec bound.
    /// [`SupervisionError::NotWatched`] for an unknown id.
    pub fn set_checkpoint(
        &mut self,
        id: SupervisedId,
        bytes: Vec<u8>,
    ) -> Result<(), SupervisionError> {
        if bytes.len().saturating_add(WIRE_COMMAND_ENVELOPE) > wire::WIRE_MAX_PAYLOAD as usize {
            return Err(SupervisionError::CheckpointTooLarge);
        }
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or(SupervisionError::NotWatched)?;
        entry.checkpoint = Some(bytes);
        Ok(())
    }

    /// Drops `id`'s registered checkpoint — later recoveries skip
    /// `RestoreState`. Returns whether a checkpoint existed;
    /// `false` for an unknown id.
    ///
    /// **RT contract:** non-RT only.
    pub fn clear_checkpoint(&mut self, id: SupervisedId) -> bool {
        self.entries
            .get_mut(&id)
            .is_some_and(|entry| entry.checkpoint.take().is_some())
    }

    /// The handle the caller may still act on: `Some(current)` for
    /// `Live` (current wins over lingering residue — residue is an
    /// internal cleanup concern), `Some(lingering)` for
    /// `Pending`/`GaveUp` carrying a live cleanup survivor so the
    /// caller can `Destroy`/salvage it instead of orphaning it,
    /// `None` otherwise.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn current_handle(&self, id: SupervisedId) -> Option<PluginHandle> {
        let entry = self.entries.get(&id)?;
        match entry.state {
            EntryState::Live { handle, .. } => Some(handle),
            _ => entry.lingering,
        }
    }

    /// One supervision tick: host-binding check → notification drain
    /// → `stable_window` sweep + lingering cleanup retry → capped
    /// attempt issuance, returning every [`SupervisionEvent`] raised.
    ///
    /// Inject `Instant::now()` (or a test value) from the caller's
    /// non-RT control loop; `now` must be non-decreasing — deadlines
    /// resolve at `poll` granularity, so a backwards `now` only warps
    /// backoff/window decisions (caller contract, unchecked).
    /// Attempts a `max_attempts_per_poll` cap leaves pending emit no
    /// event until their attempt runs (`current_handle` observes the
    /// `Pending` state as `None`).
    ///
    /// **WARNING — sole drain point:** this call is the bound host's
    /// only `try_recv_notification` drain. Draining notifications
    /// outside `poll` loses a watched `InstanceFailed` — the entry
    /// then points at a dead instance while stuck `Live` and neither
    /// recovers nor gives up (silent unrecoverable loss). Unwatched
    /// handles are never lost either way: they pass through as
    /// [`SupervisionEvent::Unsupervised`].
    ///
    /// A foreign host yields [`SupervisionEvent::WrongHost`] only —
    /// nothing is drained, reset or attempted on it.
    ///
    /// **RT contract:** non-RT only — commands wait on replies.
    #[must_use]
    pub fn poll(&mut self, host: &mut PluginHost, now: Instant) -> Vec<SupervisionEvent> {
        let mut events = Vec::new();
        // Phase 0 — host binding: the first host-receiving call
        // records the address; a mismatch rejects without touching
        // the foreign host (its queue belongs to another drainer).
        if self.bind_host(host).is_err() {
            events.push(SupervisionEvent::WrongHost);
            return events;
        }
        // Phase 1 — notification drain: the bound host's sole drain
        // point (`try_recv_notification` also runs the endpoint-leg
        // housekeeping drain).
        while let Some(notification) = host.try_recv_notification() {
            if let Some(event) = self.handle_notification(host, now, notification) {
                events.push(event);
            }
        }
        // Phase 2 — `stable_window` sweep on `Live` entries + lingering
        // cleanup retry. The drain already applied the same reset
        // predicate to entries its notifications touched (a death
        // notification moves `Live` out of this sweep's reach), so a
        // plain sweep over the survivors is enough — the reset is
        // idempotent.
        for entry in self.entries.values_mut() {
            if let EntryState::Live {
                healthy_since: Some(since),
                ..
            } = entry.state
                && entry.failures > 0
                && deadline_passed(since, self.policy.stable_window, now)
            {
                entry.failures = 0;
                entry.last_error = None;
            }
            if let Some(handle) = entry.lingering {
                match Self::cleanup_instance(host, handle) {
                    Ok(()) | Err(HostError::InvalidHandle) => entry.lingering = None,
                    Err(_) => {}
                }
            }
        }
        // Phase 3 — attempt issuance: ready entries (`None` deadline =
        // immediate, else `now >= next_attempt_at`) run in
        // `(next_attempt_at, id)` order — `None` sorts first — capped
        // at `max_attempts_per_poll`; overflow stays `Pending` for the
        // next poll with no event.
        let mut ready: Vec<(Option<Instant>, SupervisedId)> = self
            .entries
            .iter()
            .filter_map(|(id, entry)| match entry.state {
                EntryState::Pending {
                    next_attempt_at, ..
                } if next_attempt_at.is_none_or(|deadline| now >= deadline) => {
                    Some((next_attempt_at, *id))
                }
                _ => None,
            })
            .collect();
        ready.sort_unstable();
        let mut budget = self.policy.max_attempts_per_poll;
        for (_deadline, id) in ready {
            if budget == 0 {
                break;
            }
            let Some(entry) = self.entries.get_mut(&id) else {
                continue;
            };
            let EntryState::Pending { failed, .. } = entry.state else {
                continue;
            };
            budget -= 1;
            match Self::attempt(host, entry) {
                Ok((new, restored)) => {
                    entry.state = EntryState::Live {
                        handle: new,
                        healthy_since: Some(now),
                    };
                    events.push(SupervisionEvent::Restarted {
                        id,
                        old: failed,
                        new,
                        bounds: entry.recipe.endpoint_bounds,
                        restored,
                    });
                }
                Err(AttemptFailure { error, minted }) => {
                    entry.failures = entry.failures.saturating_add(1);
                    entry.last_error = Some(error);
                    if let Some(minted) = minted {
                        Self::unwind_minted(host, entry, minted);
                    }
                    if entry.failures >= self.policy.max_attempts {
                        entry.state = EntryState::GaveUp;
                        events.push(SupervisionEvent::GaveUp {
                            id,
                            attempts: entry.failures,
                            last_error: entry.last_error,
                        });
                    } else {
                        let next = now
                            .checked_add(attempt_delay(&self.policy, entry.failures))
                            .unwrap_or(now);
                        entry.state = EntryState::Pending {
                            next_attempt_at: Some(next),
                            failed,
                        };
                        events.push(SupervisionEvent::AttemptFailed {
                            id,
                            attempt: entry.failures,
                            error,
                        });
                    }
                }
            }
        }
        events
    }

    /// The 1 `Supervisor` = 1 `PluginHost` binding: the first
    /// host-receiving call records the host's address as a `usize`
    /// (a raw pointer would break `Send`/`Sync`); later foreign hosts
    /// are the defined rejection `WrongHost`.
    fn bind_host(&mut self, host: &PluginHost) -> Result<(), SupervisionError> {
        let key = std::ptr::from_ref(host) as usize;
        match self.bound_host {
            None => {
                self.bound_host = Some(key);
                Ok(())
            }
            Some(bound) if bound == key => Ok(()),
            Some(_) => Err(SupervisionError::WrongHost),
        }
    }

    /// `recipe.plugin_id` ∈ `Scan` — proof of registration only.
    /// `Scan` cannot fail on the composite surface, so an `Err` or a
    /// non-`Scanned` reply still lands on the defined rejection.
    fn scan_plugin_id(host: &mut PluginHost, plugin_id: &str) -> Result<(), SupervisionError> {
        let Ok(HostReply::Scanned(descriptors)) = host.execute(HostCommand::Scan) else {
            return Err(SupervisionError::UnknownPluginId);
        };
        if descriptors.iter().any(|d| d.id == plugin_id) {
            Ok(())
        } else {
            Err(SupervisionError::UnknownPluginId)
        }
    }

    /// Looks up which entry (if any) claims `handle` — the match set
    /// is `Live.handle` (current), `Pending.failed` and any state's
    /// `lingering`; `GaveUp` entries match only via `lingering`.
    fn find_watched(&self, handle: PluginHandle) -> Option<(SupervisedId, MatchKind)> {
        self.entries.iter().find_map(|(id, entry)| {
            let kind = match entry.state {
                EntryState::Live {
                    handle: current, ..
                } if current == handle => MatchKind::Current,
                EntryState::Pending { failed, .. } if failed == handle => MatchKind::Failed,
                _ if entry.lingering == Some(handle) => MatchKind::Lingering,
                _ => return None,
            };
            Some((*id, kind))
        })
    }

    /// Phase-1 handler for one drained notification — separated from
    /// the `try_recv_notification` loop so unit tests can drive it
    /// directly (an L0 host never produces `InstanceFailed` — the
    /// queue carries L1 `Failed` transitions only). Returns the single
    /// event the notification can raise, if any.
    fn handle_notification(
        &mut self,
        host: &mut PluginHost,
        now: Instant,
        notification: HostNotification,
    ) -> Option<SupervisionEvent> {
        // `#[allow]` instead of `#[expect]`: `HostNotification` is
        // `#[non_exhaustive]`, so this let-else is irrefutable only
        // while `InstanceFailed` stays the sole variant — a future
        // variant makes the `else` live and must pass through as
        // `Unsupervised`, not break the build.
        #[allow(irrefutable_let_patterns)]
        let HostNotification::InstanceFailed { handle, .. } = notification else {
            return Some(SupervisionEvent::Unsupervised { notification });
        };
        let Some((id, kind)) = self.find_watched(handle) else {
            // Not watched — pass through: `poll` is the sole drain
            // point, so every notification must still reach someone.
            return Some(SupervisionEvent::Unsupervised { notification });
        };
        // The failed/cleanup handle's slot is freed unconditionally —
        // `Err` is absorbed: an external `Destroy` may already have
        // freed it, and recovery is decided by the notification, not
        // by this cleanup racing.
        let _ = host.execute(HostCommand::Destroy { handle });
        let max_attempts = self.policy.max_attempts;
        let stable_window = self.policy.stable_window;
        let entry = self.entries.get_mut(&id)?;
        match kind {
            MatchKind::Current => {
                let EntryState::Live { healthy_since, .. } = entry.state else {
                    return None; // unreachable — `Current` implies `Live`
                };
                // Episode reset runs BEFORE the give-up check — a
                // sporadic crash landing past `stable_window` must not
                // carry a stale episode's `failures` into `GaveUp`.
                if let Some(since) = healthy_since
                    && deadline_passed(since, stable_window, now)
                {
                    entry.failures = 0;
                    entry.last_error = None;
                }
                if entry.failures >= max_attempts {
                    entry.state = EntryState::GaveUp;
                    Some(SupervisionEvent::GaveUp {
                        id,
                        attempts: entry.failures,
                        last_error: entry.last_error,
                    })
                } else {
                    entry.state = EntryState::Pending {
                        next_attempt_at: Some(now),
                        failed: handle,
                    };
                    None
                }
            }
            // `Pending.failed` reporting again — the entry already
            // destroyed/recovered this handle; the idempotent
            // `Destroy` above is all it needs (state unchanged).
            MatchKind::Failed => None,
            MatchKind::Lingering => {
                // Only a lingering match clears the residue — clearing
                // on a current/failed match would orphan the real
                // survivor.
                entry.lingering = None;
                match entry.state {
                    // The last live residue of a `GaveUp` entry died —
                    // re-emit so the caller sees `current_handle`
                    // change, with identical `attempts`/`last_error`
                    // (`(id, attempts)` dedupes against the first).
                    EntryState::GaveUp => Some(SupervisionEvent::GaveUp {
                        id,
                        attempts: entry.failures,
                        last_error: entry.last_error,
                    }),
                    _ => None,
                }
            }
        }
    }

    /// One recovery attempt: `Instantiate` → `Initialize` →
    /// `RestoreState` (checkpoint; `Err(Unsupported)` skips, other
    /// `Err` fails) → `Activate` (`config: Some`) → `StartProcessing`
    /// (`config: Some` + `start_processing`). `RestoreState` runs
    /// before `Activate` so `prepare` sees the restored state, and
    /// needs no lifecycle gate.
    fn attempt(
        host: &mut PluginHost,
        entry: &Entry,
    ) -> Result<(PluginHandle, RestoreOutcome), AttemptFailure> {
        let new = match host.execute(HostCommand::Instantiate {
            plugin_id: entry.recipe.plugin_id.clone(),
        }) {
            Ok(HostReply::Instantiated { handle }) => handle,
            // `Instantiate` replies only `Instantiated` — anything
            // else violates the public surface contract.
            Ok(_) => unreachable!("Instantiate replies only Instantiated"),
            Err(error) => {
                // Nothing minted — no cleanup owed.
                return Err(AttemptFailure {
                    error,
                    minted: None,
                });
            }
        };
        if let Err(error) = host.execute(HostCommand::Initialize { handle: new }) {
            return Err(AttemptFailure {
                error,
                minted: Some(new),
            });
        }
        let mut restored = RestoreOutcome::NotRequested;
        if let Some(bytes) = &entry.checkpoint {
            // `bytes` passed the `set_checkpoint` bound, so the frame
            // always fits `wire::WIRE_MAX_PAYLOAD`.
            match host.execute(HostCommand::RestoreState {
                handle: new,
                bytes: bytes.clone(),
            }) {
                Ok(_) => restored = RestoreOutcome::Applied,
                // State-less plugins skip the restore without burning
                // the attempt; the checkpoint stays registered.
                Err(HostError::Unsupported) => restored = RestoreOutcome::Skipped,
                Err(error) => {
                    return Err(AttemptFailure {
                        error,
                        minted: Some(new),
                    });
                }
            }
        }
        if let Some(config) = entry.recipe.config {
            if let Err(error) = host.execute(HostCommand::Activate {
                handle: new,
                config,
            }) {
                return Err(AttemptFailure {
                    error,
                    minted: Some(new),
                });
            }
            if entry.recipe.start_processing
                && let Err(error) = host.execute(HostCommand::StartProcessing { handle: new })
            {
                return Err(AttemptFailure {
                    error,
                    minted: Some(new),
                });
            }
        }
        Ok((new, restored))
    }

    /// Best-effort destroy of a minted handle after a failed attempt:
    /// the lifecycle mirror picks the unwind chain — `Processing`
    /// steps `StopProcessing` → `Deactivate` → `Destroy`, `Activated`
    /// steps `Deactivate` → `Destroy`, `Created`/`Initialized`/`Failed`
    /// (and any future state, conservatively) go straight to
    /// `Destroy`. The first `Err` aborts the chain.
    fn cleanup_instance(host: &mut PluginHost, handle: PluginHandle) -> Result<(), HostError> {
        let destroy =
            |host: &mut PluginHost| host.execute(HostCommand::Destroy { handle }).map(|_| ());
        match host.lifecycle_state(handle) {
            // Gone (or beyond the mirror's live vocabulary) — nothing
            // owed.
            None | Some(LifecycleState::Destroyed) => Ok(()),
            Some(LifecycleState::Activated) => {
                host.execute(HostCommand::Deactivate { handle })?;
                destroy(host)
            }
            Some(LifecycleState::Processing) => {
                host.execute(HostCommand::StopProcessing { handle })?;
                host.execute(HostCommand::Deactivate { handle })?;
                destroy(host)
            }
            // `Created`/`Initialized`/`Failed` and any future variant
            // — `Destroy` directly; an illegal transition's `Err`
            // keeps it as residue instead of forcing it.
            Some(_) => destroy(host),
        }
    }

    /// Runs [`Self::cleanup_instance`] on a failed attempt's minted
    /// handle; a persistent `Err` keeps it as `lingering` (exposed via
    /// `current_handle`, retried every Phase 2), while
    /// `Ok`/`Err(InvalidHandle)` — already freed externally — leaves
    /// nothing to track.
    fn unwind_minted(host: &mut PluginHost, entry: &mut Entry, minted: PluginHandle) {
        match Self::cleanup_instance(host, minted) {
            Ok(()) | Err(HostError::InvalidHandle) => {}
            Err(_) => Self::store_lingering(host, entry, minted),
        }
    }

    /// Tracks `handle` as the entry's cleanup residue. `lingering`
    /// holds one handle, so a residue already recorded gets one
    /// inline retry first — freed, the new failure takes the slot;
    /// still stuck, the oldest outstanding stays tracked (FIFO) and
    /// the newer live handle is left to the host's own bookkeeping —
    /// a double unwind-failure corner, not the design's normal path.
    fn store_lingering(host: &mut PluginHost, entry: &mut Entry, handle: PluginHandle) {
        match entry.lingering {
            None => entry.lingering = Some(handle),
            Some(old) if old == handle => {}
            Some(old) => match Self::cleanup_instance(host, old) {
                Ok(()) | Err(HostError::InvalidHandle) => entry.lingering = Some(handle),
                Err(_) => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use ruviform_audio_core::SampleRate;
    use ruviform_plugin_core::{
        BufferView, BufferViewMut, EventList, ParamInfo, PluginDescriptor, PluginProcessor,
        PrepareError, ProcessContext, ProcessResult, TransportInfo,
    };
    use static_assertions::{assert_impl_all, assert_not_impl_any};

    use super::*;
    use crate::{ChildExit, ErasedPlugin, Plugin, PluginFactory, erase};

    // `Send`/`Sync` witnesses (test-spec §型・語彙 witness): the
    // supervisor holds only sendable data (`usize` binding, plain
    // bookkeeping) so it stays `Send`/`Sync`-able while the host it
    // borrows keeps `!Send`/`!Sync`. `SupervisionError` pins `Copy`.
    assert_impl_all!(Supervisor: Send, Sync);
    assert_impl_all!(SupervisionError: Copy);
    assert_not_impl_any!(PluginHost: Send, Sync);

    const PID: &str = "org.ruviform.test.supervisor";
    const PID_B: &str = "org.ruviform.test.supervisor.b";

    const DESCRIPTOR: PluginDescriptor = PluginDescriptor {
        id: PID,
        name: "Supervisor Test",
        vendor: "Ruviform",
        version: "0.1.0",
        features: &[],
    };

    const DESCRIPTOR_B: PluginDescriptor = PluginDescriptor {
        id: PID_B,
        ..DESCRIPTOR
    };

    /// Minimal fixture plugin: `prepare`/`restore_state` outcomes are
    /// injection points, `process` is an inert `Continue` stub.
    struct TestPlugin {
        descriptor: PluginDescriptor,
        fail_prepare: bool,
        restore: Result<(), HostError>,
    }

    impl PluginProcessor for TestPlugin {
        fn prepare(
            &mut self,
            _config: &ProcessConfig,
            _context: &mut ProcessContext,
        ) -> Result<(), PrepareError> {
            if self.fail_prepare {
                Err(PrepareError::ActivationFailed)
            } else {
                Ok(())
            }
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
            ProcessResult::CONTINUE
        }
    }

    impl Plugin for TestPlugin {
        fn descriptor(&self) -> PluginDescriptor {
            self.descriptor
        }

        fn params(&self) -> &[ParamInfo] {
            &[]
        }

        fn restore_state(&mut self, _state: &[u8]) -> Result<(), HostError> {
            self.restore
        }
    }

    struct TestFactory {
        descriptor: PluginDescriptor,
        fail_prepare: bool,
        restore: Result<(), HostError>,
    }

    impl PluginFactory for TestFactory {
        fn descriptor(&self) -> PluginDescriptor {
            self.descriptor
        }

        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(TestPlugin {
                descriptor: self.descriptor,
                fail_prepare: self.fail_prepare,
                restore: self.restore,
            })
        }
    }

    fn host_with(
        descriptor: PluginDescriptor,
        restore: Result<(), HostError>,
        fail_prepare: bool,
    ) -> PluginHost {
        let mut host = PluginHost::new();
        host.register_factory(Box::new(TestFactory {
            descriptor,
            fail_prepare,
            restore,
        }));
        host
    }

    fn default_host() -> PluginHost {
        host_with(DESCRIPTOR, Ok(()), false)
    }

    fn instantiate(host: &mut PluginHost, plugin_id: &str) -> PluginHandle {
        let Ok(HostReply::Instantiated { handle }) = host.execute(HostCommand::Instantiate {
            plugin_id: plugin_id.to_owned(),
        }) else {
            panic!("Instantiate must succeed for {plugin_id}");
        };
        handle
    }

    fn config() -> ProcessConfig {
        ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap()
    }

    fn supervisor() -> Supervisor {
        Supervisor::new(RestartPolicy::default()).unwrap()
    }

    fn live_entry(handle: PluginHandle) -> Entry {
        Entry {
            recipe: RestartRecipe::new(PID),
            checkpoint: None,
            failures: 0,
            last_error: None,
            lingering: None,
            state: EntryState::Live {
                handle,
                healthy_since: None,
            },
        }
    }

    fn pending_entry(next_attempt_at: Option<Instant>, failed: PluginHandle) -> Entry {
        Entry {
            recipe: RestartRecipe::new(PID),
            checkpoint: None,
            failures: 0,
            last_error: None,
            lingering: None,
            state: EntryState::Pending {
                next_attempt_at,
                failed,
            },
        }
    }

    /// A synthesized `InstanceFailed` — constructible here because
    /// `HostNotification`/`ChildExit` live in this crate. (An L0 host
    /// never emits notifications, so `poll`'s drain loop is fed by
    /// calling `handle_notification` directly.)
    fn failed_notification(handle: PluginHandle) -> HostNotification {
        HostNotification::InstanceFailed {
            handle,
            exit: ChildExit {
                code: Some(1),
                signal: None,
            },
        }
    }

    fn restarted(event: &SupervisionEvent) -> (PluginHandle, PluginHandle, RestoreOutcome) {
        match event {
            SupervisionEvent::Restarted {
                old, new, restored, ..
            } => (*old, *new, *restored),
            other => panic!("expected Restarted, got {other:?}"),
        }
    }

    fn attempt_failed(event: &SupervisionEvent) -> (u32, HostError) {
        match event {
            SupervisionEvent::AttemptFailed { attempt, error, .. } => (*attempt, *error),
            other => panic!("expected AttemptFailed, got {other:?}"),
        }
    }

    fn gave_up(event: &SupervisionEvent) -> (u32, Option<HostError>) {
        match event {
            SupervisionEvent::GaveUp {
                attempts,
                last_error,
                ..
            } => (*attempts, *last_error),
            other => panic!("expected GaveUp, got {other:?}"),
        }
    }

    #[test]
    fn policy_validation_and_defaults() {
        // `matches!` — `Result<Supervisor, _>` equality would need
        // `Supervisor: PartialEq + Debug`, which is not part of the
        // public contract.
        assert!(matches!(
            Supervisor::new(RestartPolicy {
                max_attempts_per_poll: 0,
                ..RestartPolicy::default()
            }),
            Err(SupervisionError::InertPolicy)
        ));

        let policy = RestartPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay, Duration::from_millis(250));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert_eq!(policy.stable_window, Duration::from_secs(60));
        assert_eq!(policy.max_attempts_per_poll, 8);

        let recipe = RestartRecipe::new(PID);
        assert_eq!(recipe.plugin_id, PID);
        assert_eq!(recipe.config, None);
        assert!(!recipe.start_processing);
        assert_eq!(recipe.endpoint_bounds, None);
    }

    #[test]
    fn watch_registers_live_handle() {
        let mut host = default_host();
        let handle = instantiate(&mut host, PID);
        let mut sup = supervisor();

        let id = sup
            .watch(&mut host, handle, RestartRecipe::new(PID))
            .unwrap();
        assert_eq!(sup.current_handle(id), Some(handle));
        assert_eq!(sup.find_watched(handle), Some((id, MatchKind::Current)));
    }

    #[test]
    fn watch_rejects_stale_handle() {
        let mut host = default_host();
        let mut sup = supervisor();
        assert_eq!(
            sup.watch(
                &mut host,
                PluginHandle::new(999, 0),
                RestartRecipe::new(PID)
            ),
            Err(SupervisionError::InvalidHandle)
        );
    }

    #[test]
    fn watch_rejects_unregistered_plugin_id() {
        let mut host = default_host();
        let handle = instantiate(&mut host, PID);
        let mut sup = supervisor();
        assert_eq!(
            sup.watch(
                &mut host,
                handle,
                RestartRecipe::new("org.example.unregistered")
            ),
            Err(SupervisionError::UnknownPluginId)
        );
    }

    #[test]
    fn watch_rejects_duplicate_match_set() {
        let mut host = default_host();
        let handle = instantiate(&mut host, PID);
        let mut sup = supervisor();
        sup.watch(&mut host, handle, RestartRecipe::new(PID))
            .unwrap();
        // Current-handle re-watch.
        assert_eq!(
            sup.watch(&mut host, handle, RestartRecipe::new(PID)),
            Err(SupervisionError::AlreadyWatched)
        );
        // Same handle appearing as another entry's `Pending.failed` —
        // a *live* handle is required: `watch` runs `lifecycle_state`
        // before the duplicate check, so a synthesized handle would
        // land on `InvalidHandle` first.
        let failed = instantiate(&mut host, PID);
        sup.entries
            .insert(SupervisedId(100), pending_entry(None, failed));
        assert_eq!(
            sup.watch(&mut host, failed, RestartRecipe::new(PID)),
            Err(SupervisionError::AlreadyWatched)
        );
        // ... and as another entry's `lingering` residue.
        let mut entry = live_entry(handle);
        let lingering = instantiate(&mut host, PID);
        entry.lingering = Some(lingering);
        sup.entries.insert(SupervisedId(101), entry);
        assert_eq!(
            sup.watch(&mut host, lingering, RestartRecipe::new(PID)),
            Err(SupervisionError::AlreadyWatched)
        );
    }

    #[test]
    fn host_binding_rejects_foreign_host() {
        let mut host_a = default_host();
        let mut host_b = default_host();
        let handle = instantiate(&mut host_a, PID);
        let mut sup = supervisor();
        let id = sup
            .watch(&mut host_a, handle, RestartRecipe::new(PID))
            .unwrap();

        assert_eq!(
            sup.watch(
                &mut host_b,
                PluginHandle::new(1, 0),
                RestartRecipe::new(PID)
            ),
            Err(SupervisionError::WrongHost)
        );
        assert_eq!(
            sup.set_recipe(&mut host_b, id, RestartRecipe::new(PID)),
            Err(SupervisionError::WrongHost)
        );

        // A foreign `poll` emits WrongHost only — no drain, no reset,
        // no attempt on the other host's queue or on this entry.
        let ready = pending_entry(None, PluginHandle::new(52, 0));
        sup.entries.insert(SupervisedId(102), ready);
        let events = sup.poll(&mut host_b, Instant::now());
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SupervisionEvent::WrongHost));
        assert!(matches!(
            sup.entries[&SupervisedId(102)].state,
            EntryState::Pending { .. }
        ));

        // The bound host keeps working — the ready entry's attempt
        // was not spent on the foreign poll.
        let events = sup.poll(&mut host_a, Instant::now());
        assert_eq!(events.len(), 1);
        match &events[0] {
            SupervisionEvent::Restarted { id, .. } => assert_eq!(*id, SupervisedId(102)),
            other => panic!("expected Restarted, got {other:?}"),
        }
    }

    #[test]
    fn poll_binds_host_on_first_call() {
        let mut host_a = default_host();
        let mut host_b = PluginHost::new();
        let mut sup = supervisor();

        assert!(sup.poll(&mut host_a, Instant::now()).is_empty());
        let events = sup.poll(&mut host_b, Instant::now());
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SupervisionEvent::WrongHost));
        assert!(sup.poll(&mut host_a, Instant::now()).is_empty());
    }

    #[test]
    fn unwatch_returns_handle_by_state() {
        let mut sup = supervisor();
        let current = PluginHandle::new(10, 0);
        let lingering = PluginHandle::new(11, 0);
        let failed = PluginHandle::new(12, 0);

        sup.entries.insert(SupervisedId(1), live_entry(current));
        let mut pending = pending_entry(None, failed);
        pending.lingering = Some(lingering);
        sup.entries.insert(SupervisedId(2), pending);
        sup.entries
            .insert(SupervisedId(3), pending_entry(None, failed));

        assert_eq!(sup.unwatch(SupervisedId(1)), Some(current));
        assert_eq!(sup.unwatch(SupervisedId(2)), Some(lingering));
        assert_eq!(sup.unwatch(SupervisedId(3)), None);
        assert_eq!(sup.unwatch(SupervisedId(1)), None);
        assert_eq!(sup.unwatch(SupervisedId(999)), None);
    }

    #[test]
    fn set_recipe_replaces_and_validates() {
        let mut host = default_host();
        host.register_factory(Box::new(TestFactory {
            descriptor: DESCRIPTOR_B,
            fail_prepare: false,
            restore: Ok(()),
        }));
        let handle = instantiate(&mut host, PID);
        let mut sup = supervisor();
        let id = sup
            .watch(&mut host, handle, RestartRecipe::new(PID))
            .unwrap();

        assert_eq!(
            sup.set_recipe(&mut host, SupervisedId(999), RestartRecipe::new(PID)),
            Err(SupervisionError::NotWatched)
        );
        assert_eq!(
            sup.set_recipe(&mut host, id, RestartRecipe::new("org.example.missing")),
            Err(SupervisionError::UnknownPluginId)
        );

        let mut replacement = RestartRecipe::new(PID_B);
        replacement.config = Some(config());
        replacement.start_processing = true;
        sup.set_recipe(&mut host, id, replacement).unwrap();
        let recipe = &sup.entries[&id].recipe;
        assert_eq!(recipe.plugin_id, PID_B);
        assert!(recipe.start_processing);
        assert!(recipe.config.is_some());
    }

    #[test]
    fn set_checkpoint_wire_bound() {
        let mut host = default_host();
        let handle = instantiate(&mut host, PID);
        let mut sup = supervisor();
        let id = sup
            .watch(&mut host, handle, RestartRecipe::new(PID))
            .unwrap();

        let max_payload = wire::WIRE_MAX_PAYLOAD as usize;
        // `len + 17 == WIRE_MAX_PAYLOAD` still fits — the boundary's
        // two sides are pinned.
        let fitting = vec![0_u8; max_payload - WIRE_COMMAND_ENVELOPE];
        sup.set_checkpoint(id, fitting).unwrap();
        let oversized = vec![0_u8; max_payload - WIRE_COMMAND_ENVELOPE + 1];
        assert_eq!(
            sup.set_checkpoint(id, oversized),
            Err(SupervisionError::CheckpointTooLarge)
        );
        assert_eq!(
            sup.set_checkpoint(SupervisedId(999), vec![1, 2, 3]),
            Err(SupervisionError::NotWatched)
        );
    }

    #[test]
    fn clear_checkpoint_reports_presence() {
        let mut host = default_host();
        let handle = instantiate(&mut host, PID);
        let mut sup = supervisor();
        let id = sup
            .watch(&mut host, handle, RestartRecipe::new(PID))
            .unwrap();

        assert!(!sup.clear_checkpoint(id));
        sup.set_checkpoint(id, vec![9, 9]).unwrap();
        assert!(sup.clear_checkpoint(id));
        assert!(!sup.clear_checkpoint(id));
        assert!(!sup.clear_checkpoint(SupervisedId(999)));
    }

    #[test]
    fn current_handle_prefers_current_then_residue() {
        let mut sup = supervisor();
        let current = PluginHandle::new(10, 0);
        let lingering = PluginHandle::new(11, 0);
        let failed = PluginHandle::new(12, 0);

        let mut live = live_entry(current);
        live.lingering = Some(lingering);
        sup.entries.insert(SupervisedId(1), live);

        let mut pending = pending_entry(None, failed);
        pending.lingering = Some(lingering);
        sup.entries.insert(SupervisedId(2), pending);

        sup.entries
            .insert(SupervisedId(3), pending_entry(None, failed));

        let mut gave_up_entry = pending_entry(None, failed);
        gave_up_entry.state = EntryState::GaveUp;
        gave_up_entry.lingering = Some(lingering);
        sup.entries.insert(SupervisedId(4), gave_up_entry);

        assert_eq!(sup.current_handle(SupervisedId(1)), Some(current));
        assert_eq!(sup.current_handle(SupervisedId(2)), Some(lingering));
        assert_eq!(sup.current_handle(SupervisedId(3)), None);
        assert_eq!(sup.current_handle(SupervisedId(4)), Some(lingering));
        assert_eq!(sup.current_handle(SupervisedId(999)), None);
    }

    #[test]
    fn delay_grows_exponentially_and_clamps() {
        let policy = RestartPolicy::default();
        assert_eq!(attempt_delay(&policy, 1), Duration::from_millis(250));
        assert_eq!(attempt_delay(&policy, 2), Duration::from_millis(500));
        assert_eq!(attempt_delay(&policy, 7), Duration::from_secs(16));
        // 250 ms * 2^7 = 32 s — past the 30 s ceiling.
        assert_eq!(attempt_delay(&policy, 8), Duration::from_secs(30));
        assert_eq!(attempt_delay(&policy, 100), Duration::from_secs(30));

        let custom = RestartPolicy {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(2),
            ..RestartPolicy::default()
        };
        assert_eq!(attempt_delay(&custom, 1), Duration::from_secs(1));
        assert_eq!(attempt_delay(&custom, 2), Duration::from_secs(2));
        assert_eq!(attempt_delay(&custom, 3), Duration::from_secs(2));

        // `checked_mul` overflow clamps to `max_delay` too.
        let overflowing = RestartPolicy {
            base_delay: Duration::MAX,
            ..custom
        };
        assert_eq!(attempt_delay(&overflowing, 31), Duration::from_secs(2));
    }

    #[test]
    fn find_watched_distinguishes_match_set() {
        let mut sup = supervisor();
        let h_a = PluginHandle::new(20, 0);
        let h_b = PluginHandle::new(21, 0);
        let h_c = PluginHandle::new(22, 0);
        let h_d = PluginHandle::new(23, 0);
        let h_e = PluginHandle::new(24, 0);

        let mut live = live_entry(h_a);
        live.lingering = Some(h_b);
        sup.entries.insert(SupervisedId(1), live);
        let mut pending = pending_entry(None, h_c);
        pending.lingering = Some(h_d);
        sup.entries.insert(SupervisedId(2), pending);
        let mut gave_up_entry = pending_entry(None, h_c);
        gave_up_entry.state = EntryState::GaveUp;
        gave_up_entry.lingering = Some(h_e);
        sup.entries.insert(SupervisedId(3), gave_up_entry);

        assert_eq!(
            sup.find_watched(h_a),
            Some((SupervisedId(1), MatchKind::Current))
        );
        assert_eq!(
            sup.find_watched(h_b),
            Some((SupervisedId(1), MatchKind::Lingering))
        );
        assert_eq!(
            sup.find_watched(h_c),
            Some((SupervisedId(2), MatchKind::Failed))
        );
        assert_eq!(
            sup.find_watched(h_d),
            Some((SupervisedId(2), MatchKind::Lingering))
        );
        assert_eq!(
            sup.find_watched(h_e),
            Some((SupervisedId(3), MatchKind::Lingering))
        );
        assert_eq!(sup.find_watched(PluginHandle::new(99, 0)), None);
    }

    #[test]
    fn drain_current_death_destroys_and_pends() {
        let mut host = default_host();
        let handle = instantiate(&mut host, PID);
        let mut sup = supervisor();
        let id = SupervisedId(1);
        sup.entries.insert(id, live_entry(handle));
        let now = Instant::now();

        let event = sup.handle_notification(&mut host, now, failed_notification(handle));
        assert!(event.is_none());
        // The failed slot is freed — `Destroy` ran on drain.
        assert_eq!(host.lifecycle_state(handle), None);
        assert!(matches!(
            sup.entries[&id].state,
            EntryState::Pending {
                next_attempt_at: Some(deadline),
                failed
            } if deadline == now && failed == handle
        ));
    }

    #[test]
    fn drain_resets_stable_window_before_give_up() {
        let mut host = default_host();
        let handle = instantiate(&mut host, PID);
        let mut sup = supervisor();
        let id = SupervisedId(1);

        // failures already at the cap — a crash past `stable_window`
        // must reset the episode first, not give up on stale counts.
        let mut entry = live_entry(handle);
        let t0 = Instant::now();
        entry.state = EntryState::Live {
            handle,
            healthy_since: Some(t0),
        };
        entry.failures = 3;
        entry.last_error = Some(HostError::Ipc);
        sup.entries.insert(id, entry);

        let now = t0 + Duration::from_secs(60);
        let event = sup.handle_notification(&mut host, now, failed_notification(handle));
        assert!(event.is_none());
        assert!(matches!(sup.entries[&id].state, EntryState::Pending { .. }));
        assert_eq!(sup.entries[&id].failures, 0);
        assert_eq!(sup.entries[&id].last_error, None);

        // Same setup, crash inside the window: `failures` stands and
        // the entry gives up immediately.
        let mut host = default_host();
        let handle = instantiate(&mut host, PID);
        let mut sup = supervisor();
        let mut entry = live_entry(handle);
        entry.state = EntryState::Live {
            handle,
            healthy_since: Some(t0),
        };
        entry.failures = 3;
        entry.last_error = Some(HostError::Ipc);
        sup.entries.insert(id, entry);

        let event = sup.handle_notification(
            &mut host,
            t0 + Duration::from_secs(30),
            failed_notification(handle),
        );
        let (attempts, last_error) = gave_up(&event.unwrap());
        assert_eq!(attempts, 3);
        assert_eq!(last_error, Some(HostError::Ipc));
        assert!(matches!(sup.entries[&id].state, EntryState::GaveUp));
    }

    #[test]
    fn drain_zero_max_attempts_gives_up() {
        let mut host = default_host();
        let handle = instantiate(&mut host, PID);
        let mut sup = Supervisor::new(RestartPolicy {
            max_attempts: 0,
            ..RestartPolicy::default()
        })
        .unwrap();
        let id = SupervisedId(1);
        sup.entries.insert(id, live_entry(handle));

        let event = sup
            .handle_notification(&mut host, Instant::now(), failed_notification(handle))
            .unwrap();
        assert_eq!(gave_up(&event), (0, None));
        assert!(matches!(sup.entries[&id].state, EntryState::GaveUp));
        assert_eq!(host.lifecycle_state(handle), None);
    }

    #[test]
    fn drain_failed_match_preserves_pending() {
        let mut host = default_host();
        // `failed` holds a still-live L0 handle — the drain `Destroy`
        // frees it regardless (the notification is the authority).
        let failed = instantiate(&mut host, PID);
        let mut sup = supervisor();
        let id = SupervisedId(1);
        let next = Instant::now() + Duration::from_secs(1);
        sup.entries.insert(id, pending_entry(Some(next), failed));

        let event = sup.handle_notification(&mut host, Instant::now(), failed_notification(failed));
        assert!(event.is_none());
        assert_eq!(host.lifecycle_state(failed), None);
        assert!(matches!(
            sup.entries[&id].state,
            EntryState::Pending {
                next_attempt_at: Some(deadline),
                failed: f
            } if deadline == next && f == failed
        ));
    }

    #[test]
    fn drain_lingering_match_clears_residue_only() {
        let mut host = default_host();
        let current = instantiate(&mut host, PID);
        let lingering = instantiate(&mut host, PID);
        let mut sup = supervisor();
        let id = SupervisedId(1);
        let mut entry = live_entry(current);
        entry.lingering = Some(lingering);
        sup.entries.insert(id, entry);

        let event =
            sup.handle_notification(&mut host, Instant::now(), failed_notification(lingering));
        assert!(event.is_none());
        assert_eq!(host.lifecycle_state(lingering), None);
        assert_eq!(sup.entries[&id].lingering, None);
        // Current stays untouched and still watched.
        assert!(matches!(
            sup.entries[&id].state,
            EntryState::Live { handle, .. } if handle == current
        ));
        assert_eq!(host.lifecycle_state(current), Some(LifecycleState::Created));
    }

    #[test]
    fn drain_gave_up_lingering_reemits() {
        let mut host = default_host();
        let lingering = instantiate(&mut host, PID);
        let mut sup = supervisor();
        let id = SupervisedId(1);
        let mut entry = pending_entry(None, PluginHandle::new(7, 0));
        entry.state = EntryState::GaveUp;
        entry.failures = 3;
        entry.last_error = Some(HostError::PluginFailed);
        entry.lingering = Some(lingering);
        sup.entries.insert(id, entry);

        let event = sup
            .handle_notification(&mut host, Instant::now(), failed_notification(lingering))
            .unwrap();
        // Identical payload to the first give-up — `(id, attempts)`
        // dedupes it for consumers.
        assert_eq!(gave_up(&event), (3, Some(HostError::PluginFailed)));
        assert_eq!(sup.entries[&id].lingering, None);
        assert!(matches!(sup.entries[&id].state, EntryState::GaveUp));
    }

    #[test]
    fn drain_unwatched_passes_through() {
        let mut host = default_host();
        let mut sup = supervisor();
        sup.entries
            .insert(SupervisedId(1), live_entry(PluginHandle::new(1, 0)));

        // A *live* unwatched handle — the drain must neither swallow
        // its notification nor destroy the instance.
        let unknown = instantiate(&mut host, PID);
        let notification = failed_notification(unknown);
        let event = sup
            .handle_notification(&mut host, Instant::now(), notification)
            .unwrap();
        match event {
            SupervisionEvent::Unsupervised { notification: n } => {
                assert_eq!(n, notification);
            }
            other => panic!("expected Unsupervised, got {other:?}"),
        }
        assert_eq!(host.lifecycle_state(unknown), Some(LifecycleState::Created));
    }

    #[test]
    fn poll_restarts_pending_entry() {
        let mut host = default_host();
        let mut sup = supervisor();
        let id = SupervisedId(1);
        let stale = PluginHandle::new(50, 0);
        sup.entries.insert(id, pending_entry(None, stale));
        let now = Instant::now();

        let events = sup.poll(&mut host, now);
        assert_eq!(events.len(), 1);
        let (old, new, restored) = restarted(&events[0]);
        assert_eq!(old, stale);
        assert_ne!(new, stale);
        assert_eq!(restored, RestoreOutcome::NotRequested);
        // `config: None` stops at `Initialized`.
        assert_eq!(host.lifecycle_state(new), Some(LifecycleState::Initialized));
        assert_eq!(sup.current_handle(id), Some(new));
        assert!(matches!(
            sup.entries[&id].state,
            EntryState::Live {
                handle,
                healthy_since: Some(t)
            } if handle == new && t == now
        ));
    }

    #[test]
    fn poll_restart_replays_recipe() {
        let mut host = default_host();
        let mut sup = supervisor();
        let stale = PluginHandle::new(50, 0);

        // `config: Some` + `start_processing` reaches `Processing`.
        let id_full = SupervisedId(1);
        let mut entry = pending_entry(None, stale);
        entry.recipe.config = Some(config());
        entry.recipe.start_processing = true;
        entry.recipe.endpoint_bounds = Some(EndpointBounds {
            max_frames: 512,
            channels: 2,
        });
        sup.entries.insert(id_full, entry);

        // `start_processing` alone is ignored while `config: None` —
        // the guard keeps `StartProcessing` off an `Initialized`
        // instance that would reject it.
        let id_min = SupervisedId(2);
        let mut entry = pending_entry(None, PluginHandle::new(51, 0));
        entry.recipe.start_processing = true;
        sup.entries.insert(id_min, entry);

        let events = sup.poll(&mut host, Instant::now());
        assert_eq!(events.len(), 2);
        let (_, full_new, _) = restarted(&events[0]);
        let (_, min_new, _) = restarted(&events[1]);
        assert_eq!(
            host.lifecycle_state(full_new),
            Some(LifecycleState::Processing)
        );
        assert_eq!(
            host.lifecycle_state(min_new),
            Some(LifecycleState::Initialized)
        );

        match &events[0] {
            SupervisionEvent::Restarted { bounds, .. } => {
                assert_eq!(
                    *bounds,
                    Some(EndpointBounds {
                        max_frames: 512,
                        channels: 2
                    })
                );
            }
            other => panic!("expected Restarted, got {other:?}"),
        }
    }

    #[test]
    fn poll_restart_checkpoint_outcomes() {
        // Applied — the fixture's `restore_state` returns `Ok`.
        let mut host = default_host();
        let mut sup = supervisor();
        let id = SupervisedId(1);
        sup.entries
            .insert(id, pending_entry(None, PluginHandle::new(50, 0)));
        sup.set_checkpoint(id, vec![1, 2, 3]).unwrap();
        let events = sup.poll(&mut host, Instant::now());
        assert_eq!(events.len(), 1);
        let (_, _, restored) = restarted(&events[0]);
        assert_eq!(restored, RestoreOutcome::Applied);

        // Skipped — `Err(Unsupported)` does not consume the attempt.
        let mut host = host_with(DESCRIPTOR, Err(HostError::Unsupported), false);
        let mut sup = supervisor();
        let id = SupervisedId(1);
        sup.entries
            .insert(id, pending_entry(None, PluginHandle::new(50, 0)));
        sup.set_checkpoint(id, vec![1, 2, 3]).unwrap();
        let events = sup.poll(&mut host, Instant::now());
        assert_eq!(events.len(), 1);
        let (_, _, restored) = restarted(&events[0]);
        assert_eq!(restored, RestoreOutcome::Skipped);
        assert_eq!(sup.entries[&id].failures, 0);
    }

    #[test]
    fn poll_restore_rejection_burns_attempts() {
        let mut host = host_with(DESCRIPTOR, Err(HostError::PluginFailed), false);
        let mut sup = supervisor();
        let id = SupervisedId(1);
        sup.entries
            .insert(id, pending_entry(None, PluginHandle::new(50, 0)));
        sup.set_checkpoint(id, vec![1, 2, 3]).unwrap();

        let t0 = Instant::now();
        // attempt 1 → AttemptFailed + 250 ms backoff.
        let events = sup.poll(&mut host, t0);
        assert_eq!(events.len(), 1);
        assert_eq!(attempt_failed(&events[0]), (1, HostError::PluginFailed));
        assert!(matches!(
            sup.entries[&id].state,
            EntryState::Pending {
                next_attempt_at: Some(deadline),
                ..
            } if deadline == t0 + Duration::from_millis(250)
        ));
        // The minted `Initialized` instance was destroyed — no residue.
        assert_eq!(sup.entries[&id].lingering, None);
        assert_eq!(sup.current_handle(id), None);

        // attempt 2 (250 ms later — deadline reached) → AttemptFailed.
        let events = sup.poll(&mut host, t0 + Duration::from_millis(250));
        assert_eq!(events.len(), 1);
        assert_eq!(attempt_failed(&events[0]), (2, HostError::PluginFailed));
        // attempt 3 exhausts the episode — `GaveUp` replaces
        // `AttemptFailed` at the cap.
        let events = sup.poll(&mut host, t0 + Duration::from_millis(750));
        assert_eq!(events.len(), 1);
        assert_eq!(gave_up(&events[0]), (3, Some(HostError::PluginFailed)));
        assert!(matches!(sup.entries[&id].state, EntryState::GaveUp));
    }

    #[test]
    fn poll_backoff_gates_attempt_timing() {
        let mut host = default_host();
        let mut sup = supervisor();
        let id = SupervisedId(1);
        let t0 = Instant::now();
        let next = t0 + Duration::from_millis(250);
        sup.entries
            .insert(id, pending_entry(Some(next), PluginHandle::new(50, 0)));

        // Deadline not yet reached — no attempt, no events.
        assert!(sup.poll(&mut host, t0).is_empty());
        assert!(matches!(sup.entries[&id].state, EntryState::Pending { .. }));
        // Reached — the attempt runs.
        let events = sup.poll(&mut host, next);
        assert_eq!(events.len(), 1);
        restarted(&events[0]);
    }

    #[test]
    fn poll_caps_attempts_and_orders_ready() {
        let mut host = default_host();
        let mut sup = Supervisor::new(RestartPolicy {
            max_attempts_per_poll: 2,
            ..RestartPolicy::default()
        })
        .unwrap();
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(10);

        // Ready order: `None` (adopt — immediate) first, then by
        // `next_attempt_at`, `id` as tiebreak.
        sup.entries.insert(
            SupervisedId(30),
            pending_entry(Some(t1), PluginHandle::new(60, 0)),
        );
        sup.entries.insert(
            SupervisedId(10),
            pending_entry(None, PluginHandle::new(61, 0)),
        );
        sup.entries.insert(
            SupervisedId(20),
            pending_entry(Some(t0), PluginHandle::new(62, 0)),
        );

        let events = sup.poll(&mut host, t1);
        assert_eq!(events.len(), 2);
        match &events[0] {
            SupervisionEvent::Restarted { id, .. } => assert_eq!(*id, SupervisedId(10)),
            other => panic!("expected Restarted, got {other:?}"),
        }
        match &events[1] {
            SupervisionEvent::Restarted { id, .. } => assert_eq!(*id, SupervisedId(20)),
            other => panic!("expected Restarted, got {other:?}"),
        }
        // The capped entry carries over silently — still `Pending`,
        // no event.
        assert!(matches!(
            sup.entries[&SupervisedId(30)].state,
            EntryState::Pending { .. }
        ));
        let events = sup.poll(&mut host, t1);
        assert_eq!(events.len(), 1);
        match &events[0] {
            SupervisionEvent::Restarted { id, .. } => assert_eq!(*id, SupervisedId(30)),
            other => panic!("expected Restarted, got {other:?}"),
        }
    }

    #[test]
    fn poll_window_reset_sweeps_live_entries() {
        let mut host = default_host();
        let mut sup = supervisor();
        let id = SupervisedId(1);
        let t0 = Instant::now();
        let handle = PluginHandle::new(10, 0);
        let mut entry = live_entry(handle);
        entry.state = EntryState::Live {
            handle,
            healthy_since: Some(t0),
        };
        entry.failures = 2;
        entry.last_error = Some(HostError::Ipc);
        sup.entries.insert(id, entry);

        // Inside the window — untouched.
        assert!(sup.poll(&mut host, t0 + Duration::from_secs(30)).is_empty());
        assert_eq!(sup.entries[&id].failures, 2);
        // Past it — reset silently (no event either way).
        assert!(sup.poll(&mut host, t0 + Duration::from_secs(60)).is_empty());
        assert_eq!(sup.entries[&id].failures, 0);
        assert_eq!(sup.entries[&id].last_error, None);
    }

    #[test]
    fn poll_retries_lingering_cleanup() {
        let mut host = default_host();
        let lingering = instantiate(&mut host, PID);
        let mut sup = supervisor();
        let id = SupervisedId(1);
        let mut entry = pending_entry(None, PluginHandle::new(50, 0));
        entry.lingering = Some(lingering);
        sup.entries.insert(id, entry);

        // Phase 2 runs before Phase 3: the residue is destroyed first,
        // then the ready attempt mints the replacement.
        let events = sup.poll(&mut host, Instant::now());
        assert_eq!(host.lifecycle_state(lingering), None);
        assert_eq!(sup.entries[&id].lingering, None);
        assert_eq!(events.len(), 1);
        restarted(&events[0]);
    }

    #[test]
    fn poll_l0_host_is_inert() {
        let mut host = default_host();
        let mut sup = supervisor();
        assert!(sup.poll(&mut host, Instant::now()).is_empty());
    }

    #[test]
    fn cleanup_unwinds_by_mirror_state() {
        let mut host = default_host();
        // `Created` → straight `Destroy`.
        let created = instantiate(&mut host, PID);
        Supervisor::cleanup_instance(&mut host, created).unwrap();
        assert_eq!(host.lifecycle_state(created), None);

        // `Initialized` → `Destroy`.
        let initialized = instantiate(&mut host, PID);
        host.execute(HostCommand::Initialize {
            handle: initialized,
        })
        .unwrap();
        Supervisor::cleanup_instance(&mut host, initialized).unwrap();
        assert_eq!(host.lifecycle_state(initialized), None);

        // `Activated` → `Deactivate` → `Destroy`.
        let activated = instantiate(&mut host, PID);
        host.execute(HostCommand::Initialize { handle: activated })
            .unwrap();
        host.execute(HostCommand::Activate {
            handle: activated,
            config: config(),
        })
        .unwrap();
        Supervisor::cleanup_instance(&mut host, activated).unwrap();
        assert_eq!(host.lifecycle_state(activated), None);

        // `Processing` → `StopProcessing` → `Deactivate` → `Destroy`.
        let processing = instantiate(&mut host, PID);
        host.execute(HostCommand::Initialize { handle: processing })
            .unwrap();
        host.execute(HostCommand::Activate {
            handle: processing,
            config: config(),
        })
        .unwrap();
        host.execute(HostCommand::StartProcessing { handle: processing })
            .unwrap();
        Supervisor::cleanup_instance(&mut host, processing).unwrap();
        assert_eq!(host.lifecycle_state(processing), None);

        // Already gone — `Ok(())`, nothing owed.
        Supervisor::cleanup_instance(&mut host, PluginHandle::new(99, 0)).unwrap();
    }
}
