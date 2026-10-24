//! Native contracts observed through a real, pinned-SDK C fixture.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ruviform_media_core::{FrameRate, MediaTime, TimeBase};
use ruviform_plugin_core::{
    LifecycleState, ParamId, PlainParamValue, VideoPluginInstance, VideoProcessConfig,
};
use ruviform_plugin_ofx::{OfxError, OfxHost, OfxLibrary};
use ruviform_video_core::{PixelFormat, SharedPlane, VideoFrame};

const DLL_SET_HOST: usize = 0;
const GET_COUNT: usize = 1;
const GET_PLUGIN: usize = 2;
const SET_HOST: usize = 3;
const LOAD: usize = 4;
const UNLOAD: usize = 5;
const DESCRIBE: usize = 6;
const DESCRIBE_CONTEXT: usize = 7;
const CREATE_ENTER: usize = 8;
const CREATE_OK: usize = 9;
const DESTROY: usize = 10;
const BEGIN_CHANGED: usize = 11;
const CHANGED: usize = 12;
const END_CHANGED: usize = 13;
const CLIP_PREFS: usize = 14;
const BEGIN_RENDER: usize = 15;
const RENDER: usize = 16;
const END_RENDER: usize = 17;
const EVENTS: usize = 18;

const REFUSE_SET_HOST: i32 = 1;
const FAIL_CREATE: i32 = 1 << 1;
const FAIL_DESTROY: i32 = 1 << 2;
const FAIL_CLIP_PREFS: i32 = 1 << 3;
const INVALID_API: i32 = 1 << 4;
const EMPTY_ID: i32 = 1 << 5;
const NULL_SET_HOST: i32 = 1 << 6;
const NULL_MAIN: i32 = 1 << 7;
const NO_FILTER: i32 = 1 << 8;
const BAD_COUNT: i32 = 1 << 9;
const SINGLE_INSTANCE: i32 = 1 << 10;
const WIDE_BOUNDS: i32 = 1 << 11;
const NULL_ENTRY: i32 = 1 << 12;
const STRING_PARAM: i32 = 1 << 13;
const WORKER_ON_CREATE: i32 = 1 << 14;
const FAIL_BEGIN_RENDER: i32 = 1 << 15;
const FAIL_RENDER: i32 = 1 << 16;
const FAIL_END_RENDER: i32 = 1 << 17;
const REPLY_DEFAULT_BRACKET: i32 = 1 << 18;

static TEST_LOCK: Mutex<()> = Mutex::new(());
static COUNTS: [[AtomicUsize; 2]; EVENTS] = [const { [const { AtomicUsize::new(0) }; 2] }; EVENTS];
static SEQUENCE: [AtomicUsize; 256] = [const { AtomicUsize::new(0) }; 256];
static SEQUENCE_LEN: AtomicUsize = AtomicUsize::new(0);

extern "C" fn observe(event: i32, index: i32) {
    // Static atomics remain valid after the last OS library handle closes.
    let index = if index == -1 { 0 } else { index };
    if let (Ok(event), Ok(index)) = (usize::try_from(event), usize::try_from(index))
        && let Some(cell) = COUNTS.get(event).and_then(|row| row.get(index))
    {
        cell.fetch_add(1, Ordering::SeqCst);
        let position = SEQUENCE_LEN.fetch_add(1, Ordering::SeqCst);
        if let Some(cell) = SEQUENCE.get(position) {
            cell.store(event * 2 + index + 1, Ordering::SeqCst);
        }
    }
}

fn count(event: usize, index: usize) -> usize {
    COUNTS[event][index].load(Ordering::SeqCst)
}

fn snapshot() -> Vec<usize> {
    COUNTS
        .iter()
        .flat_map(|row| row.iter().map(|cell| cell.load(Ordering::SeqCst)))
        .collect()
}

fn guarded() -> std::sync::MutexGuard<'static, ()> {
    let guard = TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    for row in &COUNTS {
        for cell in row {
            cell.store(0, Ordering::SeqCst);
        }
    }
    for cell in &SEQUENCE {
        cell.store(0, Ordering::SeqCst);
    }
    SEQUENCE_LEN.store(0, Ordering::SeqCst);
    guard
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("OFX_FIXTURE_DLL"))
}

fn config() -> VideoProcessConfig {
    VideoProcessConfig::new(
        PixelFormat::Rgba8,
        2,
        2,
        FrameRate::new(30_000, 1001).expect("valid rate"),
    )
    .expect("valid geometry")
}

fn admit(host: &mut OfxHost) -> OfxLibrary {
    fixture_ffi::load(host).expect("real DLL admission")
}

// Copied probe pointers may be used only while the auxiliary handle or a
// host-owned library remains alive. No post-unload assertion calls into C.
#[expect(unsafe_code)]
mod fixture_ffi {
    use super::{OfxError, OfxHost, OfxLibrary, fixture_path, observe};
    use libloading::Library;

    type SetMode = unsafe extern "C" fn(i32);
    type Probe = unsafe extern "C" fn(i32) -> i32;
    type ReadGain = unsafe extern "C" fn(*mut f64, *mut f64) -> i32;
    type CreatedGain = unsafe extern "C" fn(*mut f64) -> i32;
    type LastRenderTime = unsafe extern "C" fn(*mut f64) -> i32;
    type LastRowBytes = unsafe extern "C" fn(*mut i32, *mut i32) -> i32;
    type SrcCorners = unsafe extern "C" fn(*mut u8) -> i32;

