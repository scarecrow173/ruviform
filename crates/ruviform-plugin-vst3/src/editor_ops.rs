//! `IRvfEditorOps` — the ruviform-private `FUnknown` interface that
//! carries the editor verbs the VST3 `IPlugView` contract has no
//! surface for (Issue #630, K1 seam).
//!
//! This is the host-side half of a fixture-private contract: the
//! adapter `queryInterface`s the open `IPlugView` for this IID and
//! drives the verbs through the returned vtable — the VST3 mirror of
//! the CLAP `plugin.get_extension("clap.ext.gui")` mechanism, not an
//! OS injection and not a standard VST3 interface. A plugin without
//! the seam answers `kNoInterface` → the adapter reports the defined
//! `Unsupported` refusal.
//!
//! The implementer half lives in the test fixture at
//! `crates/ruviform-plugin-test-vst3/src/editor_ops.rs` — the fixture
//! cdylib deliberately does not depend on this crate (its dep tree is
//! documented in its `Cargo.toml`), so the contract is duplicated by
//! definition: **IID byte string, vtable layout and verb semantics
//! must stay byte-identical between the two files**. The recorded
//! vocabulary (`fe:`/`po:`/`pc:`/`co:`/`cc:` tokens) is fixed in
//! `docs/designs/issue-630-editor-verbs.md`.
//!
//! ## Seam verbs
//!
//! - `apply_focus(element, focused)` — element-focus *delivery*: the
//!   plugin interprets `element` internally and records the intent
//!   (`fe:{element}:{state}`); it is not an OS-focus claim. The
//!   `focused` flag mirrors `IPlugView::onFocus(TBool)` so the seam
//!   can carry a release later without a signature change —
//!   `set_gui_focus_target(EditorElement)` currently always delivers
//!   `1`.
//! - `popup(kind, open)` — plugin-owned popup drive: `kind` is the
//!   `GuiPopupKind` code point below; `open` selects the transition.
//!   An unknown `kind` declines `kInvalidArgument`; both kinds are
//!   explicit-state and idempotent (a no-op request is `kResultOk`
//!   and records no token).

use std::ffi::c_void;
use std::ptr;

#[cfg(test)]
use vst3::Class;
use vst3::Interface;
use vst3::Steinberg::{FUnknown, FUnknownVtbl, TBool, TUID, uint32};
use vst3::Steinberg::{kResultOk, tresult};
use vst3::com_scrape_types::{Guid, Inherits, SmartPtr};

/// `GuiPopupKind::ChildWindow` — a `WS_CHILD` auxiliary surface under
/// the editor's own child HWND.
pub(crate) const POPUP_KIND_CHILD_WINDOW: uint32 = 0;
/// `GuiPopupKind::Dialog` — a `WS_POPUP` top-level surface owned by
/// the editor's root ancestor.
pub(crate) const POPUP_KIND_DIALOG: uint32 = 1;

/// The ruviform-private editor-ops interface — see the module docs
/// for the contract and the fixture-side mirror file.
#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct IRvfEditorOps {
    vtbl: *const IRvfEditorOpsVtbl,
}

// COM object pointers cross threads only as opaque handles the
// owning UI thread drives — the same convention the generated
// bindings apply to every interface.
#[expect(unsafe_code)]
unsafe impl Send for IRvfEditorOps {}
#[expect(unsafe_code)]
unsafe impl Sync for IRvfEditorOps {}

#[expect(unsafe_code)]
impl vst3::com_scrape_types::Unknown for IRvfEditorOps {
    /// Routes the QI through the object's own `FUnknownVtbl` prefix —
    /// `IRvfEditorOps`'s vtbl starts with the shared `FUnknownVtbl`,
    /// so `this` reinterpreted as `*mut FUnknown` is valid (the
    /// `Interface` layout contract).
    unsafe fn query_interface(this: *mut Self, iid: &Guid) -> Option<*mut c_void> {
        let ptr = this.cast::<FUnknown>();
        let mut obj = ptr::null_mut();
        // SAFETY: `ptr` is a live interface pointer whose vtbl prefix
        // is `FUnknownVtbl`; `iid`/`obj` are valid call arguments.
        let result = unsafe {
            ((*(*ptr).vtbl).queryInterface)(ptr, iid.as_ptr().cast::<TUID>(), &raw mut obj)
        };
        if result == kResultOk { Some(obj) } else { None }
    }
    unsafe fn add_ref(this: *mut Self) -> usize {
        let ptr = this.cast::<FUnknown>();
        // SAFETY: `ptr` is a live interface pointer.
        unsafe { ((*(*ptr).vtbl).addRef)(ptr) as usize }
    }
    unsafe fn release(this: *mut Self) -> usize {
        let ptr = this.cast::<FUnknown>();
        // SAFETY: `ptr` is a live interface pointer whose last
        // `release` runs while the implementer is still alive.
        unsafe { ((*(*ptr).vtbl).release)(ptr) as usize }
    }
}

#[expect(unsafe_code)]
unsafe impl Interface for IRvfEditorOps {
    type Vtbl = IRvfEditorOpsVtbl;
    /// `"RVFEDITOROPS0001"` — a ruviform-namespaced 16-byte TUID, not
    /// an SDK-assigned IID; collision with a standard interface is
    /// impossible by construction.
    const IID: Guid = *b"RVFEDITOROPS0001";
    #[inline]
    fn inherits(iid: &Guid) -> bool {
        iid == &Self::IID || FUnknown::inherits(iid)
    }
}

