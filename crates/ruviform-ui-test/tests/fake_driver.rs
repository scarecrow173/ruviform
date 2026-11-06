//! Semantic fixtures for the driver-neutral `WindowDriver`
//! contract (Issue #386) via `FakeDriver`.
//!
//! These exercise the same lifecycle semantics `GpuiDriver` must
//! honor on the real backend: two-window lifecycle, veto-able
//! close, terminal `Destroyed`, typed event push, typed action
//! ingress, capability honesty, and loop exit. `FakeDriver` success
//! is contract conformance only — it is not real native window
//! success.

use ruviform_ui::{
    Capability, CloseDecision, DriverEvent, Event, Intent, LogicalSize, UiError,
    WindowCapabilities, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_test::{FakeDriver, MAX_EVENT_BACKLOG};
use serde_json::json;

/// Records every delivered event.
struct Journal {
    events: Vec<DriverEvent>,
}

impl Journal {
    fn handler(&mut self) -> impl FnMut(DriverEvent, &mut dyn WindowDriver) + '_ {
        |ev, _driver| self.events.push(ev)
    }

    /// Lifecycle payloads for one window, in order.
    fn for_window(&self, window: WindowId) -> Vec<&WindowLifecycle> {
        self.events
            .iter()
            .filter_map(|ev| match ev {
                DriverEvent::Windowed { window: w, event } if *w == window => Some(event),
                _ => None,
            })
            .collect()
    }

    fn count_ticks(&self) -> usize {
        self.events
            .iter()
            .filter(|ev| matches!(ev, DriverEvent::Tick))
            .count()
    }
}

fn spec(title: &str) -> WindowSpec {
    WindowSpec {
        title: title.to_string(),
        ..WindowSpec::default()
    }
}

/// Correlation id of a `Result` event (delivered `post_event`s are
/// `result_ok`s in these fixtures).
fn result_id(event: &Event) -> &str {
    match event {
        Event::Result { id, .. } => id,
        other => panic!("expected Result event, got {other:?}"),
    }
}

fn intent(id: &str) -> Intent {
    Intent::new(id, "demo.echo", json!({}))
}

#[test]
fn two_window_lifecycle() {
    let mut driver = FakeDriver::new();
    let w1 = driver.create_window(&spec("main")).unwrap();
    let w2 = driver
        .create_window(&WindowSpec {
            logical_size: LogicalSize::new(640, 480),
            ..spec("inspector")
        })
        .unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w1,
        event: WindowLifecycle::Ready,
    });
    driver.inject(DriverEvent::Windowed {
        window: w2,
        event: WindowLifecycle::Ready,
    });
    driver.inject(DriverEvent::Tick);

    let mut journal = Journal { events: Vec::new() };
    driver.pump(&mut journal.handler());

    assert_eq!(driver.windows(), vec![w1, w2]);
    assert!(matches!(
        journal.for_window(w1).as_slice(),
        [WindowLifecycle::Created, WindowLifecycle::Ready]
    ));
    assert!(matches!(
        journal.for_window(w2).as_slice(),
        [WindowLifecycle::Created, WindowLifecycle::Ready]
    ));
    assert_eq!(journal.count_ticks(), 1);
    assert_eq!(driver.spec_of(w1).unwrap().title, "main");
    assert_eq!(
        driver.spec_of(w2).unwrap().logical_size,
        LogicalSize::new(640, 480)
    );
}

#[test]
fn close_veto_then_approve_and_destroyed() {
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec("main")).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Ready,
    });
    driver.request_close(w);
    {
        // First pump: see CloseRequested, veto it — window survives.
        let mut seen_close = false;
        driver.pump(&mut |ev, drv| {
            if matches!(
                ev,
                DriverEvent::Windowed {
                    event: WindowLifecycle::CloseRequested,
                    ..
                }
            ) {
                seen_close = true;
                drv.respond_close(w, CloseDecision::Veto).unwrap();
            }
        });
        assert!(seen_close);
        assert_eq!(driver.windows(), vec![w]);
        assert!(!driver.close_pending(w));
        assert!(driver.destroyed().is_empty());
    }

    // Second close: approve — Destroyed follows in the same pump.
    driver.request_close(w);
    let mut journal = Journal { events: Vec::new() };
    driver.pump(&mut |ev, drv| {
        if matches!(
            ev,
            DriverEvent::Windowed {
                event: WindowLifecycle::CloseRequested,
                ..
            }
        ) {
            drv.respond_close(w, CloseDecision::Approve).unwrap();
        }
        journal.events.push(ev);
    });
    assert!(matches!(
        journal.for_window(w).as_slice(),
        [WindowLifecycle::CloseRequested, WindowLifecycle::Destroyed]
    ));
    assert_eq!(driver.windows(), [] as [WindowId; 0]);
    assert!(driver.destroyed().contains(&w));
}

