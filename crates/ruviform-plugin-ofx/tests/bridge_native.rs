//! Bottom-up C ABI execution gate, independent of the public Rust facade.
//! This owns the real DLL until catalog Unload and host teardown finish.

#[expect(unsafe_code)]
mod native {
    use std::ffi::{CStr, c_char, c_void};
    use std::path::Path;

    use libloading::Library;

    #[link(name = "ofx_bridge", kind = "static")]
    unsafe extern "C" {
        fn rv_ofx_host_new() -> *mut c_void;
        fn rv_ofx_host_free(host: *mut c_void);
        fn rv_ofx_catalog_open(
            host: *mut c_void,
            get_count: *const c_void,
            get_plugin: *const c_void,
            set_host: *const c_void,
            catalog: *mut *mut c_void,
        ) -> i32;
        fn rv_ofx_catalog_close(catalog: *mut c_void);
        fn rv_ofx_catalog_count(catalog: *mut c_void) -> usize;
        fn rv_ofx_catalog_descriptor(catalog: *mut c_void, index: usize) -> *mut c_void;
        fn rv_ofx_descriptor_string(descriptor: *mut c_void, field: i32) -> *const c_char;
        fn rv_ofx_param_count(effect: *mut c_void) -> usize;
        fn rv_ofx_param_set(
            effect: *mut c_void,
            index: usize,
            kind: i32,
            double: f64,
            integer: i32,
            host_write: i32,
        ) -> i32;
        fn rv_ofx_instance_new(descriptor: *mut c_void, effect: *mut *mut c_void) -> i32;
        fn rv_ofx_instance_configure(
            effect: *mut c_void, width: u32, height: u32, frame_rate: f64,
        ) -> i32;
        fn rv_ofx_instance_create(effect: *mut c_void) -> i32;
        fn rv_ofx_instance_destroy(effect: *mut c_void) -> i32;
        fn rv_ofx_instance_release(effect: *mut c_void);
    }

    struct Bridge {
        host: *mut c_void,
        catalog: *mut c_void,
        effect: *mut c_void,
        library: Library,
    }

    impl Bridge {
        fn new() -> Self {
            let path = Path::new(env!("OFX_FIXTURE_DLL"));
            // SAFETY: path is our SDK-built fixture; all signatures match
            // fixture_control.h and the primitive-only ofx_bridge.h.
            unsafe {
                let library = Library::new(path).expect("compiled fixture");
                let configure = library
                    .get::<unsafe extern "C" fn(i32, Option<extern "C" fn(i32, i32)>)>(
                        b"ofx_fixture_configure\0",
                    )
                    .expect("fixture configure");
                // The native Create action itself spawns and joins the worker.
                configure(1 << 14, None);
                let mut bridge = Self {
                    host: rv_ofx_host_new(),
                    catalog: std::ptr::null_mut(),
                    effect: std::ptr::null_mut(),
                    library,
                };
                assert!(!bridge.host.is_null(), "real host allocation");
                let count = *bridge.library.get::<*const c_void>(
                    b"OfxGetNumberOfPlugins\0",
                ).expect("count symbol");
                let plugin = *bridge.library.get::<*const c_void>(
                    b"OfxGetPlugin\0",
                ).expect("plugin symbol");
                let set_host = *bridge.library.get::<*const c_void>(
                    b"OfxSetHost\0",
                ).expect("DLL SetHost symbol");
                assert_eq!(
                    rv_ofx_catalog_open(
                        bridge.host, count, plugin, set_host, &raw mut bridge.catalog,
                    ),
                    0,
                    "native catalog open",
                );
                assert!(!bridge.catalog.is_null());
                bridge
            }
        }
    }

    impl Drop for Bridge {
        fn drop(&mut self) {
            // SAFETY: these resources belong exclusively to this test and are
            // released in dependency order, before the real DLL field drops.
            unsafe {
                if !self.effect.is_null() {
                    rv_ofx_instance_release(self.effect);
                }
                if !self.catalog.is_null() {
                    rv_ofx_catalog_close(self.catalog);
                }
                if !self.host.is_null() {
                    rv_ofx_host_free(self.host);
                }
            }
        }
    }

    #[test]
    fn native_bridge_executes_sdk_actions_and_all_suite_probes() {
        let mut bridge = Bridge::new();
        // SAFETY: all handles are obtained from the live owned catalog.
        // The fixture exports call actual SDK suites, not Rust-side mocks.
        unsafe {
            assert_eq!(rv_ofx_catalog_count(bridge.catalog), 2);
            let descriptor = rv_ofx_catalog_descriptor(bridge.catalog, 0);
            assert!(!descriptor.is_null(), "Filter admission");
            assert!(rv_ofx_catalog_descriptor(bridge.catalog, 1).is_null());
            let identifier = rv_ofx_descriptor_string(descriptor, 0);
            assert!(!identifier.is_null());
            assert_eq!(CStr::from_ptr(identifier).to_str().expect("UTF-8"), "org.ruviform.gain");
            assert_eq!(rv_ofx_param_count(descriptor), 5);
            assert_eq!(rv_ofx_instance_new(descriptor, &raw mut bridge.effect), 0);
            assert!(!bridge.effect.is_null());
            assert_eq!(rv_ofx_param_count(bridge.effect), 5);
            assert_eq!(rv_ofx_instance_configure(bridge.effect, 3, 2, 30.0), 0);
            assert_eq!(rv_ofx_param_set(bridge.effect, 0, 0, 3.0, 0, 1), 0);
            assert_eq!(rv_ofx_instance_create(bridge.effect), 0, "Create with joined OS worker");
            let created = bridge.library
                .get::<unsafe extern "C" fn(*mut f64) -> i32>(b"ofx_fixture_created_gain\0")
                .expect("created value probe");
            let mut value = f64::NAN;
            assert_eq!(created(&raw mut value), 0);
            assert_eq!(value, 3.0, "C Create must read the staged native value");
            let probe = bridge.library
                .get::<unsafe extern "C" fn(i32) -> i32>(b"ofx_fixture_probe\0")
                .expect("suite probe");
            for which in 0..5 {
                assert_eq!(probe(which), 0, "SDK suite oracle {which}");
            }
            assert_eq!(rv_ofx_instance_destroy(bridge.effect), 0);
        }
        // Drop performs host-owned release and real plugin Unload while code
        // is still loaded. Repeating the test under tools must not leak tokens.
    }
}
