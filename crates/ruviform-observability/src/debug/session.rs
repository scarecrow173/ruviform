//! Debug session vocabulary — the `Disabled → Prepared → Active →
//! Draining → Stopped` lifecycle, the capability model that gates
//! it, and the drain port that recovers RT buffers (Issue #292,
//! OBS-10).
//!
//! # Lifecycle
//!
//! A debug session is the *additional-cost* part of debug
//! instrumentation: bounded capture of domain debug records through
//! [`SessionDrain`] endpoints. The lifecycle is one-directional
//! except `Stopped → Prepared` (a re-prepare mints a new session
//! `generation`):
//!
//! - [`Disabled`](DebugSessionState::Disabled) — default; inspection
//!   of existing observability data still works, only
//!   session-gated capture is absent.
//! - `Prepared` — a [`SessionPermit`] was consumed and drains may be
//!   attached; nothing is active yet.
//! - `Active` — drains accept records; requires at least one
//!   attached drain.
//! - `Draining` — stop requested; no new work, final RT records
//!   being collected.
//! - `Stopped` — terminal for this generation; drain endpoints are
//!   dropped (releasing ring/`Arc`s) **only after** every endpoint
//!   confirmed writer quiescence and drained to `lag == 0` — or via
//!   the explicit [`abandon`](SessionCore::abandon) path, which is
//!   counted and reported, never silently treated as clean.
//!
//! # Capability model
//!
//! [`DebugAuthority`] mints [`SessionPermit`]s bound to its internal
//! token; `prepare`/`request_stop`/`abandon` consume a permit by
//! value and the inspector verifies the permit was minted by *its*
//! installed authority — a permit from a different authority is
//! [`SessionError::PermitDenied`]. Real ACL/audit issuance is
//! OBS-11 (#293); this models the boundary as types.
//!
//! # RT buffer recovery
//!
//! [`SessionDrain`] is the port domain adapters implement;
//! [`EventRecordDrain`] is the in-crate concrete adapter over
//! [`rt_bridge::RecordSink<EventRecord>`](crate::rt_bridge::RecordSink)
//! — reusing the OBS-05 bridge rather than growing a second record
//! path. `writer_quiesced` reports the shared `Arc<AtomicBool>` flag
//! the domain sets when its recorder leaves the RT path (the same
//! shared-cell precedent as [`SourceState`](crate::source::SourceState)).
//! `drain_final` is contractually called only after quiescence —
//! matching the bridge's own `drain_final` precondition.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::record::EventRecord;
use crate::rt_bridge::RecordSink;

/// Debug session lifecycle — see the module docs for the transition
/// rules. `Stopped → Prepared` is permitted and mints a new
/// generation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebugSessionState {
    /// No session — the default.
    Disabled,
    /// Permit consumed, drains may attach, not yet active.
    Prepared,
    /// Debug capture may run; drains accept records.
    Active,
    /// Stop requested — collecting final RT records.
    Draining,
    /// Terminal for this generation — buffers released after
    /// confirmed quiescence (or explicit abandon).
    Stopped,
}

/// Opaque session-control capability — minted only by
/// [`DebugAuthority::permit`] and bound to that authority's token.
/// `prepare`/`request_stop`/`abandon` consume it by value.
#[derive(Debug)]
pub struct SessionPermit {
    token: Arc<()>,
}

impl SessionPermit {
    /// The authority binding token — crate-internal.
    pub(crate) fn token(&self) -> &Arc<()> {
        &self.token
    }
}

/// Host-side authority marker — installing one on the inspector
/// enables session control. `permit()` mints bound capabilities;
/// `revoke()` denies future issuance (existing permits remain valid
/// until consumed — revocation models *issuance* policy, matching
/// the capability boundary this slice owns).
#[derive(Debug)]
pub struct DebugAuthority {
    token: Arc<()>,
    revoked: AtomicBool,
}