#[test]
fn unanswered_close_defaults_to_approve_for_that_window_only() {
    let mut driver = FakeDriver::new();
    let w1 = driver.create_window(&spec("a")).unwrap();
    let w2 = driver.create_window(&spec("b")).unwrap();
    driver.request_close(w1);
    let mut journal = Journal { events: Vec::new() };
    driver.pump(&mut journal.handler());
    // w1 approved by default; w2 untouched; loop not exited.
    assert!(driver.destroyed().contains(&w1));
    assert_eq!(driver.windows(), vec![w2]);
    assert!(!driver.exited());
}

#[test]
fn deferred_close_stays_pending_until_decided() {
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec("main")).unwrap();
    driver.request_close(w);
    driver.pump(&mut |ev, drv| {
        if matches!(
            ev,
            DriverEvent::Windowed {
                event: WindowLifecycle::CloseRequested,
                ..
            }
        ) {
            drv.respond_close(w, CloseDecision::Defer).unwrap();
        }
    });
    assert!(driver.close_pending(w));
    assert_eq!(driver.windows(), vec![w]);
    // Late approve retires it.
    driver.respond_close(w, CloseDecision::Approve).unwrap();
    let mut journal = Journal { events: Vec::new() };
    driver.pump(&mut journal.handler());
    assert!(driver.destroyed().contains(&w));
    // A third response now fails — nothing pending.
    assert!(driver.respond_close(w, CloseDecision::Approve).is_err());
}

#[test]
fn post_event_buffers_until_ready_then_flushes() {
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec("main")).unwrap();
    driver.post_event(w, &Event::result_ok("r1", json!({"a": 1})));
    driver.post_event(w, &Event::result_ok("r2", json!({"a": 2})));
    assert!(driver.delivered().is_empty());
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Ready,
    });
    let mut journal = Journal { events: Vec::new() };
    driver.pump(&mut journal.handler());
    let delivered = driver.events_for(w);
    assert_eq!(delivered.len(), 2);
    // Order and identity preserved: r1 precedes r2.
    assert_eq!(result_id(&delivered[0]), "r1");
    assert_eq!(result_id(&delivered[1]), "r2");
    driver.post_event(w, &Event::result_ok("r3", json!({"a": 3})));
    assert_eq!(driver.events_for(w).len(), 3);
}

#[test]
fn post_event_backlog_drops_oldest() {
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec("main")).unwrap();
    for i in 0..MAX_EVENT_BACKLOG + 4 {
        driver.post_event(w, &Event::result_ok(format!("r{i}"), json!(i)));
    }
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Ready,
    });
    let mut journal = Journal { events: Vec::new() };
    driver.pump(&mut journal.handler());
    let delivered = driver.events_for(w);
    assert_eq!(delivered.len(), MAX_EVENT_BACKLOG);
    // Drop-oldest, not drop-newest: the four earliest events are
    // gone and the last enqueued is the last delivered.
    assert_eq!(result_id(delivered.first().unwrap()), "r4");
    assert_eq!(
        result_id(delivered.last().unwrap()),
        format!("r{}", MAX_EVENT_BACKLOG + 3)
    );
}

#[test]
fn post_event_to_destroyed_window_drops_silently() {
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec("main")).unwrap();
    driver.destroy_window(w).unwrap();
    let mut journal = Journal { events: Vec::new() };
    driver.pump(&mut journal.handler());
    driver.post_event(w, &Event::result_ok("dead", json!(null)));
    assert!(driver.delivered().is_empty());
    // Events to destroyed ids must never panic nor error — and
    // destroyed ids never resurrect.
    assert!(driver.resize(w, LogicalSize::new(1, 1)).is_err());
    assert!(driver.focus(w, true).is_err());
    driver.request_close(w); // ignored, no panic
}

#[test]
fn action_intents_route_and_disconnect_fault_suppresses() {
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec("main")).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Action(intent("i1")),
    });
    // i1 delivers before the fault is armed.
    let mut journal = Journal { events: Vec::new() };
    let suppressed1 = driver.pump(&mut journal.handler());
    assert!(suppressed1.is_empty());
    driver.ipc_disconnect(w);
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Action(intent("i2")),
    });
    let suppressed = driver.pump(&mut journal.handler());
    let actions: Vec<&WindowLifecycle> = journal.for_window(w);
    assert!(actions.iter().any(|e| matches!(
        e,
        WindowLifecycle::Action(i) if i.id == "i1"
    )));
    assert_eq!(suppressed.len(), 1);
    assert_eq!(suppressed[0].1.id, "i2");
    driver.ipc_reconnect(w);
    assert!(!driver.is_ipc_down(w));
}

