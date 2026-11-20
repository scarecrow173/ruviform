//! The `test_stateful` signal law and its real state surface — the
//! suite's state-persistence oracle (Issue #630 `test_stateful`
//! slice).
//!
//! ## DSP law
//!
//! `test_stateful` reuses the `test_gain` parameter law verbatim:
//! `Gain` is parameter id `0`, plain range `[0.0, 2.0]`, default
//! `1.0`, automatable, with the deliberately non-identity
//! normalized↔plain `ParamInfo` mapping. The DSP is the same
//! `out = in * gain` law — the fixture exists to prove *state*
//! behavior, not new audio math, so [`crate::gain::GAIN_PARAM`] and
//! [`crate::gain::apply_gain`] are shared rather than duplicated.
//!
//! ## Persistent-state law
//!
//! The gain lives in a [`SharedGain`] cell — an `Arc<AtomicU64>` of
//! `f64` bits in the **plain** domain — shared between the RT
//! processor (one atomic load at block start, one atomic store at
//! block end; allocation-free, lock-free) and the component's
//! non-RT state verbs. This is the fixture's deliberate divergence
//! from `test_gain`: `prepare` does **not** reset the gain. A
//! stateful plugin's whole contract is that restored state survives
//! re-activation — a host that restores a preset and then starts
//! playback must hear the restored gain, so resetting on
//! `setupProcessing` would make the fixture lie about real plugin
//! behavior. The deterministic initial state comes from construction
//! (parameter default) and from `setState`, never from activation.
//!
//! ## State blob law
//!
//! Both the component and controller payloads are the same
//! plugin-defined format — the current **v2** layout is
//! `[u32 "RFST" LE][u32 version = 2 LE][f64 LE][u32 checksum LE]`
//! (20 bytes), where `checksum` is FNV-1a-32 over the 16-byte core.
//! The **v1** layout is the same without the checksum field (16
//! bytes). Payloads differ only in the `f64` domain:
//!
//! | payload | `f64` domain | written by | read by |
//! | ------- | ------------ | ---------- | ------- |
//! | component | plain gain `[0.0, 2.0]` | `IComponent::getState` | `IComponent::setState` |
//! | controller | normalized param `[0.0, 1.0]` | `IEditController::getState` | `IEditController::setState` |
//!
//! `IEditController::setComponentState` receives the *component*
//! payload (the adapter re-serializes it through
//! `component.getState` before the hand-off), decodes the plain
//! gain and folds it into the controller's normalized value — the
//! standard distributed-plugin state hand-off.
//!
//! Restore is a defined, typed outcome — never a silent accept:
//!
//! | input | outcome |
//! | ----- | ------- |
//! | v2, valid checksum, `len == 20` | **Canonical** accept |
//! | v1, `len == 16` | **Migrated** accept — the next save writes v2 |
//! | v2, valid checksum, `len > 20` | **Degraded** accept — the unknown extension tail is dropped and the next save re-serializes canonical |
//! | v2, invalid checksum | `ChecksumMismatch` rejection |
//! | any other version | `UnsupportedVersion` rejection |
//! | bad magic, bad length, non-finite `f64` | `Malformed` rejection |
//!
//! Every rejection maps to `kResultFalse` (component/controller) →
//! the host adapter's `PluginFailed`; the instance survives and its
//! stored state is untouched. A finite out-of-range plain value
//! clamps into the parameter domain per the `ParamInfo` law — the
//! same clamp the CLAP twin's `set_gain_plain` applies.
//!
//! Serialization is deterministic: fixed-width little-endian fields,
//! no timestamps, no padding — `save → restore → save` is an exact
//! byte round-trip and `save → save` is byte-identical. That
//! determinism is the oracle the L1 e2e asserts.
//!
//! ## RT boundary
//!
//! The state verbs (`setState`/`getState`/`setComponentState`) are
//! non-RT by the VST3 contract — they may allocate (the `IBStream`
//! drain allocates a `Vec`) and never run on the process path. The
//! processor's only state-surface contact is the atomic cell
//! load/store inside `process`, which stays allocation-free.

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventKind, EventList, PluginProcessor, PrepareError, ProcessConfig,
    ProcessContext, ProcessResult, TransportInfo,
};
use vst3::Steinberg::{
    IBStream, IBStreamTrait, TUID, kInvalidArgument, kResultFalse, kResultOk, tresult,
};
use vst3::{ComRef, uid};

use crate::component::ComponentStateSurface;
use crate::gain::{GAIN_PARAM, apply_gain};

/// `test_stateful`'s controller class ID — a fresh `uid` for the
/// test suite, distinct from `test_gain`/`test_fault`/`test_editor`
/// controller CIDs and the adapter fixture's CIDs.
pub(crate) const STATEFUL_CONTROLLER_CID: TUID =
    uid(0x9E4C_7A15, 0x3F8D_2B61, 0xC5E9_04A7, 0x8B1D_6F3E);

/// The persistent gain cell shared between the RT processor and the
/// component's non-RT state verbs — `f64` bits in the plain domain,
/// always clamped into the parameter range on store.
///
/// **RT contract:** `load`/`store` are single atomic ops —
/// RT-suitable; `new` is construction-time only.
#[derive(Clone)]
pub(crate) struct SharedGain(Arc<AtomicU64>);