impl DebugAuthority {
    /// Creates an authority with a fresh binding token.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: Arc::new(()),
            revoked: AtomicBool::new(false),
        }
    }

    /// Issues a permit bound to this authority, or `None` when
    /// revoked — modeled as `Option`, not an error, because denial
    /// is a policy outcome rather than a fault.
    #[must_use]
    pub fn permit(&self) -> Option<SessionPermit> {
        if self.revoked.load(Ordering::Relaxed) {
            return None;
        }
        Some(SessionPermit {
            token: Arc::clone(&self.token),
        })
    }

    /// Revokes future permit issuance.
    pub fn revoke(&self) {
        self.revoked.store(true, Ordering::Relaxed);
    }

    /// The binding token — crate-internal, used to verify permits.
    pub(crate) fn token(&self) -> &Arc<()> {
        &self.token
    }
}

impl Default for DebugAuthority {
    fn default() -> Self {
        Self::new()
    }
}

/// What `prepare_session` asks for.
#[derive(Debug, Clone, Copy)]
pub struct SessionRequest {
    /// Domain-owned static label (e.g. `"synvelis.signal-flow"`) —
    /// `&'static str` bounds cardinality at compile time.
    pub label: &'static str,
    /// Bound on drained records retained for inspection/report —
    /// overflow is counted in `records_overflowed`, never silently
    /// dropped. Must be `>= 1` (`0` is [`SessionError::InvalidRequest`]).
    pub max_records: usize,
}

/// Errors session operations can return — every rejected request is
/// typed; an abandoned request is never reported as success.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// The inspector has no [`DebugAuthority`] installed — session
    /// control was never enabled.
    NoAuthority,
    /// The presented [`SessionPermit`] was not minted by this
    /// inspector's authority (different token, or the authority was
    /// replaced).
    PermitDenied,
    /// The operation is not allowed from the current state — the
    /// per-method docs list the allowed source states.
    InvalidTransition {
        /// The state actually found.
        actual: DebugSessionState,
    },
    /// A non-`Stopped` session already exists — `prepare` is
    /// rejected rather than queued (the switch contract: wait for
    /// `Stopped`, then prepare a new generation).
    SessionBusy {
        /// The state the existing session is in.
        state: DebugSessionState,
    },
    /// `activate` requires at least one attached drain.
    NoDrains,
    /// The request itself is invalid (e.g. `max_records == 0`).
    InvalidRequest,
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NoAuthority => write!(f, "no debug authority installed"),
            Self::PermitDenied => write!(f, "session permit not recognized"),
            Self::InvalidTransition { actual } => {
                write!(f, "invalid session transition from {actual:?}")
            }
            Self::SessionBusy { state } => {
                write!(f, "session already active in state {state:?}")
            }
            Self::NoDrains => write!(f, "cannot activate without drains"),
            Self::InvalidRequest => write!(f, "invalid session request"),
        }
    }
}

impl std::error::Error for SessionError {}

/// One drained debug record — an [`EventRecord`] (fixed-size,
/// cannot carry media/secrets) plus the positional loss the bridge
/// reported for it.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Clone, Copy, Debug)]
pub struct SessionRecord {
    /// The delivered record.
    pub record: EventRecord,
    /// Records dropped immediately before this one.
    pub dropped_before: u64,
}

/// Per-call drain accounting returned by
/// [`SessionDrain::drain_final`].
#[derive(Clone, Copy, Debug, Default)]
pub struct DrainReport {
    /// Records delivered to the session this call.
    pub delivered: usize,
    /// Cumulative dropped records on this endpoint.
    pub dropped_total: u64,
    /// Records still in flight after this call.
    pub lag: u64,
}

/// Result of [`SessionCore::poll_drain`] — the honest answer to
/// "is the session done draining yet".
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionDrainStatus {
    /// Still draining — counts of endpoints awaiting quiescence and
    /// records still in flight; `deadline_exceeded` reports the
    /// configured `drain_timeout` elapsed without forcing a stop.
    Draining {
        /// Endpoints whose writer has not confirmed quiescence.
        pending_quiescence: usize,
        /// Records still in flight across all endpoints.
        pending_records: u64,
        /// `true` when `drain_timeout` elapsed — draining continues.
        deadline_exceeded: bool,
    },
    /// All endpoints quiesced and drained — the session is `Stopped`
    /// and drain endpoints (ring/`Arc`s) were released.
    Stopped {
        /// `true` when quiescence was confirmed on every endpoint
        /// (always `true` on this path — `abandon` reports through
        /// [`SessionView::abandoned`] instead).
        clean: bool,
        /// Records retained in the session.
        records_collected: usize,
        /// Records that exceeded `max_records`.
        records_overflowed: u64,
    },
    /// `poll_drain` called while not `Draining`.
    NotDraining {
        /// The current state.
        state: DebugSessionState,
    },
}

