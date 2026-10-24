//! Shared-library loading — the only point where dynamic OFX modules are
//! opened and passed to the C bridge.
//!
//! A path is canonicalized before it hits the cache so alias spellings of
//! the same binary share one admitted catalog instead of running the
//! native Load/Describe sequence twice.

use std::ffi::c_void;
use std::path::Path;
use std::rc::Rc;

use libloading::Library;

use crate::error::OfxError;
use crate::ffi::sys;
use crate::host::{NativeLib, OfxHost, OfxLibrary, PluginDesc, PluginRejection};

/// A required exported entry point read as a raw symbol address.
///
/// # Safety
///
/// The exported symbol is passed to the C bridge which calls it under the
/// OFX ABI contract; the `libloading` handle keeps the module mapped for
/// the catalog's lifetime.
#[expect(unsafe_code)]
unsafe fn entry(lib: &Library, name: &'static [u8]) -> Result<*const c_void, OfxError> {
    unsafe { lib.get::<*const c_void>(name) }.map(|s| *s).map_err(|_| {
        OfxError::Library(format!(
            "missing OFX export `{}`",
            String::from_utf8_lossy(name)
        ))
    })
}

/// A `*const c_char` from the bridge copied out to an owned `String`.
///
/// # Safety
///
/// The bridge returns NUL-terminated strings owned by the catalog; they are
/// valid for the catalog's lifetime and are copied before the call returns.
#[expect(unsafe_code)]
unsafe fn copy_cstr(raw: *const std::ffi::c_char) -> Option<String> {
    if raw.is_null() {
        return None;
    }
    Some(unsafe { std::ffi::CStr::from_ptr(raw) }.to_string_lossy().into_owned())
}

fn rejection_error(status: i32) -> OfxError {
    match status {
        k if k == crate::ffi::status::ERR_UNSUPPORTED => {
            OfxError::Unsupported("unsupported context, API, or parameter kind")
        }
        k if k == crate::ffi::status::ERR_MEMORY => OfxError::Memory,
        k if k == crate::ffi::status::ERR_BAD_HANDLE => OfxError::BadHandle,
        k if k == crate::ffi::status::ERR_VALUE => {
            OfxError::InvalidValue("invalid descriptor value")
        }
        s => OfxError::Native { action: "admission", status: s },
    }
}

impl OfxHost {
    /// Load and admit an OFX plugin binary.
    ///
    /// The canonical path is cached: loading an alias of an already-loaded
    /// library re-uses the admitted catalog without re-entering native
    /// `Load`/`Describe`/`DescribeInContext`.
    ///
    /// # Errors
    ///
    /// [`OfxError::Library`] when the module cannot be loaded or does not
    /// export `OfxGetNumberOfPlugins`/`OfxGetPlugin`;
    /// [`OfxError::Native`] when library admission fails.
    ///
    /// # Safety
    ///
    /// The loaded native code is arbitrary plugin code; callers must
    /// guarantee the path points at a compatible OFX binary. The caller
    /// must not hold borrowed parameter/descriptor views across this call.
    #[expect(unsafe_code)]
    pub unsafe fn load_library(&mut self, path: &Path) -> Result<OfxLibrary, OfxError> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Some(inner) = self
            .libraries()
            .get(&canonical)
            .and_then(std::rc::Weak::upgrade)
        {
            return Ok(OfxLibrary::share(inner));
        }

        // SAFETY: loading arbitrary plugin code is the caller's contract;
        // entry points are exercised only through the C bridge afterwards.
        let lib = unsafe { Library::new(path) }
            .map_err(|e| OfxError::Library(format!("load failed: {e}")))?;
        let get_count = unsafe { entry(&lib, b"OfxGetNumberOfPlugins\0") }?;
        let get_plugin = unsafe { entry(&lib, b"OfxGetPlugin\0") }?;
        let set_host =
            unsafe { entry(&lib, b"OfxSetHost\0") }.unwrap_or(std::ptr::null());

        let mut catalog = std::ptr::null_mut();
        // SAFETY: all pointers are valid — `host` is a live RvHost,
        // entry points are real symbols, `catalog` is an out-slot.
        let rc = unsafe {
            sys::rv_ofx_catalog_open(
                self.native().raw(),
                get_count,
                get_plugin,
                set_host,
                &raw mut catalog,
            )
        };
        if rc != 0 || catalog.is_null() {
            // SAFETY: a partially admitted catalog is valid to close — it
            // unloads whatever reached Load and frees its entries.
            if !catalog.is_null() {
                unsafe { sys::rv_ofx_catalog_close(catalog) }
            }
            return Err(if rc == crate::ffi::status::ERR_MEMORY {
                OfxError::Memory
            } else {
                OfxError::Native { action: "catalog admission", status: rc }
            });
        }

        // SAFETY: `catalog` is a live catalog; indices are bounded by
        // `rv_ofx_catalog_count` and returned strings are copied out.
        let (plugins, rejections) = unsafe {
            let count = sys::rv_ofx_catalog_count(catalog);
            let mut plugins = Vec::new();
            let mut rejections = Vec::new();
            for i in 0..count {
                let desc = sys::rv_ofx_catalog_descriptor(catalog, i);
                if desc.is_null() {
                    let identifier =
                        copy_cstr(sys::rv_ofx_catalog_identifier(catalog, i));
                    let status = sys::rv_ofx_catalog_rejection(catalog, i);
                    rejections.push(PluginRejection::new(
                        i,
                        identifier,
                        rejection_error(status),
                    ));
                } else {
                    plugins.push(PluginDesc {
                        token: desc,
                        id: copy_cstr(sys::rv_ofx_descriptor_string(desc, 0))
                            .unwrap_or_default(),
                        name: copy_cstr(sys::rv_ofx_descriptor_string(desc, 1))
                            .unwrap_or_default(),
                        vendor: copy_cstr(sys::rv_ofx_descriptor_string(desc, 2))
                            .unwrap_or_default(),
                        version: copy_cstr(sys::rv_ofx_descriptor_string(desc, 3))
                            .unwrap_or_default(),
                    });
                }
            }
            (plugins, rejections)
        };

        let inner = Rc::new(NativeLib::new(
            catalog,
            lib,
            Rc::clone(self.native()),
            plugins,
            rejections,
        ));
        self.libraries().insert(canonical, Rc::downgrade(&inner));
        Ok(OfxLibrary::share(inner))
    }
}
