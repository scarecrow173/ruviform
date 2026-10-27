//! Cross-window host-teardown contract (Issue #124).
//!
//! `rv_ofx_catalog_open` releases the global mutex around every foreign
//! call (`OfxSetHost`, `OfxGetNumberOfPlugins`, `OfxGetPlugin`, and each
//! admission `setHost`/`mainEntry` window). A callback on that window may
//! re-enter `rv_ofx_host_free`. The owning C layer must poison the host
//! immediately — alive flag dropped, every session token swept — while
//! deferring reclaim of the `RvHost` block until the open-call lease
//! drops, so every post-window access is defined. The API object handed
//! to foreign code lives in static storage, never inside the reclaimable
//! block, and `g_api.host` names a dedicated process-lifetime property
//! set that is bound once and never rebound or cleared (Issue #131), so
//! a value a plugin retains can never alias a later session's state. The
//! caller-visible host handle is itself a generation token. The open
//! then rejects with `kOfxStatErrBadHandle`, yields no catalog, a second
//! `rv_ofx_host_free` is a defined no-op, and a fresh `rv_ofx_host_new`
//! starts a clean session.
//!
//! These tests link the static bridge directly; the hostile callbacks run
//! on the same thread inside the released-mutex window, which exercises
//! the exact code path a foreign teardown would take.

use std::sync::Mutex;

/// The bridge keeps one global active host; ALL tests in this file
/// serialize on this single guard — both modules below mutate the same
/// process-global session state.
static GUARD: Mutex<()> = Mutex::new(());

#[expect(unsafe_code)]
mod t {
    use std::ffi::{CStr, c_char, c_void};
    use std::ptr;
    use std::sync::atomic::{AtomicPtr, Ordering};

    use crate::GUARD;

    const K_OFX_STAT_OK: i32 = 0;
    const K_OFX_STAT_ERR_UNKNOWN: i32 = 3;
    const K_OFX_STAT_ERR_MEMORY: i32 = 8;
    const K_OFX_STAT_ERR_BAD_HANDLE: i32 = 9;