/// Non-RT drain endpoint — implemented by domain adapters and by
/// the provided [`EventRecordDrain`]. The session retains endpoints
/// until quiescence is confirmed; it never drops a sink while the
/// writer might still produce.
pub trait SessionDrain: Send {
    /// `true` once the RT writer has left the RT path — the
    /// precondition for [`drain_final`](Self::drain_final), mirroring
    /// the `rt_bridge` writer-quiescence contract.
    fn writer_quiesced(&self) -> bool;

    /// Records accepted but not yet delivered — live in-flight
    /// count used for pending accounting and the drain-completion
    /// check.
    fn in_flight(&self) -> u64;

    /// Bounded final drain; delivers each pending record to `emit`.
    /// Called only after [`writer_quiesced`](Self::writer_quiesced)
    /// returns `true`.
    fn drain_final(&mut self, emit: &mut dyn FnMut(SessionRecord)) -> DrainReport;
}

/// Concrete [`SessionDrain`] over
/// [`rt_bridge::RecordSink<EventRecord>`](crate::rt_bridge::RecordSink)
/// — the in-crate adapter proving reuse of the OBS-05 bridge rather
/// than a second record path.
///
/// Quiescence is reported through a shared `Arc<AtomicBool>` the
/// domain sets (store `true`, `Relaxed`) once its `RtRecorder` has
/// left the RT path — the same shared-cell precedent as
/// [`SourceState`](crate::source::SourceState).
pub struct EventRecordDrain {
    sink: RecordSink<EventRecord>,
    quiesced: Arc<AtomicBool>,
}

impl EventRecordDrain {
    /// Wraps `sink`; `quiesced` is the shared flag described above.
    #[must_use]
    pub fn new(sink: RecordSink<EventRecord>, quiesced: Arc<AtomicBool>) -> Self {
        Self { sink, quiesced }
    }

    /// The shared quiescence flag — for tests and adapter
    /// composition.
    #[must_use]
    pub fn quiesced_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.quiesced)
    }

    /// The wrapped sink — for counter readers and inspection.
    #[must_use]
    pub fn sink(&self) -> &RecordSink<EventRecord> {
        &self.sink
    }
}

impl SessionDrain for EventRecordDrain {
    fn writer_quiesced(&self) -> bool {
        self.quiesced.load(Ordering::Relaxed)
    }

    fn in_flight(&self) -> u64 {
        self.sink.lag()
    }

    fn drain_final(&mut self, emit: &mut dyn FnMut(SessionRecord)) -> DrainReport {
        let delivered = self.sink.drain_final(|received| {
            emit(SessionRecord {
                record: received.record(),
                dropped_before: received.dropped_before(),
            });
        });
        DrainReport {
            delivered,
            dropped_total: self.sink.dropped(),
            lag: self.sink.lag(),
        }
    }
}

/// Owned snapshot of the session slot — what inspection and reports
/// read. Cheap to produce; all fields are values.
#[derive(Debug, Clone)]
pub struct SessionView {
    /// Current lifecycle state.
    pub state: DebugSessionState,
    /// Session instance generation (`0` while `Disabled`).
    pub generation: u64,
    /// Domain-declared label, if prepared.
    pub label: Option<&'static str>,
    /// Attached drain endpoints.
    pub drains_attached: usize,
    /// Records retained (bounded by `max_records`).
    pub records_retained: usize,
    /// Records that exceeded the cap.
    pub records_overflowed: u64,
    /// Endpoints still awaiting writer quiescence (live count).
    pub pending_quiescence: usize,
    /// Records still in flight across endpoints (live count).
    pub pending_records: u64,
    /// `drain_timeout` elapsed while `Draining`.
    pub deadline_exceeded: bool,
    /// `true` when `Stopped` was reached via `abandon`.
    pub abandoned: bool,
    /// Lifecycle timestamps, when reached.
    pub prepared_at: Option<Instant>,
    /// Lifecycle timestamps, when reached.
    pub activated_at: Option<Instant>,
    /// Lifecycle timestamps, when reached.
    pub drain_started_at: Option<Instant>,
    /// Lifecycle timestamps, when reached.
    pub stopped_at: Option<Instant>,
}

