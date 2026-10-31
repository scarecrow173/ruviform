//! Contract tests for the Issue #171 `DataRegistry` release path:
//! explicit `unregister`/`clear`, the `max_sources` LRU bound, and
//! deferred-free semantics for in-flight pulls.

use std::sync::Arc;

use ruviform_ui::{DataError, DataRegistry, DataRegistryLimits, DataRequest, VecDataSource};

fn source(byte: u8, len: usize) -> Arc<VecDataSource> {
    Arc::new(VecDataSource::new(vec![byte; len]))
}

fn pull(registry: &DataRegistry, name: &str) -> Result<Vec<u8>, DataError> {
    registry
        .read(&DataRequest {
            name: name.into(),
            offset: 0,
            len: None,
        })
        .map(|c| c.bytes)
}

#[test]
fn unregister_releases_then_reregisters() {
    let registry = DataRegistry::new();
    registry.register("view/1", source(1, 8));
    assert_eq!(registry.len(), 1);
    assert!(!registry.is_empty());

    // Released name is gone for both get() and bounded reads.
    assert!(registry.unregister("view/1"));
    assert!(registry.get("view/1").is_none());
    assert!(registry.is_empty());
    assert!(matches!(
        pull(&registry, "view/1"),
        Err(DataError::NotFound { .. })
    ));

    // Double-unregister is a no-op, not an error.
    assert!(!registry.unregister("view/1"));

    // The name is free for reuse — re-registration serves new bytes.
    registry.register("view/1", source(2, 4));
    assert_eq!(pull(&registry, "view/1").unwrap(), vec![2u8; 4]);
}

#[test]
fn clear_removes_all_sources() {
    let registry = DataRegistry::new();
    for i in 0..4 {
        registry.register(&format!("view/{i}"), source(i as u8, 8));
    }
    assert_eq!(registry.len(), 4);
    registry.clear();
    assert!(registry.is_empty());
    assert!(registry.names().is_empty());
    assert!(matches!(
        pull(&registry, "view/0"),
        Err(DataError::NotFound { .. })
    ));
}

#[test]
fn held_source_outlives_unregister() {
    let registry = DataRegistry::new();
    registry.register("view/1", source(7, 16));
    // A pull already holding the Arc finishes normally after release.
    let held = registry.get("view/1").unwrap();
    assert!(registry.unregister("view/1"));
    let mut buf = [0u8; 16];
    assert_eq!(held.read(0, &mut buf).unwrap(), 16);
    assert_eq!(buf, [7u8; 16]);
    // New pulls for the name fail.
    assert!(matches!(
        pull(&registry, "view/1"),
        Err(DataError::NotFound { .. })
    ));
}

#[test]
fn unbounded_default_keeps_every_registration() {
    let registry = DataRegistry::new();
    for i in 0..64 {
        registry.register(&format!("view/{i}"), source(1, 8));
    }
    assert_eq!(registry.len(), 64);
}

#[test]
fn max_sources_evicts_least_recently_used() {
    let registry = DataRegistry::with_max_sources(2);
    registry.register("view/1", source(1, 8));
    registry.register("view/2", source(2, 8));
    registry.register("view/3", source(3, 8));
    // Oldest registration is evicted past the bound.
    assert_eq!(
        registry.names(),
        vec!["view/2".to_string(), "view/3".to_string()]
    );
    assert!(matches!(
        pull(&registry, "view/1"),
        Err(DataError::NotFound { .. })
    ));

    // Reading "view/2" makes it hot; the next registration evicts
    // "view/3" instead.
    pull(&registry, "view/2").unwrap();
    registry.register("view/4", source(4, 8));
    assert_eq!(
        registry.names(),
        vec!["view/2".to_string(), "view/4".to_string()]
    );
}

#[test]
fn reregister_counts_as_recent_use() {
    let registry = DataRegistry::with_max_sources(2);
    registry.register("view/1", source(1, 8));
    registry.register("view/2", source(2, 8));
    // Re-publishing "view/1" refreshes it, so "view/2" is evicted next.
    registry.register("view/1", source(9, 8));
    registry.register("view/3", source(3, 8));
    assert_eq!(
        registry.names(),
        vec!["view/1".to_string(), "view/3".to_string()]
    );
    // Replacement also serves the new payload.
    assert_eq!(pull(&registry, "view/1").unwrap(), vec![9u8; 8]);
}

#[test]
fn sequential_payload_names_stay_bounded() {
    // The audio-lens view-op pattern: every op mints the next
    // sequential payload name. With a bound, long sessions keep a
    // constant working set instead of growing without limit.
    let registry = DataRegistry::with_max_sources(8);
    for i in 0..1000 {
        registry.register(&format!("view/{i}"), source(1, 1024));
    }
    assert_eq!(registry.len(), 8);
    for i in 992..1000 {
        assert!(pull(&registry, &format!("view/{i}")).is_ok());
    }
    assert!(matches!(
        pull(&registry, "view/991"),
        Err(DataError::NotFound { .. })
    ));
}

#[test]
fn max_sources_zero_retains_nothing() {
    let registry = DataRegistry::with_limits(DataRegistryLimits::with_max_sources(0));
    registry.register("view/1", source(1, 8));
    assert!(registry.is_empty());
    assert!(matches!(
        pull(&registry, "view/1"),
        Err(DataError::NotFound { .. })
    ));
}

#[test]
fn eviction_and_unregister_drop_the_registry_arc() {
    let registry = DataRegistry::with_max_sources(1);
    registry.register("view/1", source(1, 8));

    // An Arc clone held elsewhere survives eviction.
    let held = registry.get("view/1").unwrap();
    assert_eq!(Arc::strong_count(&held), 2);
    registry.register("view/2", source(2, 8));
    assert_eq!(
        Arc::strong_count(&held),
        1,
        "evicted name's registry Arc is dropped"
    );
    let mut buf = [0u8; 8];
    assert_eq!(held.read(0, &mut buf).unwrap(), 8);

    // unregister drops the registry Arc the same way.
    let held2 = registry.get("view/2").unwrap();
    assert!(registry.unregister("view/2"));
    assert_eq!(Arc::strong_count(&held2), 1);
}