impl SharedGain {
    /// A cell at `plain`, clamped into the parameter domain.
    pub(crate) fn new(plain: f64) -> Self {
        Self(Arc::new(AtomicU64::new(
            plain
                .clamp(GAIN_PARAM.min_value, GAIN_PARAM.max_value)
                .to_bits(),
        )))
    }

    /// The current plain gain.
    ///
    /// **RT contract:** RT-suitable — atomic load, allocation-free.
    pub(crate) fn load(&self) -> f64 {
        f64::from_bits(self.0.load(Ordering::Acquire))
    }

    /// Stores a plain gain — clamped into the parameter domain, the
    /// same law the CLAP twin's `set_gain_plain` applies.
    ///
    /// **RT contract:** RT-suitable — atomic store, allocation-free.
    /// Callers pass finite values only (the blob decoder rejects
    /// non-finite `f64`s; `ParamValue` rejects non-finite normalized
    /// values before `denormalize` runs).
    pub(crate) fn store(&self, plain: f64) {
        self.0.store(
            plain
                .clamp(GAIN_PARAM.min_value, GAIN_PARAM.max_value)
                .to_bits(),
            Ordering::Release,
        );
    }
}

/// The cell is the component's state surface — the component-payload
/// blob carries the plain gain verbatim (the codec's `f64` domain).
impl ComponentStateSurface for SharedGain {
    fn set_component_state(&self, bytes: &[u8]) -> bool {
        let Some(plain) = decode_state_blob(bytes) else {
            return false;
        };
        self.store(plain);
        true
    }

    fn component_state_blob(&self) -> Vec<u8> {
        encode_state_blob(self.load()).to_vec()
    }
}

// ---------------------------------------------------------------------------
// State blob law — `[u32 "RFST" LE][u32 version LE][f64 LE][u32 checksum LE]`
// ---------------------------------------------------------------------------

/// Blob magic — the ASCII tag `"RFST"` (Ruviform Stateful Test) as a
/// little-endian `u32`. A foreign or corrupt payload fails this check
/// before any field is interpreted.
const STATE_MAGIC: u32 = u32::from_le_bytes(*b"RFST");

/// The current blob format version — `2` appends a `u32` checksum
/// over the core fields to the v1 layout.
const STATE_VERSION: u32 = 2;

/// The legacy format version — the original 16-byte
/// `[magic][version][f64]` layout with no checksum field.
const STATE_VERSION_V1: u32 = 1;

/// The shared core: magic + version + one `f64` — the field span the
/// v2 checksum covers and the exact length of a legacy v1 blob.
const STATE_CORE_LEN: usize = 16;

/// Exact current blob length — the 16-byte core plus the `u32`
/// checksum.
pub(crate) const STATE_BLOB_LEN: usize = 20;

/// FNV-1a-32 over the blob's core fields — a deterministic integrity
/// tag that lets restore distinguish a corrupt payload (rejection)
/// from an older-format one (migration).
fn state_checksum(core: &[u8]) -> u32 {
    const FNV_OFFSET: u32 = 0x811C_9DC5;
    const FNV_PRIME: u32 = 0x0100_0193;
    let mut hash = FNV_OFFSET;
    for &byte in core {
        hash = (hash ^ u32::from(byte)).wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Serializes `value` into the deterministic 20-byte v2 blob.
#[must_use]
pub(crate) fn encode_state_blob(value: f64) -> [u8; STATE_BLOB_LEN] {
    let mut blob = [0_u8; STATE_BLOB_LEN];
    blob[0..4].copy_from_slice(&STATE_MAGIC.to_le_bytes());
    blob[4..8].copy_from_slice(&STATE_VERSION.to_le_bytes());
    blob[8..16].copy_from_slice(&value.to_le_bytes());
    let checksum = state_checksum(&blob[0..16]);
    blob[16..20].copy_from_slice(&checksum.to_le_bytes());
    blob
}

/// Serializes `value` into the legacy 16-byte v1 blob — the payload a
/// pre-checksum fixture saved. Exists so tests can hand `setState` /
/// `setComponentState` a real older-format payload and observe the
/// migration path.
#[cfg(test)]
fn encode_state_blob_v1(value: f64) -> [u8; STATE_CORE_LEN] {
    let mut blob = [0_u8; STATE_CORE_LEN];
    blob[0..4].copy_from_slice(&STATE_MAGIC.to_le_bytes());
    blob[4..8].copy_from_slice(&STATE_VERSION_V1.to_le_bytes());
    blob[8..16].copy_from_slice(&value.to_le_bytes());
    blob
}

/// How an accepted blob related to the current format — the restore
/// path's provenance report. Every provenance still stores the
/// decoded value; the distinctions exist so tests (and future
/// debuggability) can observe migration and forward-compat degrade
/// rather than have them pass silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StateProvenance {
    /// A current-version blob with a valid checksum.
    Canonical,
    /// A legacy v1 blob — accepted; the next save re-serializes the
    /// migrated value in the v2 layout.
    Migrated,
    /// A v2 core with a valid checksum followed by unknown extension
    /// bytes — accepted with the tail dropped (forward-compat
    /// degrade); the next save re-serializes the canonical layout.
    Degraded,
}

/// Why a blob was rejected — one defined outcome per violation, all
/// of which the ABI verbs map to their defined rejection
/// (`kResultFalse` / `PluginError`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StateRejection {
    /// Bad magic, a length no known layout admits, or a non-finite
    /// `f64` payload.
    Malformed,
    /// A version this build does not know — including newer formats
    /// the fixture cannot interpret.
    UnsupportedVersion,
    /// A v2 blob whose stored checksum does not cover its core —
    /// corruption, not an unknown format.
    ChecksumMismatch,
}

