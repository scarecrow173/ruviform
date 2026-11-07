//! Headless contract tests for `ruviform_ui::native_slot`
//! (`SurfaceSlotRegistry` / `SlotLease` / `NativeSurfaceSlot`).
//!
//! These pin the *driver-neutral* slot/lease contract — mint, validate,
//! revoke, stale generation, busy, teardown, event drain. They are
//! **not** evidence of a real Dock PASS: no GPUI window, OS parent or
//! plugin participates. Real-machine outcomes stay `NotRun`/`Blocked`
//! until an RF386 GPUI fixture and real plugin binaries exist.

use ruviform_ui::native_slot::{
    LeaseInfo, LogicalRect, SlotCapabilities, SlotError, SlotEvent, SlotGeometry, SlotSpec,
    SlotState, SlotVisibility, Support, SurfaceSlotRegistry,
};
use ruviform_ui::{ViewInstanceId, WindowId};

fn window() -> WindowId {
    WindowId::new(1).expect("nonzero")
}

fn view() -> ViewInstanceId {
    ViewInstanceId::new(10).expect("nonzero")
}

fn geometry() -> SlotGeometry {
    SlotGeometry {
        rect: LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        },
        scale_factor: 1.0,
    }
}

fn spec() -> SlotSpec {
    spec_with(SlotCapabilities {
        attach_parent: Support::Supported,
        hide_show: Support::Supported,
        clip: Support::Supported,
        ..SlotCapabilities::default()
    })
}

fn spec_with(capabilities: SlotCapabilities) -> SlotSpec {
    SlotSpec {
        window: window(),
        view: view(),
        geometry: geometry(),
        visibility: SlotVisibility::Visible,
        capabilities,
    }
}

#[test]
fn declare_slot_returns_declared_live_id() {
    let mut registry = SurfaceSlotRegistry::new();
    let slot = registry.declare_slot(spec()).expect("declare");
    assert_eq!(registry.state(slot), Some(SlotState::Declared));
    assert_eq!(registry.spec(slot).expect("spec").view, view());
    assert!(
        registry
            .capabilities(slot)
            .expect("caps")
            .attach_parent
            .supported()
    );
}

#[test]
fn declare_rejects_non_finite_geometry() {
    let mut registry = SurfaceSlotRegistry::new();
    let mut bad = spec();
    bad.geometry.rect.width = f64::NAN;
    assert_eq!(registry.declare_slot(bad), Err(SlotError::InvalidValue));
    let mut bad = spec();
    bad.geometry.rect.width = -1.0;
    assert_eq!(registry.declare_slot(bad), Err(SlotError::InvalidValue));
    let mut bad = spec();
    bad.geometry.scale_factor = 0.0;
    assert_eq!(registry.declare_slot(bad), Err(SlotError::InvalidValue));
}

#[test]
fn lease_parent_validates_and_one_live_per_slot() {
    let mut registry = SurfaceSlotRegistry::new();
    let slot = registry.declare_slot(spec()).unwrap();
    let lease = registry.lease_parent(slot).expect("lease");
    assert_eq!(registry.state(slot), Some(SlotState::Leased));
    assert_eq!(
        registry.validate_lease(lease).expect("valid"),
        LeaseInfo { slot, seq: 0 }
    );
    // A second lease while the first is live is refused.
    assert_eq!(registry.lease_parent(slot), Err(SlotError::LeaseBusy));
}

#[test]
fn released_lease_is_revoked_and_releasable() {
    let mut registry = SurfaceSlotRegistry::new();
    let slot = registry.declare_slot(spec()).unwrap();
    let lease = registry.lease_parent(slot).unwrap();
    registry.release_lease(lease).expect("release");
    // Old lease is now revoked.
    assert_eq!(registry.validate_lease(lease), Err(SlotError::LeaseRevoked));
    // A fresh lease is mintable and carries a new sequence.
    let lease2 = registry.lease_parent(slot).expect("re-lease");
    assert_eq!(lease2.seq(), 1);
    assert!(registry.validate_lease(lease2).is_ok());
    // Mint/revoke transitions are observable: Declared + Leased on
    // mint, LeaseRevoked on release, Leased on re-mint.
    assert_eq!(
        registry.drain_events(),
        vec![
            SlotEvent::Declared { slot },
            SlotEvent::Leased { slot },
            SlotEvent::LeaseRevoked { slot },
            SlotEvent::Leased { slot },
        ]
    );
}

#[test]
fn unknown_and_forged_leases_are_rejected() {
    let mut registry = SurfaceSlotRegistry::new();
    let slot = registry.declare_slot(spec()).unwrap();
    // Unknown index.
    let bogus = ruviform_ui::native_slot::SlotLease::encode_parts(99, 0);
    assert_eq!(registry.validate_lease(bogus), Err(SlotError::UnknownSlot));
    // A seq never minted by this registry — a forged token.
    let forged = ruviform_ui::native_slot::SlotLease::encode_parts(slot.index(), 42);
    assert_eq!(
        registry.validate_lease(forged),
        Err(SlotError::InvalidLease)
    );
    // A minted seq on the wrong slot is not live there — issued once,
    // now dead → revoked, never a fake success.
    let live = registry.lease_parent(slot).expect("lease");
    assert!(registry.validate_lease(live).is_ok());
    registry.release_lease(live).unwrap();
    assert_eq!(registry.validate_lease(live), Err(SlotError::LeaseRevoked));
}

