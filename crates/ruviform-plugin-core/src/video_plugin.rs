//! The plugin-side video contracts a host (or an L1 video runner)
//! drives: [`VideoPluginFactory`] (one registered video plugin type),
//! [`ErasedVideoPlugin`] (the dyn-safe mirror of
//! [`VideoPluginInstance`]) and [`erase_video`], which wraps any
//! `P: VideoPluginInstance` into the boxed erased surface.
//!
//! `VideoPluginInstance` is not dyn-compatible as a host storage type
//! — its associated `Error` type differs per plugin — so the erased
//! trait drops `type Error` and normalizes every plugin error to
//! [`HostError::PluginFailed`], the same mapping the L0 host surface
//! applies to adapter errors.
//!
//! ## Encode obligations
//!
//! The erased adapter is also the encode-side boundary of the L1 wire
//! contract (wire v3, Issue #118): wire decode is strict, so a
//! contract-conforming-but-wire-unrepresentable plugin report —
//! encoded verbatim — would bounce off the peer's decode as a
//! protocol violation and kill the child. The adapter normalizes
//! those reports to `Err(HostError::PluginFailed)` (a plugin wart is
//! a plugin error, never a protocol violation) instead of letting
//! them onto the wire:
//!
//! * **(b)** `parameter_value` returning
//!   [`PlainParamValue::Double`] with a non-finite payload →
//!   `Err(PluginFailed)` (wire `Double` must be finite).
//! * **(c)** `parameters()` reporting more than
//!   [`wire::WIRE_MAX_ELEMENTS`] ids, or `parameter()` reporting a
//!   [`PlainParamInfo`] the wire decode would reject — over-bound
//!   `choices` count or string byte length, or a violation of the
//!   `PlainParamInfo` contract the decode re-validates
//!   (`min_value`/`max_value` finite and ordered, `default_value`
//!   inside them, `choices` non-empty exactly for `Choice` defaults
//!   with the index in bounds) — → `Err(PluginFailed)`. Never
//!   truncated: a plugin that cannot enumerate itself inside the
//!   wire bounds is broken.
//! * **(d)** `parameter()` reporting a non-finite `display_min` /
//!   `display_max` hint is *clamped*, not rejected — wire decode
//!   accepts non-finite `display_*` leniently (they are UI hints),
//!   so this is host-side sanity normalization, mirroring the OFX
//!   `±DBL_MAX` default convention: the value is folded into
//!   `[min_value, max_value]` (`±f64::MAX` when that lands inside
//!   the domain), with `NaN` folded to the bound endpoint.
//!
//! **RT contract:** every item here is **non-RT only** — the video
//! channel carries no real-time guarantee.

use ruviform_media_core::MediaTime;
use ruviform_video_core::VideoFrame;

use crate::descriptor::{PluginDescriptor, PluginDescriptorView};
use crate::lifecycle::LifecycleState;
use crate::param::ParamId;
use crate::plain_param::{PlainParamInfo, PlainParamInfoOwned, PlainParamValue};
use crate::video::{VideoPluginInstance, VideoProcessConfig};
use crate::{HostError, wire};