/// A decoded blob: the plain-domain value plus its provenance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DecodedState {
    /// The decoded `f64` payload.
    pub(crate) value: f64,
    /// How the blob related to the current format.
    pub(crate) provenance: StateProvenance,
}

/// Decodes a blob with full provenance — the detailed restore
/// contract the module docs tabulate. Unknown-version blobs report
/// [`StateRejection::UnsupportedVersion`] before any field beyond the
/// version is trusted.
pub(crate) fn inspect_state_blob(bytes: &[u8]) -> Result<DecodedState, StateRejection> {
    if bytes.len() < 8 {
        return Err(StateRejection::Malformed);
    }
    if u32::from_le_bytes(
        bytes[0..4]
            .try_into()
            .map_err(|_| StateRejection::Malformed)?,
    ) != STATE_MAGIC
    {
        return Err(StateRejection::Malformed);
    }
    let version = u32::from_le_bytes(
        bytes[4..8]
            .try_into()
            .map_err(|_| StateRejection::Malformed)?,
    );
    match version {
        STATE_VERSION_V1 => {
            if bytes.len() != STATE_CORE_LEN {
                return Err(StateRejection::Malformed);
            }
            let value = f64::from_le_bytes(
                bytes[8..16]
                    .try_into()
                    .map_err(|_| StateRejection::Malformed)?,
            );
            if !value.is_finite() {
                return Err(StateRejection::Malformed);
            }
            Ok(DecodedState {
                value,
                provenance: StateProvenance::Migrated,
            })
        }
        STATE_VERSION => {
            if bytes.len() < STATE_BLOB_LEN {
                return Err(StateRejection::Malformed);
            }
            let stored = u32::from_le_bytes(
                bytes[16..20]
                    .try_into()
                    .map_err(|_| StateRejection::Malformed)?,
            );
            if stored != state_checksum(&bytes[0..16]) {
                return Err(StateRejection::ChecksumMismatch);
            }
            let value = f64::from_le_bytes(
                bytes[8..16]
                    .try_into()
                    .map_err(|_| StateRejection::Malformed)?,
            );
            if !value.is_finite() {
                return Err(StateRejection::Malformed);
            }
            Ok(DecodedState {
                value,
                provenance: if bytes.len() == STATE_BLOB_LEN {
                    StateProvenance::Canonical
                } else {
                    StateProvenance::Degraded
                },
            })
        }
        _ => Err(StateRejection::UnsupportedVersion),
    }
}

/// Decodes a blob produced by [`encode_state_blob`] — or any blob
/// [`inspect_state_blob`] accepts (canonical, migrated, or
/// degraded) — returning only the value. `None` on any rejection.
/// The caller maps `None` to its ABI's defined rejection
/// (`kResultFalse` / `PluginError`).
#[must_use]
pub(crate) fn decode_state_blob(bytes: &[u8]) -> Option<f64> {
    inspect_state_blob(bytes).ok().map(|decoded| decoded.value)
}

// ---------------------------------------------------------------------------
// IBStream helpers — the non-RT read/write halves of the state verbs
// ---------------------------------------------------------------------------

/// Drains an `IBStream` into a `Vec` — the plugin-side read half.
/// Allocation is confined to this non-RT verb.
///
/// # Safety
///
/// `state` must be null or a valid `IBStream` for the call's
/// duration (the VST3 state-verb contract provides one).
#[expect(unsafe_code)]
pub(crate) unsafe fn stream_read_all(state: *mut IBStream) -> Result<Vec<u8>, tresult> {
    let Some(stream) = (unsafe { ComRef::from_raw(state) }) else {
        return Err(kInvalidArgument);
    };
    let mut out = Vec::new();
    let mut buf = [0_u8; 256];
    loop {
        let mut got = 0_i32;
        // SAFETY: `stream` is a valid `IBStream` for the call's
        // duration; `buf` is a live 256-byte destination.
        let result = unsafe {
            stream.read(
                buf.as_mut_ptr().cast::<c_void>(),
                i32::try_from(buf.len()).unwrap_or(i32::MAX),
                &raw mut got,
            )
        };
        if result != kResultOk {
            return Err(result);
        }
        if got <= 0 {
            return Ok(out);
        }
        let count = usize::try_from(got).unwrap_or(0).min(buf.len());
        out.extend_from_slice(&buf[..count]);
    }
}

