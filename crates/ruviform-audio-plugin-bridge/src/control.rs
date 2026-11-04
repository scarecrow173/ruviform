//! The non-RT → RT control channel and the RT → non-RT report channel
//! of a [`crate::PluginNode`].
//!
//! `NodeControl` messages are staged by [`PluginNodeControl`] (non-RT)
//! and applied at the head of the node's next `process` callback —
//! bounded, non-allocating, FIFO. `NodeReport` messages flow the other
//! way; the control side drains them whenever convenient.

use std::sync::Arc;

use ruviform_media_core::{
    AppliedReceipt, ClockAnchor, ClockEpochId, PlanGeneration, ScheduleFault, TimeBase,
    TransportCommand, TransportPlan, TransportPlanBuilder,
};
use ruviform_plugin_core::EventKind;
use ruviform_rt::{Consumer, Producer};

use crate::status::{Counter, NodeStatus};

/// A control message the RT node applies at callback head.
///
/// `due_frames` are *callback-relative*: the node adds them to its own
/// engine-frame cursor, so the non-RT side never needs the node's raw
/// frame counter.
#[non_exhaustive]
#[derive(Debug)]
pub enum NodeControl {
    /// `TransportScheduler::request` — `due_frames` ahead of the next
    /// callback's start.
    Request {
        /// The transport command.
        cmd: TransportCommand,
        /// Frames ahead of the next callback start.
        due_frames: u64,
    },
    /// `TransportScheduler::install_plan` — same due convention. The
    /// node clones the `Arc` into its [`crate::PlanVault`] *before*
    /// staging so the RT-side apply/reject/restart can never run the
    /// plan's destructor.
    InstallPlan {
        /// The compiled plan.
        plan: Arc<TransportPlan<EventKind>>,
        /// Frames ahead of the next callback start.
        due_frames: u64,
    },
    /// Rebase the node's frame cursor (e.g. after a stream re-anchor).
    SetClock {
        /// New clock epoch.
        epoch: ClockEpochId,
        /// New frame inside the epoch.
        frame: u64,
    },
    /// `TransportScheduler::device_restart` — rebase the epoch.
    DeviceRestart {
        /// Frame the new epoch starts at.
        base_frame: u64,
    },
    /// Publish (or clear) the live-input stamp→frame anchor — the
    /// node's [`crate::InputStage`] maps backend stamps against it at
    /// the next callback's `collect_block`. A defined no-op on nodes
    /// attached without an input leg (`AttachConfig::input == None`).
    SetClockAnchor {
        /// The anchor, or `None` to suspend input mapping until the
        /// next anchor lands.
        anchor: Option<ClockAnchor>,
    },
}

/// A report the RT node emits toward the non-RT control side. Kept
/// `Copy`-free where payloads own data; the channel is bounded so a
/// full queue drops (and counts) rather than blocking.
#[non_exhaustive]
#[derive(Debug)]
pub enum NodeReport {
    /// A scheduler receipt (Requested/Pending/Applied/Rejected).
    Receipt(AppliedReceipt),
    /// A scheduler fault (stale generation, undeliverable apply, …).
    Fault(ScheduleFault),
    /// A span the contract could not partition — emitted as silence.
    UndeliverableSpan {
        /// Span start frame.
        start: u64,
        /// Span end frame (exclusive).
        end: u64,
    },
    /// An event window exceeded the per-call event scratch; events
    /// for that call were dropped.
    EventScratchOverflow,
    /// `PluginEndpoint::process` returned `Err` — the call's output
    /// region was silenced. Payload is a coarse category (`0`
    /// lifecycle, `1` prepare, `2` busy/unavailable, `3` other).
    EndpointError {
        /// Coarse error category.
        kind: u8,
    },
    /// A plan install was rejected because the [`crate::PlanVault`]
    /// is full — the scheduler never saw it.
    VaultExceeded,
    /// The delivered-note ledger force-released an entry on overflow —
    /// an active note was cut early (bounded-table defined loss).
    LedgerOverflow,
    /// Chase could not re-fire every key sounding at a span's start —
    /// `dropped` keys stayed silent (scratch or tracker-table bound).
    ChaseOverflow {
        /// Sounding-or-unknown keys not re-fired.
        dropped: u64,
    },
    /// A `SetClock` control named a different clock epoch than the
    /// scheduler's — the move was ignored. Moving the frame cursor to
    /// a foreign epoch makes every callback boundary unrepresentable
    /// (a hard output stall); epoch rebases go through
    /// `DeviceRestart`, which bumps the scheduler epoch itself.
    StaleEpochControl,
}