    pub struct Fixture {
        auxiliary: Option<Library>,
        set_mode: SetMode,
        probe: Probe,
        read_gain: ReadGain,
        created_gain: CreatedGain,
        last_render_time: LastRenderTime,
        last_rowbytes: LastRowBytes,
        src_corners: SrcCorners,
    }

    impl Fixture {
        pub fn new(mode: i32) -> Self {
            // SAFETY: path names our compiled test fixture; signatures match
            // fixture_control.h. Symbol values remain owned by this library.
            unsafe {
                let library = Library::new(fixture_path()).expect("load fixture helper");
                let configure = library
                    .get::<unsafe extern "C" fn(i32, extern "C" fn(i32, i32))>(
                        b"ofx_fixture_configure\0",
                    )
                    .expect("configure export");
                let set_mode = *library.get(b"ofx_fixture_set_mode\0").expect("mode export");
                let probe = *library.get(b"ofx_fixture_probe\0").expect("probe export");
                let read_gain = *library.get(b"ofx_fixture_read_gain\0").expect("get export");
                let created_gain = *library
                    .get(b"ofx_fixture_created_gain\0")
                    .expect("created value export");
                let last_render_time = *library
                    .get(b"ofx_fixture_last_render_time\0")
                    .expect("render time export");
                let last_rowbytes = *library
                    .get(b"ofx_fixture_last_rowbytes\0")
                    .expect("rowbytes export");
                let src_corners = *library
                    .get(b"ofx_fixture_src_corners\0")
                    .expect("corners export");
                configure(mode, observe);
                Self {
                    auxiliary: Some(library),
                    set_mode,
                    probe,
                    read_gain,
                    created_gain,
                    last_render_time,
                    last_rowbytes,
                    src_corners,
                }
            }
        }

        pub fn release_auxiliary(&mut self) {
            drop(self.auxiliary.take());
        }

        pub fn mode(&self, mode: i32) {
            // SAFETY: each caller keeps the fixture loaded through the host.
            unsafe { (self.set_mode)(mode) };
        }

        pub fn probe(&self, which: i32) {
            // SAFETY: the host still owns an initialized instance and the DLL.
            let result = unsafe { (self.probe)(which) };
            assert_eq!(result, 0, "C suite probe {which} failed at line {result}");
        }

        pub fn gain(&self) -> (f64, f64) {
            let (mut value, mut at_time) = (f64::NAN, f64::NAN);
            // SAFETY: outputs are live writable doubles; host DLL is live.
            assert_eq!(unsafe { (self.read_gain)(&raw mut value, &raw mut at_time) }, 0);
            (value, at_time)
        }

        pub fn created_gain(&self) -> f64 {
            let mut value = f64::NAN;
            // SAFETY: output is a live double and the host DLL is live.
            assert_eq!(unsafe { (self.created_gain)(&raw mut value) }, 0);
            value
        }

        pub fn last_render_time(&self) -> f64 {
            let mut value = f64::NAN;
            // SAFETY: output is a live double and the host DLL is live.
            assert_eq!(unsafe { (self.last_render_time)(&raw mut value) }, 0);
            value
        }

        pub fn last_rowbytes(&self) -> (i32, i32) {
            let (mut src, mut dst) = (0, 0);
            // SAFETY: outputs are live writable ints and the host DLL is live.
            assert_eq!(
                unsafe { (self.last_rowbytes)(&raw mut src, &raw mut dst) },
                0
            );
            (src, dst)
        }

        pub fn src_corners(&self) -> [[u8; 4]; 4] {
            let mut bytes = [0_u8; 16];
            // SAFETY: output is a live 16-byte buffer and the host DLL is live.
            assert_eq!(unsafe { (self.src_corners)(bytes.as_mut_ptr()) }, 0);
            [
                bytes[0..4].try_into().expect("pixel"),
                bytes[4..8].try_into().expect("pixel"),
                bytes[8..12].try_into().expect("pixel"),
                bytes[12..16].try_into().expect("pixel"),
            ]
        }
    }

    pub fn load(host: &mut OfxHost) -> Result<OfxLibrary, OfxError> {
        // SAFETY: only our locally compiled fixture can reach this helper.
        unsafe { host.load_library(&fixture_path()) }
    }

    pub fn load_alias(host: &mut OfxHost) -> Result<OfxLibrary, OfxError> {
        let path = fixture_path().canonicalize().expect("canonical fixture");
        // SAFETY: canonical alias names the same trusted test fixture.
        unsafe { host.load_library(&path) }
    }
}

#[test]
fn fixture_exports_real_compiled_control_abi() {
    let _guard = guarded();
    assert!(fixture_path().metadata().expect("fixture artifact").len() > 0);
    let _fixture = fixture_ffi::Fixture::new(0);
}