/// Writes `bytes` into an `IBStream` — the plugin-side write half.
///
/// # Safety
///
/// `state` must be null or a valid `IBStream` for the call's
/// duration; `bytes` must be a live readable slice.
#[expect(unsafe_code)]
pub(crate) unsafe fn stream_write_all(state: *mut IBStream, bytes: &[u8]) -> tresult {
    let Some(stream) = (unsafe { ComRef::from_raw(state) }) else {
        return kInvalidArgument;
    };
    let mut written = 0_i32;
    // SAFETY: `stream` is a valid `IBStream`; `bytes` is a live
    // readable slice of the reported length.
    let result = unsafe {
        stream.write(
            bytes.as_ptr().cast::<c_void>().cast_mut(),
            i32::try_from(bytes.len()).unwrap_or(i32::MAX),
            &raw mut written,
        )
    };
    if result != kResultOk {
        return result;
    }
    // A short write would silently truncate `getState` output —
    // treat it as the defined rejection the caller maps to
    // `PluginFailed`.
    if usize::try_from(written).unwrap_or(0) != bytes.len() {
        return kResultFalse;
    }
    kResultOk
}

// ---------------------------------------------------------------------------
// The stateful processor — the `test_gain` DSP law over the shared cell
// ---------------------------------------------------------------------------

/// The `test_stateful` processor — the `test_gain` gain law with the
/// gain held in the shared [`SharedGain`] cell instead of a private
/// field, so the component's state verbs and the RT process path see
/// one source of truth.
pub(crate) struct TestStatefulProcessor {
    /// The persistent plain gain — the state surface's cell.
    gain: SharedGain,
}

impl TestStatefulProcessor {
    /// A processor bound to `gain` — the cell the component's state
    /// verbs also hold.
    pub(crate) fn new(gain: SharedGain) -> Self {
        Self { gain }
    }
}

impl PluginProcessor for TestStatefulProcessor {
    /// Activation hook — deliberately a no-op: the [`SharedGain`]
    /// cell is authoritative persistent state (see module docs), so
    /// `setupProcessing` must not clobber a restored or mutated gain.
    /// Non-RT (may allocate; this implementation does not need to).
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        Ok(())
    }

    /// Runs one block: loads the cell's plain gain once, applies the
    /// persisted gain from `cursor = 0` switching at each gain event's
    /// frame offset (the `test_gain` law verbatim), then stores the
    /// tail gain back — two atomic ops per block.
    ///
    /// **RT contract:** RT-suitable — bounded `O(frames × channels +
    /// events)` work; no allocation, locking, waiting or I/O.
    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        let frames = inputs
            .first()
            .map_or(0, BufferView::frames)
            .min(outputs.first().map_or(0, BufferView::frames));
        let mut cursor = 0_usize;
        let mut gain = self.gain.load();
        for event in events.iter() {
            let EventKind::ParamValue(param) = event.kind() else {
                continue;
            };
            if param.param_id != GAIN_PARAM.id {
                continue;
            }
            let offset = usize::try_from(event.frame_offset())
                .unwrap_or(usize::MAX)
                .min(frames);
            #[expect(clippy::cast_possible_truncation)]
            let segment_gain = gain as f32;
            apply_gain(inputs, outputs, cursor, offset, segment_gain);
            gain = GAIN_PARAM.denormalize(param.value);
            cursor = offset;
        }
        #[expect(clippy::cast_possible_truncation)]
        let tail_gain = gain as f32;
        apply_gain(inputs, outputs, cursor, frames, tail_gain);
        self.gain.store(gain);
        if inputs.is_empty() || outputs.is_empty() {
            ProcessResult::SLEEP
        } else {
            ProcessResult::CONTINUE
        }
    }
}

/// A Vec-backed `IBStream` shared by the state-verb tests across
/// the crate — the same file-like semantics the production
/// `BridgeStream` implements (EOF-honest reads, zero-filled seek
/// gaps, `int64` positions), minus the wire ceiling the tests do
/// not need. `test_stateful` and `test_stateful_single` both drive
/// their state-verb tests through it.
#[cfg(test)]
pub(crate) mod test_stream {
    use super::*;
    use std::cell::RefCell;

    use vst3::Steinberg::IBStream_::IStreamSeekMode_::{kIBSeekCur, kIBSeekEnd, kIBSeekSet};
    use vst3::{Class, ComPtr, ComWrapper};

    use crate::sdk_enum_i32;

    /// See the module docs — a file-like `IBStream` test double.
    pub(crate) struct TestStream {
        state: RefCell<TestStreamState>,
    }

    struct TestStreamState {
        data: Vec<u8>,
        pos: u64,
    }

    impl TestStream {
        /// An empty stream — the `getState` write target.
        pub(crate) fn new() -> Self {
            Self {
                state: RefCell::new(TestStreamState {
                    data: Vec::new(),
                    pos: 0,
                }),
            }
        }

        /// A stream pre-filled with `bytes` at position 0 — the
        /// `setState`/`setComponentState` read source.
        pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
            Self {
                state: RefCell::new(TestStreamState {
                    data: bytes,
                    pos: 0,
                }),
            }
        }

