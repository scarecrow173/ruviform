//! `VideoPluginInstance` — one OFX effect instance.
//!
//! The wrapper owns an opaque C-side attempt token; parameter metadata and
//! strings are copied out at construction so the borrowed views handed to
//! the generic UI never point into the bridge registries.

use std::ffi::{CStr, c_void};
use std::rc::Rc;

use ruviform_media_core::MediaTime;
use ruviform_plugin_core::{
    Lifecycle, LifecycleEvent, LifecycleState, ParamFlags, ParamId, PlainParamInfo,
    PlainParamValue, PluginDescriptorView, VideoPluginInstance, VideoProcessConfig,
};
use ruviform_video_core::{PixelFormat, VideoFrame};

use crate::error::OfxError;
use crate::ffi::{status, sys};
use crate::host::NativeLib;

/// Param kinds mirrored from the C bridge (`PK_*` in `ofx_params.c`).
mod pk {
    pub(crate) const DOUBLE: i32 = 0;
    pub(crate) const INTEGER: i32 = 1;
    pub(crate) const BOOLEAN: i32 = 2;
    pub(crate) const CHOICE: i32 = 3;
}

/// Flags bits the bridge reports — they alias `ParamFlags` bit positions
/// (`1<<2` = stepped/choice, `1<<3` = hidden/secret, `1<<4` = readonly).
fn param_flags(raw: u32) -> ParamFlags {
    let mut flags = ParamFlags::empty();
    if raw & (1 << 4) != 0 {
        flags = flags.union(ParamFlags::READONLY);
    }
    if raw & (1 << 3) != 0 {
        flags = flags.union(ParamFlags::HIDDEN);
    }
    if raw & (1 << 2) != 0 {
        flags = flags.union(ParamFlags::STEPPED);
    }
    flags
}

/// Native status → `OfxError` for the action calls below.
fn map_status(action: &'static str, status: i32) -> OfxError {
    match status {
        s if s == status::ERR_BAD_HANDLE => OfxError::BadHandle,
        s if s == status::ERR_VALUE => OfxError::InvalidValue("invalid value"),
        s if s == status::ERR_MEMORY => OfxError::Memory,
        s if s == status::ERR_UNSUPPORTED => OfxError::Unsupported("unsupported feature"),
        s => OfxError::Native { action, status: s },
    }
}

/// A `*const c_char` from the bridge copied out to an owned `String`.
///
/// # Safety
///
/// The bridge returns NUL-terminated strings owned by the registry; they
/// are valid while the owning effect token is alive — copied out before
/// the token can be revoked.
#[expect(unsafe_code)]
unsafe fn copy_cstr(raw: *const std::ffi::c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned()
}

/// Owned snapshot of one parameter's metadata + kind, kept beside the
/// `ParamId` so `parameter()` can hand out a borrowed [`PlainParamInfo`].
#[derive(Debug)]
struct ParamMeta {
    name: String,
    unit: String,
    kind: i32,
    min_value: f64,
    max_value: f64,
    display_min_value: f64,
    display_max_value: f64,
    default_value: PlainParamValue,
    flags: ParamFlags,
    choices: Vec<String>,
}

/// Owned descriptor strings, handed out as a borrowed view on demand.
#[derive(Debug)]
struct DescMeta {
    id: String,
    name: String,
    vendor: String,
    version: String,
}

/// One OFX effect instance.
///
/// Non-RT control plane: `!Send`/`!Sync` (via `Rc`), one owner thread.
/// The neutral contract (`VideoPluginInstance`) is non-RT — parameter and
/// lifecycle calls are serialized through the C bridge's recursive lock.
#[derive(Debug)]
pub struct OfxInstance {
    /// Opaque attempt token — `(void *)(uintptr_t)i->tok`.
    effect: *mut c_void,
    /// Keep-alive share of the owning library — outlives this instance so
    /// the catalog's `Unload` and dlclose only run after release.
    _lib: Rc<NativeLib>,
    lifecycle: Lifecycle,
    /// Whether the native `CreateInstance` succeeded — staged vs live
    /// write-through differ around this point.
    created: bool,
    /// Configured frame size — `render` frames must match it.
    dims: Option<(u32, u32)>,
    desc: DescMeta,
    ids: Vec<ParamId>,
    metas: Vec<ParamMeta>,
}