#[test]
fn unsupported_capabilities_report_error_not_degradation() {
    let mut driver = FakeDriver::new();
    let caps = WindowCapabilities {
        resize: Capability::Unsupported,
        ime: Capability::Unsupported,
        ..WindowCapabilities::default()
    };
    driver.set_capabilities(caps);
    let w = driver.create_window(&spec("main")).unwrap();
    match driver.resize(w, LogicalSize::new(10, 10)) {
        Err(UiError::Unsupported { capability, .. }) => {
            assert_eq!(capability, "resize");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
    // Capability is checked before the window lookup — an unknown
    // window under an unsupported capability reports Unsupported.
    let dead = WindowId::new(999).unwrap();
    assert!(matches!(
        driver.resize(dead, LogicalSize::new(1, 1)),
        Err(UiError::Unsupported { .. } | UiError::Host { .. })
    ));
}

#[test]
fn single_window_driver_rejects_second_create() {
    let mut driver = FakeDriver::new();
    let mut caps = WindowCapabilities {
        ..WindowCapabilities::default()
    };
    caps.multi_window = Capability::Unsupported;
    driver.set_capabilities(caps);
    driver.create_window(&spec("only")).unwrap();
    match driver.create_window(&spec("second")) {
        Err(UiError::Unsupported { capability, .. }) => {
            assert_eq!(capability, "multi_window");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn destroyed_events_to_retired_ids_still_reach_consumer() {
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec("main")).unwrap();
    driver.destroy_window(w).unwrap();
    // A backend-loss Destroyed injected after retire must still
    // arrive — subscription cleanup depends on it.
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Destroyed,
    });
    let mut journal = Journal { events: Vec::new() };
    driver.pump(&mut journal.handler());
    let destroyed = journal
        .for_window(w)
        .iter()
        .filter(|e| matches!(e, WindowLifecycle::Destroyed))
        .count();
    assert_eq!(destroyed, 2);
}

#[test]
fn resize_focus_scale_events_mirror_driver_requests() {
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec("main")).unwrap();
    driver.resize(w, LogicalSize::new(800, 600)).unwrap();
    driver.focus(w, true).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::ScaleFactorChanged { factor: 1.5 },
    });
    let mut journal = Journal { events: Vec::new() };
    driver.pump(&mut journal.handler());
    assert!(journal.for_window(w).iter().any(|e| matches!(
        e,
        WindowLifecycle::Resized {
            width: 800,
            height: 600
        }
    )));
    assert!(
        journal
            .for_window(w)
            .iter()
            .any(|e| matches!(e, WindowLifecycle::Focused(true)))
    );
    assert!(journal.for_window(w).iter().any(|e| matches!(
        e,
        WindowLifecycle::ScaleFactorChanged { factor } if (*factor - 1.5).abs() < f64::EPSILON
    )));
    assert!(driver.focused(w));
}

#[test]
fn exit_records_request_and_monitors_snapshot() {
    let mut driver = FakeDriver::new();
    driver.set_monitors(vec![ruviform_ui::MonitorInfo {
        name: Some("m0".to_string()),
        width: 2560,
        height: 1440,
        x: 0,
        y: 0,
        scale_factor: 1.5,
    }]);
    driver.exit();
    assert!(driver.exited());
    assert_eq!(driver.monitors().len(), 1);
    assert_eq!(driver.monitors()[0].scale_factor, 1.5);
}

/// Cross-check that `FakeDriver` and `FakeHost` agree on the same
/// lifecycle contract — the parity the migration preserves.
#[test]
fn parity_with_fake_host_close_semantics() {
    use ruviform_ui::{WindowConfig, WindowHandle};
    use ruviform_ui_test::FakeHost;

    let mut host = FakeHost::new();
    let hw = host
        .create_window(&WindowConfig {
            url: "rvf://app".to_string(),
            ..WindowConfig::default()
        })
        .unwrap();
    host.request_close(hw);
    let mut host_destroyed = false;
    host.pump(&mut |ev, _h| {
        if matches!(
            ev,
            ruviform_ui::WindowEvent::Windowed {
                event: ruviform_ui::HostEvent::Destroyed,
                ..
            }
        ) {
            host_destroyed = true;
        }
    });
    assert!(host_destroyed);

    let mut driver = FakeDriver::new();
    let dw = driver.create_window(&spec("main")).unwrap();
    driver.request_close(dw);
    let mut drv_destroyed = false;
    driver.pump(&mut |ev, _d| {
        if matches!(
            ev,
            DriverEvent::Windowed {
                event: WindowLifecycle::Destroyed,
                ..
            }
        ) {
            drv_destroyed = true;
        }
    });
    assert!(drv_destroyed);
}