/// The dyn-safe mirror of [`VideoPluginInstance`]: the storage type
/// `Box<dyn ErasedVideoPlugin>` a host or an L1 video runner holds
/// per video instance.
///
/// Adapters do not implement this trait directly — [`erase_video`]
/// wraps any `P: VideoPluginInstance` into it. Plugin `Error`s are
/// normalized to [`HostError::PluginFailed`]; the parameter reads are
/// `Result`-typed here (unlike the borrowed views of the underlying
/// trait) so the wire encode obligations the adapter enforces can
/// report a broken plugin report — see the module documentation.
///
/// **RT contract:** every method is **non-RT only** — `render` is a
/// synchronous control-plane call, not a real-time callback.
pub trait ErasedVideoPlugin {
    /// [`VideoPluginInstance::state`].
    fn state(&self) -> LifecycleState;
    /// [`VideoPluginInstance::descriptor`].
    fn descriptor(&self) -> PluginDescriptorView<'_>;
    /// [`VideoPluginInstance::parameters`] — `Err(PluginFailed)`
    /// when the plugin reports an id list longer than
    /// [`wire::WIRE_MAX_ELEMENTS`] (encode obligation (c)).
    fn parameters(&self) -> Result<&[ParamId], HostError>;
    /// [`VideoPluginInstance::parameter`] — `Err(PluginFailed)`
    /// when the plugin reports metadata the wire decode would
    /// reject (encode obligation (c)); non-finite `display_*` hints
    /// are clamped into `[min_value, max_value]` (obligation (d)).
    /// Returns the owned form: the adapter must be able to rewrite
    /// the hints, which a borrowed view cannot carry.
    fn parameter(&self, id: ParamId) -> Result<Option<PlainParamInfoOwned>, HostError>;
    /// [`VideoPluginInstance::parameter_value`] — `Err(PluginFailed)`
    /// on any plugin error or on a non-finite `Double` (encode
    /// obligation (b)).
    fn parameter_value(&self, id: ParamId) -> Result<PlainParamValue, HostError>;
    /// [`VideoPluginInstance::set_parameter`].
    fn set_parameter(&mut self, id: ParamId, value: PlainParamValue) -> Result<(), HostError>;
    /// [`VideoPluginInstance::initialize`].
    fn initialize(&mut self, config: &VideoProcessConfig) -> Result<(), HostError>;
    /// [`VideoPluginInstance::activate`].
    fn activate(&mut self) -> Result<(), HostError>;
    /// [`VideoPluginInstance::start_processing`].
    fn start_processing(&mut self) -> Result<(), HostError>;
    /// [`VideoPluginInstance::render`].
    fn render(
        &mut self,
        input: &VideoFrame,
        output: &mut VideoFrame,
        at: MediaTime,
    ) -> Result<(), HostError>;
    /// [`VideoPluginInstance::stop_processing`].
    fn stop_processing(&mut self) -> Result<(), HostError>;
    /// [`VideoPluginInstance::deactivate`].
    fn deactivate(&mut self) -> Result<(), HostError>;
    /// [`VideoPluginInstance::destroy`].
    fn destroy(&mut self) -> Result<(), HostError>;
}

/// One registered video plugin type: descriptor plus instance
/// construction — the [`crate::PluginFactory`] counterpart for the
/// video kind. No `create_sendable` pair exists: the video channel
/// has no RT endpoint path that would need a `Send` instance.
///
/// **RT contract:** non-RT only — registration and instantiation are
/// host-side setup operations.
pub trait VideoPluginFactory {
    /// The descriptor a `Scan` reports and `CreateVideoInstance`
    /// requests match `plugin_id` against.
    ///
    /// **RT contract:** non-RT only.
    fn descriptor(&self) -> PluginDescriptor;
    /// Creates one fresh video instance in `Created` state.
    ///
    /// **RT contract:** non-RT only — may allocate and block.
    fn create(&self) -> Box<dyn ErasedVideoPlugin>;
}

/// Type-erasing wrapper produced by [`erase_video`].
struct ErasedVideo<P>(P);

impl<P: VideoPluginInstance> ErasedVideoPlugin for ErasedVideo<P> {
    fn state(&self) -> LifecycleState {
        self.0.state()
    }
    fn descriptor(&self) -> PluginDescriptorView<'_> {
        self.0.descriptor()
    }
    fn parameters(&self) -> Result<&[ParamId], HostError> {
        let ids = self.0.parameters();
        // Encode obligation (c): an id list over the wire element
        // bound cannot be encoded verbatim without the peer's decode
        // rejecting it — the plugin is broken, so `PluginFailed`.
        if ids.len() > usize::try_from(wire::WIRE_MAX_ELEMENTS).unwrap_or(usize::MAX) {
            return Err(HostError::PluginFailed);
        }
        Ok(ids)
    }
    fn parameter(&self, id: ParamId) -> Result<Option<PlainParamInfoOwned>, HostError> {
        self.0
            .parameter(id)
            .map(|info| checked_param_info(&info))
            .transpose()
    }
    fn parameter_value(&self, id: ParamId) -> Result<PlainParamValue, HostError> {
        let value = self
            .0
            .parameter_value(id)
            .map_err(|_| HostError::PluginFailed)?;
        // Encode obligation (b): a non-finite `Double` has no wire
        // representation — the plugin's wart is a plugin error, not
        // a protocol violation.
        if let PlainParamValue::Double(v) = value
            && !v.is_finite()
        {
            return Err(HostError::PluginFailed);
        }
        Ok(value)
    }
    fn set_parameter(&mut self, id: ParamId, value: PlainParamValue) -> Result<(), HostError> {
        self.0
            .set_parameter(id, value)
            .map_err(|_| HostError::PluginFailed)
    }
    fn initialize(&mut self, config: &VideoProcessConfig) -> Result<(), HostError> {
        self.0
            .initialize(config)
            .map_err(|_| HostError::PluginFailed)
    }
    fn activate(&mut self) -> Result<(), HostError> {
        self.0.activate().map_err(|_| HostError::PluginFailed)
    }
    fn start_processing(&mut self) -> Result<(), HostError> {
        self.0
            .start_processing()
            .map_err(|_| HostError::PluginFailed)
    }
    fn render(
        &mut self,
        input: &VideoFrame,
        output: &mut VideoFrame,
        at: MediaTime,
    ) -> Result<(), HostError> {
        self.0
            .render(input, output, at)
            .map_err(|_| HostError::PluginFailed)
    }
    fn stop_processing(&mut self) -> Result<(), HostError> {
        self.0
            .stop_processing()
            .map_err(|_| HostError::PluginFailed)
    }
    fn deactivate(&mut self) -> Result<(), HostError> {
        self.0.deactivate().map_err(|_| HostError::PluginFailed)
    }
    fn destroy(&mut self) -> Result<(), HostError> {
        self.0.destroy().map_err(|_| HostError::PluginFailed)
    }
}