        /// Drains the accumulated bytes — the `getState` read-out.
        pub(crate) fn take_bytes(&self) -> Vec<u8> {
            std::mem::take(&mut self.state.borrow_mut().data)
        }
    }

    impl Class for TestStream {
        type Interfaces = (IBStream,);
    }

    #[expect(unsafe_code)]
    impl IBStreamTrait for TestStream {
        unsafe fn read(
            &self,
            buffer: *mut c_void,
            num_bytes: i32,
            num_bytes_read: *mut i32,
        ) -> tresult {
            if num_bytes < 0 || (buffer.is_null() && num_bytes > 0) {
                return kInvalidArgument;
            }
            let mut state = self.state.borrow_mut();
            let start = usize::try_from(state.pos).unwrap_or(usize::MAX);
            let count = state
                .data
                .len()
                .saturating_sub(start)
                .min(usize::try_from(num_bytes).unwrap_or(0));
            if count > 0 {
                // SAFETY: `buffer` is a valid `num_bytes`-byte
                // destination per the IBStream contract; `count <=
                // num_bytes` and the source slice is inside `data`.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        state.data[start..start + count].as_ptr(),
                        buffer.cast::<u8>(),
                        count,
                    );
                }
            }
            state.pos = state
                .pos
                .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
            if !num_bytes_read.is_null() {
                // SAFETY: contract out-ptr; `count <= i32::MAX`.
                unsafe { *num_bytes_read = i32::try_from(count).unwrap_or(i32::MAX) };
            }
            kResultOk
        }

        unsafe fn write(
            &self,
            buffer: *mut c_void,
            num_bytes: i32,
            num_bytes_written: *mut i32,
        ) -> tresult {
            if num_bytes < 0 || (buffer.is_null() && num_bytes > 0) {
                return kInvalidArgument;
            }
            let mut state = self.state.borrow_mut();
            let start = usize::try_from(state.pos).unwrap_or(usize::MAX);
            let count = usize::try_from(num_bytes).unwrap_or(0);
            if count > 0 {
                let end = start.saturating_add(count);
                state.data.resize(end, 0);
                // SAFETY: `buffer` points at `num_bytes` readable
                // bytes per the IBStream contract; the destination
                // was just sized to hold `count` bytes.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buffer.cast::<u8>(),
                        state.data[start..end].as_mut_ptr(),
                        count,
                    );
                }
                state.pos = state
                    .pos
                    .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
            }
            if !num_bytes_written.is_null() {
                // SAFETY: contract out-ptr; `count <= i32::MAX`.
                unsafe { *num_bytes_written = i32::try_from(count).unwrap_or(i32::MAX) };
            }
            kResultOk
        }

        unsafe fn seek(&self, pos: i64, mode: i32, result: *mut i64) -> tresult {
            let mut state = self.state.borrow_mut();
            let base = if mode == sdk_enum_i32(kIBSeekSet) {
                0
            } else if mode == sdk_enum_i32(kIBSeekCur) {
                i64::try_from(state.pos).unwrap_or(i64::MAX)
            } else if mode == sdk_enum_i32(kIBSeekEnd) {
                i64::try_from(state.data.len()).unwrap_or(i64::MAX)
            } else {
                return kInvalidArgument;
            };
            let Some(target) = base.checked_add(pos) else {
                return kResultFalse;
            };
            if target < 0 {
                return kResultFalse;
            }
            state.pos = u64::try_from(target).unwrap_or(u64::MAX);
            if !result.is_null() {
                // SAFETY: `result` is the contract's out-ptr.
                unsafe { *result = target };
            }
            kResultOk
        }

        unsafe fn tell(&self, pos: *mut i64) -> tresult {
            let state = self.state.borrow();
            if !pos.is_null() {
                // SAFETY: `pos` is the contract's out-ptr.
                unsafe { *pos = i64::try_from(state.pos).unwrap_or(i64::MAX) };
            }
            kResultOk
        }
    }

    /// The `IBStream` COM pointer of a wrapped [`TestStream`] — the
    /// returned `ComPtr` owns a reference and must outlive the FFI
    /// call it feeds.
    pub(crate) fn ib_stream(stream: &ComWrapper<TestStream>) -> ComPtr<IBStream> {
        stream
            .to_com_ptr::<IBStream>()
            .expect("TestStream must expose IBStream")
    }
}

#[cfg(test)]
mod tests {
    use super::test_stream::{TestStream, ib_stream};
    use super::*;

    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
    use ruviform_plugin_core::{Event, ParamEvent, ParamValue};
    use vst3::ComWrapper;
    use vst3::Steinberg::Vst::{IComponentTrait, IEditControllerTrait};
    use vst3::Steinberg::{kNotImplemented, kResultFalse};

    use crate::controller::ParamController;
    use crate::preset::TestStatefulComponent;

    /// `encode`/`decode` round-trips exactly — the determinism the
    /// save→restore→save oracle asserts.
    #[test]
    fn blob_round_trips_bit_exact() {
        for value in [0.0, 0.5, 0.75, 1.0, 1.75, 2.0] {
            let blob = encode_state_blob(value);
            assert_eq!(blob.len(), STATE_BLOB_LEN);
            assert_eq!(&blob[0..4], b"RFST");
            assert_eq!(decode_state_blob(&blob), Some(value));
        }
    }

