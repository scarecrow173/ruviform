//! Reference L1 video runner binary
//! (`ruviform-plugin-l1-video-runner`) — the spawn fixture for the
//! `ruviform-plugin-export` video E2E tests
//! (`tests/l1_video_runner_e2e.rs`, via `CARGO_BIN_EXE_*`).
//!
//! It embeds four mock video factories:
//!
//! - `InvertVideo` (`dev.ruviform.mock.video.invert`) — a
//!   deterministic pixel transform: every output byte is the input
//!   byte inverted (`Mode` `Invert`, the default) or a flat `0x80`
//!   fill (`Mode` `Fill`) — the host-side E2E pixel oracle. It
//!   carries a small plain-parameter surface (`Amount`, `Mode`,
//!   `EchoWidth`, `FailDeactivate`, `FailRender`): `EchoWidth` reads back the
//!   width of the config `initialize` received, so the stash/replay
//!   of `CreateVideoInstance`'s config is wire-observable,
//!   `FailDeactivate` arms a defined `deactivate` error so the
//!   commit-then-advisory transition is testable, `FailRender` arms
//!   a defined `render` error, and `RUVIFORM_PANIC_RENDER` makes
//!   `render` panic — the mid-processing panic fixture.
//! - `PanicVideo` (`dev.ruviform.mock.video.panic`) — panics inside
//!   `initialize`; the deterministic fixture for the panic →
//!   `Err(HostError::PluginFailed)` contract (the child must survive
//!   and keep serving commands). Its *factory* `descriptor()` panics
//!   when
//!   `RUVIFORM_PANIC_DESCRIPTOR` is set, and `create()` panics when
//!   `RUVIFORM_PANIC_CREATE` is set — the fixtures for guarded
//!   factory calls (`Scan`/`CreateVideoInstance` →
//!   `Err(HostError::PluginFailed)` on the wire, child alive).
//! - `OverLimitVideo` (`dev.ruviform.mock.video.overlimit`) —
//!   reports `WIRE_MAX_ELEMENTS + 1` parameter ids: the encode
//!   obligation (c) fixture (`parameters()` →
//!   `Err(HostError::PluginFailed)`, never truncated).
//! - `OverLimitInfoVideo`
//!   (`dev.ruviform.mock.video.overlimit-info`) — `parameter()`
//!   reports metadata the wire cannot carry verbatim: id 0 has
//!   `WIRE_MAX_ELEMENTS + 1` choices, id 1 a
//!   `WIRE_MAX_STRING_BYTES + 1` name (both →
//!   `Err(HostError::PluginFailed)`); id 2 has non-finite
//!   `display_*` hints the adapter clamps into `[min,max]`
//!   (obligation (d) — observable through `VideoGetParamInfo`).
//!
//! Wire v3 takes no spawn arguments — the binary is spawned bare and
//! the host picks a plugin id per `CreateVideoInstance`.

use std::process::ExitCode;

use ruviform_media_core::MediaTime;
use ruviform_plugin_core::{
    ErasedVideoPlugin, Lifecycle, LifecycleState, ParamFlags, ParamId, PlainParamInfo,
    PlainParamValue, PluginDescriptor, PluginDescriptorView, VideoPluginFactory,
    VideoPluginInstance, VideoProcessConfig, erase_video,
};
use ruviform_video_core::VideoFrame;

fn main() -> ExitCode {
    ruviform_plugin_export::run_video(vec![
        Box::new(InvertFactory),
        Box::new(PanicFactory),
        Box::new(OverLimitFactory),
        Box::new(OverLimitInfoFactory),
    ])
}

/// The mock plugins' error type — any `Error` implementor works;
/// `erase_video` normalizes it to `HostError::PluginFailed`.
#[derive(Debug)]
struct MockError(&'static str);

impl std::fmt::Display for MockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for MockError {}

const INVERT_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.video.invert",
    name: "Invert",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["video-effect"],
};