/// Encode obligation (c) for `parameter()` plus the (d) hint clamp:
/// validates `info` against the same contract the wire decode
/// re-checks — bounded strings and `choices`, finite ordered
/// `min_value`/`max_value`, `default_value` inside them, `choices`
/// non-empty exactly for a `Choice` default with the index in range —
/// then folds non-finite `display_*` hints into `[min_value,
/// max_value]`. Any violation is `Err(HostError::PluginFailed)`: the
/// report can never be encoded verbatim.
fn checked_param_info(info: &PlainParamInfo<'_>) -> Result<PlainParamInfoOwned, HostError> {
    let max_str = usize::try_from(wire::WIRE_MAX_STRING_BYTES).unwrap_or(usize::MAX);
    let max_elements = usize::try_from(wire::WIRE_MAX_ELEMENTS).unwrap_or(usize::MAX);
    if info.name.len() > max_str
        || info.unit.len() > max_str
        || info.choices.len() > max_elements
        || info.choices.iter().any(|choice| choice.len() > max_str)
    {
        return Err(HostError::PluginFailed);
    }
    if !info.min_value.is_finite() || !info.max_value.is_finite() || info.min_value > info.max_value
    {
        return Err(HostError::PluginFailed);
    }
    // The default's scalar must lie inside [min, max] — the check
    // the wire decode re-applies. `Choice` has no scalar domain:
    // its contract is the in-bounds index checked below.
    let default_scalar = match info.default_value {
        PlainParamValue::Double(v) => Some(v),
        PlainParamValue::Integer(v) => Some(f64::from(v)),
        PlainParamValue::Boolean(v) => Some(f64::from(u8::from(v))),
        _ => None,
    };
    match info.default_value {
        PlainParamValue::Choice(index) => {
            if index as usize >= info.choices.len() {
                // A `Choice` default must name a real choice —
                // `choices` non-empty is folded into this range
                // check, mirroring the wire decode.
                return Err(HostError::PluginFailed);
            }
        }
        _ => {
            if !info.choices.is_empty()
                || !default_scalar.is_some_and(|v| {
                    v.is_finite() && (info.min_value..=info.max_value).contains(&v)
                })
            {
                return Err(HostError::PluginFailed);
            }
        }
    }
    Ok(PlainParamInfoOwned {
        id: info.id,
        name: info.name.to_owned(),
        unit: info.unit.to_owned(),
        min_value: info.min_value,
        max_value: info.max_value,
        display_min_value: clamp_hint(
            info.display_min_value,
            info.min_value,
            info.max_value,
            info.min_value,
        ),
        display_max_value: clamp_hint(
            info.display_max_value,
            info.min_value,
            info.max_value,
            info.max_value,
        ),
        default_value: info.default_value,
        flags: info.flags,
        choices: info.choices.to_vec(),
    })
}

/// Encode obligation (d): folds a `display_*` hint into the ordered
/// `[min, max]` domain the caller already validated — the OFX
/// `±DBL_MAX` default convention's mapping. `f64::clamp` covers the
/// infinities (`-inf → min`, `+inf → max`); `NaN`, which `clamp`
/// would propagate, folds to the hint's own endpoint (`nan_fallback`)
/// — a meaningless hint becomes its domain end rather than
/// collapsing the display range or poisoning downstream UI scaling.
fn clamp_hint(value: f64, min: f64, max: f64, nan_fallback: f64) -> f64 {
    if value.is_nan() {
        nan_fallback
    } else {
        value.clamp(min, max)
    }
}