/// The bounded control queue was full — the message was dropped, the
/// `control_overflows` counter bumped. Retry is the caller's policy.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct QueueFull;

impl core::fmt::Display for QueueFull {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("plugin-node control queue full")
    }
}

impl core::error::Error for QueueFull {}

/// A supplied `TimeBase` differs from the node's attach-time frame
/// base — the verified context minted by [`crate::attach`] from
/// `AttachConfig::frame_base`.
///
/// A `TransportPlan` compiled in a different tick base would have its
/// `position_ticks` misread in the scheduler's frame space;
/// `TimedParamEvent`/Seek/Plan-Receipt alignment and offline accuracy
/// all depend on the bases being substantively equal.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FrameBaseMismatch {
    /// The node's attach-time frame base (the verified context).
    pub expected: TimeBase,
    /// The foreign frame base that was supplied.
    pub actual: TimeBase,
}

impl core::fmt::Display for FrameBaseMismatch {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "frame base mismatch: expected {}/{}, got {}/{}",
            self.expected.numerator(),
            self.expected.denominator(),
            self.actual.numerator(),
            self.actual.denominator(),
        )
    }
}

impl core::error::Error for FrameBaseMismatch {}

/// `install_plan` failure — the plan was *not* staged.
///
/// [`FrameBaseMismatch`](InstallPlanError::FrameBaseMismatch) means the
/// plan was compiled in a foreign frame base and was rejected before
/// any queue interaction: nothing was pushed, `control_overflows` was
/// not bumped, and the previously installed plan/graph/node state is
/// untouched.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InstallPlanError {
    /// The bounded control queue was full — the message was dropped
    /// and `control_overflows` bumped (unchanged `QueueFull` semantics).
    QueueFull(QueueFull),
    /// `plan.frame_base` differs from the attach-time frame base.
    FrameBaseMismatch(FrameBaseMismatch),
}

impl core::fmt::Display for InstallPlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::QueueFull(e) => write!(f, "{e}"),
            Self::FrameBaseMismatch(e) => write!(f, "{e}"),
        }
    }
}

impl core::error::Error for InstallPlanError {}

/// The non-RT handle of a [`crate::PluginNode`]: stage transport
/// control, drain reports, read fault counters.
///
/// **RT contract:** non-RT only — `try_push`/`try_pop` never block,
/// `drain_reports` allocates.
#[derive(Debug)]
pub struct PluginNodeControl {
    tx: Producer<NodeControl>,
    rx: Consumer<NodeReport>,
    status: Arc<NodeStatus>,
    frame_base: TimeBase,
}

impl PluginNodeControl {
    pub(crate) const fn new(
        tx: Producer<NodeControl>,
        rx: Consumer<NodeReport>,
        status: Arc<NodeStatus>,
        frame_base: TimeBase,
    ) -> Self {
        Self {
            tx,
            rx,
            status,
            frame_base,
        }
    }

    /// The attach-time frame base — the same `AttachConfig::frame_base`
    /// the node's `TransportScheduler` runs on (one tick = one engine
    /// frame). This is the verified clock context for plan/dispatcher
    /// construction: values obtained anywhere else must be validated
    /// with [`Self::check_frame_base`] before use.
    ///
    /// Invariant: fixed at `attach` and never mutated — clock-epoch
    /// identity (`SetClock`/`DeviceRestart`), transport position
    /// (Play/Seek/Stop) and plan generations are independent and do
    /// not change it.
    ///
    /// **RT contract:** non-RT.
    #[must_use]
    pub const fn frame_base(&self) -> TimeBase {
        self.frame_base
    }

    /// Verifies `base` against the attach-time frame base using the
    /// contract's substantive equality (gcd-normalized `TimeBase`
    /// `PartialEq`). Returns [`FrameBaseMismatch`] carrying
    /// `expected`/`actual` so consumer constructors (e.g. a param
    /// dispatcher taking an external `frame_base`) can propagate the
    /// typed rejection with `?`.
    ///
    /// **RT contract:** non-RT.
    ///
    /// # Errors
    ///
    /// [`FrameBaseMismatch`] when `base` differs from
    /// [`Self::frame_base`].
    pub fn check_frame_base(&self, base: TimeBase) -> Result<(), FrameBaseMismatch> {
        if base == self.frame_base {
            Ok(())
        } else {
            Err(FrameBaseMismatch {
                expected: self.frame_base,
                actual: base,
            })
        }
    }