#[test]
fn mixed_admission_preserves_native_indexes_and_order() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    fixture.release_auxiliary();
    assert_eq!(library.plugin_count(), 1);
    assert_eq!(library.plugin(0).expect("gain").id, "org.ruviform.gain");
    assert!(library.plugin(1).is_none());
    assert!(library.create_instance(1).is_err());
    let rejected = library.rejections();
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].native_index(), 1);
    assert_eq!(rejected[0].identifier(), Some("org.ruviform.generator"));
    assert!(matches!(rejected[0].error(), OfxError::Unsupported(_)));
    let len = SEQUENCE_LEN.load(Ordering::SeqCst);
    assert!(len <= SEQUENCE.len(), "fixture sequence overflow");
    let events: Vec<_> = SEQUENCE[..len]
        .iter()
        .map(|cell| (cell.load(Ordering::SeqCst) - 1) / 2)
        .collect();
    let position = |event| {
        events
            .iter()
            .position(|value| *value == event)
            .expect("action")
    };
    assert!(position(DLL_SET_HOST) < position(GET_COUNT));
    assert!(position(GET_COUNT) < position(GET_PLUGIN));
    assert!(position(GET_PLUGIN) < position(SET_HOST));
    assert!(position(SET_HOST) < position(LOAD));
    assert!(position(LOAD) < position(DESCRIBE));
    assert!(position(DESCRIBE) < position(DESCRIBE_CONTEXT));
    assert_eq!(count(LOAD, 0), 1);
    assert_eq!(count(LOAD, 1), count(UNLOAD, 1));
    drop(library);
    drop(host);
    assert_eq!(count(UNLOAD, 0), 1);
}

macro_rules! rejected_entry {
    ($name:ident, $mode:expr) => {
        #[test]
        fn $name() {
            let _guard = guarded();
            let mut fixture = fixture_ffi::Fixture::new($mode);
            let mut host = OfxHost::new().expect("host");
            let library = admit(&mut host);
            fixture.release_auxiliary();
            assert_eq!(library.plugin_count(), 0);
            assert_eq!(library.rejections().len(), 2);
            assert_eq!(library.rejections()[0].native_index(), 0);
            assert_eq!(library.rejections()[1].native_index(), 1);
            assert!(library.plugin(0).is_none());
            assert!(library.create_instance(0).is_err());
            drop(library);
            drop(host);
            for index in 0..2 {
                assert_eq!(count(LOAD, index), count(UNLOAD, index));
            }
        }
    };
}

rejected_entry!(invalid_api_rejected_per_entry, INVALID_API);
rejected_entry!(empty_identifier_rejected_per_entry, EMPTY_ID);
rejected_entry!(null_set_host_rejected_per_entry, NULL_SET_HOST);
rejected_entry!(null_main_entry_rejected_per_entry, NULL_MAIN);
rejected_entry!(null_raw_entry_rejected_per_entry, NULL_ENTRY);
rejected_entry!(
    unsupported_context_returns_empty_admitted_library,
    NO_FILTER
);
rejected_entry!(unsupported_parameter_kind_rejected_per_entry, STRING_PARAM);

#[test]
fn library_failures_are_not_successful_empty_admission() {
    let _guard = guarded();
    for mode in [BAD_COUNT, REFUSE_SET_HOST] {
        let _fixture = fixture_ffi::Fixture::new(mode);
        let mut host = OfxHost::new().expect("host");
        assert!(matches!(
            fixture_ffi::load(&mut host),
            Err(OfxError::Library(_) | OfxError::Native { .. })
        ));
        assert_eq!(count(LOAD, 0), 0);
        assert_eq!(count(LOAD, 1), 0);
        drop(host);
    }
}

#[test]
fn second_live_session_is_rejected() {
    let _guard = guarded();
    let first = OfxHost::new().expect("first session");
    assert!(OfxHost::new().is_err());
    drop(first);
    assert!(OfxHost::new().is_ok());
}

#[test]
fn canonical_path_cache_does_not_repeat_native_admission() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let mut host = OfxHost::new().expect("host");
    let first = admit(&mut host);
    fixture.release_auxiliary();
    let before = snapshot();
    let second = fixture_ffi::load_alias(&mut host).expect("canonical alias");
    assert_eq!(second.plugin_count(), 1);
    assert_eq!(snapshot(), before, "cache hit must not call the DLL");
    drop(first);
    drop(second);
    drop(host);
    assert_eq!(count(UNLOAD, 0), 1);
}

#[test]
fn live_instance_owns_native_code_after_root_and_library_drop() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut instance = library.create_instance(0).expect("created wrapper");
    instance.initialize(&config()).expect("native Create");
    fixture.release_auxiliary();
    drop(library);
    drop(host);
    assert_eq!(count(UNLOAD, 0), 0);
    let before = count(CHANGED, 0);
    instance
        .set_parameter(ParamId(0), PlainParamValue::Double(3.0))
        .expect("DLL still alive");
    assert_eq!(count(CHANGED, 0), before + 1);
    assert_eq!(fixture.gain(), (3.0, 3.0));
    instance.destroy().expect("native Destroy");
    drop(instance);
    // Only Rust-owned static atomics are read after potential OS unload.
    assert_eq!(count(DESTROY, 0), 1);
    assert_eq!(count(UNLOAD, 0), 1);
}

