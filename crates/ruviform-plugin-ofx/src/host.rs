//! OFX host + library admission API over the private native catalog.

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use libloading::Library;
use ruviform_plugin_core::PluginDescriptorView;

use crate::error::OfxError;
use crate::ffi::sys;
use crate::instance::OfxInstance;

/// Per-plugin admission result for a rejected native entry.
///
/// Admitted index (the position in this library's admitted list) is NOT the
/// native plugin index — use [`PluginRejection::native_index`] for that.
#[derive(Debug)]
pub struct PluginRejection {
    native_index: usize,
    identifier: Option<String>,
    error: OfxError,
}

impl PluginRejection {
    pub(crate) fn new(native_index: usize, identifier: Option<String>, error: OfxError) -> Self {
        Self {
            native_index,
            identifier,
            error,
        }
    }

    /// Index of the rejected entry in the native plugin enumeration.
    #[must_use]
    pub fn native_index(&self) -> usize {
        self.native_index
    }

    /// Plugin identifier string, when the entry declared one.
    #[must_use]
    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }

    /// Why the entry was rejected.
    #[must_use]
    pub fn error(&self) -> &OfxError {
        &self.error
    }
}

/// One admitted descriptor's owned metadata + native token.
#[derive(Debug)]
pub(crate) struct PluginDesc {
    /// Opaque descriptor token (`rv_ofx_catalog_descriptor`).
    pub(crate) token: *mut std::ffi::c_void,
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) vendor: String,
    pub(crate) version: String,
}

/// Native host singleton — `rv_ofx_host_new` rejects a second session.
#[derive(Debug)]
pub(crate) struct NativeHost {
    raw: *mut std::ffi::c_void,
}

impl NativeHost {
    #[expect(unsafe_code)]
    fn new() -> Result<Rc<Self>, OfxError> {
        // SAFETY: `rv_ofx_host_new` returns NULL when a session is live; a
        // non-null value is the session's own host token, retired via Drop.
        let raw = unsafe { sys::rv_ofx_host_new() };
        if raw.is_null() {
            return Err(OfxError::Unsupported("a live host session already exists"));
        }
        Ok(Rc::new(Self { raw }))
    }

    pub(crate) fn raw(&self) -> *mut std::ffi::c_void {
        self.raw
    }
}

impl Drop for NativeHost {
    #[expect(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: `raw` is the session token `rv_ofx_host_new` returned for
        // this host; `rv_ofx_host_free` resolves and retires it exactly once
        // here — this is the only `rv_ofx_host_free` call site for the token.
        unsafe { sys::rv_ofx_host_free(self.raw) }
    }
}

/// One loaded native library admitted into this host.
///
/// Owns the catalog (whose `Unload` runs while the DLL is still mapped) plus
/// the `libloading` handle and a shared ref on the native host — a live
/// instance therefore keeps native code resident after the public library
/// and host handles are gone.
#[derive(Debug)]
pub(crate) struct NativeLib {
    catalog: *mut std::ffi::c_void,
    // Field order matters for drop: `catalog_close` (in `Drop::drop`) runs
    // while `_lib` is still mapped and `_host` still owns `g_active`; the
    // fields then drop in declaration order — dlclose, then host_free.
    _lib: Library,
    _host: Rc<NativeHost>,
    pub(crate) plugins: Vec<PluginDesc>,
    pub(crate) rejections: Vec<PluginRejection>,
}

impl NativeLib {
    pub(crate) fn new(
        catalog: *mut std::ffi::c_void,
        lib: Library,
        host: Rc<NativeHost>,
        plugins: Vec<PluginDesc>,
        rejections: Vec<PluginRejection>,
    ) -> Self {
        Self {
            catalog,
            _lib: lib,
            _host: host,
            plugins,
            rejections,
        }
    }
}

impl Drop for NativeLib {
    #[expect(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: `catalog` was produced by `rv_ofx_catalog_open`, is closed
        // exactly once here, and `_lib`/`_host` are still alive (they drop
        // after this method returns).
        unsafe { sys::rv_ofx_catalog_close(self.catalog) }
    }
}

/// One loaded native library admitted into this host.
///
/// `Rc`-based control plane — intentionally `!Send`/`!Sync`.
#[derive(Debug)]
pub struct OfxLibrary {
    inner: Rc<NativeLib>,
}

impl OfxLibrary {
    pub(crate) fn share(inner: Rc<NativeLib>) -> Self {
        Self { inner }
    }

    /// Number of admitted plugins in this library.
    #[must_use]
    pub fn plugin_count(&self) -> usize {
        self.inner.plugins.len()
    }

    /// Borrowed descriptor view of the admitted plugin at `admitted_index`.
    #[must_use]
    pub fn plugin(&self, admitted_index: usize) -> Option<PluginDescriptorView<'_>> {
        self.inner
            .plugins
            .get(admitted_index)
            .map(|p| PluginDescriptorView {
                id: &p.id,
                name: &p.name,
                vendor: &p.vendor,
                version: &p.version,
                features: &[],
            })
    }

    /// Create an instance of the admitted plugin at `admitted_index`.
    ///
    /// The returned wrapper starts in [`LifecycleState::Created`]; native
    /// `CreateInstance` runs on [`VideoPluginInstance::initialize`].
    ///
    /// [`LifecycleState::Created`]: ruviform_plugin_core::LifecycleState::Created
    /// [`VideoPluginInstance::initialize`]: ruviform_plugin_core::VideoPluginInstance::initialize
    ///
    /// # Errors
    ///
    /// [`OfxError::InvalidValue`] when `admitted_index` is out of range;
    /// [`OfxError::Native`]/[`OfxError::Memory`] when the attempt cannot be
    /// allocated.
    pub fn create_instance(&self, admitted_index: usize) -> Result<OfxInstance, OfxError> {
        let desc = self
            .inner
            .plugins
            .get(admitted_index)
            .ok_or(OfxError::InvalidValue("admitted index out of range"))?;
        OfxInstance::new(Rc::clone(&self.inner), desc.token)
    }

    /// Per-entry rejections recorded during admission.
    #[must_use]
    pub fn rejections(&self) -> &[PluginRejection] {
        &self.inner.rejections
    }
}

/// Host session — the single active admission owner.
///
/// `Rc`-based control plane: `!Send`, one creator thread.
#[derive(Debug)]
pub struct OfxHost {
    native: Rc<NativeHost>,
    libraries: HashMap<PathBuf, Weak<NativeLib>>,
}

impl OfxHost {
    /// Create the host session.
    ///
    /// The C bridge admits exactly one live host; a second concurrent
    /// `OfxHost` is rejected.
    ///
    /// # Errors
    ///
    /// [`OfxError::Unsupported`] when another host session is still alive.
    pub fn new() -> Result<Self, OfxError> {
        Ok(Self {
            native: NativeHost::new()?,
            libraries: HashMap::new(),
        })
    }

    pub(crate) fn native(&self) -> &Rc<NativeHost> {
        &self.native
    }

    pub(crate) fn libraries(&mut self) -> &mut HashMap<PathBuf, Weak<NativeLib>> {
        &mut self.libraries
    }

    // `load_library` is declared in `ffi::loader` — the private unsafe
    // boundary module (see `crate::ffi::loader`).
}
