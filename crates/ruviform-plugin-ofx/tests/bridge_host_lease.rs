//! Cross-window host-teardown contract (Issue #124).
//!
//! `rv_ofx_catalog_open` releases the global mutex around every foreign
//! call (`OfxSetHost`, `OfxGetNumberOfPlugins`, `OfxGetPlugin`, and each
//! admission `setHost`/`mainEntry` window). A callback on that window may
//! re-enter `rv_ofx_host_free`. The owning C layer must poison the host
//! immediately — `g_api.host` cleared, alive flag dropped — while
//! deferring reclaim of the `RvHost` block until the open-call lease
//! drops, so every post-window access is defined. The API object handed
//! to foreign code lives in static storage, never inside the reclaimable
//! block. The open then rejects with `kOfxStatErrBadHandle`, yields no
//! catalog, a second `rv_ofx_host_free` is a defined no-op, and a fresh
//! `rv_ofx_host_new` starts a clean session.
//!
//! These tests link the static bridge directly; the hostile callbacks run
//! on the same thread inside the released-mutex window, which exercises
//! the exact code path a foreign teardown would take.

#[expect(unsafe_code)]
mod t {
    use std::ffi::{c_char, c_void};
    use std::ptr;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicPtr, Ordering};

    const K_OFX_STAT_OK: i32 = 0;
    const K_OFX_STAT_ERR_BAD_HANDLE: i32 = 9;

    /// `OfxHost` layout mirror (vendor/openfx `ofxCore.h`).
    #[repr(C)]
    struct OfxHost {
        host: *mut c_void,
        fetch_suite: Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32) -> *const c_void>,
    }

    /// `OfxPlugin` layout mirror (vendor/openfx `ofxCore.h`).
    #[repr(C)]
    struct OfxPlugin {
        plugin_api: *const c_char,
        api_version: i32,
        plugin_identifier: *const c_char,
        plugin_version_major: u32,
        plugin_version_minor: u32,
        set_host: Option<unsafe extern "C" fn(*mut OfxHost)>,
        main_entry: Option<
            unsafe extern "C" fn(*const c_char, *const c_void, *mut c_void, *mut c_void) -> i32,
        >,
    }

    // SAFETY: the mirror is immutable FFI descriptor data shared with the
    // C bridge by pointer only; it carries no interior mutability.
    unsafe impl Sync for OfxPlugin {}

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
    }

    /// The bridge keeps one global active host; tests serialize on this.
    static GUARD: Mutex<()> = Mutex::new(());

    /// Host pointer the hostile callback tears down mid-window.
    static VICTIM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    /// `OfxHost*` observed inside the window, plus its `host`/`fetchSuite`
    /// fields before and after the re-entrant `rv_ofx_host_free`.
    static SEEN_API: AtomicPtr<OfxHost> = AtomicPtr::new(ptr::null_mut());
    static API_HOST_BEFORE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static API_HOST_AFTER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static API_FETCH_AFTER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

    fn reset_observations(h: *mut c_void) {
        VICTIM.store(h, Ordering::Relaxed);
        SEEN_API.store(ptr::null_mut(), Ordering::Relaxed);
        API_HOST_BEFORE.store(ptr::null_mut(), Ordering::Relaxed);
        API_HOST_AFTER.store(ptr::null_mut(), Ordering::Relaxed);
        API_FETCH_AFTER.store(ptr::null_mut(), Ordering::Relaxed);
    }

    /// Tear the host down inside the window and record what the static API
    /// object looks like across the re-entrant free. `api` must stay a
    /// valid dereference for the whole call: it is static storage.
    unsafe fn poison_and_observe(api: *mut OfxHost) {
        // SAFETY: `api` is the host struct the bridge handed across the
        // window; the contract under test requires it to outlive the host
        // block, so both reads here are defined.
        unsafe {
            SEEN_API.store(api, Ordering::Relaxed);
            API_HOST_BEFORE.store((*api).host, Ordering::Relaxed);
            rv_ofx_host_free(VICTIM.load(Ordering::Relaxed));
            API_HOST_AFTER.store((*api).host, Ordering::Relaxed);
            API_FETCH_AFTER.store(
                (*api)
                    .fetch_suite
                    .map_or(ptr::null_mut(), |f| f as *mut c_void),
                Ordering::Relaxed,
            );
        }
    }

    /// Assert the full post-teardown contract after `catalog_open` rejected.
    unsafe fn assert_dead_open_outcome(h: *mut c_void, catalog: *mut c_void) {
        assert!(catalog.is_null(), "dead host must yield no catalog");
        assert!(
            !SEEN_API.load(Ordering::Relaxed).is_null(),
            "foreign code must receive an OfxHost*"
        );
        assert!(
            !API_HOST_BEFORE.load(Ordering::Relaxed).is_null(),
            "api.host must point at the live host property set on entry"
        );
        assert!(
            API_HOST_AFTER.load(Ordering::Relaxed).is_null(),
            "api.host must be poisoned to NULL the instant the host dies"
        );
        assert!(
            !API_FETCH_AFTER.load(Ordering::Relaxed).is_null(),
            "api.fetchSuite stays valid on the static API object"
        );
        // The host was reclaimed when the window lease dropped; a second
        // free on the stale pointer is a defined no-op, and a new session
        // starts cleanly.
        unsafe {
            rv_ofx_host_free(h);
            let h2 = rv_ofx_host_new();
            assert!(!h2.is_null(), "a new session must start after reclaim");
            rv_ofx_host_free(h2);
        }
    }

    /* ---- shared benign callbacks ---- */

    extern "C" fn get_count_one() -> i32 {
        1
    }
    extern "C" fn benign_dll_set_host(_api: *mut OfxHost) -> i32 {
        K_OFX_STAT_OK
    }
    unsafe extern "C" fn get_plugin_null(_i: i32) -> *mut c_void {
        ptr::null_mut()
    }

    /// Hostile `OfxSetHost`: tears the host down inside the catalog-level
    /// `sh(&g_api)` window.
    unsafe extern "C" fn hostile_dll_set_host(api: *mut OfxHost) -> i32 {
        unsafe {
            poison_and_observe(api);
        }
        K_OFX_STAT_OK
    }

    /// Hostile `OfxGetNumberOfPlugins`: tears down inside the `gc()` window.
    unsafe extern "C" fn hostile_get_count() -> i32 {
        unsafe {
            rv_ofx_host_free(VICTIM.load(Ordering::Relaxed));
        }
        1
    }

    /// Hostile `OfxGetPlugin`: tears down inside the `gp(i)` window.
    unsafe extern "C" fn hostile_get_plugin(_i: i32) -> *mut c_void {
        unsafe {
            rv_ofx_host_free(VICTIM.load(Ordering::Relaxed));
        }
        ptr::null_mut()
    }

    /* ---- hostile plugin for the admission setHost/Load window ---- */

    /// Plugin `setHost`: tears the host down inside `entry_admit`'s
    /// `raw->setHost(&g_api)` window, before `kOfxActionLoad` runs.
    unsafe extern "C" fn hostile_plugin_set_host(api: *mut OfxHost) {
        unsafe {
            poison_and_observe(api);
        }
    }
    unsafe extern "C" fn hostile_main_entry(
        _action: *const c_char,
        _handle: *const c_void,
        _in_args: *mut c_void,
        _out_args: *mut c_void,
    ) -> i32 {
        K_OFX_STAT_OK
    }
    static HOSTILE_PLUGIN: OfxPlugin = OfxPlugin {
        plugin_api: c"OfxImageEffectPluginAPI".as_ptr(),
        api_version: 1,
        plugin_identifier: c"org.ruviform.hostile".as_ptr(),
        plugin_version_major: 0,
        plugin_version_minor: 1,
        set_host: Some(hostile_plugin_set_host),
        main_entry: Some(hostile_main_entry),
    };
    unsafe extern "C" fn get_hostile_plugin(_i: i32) -> *mut c_void {
        &raw const HOSTILE_PLUGIN as *mut c_void
    }

    /* ---- the four windows ---- */

    #[test]
    fn free_inside_dll_set_host_window_rejects() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            let h = rv_ofx_host_new();
            assert!(!h.is_null());
            reset_observations(h);
            let mut c = ptr::null_mut();
            let rc = rv_ofx_catalog_open(
                h,
                get_count_one as extern "C" fn() -> i32 as *const c_void,
                get_plugin_null as unsafe extern "C" fn(i32) -> *mut c_void as *const c_void,
                hostile_dll_set_host as unsafe extern "C" fn(*mut OfxHost) -> i32 as *const c_void,
                &raw mut c,
            );
            assert_eq!(rc, K_OFX_STAT_ERR_BAD_HANDLE);
            assert_dead_open_outcome(h, c);
        }
    }

    #[test]
    fn free_inside_get_count_window_rejects() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            let h = rv_ofx_host_new();
            assert!(!h.is_null());
            reset_observations(h);
            let mut c = ptr::null_mut();
            let rc = rv_ofx_catalog_open(
                h,
                hostile_get_count as unsafe extern "C" fn() -> i32 as *const c_void,
                get_plugin_null as unsafe extern "C" fn(i32) -> *mut c_void as *const c_void,
                benign_dll_set_host as extern "C" fn(*mut OfxHost) -> i32 as *const c_void,
                &raw mut c,
            );
            assert_eq!(rc, K_OFX_STAT_ERR_BAD_HANDLE);
            // The gc window never saw the api object; just the outcome.
            assert!(c.is_null(), "dead host must yield no catalog");
            rv_ofx_host_free(h);
            let h2 = rv_ofx_host_new();
            assert!(!h2.is_null(), "a new session must start after reclaim");
            rv_ofx_host_free(h2);
        }
    }

    #[test]
    fn free_inside_get_plugin_window_rejects() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            let h = rv_ofx_host_new();
            assert!(!h.is_null());
            reset_observations(h);
            let mut c = ptr::null_mut();
            let rc = rv_ofx_catalog_open(
                h,
                get_count_one as extern "C" fn() -> i32 as *const c_void,
                hostile_get_plugin as unsafe extern "C" fn(i32) -> *mut c_void as *const c_void,
                benign_dll_set_host as extern "C" fn(*mut OfxHost) -> i32 as *const c_void,
                &raw mut c,
            );
            assert_eq!(rc, K_OFX_STAT_ERR_BAD_HANDLE);
            assert!(c.is_null(), "dead host must yield no catalog");
            rv_ofx_host_free(h);
            let h2 = rv_ofx_host_new();
            assert!(!h2.is_null(), "a new session must start after reclaim");
            rv_ofx_host_free(h2);
        }
    }

    #[test]
    fn free_inside_admission_set_host_window_rejects() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            let h = rv_ofx_host_new();
            assert!(!h.is_null());
            reset_observations(h);
            let mut c = ptr::null_mut();
            let rc = rv_ofx_catalog_open(
                h,
                get_count_one as extern "C" fn() -> i32 as *const c_void,
                get_hostile_plugin as unsafe extern "C" fn(i32) -> *mut c_void as *const c_void,
                benign_dll_set_host as extern "C" fn(*mut OfxHost) -> i32 as *const c_void,
                &raw mut c,
            );
            assert_eq!(rc, K_OFX_STAT_ERR_BAD_HANDLE);
            assert_dead_open_outcome(h, c);
        }
    }
}