#[test]
fn failed_create_preserves_staging_and_can_retry_without_destroy() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut instance = library.create_instance(0).expect("created");
    fixture.release_auxiliary();
    let staged = PlainParamValue::Double(3.0);
    instance.set_parameter(ParamId(0), staged).expect("stage");
    assert_eq!(count(CHANGED, 0), 0);
    fixture.mode(FAIL_CREATE);
    assert!(instance.initialize(&config()).is_err());
    assert_eq!(instance.state(), LifecycleState::Created);
    assert_eq!(
        instance.parameter_value(ParamId(0)).expect("staged"),
        staged
    );
    assert_eq!(count(CREATE_ENTER, 0), 1);
    assert_eq!(count(CREATE_OK, 0), 0);
    assert_eq!(count(DESTROY, 0), 0);
    fixture.mode(0);
    instance.initialize(&config()).expect("retry");
    assert_eq!(instance.state(), LifecycleState::Initialized);
    assert_eq!(fixture.created_gain(), 3.0);
    assert_eq!(count(CREATE_ENTER, 0), 2);
    assert_eq!(count(CREATE_OK, 0), 1);
    instance.destroy().expect("destroy successful Create");
    assert_eq!(count(DESTROY, 0), 1);
}

#[test]
fn single_instance_is_reserved_at_initialize_not_wrapper_creation() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(SINGLE_INSTANCE);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut first = library.create_instance(0).expect("first Created wrapper");
    let mut second = library.create_instance(0).expect("second Created wrapper");
    fixture.release_auxiliary();
    first.initialize(&config()).expect("first native Create");
    let creates = count(CREATE_ENTER, 0);
    assert!(second.initialize(&config()).is_err());
    assert_eq!(count(CREATE_ENTER, 0), creates);
    assert_eq!(second.state(), LifecycleState::Created);
    first.destroy().expect("release reservation");
    second.initialize(&config()).expect("new reservation");
    assert_eq!(count(CREATE_ENTER, 0), creates + 1);
    second.destroy().expect("destroy second");
}

#[test]
fn failed_destroy_is_terminal_and_quarantines_single_instance() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(SINGLE_INSTANCE);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut first = library.create_instance(0).expect("first");
    let mut second = library.create_instance(0).expect("second");
    fixture.release_auxiliary();
    first.initialize(&config()).expect("first native Create");
    fixture.mode(SINGLE_INSTANCE | FAIL_DESTROY);
    assert!(first.destroy().is_err());
    assert_eq!(first.state(), LifecycleState::Destroyed);
    assert_eq!(count(DESTROY, 0), 1);
    let before = snapshot();
    assert!(first.destroy().is_err());
    assert!(second.initialize(&config()).is_err());
    assert_eq!(
        snapshot(),
        before,
        "uncertain native instance remains reserved"
    );
    drop(first);
    drop(second);
    drop(library);
    drop(host);
    assert_eq!(count(DESTROY, 0), 1);
    assert_eq!(count(UNLOAD, 0), 1);
}

#[test]
fn lifecycle_rejections_do_not_enter_native_code() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut instance = library.create_instance(0).expect("created");
    fixture.release_auxiliary();
    let before = snapshot();
    assert!(instance.activate().is_err());
    assert!(instance.start_processing().is_err());
    assert!(instance.stop_processing().is_err());
    assert!(instance.deactivate().is_err());
    assert_eq!(snapshot(), before);
    instance.initialize(&config()).expect("initialize");
    assert_eq!(instance.state(), LifecycleState::Initialized);
    let before = snapshot();
    assert!(instance.initialize(&config()).is_err());
    assert!(instance.start_processing().is_err());
    assert_eq!(snapshot(), before);
    fixture.mode(FAIL_CLIP_PREFS);
    assert!(instance.activate().is_err());
    assert_eq!(instance.state(), LifecycleState::Initialized);
    assert_eq!(count(CLIP_PREFS, 0), 1);
    fixture.mode(0);
    instance.activate().expect("activation retry");
    assert_eq!(instance.state(), LifecycleState::Activated);
    instance.start_processing().expect("start");
    assert_eq!(instance.state(), LifecycleState::Processing);
    instance.stop_processing().expect("stop");
    assert_eq!(instance.state(), LifecycleState::Activated);
    instance.deactivate().expect("deactivate");
    assert_eq!(instance.state(), LifecycleState::Initialized);
    instance.destroy().expect("destroy");
    assert_eq!(instance.state(), LifecycleState::Destroyed);
    let before = snapshot();
    assert_ne!(instance.descriptor().id, "");
    assert!(instance.parameter(ParamId(0)).is_some());
    assert!(instance.parameter_value(ParamId(0)).is_err());
    assert!(
        instance
            .set_parameter(ParamId(0), PlainParamValue::Double(1.0))
            .is_err()
    );
    assert!(instance.destroy().is_err());
    assert_eq!(snapshot(), before);
}

#[test]
fn created_only_destroy_never_calls_native_destroy() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut instance = library.create_instance(0).expect("created");
    fixture.release_auxiliary();
    let before = snapshot();
    instance.destroy().expect("destroy wrapper only");
    assert_eq!(instance.state(), LifecycleState::Destroyed);
    assert_eq!(snapshot(), before);
}