#[expect(unsafe_code)]
unsafe impl Inherits<FUnknown> for IRvfEditorOps {}

/// The seam vtable — `FUnknownVtbl` prefix plus the two editor-ops
/// verbs. Layout is the contract: fields may only ever be appended.
#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct IRvfEditorOpsVtbl {
    pub base: FUnknownVtbl,
    /// `apply_focus(element, focused)` — see the module docs.
    pub apply_focus: unsafe extern "system" fn(
        this: *mut IRvfEditorOps,
        element: uint32,
        focused: TBool,
    ) -> tresult,
    /// `popup(kind, open)` — see the module docs.
    pub popup:
        unsafe extern "system" fn(this: *mut IRvfEditorOps, kind: uint32, open: TBool) -> tresult,
}

/// The seam's verb surface — the same blanket-`SmartPtr` dispatch the
/// generated `*Trait`s use, so `ComPtr<IRvfEditorOps>` calls through
/// the vtable exactly like `IPlugView`.
#[expect(unsafe_code)]
pub(crate) trait IRvfEditorOpsTrait {
    /// Delivers an element-focus intent — see the module docs.
    unsafe fn apply_focus(&self, element: uint32, focused: TBool) -> tresult;
    /// Drives a plugin-owned popup transition — see the module docs.
    unsafe fn popup(&self, kind: uint32, open: TBool) -> tresult;
}

#[expect(unsafe_code)]
impl<P> IRvfEditorOpsTrait for P
where
    P: SmartPtr,
    P::Target: Inherits<IRvfEditorOps>,
{
    #[inline]
    unsafe fn apply_focus(&self, element: uint32, focused: TBool) -> tresult {
        let ptr = self.ptr().cast::<IRvfEditorOps>();
        // SAFETY: `ptr` is a live `IRvfEditorOps` pointer; the vtbl
        // slot is the implementer's `apply_focus`.
        unsafe { ((*(*ptr).vtbl).apply_focus)(ptr, element, focused) }
    }
    #[inline]
    unsafe fn popup(&self, kind: uint32, open: TBool) -> tresult {
        let ptr = self.ptr().cast::<IRvfEditorOps>();
        // SAFETY: `ptr` is a live `IRvfEditorOps` pointer.
        unsafe { ((*(*ptr).vtbl).popup)(ptr, kind, open) }
    }
}

// The construction half (`Construct`/`make_vtbl`) is compiled only
// for tests — the adapter never implements the seam, it only queries
// and calls it. Fixture-side this block compiles unconditionally.
#[cfg(test)]
#[expect(unsafe_code)]
impl IRvfEditorOps {
    const fn make_vtbl<C, W, const OFFSET: isize>() -> IRvfEditorOpsVtbl
    where
        C: IRvfEditorOpsTrait + Class,
        W: vst3::com_scrape_types::Wrapper<C>,
    {
        unsafe extern "system" fn apply_focus<C, W, const OFFSET: isize>(
            this: *mut IRvfEditorOps,
            element: uint32,
            focused: TBool,
        ) -> tresult
        where
            C: IRvfEditorOpsTrait + Class,
            W: vst3::com_scrape_types::Wrapper<C>,
        {
            // SAFETY: `this` points at the `IRvfEditorOps` slot inside
            // the object's `Header<C>` — OFFSET is that slot's byte
            // offset, so `header_ptr` is the valid header the
            // com-scrape-types dispatch contract guarantees.
            unsafe {
                let header_ptr = this
                    .cast::<u8>()
                    .offset(-OFFSET)
                    .cast::<vst3::com_scrape_types::Header<C>>();
                let ptr = <W as vst3::com_scrape_types::Wrapper<C>>::data_from_header(header_ptr);
                (*ptr).apply_focus(element, focused)
            }
        }
        unsafe extern "system" fn popup<C, W, const OFFSET: isize>(
            this: *mut IRvfEditorOps,
            kind: uint32,
            open: TBool,
        ) -> tresult
        where
            C: IRvfEditorOpsTrait + Class,
            W: vst3::com_scrape_types::Wrapper<C>,
        {
            // SAFETY: same dispatch contract as `apply_focus`.
            unsafe {
                let header_ptr = this
                    .cast::<u8>()
                    .offset(-OFFSET)
                    .cast::<vst3::com_scrape_types::Header<C>>();
                let ptr = <W as vst3::com_scrape_types::Wrapper<C>>::data_from_header(header_ptr);
                (*ptr).popup(kind, open)
            }
        }
        IRvfEditorOpsVtbl {
            base: FUnknown::make_vtbl::<C, W, OFFSET>(),
            apply_focus: apply_focus::<C, W, OFFSET>,
            popup: popup::<C, W, OFFSET>,
        }
    }
}

#[cfg(test)]
#[expect(unsafe_code)]
unsafe impl<C, W, const OFFSET: isize> vst3::com_scrape_types::Construct<C, W, OFFSET>
    for IRvfEditorOps
where
    C: IRvfEditorOpsTrait + Class,
    W: vst3::com_scrape_types::Wrapper<C>,
{
    const OBJ: Self = IRvfEditorOps {
        vtbl: &Self::make_vtbl::<C, W, OFFSET>(),
    };
}
