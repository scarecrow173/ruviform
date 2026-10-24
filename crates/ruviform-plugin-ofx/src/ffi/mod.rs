//! Private FFI boundary — the only place `unsafe` is permitted.
//!
//! The C bridge (`ofx_bridge.c`) owns SDK layouts, suite tables, variadic
//! host callbacks, `OfxHost`, and typed native-action wrappers; Rust calls
//! fixed-arity primitive/pointer helpers declared in `ofx_bridge.h` only.

pub(crate) mod loader;

/// OFX status codes shared with the C bridge (`kOfxStat*`).
pub(crate) mod status {
    pub(crate) const OK: i32 = 0;
    pub(crate) const ERR_UNSUPPORTED: i32 = 5;
    pub(crate) const ERR_MEMORY: i32 = 8;
    pub(crate) const ERR_BAD_HANDLE: i32 = 9;
    pub(crate) const ERR_VALUE: i32 = 11;
}

/// Raw bridge ABI — mirrors `ofx_bridge.h`. All opaque handles except the
/// host/catalog pointers are generation tokens; nothing here dereferences
/// an SDK layout.
#[expect(unsafe_code)]
pub(crate) mod sys {
    use std::ffi::{c_char, c_void};

    unsafe extern "C" {
        pub(crate) fn rv_ofx_host_new() -> *mut c_void;
        pub(crate) fn rv_ofx_host_free(host: *mut c_void);
        pub(crate) fn rv_ofx_catalog_open(
            host: *mut c_void,
            get_count: *const c_void,
            get_plugin: *const c_void,
            set_host: *const c_void,
            catalog: *mut *mut c_void,
        ) -> i32;
        pub(crate) fn rv_ofx_catalog_close(catalog: *mut c_void);
        pub(crate) fn rv_ofx_catalog_count(catalog: *mut c_void) -> usize;
        pub(crate) fn rv_ofx_catalog_descriptor(catalog: *mut c_void, index: usize) -> *mut c_void;
        pub(crate) fn rv_ofx_catalog_identifier(
            catalog: *mut c_void,
            index: usize,
        ) -> *const c_char;
        pub(crate) fn rv_ofx_catalog_rejection(catalog: *mut c_void, index: usize) -> i32;
        pub(crate) fn rv_ofx_descriptor_string(
            descriptor: *mut c_void,
            field: i32,
        ) -> *const c_char;
        pub(crate) fn rv_ofx_param_count(effect: *mut c_void) -> usize;
        pub(crate) fn rv_ofx_param_info(
            effect: *mut c_void,
            index: usize,
            kind: *mut i32,
            name: *mut *const c_char,
            unit: *mut *const c_char,
            min_value: *mut f64,
            max_value: *mut f64,
            display_min: *mut f64,
            display_max: *mut f64,
            default_double: *mut f64,
            default_int: *mut i32,
            flags: *mut u32,
            choice_count: *mut usize,
        ) -> i32;
        pub(crate) fn rv_ofx_param_choice(
            effect: *mut c_void,
            index: usize,
            choice: usize,
        ) -> *const c_char;
        pub(crate) fn rv_ofx_param_get(
            effect: *mut c_void,
            index: usize,
            value_double: *mut f64,
            value_int: *mut i32,
        ) -> i32;
        pub(crate) fn rv_ofx_param_set(
            effect: *mut c_void,
            index: usize,
            kind: i32,
            double: f64,
            integer: i32,
            host_write: i32,
        ) -> i32;
        pub(crate) fn rv_ofx_instance_new(descriptor: *mut c_void, effect: *mut *mut c_void)
        -> i32;
        pub(crate) fn rv_ofx_instance_configure(
            effect: *mut c_void,
            width: u32,
            height: u32,
            frame_rate: f64,
        ) -> i32;
        pub(crate) fn rv_ofx_instance_create(effect: *mut c_void) -> i32;
        pub(crate) fn rv_ofx_instance_changed(effect: *mut c_void, index: usize) -> i32;
        pub(crate) fn rv_ofx_instance_preferences(effect: *mut c_void) -> i32;
        pub(crate) fn rv_ofx_instance_render(
            effect: *mut c_void,
            src: *const u8,
            src_stride: usize,
            dst: *mut u8,
            dst_stride: usize,
            time: f64,
        ) -> i32;
        pub(crate) fn rv_ofx_instance_destroy(effect: *mut c_void) -> i32;
        pub(crate) fn rv_ofx_instance_release(effect: *mut c_void);
    }
}
