//! Video plugin contract tests — neutral core boundary (Issue #98 Task 1).
//! Contract presence + typed-plain channel semantics; no OFX/SDK types allowed here.

use ruviform_media_core::{FrameRate, MediaTime};
use ruviform_plugin_core::{
    LifecycleState, ParamFlags, ParamId, PlainParamInfo, PlainParamValue, PluginDescriptor,
    PluginDescriptorView, VideoPluginInstance, VideoProcessConfig,
};
use ruviform_video_core::{PixelFormat, VideoFrame};

fn rate() -> FrameRate {
    FrameRate::new(30, 1).expect("valid rate")
}

#[test]
fn descriptor_view_borrows_all_fields() {
    const D: PluginDescriptor = PluginDescriptor {
        id: "org.example.d",
        name: "D",
        vendor: "V",
        version: "0.1.0",
        features: &["video-effect"],
    };
    let v: PluginDescriptorView<'_> = D.view();
    assert_eq!(v.id, "org.example.d");
    assert_eq!(v.name, "D");
    assert_eq!(v.vendor, "V");
    assert_eq!(v.version, "0.1.0");
    assert_eq!(v.features, &["video-effect"]);
}

#[test]
fn descriptor_view_borrows_owned_strings() {
    let id = String::from("org.example.dynamic");
    let name = String::from("Dynamic");
    let vendor = String::from("V");
    let version = String::from("2.0.0");
    let features = [String::from("f1"), String::from("f2")];
    let fv: Vec<&str> = features.iter().map(String::as_str).collect();
    let view = PluginDescriptorView {
        id: &id,
        name: &name,
        vendor: &vendor,
        version: &version,
        features: &fv,
    };
    assert_eq!(view.id, "org.example.dynamic");
    assert_eq!(view.features, &["f1", "f2"]);
}

#[test]
fn plain_param_value_stable_copy_variants() {
    let d1 = PlainParamValue::Double(1.0);
    let d0 = PlainParamValue::Double(0.0);
    let copy = d1; // Copy
    assert_eq!(d1, copy);
    assert_ne!(d1, d0);
    assert_eq!(PlainParamValue::Integer(7), PlainParamValue::Integer(7));
    assert_eq!(
        PlainParamValue::Boolean(true),
        PlainParamValue::Boolean(true)
    );
    assert_eq!(PlainParamValue::Choice(2), PlainParamValue::Choice(2));
    // wide finite values pass through unnormalized
    let wide = PlainParamValue::Double(1e20);
    assert_eq!(wide, PlainParamValue::Double(1e20));
    let neg = PlainParamValue::Double(-1e20);
    assert_eq!(neg, PlainParamValue::Double(-1e20));
    // raw NaN construction is possible (raw type; finite enforcement is setter-side)
    let nan = PlainParamValue::Double(f64::NAN);
    match nan {
        PlainParamValue::Double(v) => assert!(v.is_nan()),
        _ => unreachable!(),
    }
}

#[test]
fn plain_param_info_double_metadata_contract() {
    let def = PlainParamValue::Double(3.0);
    let flags = ParamFlags::default();
    let info = PlainParamInfo {
        id: ParamId(7),
        name: "gain",
        unit: "dB",
        min_value: -1e20,
        max_value: 1e20,
        display_min_value: 0.0,
        display_max_value: 2.0,
        default_value: def,
        flags,
        choices: &[],
    };
    assert_eq!(info.id, ParamId(7));
    assert_eq!(info.name, "gain");
    assert_eq!(info.unit, "dB");
    assert_eq!(info.min_value, -1e20);
    assert_eq!(info.max_value, 1e20);
    assert_eq!(info.display_min_value, 0.0);
    assert_eq!(info.display_max_value, 2.0);
    assert_eq!(info.default_value, def);
    assert_eq!(info.flags, flags);
    assert_eq!(info.choices, &[] as &[String]); // non-choice kind -> empty
}