impl OfxInstance {
    /// Allocate a native attempt for descriptor `desc` in `Created` state.
    ///
    /// Reads all descriptor/param metadata up-front so `parameter()` and
    /// `descriptor()` stay usable after `DestroyInstance`.
    ///
    /// # Errors
    ///
    /// [`OfxError::BadHandle`] when `desc` is not a live descriptor token;
    /// [`OfxError::Memory`]/[`OfxError::Native`] on allocation failure.
    #[expect(unsafe_code)]
    pub(crate) fn new(lib: Rc<NativeLib>, desc: *mut c_void) -> Result<Self, OfxError> {
        let mut effect = std::ptr::null_mut();
        // SAFETY: `desc` is a live descriptor token from the catalog; `effect`
        // is an out-slot filled on success.
        let rc = unsafe { sys::rv_ofx_instance_new(desc, &raw mut effect) };
        if rc != 0 || effect.is_null() {
            return Err(map_status("instance_new", rc));
        }
        // SAFETY: `desc`/`effect` are live tokens; out-params are valid
        // locals and returned strings are copied before release.
        let (desc_meta, metas) = unsafe {
            let desc_meta = DescMeta {
                id: copy_cstr(sys::rv_ofx_descriptor_string(desc, 0)),
                name: copy_cstr(sys::rv_ofx_descriptor_string(desc, 1)),
                vendor: copy_cstr(sys::rv_ofx_descriptor_string(desc, 2)),
                version: copy_cstr(sys::rv_ofx_descriptor_string(desc, 3)),
            };
            let n = sys::rv_ofx_param_count(effect);
            let mut metas = Vec::with_capacity(n);
            for i in 0..n {
                let mut kind = 0;
                let mut name = std::ptr::null();
                let mut unit = std::ptr::null();
                let mut min = 0.0;
                let mut max = 0.0;
                let mut dmin = 0.0;
                let mut dmax = 0.0;
                let mut ddef = 0.0;
                let mut idef = 0;
                let mut raw_flags = 0u32;
                let mut nchoices = 0usize;
                let rc = sys::rv_ofx_param_info(
                    effect,
                    i,
                    &raw mut kind,
                    &raw mut name,
                    &raw mut unit,
                    &raw mut min,
                    &raw mut max,
                    &raw mut dmin,
                    &raw mut dmax,
                    &raw mut ddef,
                    &raw mut idef,
                    &raw mut raw_flags,
                    &raw mut nchoices,
                );
                if rc != 0 {
                    sys::rv_ofx_instance_release(effect);
                    return Err(map_status("param_info", rc));
                }
                let default_value = match kind {
                    pk::DOUBLE => PlainParamValue::Double(ddef),
                    pk::BOOLEAN => PlainParamValue::Boolean(idef != 0),
                    pk::CHOICE => PlainParamValue::Choice(idef.cast_unsigned()),
                    _ => PlainParamValue::Integer(idef),
                };
                let mut choices = Vec::with_capacity(nchoices);
                for c in 0..nchoices {
                    choices.push(copy_cstr(sys::rv_ofx_param_choice(effect, i, c)));
                }
                metas.push(ParamMeta {
                    name: copy_cstr(name),
                    unit: copy_cstr(unit),
                    kind,
                    min_value: min,
                    max_value: max,
                    display_min_value: dmin,
                    display_max_value: dmax,
                    default_value,
                    flags: param_flags(raw_flags),
                    choices,
                });
            }
            (desc_meta, metas)
        };
        let ids = (0..metas.len())
            .map(|i| {
                u32::try_from(i)
                    .map(ParamId)
                    .map_err(|_| OfxError::InvalidValue("parameter index overflow"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            effect,
            _lib: lib,
            lifecycle: Lifecycle::new(),
            created: false,
            dims: None,
            desc: desc_meta,
            ids,
            metas,
        })
    }

    /// Illegal current state for `op` — rejected before entering native.
    fn illegal(&self, op: &'static str) -> OfxError {
        OfxError::InvalidState { operation: op, state: self.state() }
    }

    /// Apply `event` to the lifecycle, mapping the transition error.
    fn transit(&mut self, event: LifecycleEvent, op: &'static str) -> Result<(), OfxError> {
        self.lifecycle.transition(event).map_err(|_| self.illegal(op))?;
        Ok(())
    }

    /// `id` → `(index, meta)`; `BadHandle` for an out-of-range `ParamId`.
    fn meta_of(&self, id: ParamId) -> Result<(usize, &ParamMeta), OfxError> {
        self.metas
            .get(id.0 as usize)
            .map(|m| (id.0 as usize, m))
            .ok_or(OfxError::BadHandle)
    }
}

impl Drop for OfxInstance {
    #[expect(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: `effect` is a live or quarantined attempt token produced
        // by `rv_ofx_instance_new`; release runs once and the catalog `Rc`
        // still holds the native module mapped.
        unsafe { sys::rv_ofx_instance_release(self.effect) }
    }
}

#[expect(unsafe_code)]
impl VideoPluginInstance for OfxInstance {
    type Error = OfxError;