    /// `OfxHost` layout mirror (vendor/openfx `ofxCore.h`).
    #[repr(C)]
    struct OfxHost {
        host: *mut c_void,
        fetch_suite: Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32) -> *const c_void>,
    }

    /// `OfxPropertySuiteV1` layout mirror (vendor/openfx `ofxProperty.h`)
    /// in SDK field order — only the slots a test drives are read, the
    /// rest exist to keep the ABI offsets correct.
    #[repr(C)]
    #[allow(dead_code)]
    // SDK field names all share the `prop` prefix by definition.
    #[allow(clippy::struct_field_names)]
    struct OfxPropertySuiteV1 {
        prop_set_pointer:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *mut c_void) -> i32>,
        prop_set_string:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *const c_char) -> i32>,
        prop_set_double: Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, f64) -> i32>,
        prop_set_int: Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, i32) -> i32>,
        prop_set_pointer_n: Option<
            unsafe extern "C" fn(*mut c_void, *const c_char, i32, *const *mut c_void) -> i32,
        >,
        prop_set_string_n: Option<
            unsafe extern "C" fn(*mut c_void, *const c_char, i32, *const *const c_char) -> i32,
        >,
        prop_set_double_n:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *const f64) -> i32>,
        prop_set_int_n:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *const i32) -> i32>,
        prop_get_pointer:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *mut *mut c_void) -> i32>,
        prop_get_string:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *mut *mut c_char) -> i32>,
        prop_get_double:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *mut f64) -> i32>,
        prop_get_int:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *mut i32) -> i32>,
        prop_get_pointer_n:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *mut *mut c_void) -> i32>,
        prop_get_string_n:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *mut *mut c_char) -> i32>,
        prop_get_double_n:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *mut f64) -> i32>,
        prop_get_int_n:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, i32, *mut i32) -> i32>,
        prop_reset: Option<unsafe extern "C" fn(*mut c_void, *const c_char) -> i32>,
        prop_get_dimension:
            Option<unsafe extern "C" fn(*mut c_void, *const c_char, *mut i32) -> i32>,
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
        fn rv_ofx_catalog_close(c: *mut c_void);
    }

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
            "api.host must name the dedicated host property set on entry"
        );
        assert_eq!(
            API_HOST_AFTER.load(Ordering::Relaxed),
            API_HOST_BEFORE.load(Ordering::Relaxed),
            "api.host is bound once and never rebound or poisoned (Issue #131)"
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
    /// Benign catalog-level `OfxSetHost` that only captures the static
    /// API object so a test can drive `g_api.host`'s property set through
    /// the real `PropertyV1` suite afterwards.
    extern "C" fn capture_api_set_host(api: *mut OfxHost) -> i32 {
        SEEN_API.store(api, Ordering::Relaxed);
        K_OFX_STAT_OK
    }

    /// Read `host` + `fetchSuite` off a captured `OfxHost*` and resolve
    /// the `PropertyV1` table. `api` is process-static storage and the
    /// returned suite pointer names the static `g_prop_suite`.
    unsafe fn api_host_prop_suite(api: *mut OfxHost) -> (*mut c_void, *const OfxPropertySuiteV1) {
        // SAFETY: `api` is the process-static `g_api` the bridge handed
        // the callback; its fields and the fetched suite table outlive
        // every session.
        unsafe {
            let host_ps = (*api).host;
            let fetch = (*api).fetch_suite.expect("api.fetchSuite must exist");
            let suite = fetch(host_ps, c"OfxPropertySuite".as_ptr(), 1);
            assert!(!suite.is_null(), "PropertyV1 must be fetchable");
            (host_ps, suite.cast::<OfxPropertySuiteV1>())
        }
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

    /// Issue #131 regression: `g_api.host`'s dedicated property set is
    /// process-lifetime, but every record a plugin writes pools its
    /// name/value strings on the *session* host. `rv_ofx_host_free` must
    /// retire the records with the session so a post-teardown suite call
    /// on the still-bound `api.host` can never read a reclaimed pool —
    /// and the next session must see a clean set. Without the retire,
    /// `pfind`/`propGetString` would strcmp/deref freed pool strings.
    #[test]
    fn host_propset_records_retire_with_session() {
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
                capture_api_set_host as extern "C" fn(*mut OfxHost) -> i32 as *const c_void,
                &raw mut c,
            );
            assert_eq!(rc, K_OFX_STAT_OK);
            assert!(
                !c.is_null(),
                "a catalog of only rejected entries still opens"
            );
            let api = SEEN_API.load(Ordering::Relaxed);
            assert!(!api.is_null(), "catalog setHost must deliver g_api");
            let (host_ps, suite) = api_host_prop_suite(api);
            assert!(!host_ps.is_null(), "api.host must name the host propset");
            let set_string = (*suite).prop_set_string.expect("propSetString");
            let get_string = (*suite).prop_get_string.expect("propGetString");
            let get_dim = (*suite).prop_get_dimension.expect("propGetDimension");
            let reset = (*suite).prop_reset.expect("propReset");
            let name = c"ruviform.test.hostprop".as_ptr();
            // session-1 write lands a record on the once-bound set, its
            // strings pooled on the session host
            assert_eq!(
                set_string(host_ps, name, 0, c"session-one".as_ptr()),
                K_OFX_STAT_OK
            );
            let mut dim = 0;
            assert_eq!(get_dim(host_ps, name, &raw mut dim), K_OFX_STAT_OK);
            assert_eq!(dim, 1);
            rv_ofx_catalog_close(c);
            rv_ofx_host_free(h);
            // post-teardown touch is defined: the records retired with
            // the session, so reads/reset miss on the empty set and a
            // write fails on the absent session pool
            assert_eq!(get_dim(host_ps, name, &raw mut dim), K_OFX_STAT_ERR_UNKNOWN);
            let mut s: *mut c_char = ptr::null_mut();
            assert_eq!(
                get_string(host_ps, name, 0, &raw mut s),
                K_OFX_STAT_ERR_UNKNOWN
            );
            assert_eq!(reset(host_ps, name), K_OFX_STAT_ERR_UNKNOWN);
            assert_eq!(
                set_string(host_ps, name, 0, c"dead".as_ptr()),
                K_OFX_STAT_ERR_MEMORY
            );
            // the next session rewrites fresh records on its own pool and
            // sees none of session one's state
            let h2 = rv_ofx_host_new();
            assert!(!h2.is_null(), "a new session must start after reclaim");
            assert_eq!(
                set_string(host_ps, name, 0, c"session-two".as_ptr()),
                K_OFX_STAT_OK
            );
            assert_eq!(get_dim(host_ps, name, &raw mut dim), K_OFX_STAT_OK);
            assert_eq!(dim, 1);
            let mut s2: *mut c_char = ptr::null_mut();
            assert_eq!(get_string(host_ps, name, 0, &raw mut s2), K_OFX_STAT_OK);
            assert_eq!(CStr::from_ptr(s2).to_bytes(), b"session-two");
            rv_ofx_host_free(h2);
            assert_eq!(get_dim(host_ps, name, &raw mut dim), K_OFX_STAT_ERR_UNKNOWN);
        }
    }
}