#[test]
fn neutral_configuration_is_restricted_before_native_create() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut instance = library.create_instance(0).expect("created");
    fixture.release_auxiliary();
    let unsupported = VideoProcessConfig::new(PixelFormat::Yuv420p, 3, 2, config().frame_rate())
        .expect("neutral core accepts format");
    let before = snapshot();
    assert!(instance.initialize(&unsupported).is_err());
    assert_eq!(instance.state(), LifecycleState::Created);
    assert_eq!(snapshot(), before);
    instance
        .initialize(&config())
        .expect("supported config retry");
}

#[test]
fn plain_double_roundtrip_preserves_actual_c_values() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut instance = library.create_instance(0).expect("created");
    fixture.release_auxiliary();
    instance.initialize(&config()).expect("initialize");
    instance.activate().expect("activate");
    instance.start_processing().expect("start");
    let gain = ParamId(0);
    assert_eq!(fixture.gain(), (1.0, 1.0));
    let value = instance.parameter_value(gain).expect("plain read");
    assert_eq!(value, PlainParamValue::Double(1.0));
    instance
        .set_parameter(gain, value)
        .expect("unchanged write-back");
    assert_eq!(fixture.gain(), (1.0, 1.0));
    for value in [0.0, 1.0, 3.0] {
        let before = count(CHANGED, 0);
        instance
            .set_parameter(gain, PlainParamValue::Double(value))
            .expect("plain set");
        assert_eq!(fixture.gain(), (value, value));
        assert_eq!(
            instance.parameter_value(gain).expect("read"),
            PlainParamValue::Double(value)
        );
        assert_eq!(count(CHANGED, 0), before + 1);
    }
    assert_eq!(count(BEGIN_CHANGED, 0), count(END_CHANGED, 0));
    for bad in [
        PlainParamValue::Double(f64::NAN),
        PlainParamValue::Double(f64::INFINITY),
        PlainParamValue::Double(f64::NEG_INFINITY),
        PlainParamValue::Double(2e20),
        PlainParamValue::Integer(1),
    ] {
        let before = snapshot();
        assert!(instance.set_parameter(gain, bad).is_err());
        assert_eq!(snapshot(), before);
        assert_eq!(fixture.gain(), (3.0, 3.0));
    }
    let before = snapshot();
    assert!(
        instance
            .set_parameter(ParamId(1), PlainParamValue::Choice(2))
            .is_err()
    );
    assert!(
        instance
            .set_parameter(ParamId(2), PlainParamValue::Integer(11))
            .is_err()
    );
    assert_eq!(snapshot(), before);
    assert_eq!(
        instance.parameter_value(ParamId(1)).expect("choice"),
        PlainParamValue::Choice(1)
    );
    assert_eq!(
        instance.parameter_value(ParamId(2)).expect("integer"),
        PlainParamValue::Integer(0)
    );
}

#[test]
fn finite_wide_endpoints_do_not_require_finite_domain_width() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(WIDE_BOUNDS);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut instance = library.create_instance(0).expect("created");
    fixture.release_auxiliary();
    let metadata = instance.parameter(ParamId(0)).expect("gain");
    assert_eq!(metadata.min_value, -f64::MAX);
    assert_eq!(metadata.max_value, f64::MAX);
    instance
        .initialize(&config())
        .expect("finite endpoints accepted");
    instance
        .set_parameter(ParamId(0), PlainParamValue::Double(3.0))
        .expect("valid plain value");
    assert_eq!(fixture.gain(), (3.0, 3.0));
}

macro_rules! suite_probe {
    ($name:ident, $probe:expr, $mode:expr) => {
        #[test]
        fn $name() {
            let _guard = guarded();
            let mut fixture = fixture_ffi::Fixture::new($mode);
            let mut host = OfxHost::new().expect("host");
            let library = admit(&mut host);
            let mut instance = library.create_instance(0).expect("created");
            fixture.release_auxiliary();
            instance.initialize(&config()).expect("initialize");
            fixture.probe($probe);
        }
    };
}

suite_probe!(property_suite_typed_scalar_n_reset_and_errors, 0, 0);
suite_probe!(parameter_suite_varargs_keys_and_unsupported_slots, 1, 0);
suite_probe!(
    memory_suites_own_aligned_storage_and_reject_stale_handles,
    2, 0
);
suite_probe!(
    foreign_os_worker_calls_suites_without_control_plane_access,
    3, WORKER_ON_CREATE
);

#[test]
fn readonly_is_a_host_ui_restriction_not_a_native_write_prohibition() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut instance = library.create_instance(0).expect("created");
    fixture.release_auxiliary();
    instance.initialize(&config()).expect("initialize");
    let readonly = ParamId(4);
    let before = snapshot();
    assert!(
        instance
            .set_parameter(readonly, PlainParamValue::Double(4.0))
            .is_err()
    );
    assert_eq!(snapshot(), before);
    assert_eq!(
        instance.parameter_value(readonly).expect("default"),
        PlainParamValue::Double(2.0)
    );
    fixture.probe(4);
    assert_eq!(
        instance.parameter_value(readonly).expect("native changed"),
        PlainParamValue::Double(4.0)
    );
}