/// The session slot owned by [`DebugInspector`](crate::debug::DebugInspector)
/// — crate-internal state machine; every transition is validated here
/// so the inspector's public methods stay thin delegations.
pub(crate) struct SessionCore {
    state: DebugSessionState,
    generation: u64,
    label: Option<&'static str>,
    max_records: usize,
    drains: Vec<Box<dyn SessionDrain>>,
    records: VecDeque<SessionRecord>,
    overflow: u64,
    prepared_at: Option<Instant>,
    activated_at: Option<Instant>,
    drain_started_at: Option<Instant>,
    stopped_at: Option<Instant>,
    abandoned: bool,
    deadline_exceeded: bool,
    /// Lifetime cumulative delivered records — never reset on
    /// re-prepare so health-counter deltas cannot underflow.
    delivered_total: u64,
    /// Lifetime cumulative overflowed records — same contract.
    overflow_total: u64,
}

impl SessionCore {
    /// A disabled session — the default.
    pub(crate) fn new() -> Self {
        Self {
            state: DebugSessionState::Disabled,
            generation: 0,
            label: None,
            max_records: 0,
            drains: Vec::new(),
            records: VecDeque::new(),
            overflow: 0,
            prepared_at: None,
            activated_at: None,
            drain_started_at: None,
            stopped_at: None,
            abandoned: false,
            deadline_exceeded: false,
            delivered_total: 0,
            overflow_total: 0,
        }
    }

    /// Current state.
    pub(crate) const fn state(&self) -> DebugSessionState {
        self.state
    }

    /// Retained drained records, oldest first.
    pub(crate) fn records(&self) -> &VecDeque<SessionRecord> {
        &self.records
    }

    /// `Disabled | Stopped → Prepared` (new generation); any other
    /// state is [`SessionError::SessionBusy`].
    pub(crate) fn prepare(
        &mut self,
        label: &'static str,
        max_records: usize,
        now: Instant,
    ) -> Result<u64, SessionError> {
        match self.state {
            DebugSessionState::Disabled | DebugSessionState::Stopped => {}
            state => return Err(SessionError::SessionBusy { state }),
        }
        if max_records == 0 {
            return Err(SessionError::InvalidRequest);
        }
        self.generation += 1;
        self.state = DebugSessionState::Prepared;
        self.label = Some(label);
        self.max_records = max_records;
        self.drains.clear();
        self.records.clear();
        self.overflow = 0;
        self.prepared_at = Some(now);
        self.activated_at = None;
        self.drain_started_at = None;
        self.stopped_at = None;
        self.abandoned = false;
        self.deadline_exceeded = false;
        Ok(self.generation)
    }

    /// `Prepared | Active` only — drains cannot attach once draining
    /// or stopped (a stopped session's buffers are already released).
    pub(crate) fn attach(&mut self, drain: Box<dyn SessionDrain>) -> Result<(), SessionError> {
        match self.state {
            DebugSessionState::Prepared | DebugSessionState::Active => {
                self.drains.push(drain);
                Ok(())
            }
            actual => Err(SessionError::InvalidTransition { actual }),
        }
    }

    /// `Prepared → Active`; requires at least one drain.
    pub(crate) fn activate(&mut self, now: Instant) -> Result<(), SessionError> {
        if self.state != DebugSessionState::Prepared {
            return Err(SessionError::InvalidTransition { actual: self.state });
        }
        if self.drains.is_empty() {
            return Err(SessionError::NoDrains);
        }
        self.state = DebugSessionState::Active;
        self.activated_at = Some(now);
        Ok(())
    }

    /// `Prepared | Active → Draining`.
    pub(crate) fn request_stop(&mut self, now: Instant) -> Result<(), SessionError> {
        match self.state {
            DebugSessionState::Prepared | DebugSessionState::Active => {
                self.state = DebugSessionState::Draining;
                self.drain_started_at = Some(now);
                self.deadline_exceeded = false;
                Ok(())
            }
            actual => Err(SessionError::InvalidTransition { actual }),
        }
    }