#[test]
fn release_slot_revokes_lease_and_destroyed_is_terminal() {
    let mut registry = SurfaceSlotRegistry::new();
    let slot = registry.declare_slot(spec()).unwrap();
    let lease = registry.lease_parent(slot).unwrap();
    registry.drain_events(); // consume Declared + Leased
    registry.release_slot(slot).expect("release");
    assert_eq!(registry.state(slot), Some(SlotState::Released));
    assert_eq!(registry.validate_lease(lease), Err(SlotError::LeaseRevoked));
    assert_eq!(registry.lease_parent(slot), Err(SlotError::Released));
    assert_eq!(
        registry.set_visibility(slot, SlotVisibility::Hidden),
        Err(SlotError::Released)
    );
    assert_eq!(
        registry.drain_events(),
        vec![
            SlotEvent::LeaseRevoked { slot },
            SlotEvent::Destroyed { slot },
        ]
    );
    // `Destroyed` is terminal — releasing again errors, no more events.
    assert_eq!(registry.release_slot(slot), Err(SlotError::Released));
    assert_eq!(registry.drain_events(), Vec::<SlotEvent>::new());
}

#[test]
fn released_slot_index_recycles_with_bumped_generation() {
    let mut registry = SurfaceSlotRegistry::new();
    let slot = registry.declare_slot(spec()).unwrap();
    let index = slot.index();
    let generation = slot.generation();
    registry.release_slot(slot).unwrap();
    let slot2 = registry.declare_slot(spec()).expect("re-declare");
    assert_eq!(slot2.index(), index);
    assert_eq!(slot2.generation(), generation.wrapping_add(1));
    // The old id is now stale; the new id is live.
    assert_eq!(registry.state(slot), None);
    assert_eq!(registry.state(slot2), Some(SlotState::Declared));
    assert_eq!(registry.lease_parent(slot), Err(SlotError::StaleGeneration));
    assert!(registry.lease_parent(slot2).is_ok());
}

#[test]
fn busy_slot_refuses_new_leases_until_cleared() {
    let mut registry = SurfaceSlotRegistry::new();
    let slot = registry.declare_slot(spec()).unwrap();
    registry.mark_busy(slot).expect("busy");
    assert_eq!(registry.lease_parent(slot), Err(SlotError::SlotBusy));
    registry.clear_busy(slot).expect("clear");
    assert!(registry.lease_parent(slot).is_ok());
}

#[test]
fn geometry_and_visibility_changes_emit_events() {
    let mut registry = SurfaceSlotRegistry::new();
    let slot = registry.declare_slot(spec()).unwrap();
    let geo = SlotGeometry {
        rect: LogicalRect {
            x: 10.0,
            y: 20.0,
            width: 640.0,
            height: 480.0,
        },
        scale_factor: 2.0,
    };
    registry.drain_events(); // consume Declared
    registry.set_geometry(slot, geo).expect("geometry");
    registry
        .set_visibility(slot, SlotVisibility::Hidden)
        .expect("hide");
    assert_eq!(
        registry.drain_events(),
        vec![
            SlotEvent::GeometryChanged { slot },
            SlotEvent::VisibilityChanged { slot },
        ]
    );
    let spec = registry.spec(slot).unwrap();
    assert_eq!(spec.geometry, geo);
    assert_eq!(spec.visibility, SlotVisibility::Hidden);
}

#[test]
fn declared_unsupported_capability_is_explicit() {
    let mut registry = SurfaceSlotRegistry::new();
    let slot = registry
        .declare_slot(spec_with(SlotCapabilities::default()))
        .unwrap();
    let caps = registry.capabilities(slot).unwrap();
    // No flags set — nothing is claimed; a driver must refuse ops on
    // these rather than fake success.
    assert!(!caps.attach_parent.supported());
    assert!(!caps.ime.supported());
    assert!(!caps.popup.supported());
    assert!(!caps.clip.supported());
    // Enforcement is real: ops on declared-Unsupported capabilities
    // are explicit refusals, never recorded successes.
    assert_eq!(registry.lease_parent(slot), Err(SlotError::Unsupported));
    assert_eq!(
        registry.set_visibility(slot, SlotVisibility::Hidden),
        Err(SlotError::Unsupported)
    );
    assert_eq!(registry.state(slot), Some(SlotState::Declared));
}

#[test]
fn lease_encode_decode_roundtrips_parts() {
    let lease = ruviform_ui::native_slot::SlotLease::encode_parts(7, 0xBEEF_0003);
    assert_eq!(lease.index(), 7);
    assert_eq!(lease.seq(), 0xBEEF_0003);
    assert_eq!(
        ruviform_ui::native_slot::SlotLease::new(lease.raw()).raw(),
        lease.raw()
    );
}