const PARAM_AMOUNT: u32 = 0;
const PARAM_MODE: u32 = 1;
const PARAM_ECHO_WIDTH: u32 = 2;
const PARAM_FAIL_DEACTIVATE: u32 = 3;
const PARAM_FAIL_RENDER: u32 = 4;

/// The deterministic invert/fill pixel transform — the E2E's pixel
/// oracle — plus a small parameter surface that makes the runner's
/// stash/replay and defined rejections observable over the wire.
struct InvertVideo {
    lifecycle: Lifecycle,
    ids: Vec<ParamId>,
    mode_choices: Vec<String>,
    config: Option<VideoProcessConfig>,
    amount: f64,
    mode: u32,
    fail_deactivate: bool,
    fail_render: bool,
}

impl InvertVideo {
    fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(),
            ids: vec![
                ParamId(PARAM_AMOUNT),
                ParamId(PARAM_MODE),
                ParamId(PARAM_ECHO_WIDTH),
                ParamId(PARAM_FAIL_DEACTIVATE),
                ParamId(PARAM_FAIL_RENDER),
            ],
            mode_choices: vec!["Invert".to_owned(), "Fill".to_owned()],
            config: None,
            amount: 1.0,
            mode: 0,
            fail_deactivate: false,
            fail_render: false,
        }
    }
}

impl VideoPluginInstance for InvertVideo {
    type Error = MockError;

    fn state(&self) -> LifecycleState {
        self.lifecycle.state()
    }