    /// Advances `Draining`: drains every quiesced endpoint (bounded
    /// by each bridge's ring capacity), retains records up to
    /// `max_records`, and reaches `Stopped` — dropping the drain
    /// endpoints and releasing their rings/`Arc`s — only when every
    /// endpoint is quiesced *and* in-flight is zero. `drain_timeout`
    /// elapsed is reported via `deadline_exceeded` but never forces
    /// a stop.
    pub(crate) fn poll_drain(
        &mut self,
        drain_timeout: Option<Duration>,
        now: Instant,
    ) -> SessionDrainStatus {
        if self.state != DebugSessionState::Draining {
            return SessionDrainStatus::NotDraining { state: self.state };
        }
        if let (Some(start), Some(timeout)) = (self.drain_started_at, drain_timeout)
            && now.saturating_duration_since(start) >= timeout
        {
            self.deadline_exceeded = true;
        }

        let SessionCore {
            drains,
            records,
            overflow,
            max_records,
            delivered_total,
            overflow_total,
            ..
        } = self;
        let mut pending_quiescence = 0usize;
        let mut pending_records = 0u64;
        let mut all_done = true;
        for drain in drains.iter_mut() {
            if !drain.writer_quiesced() {
                pending_quiescence += 1;
                pending_records += drain.in_flight();
                all_done = false;
                continue;
            }
            let report = drain.drain_final(&mut |record| {
                *delivered_total += 1;
                if records.len() < *max_records {
                    records.push_back(record);
                } else {
                    *overflow += 1;
                    *overflow_total += 1;
                }
            });
            pending_records += report.lag;
            if report.lag > 0 {
                all_done = false;
            }
        }

        if !all_done {
            return SessionDrainStatus::Draining {
                pending_quiescence,
                pending_records,
                deadline_exceeded: self.deadline_exceeded,
            };
        }

        // Every endpoint quiesced and fully drained — the only point
        // where dropping sinks/rings is contractually safe.
        self.state = DebugSessionState::Stopped;
        self.stopped_at = Some(now);
        self.drains.clear();
        SessionDrainStatus::Stopped {
            clean: true,
            records_collected: self.records.len(),
            records_overflowed: self.overflow,
        }
    }

    /// `Prepared | Active | Draining → Stopped` via the explicit
    /// abandon path — drops drain endpoints without confirmed
    /// quiescence. Counted and reported, never silent.
    pub(crate) fn abandon(&mut self, now: Instant) -> Result<(), SessionError> {
        match self.state {
            DebugSessionState::Prepared
            | DebugSessionState::Active
            | DebugSessionState::Draining => {
                self.state = DebugSessionState::Stopped;
                self.stopped_at = Some(now);
                self.abandoned = true;
                self.drains.clear();
                Ok(())
            }
            actual => Err(SessionError::InvalidTransition { actual }),
        }
    }

    /// Lifetime cumulative `(delivered, overflowed)` drain totals —
    /// monotonic across generations so callers can diff safely.
    pub(crate) const fn drain_totals(&self) -> (u64, u64) {
        (self.delivered_total, self.overflow_total)
    }

    /// Owned view for inspection/report — `pending_*` counts are
    /// computed live from the drains.
    pub(crate) fn view(&self) -> SessionView {
        let mut pending_quiescence = 0usize;
        let mut pending_records = 0u64;
        for drain in &self.drains {
            if !drain.writer_quiesced() {
                pending_quiescence += 1;
            }
            pending_records += drain.in_flight();
        }
        SessionView {
            state: self.state,
            generation: self.generation,
            label: self.label,
            drains_attached: self.drains.len(),
            records_retained: self.records.len(),
            records_overflowed: self.overflow,
            pending_quiescence,
            pending_records,
            deadline_exceeded: self.deadline_exceeded,
            abandoned: self.abandoned,
            prepared_at: self.prepared_at,
            activated_at: self.activated_at,
            drain_started_at: self.drain_started_at,
            stopped_at: self.stopped_at,
        }
    }
}

impl Default for SessionCore {
    fn default() -> Self {
        Self::new()
    }
}
