#[expect(unsafe_code)]
mod t {
    use libloading::Library;
    use std::ffi::c_void;
    use std::path::Path;
    #[link(name = "ofx_bridge", kind = "static")]
    unsafe extern "C" {
        fn rv_ofx_host_new() -> *mut c_void;
        fn rv_ofx_host_free(h: *mut c_void);
        fn rv_ofx_catalog_open(
            h: *mut c_void,
            gc: *const c_void,
            gp: *const c_void,
            sh: *const c_void,
            c: *mut *mut c_void,
        ) -> i32;
        fn rv_ofx_catalog_close(c: *mut c_void);
        fn rv_ofx_catalog_descriptor(c: *mut c_void, i: usize) -> *mut c_void;
        fn rv_ofx_instance_new(d: *mut c_void, e: *mut *mut c_void) -> i32;
        fn rv_ofx_instance_configure(e: *mut c_void, w: u32, h: u32, fps: f64) -> i32;
        fn rv_ofx_instance_create(e: *mut c_void) -> i32;
        fn rv_ofx_instance_destroy(e: *mut c_void) -> i32;
        fn rv_ofx_instance_release(e: *mut c_void);
        fn rv_ofx_param_set(e: *mut c_void, i: usize, kind: i32, d: f64, iv: i32, hw: i32) -> i32;
    }
    #[test]
    fn probe_each() {
        let path = Path::new(env!("OFX_FIXTURE_DLL"));
        unsafe {
            let lib = Library::new(path).expect("fixture");
            let configure = lib
                .get::<unsafe extern "C" fn(i32, Option<extern "C" fn(i32, i32)>)>(
                    b"ofx_fixture_configure\0",
                )
                .unwrap();
            configure(0, None); // no worker on create
            let probe = lib
                .get::<unsafe extern "C" fn(i32) -> i32>(b"ofx_fixture_probe\0")
                .unwrap();
            let gc = *lib
                .get::<*const c_void>(b"OfxGetNumberOfPlugins\0")
                .unwrap();
            let gp = *lib.get::<*const c_void>(b"OfxGetPlugin\0").unwrap();
            let sh = *lib.get::<*const c_void>(b"OfxSetHost\0").unwrap();
            let h = rv_ofx_host_new();
            let mut c = std::ptr::null_mut();
            assert_eq!(rv_ofx_catalog_open(h, gc, gp, sh, &raw mut c), 0);
            let d = rv_ofx_catalog_descriptor(c, 0);
            let mut e = std::ptr::null_mut();
            assert_eq!(rv_ofx_instance_new(d, &raw mut e), 0);
            assert_eq!(rv_ofx_instance_configure(e, 3, 2, 30.0), 0);
            assert_eq!(rv_ofx_param_set(e, 0, 0, 3.0, 0, 1), 0);
            assert_eq!(rv_ofx_instance_create(e), 0);
            for which in 0..5 {
                assert_eq!(probe(which), 0, "probe({which}) failed");
            }
            assert_eq!(rv_ofx_instance_destroy(e), 0);
            rv_ofx_instance_release(e);
            rv_ofx_catalog_close(c);
            rv_ofx_host_free(h);
        }
    }
}