#[test]
fn plain_param_info_choice_metadata_contract() {
    let labels = [String::from("a"), String::from("b")];
    let def = PlainParamValue::Choice(1);
    let flags = ParamFlags::default();
    let info = PlainParamInfo {
        id: ParamId(8),
        name: "mode",
        unit: "",
        min_value: 0.0,
        max_value: 1.0,
        display_min_value: 0.0,
        display_max_value: 1.0,
        default_value: def,
        flags,
        choices: &labels,
    };
    assert_eq!(info.id, ParamId(8));
    assert_eq!(info.name, "mode");
    assert_eq!(info.unit, "");
    assert_eq!(info.min_value, 0.0);
    assert_eq!(info.max_value, 1.0);
    assert_eq!(info.display_min_value, 0.0);
    assert_eq!(info.display_max_value, 1.0);
    assert_eq!(info.default_value, PlainParamValue::Choice(1));
    assert_eq!(info.flags, flags);
    assert_eq!(info.choices.len(), 2);
    assert_eq!(info.choices[0], "a");
    assert_eq!(info.choices[1], "b");
}

#[test]
fn video_process_config_accepts_neutral_formats() {
    // Neutral core validates nonzero dims only — adapter restricts formats.
    let rate = FrameRate::new(30_000, 1001).expect("valid rate");
    let c = VideoProcessConfig::new(PixelFormat::Yuv420p, 3, 2, rate).expect("nonzero dims");
    let copy = c; // Copy
    assert_eq!(copy.format(), PixelFormat::Yuv420p);
    assert_eq!(copy.width(), 3);
    assert_eq!(copy.height(), 2);
    assert_eq!(copy.frame_rate(), rate);
    assert_eq!(c, copy); // original still usable — E0382 if Copy is lost
}

#[test]
fn video_process_config_validates_nonzero_dims() {
    assert!(VideoProcessConfig::new(PixelFormat::Rgba8, 0, 2, rate()).is_err());
    assert!(VideoProcessConfig::new(PixelFormat::Rgba8, 2, 0, rate()).is_err());
    let c = VideoProcessConfig::new(PixelFormat::Rgba8, 2, 2, rate()).expect("ok");
    assert_eq!(c.format(), PixelFormat::Rgba8);
    assert_eq!(c.width(), 2);
    assert_eq!(c.height(), 2);
    assert_eq!(c.frame_rate(), rate());
}

// minimal generic consumer: implements the trait using only core/media/video types
struct Stub;
#[derive(Debug)]
struct StubErr;
impl std::fmt::Display for StubErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stub")
    }
}
impl std::error::Error for StubErr {}

impl VideoPluginInstance for Stub {
    type Error = StubErr;
    fn state(&self) -> LifecycleState {
        LifecycleState::Created
    }
    fn descriptor(&self) -> PluginDescriptorView<'_> {
        PluginDescriptorView {
            id: "",
            name: "",
            vendor: "",
            version: "",
            features: &[],
        }
    }
    fn parameters(&self) -> &[ParamId] {
        &[]
    }
    fn parameter(&self, _id: ParamId) -> Option<PlainParamInfo<'_>> {
        None
    }
    fn parameter_value(&self, _id: ParamId) -> Result<PlainParamValue, Self::Error> {
        Err(StubErr)
    }
    fn set_parameter(&mut self, _id: ParamId, _v: PlainParamValue) -> Result<(), Self::Error> {
        Err(StubErr)
    }
    fn initialize(&mut self, _c: &VideoProcessConfig) -> Result<(), Self::Error> {
        Err(StubErr)
    }
    fn activate(&mut self) -> Result<(), Self::Error> {
        Err(StubErr)
    }
    fn start_processing(&mut self) -> Result<(), Self::Error> {
        Err(StubErr)
    }
    fn render(
        &mut self,
        _i: &VideoFrame,
        _o: &mut VideoFrame,
        _at: MediaTime,
    ) -> Result<(), Self::Error> {
        Err(StubErr)
    }
    fn stop_processing(&mut self) -> Result<(), Self::Error> {
        Err(StubErr)
    }
    fn deactivate(&mut self) -> Result<(), Self::Error> {
        Err(StubErr)
    }
    fn destroy(&mut self) -> Result<(), Self::Error> {
        Err(StubErr)
    }
}

#[test]
fn generic_consumer_compiles_without_ofx_types() {
    let s = Stub;
    assert_eq!(s.state(), LifecycleState::Created);
    assert_eq!(s.parameters(), &[] as &[ParamId]);
}