/// Foreign-window lease coverage for the non-catalog lifecycle paths
/// (Issue #131). These link the real fixture DLL: its observer fires
/// inside the plugin action, so the re-entrant `rv_ofx_host_free` lands
/// inside the bridge's released-mutex window — the exact path a hostile
/// plugin teardown takes. Every window now runs under a session lease,
/// so post-window bookkeeping reads a poisoned but still-allocated host
/// and the call rejects with `kOfxStatErrBadHandle` (or preserves the
/// action's own failure) instead of dereferencing a reclaimed pool.
#[expect(unsafe_code)]
mod window_lease {
    use std::ffi::c_void;
    use std::path::Path;
    use std::ptr;
    use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering};

    use libloading::Library;

    use crate::GUARD;

    const K_OFX_STAT_OK: i32 = 0;
    const K_OFX_STAT_FAILED: i32 = 1;
    const K_OFX_STAT_ERR_BAD_HANDLE: i32 = 9;

    // Fixture observer events (tests/fixture/fixture_control.h).
    const FX_UNLOAD: i32 = 5;
    const FX_CREATE_ENTER: i32 = 8;
    const FX_DESTROY: i32 = 10;
    const FX_CHANGED: i32 = 12;
    const FX_CLIP_PREFS: i32 = 14;
    const FX_RENDER: i32 = 16;

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
        fn rv_ofx_catalog_descriptor(catalog: *mut c_void, index: usize) -> *mut c_void;
        fn rv_ofx_instance_new(descriptor: *mut c_void, effect: *mut *mut c_void) -> i32;
        fn rv_ofx_instance_configure(
            effect: *mut c_void,
            width: u32,
            height: u32,
            frame_rate: f64,
        ) -> i32;
        fn rv_ofx_instance_create(effect: *mut c_void) -> i32;
        fn rv_ofx_instance_changed(effect: *mut c_void, param_index: usize) -> i32;
        fn rv_ofx_instance_preferences(effect: *mut c_void) -> i32;
        fn rv_ofx_instance_render(
            effect: *mut c_void,
            src: *const u8,
            src_stride: usize,
            dst: *mut u8,
            dst_stride: usize,
            time: f64,
        ) -> i32;
        fn rv_ofx_instance_destroy(effect: *mut c_void) -> i32;
        fn rv_ofx_instance_release(effect: *mut c_void);
    }

    /// Session token the hostile observer tears down mid-window.
    static VICTIM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    /// Catalog the hostile observer re-closes inside its own Unload window.
    static CCAT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    /// Observer fire gate and trigger event.
    static ARMED: AtomicBool = AtomicBool::new(false);
    static TARGET: AtomicI32 = AtomicI32::new(-1);
    /// When set, the observer also re-enters `rv_ofx_catalog_close` before
    /// freeing the host (nested-close guard coverage).
    static REENTER_CLOSE: AtomicBool = AtomicBool::new(false);

    extern "C" fn hostile_observer(event: i32, _index: i32) {
        if event != TARGET.load(Ordering::Relaxed) || !ARMED.swap(false, Ordering::Relaxed) {
            return;
        }
        if REENTER_CLOSE.load(Ordering::Relaxed) {
            // SAFETY: CCAT is the catalog whose close is currently
            // unwinding; a re-entrant close must degrade to a no-op.
            unsafe { rv_ofx_catalog_close(CCAT.load(Ordering::Relaxed)) };
        }
        // SAFETY: VICTIM is the live session token; freeing it inside the
        // foreign window is exactly the hostile teardown under test.
        unsafe { rv_ofx_host_free(VICTIM.load(Ordering::Relaxed)) };
    }

    struct Session {
        /// Held only to keep the fixture DLL mapped for the session.
        _lib: Library,
        host: *mut c_void,
        catalog: *mut c_void,
        effect: *mut c_void,
    }

    impl Session {
        /// Open a catalog on the fixture with the hostile observer armed
        /// for `target` (-1 = observe only). Returns with `effect` holding
        /// a configured instance attempt when `want_instance` is set.
        fn open(target: i32, want_instance: bool, reenter_close: bool) -> Self {
            let path = Path::new(env!("OFX_FIXTURE_DLL"));
            // SAFETY: fixture build output; signatures per fixture_control.h.
            let lib = unsafe { Library::new(path).expect("compiled fixture") };
            let configure = unsafe {
                lib.get::<unsafe extern "C" fn(i32, Option<extern "C" fn(i32, i32)>)>(
                    b"ofx_fixture_configure\0",
                )
                .expect("fixture configure")
            };
            let set_mode =
                unsafe { lib.get::<unsafe extern "C" fn(i32)>(b"ofx_fixture_set_mode\0") }
                    .expect("fixture set_mode");
            // configure refuses while a previous test left fixture
            // instances live; the observer stays installed either way and
            // the mode reset always applies.
            unsafe {
                configure(0, Some(hostile_observer));
                set_mode(0);
            }
            ARMED.store(false, Ordering::Relaxed);
            TARGET.store(target, Ordering::Relaxed);
            REENTER_CLOSE.store(reenter_close, Ordering::Relaxed);
            // SAFETY: opaque bridge calls over the live session.
            unsafe {
                let host = rv_ofx_host_new();
                assert!(!host.is_null());
                VICTIM.store(host, Ordering::Relaxed);
                let count = *lib
                    .get::<*const c_void>(b"OfxGetNumberOfPlugins\0")
                    .unwrap();
                let plugin = *lib.get::<*const c_void>(b"OfxGetPlugin\0").unwrap();
                let set_host = *lib.get::<*const c_void>(b"OfxSetHost\0").unwrap();
                let mut s = Session {
                    _lib: lib,
                    host,
                    catalog: ptr::null_mut(),
                    effect: ptr::null_mut(),
                };
                let rc = rv_ofx_catalog_open(s.host, count, plugin, set_host, &raw mut s.catalog);
                assert_eq!(rc, K_OFX_STAT_OK, "catalog open must succeed pre-window");
                assert!(!s.catalog.is_null());
                CCAT.store(s.catalog, Ordering::Relaxed);
                if want_instance {
                    let d = rv_ofx_catalog_descriptor(s.catalog, 0);
                    assert!(!d.is_null(), "gain descriptor admitted");
                    assert_eq!(rv_ofx_instance_new(d, &raw mut s.effect), K_OFX_STAT_OK);
                    assert!(!s.effect.is_null());
                    assert_eq!(
                        rv_ofx_instance_configure(s.effect, 4, 4, 30.0),
                        K_OFX_STAT_OK
                    );
                }
                s
            }
        }
    }

    /// After any hostile window: the session token is dead, a second free
    /// is a defined no-op, and a fresh session starts clean.
    unsafe fn assert_dead_session(h: *mut c_void) {
        unsafe {
            rv_ofx_host_free(h);
            let h2 = rv_ofx_host_new();
            assert!(!h2.is_null(), "a new session must start after reclaim");
            rv_ofx_host_free(h2);
        }
    }

    #[test]
    fn free_inside_create_window_rejects() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let s = Session::open(FX_CREATE_ENTER, true, false);
        ARMED.store(true, Ordering::Relaxed);
        unsafe {
            assert_eq!(rv_ofx_instance_create(s.effect), K_OFX_STAT_ERR_BAD_HANDLE);
            rv_ofx_instance_release(s.effect);
            rv_ofx_catalog_close(s.catalog);
            assert_dead_session(s.host);
        }
    }

    #[test]
    fn free_inside_changed_window_rejects() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let s = Session::open(-1, true, false);
        unsafe {
            assert_eq!(rv_ofx_instance_create(s.effect), K_OFX_STAT_OK);
        }
        TARGET.store(FX_CHANGED, Ordering::Relaxed);
        ARMED.store(true, Ordering::Relaxed);
        unsafe {
            assert_eq!(
                rv_ofx_instance_changed(s.effect, 0),
                K_OFX_STAT_ERR_BAD_HANDLE
            );
            rv_ofx_instance_release(s.effect);
            rv_ofx_catalog_close(s.catalog);
            assert_dead_session(s.host);
        }
    }

    #[test]
    fn free_inside_clip_prefs_window_rejects() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let s = Session::open(-1, true, false);
        unsafe {
            assert_eq!(rv_ofx_instance_create(s.effect), K_OFX_STAT_OK);
        }
        TARGET.store(FX_CLIP_PREFS, Ordering::Relaxed);
        ARMED.store(true, Ordering::Relaxed);
        unsafe {
            assert_eq!(
                rv_ofx_instance_preferences(s.effect),
                K_OFX_STAT_ERR_BAD_HANDLE
            );
            rv_ofx_instance_release(s.effect);
            rv_ofx_catalog_close(s.catalog);
            assert_dead_session(s.host);
        }
    }

    #[test]
    fn free_inside_render_window_defined() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let s = Session::open(-1, true, false);
        let src = [0x40u8; 64];
        let mut dst = [0xEEu8; 64];
        unsafe {
            assert_eq!(rv_ofx_instance_create(s.effect), K_OFX_STAT_OK);
        }
        TARGET.store(FX_RENDER, Ordering::Relaxed);
        ARMED.store(true, Ordering::Relaxed);
        unsafe {
            let rc = rv_ofx_instance_render(s.effect, src.as_ptr(), 16, dst.as_mut_ptr(), 16, 0.0);
            assert_eq!(
                rc, K_OFX_STAT_FAILED,
                "dead-session render preserves the action's own failure \
                 (its in-window suite calls hit swept tokens)"
            );
            assert_eq!(
                dst, [0xEEu8; 64],
                "untrusted staged pixels are not committed"
            );
            rv_ofx_instance_release(s.effect);
            rv_ofx_catalog_close(s.catalog);
            assert_dead_session(s.host);
        }
    }

    #[test]
    fn free_inside_destroy_window_defined() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let s = Session::open(-1, true, false);
        unsafe {
            assert_eq!(rv_ofx_instance_create(s.effect), K_OFX_STAT_OK);
        }
        TARGET.store(FX_DESTROY, Ordering::Relaxed);
        ARMED.store(true, Ordering::Relaxed);
        unsafe {
            let rc = rv_ofx_instance_destroy(s.effect);
            assert_eq!(
                rc, K_OFX_STAT_FAILED,
                "dead-session destroy preserves the action's own failure \
                 (its in-window suite calls hit swept tokens)"
            );
            rv_ofx_instance_release(s.effect);
            rv_ofx_catalog_close(s.catalog);
            assert_dead_session(s.host);
        }
    }

    /// Same Destroy window, reached through the release-drop path:
    /// `rv_ofx_instance_release` runs `DestroyInstance` itself when the
    /// attempt was created, and the hostile free landing inside that
    /// window must still leave release/close/session-teardown defined.
    #[test]
    fn free_inside_release_destroy_window_defined() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let s = Session::open(-1, true, false);
        unsafe {
            assert_eq!(rv_ofx_instance_create(s.effect), K_OFX_STAT_OK);
        }
        TARGET.store(FX_DESTROY, Ordering::Relaxed);
        ARMED.store(true, Ordering::Relaxed);
        unsafe {
            rv_ofx_instance_release(s.effect);
            rv_ofx_catalog_close(s.catalog);
            assert_dead_session(s.host);
        }
    }

    #[test]
    fn close_inside_unload_window_reenters_safely() {
        let _g = GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Armed for FX_UNLOAD with re-entrant close + host free: the outer
        // close must finish defined, the inner close must no-op via the
        // closing guard, and the window lease defers reclaim past both.
        let s = Session::open(FX_UNLOAD, false, true);
        ARMED.store(true, Ordering::Relaxed);
        unsafe {
            rv_ofx_catalog_close(s.catalog);
            assert_dead_session(s.host);
        }
    }
}