#[test]
fn generic_ui_consumes_borrowed_metadata_and_plain_values() {
    fn consume<P: VideoPluginInstance>(plugin: &mut P)
    where
        P::Error: std::fmt::Debug,
    {
        use ruviform_plugin_core::ParamFlags;
        let ids = plugin.parameters().to_vec();
        assert_eq!(ids.len(), 5);
        let choice = plugin.parameter(ids[1]).expect("choice metadata");
        assert_eq!(choice.choices, &["a", "b"]);
        assert_eq!(choice.default_value, PlainParamValue::Choice(1));
        let gain = plugin.parameter(ids[0]).expect("gain metadata");
        assert_eq!((gain.min_value, gain.max_value), (-1e20, 1e20));
        assert_eq!((gain.display_min_value, gain.display_max_value), (0.0, 2.0));
        for id in ids {
            let info = plugin.parameter(id).expect("metadata");
            assert_ne!(info.name, "");
            if !info.flags.contains(ParamFlags::READONLY) {
                let value = plugin.parameter_value(id).expect("plain get");
                plugin
                    .set_parameter(id, value)
                    .expect("same-value write-back");
                assert_eq!(plugin.parameter_value(id).expect("read"), value);
            }
        }
        plugin
            .set_parameter(ParamId(0), PlainParamValue::Double(3.0))
            .expect("outside UI range");
        assert_eq!(
            plugin.parameter_value(ParamId(0)).expect("read"),
            PlainParamValue::Double(3.0)
        );
    }
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut instance = library.create_instance(0).expect("created");
    fixture.release_auxiliary();
    consume(&mut instance);
    assert_eq!(count(CHANGED, 0), 0, "Created writes are staged only");
    instance.initialize(&config()).expect("initialize");
    assert_eq!(fixture.created_gain(), 3.0);
}

/* ---------- per-frame render contract ---------- */

const PTS: MediaTime = MediaTime::ZERO;

/// 2×2 RGBA8 frame at stride 16 (`row_bytes` 8) — the padding bytes between
/// rows carry `0xAA` sentinels the plugin must never observe or produce.
fn padded_frame(fill: &[u8]) -> VideoFrame {
    let mut data = vec![0xAA_u8; 24];
    data[0..8].copy_from_slice(&fill[0..8]);
    data[16..24].copy_from_slice(&fill[8..16]);
    VideoFrame::from_shared_planes(
        PixelFormat::Rgba8,
        2,
        2,
        PTS,
        vec![SharedPlane {
            data: Arc::from(data.into_boxed_slice()),
            stride: 16,
        }],
    )
    .expect("frame geometry")
}

/// Top-left, top-right, bottom-left, bottom-right pixels — video frames
/// are top-row-first; the OFX bottom-left contract flips the exposure.
fn pattern_input() -> VideoFrame {
    padded_frame(&[
        10, 20, 30, 255, 40, 50, 60, 255, // top row
        70, 80, 90, 255, 100, 110, 120, 255, // bottom row
    ])
}

fn row(frame: &VideoFrame, y: u32) -> Vec<u8> {
    frame
        .plane(0)
        .expect("plane 0")
        .row(y)
        .expect("row")
        .to_vec()
}

fn padding(frame: &VideoFrame) -> &[u8] {
    &frame.plane(0).expect("plane 0").data()[8..16]
}

fn processing_instance(
    fixture: &fixture_ffi::Fixture,
    mode: i32,
) -> (OfxHost, OfxLibrary, ruviform_plugin_ofx::OfxInstance) {
    let mut host = OfxHost::new().expect("host");
    let library = admit(&mut host);
    let mut instance = library.create_instance(0).expect("created");
    fixture.mode(mode);
    instance.initialize(&config()).expect("initialize");
    instance.activate().expect("activate");
    instance.start_processing().expect("start");
    (host, library, instance)
}

#[test]
fn render_bracket_maps_frames_and_time_bottom_left() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let (host, library, mut instance) = processing_instance(&fixture, 0);
    let prefs_before = count(CLIP_PREFS, 0);
    instance
        .set_parameter(ParamId(0), PlainParamValue::Double(2.0))
        .expect("live gain write");

    let input = pattern_input();
    let mut output = padded_frame(&[0_u8; 16]);
    let at = MediaTime::new(45_000, TimeBase::COMMON_90K);
    instance.render(&input, &mut output, at).expect("render");
    fixture.release_auxiliary();

    // gain 2.0 applied per RGB channel: out = in * 2 (+0.5 truncates);
    // alpha is copied unscaled.
    assert_eq!(row(&output, 0), vec![20, 40, 60, 255, 80, 100, 120, 255]);
    assert_eq!(
        row(&output, 1),
        vec![140, 160, 180, 255, 200, 220, 240, 255]
    );
    assert_eq!(padding(&output), &[0xAA; 8], "row padding untouched");

    // Bottom-left exposure: image (0,0) is the frame's bottom-left pixel,
    // and rowBytes is the negated padded staging stride.
    assert_eq!(
        fixture.src_corners(),
        [
            [70, 80, 90, 255],
            [100, 110, 120, 255],
            [10, 20, 30, 255],
            [40, 50, 60, 255],
        ]
    );
    assert_eq!(fixture.last_rowbytes(), (-16, -16));
    assert_eq!(fixture.last_render_time(), 0.5);

    // Transactional ordering: clip-pref revalidation precedes one
    // Begin -> Render -> End bracket per call.
    assert_eq!(count(CLIP_PREFS, 0), prefs_before + 1);
    assert_eq!(count(BEGIN_RENDER, 0), 1);
    assert_eq!(count(RENDER, 0), 1);
    assert_eq!(count(END_RENDER, 0), 1);
    let len = SEQUENCE_LEN.load(Ordering::SeqCst);
    let events: Vec<_> = SEQUENCE[..len]
        .iter()
        .map(|cell| (cell.load(Ordering::SeqCst) - 1) / 2)
        .collect();
    let position = |event| events.iter().position(|value| *value == event);
    assert!(
        position(BEGIN_RENDER) < position(RENDER)
            && position(RENDER) < position(END_RENDER)
    );
    assert_eq!(instance.state(), LifecycleState::Processing);
    drop(library);
    drop(host);
}