    /// Every malformed shape is a defined `None` — never a panic or
    /// a fabricated value.
    #[test]
    fn malformed_blobs_are_rejected() {
        let good = encode_state_blob(0.75);
        // Wrong length — short and long.
        assert_eq!(decode_state_blob(&good[..8]), None);
        assert_eq!(decode_state_blob(&[0_u8; 24]), None);
        // Magic mismatch.
        let mut bad_magic = good;
        bad_magic[0] ^= 0xFF;
        assert_eq!(decode_state_blob(&bad_magic), None);
        // Version mismatch — a bogus high byte makes the version
        // field unreadable to this build.
        let mut bad_version = good;
        bad_version[7] = 0x02;
        assert_eq!(decode_state_blob(&bad_version), None);
        // Non-finite payloads — NaN and ±inf bit patterns. Note the
        // checksum must be recomputed for the mutated core or the
        // blob fails integrity first — either way it is a rejection.
        for bits in [
            f64::NAN.to_bits(),
            f64::INFINITY.to_bits(),
            f64::NEG_INFINITY.to_bits(),
        ] {
            let mut bad_value = good;
            bad_value[8..16].copy_from_slice(&bits.to_le_bytes());
            assert_eq!(decode_state_blob(&bad_value), None);
        }
    }

    /// `inspect_state_blob` reports the full restore contract:
    /// canonical acceptance, v1 migration, forward-compat degrade,
    /// and one typed rejection per violation class.
    #[test]
    fn inspect_reports_provenance_and_typed_rejections() {
        let blob = encode_state_blob(0.75);
        assert_eq!(
            inspect_state_blob(&blob),
            Ok(DecodedState {
                value: 0.75,
                provenance: StateProvenance::Canonical,
            })
        );

        // A legacy v1 payload — no checksum field — migrates: it is
        // accepted and the next save re-serializes v2.
        let v1 = encode_state_blob_v1(0.75);
        assert_eq!(v1.len(), STATE_CORE_LEN);
        assert_eq!(
            inspect_state_blob(&v1),
            Ok(DecodedState {
                value: 0.75,
                provenance: StateProvenance::Migrated,
            })
        );
        assert_eq!(decode_state_blob(&v1), Some(0.75));

        // A v2 core with a valid checksum plus unknown extension
        // bytes degrades — accepted, tail dropped.
        let mut extended = blob.to_vec();
        extended.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(
            inspect_state_blob(&extended),
            Ok(DecodedState {
                value: 0.75,
                provenance: StateProvenance::Degraded,
            })
        );
        assert_eq!(decode_state_blob(&extended), Some(0.75));

        // Truncated v2 (no checksum field) — malformed, not a
        // downgrade attack on the integrity field.
        assert_eq!(
            inspect_state_blob(&blob[..STATE_CORE_LEN]),
            Err(StateRejection::Malformed)
        );
        // Corrupt core or corrupt checksum — integrity failure.
        let mut corrupt_core = blob;
        corrupt_core[9] ^= 0xFF;
        assert_eq!(
            inspect_state_blob(&corrupt_core),
            Err(StateRejection::ChecksumMismatch)
        );
        let mut corrupt_sum = blob;
        corrupt_sum[16] ^= 0xFF;
        assert_eq!(
            inspect_state_blob(&corrupt_sum),
            Err(StateRejection::ChecksumMismatch)
        );
        // A version this build does not know — unsupported, and the
        // answer does not depend on checksum bytes.
        let mut future = blob;
        future[4..8].copy_from_slice(&3_u32.to_le_bytes());
        assert_eq!(
            inspect_state_blob(&future),
            Err(StateRejection::UnsupportedVersion)
        );
        // A v1-versioned blob at the v2 length is not a v1 blob —
        // malformed, not migrated.
        let mut fake_v1 = encode_state_blob_v1(0.75).to_vec();
        fake_v1.extend_from_slice(&[0; 4]);
        assert_eq!(inspect_state_blob(&fake_v1), Err(StateRejection::Malformed));
    }

    /// The cell clamps into the parameter domain on store — the
    /// `set_gain_plain` law.
    #[test]
    fn cell_store_clamps_into_param_domain() {
        let cell = SharedGain::new(GAIN_PARAM.default_value);
        assert_eq!(cell.load(), 1.0);
        cell.store(9.0);
        assert_eq!(cell.load(), 2.0);
        cell.store(-1.0);
        assert_eq!(cell.load(), 0.0);
    }