    fn descriptor(&self) -> PluginDescriptorView<'_> {
        INVERT_DESCRIPTOR.view()
    }

    fn parameters(&self) -> &[ParamId] {
        &self.ids
    }

    fn parameter(&self, id: ParamId) -> Option<PlainParamInfo<'_>> {
        match id.0 {
            PARAM_AMOUNT => Some(PlainParamInfo {
                id,
                name: "Amount",
                unit: "",
                min_value: 0.0,
                max_value: 1.0,
                display_min_value: 0.0,
                display_max_value: 1.0,
                default_value: PlainParamValue::Double(1.0),
                flags: ParamFlags::AUTOMATABLE,
                choices: &[],
            }),
            PARAM_MODE => Some(PlainParamInfo {
                id,
                name: "Mode",
                unit: "",
                min_value: 0.0,
                max_value: 1.0,
                display_min_value: 0.0,
                display_max_value: 1.0,
                default_value: PlainParamValue::Choice(0),
                flags: ParamFlags::AUTOMATABLE.union(ParamFlags::STEPPED),
                choices: &self.mode_choices,
            }),
            PARAM_ECHO_WIDTH => Some(PlainParamInfo {
                id,
                name: "EchoWidth",
                unit: "px",
                min_value: 0.0,
                max_value: 8_192.0,
                display_min_value: 0.0,
                display_max_value: 8_192.0,
                default_value: PlainParamValue::Integer(0),
                flags: ParamFlags::READONLY,
                choices: &[],
            }),
            PARAM_FAIL_DEACTIVATE => Some(PlainParamInfo {
                id,
                name: "FailDeactivate",
                unit: "",
                min_value: 0.0,
                max_value: 1.0,
                display_min_value: 0.0,
                display_max_value: 1.0,
                default_value: PlainParamValue::Boolean(false),
                flags: ParamFlags::HIDDEN,
                choices: &[],
            }),
            PARAM_FAIL_RENDER => Some(PlainParamInfo {
                id,
                name: "FailRender",
                unit: "",
                min_value: 0.0,
                max_value: 1.0,
                display_min_value: 0.0,
                display_max_value: 1.0,
                default_value: PlainParamValue::Boolean(false),
                flags: ParamFlags::HIDDEN,
                choices: &[],
            }),
            _ => None,
        }
    }

    fn parameter_value(&self, id: ParamId) -> Result<PlainParamValue, Self::Error> {
        match id.0 {
            PARAM_AMOUNT => Ok(PlainParamValue::Double(self.amount)),
            PARAM_MODE => Ok(PlainParamValue::Choice(self.mode)),
            // The stash/replay oracle: `0` until `initialize` stores
            // the config, then its width — so a host can tell the
            // `CreateVideoInstance` payload reached the hook.
            PARAM_ECHO_WIDTH => {
                Ok(PlainParamValue::Integer(self.config.map_or(0, |config| {
                    i32::try_from(config.width()).unwrap_or(0)
                })))
            }
            PARAM_FAIL_DEACTIVATE => Ok(PlainParamValue::Boolean(self.fail_deactivate)),
            PARAM_FAIL_RENDER => Ok(PlainParamValue::Boolean(self.fail_render)),
            _ => Err(MockError("unknown parameter")),
        }
    }

    fn set_parameter(&mut self, id: ParamId, value: PlainParamValue) -> Result<(), Self::Error> {
        match (id.0, value) {
            (PARAM_AMOUNT, PlainParamValue::Double(v)) => self.amount = v,
            (PARAM_MODE, PlainParamValue::Choice(v)) => self.mode = v,
            (PARAM_FAIL_DEACTIVATE, PlainParamValue::Boolean(v)) => self.fail_deactivate = v,
            (PARAM_FAIL_RENDER, PlainParamValue::Boolean(v)) => self.fail_render = v,
            _ => return Err(MockError("parameter/type mismatch")),
        }
        Ok(())
    }

    fn initialize(&mut self, config: &VideoProcessConfig) -> Result<(), Self::Error> {
        self.config = Some(*config);
        Ok(())
    }

    fn activate(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn start_processing(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(
        &mut self,
        input: &VideoFrame,
        output: &mut VideoFrame,
        _at: MediaTime,
    ) -> Result<(), Self::Error> {
        // Env-gated mid-processing panic — the same `PluginFailed`
        // normalization class as a hook error, with the out region
        // left uncommitted.
        assert!(
            std::env::var_os("RUVIFORM_PANIC_RENDER").is_none(),
            "ruviform-plugin-l1-video-runner: InvertVideo render (env-gated E2E fixture)"
        );
        // The defined hook-error fixture: `Err` reply, out region
        // uncommitted, child alive.
        if self.fail_render {
            return Err(MockError("render"));
        }
        for index in 0..input.plane_count().min(output.plane_count()) {
            let (Some(source), Some(mut target)) = (input.plane(index), output.plane_mut(index))
            else {
                continue;
            };
            let (source, target) = (source.data(), target.data_mut());
            for (src, dst) in source.iter().zip(target.iter_mut()) {
                *dst = if self.mode == 0 { !src } else { 0x80 };
            }
        }
        Ok(())
    }

    fn stop_processing(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn deactivate(&mut self) -> Result<(), Self::Error> {
        // The commit-then-advisory fixture: a defined hook error that
        // must still leave the mirror back at `Initialized`.
        if self.fail_deactivate {
            return Err(MockError("deactivate"));
        }
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct InvertFactory;

impl VideoPluginFactory for InvertFactory {
    fn descriptor(&self) -> PluginDescriptor {
        INVERT_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ErasedVideoPlugin> {
        erase_video(InvertVideo::new())
    }
}

const PANIC_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.video.panic",
    name: "PanicVideo",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["test-fixture"],
};

/// Deterministic panic fixture: `initialize` panics so the E2E can
/// verify the panic → `Err(HostError::PluginFailed)` mapping and
/// that the child keeps serving commands afterwards; `render` panics
/// too when `RUVIFORM_PANIC_RENDER` is set.
struct PanicVideo {
    lifecycle: Lifecycle,
    ids: Vec<ParamId>,
}

impl VideoPluginInstance for PanicVideo {
    type Error = MockError;

    fn state(&self) -> LifecycleState {
        self.lifecycle.state()
    }

    fn descriptor(&self) -> PluginDescriptorView<'_> {
        PANIC_DESCRIPTOR.view()
    }

    fn parameters(&self) -> &[ParamId] {
        &self.ids
    }

    fn parameter(&self, _id: ParamId) -> Option<PlainParamInfo<'_>> {
        None
    }

    fn parameter_value(&self, _id: ParamId) -> Result<PlainParamValue, Self::Error> {
        Err(MockError("unknown parameter"))
    }

    fn set_parameter(&mut self, _id: ParamId, _value: PlainParamValue) -> Result<(), Self::Error> {
        Err(MockError("unknown parameter"))
    }

    fn initialize(&mut self, _config: &VideoProcessConfig) -> Result<(), Self::Error> {
        panic!(
            "ruviform-plugin-l1-video-runner: PanicVideo initialize (deterministic E2E fixture)"
        );
    }

    fn activate(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn start_processing(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(
        &mut self,
        _input: &VideoFrame,
        _output: &mut VideoFrame,
        _at: MediaTime,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn stop_processing(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn deactivate(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct PanicFactory;

impl VideoPluginFactory for PanicFactory {
    fn descriptor(&self) -> PluginDescriptor {
        // Env-gated fixture for the guarded `descriptor()` paths:
        // `Scan` and the `CreateVideoInstance` factory lookup both
        // catch this → `Err(HostError::PluginFailed)` on the wire.
        assert!(
            std::env::var_os("RUVIFORM_PANIC_DESCRIPTOR").is_none(),
            "ruviform-plugin-l1-video-runner: PanicFactory::descriptor (deterministic E2E fixture)"
        );
        PANIC_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ErasedVideoPlugin> {
        // Env-gated fixture for the guarded `create()` path inside
        // `CreateVideoInstance` — same `PluginFailed` normalization
        // as a hook panic; the registry entry is never inserted.
        assert!(
            std::env::var_os("RUVIFORM_PANIC_CREATE").is_none(),
            "ruviform-plugin-l1-video-runner: PanicFactory::create (deterministic E2E fixture)"
        );
        erase_video(PanicVideo {
            lifecycle: Lifecycle::new(),
            ids: Vec::new(),
        })
    }
}

const OVERLIMIT_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.video.overlimit",
    name: "OverLimit",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["test-fixture"],
};

/// Encode-obligation-(c) fixture, id-list arm: `parameters()`
/// reports `WIRE_MAX_ELEMENTS + 1` ids — unencodable verbatim, so
/// the erased surface must answer `Err(HostError::PluginFailed)`
/// rather than truncate.
struct OverLimitVideo {
    lifecycle: Lifecycle,
    ids: Vec<ParamId>,
}

impl VideoPluginInstance for OverLimitVideo {
    type Error = MockError;

    fn state(&self) -> LifecycleState {
        self.lifecycle.state()
    }

    fn descriptor(&self) -> PluginDescriptorView<'_> {
        OVERLIMIT_DESCRIPTOR.view()
    }

    fn parameters(&self) -> &[ParamId] {
        &self.ids
    }

    fn parameter(&self, _id: ParamId) -> Option<PlainParamInfo<'_>> {
        None
    }

    fn parameter_value(&self, _id: ParamId) -> Result<PlainParamValue, Self::Error> {
        Err(MockError("unknown parameter"))
    }

    fn set_parameter(&mut self, _id: ParamId, _value: PlainParamValue) -> Result<(), Self::Error> {
        Err(MockError("unknown parameter"))
    }

    fn initialize(&mut self, _config: &VideoProcessConfig) -> Result<(), Self::Error> {
        Ok(())
    }

    fn activate(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn start_processing(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(
        &mut self,
        _input: &VideoFrame,
        _output: &mut VideoFrame,
        _at: MediaTime,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn stop_processing(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn deactivate(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct OverLimitFactory;

impl VideoPluginFactory for OverLimitFactory {
    fn descriptor(&self) -> PluginDescriptor {
        OVERLIMIT_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ErasedVideoPlugin> {
        erase_video(OverLimitVideo {
            lifecycle: Lifecycle::new(),
            ids: (0..=ruviform_plugin_core::wire::WIRE_MAX_ELEMENTS)
                .map(ParamId)
                .collect(),
        })
    }
}

const OVERLIMIT_INFO_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.video.overlimit-info",
    name: "OverLimitInfo",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["test-fixture"],
};

/// Encode-obligation fixture, metadata arm: `parameters()` stays
/// inside the wire bounds so `parameter()` is actually reached, but
/// the infos it reports cannot be encoded verbatim — id 0 over the
/// `choices` bound, id 1 over the string bound, both →
/// `Err(HostError::PluginFailed)`. Id 2 reports non-finite
/// `display_*` hints: obligation (d) clamps them into `[min,max]`,
/// observable through `VideoGetParamInfo`.
struct OverLimitInfoVideo {
    lifecycle: Lifecycle,
    ids: Vec<ParamId>,
    many_choices: Vec<String>,
    long_name: String,
}

impl VideoPluginInstance for OverLimitInfoVideo {
    type Error = MockError;

    fn state(&self) -> LifecycleState {
        self.lifecycle.state()
    }

    fn descriptor(&self) -> PluginDescriptorView<'_> {
        OVERLIMIT_INFO_DESCRIPTOR.view()
    }

    fn parameters(&self) -> &[ParamId] {
        &self.ids
    }

    fn parameter(&self, id: ParamId) -> Option<PlainParamInfo<'_>> {
        match id.0 {
            0 => Some(PlainParamInfo {
                id,
                name: "TooManyChoices",
                unit: "",
                min_value: 0.0,
                max_value: 1.0,
                display_min_value: 0.0,
                display_max_value: 1.0,
                default_value: PlainParamValue::Choice(0),
                flags: ParamFlags::NONE,
                choices: &self.many_choices,
            }),
            1 => Some(PlainParamInfo {
                id,
                name: &self.long_name,
                unit: "",
                min_value: 0.0,
                max_value: 1.0,
                display_min_value: 0.0,
                display_max_value: 1.0,
                default_value: PlainParamValue::Double(0.5),
                flags: ParamFlags::NONE,
                choices: &[],
            }),
            2 => Some(PlainParamInfo {
                id,
                name: "InfiniteHints",
                unit: "",
                min_value: 0.0,
                max_value: 1.0,
                display_min_value: f64::NEG_INFINITY,
                display_max_value: f64::INFINITY,
                default_value: PlainParamValue::Double(0.5),
                flags: ParamFlags::NONE,
                choices: &[],
            }),
            _ => None,
        }
    }

    fn parameter_value(&self, _id: ParamId) -> Result<PlainParamValue, Self::Error> {
        Err(MockError("unknown parameter"))
    }

    fn set_parameter(&mut self, _id: ParamId, _value: PlainParamValue) -> Result<(), Self::Error> {
        Err(MockError("unknown parameter"))
    }

    fn initialize(&mut self, _config: &VideoProcessConfig) -> Result<(), Self::Error> {
        Ok(())
    }

    fn activate(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn start_processing(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(
        &mut self,
        _input: &VideoFrame,
        _output: &mut VideoFrame,
        _at: MediaTime,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn stop_processing(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn deactivate(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct OverLimitInfoFactory;

impl VideoPluginFactory for OverLimitInfoFactory {
    fn descriptor(&self) -> PluginDescriptor {
        OVERLIMIT_INFO_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ErasedVideoPlugin> {
        erase_video(OverLimitInfoVideo {
            lifecycle: Lifecycle::new(),
            ids: vec![ParamId(0), ParamId(1), ParamId(2)],
            many_choices: vec![
                "x".to_owned();
                ruviform_plugin_core::wire::WIRE_MAX_ELEMENTS as usize + 1
            ],
            long_name: "n".repeat(ruviform_plugin_core::wire::WIRE_MAX_STRING_BYTES as usize + 1),
        })
    }
}