    /// Mints a [`TransportPlanBuilder`] bound to this node's verified
    /// attach-time frame base. Plans built through it always satisfy
    /// [`Self::install_plan`]'s base check — the correct-construction
    /// path for consumers (plan generation stays the caller's choice,
    /// matching the scheduler's `PlanGeneration` adoption contract).
    ///
    /// **RT contract:** non-RT.
    #[must_use]
    pub fn plan_builder(&self, generation: PlanGeneration) -> TransportPlanBuilder<EventKind> {
        TransportPlanBuilder::new(self.frame_base, generation)
    }

    /// Stages a transport command `due_frames` ahead of the next
    /// callback.
    ///
    /// # Errors
    ///
    /// [`QueueFull`] when the control queue is full — the message is
    /// dropped and `control_overflows` is bumped.
    pub fn request(&mut self, cmd: TransportCommand, due_frames: u64) -> Result<(), QueueFull> {
        self.tx
            .try_push(NodeControl::Request { cmd, due_frames })
            .map_err(|_| self.queue_full())
    }

    /// Stages a plan install `due_frames` ahead of the next callback.
    /// The node vault-clones the `Arc` before staging.
    ///
    /// `plan.frame_base` is verified against the attach-time frame base
    /// *before* any queue interaction — a mismatched plan is rejected
    /// with [`InstallPlanError::FrameBaseMismatch`] and nothing is
    /// staged, queued, or counted; the installed plan and node state
    /// are untouched.
    ///
    /// # Errors
    ///
    /// [`InstallPlanError::FrameBaseMismatch`] when `plan.frame_base`
    /// differs from [`Self::frame_base`];
    /// [`InstallPlanError::QueueFull`] when the control queue is full.
    pub fn install_plan(
        &mut self,
        plan: Arc<TransportPlan<EventKind>>,
        due_frames: u64,
    ) -> Result<(), InstallPlanError> {
        self.check_frame_base(plan.frame_base)
            .map_err(InstallPlanError::FrameBaseMismatch)?;
        self.tx
            .try_push(NodeControl::InstallPlan { plan, due_frames })
            .map_err(|_| InstallPlanError::QueueFull(self.queue_full()))
    }

    /// Rebases the node's frame cursor.
    ///
    /// # Errors
    ///
    /// [`QueueFull`] when the control queue is full.
    pub fn set_clock(&mut self, epoch: ClockEpochId, frame: u64) -> Result<(), QueueFull> {
        self.tx
            .try_push(NodeControl::SetClock { epoch, frame })
            .map_err(|_| self.queue_full())
    }

    /// Schedules a device restart (new clock epoch).
    ///
    /// # Errors
    ///
    /// [`QueueFull`] when the control queue is full.
    pub fn device_restart(&mut self, base_frame: u64) -> Result<(), QueueFull> {
        self.tx
            .try_push(NodeControl::DeviceRestart { base_frame })
            .map_err(|_| self.queue_full())
    }

    /// Publishes the live-input stamp→frame anchor (or clears it with
    /// `None`) — applied at the node's next callback head.
    ///
    /// # Errors
    ///
    /// [`QueueFull`] when the control queue is full.
    pub fn set_clock_anchor(&mut self, anchor: Option<ClockAnchor>) -> Result<(), QueueFull> {
        self.tx
            .try_push(NodeControl::SetClockAnchor { anchor })
            .map_err(|_| self.queue_full())
    }

    /// Drains every pending report — non-RT.
    #[must_use]
    pub fn drain_reports(&mut self) -> Vec<NodeReport> {
        let mut out = Vec::new();
        while let Ok(report) = self.rx.try_pop() {
            out.push(report);
        }
        out
    }

    fn queue_full(&self) -> QueueFull {
        self.status.bump(Counter::ControlOverflow);
        QueueFull
    }

    /// The node's shared counters.
    #[must_use]
    pub const fn status(&self) -> &Arc<NodeStatus> {
        &self.status
    }

    /// Counter snapshot convenience.
    #[must_use]
    pub fn snapshot(&self) -> crate::NodeStatusSnapshot {
        self.status.snapshot()
    }
}