#[test]
fn render_rejections_happen_before_any_native_entry() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let (host, library, mut instance) = processing_instance(&fixture, 0);
    fixture.release_auxiliary();
    let input = pattern_input();
    let mut output = padded_frame(&[0_u8; 16]);
    let at = MediaTime::new(0, TimeBase::COMMON_90K);

    let before = count(BEGIN_RENDER, 0);
    let wrong = VideoFrame::new_owned(PixelFormat::Rgba8, 4, 4, PTS).expect("other size");
    let mut wrong_out = wrong.clone();
    assert!(instance.render(&wrong, &mut wrong_out, at).is_err());
    let yuv = VideoFrame::new_owned(PixelFormat::Yuv420p, 2, 2, PTS).expect("yuv");
    let mut yuv_out = VideoFrame::new_owned(PixelFormat::Yuv420p, 2, 2, PTS).expect("yuv");
    assert!(instance.render(&yuv, &mut yuv_out, at).is_err());
    assert_eq!(count(BEGIN_RENDER, 0), before);

    instance.stop_processing().expect("stop");
    let before_seq = snapshot();
    assert!(instance.render(&input, &mut output, at).is_err());
    assert_eq!(snapshot(), before_seq);
    assert_eq!(instance.state(), LifecycleState::Activated);
    drop(library);
    drop(host);
}

#[test]
fn render_revalidates_clip_preferences_before_every_bracket() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let (host, library, mut instance) = processing_instance(&fixture, 0);
    fixture.release_auxiliary();
    let input = pattern_input();
    let mut output = padded_frame(&[0xAA_u8; 16]);
    let at = MediaTime::new(0, TimeBase::COMMON_90K);

    // enabled = 0 makes GetClipPreferences offer RGB output — outside the
    // fixed RGBA8 contract — which must fail before the bracket opens.
    instance
        .set_parameter(ParamId(3), PlainParamValue::Boolean(false))
        .expect("enabled write");
    let prefs_before = count(CLIP_PREFS, 0);
    let result = instance.render(&input, &mut output, at);
    assert!(
        matches!(result, Err(OfxError::Unsupported(_))),
        "unsupported offer must fail: {result:?}"
    );
    assert_eq!(count(CLIP_PREFS, 0), prefs_before + 1);
    assert_eq!(count(BEGIN_RENDER, 0), 0);
    assert_eq!(count(RENDER, 0), 0);
    assert_eq!(count(END_RENDER, 0), 0);
    assert_eq!(instance.state(), LifecycleState::Processing);
    assert_eq!(
        output.plane(0).expect("plane").data(),
        &[0xAA; 24],
        "untouched output"
    );
    drop(library);
    drop(host);
}

#[test]
fn failed_begin_render_skips_bracket_and_keeps_output_untouched() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let (host, library, mut instance) = processing_instance(&fixture, 0);
    fixture.release_auxiliary();
    fixture.mode(FAIL_BEGIN_RENDER);
    let input = pattern_input();
    let mut output = padded_frame(&[0xAA_u8; 16]);
    assert!(
        instance
            .render(&input, &mut output, MediaTime::new(0, TimeBase::COMMON_90K))
            .is_err()
    );
    assert_eq!(count(BEGIN_RENDER, 0), 1);
    assert_eq!(count(RENDER, 0), 0);
    assert_eq!(count(END_RENDER, 0), 0, "a failed Begin opens no bracket");
    assert_eq!(output.plane(0).expect("plane").data(), &[0xAA; 24]);
    assert_eq!(instance.state(), LifecycleState::Processing);
    fixture.mode(0);
    instance
        .render(&input, &mut output, MediaTime::new(0, TimeBase::COMMON_90K))
        .expect("brackets are independent per call");
    assert_eq!(count(END_RENDER, 0), 1);
    drop(library);
    drop(host);
}

