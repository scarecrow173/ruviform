//! Contract fixtures for the driver-neutral `NativeSurfaceSlot` port
//! (Issue #388) via `FakeNativeSlot`.
//!
//! These pin the port semantics a real GPUI/OS driver must honor:
//! attach / hide / resize / clip / z-order / focus, typed refusals,
//! attach-failure rollback, stale/forged ids and leases, teardown
//! ordering, and the 2-phase Dock move (`DockWorkspace` begin/abort/
//! commit) linked to slot busy marks.
//!
//! **Fake PASS is contract conformance only — not a real Dock PASS.**
//! No OS window, GPUI surface or plugin participates; real-machine
//! outcomes stay `NotRun` in the capability matrix.

use ruviform_ui::native_slot::{
    LogicalRect, SlotCapabilities, SlotError, SlotEvent, SlotGeometry, SlotLease, SlotSpec,
    SlotState, SlotVisibility, Support, SurfaceSlotId,
};
use ruviform_ui::{DockTarget, DockWorkspace, NativeSurfaceSlot, ViewInstanceId, WindowId};
use ruviform_ui_test::FakeNativeSlot;

fn win(raw: u64) -> WindowId {
    WindowId::new(raw).expect("nonzero")
}

fn view(raw: u64) -> ViewInstanceId {
    ViewInstanceId::new(raw).expect("nonzero")
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn geom(x: f64, y: f64, width: f64, height: f64) -> SlotGeometry {
    SlotGeometry {
        rect: rect(x, y, width, height),
        scale_factor: 1.0,
    }
}

/// A slot that declares attach / hide / clip / z-order / focus support.
fn caps_full() -> SlotCapabilities {
    SlotCapabilities {
        attach_parent: Support::Supported,
        hide_show: Support::Supported,
        clip: Support::Supported,
        z_order: Support::Supported,
        focus: Support::Supported,
        ..SlotCapabilities::default()
    }
}

fn spec_on(window: WindowId, view_id: ViewInstanceId, caps: SlotCapabilities) -> SlotSpec {
    SlotSpec {
        window,
        view: view_id,
        geometry: geom(0.0, 0.0, 400.0, 300.0),
        visibility: SlotVisibility::Visible,
        capabilities: caps,
    }
}

fn spec() -> SlotSpec {
    spec_on(win(1), view(10), caps_full())
}

/// Declares a slot and attaches its child; returns (fake, slot, lease).
fn attached() -> (FakeNativeSlot, SurfaceSlotId, SlotLease) {
    let mut fake = FakeNativeSlot::new();
    let slot = fake.declare_slot(spec()).expect("declare");
    let lease = fake.lease_parent(slot).expect("lease");
    (fake, slot, lease)
}

#[test]
fn attach_applies_recorded_clip_and_z_to_the_child() {
    let mut fake = FakeNativeSlot::new();
    let slot = fake.declare_slot(spec()).expect("declare");
    let clip = rect(10.0, 10.0, 100.0, 80.0);
    fake.set_clip(slot, Some(clip)).expect("clip before lease");
    fake.set_zorder(slot, 3).expect("z before lease");
    assert_eq!(fake.child(slot), None, "no child before attach");

    let lease = fake.lease_parent(slot).expect("lease");
    let child = fake.child(slot).expect("attached child");
    assert_eq!(child.lease, lease);
    assert_eq!(child.geometry, geom(0.0, 0.0, 400.0, 300.0));
    assert_eq!(child.visibility, SlotVisibility::Visible);
    assert_eq!(child.clip, Some(clip));
    assert_eq!(child.z, 3);
    assert!(!child.focused);
    assert_eq!(fake.attached_count(), 1);
    assert!(fake.validate_lease(lease).is_ok());
}

#[test]
fn move_and_hide_are_applied_to_the_attached_child_and_keep_the_lease() {
    let (mut fake, slot, lease) = attached();
    fake.set_geometry(slot, geom(40.0, 40.0, 360.0, 260.0))
        .expect("move");
    fake.set_visibility(slot, SlotVisibility::Hidden)
        .expect("hide");

    let child = fake.child(slot).expect("still attached");
    assert_eq!(child.geometry, geom(40.0, 40.0, 360.0, 260.0));
    assert_eq!(child.visibility, SlotVisibility::Hidden);
    assert_eq!(child.lease, lease, "hide does not detach");
    assert!(fake.validate_lease(lease).is_ok());

    let events = fake.drain_events();
    assert!(events.contains(&SlotEvent::GeometryChanged { slot }));
    assert!(events.contains(&SlotEvent::VisibilityChanged { slot }));
}

#[test]
fn resize_rejects_invalid_geometry_and_leaves_the_child_unchanged() {
    let (mut fake, slot, _lease) = attached();
    let before = fake.child(slot).expect("child");
    let nan = geom(0.0, 0.0, f64::NAN, 300.0);
    let negative = geom(0.0, 0.0, 400.0, -1.0);
    assert_eq!(fake.set_geometry(slot, nan), Err(SlotError::InvalidValue));
    assert_eq!(
        fake.set_geometry(slot, negative),
        Err(SlotError::InvalidValue)
    );
    assert_eq!(fake.child(slot), Some(before));
}

#[test]
fn unsupported_capabilities_are_explicit_refusals_not_fake_successes() {
    let caps = SlotCapabilities::default();
    let mut fake = FakeNativeSlot::new();
    let slot = fake
        .declare_slot(spec_on(win(1), view(10), caps))
        .expect("declare");
    assert_eq!(fake.set_clip(slot, None), Err(SlotError::Unsupported));
    assert_eq!(fake.set_zorder(slot, 1), Err(SlotError::Unsupported));
    assert_eq!(
        fake.set_visibility(slot, SlotVisibility::Hidden),
        Err(SlotError::Unsupported)
    );
    assert_eq!(fake.lease_parent(slot), Err(SlotError::Unsupported));
    assert_eq!(fake.child(slot), None);
    assert_eq!(
        fake.state(slot),
        Some(SlotState::Declared),
        "a refused attach leaves the slot declared"
    );
}

#[test]
fn focus_requires_an_attached_child_and_moves_between_slots() {
    let mut fake = FakeNativeSlot::new();
    let a = fake.declare_slot(spec()).expect("declare a");
    let b = fake
        .declare_slot(spec_on(win(1), view(11), caps_full()))
        .expect("declare b");
    assert_eq!(
        fake.request_focus(a),
        Err(SlotError::Backend),
        "no child to focus"
    );

    fake.lease_parent(a).expect("lease a");
    fake.lease_parent(b).expect("lease b");
    fake.request_focus(a).expect("focus a");
    assert!(fake.child(a).is_some_and(|c| c.focused));
    fake.request_focus(b).expect("focus b");
    assert!(!fake.child(a).is_some_and(|c| c.focused));
    assert!(fake.child(b).is_some_and(|c| c.focused));
}

#[test]
fn attach_fault_is_reported_and_leaks_no_live_lease() {
    let mut fake = FakeNativeSlot::new();
    let slot = fake.declare_slot(spec()).expect("declare");
    fake.arm_attach_fault(SlotError::Backend);

    assert_eq!(fake.lease_parent(slot), Err(SlotError::Backend));
    assert_eq!(fake.state(slot), Some(SlotState::Declared));
    assert_eq!(fake.child(slot), None);
    assert_eq!(fake.attached_count(), 0);

    let retry = fake
        .lease_parent(slot)
        .expect("retry after the fault is consumed");
    assert!(fake.validate_lease(retry).is_ok());
}

#[test]
fn stale_ids_and_leases_are_rejected_after_index_reuse() {
    let (mut fake, old, old_lease) = attached();
    fake.release_slot(old).expect("release");
    assert_eq!(
        fake.set_geometry(old, geom(0.0, 0.0, 1.0, 1.0)),
        Err(SlotError::Released),
        "a released id reports Released before its index is reused"
    );

    let fresh = fake.declare_slot(spec()).expect("recycled declare");
    assert_ne!(fresh, old);
    assert_eq!(fresh.index(), old.index());
    assert_eq!(
        fake.set_geometry(old, geom(0.0, 0.0, 1.0, 1.0)),
        Err(SlotError::StaleGeneration)
    );
    assert_eq!(fake.capabilities(old), Err(SlotError::StaleGeneration));
    assert_eq!(fake.child(old), None);
    assert_eq!(
        fake.validate_lease(old_lease).map(|_| ()),
        Err(SlotError::LeaseRevoked),
        "a lease from the previous generation is expired, not valid"
    );

    let fresh_lease = fake.lease_parent(fresh).expect("new generation leases");
    assert!(fake.validate_lease(fresh_lease).is_ok());
}

#[test]
fn forged_and_double_leases_are_rejected() {
    let (mut fake, slot, lease) = attached();
    let forged = SlotLease::encode_parts(slot.index(), 9_999);
    assert_eq!(
        fake.validate_lease(forged).map(|_| ()),
        Err(SlotError::InvalidLease)
    );
    assert_eq!(fake.release_lease(forged), Err(SlotError::InvalidLease));
    assert_eq!(fake.lease_parent(slot), Err(SlotError::LeaseBusy));
    assert!(
        fake.validate_lease(lease).is_ok(),
        "the live lease survives"
    );
}

#[test]
fn release_lease_detaches_the_child_and_a_new_lease_is_distinct() {
    let (mut fake, slot, lease) = attached();
    fake.release_lease(lease).expect("release lease");
    assert_eq!(fake.child(slot), None);
    assert_eq!(fake.state(slot), Some(SlotState::Declared));
    assert_eq!(
        fake.validate_lease(lease).map(|_| ()),
        Err(SlotError::LeaseRevoked)
    );

    let again = fake.lease_parent(slot).expect("re-attach");
    assert_ne!(again, lease);
    assert!(fake.child(slot).is_some());
}

#[test]
fn teardown_detaches_child_and_emits_revoke_then_destroyed() {
    let (mut fake, slot, lease) = attached();
    fake.drain_events();
    fake.release_slot(slot).expect("teardown");

    assert_eq!(
        fake.drain_events(),
        vec![
            SlotEvent::LeaseRevoked { slot },
            SlotEvent::Destroyed { slot }
        ]
    );
    assert_eq!(fake.child(slot), None);
    assert_eq!(fake.attached_count(), 0);
    assert_eq!(fake.state(slot), Some(SlotState::Released));
    assert_eq!(fake.lease_parent(slot), Err(SlotError::Released));
    assert_eq!(
        fake.validate_lease(lease).map(|_| ()),
        Err(SlotError::LeaseRevoked)
    );
}

#[test]
fn busy_mark_refuses_new_leases_until_cleared() {
    let (mut fake, slot, lease) = attached();
    fake.release_lease(lease).expect("detach");
    fake.mark_busy(slot).expect("busy");
    assert_eq!(fake.lease_parent(slot), Err(SlotError::SlotBusy));
    fake.clear_busy(slot).expect("clear");
    assert!(fake.lease_parent(slot).is_ok());
}

/// Dock 2-phase move, abort path (issue-182 `DockWorkspace` +
/// D5 contract): `begin_move` marks the slot busy while the live child
/// stays attached; `abort_move` restores the source with the same lease.
#[test]
fn dock_move_abort_rolls_back_with_the_live_lease_kept() {
    let (w1, w2, v) = (win(1), win(2), view(10));
    let mut ws = DockWorkspace::new();
    ws.register_window(w1);
    ws.register_window(w2);
    ws.attach_view(v, DockTarget::Window(w1))
        .expect("attach view");

    let mut fake = FakeNativeSlot::new();
    let slot = fake
        .declare_slot(spec_on(w1, v, caps_full()))
        .expect("declare");
    let lease = fake.lease_parent(slot).expect("lease");

    let token = ws
        .begin_move(v, DockTarget::Window(w2))
        .expect("begin move");
    fake.mark_busy(slot).expect("busy during transfer");
    assert!(ws.is_transferring(v));
    assert_eq!(fake.child(slot).map(|c| c.lease), Some(lease));

    ws.abort_move(token).expect("abort");
    fake.clear_busy(slot).expect("clear busy");
    assert_eq!(ws.owner_of(v), Some(w1), "view stays at its source window");
    assert!(!ws.is_transferring(v));
    assert_eq!(fake.child(slot).map(|c| c.lease), Some(lease));
    assert!(fake.validate_lease(lease).is_ok());
}

/// Dock 2-phase move, commit path: the slot is torn down under the old
/// window and re-declared under the target window with a new lease; the
/// old lease is revoked and the old id is terminal.
#[test]
fn dock_move_commit_reattaches_under_a_new_slot_and_revokes_the_old() {
    let (w1, w2, v) = (win(1), win(2), view(10));
    let mut ws = DockWorkspace::new();
    ws.register_window(w1);
    ws.register_window(w2);
    ws.attach_view(v, DockTarget::Window(w1))
        .expect("attach view");

    let mut fake = FakeNativeSlot::new();
    let old = fake
        .declare_slot(spec_on(w1, v, caps_full()))
        .expect("declare");
    let old_lease = fake.lease_parent(old).expect("lease");

    let token = ws
        .begin_move(v, DockTarget::Window(w2))
        .expect("begin move");
    fake.mark_busy(old).expect("busy during transfer");
    ws.commit_move(token).expect("commit");
    assert_eq!(ws.owner_of(v), Some(w2));

    fake.release_lease(old_lease).expect("detach from source");
    fake.clear_busy(old).expect("clear busy");
    fake.release_slot(old).expect("retire source slot");

    let new = fake
        .declare_slot(spec_on(w2, v, caps_full()))
        .expect("declare at target");
    let new_lease = fake.lease_parent(new).expect("lease at target");
    assert_eq!(fake.child(new).map(|c| c.lease), Some(new_lease));
    assert_eq!(
        fake.validate_lease(old_lease).map(|_| ()),
        Err(SlotError::LeaseRevoked)
    );
    assert_eq!(
        fake.capabilities(old),
        Err(SlotError::StaleGeneration),
        "the retired id is stale once its index is recycled for the target window"
    );
}
