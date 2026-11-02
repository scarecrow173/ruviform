//! The non-RT → RT control channel and the RT → non-RT report channel
//! of a [`crate::PluginNode`].
//!
//! `NodeControl` messages are staged by [`PluginNodeControl`] (non-RT)
//! and applied at the head of the node's next `process` callback —
//! bounded, non-allocating, FIFO. `NodeReport` messages flow the other
//! way; the control side drains them whenever convenient.

use std::sync::Arc;

use ruviform_media_core::{
    AppliedReceipt, ClockEpochId, ScheduleFault, TransportCommand, TransportPlan,
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
}

impl PluginNodeControl {
    pub(crate) const fn new(
        tx: Producer<NodeControl>,
        rx: Consumer<NodeReport>,
        status: Arc<NodeStatus>,
    ) -> Self {
        Self { tx, rx, status }
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
    /// # Errors
    ///
    /// [`QueueFull`] when the control queue is full.
    pub fn install_plan(
        &mut self,
        plan: Arc<TransportPlan<EventKind>>,
        due_frames: u64,
    ) -> Result<(), QueueFull> {
        self.tx
            .try_push(NodeControl::InstallPlan { plan, due_frames })
            .map_err(|_| self.queue_full())
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