#[test]
fn failed_render_still_closes_bracket_and_keeps_output_untouched() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let (host, library, mut instance) = processing_instance(&fixture, 0);
    fixture.release_auxiliary();
    fixture.mode(FAIL_RENDER);
    let input = pattern_input();
    let mut output = padded_frame(&[0xAA_u8; 16]);
    assert!(
        instance
            .render(&input, &mut output, MediaTime::new(0, TimeBase::COMMON_90K))
            .is_err()
    );
    assert_eq!(count(BEGIN_RENDER, 0), 1);
    assert_eq!(count(RENDER, 0), 1);
    assert_eq!(count(END_RENDER, 0), 1, "an open bracket is always closed");
    assert_eq!(output.plane(0).expect("plane").data(), &[0xAA; 24]);
    assert_eq!(instance.state(), LifecycleState::Processing);
    drop(library);
    drop(host);
}

#[test]
fn failed_end_render_propagates_and_keeps_output_untouched() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let (host, library, mut instance) = processing_instance(&fixture, 0);
    fixture.release_auxiliary();
    fixture.mode(FAIL_END_RENDER);
    let input = pattern_input();
    let mut output = padded_frame(&[0xAA_u8; 16]);
    assert!(
        instance
            .render(&input, &mut output, MediaTime::new(0, TimeBase::COMMON_90K))
            .is_err()
    );
    assert_eq!(count(BEGIN_RENDER, 0), 1);
    assert_eq!(count(RENDER, 0), 1);
    assert_eq!(count(END_RENDER, 0), 1);
    assert_eq!(output.plane(0).expect("plane").data(), &[0xAA; 24]);
    assert_eq!(instance.state(), LifecycleState::Processing);
    drop(library);
    drop(host);
}

#[test]
fn render_destroy_and_release_stay_consistent_after_brackets() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let (host, library, mut instance) = processing_instance(&fixture, 0);
    fixture.release_auxiliary();
    let input = pattern_input();
    let mut output = padded_frame(&[0_u8; 16]);
    let at = MediaTime::new(0, TimeBase::COMMON_90K);
    instance.render(&input, &mut output, at).expect("first");
    instance.render(&input, &mut output, at).expect("second");
    assert_eq!(count(BEGIN_RENDER, 0), 2);
    assert_eq!(count(RENDER, 0), 2);
    assert_eq!(count(END_RENDER, 0), 2);
    instance.stop_processing().expect("stop");
    instance.deactivate().expect("deactivate");
    instance.destroy().expect("destroy");
    drop(instance);
    drop(library);
    drop(host);
    assert_eq!(count(DESTROY, 0), 1);
    assert_eq!(count(UNLOAD, 0), 1);
}

#[test]
fn reply_default_bracket_is_a_legal_render() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let (host, library, mut instance) = processing_instance(&fixture, 0);
    // Begin/EndSequenceRender are optional actions — ReplyDefault means
    // "not trapped, but all is well anyway", not a bracket failure.
    fixture.mode(REPLY_DEFAULT_BRACKET);
    fixture.release_auxiliary();
    let input = pattern_input();
    let mut output = padded_frame(&[0_u8; 16]);
    instance
        .render(&input, &mut output, MediaTime::new(0, TimeBase::COMMON_90K))
        .expect("ReplyDefault bracket is a legal render");

    // Default gain 1.0: out = in * 1.0 + 0.5 truncates to the input pixels.
    assert_eq!(row(&output, 0), vec![10, 20, 30, 255, 40, 50, 60, 255]);
    assert_eq!(row(&output, 1), vec![70, 80, 90, 255, 100, 110, 120, 255]);
    assert_eq!(count(BEGIN_RENDER, 0), 1);
    assert_eq!(count(RENDER, 0), 1);
    assert_eq!(count(END_RENDER, 0), 1);
    assert_eq!(instance.state(), LifecycleState::Processing);
    drop(library);
    drop(host);
}

#[test]
fn render_rejects_float_depth_offer_and_recovers() {
    let _guard = guarded();
    let mut fixture = fixture_ffi::Fixture::new(0);
    let (host, library, mut instance) = processing_instance(&fixture, 0);
    fixture.release_auxiliary();
    let input = pattern_input();
    let mut output = padded_frame(&[0xAA_u8; 16]);
    let at = MediaTime::new(0, TimeBase::COMMON_90K);

    // mode choice 0 makes GetClipPreferences offer float depth — outside
    // the byte contract — which must fail before the bracket opens.
    instance
        .set_parameter(ParamId(1), PlainParamValue::Choice(0))
        .expect("mode write");
    let result = instance.render(&input, &mut output, at);
    assert!(
        matches!(result, Err(OfxError::Unsupported(_))),
        "float depth offer must fail: {result:?}"
    );
    assert_eq!(count(BEGIN_RENDER, 0), 0);
    assert_eq!(count(RENDER, 0), 0);
    assert_eq!(count(END_RENDER, 0), 0);
    assert_eq!(instance.state(), LifecycleState::Processing);
    assert_eq!(output.plane(0).expect("plane").data(), &[0xAA; 24]);

    // Restoring the byte-conformant offer renders on the next call.
    instance
        .set_parameter(ParamId(1), PlainParamValue::Choice(1))
        .expect("restore mode");
    instance
        .render(&input, &mut output, at)
        .expect("recovered render");
    assert_eq!(count(END_RENDER, 0), 1);
    assert_eq!(instance.state(), LifecycleState::Processing);
    drop(library);
    drop(host);
}
