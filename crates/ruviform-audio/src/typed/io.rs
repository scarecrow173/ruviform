//! Borrowed per-node I/O views handed to [`GraphProcessor`]s.
//!
//! [`NodeIo`] is built on the stack per scheduled node — it borrows
//! the runtime's `RefCell<Slot>` arena and the immutable plan
//! bindings, so constructing it performs no allocation. Ports are
//! materialized per access call (`input`/`output`), which also keeps
//! borrows short-lived and lets `RefCell` flag any residual double
//! borrow as a programming error instead of silently aliasing.

use core::cell::{Ref, RefMut};

use super::buffer::Slot;
use super::compile::ScheduledNode;
use super::runtime::SharedStatus;

/// Read view over one input port's materialized slot.
pub struct InputView<'a> {
    slot: Ref<'a, Slot>,
    frames: usize,
}

impl InputView<'_> {
    /// Logical channels of this input.
    #[must_use]
    pub fn channels(&self) -> usize {
        usize::from(self.slot.channels)
    }

    /// Frames valid this callback.
    #[must_use]
    pub const fn frames(&self) -> usize {
        self.frames
    }

    /// Channel `c`'s signal — `frames` valid samples.
    #[must_use]
    pub fn channel(&self, c: usize) -> Option<&[f32]> {
        self.slot.data.channel(c).map(|plane| &plane[..self.frames])
    }
}

/// Write view over one output port's slot.
pub struct OutputView<'a> {
    slot: RefMut<'a, Slot>,
    frames: usize,
}

impl OutputView<'_> {
    /// Logical channels of this output.
    #[must_use]
    pub fn channels(&self) -> usize {
        usize::from(self.slot.channels)
    }

    /// Frames the processor must write this callback.
    #[must_use]
    pub const fn frames(&self) -> usize {
        self.frames
    }

    /// Channel `c`'s writable plane (`frames` valid prefix).
    #[must_use]
    pub fn channel_mut(&mut self, c: usize) -> Option<&mut [f32]> {
        self.slot
            .data
            .channel_mut(c)
            .map(|plane| &mut plane[..self.frames])
    }
}

/// Per-node I/O context passed to
/// [`GraphProcessor::process`](crate::typed::GraphProcessor::process).
///
/// Borrows the arena (`'a`); holds the immutable `ScheduledNode`
/// wiring plus the callback `seq` used by the uninitialized-read
/// audit.
pub struct NodeIo<'a> {
    pub(crate) arena: &'a [core::cell::RefCell<Slot>],
    pub(crate) node: &'a ScheduledNode,
    pub(crate) frames: usize,
    pub(crate) seq: u64,
    pub(crate) status: &'a SharedStatus,
}

impl<'a> NodeIo<'a> {
    /// Number of audio input ports bound for this node.
    #[must_use]
    pub fn inputs(&self) -> usize {
        self.node.inputs.len()
    }

    /// Number of audio output ports bound for this node.
    #[must_use]
    pub fn outputs(&self) -> usize {
        self.node.outputs.len()
    }

    /// Frames for this callback.
    #[must_use]
    pub const fn frames(&self) -> usize {
        self.frames
    }

    /// Input port `i`'s view — resolved to its scratch slot (merge)
    /// or directly to the producer's slot.
    ///
    /// `None` when `i` is out of range, when the port is unconnected,
    /// or when the producer slot carries a stale `stamp` — an
    /// uninitialized read; the fault is recorded on the shared
    /// status (`UninitializedRead`) instead of being silently served.
    #[must_use]
    pub fn input(&self, i: usize) -> Option<InputView<'a>> {
        let binding = self.node.inputs.get(i)?;
        let slot_idx = if let Some(scratch) = binding.scratch {
            scratch
        } else {
            let edge = binding.edges.first()?;
            // Uninitialized-read audit: a direct edge to a slot the
            // producer has not written this callback is rejected.
            let src = self.arena.get(edge.slot)?;
            if src.borrow().stamp() != self.seq {
                self.status.record_uninit_read();
                return None;
            }
            edge.slot
        };
        let slot = self.arena.get(slot_idx)?.borrow();
        Some(InputView {
            slot,
            frames: self.frames,
        })
    }

    /// Output port `i`'s writable view. `None` when out of range.
    #[must_use]
    pub fn output(&self, i: usize) -> Option<OutputView<'a>> {
        let slot_idx = *self.node.outputs.get(i)?;
        let slot = self.arena.get(slot_idx)?.borrow_mut();
        Some(OutputView {
            slot,
            frames: self.frames,
        })
    }
}