    fn state(&self) -> LifecycleState {
        self.lifecycle.state()
    }

    fn descriptor(&self) -> PluginDescriptorView<'_> {
        PluginDescriptorView {
            id: &self.desc.id,
            name: &self.desc.name,
            vendor: &self.desc.vendor,
            version: &self.desc.version,
            features: &[],
        }
    }

    fn parameters(&self) -> &[ParamId] {
        &self.ids
    }

    fn parameter(&self, id: ParamId) -> Option<PlainParamInfo<'_>> {
        self.metas.get(id.0 as usize).map(|m| PlainParamInfo {
            id,
            name: &m.name,
            unit: &m.unit,
            min_value: m.min_value,
            max_value: m.max_value,
            display_min_value: m.display_min_value,
            display_max_value: m.display_max_value,
            default_value: m.default_value,
            flags: m.flags,
            choices: &m.choices,
        })
    }

    fn parameter_value(&self, id: ParamId) -> Result<PlainParamValue, OfxError> {
        if self.state().is_destroyed() {
            return Err(self.illegal("parameter_value"));
        }
        let (idx, meta) = self.meta_of(id)?;
        let mut vd = 0.0;
        let mut vi = 0;
        // SAFETY: `effect` is a live attempt token; `idx` is bounded.
        let rc = unsafe { sys::rv_ofx_param_get(self.effect, idx, &raw mut vd, &raw mut vi) };
        if rc != 0 {
            return Err(map_status("param_get", rc));
        }
        Ok(match meta.kind {
            pk::DOUBLE => PlainParamValue::Double(vd),
            pk::INTEGER => PlainParamValue::Integer(vi),
            pk::BOOLEAN => PlainParamValue::Boolean(vi != 0),
            pk::CHOICE => PlainParamValue::Choice(vi.cast_unsigned()),
            _ => return Err(OfxError::Unsupported("unsupported param kind")),
        })
    }

    fn set_parameter(&mut self, id: ParamId, value: PlainParamValue) -> Result<(), OfxError> {
        if self.state().is_destroyed() {
            return Err(self.illegal("set_parameter"));
        }
        let (idx, _m) = self.meta_of(id)?;
        let (kind, vd, vi) = match value {
            PlainParamValue::Double(v) => (pk::DOUBLE, v, 0),
            PlainParamValue::Integer(v) => (pk::INTEGER, 0.0, v),
            PlainParamValue::Boolean(v) => (pk::BOOLEAN, 0.0, i32::from(v)),
            PlainParamValue::Choice(v) => (pk::CHOICE, 0.0, v.cast_signed()),
            _ => return Err(OfxError::InvalidValue("unsupported value kind")),
        };
        // SAFETY: `effect` is a live attempt token; `idx` is bounded by
        // `parameters()`; `host_write=1` enforces readonly flags.
        let rc = unsafe { sys::rv_ofx_param_set(self.effect, idx, kind, vd, vi, 1) };
        if rc != 0 {
            return Err(map_status("param_set", rc));
        }
        if self.created {
            // SAFETY: `effect` is a created instance token; `idx` is a valid
            // param index.
            let rc = unsafe { sys::rv_ofx_instance_changed(self.effect, idx) };
            if rc != 0 {
                return Err(map_status("InstanceChanged", rc));
            }
        }
        Ok(())
    }

    fn initialize(&mut self, config: &VideoProcessConfig) -> Result<(), OfxError> {
        if self.state() != LifecycleState::Created {
            return Err(self.illegal("initialize"));
        }
        if config.format() != PixelFormat::Rgba8 {
            return Err(OfxError::Unsupported("only PixelFormat::Rgba8 is supported"));
        }
        let fps = f64::from(config.frame_rate().numerator())
            / f64::from(config.frame_rate().denominator());
        // SAFETY: `effect` is a live attempt token; dims were validated by
        // `VideoProcessConfig::new`.
        let rc = unsafe {
            sys::rv_ofx_instance_configure(self.effect, config.width(), config.height(), fps)
        };
        if rc != 0 {
            return Err(map_status("configure", rc));
        }
        // SAFETY: `effect` is a live attempt token; the bridge tracks
        // `created`/`create_attempted` so retries are safe.
        let rc = unsafe { sys::rv_ofx_instance_create(self.effect) };
        if rc != 0 {
            return Err(map_status("CreateInstance", rc));
        }
        self.created = true;
        self.dims = Some((config.width(), config.height()));
        self.transit(LifecycleEvent::Initialize, "initialize")
    }

    fn activate(&mut self) -> Result<(), OfxError> {
        if self.state() != LifecycleState::Initialized {
            return Err(self.illegal("activate"));
        }
        // SAFETY: `effect` is a live created instance token.
        let rc = unsafe { sys::rv_ofx_instance_preferences(self.effect) };
        if rc != 0 {
            self.transit(LifecycleEvent::ActivateFailed, "activate")?;
            return Err(map_status("GetClipPreferences", rc));
        }
        self.transit(LifecycleEvent::Activate, "activate")
    }

    fn start_processing(&mut self) -> Result<(), OfxError> {
        // The native sequence bracket is per-frame — each `render` call
        // wraps Begin/Render/EndSequenceRender around its own staging.
        self.transit(LifecycleEvent::StartProcessing, "start_processing")
    }

    #[expect(unsafe_code)]
    fn render(
        &mut self,
        input: &VideoFrame,
        output: &mut VideoFrame,
        at: MediaTime,
    ) -> Result<(), OfxError> {
        // Transit first: `Process` is only legal from `Processing`, so an
        // out-of-state render is rejected before any foreign call.
        self.transit(LifecycleEvent::Process, "render")?;
        let Some((w, h)) = self.dims else {
            return Err(self.illegal("render"));
        };
        if input.format() != PixelFormat::Rgba8 || output.format() != PixelFormat::Rgba8 {
            return Err(OfxError::Unsupported("only PixelFormat::Rgba8 is supported"));
        }
        if input.width() != w
            || input.height() != h
            || output.width() != w
            || output.height() != h
        {
            return Err(OfxError::InvalidValue(
                "frame dimensions differ from the configured size",
            ));
        }
        let src = input
            .plane(0)
            .ok_or(OfxError::InvalidValue("input frame has no plane 0"))?;
        let mut dst = output
            .plane_mut(0)
            .ok_or(OfxError::InvalidValue("output frame has no plane 0"))?;
        let time = at.to_seconds_f64();
        if !time.is_finite() {
            return Err(OfxError::InvalidValue("render time is not finite"));
        }
        // SAFETY: `effect` is a live configured instance token. `src`/`dst`
        // rows are at least `w*4` bytes (bridge rejects smaller strides and
        // copies only `w*4` per row), and both planes outlive the call —
        // the staging buffers exposed to the plugin are bracket-scoped.
        let rc = unsafe {
            sys::rv_ofx_instance_render(
                self.effect,
                src.data().as_ptr(),
                src.stride(),
                dst.data_mut().as_mut_ptr(),
                dst.stride(),
                time,
            )
        };
        if rc != status::OK {
            return Err(map_status("Render", rc));
        }
        Ok(())
    }

    fn stop_processing(&mut self) -> Result<(), OfxError> {
        self.transit(LifecycleEvent::StopProcessing, "stop_processing")
    }

    fn deactivate(&mut self) -> Result<(), OfxError> {
        self.transit(LifecycleEvent::Deactivate, "deactivate")
    }

    fn destroy(&mut self) -> Result<(), OfxError> {
        if self.state().is_destroyed() {
            return Err(self.illegal("destroy"));
        }
        // Transit first: an illegal Destroy must be rejected before any
        // foreign call. Once the transition lands, destroy is terminal from
        // the adapter's view even when the native action fails — the C
        // bridge quarantines the instance for `Unload`.
        self.transit(LifecycleEvent::Destroy, "destroy")?;
        if self.created {
            // SAFETY: `effect` is a live created instance token.
            let rc = unsafe { sys::rv_ofx_instance_destroy(self.effect) };
            if rc != status::OK {
                return Err(map_status("DestroyInstance", rc));
            }
        }
        // Not created — the bridge's destroy is a no-op; skipping the call
        // keeps the created-only path free of native round-trips.
        Ok(())
    }
}