/// Erases a concrete [`VideoPluginInstance`] into the boxed
/// [`ErasedVideoPlugin`] the host or L1 video runner stores — the
/// one-line adapter-side companion of [`VideoPluginFactory::create`].
///
/// **RT contract:** non-RT only — allocates the box.
pub fn erase_video<P: VideoPluginInstance + 'static>(plugin: P) -> Box<dyn ErasedVideoPlugin> {
    Box::new(ErasedVideo(plugin))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lifecycle, ParamFlags, VideoProcessConfig};
    use ruviform_media_core::FrameRate;
    use ruviform_video_core::PixelFormat;
    use std::fmt;

    const DESC: PluginDescriptor = PluginDescriptor {
        id: "dev.ruviform.test.video",
        name: "TestVideo",
        vendor: "Ruviform",
        version: "1.0.0",
        features: &["video-effect"],
    };

    #[derive(Debug)]
    struct TestError;

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("test error")
        }
    }
    impl std::error::Error for TestError {}

    /// A configurable video fixture: every trait input is stored so a
    /// test can set the exact report the adapter must normalize.
    struct Fixture {
        lifecycle: Lifecycle,
        ids: Vec<ParamId>,
        info: Option<PlainParamInfoOwned>,
        value: PlainParamValue,
        fail: bool,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                lifecycle: Lifecycle::new(),
                ids: Vec::new(),
                info: None,
                value: PlainParamValue::Double(0.5),
                fail: false,
            }
        }
    }

    impl VideoPluginInstance for Fixture {
        type Error = TestError;
        fn state(&self) -> LifecycleState {
            self.lifecycle.state()
        }
        fn descriptor(&self) -> PluginDescriptorView<'_> {
            DESC.view()
        }
        fn parameters(&self) -> &[ParamId] {
            &self.ids
        }
        fn parameter(&self, id: ParamId) -> Option<PlainParamInfo<'_>> {
            self.info
                .as_ref()
                .filter(|info| info.id == id)
                .map(PlainParamInfoOwned::as_view)
        }
        fn parameter_value(&self, _id: ParamId) -> Result<PlainParamValue, Self::Error> {
            if self.fail {
                Err(TestError)
            } else {
                Ok(self.value)
            }
        }
        fn set_parameter(
            &mut self,
            _id: ParamId,
            _value: PlainParamValue,
        ) -> Result<(), Self::Error> {
            if self.fail { Err(TestError) } else { Ok(()) }
        }
        fn initialize(&mut self, _config: &VideoProcessConfig) -> Result<(), Self::Error> {
            if self.fail { Err(TestError) } else { Ok(()) }
        }
        fn activate(&mut self) -> Result<(), Self::Error> {
            if self.fail { Err(TestError) } else { Ok(()) }
        }
        fn start_processing(&mut self) -> Result<(), Self::Error> {
            if self.fail { Err(TestError) } else { Ok(()) }
        }
        fn render(
            &mut self,
            _input: &VideoFrame,
            _output: &mut VideoFrame,
            _at: MediaTime,
        ) -> Result<(), Self::Error> {
            if self.fail { Err(TestError) } else { Ok(()) }
        }
        fn stop_processing(&mut self) -> Result<(), Self::Error> {
            if self.fail { Err(TestError) } else { Ok(()) }
        }
        fn deactivate(&mut self) -> Result<(), Self::Error> {
            if self.fail { Err(TestError) } else { Ok(()) }
        }
        fn destroy(&mut self) -> Result<(), Self::Error> {
            if self.fail { Err(TestError) } else { Ok(()) }
        }
    }

    fn info(id: u32) -> PlainParamInfoOwned {
        PlainParamInfoOwned {
            id: ParamId(id),
            name: "Amount".to_owned(),
            unit: String::new(),
            min_value: 0.0,
            max_value: 1.0,
            display_min_value: 0.0,
            display_max_value: 1.0,
            default_value: PlainParamValue::Double(0.5),
            flags: ParamFlags::AUTOMATABLE,
            choices: Vec::new(),
        }
    }

    #[test]
    fn erase_video_normalizes_plugin_errors_to_plugin_failed() {
        let mut fixture = Fixture::new();
        fixture.fail = true;
        let mut plugin = erase_video(fixture);
        let config =
            VideoProcessConfig::new(PixelFormat::Rgba8, 64, 64, FrameRate::new(30, 1).unwrap())
                .unwrap();
        assert_eq!(plugin.initialize(&config), Err(HostError::PluginFailed));
        assert_eq!(plugin.activate(), Err(HostError::PluginFailed));
        assert_eq!(plugin.start_processing(), Err(HostError::PluginFailed));
        assert_eq!(plugin.stop_processing(), Err(HostError::PluginFailed));
        assert_eq!(plugin.deactivate(), Err(HostError::PluginFailed));
        assert_eq!(plugin.destroy(), Err(HostError::PluginFailed));
        assert_eq!(
            plugin.parameter_value(ParamId(0)),
            Err(HostError::PluginFailed)
        );
        assert_eq!(
            plugin.set_parameter(ParamId(0), PlainParamValue::Double(0.5)),
            Err(HostError::PluginFailed)
        );
    }

    #[test]
    fn erase_video_passes_through_healthy_hooks() {
        let mut fixture = Fixture::new();
        fixture.ids = vec![ParamId(0)];
        fixture.info = Some(info(0));
        let plugin = erase_video(fixture);
        assert_eq!(plugin.parameters(), Ok(&[ParamId(0)][..]));
        assert_eq!(
            plugin.parameter(ParamId(0)).unwrap().map(|i| i.id),
            Some(ParamId(0))
        );
        assert_eq!(plugin.parameter(ParamId(9)), Ok(None));
        assert_eq!(
            plugin.parameter_value(ParamId(0)),
            Ok(PlainParamValue::Double(0.5))
        );
        assert_eq!(plugin.state(), LifecycleState::Created);
    }

    #[test]
    fn obligation_b_non_finite_double_value_is_plugin_failed() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut fixture = Fixture::new();
            fixture.value = PlainParamValue::Double(bad);
            let plugin = erase_video(fixture);
            assert_eq!(
                plugin.parameter_value(ParamId(0)),
                Err(HostError::PluginFailed),
                "non-finite {bad} must normalize to PluginFailed"
            );
        }
        // Non-Double values are always wire-representable.
        for value in [
            PlainParamValue::Integer(-7),
            PlainParamValue::Boolean(true),
            PlainParamValue::Choice(2),
        ] {
            let mut fixture = Fixture::new();
            fixture.value = value;
            let plugin = erase_video(fixture);
            assert_eq!(plugin.parameter_value(ParamId(0)), Ok(value));
        }
    }

    #[test]
    fn obligation_c_over_limit_id_list_is_plugin_failed() {
        let mut fixture = Fixture::new();
        fixture.ids = (0..=wire::WIRE_MAX_ELEMENTS).map(ParamId).collect();
        let plugin = erase_video(fixture);
        assert_eq!(plugin.parameters(), Err(HostError::PluginFailed));
    }

    #[test]
    fn obligation_c_over_limit_info_is_plugin_failed() {
        let cases: Vec<PlainParamInfoOwned> = vec![
            // choices over WIRE_MAX_ELEMENTS.
            PlainParamInfoOwned {
                default_value: PlainParamValue::Choice(0),
                choices: vec!["x".to_owned(); wire::WIRE_MAX_ELEMENTS as usize + 1],
                ..info(0)
            },
            // a choice string over WIRE_MAX_STRING_BYTES.
            PlainParamInfoOwned {
                default_value: PlainParamValue::Choice(0),
                choices: vec!["x".repeat(wire::WIRE_MAX_STRING_BYTES as usize + 1)],
                ..info(0)
            },
            // name over WIRE_MAX_STRING_BYTES.
            PlainParamInfoOwned {
                name: "n".repeat(wire::WIRE_MAX_STRING_BYTES as usize + 1),
                ..info(0)
            },
            // unit over WIRE_MAX_STRING_BYTES.
            PlainParamInfoOwned {
                unit: "u".repeat(wire::WIRE_MAX_STRING_BYTES as usize + 1),
                ..info(0)
            },
        ];
        for (index, owned) in cases.into_iter().enumerate() {
            let mut fixture = Fixture::new();
            fixture.ids = vec![ParamId(0)];
            fixture.info = Some(owned);
            let plugin = erase_video(fixture);
            assert_eq!(
                plugin.parameter(ParamId(0)),
                Err(HostError::PluginFailed),
                "case {index} must normalize to PluginFailed"
            );
        }
    }

    #[test]
    fn obligation_c_contract_violating_info_is_plugin_failed() {
        let cases: Vec<PlainParamInfoOwned> = vec![
            // Non-finite / unordered hard endpoints.
            PlainParamInfoOwned {
                min_value: f64::NAN,
                ..info(0)
            },
            PlainParamInfoOwned {
                max_value: f64::INFINITY,
                ..info(0)
            },
            PlainParamInfoOwned {
                min_value: 2.0,
                ..info(0)
            },
            // Default outside [min, max].
            PlainParamInfoOwned {
                default_value: PlainParamValue::Double(2.0),
                ..info(0)
            },
            PlainParamInfoOwned {
                default_value: PlainParamValue::Double(f64::INFINITY),
                ..info(0)
            },
            // `choices` on a non-Choice default.
            PlainParamInfoOwned {
                choices: vec!["x".to_owned()],
                ..info(0)
            },
            // Choice default with empty choices / out-of-range index.
            PlainParamInfoOwned {
                default_value: PlainParamValue::Choice(0),
                ..info(0)
            },
            PlainParamInfoOwned {
                default_value: PlainParamValue::Choice(7),
                choices: vec!["x".to_owned()],
                ..info(0)
            },
        ];
        for (index, owned) in cases.into_iter().enumerate() {
            let mut fixture = Fixture::new();
            fixture.ids = vec![ParamId(0)];
            fixture.info = Some(owned);
            let plugin = erase_video(fixture);
            assert_eq!(
                plugin.parameter(ParamId(0)),
                Err(HostError::PluginFailed),
                "case {index} must normalize to PluginFailed"
            );
        }
    }

    #[test]
    fn obligation_d_non_finite_display_hints_are_clamped() {
        let mut fixture = Fixture::new();
        fixture.ids = vec![ParamId(0)];
        fixture.info = Some(PlainParamInfoOwned {
            display_min_value: f64::NEG_INFINITY,
            display_max_value: f64::INFINITY,
            ..info(0)
        });
        let plugin = erase_video(fixture);
        let owned = plugin.parameter(ParamId(0)).unwrap().unwrap();
        // `-inf` folds to the domain's lower end, `+inf` to the upper.
        assert_eq!(owned.display_min_value, 0.0);
        assert_eq!(owned.display_max_value, 1.0);
    }

    #[test]
    fn obligation_d_nan_display_hint_folds_to_the_bound() {
        let mut fixture = Fixture::new();
        fixture.ids = vec![ParamId(0)];
        fixture.info = Some(PlainParamInfoOwned {
            display_min_value: f64::NAN,
            display_max_value: f64::NAN,
            ..info(0)
        });
        let plugin = erase_video(fixture);
        let owned = plugin.parameter(ParamId(0)).unwrap().unwrap();
        assert_eq!(owned.display_min_value, 0.0);
        assert_eq!(owned.display_max_value, 1.0);
    }

    #[test]
    fn obligation_d_finite_display_hints_pass_through() {
        let mut fixture = Fixture::new();
        fixture.ids = vec![ParamId(0)];
        fixture.info = Some(PlainParamInfoOwned {
            // UI hints may legally narrow the domain's ends.
            display_min_value: 0.25,
            display_max_value: 0.75,
            ..info(0)
        });
        let plugin = erase_video(fixture);
        let owned = plugin.parameter(ParamId(0)).unwrap().unwrap();
        assert_eq!(owned.display_min_value, 0.25);
        assert_eq!(owned.display_max_value, 0.75);
    }

    #[test]
    fn obligation_d_extreme_finite_domain_keeps_f64_max() {
        // A domain of ±f64::MAX keeps the OFX-style ±MAX default —
        // the "lands inside [min,max]" branch of the clamp.
        let mut fixture = Fixture::new();
        fixture.ids = vec![ParamId(0)];
        fixture.info = Some(PlainParamInfoOwned {
            min_value: -f64::MAX,
            max_value: f64::MAX,
            default_value: PlainParamValue::Double(0.0),
            display_min_value: f64::NEG_INFINITY,
            display_max_value: f64::INFINITY,
            ..info(0)
        });
        let plugin = erase_video(fixture);
        let owned = plugin.parameter(ParamId(0)).unwrap().unwrap();
        assert_eq!(owned.display_min_value, -f64::MAX);
        assert_eq!(owned.display_max_value, f64::MAX);
    }
}