    /// The processor applies the `test_gain` law through the cell:
    /// an offset-0 event switches the whole block and the tail gain
    /// persists into the next block via the shared cell.
    #[test]
    fn processor_gain_flows_through_shared_cell() {
        let cell = SharedGain::new(GAIN_PARAM.default_value);
        let mut processor = TestStatefulProcessor::new(cell.clone());
        let events = [Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: GAIN_PARAM.id,
                value: ParamValue::new(0.375).unwrap(), // → plain 0.75
            }),
        )];
        let input = [vec![0.5_f32; 4]];
        let mut output = [vec![0.0_f32; 4]];
        {
            let frames = 4_u32;
            let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
            let inputs = [PlanarBufferView::new(&in_views).unwrap()];
            let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
            let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
            let list = EventList::new(&events, frames).unwrap();
            let mut context = ProcessContext::new();
            processor.process(&inputs, &mut outputs, list, None, &mut context);
        }
        assert_eq!(output[0], vec![0.375_f32; 4]);
        // The cell — the state surface's view — sees the same gain.
        assert_eq!(cell.load(), 0.75);
        // A state write lands in the processor's next block.
        cell.store(1.75);
        output[0].fill(0.0);
        {
            let frames = 4_u32;
            let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
            let inputs = [PlanarBufferView::new(&in_views).unwrap()];
            let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
            let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
            let list = EventList::empty(frames);
            let mut context = ProcessContext::new();
            processor.process(&inputs, &mut outputs, list, None, &mut context);
        }
        assert_eq!(output[0], vec![0.875_f32; 4]);
    }

    /// `prepare` does not reset the cell — the persistent-state law:
    /// a restored/mutated gain survives re-activation.
    #[test]
    fn prepare_preserves_cell_state() {
        let cell = SharedGain::new(GAIN_PARAM.default_value);
        cell.store(1.75);
        let mut processor = TestStatefulProcessor::new(cell.clone());
        let config =
            ProcessConfig::new(ruviform_audio_core::SampleRate::new(48_000).unwrap(), 1, 64)
                .unwrap();
        processor
            .prepare(&config, &mut ProcessContext::new())
            .unwrap();
        assert_eq!(cell.load(), 1.75);
    }

    // -------------------------------------------------------------------
    // COM-level state verbs — real `IBStream` round-trips
    // -------------------------------------------------------------------

    /// `IComponent::getState` writes the deterministic plain-gain
    /// blob through a real `IBStream`; `setState` restores it into
    /// the shared cell and a malformed blob is the defined
    /// `kResultFalse` rejection — never a silent accept.
    #[expect(unsafe_code)]
    #[test]
    fn component_state_verbs_round_trip_through_ibstream() {
        let component = TestStatefulComponent::new();

        // getState → the default plain gain (1.0) as the 16-byte blob.
        let out = ComWrapper::new(TestStream::new());
        let result = unsafe { component.getState(ib_stream(&out).as_ptr()) };
        assert_eq!(result, kResultOk);
        assert_eq!(out.take_bytes(), encode_state_blob(1.0).to_vec());

        // setState with a mutated plain gain → the cell sees it.
        let blob = encode_state_blob(1.75);
        let input = ComWrapper::new(TestStream::from_bytes(blob.to_vec()));
        let result = unsafe { component.setState(ib_stream(&input).as_ptr()) };
        assert_eq!(result, kResultOk);
        // getState re-serializes the restored value — deterministic.
        let out = ComWrapper::new(TestStream::new());
        let result = unsafe { component.getState(ib_stream(&out).as_ptr()) };
        assert_eq!(result, kResultOk);
        assert_eq!(out.take_bytes(), blob.to_vec());

        // A malformed blob is `kResultFalse` — defined rejection.
        let bad = ComWrapper::new(TestStream::from_bytes(vec![0_u8; 8]));
        let result = unsafe { component.setState(ib_stream(&bad).as_ptr()) };
        assert_eq!(result, kResultFalse);
        // The rejected write left the restored state untouched.
        let out = ComWrapper::new(TestStream::new());
        unsafe { component.getState(ib_stream(&out).as_ptr()) };
        assert_eq!(out.take_bytes(), blob.to_vec());
    }

    /// A legacy v1 blob restores through the real `IBStream` verb
    /// and the next `getState` re-serializes in the current v2
    /// layout — the migration path observable end-to-end through the
    /// COM surface.
    #[expect(unsafe_code)]
    #[test]
    fn v1_component_blob_migrates_on_next_save() {
        let component = TestStatefulComponent::new();

        // setState with a legacy 16-byte payload → accepted (Migrated).
        let v1 = encode_state_blob_v1(1.25);
        let input = ComWrapper::new(TestStream::from_bytes(v1.to_vec()));
        let result = unsafe { component.setState(ib_stream(&input).as_ptr()) };
        assert_eq!(result, kResultOk);

        // getState writes the migrated value as canonical v2 — the
        // payload grew the checksum field.
        let out = ComWrapper::new(TestStream::new());
        unsafe { component.getState(ib_stream(&out).as_ptr()) };
        assert_eq!(out.take_bytes(), encode_state_blob(1.25).to_vec());

        // A degraded blob — v2 core + extension tail — is also
        // accepted and re-serializes canonical (the tail is dropped).
        let mut extended = encode_state_blob(0.5).to_vec();
        extended.extend_from_slice(&[0xAB, 0xCD]);
        let input = ComWrapper::new(TestStream::from_bytes(extended));
        let result = unsafe { component.setState(ib_stream(&input).as_ptr()) };
        assert_eq!(result, kResultOk);
        let out = ComWrapper::new(TestStream::new());
        unsafe { component.getState(ib_stream(&out).as_ptr()) };
        assert_eq!(out.take_bytes(), encode_state_blob(0.5).to_vec());

        // A checksum-corrupt v2 blob and an unknown-version blob are
        // the same defined `kResultFalse` the malformed cases get.
        let mut corrupt = encode_state_blob(0.5);
        corrupt[16] ^= 0xFF;
        let input = ComWrapper::new(TestStream::from_bytes(corrupt.to_vec()));
        assert_eq!(
            unsafe { component.setState(ib_stream(&input).as_ptr()) },
            kResultFalse
        );
        let mut future = encode_state_blob(0.5);
        future[4..8].copy_from_slice(&9_u32.to_le_bytes());
        let input = ComWrapper::new(TestStream::from_bytes(future.to_vec()));
        assert_eq!(
            unsafe { component.setState(ib_stream(&input).as_ptr()) },
            kResultFalse
        );
    }

    /// The controller's state verbs speak the *normalized* domain:
    /// `setComponentState` folds the plain component payload into the
    /// normalized mirror, `setState`/`getState` carry the controller
    /// payload — the distributed-plugin hand-off the adapter drives.
    #[expect(unsafe_code)]
    #[test]
    fn controller_state_verbs_round_trip_domains() {
        let controller = ParamController::new_stateful(&GAIN_PARAM, 0);

        // getState → the default normalized value (0.5 for plain 1.0
        // on [0, 2]) as the 16-byte blob.
        let out = ComWrapper::new(TestStream::new());
        let result = unsafe { controller.getState(ib_stream(&out).as_ptr()) };
        assert_eq!(result, kResultOk);
        assert_eq!(out.take_bytes(), encode_state_blob(0.5).to_vec());

        // setComponentState with a plain-gain component blob (1.75 →
        // normalized 0.875) → getState re-serializes normalized.
        let comp_blob = encode_state_blob(1.75);
        let input = ComWrapper::new(TestStream::from_bytes(comp_blob.to_vec()));
        let result = unsafe { controller.setComponentState(ib_stream(&input).as_ptr()) };
        assert_eq!(result, kResultOk);
        let out = ComWrapper::new(TestStream::new());
        unsafe { controller.getState(ib_stream(&out).as_ptr()) };
        assert_eq!(out.take_bytes(), encode_state_blob(0.875).to_vec());

        // setState with a normalized controller blob (0.25) → the
        // parameter mirror follows.
        let ctrl_blob = encode_state_blob(0.25);
        let input = ComWrapper::new(TestStream::from_bytes(ctrl_blob.to_vec()));
        let result = unsafe { controller.setState(ib_stream(&input).as_ptr()) };
        assert_eq!(result, kResultOk);
        assert_eq!(
            unsafe { controller.getParamNormalized(GAIN_PARAM.id.0) },
            0.25
        );

        // Malformed blobs are `kResultFalse` on both verbs.
        let bad = ComWrapper::new(TestStream::from_bytes(vec![0_u8; 4]));
        let result = unsafe { controller.setComponentState(ib_stream(&bad).as_ptr()) };
        assert_eq!(result, kResultFalse);
        let bad = ComWrapper::new(TestStream::from_bytes(vec![0_u8; 4]));
        let result = unsafe { controller.setState(ib_stream(&bad).as_ptr()) };
        assert_eq!(result, kResultFalse);
    }

    /// The stateless controllers (`test_gain`/`test_fault`) keep the
    /// honest `kNotImplemented` on every state verb — the pin the L1
    /// e2e asserts through `HostError::Unsupported`.
    #[expect(unsafe_code)]
    #[test]
    fn stateless_controller_verbs_stay_not_implemented() {
        let controller = ParamController::new(&GAIN_PARAM, 0);
        let stream = ComWrapper::new(TestStream::new());
        let ptr = ib_stream(&stream);
        // SAFETY: `ptr` is a live `IBStream` for the calls.
        unsafe {
            assert_eq!(controller.setComponentState(ptr.as_ptr()), kNotImplemented);
            assert_eq!(controller.setState(ptr.as_ptr()), kNotImplemented);
            assert_eq!(controller.getState(ptr.as_ptr()), kNotImplemented);
        }
    }

    /// RT-contract test (Issue #22 lane): the stateful processor's
    /// `process` — the `test_gain` event loop over the atomic cell —
    /// performs no allocation inside the `assert_no_alloc` window.
    /// Fixture/buffer/event-list setup stays outside; only the
    /// measured call is inside.
    #[test]
    fn rt_process_through_shared_cell_is_allocation_free() {
        let cell = SharedGain::new(GAIN_PARAM.default_value);
        let mut processor = TestStatefulProcessor::new(cell.clone());
        let events = [Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: GAIN_PARAM.id,
                value: ParamValue::new(0.375).unwrap(), // → plain 0.75
            }),
        )];
        let input = [vec![0.5_f32; 64], vec![-0.25; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let frames = 64_u32;
        let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
        let inputs = [PlanarBufferView::new(&in_views).unwrap()];
        let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
        let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
        let list = EventList::new(&events, frames).unwrap();
        let mut context = ProcessContext::new();
        assert_no_alloc::assert_no_alloc(|| {
            processor.process(&inputs, &mut outputs, list, None, &mut context);
        });
        assert_eq!(output[0], vec![0.375_f32; 64]);
        assert_eq!(cell.load(), 0.75);
    }
}
