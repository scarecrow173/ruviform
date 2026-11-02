//! `ruviform-audio-core` — audio data types for the Ruviform media
//! foundation: the sealed [`Sample`] trait (`f32`/`f64`), the audio time
//! contract ([`SampleRate`], [`SampleIndex`]), channel layouts and
//! speaker masks ([`ChannelLayout`], [`SpeakerMask`],
//! [`SpeakerMaskSet`]), stream-level
//! format description ([`AudioFormat`], [`SampleFormat`]) and the
//! channel-major contiguous planar [`AudioBuffer`] with its borrowed
//! views ([`AudioBufferView`], [`AudioBufferViewMut`], [`BufferSpec`]),
//! and the device boundary ports ([`device`]), plus independent per-channel
//! planar borrowed views ([`PlanarBufferView`], [`PlanarBufferViewMut`]) for
//! disjoint channel slice lists at processing boundaries (plugin APIs, device
//! callbacks).
//!
//! Planar `f32` is the canonical processing format; interleaved sample
//! order exists only at device/file boundaries — through
//! [`AudioBuffer::from_interleaved`], [`AudioBuffer::copy_to_interleaved`]
//! and [`AudioBuffer::copy_from_interleaved`].
//!
//! Holds no processing, I/O, or device implementations. `ruviform-audio`
//! (engine) and `ruviform-audio-device` (adapter) connect through the
//! device boundary ports defined in [`device`], structurally eliminating
//! a direct engine ↔ adapter dependency. The ports live in their own
//! namespace (`ruviform_audio_core::device::*`) — deliberately not
//! re-exported at the crate root, since the module bundles many items
//! as one named "boundary" (see
//! `docs/designs/audio-device-boundary-and-poc.md`).
//!
//! ## Allowed dependency layers
//!
//! Primitives (`ruviform-math`, `ruviform-rt`) and `ruviform-media-core`.
//! Actually used: `ruviform-media-core`
//! ([`TimeBase`](ruviform_media_core::TimeBase),
//! [`MediaTime`](ruviform_media_core::MediaTime),
//! [`MediaTimestamp`](ruviform_media_core::MediaTimestamp),
//! [`TimestampProvenance`](ruviform_media_core::TimestampProvenance),
//! [`ClockDomainId`](ruviform_media_core::ClockDomainId),
//! [`InputStamp`](ruviform_media_core::InputStamp),
//! [`Rounding`](ruviform_media_core::Rounding),
//! [`Error`](ruviform_media_core::Error)/[`Result`](ruviform_media_core::Result)).
//! `ruviform-audio-core` must not
//! depend on `ruviform-video-core` (same-tier dependency is prohibited).
//!
//! ## RT contract
//!
//! Buffer views and accessors are allocation-free, non-blocking borrows —
//! RT-suitable. Allocating constructors ([`AudioBuffer::new`],
//! [`AudioBuffer::try_new`], [`AudioBuffer::from_interleaved`]) are
//! labelled `**RT contract:**
//! non-RT only`. A buffer never grows after construction: no
//! `push`/`reserve`-style API exists. Per-API guarantees are documented
//! as `**RT contract:**` lines on each item, mirroring the RT ADR rule
//! that the guaranteed scope is defined per API.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "音声データモデル") and
//! `docs/adr/realtime-contract-and-verification-policy.md`.

// The public names and shapes below are fixed API decisions of the data
// model: they deliberately echo their module (`buffer::AudioBuffer`,
// `layout::ChannelLayout`, …) while being re-exported at the crate root,
// and as a `publish = false` workspace-internal value-type crate this API
// relies on literal construction for structs. Every public enum carries
// enum-level `#[non_exhaustive]`: exhaustive matching is reserved for
// crate-internal code while external users (sibling crates included) get
// forward compatibility — see
// `docs/adr/public-enum-non-exhaustive-policy.md`.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::exhaustive_structs)]

mod buffer;
pub mod device;
mod format;
mod layout;
pub mod midi_input;
mod planar;
mod sample;
mod time;

pub use buffer::{AudioBuffer, AudioBufferView, AudioBufferViewMut, BufferSpec};
pub use format::{AudioFormat, SampleFormat};
pub use layout::{ChannelLayout, SpeakerMask, SpeakerMaskSet};
pub use planar::{PlanarBufferView, PlanarBufferViewMut};
pub use sample::Sample;
pub use time::{SampleIndex, SampleRate};

/// Returns the crate version string.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_audio_core::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::Rounding;

    fn stereo_spec(frames: usize) -> BufferSpec {
        BufferSpec {
            layout: ChannelLayout::Stereo,
            frames,
            sample_rate: SampleRate::new(48_000).unwrap(),
        }
    }

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }

    /// RT contract: view channel access is an allocation-free borrow;
    /// writes through the mutable view land in the channel-major slab.
    #[test]
    fn rt_view_channel_access() {
        let mut buffer = AudioBuffer::from_interleaved(
            &[0.0_f32, 10.0, 1.0, 11.0, 2.0, 12.0, 3.0, 13.0],
            stereo_spec(4),
        )
        .unwrap();
        {
            let view = buffer.view();
            assert_eq!(view.channels(), 2);
            assert_eq!(view.frames(), 4);
            assert_eq!(view.channel(0), Some(&[0.0, 1.0, 2.0, 3.0][..]));
            assert_eq!(view.channel(1), Some(&[10.0, 11.0, 12.0, 13.0][..]));
            assert_eq!(view.channel(2), None);
        }
        {
            let mut view = buffer.view_mut();
            for sample in view.channel_mut(1).unwrap() {
                *sample += 1.0;
            }
            assert_eq!(view.channel(1), Some(&[11.0, 12.0, 13.0, 14.0][..]));
        }
        assert_eq!(buffer.channel(1), Some(&[11.0, 12.0, 13.0, 14.0][..]));
    }

    /// RT contract: interleaved boundary copies through views are
    /// allocation-free bounded copies — the canonical layout stays planar.
    #[test]
    fn rt_interleaved_roundtrip_via_views() {
        let src = [0.0_f32, 100.0, 1.0, 101.0, 2.0, 102.0];
        let mut buffer = AudioBuffer::from_interleaved(&src, stereo_spec(3)).unwrap();

        let mut packed = [0.0_f32; 6];
        buffer.view().copy_to_interleaved(&mut packed).unwrap();
        assert_eq!(packed, src);

        let other = [5.0_f32, 105.0, 6.0, 106.0, 7.0, 107.0];
        buffer.view_mut().copy_from_interleaved(&other).unwrap();
        let mut out = [0.0_f32; 6];
        buffer.view().copy_to_interleaved(&mut out).unwrap();
        assert_eq!(out, other);
    }

    /// RT contract: sample-index ↔ `MediaTime` conversion is exact
    /// rational math — RT-suitable, allocation-free.
    #[test]
    fn rt_sample_index_media_time_conversion() {
        let rate = SampleRate::new(48_000).unwrap();
        let index = SampleIndex(96_000); // exactly 2 s
        let time = index.to_media_time(rate);
        assert_eq!(
            SampleIndex::from_media_time(time, rate, Rounding::NearestEven),
            Ok(index)
        );
    }

    /// Compile-level contract tests for the [`crate::device`] port
    /// (design `audio-device-boundary-and-poc.md` 検証設計節): object
    /// safety, `Send` through trait objects, the HRTB coercion of
    /// `OutputDataCallback`, `Copy`/`Clone` bounds on the value types
    /// and the `DeviceError` contract. `StubEnumerator`/`StubDevice`/
    /// `StubStream` double as the mock-backend check — a non-backend
    /// implementation can implement the ports and drive the full flow
    /// through `Box<dyn …>` without touching hardware.
    mod device_contract {
        use super::*;
        use crate::device::{
            BufferSize, DEVICE_CLOCK_TIMEBASE, Device, DeviceEnumerator, DeviceError, DeviceId,
            DeviceResult, InputCallbackInfo, InputDataCallback, InputErrorCallback, InputStream,
            InputStreamConfig, InputTimestamp, LatencyCoverage, OutputCallbackInfo,
            OutputDataCallback, OutputErrorCallback, OutputStream, OutputStreamConfig,
            OutputTimestamp, StreamLatency, SupportedBufferSize, SupportedInputFormat,
            SupportedOutputFormat,
        };
        use ruviform_media_core::MediaTime;
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        fn stub_format() -> AudioFormat {
            AudioFormat {
                sample_rate: SampleRate::new(48_000).unwrap(),
                layout: ChannelLayout::Stereo,
                sample_format: SampleFormat::F32,
            }
        }

        struct StubStream {
            format: AudioFormat,
        }

        impl OutputStream for StubStream {
            fn start(&self) -> DeviceResult<()> {
                Ok(())
            }

            fn stop(&self) -> DeviceResult<()> {
                Ok(())
            }

            fn format(&self) -> AudioFormat {
                self.format
            }

            fn buffer_size(&self) -> DeviceResult<BufferSize> {
                Ok(BufferSize::Fixed(512))
            }

            fn now(&self) -> DeviceResult<MediaTime> {
                Ok(MediaTime::new(0, DEVICE_CLOCK_TIMEBASE))
            }
        }

        struct StubDevice {
            format: AudioFormat,
        }

        impl Device for StubDevice {
            fn name(&self) -> DeviceResult<String> {
                Ok("stub-device".to_owned())
            }

            fn id(&self) -> DeviceResult<Option<DeviceId>> {
                Ok(Some(DeviceId::new("stub-0".to_owned())))
            }

            fn default_output_format(&self) -> DeviceResult<AudioFormat> {
                Ok(self.format)
            }

            fn supported_output_formats(&self) -> DeviceResult<Vec<SupportedOutputFormat>> {
                Ok(vec![SupportedOutputFormat {
                    channels: 2,
                    min_sample_rate: 44_100,
                    max_sample_rate: 48_000,
                    buffer_size: SupportedBufferSize::Range { min: 32, max: 1024 },
                    sample_format: SampleFormat::F32,
                }])
            }

            fn open_output_stream(
                &self,
                config: &OutputStreamConfig,
                mut data_callback: OutputDataCallback,
                _error_callback: OutputErrorCallback,
            ) -> DeviceResult<Box<dyn OutputStream>> {
                // Fire one scripted callback on a scratch slab — the
                // adapter-shaped per-callback `try_new` view.
                let channels = config.format.layout.channels() as usize;
                let frames = 4;
                let mut scratch = vec![0.0_f32; channels * frames];
                let view = AudioBufferViewMut::try_new(&mut scratch, channels, frames)
                    .map_err(|_| DeviceError::Backend("stub_view"))?;
                data_callback(
                    view,
                    OutputCallbackInfo {
                        timestamp: OutputTimestamp {
                            callback: MediaTime::new(0, DEVICE_CLOCK_TIMEBASE),
                            playback: None,
                        },
                    },
                );
                Ok(Box::new(StubStream {
                    format: config.format,
                }))
            }

            fn default_input_format(&self) -> DeviceResult<AudioFormat> {
                Err(DeviceError::Unsupported)
            }

            fn supported_input_formats(&self) -> DeviceResult<Vec<SupportedInputFormat>> {
                Ok(Vec::new())
            }

            fn open_input_stream(
                &self,
                _config: &InputStreamConfig,
                _data_callback: InputDataCallback,
                _error_callback: InputErrorCallback,
            ) -> DeviceResult<Box<dyn InputStream>> {
                Err(DeviceError::Unsupported)
            }
        }

        struct StubEnumerator;

        impl DeviceEnumerator for StubEnumerator {
            fn output_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>> {
                Ok(vec![Box::new(StubDevice {
                    format: stub_format(),
                })])
            }

            fn default_output_device(&self) -> DeviceResult<Option<Box<dyn Device>>> {
                Ok(Some(Box::new(StubDevice {
                    format: stub_format(),
                })))
            }

            fn input_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>> {
                Ok(Vec::new())
            }

            fn default_input_device(&self) -> DeviceResult<Option<Box<dyn Device>>> {
                Ok(None)
            }
        }

        /// `Box<dyn …>` composability + `Send` bounds on the port
        /// traits and the exchanged value types.
        #[test]
        fn port_traits_compose_as_send_boxed_trait_objects() {
            // Object safety: the ports are usable behind `&dyn …`
            // (no generics, no `Self` returns, no associated types).
            fn borrow_objects(_: &dyn DeviceEnumerator, _: &dyn Device, _: &dyn OutputStream) {}

            // `Send` reaches through the trait objects — `Device` and
            // `OutputStream` are `Send` supertrait-bound.
            // `DeviceEnumerator` deliberately carries no `Send` bound
            // (control-thread object); nothing asserts it either way.
            fn requires_send<T: Send>() {}
            requires_send::<Box<dyn Device>>();
            requires_send::<Box<dyn OutputStream>>();
            requires_send::<OutputDataCallback>();
            requires_send::<OutputErrorCallback>();
            requires_send::<Box<dyn InputStream>>();
            requires_send::<InputDataCallback>();
            requires_send::<InputErrorCallback>();
            requires_send::<DeviceError>();
            requires_send::<OutputStreamConfig>();
            requires_send::<InputStreamConfig>();
            requires_send::<BufferSize>();
            requires_send::<SupportedBufferSize>();
            requires_send::<SupportedOutputFormat>();
            requires_send::<SupportedInputFormat>();
            requires_send::<DeviceId>();
            requires_send::<OutputTimestamp>();
            requires_send::<OutputCallbackInfo>();
            requires_send::<InputTimestamp>();
            requires_send::<InputCallbackInfo>();
            requires_send::<StreamLatency>();

            let enumerator: Box<dyn DeviceEnumerator> = Box::new(StubEnumerator);
            let devices = enumerator.output_devices().unwrap();
            assert_eq!(devices.len(), 1);
            let device = enumerator.default_output_device().unwrap().unwrap();
            let stream = device
                .open_output_stream(
                    &OutputStreamConfig {
                        format: stub_format(),
                        buffer_size: BufferSize::Default,
                    },
                    Box::new(|_out, _info| {}),
                    Box::new(|_error| {}),
                )
                .unwrap();
            borrow_objects(&*enumerator, &*device, &*stream);
        }

        /// A plain `FnMut(AudioBufferViewMut<'_, f32>,
        /// OutputCallbackInfo) + Send + 'static` closure coerces onto
        /// `OutputDataCallback`'s `for<'a>` bound (HRTB).
        #[test]
        fn output_data_callback_accepts_hrtb_closure() {
            let frames_seen = Arc::new(AtomicUsize::new(0));
            let seen = Arc::clone(&frames_seen);
            let mut cb: OutputDataCallback = Box::new(
                move |out: AudioBufferViewMut<'_, f32>, info: OutputCallbackInfo| {
                    seen.fetch_add(out.frames(), Ordering::Relaxed);
                    assert_eq!(info.timestamp.callback.base(), DEVICE_CLOCK_TIMEBASE);
                },
            );
            // Drive the one closure from stack slabs of different
            // lifetimes — the `for<'a>` bound is what makes a single
            // callback serve every borrow.
            for frames in [4_usize, 2] {
                let mut scratch = vec![0.0_f32; 2 * frames];
                cb(
                    AudioBufferViewMut::try_new(&mut scratch, 2, frames).unwrap(),
                    OutputCallbackInfo {
                        timestamp: OutputTimestamp {
                            callback: MediaTime::new(0, DEVICE_CLOCK_TIMEBASE),
                            playback: Some(MediaTime::new(1_000_000, DEVICE_CLOCK_TIMEBASE)),
                        },
                    },
                );
            }
            assert_eq!(frames_seen.load(Ordering::Relaxed), 6);
        }

        /// `DeviceError`: `Copy` + `Display` + `core::error::Error` —
        /// the allocation-free normalized error contract.
        #[test]
        fn device_error_is_copy_display_and_std_error() {
            fn requires_copy<T: Copy>() {}
            fn requires_clone<T: Clone>() {}
            fn requires_std_error<T: core::error::Error>() {}

            requires_copy::<DeviceError>();
            requires_copy::<OutputTimestamp>();
            requires_copy::<OutputCallbackInfo>();
            requires_copy::<InputTimestamp>();
            requires_copy::<InputCallbackInfo>();
            requires_copy::<StreamLatency>();
            requires_copy::<LatencyCoverage>();
            requires_copy::<OutputStreamConfig>();
            requires_copy::<InputStreamConfig>();
            requires_copy::<BufferSize>();
            requires_copy::<SupportedBufferSize>();
            // `Clone`-only value types (growth headroom).
            requires_clone::<DeviceId>();
            requires_clone::<SupportedOutputFormat>();
            requires_clone::<SupportedInputFormat>();

            // Standard error contract — usable as `&dyn Error`.
            requires_std_error::<DeviceError>();
            let as_std: &dyn core::error::Error = &DeviceError::StreamInvalidated;
            assert_ne!(as_std.to_string(), "");

            // `Copy` semantics: the original stays usable.
            let original = DeviceError::Backend("device_changed");
            let copied = original;
            assert_eq!(original, copied);

            // Every variant produces non-empty fixed text; `Backend`
            // emits its classification name verbatim.
            for error in [
                DeviceError::DeviceNotFound,
                DeviceError::DeviceBusy,
                DeviceError::UnsupportedConfig,
                DeviceError::Unsupported,
                DeviceError::Xrun,
                DeviceError::RealtimeDenied,
                DeviceError::StreamInvalidated,
                DeviceError::ClockDiscontinuity,
                DeviceError::Backend("backend_failure"),
            ] {
                assert_ne!(error.to_string(), "");
            }
            assert_eq!(
                DeviceError::Backend("device_changed").to_string(),
                "device_changed"
            );

            // `DeviceResult` is the module's `Result` alias.
            let failure: DeviceResult<()> = Err(DeviceError::Xrun);
            assert_eq!(failure, Err(DeviceError::Xrun));
        }

        /// The public enums carry the intended variants.
        ///
        /// `#[non_exhaustive]` itself is a compile-time property that is
        /// invisible to `match` inside the defining crate — the
        /// `clippy::exhaustive_enums` lint gate (`mise run lint`,
        /// `-D warnings`) enforces it on every public enum. These
        /// assertions pin the variants and value semantics the gate
        /// protects.
        #[test]
        fn public_enums_expose_the_intended_variants() {
            assert_eq!(BufferSize::default(), BufferSize::Default);
            assert_ne!(BufferSize::Default, BufferSize::Fixed(0));
            assert!(matches!(
                SupportedBufferSize::Unknown,
                SupportedBufferSize::Unknown
            ));
            let range = SupportedBufferSize::Range { min: 32, max: 1024 };
            assert!(matches!(range, SupportedBufferSize::Range { min, max } if min <= max));
            assert!(matches!(
                DeviceError::Backend("xrun_minor"),
                DeviceError::Backend("xrun_minor")
            ));
        }

        /// The mock-backend walk: enumerate → default → describe →
        /// open → scripted callback → start/stop/format/now, entirely
        /// through `Box<dyn …>` — the replaceability contract.
        #[test]
        fn mock_backend_walks_the_port_flow() {
            let enumerator: Box<dyn DeviceEnumerator> = Box::new(StubEnumerator);

            let devices = enumerator.output_devices().unwrap();
            assert_eq!(devices.len(), 1);
            let listed = &*devices[0];
            assert_eq!(listed.name().unwrap(), "stub-device");
            assert_eq!(
                listed.id().unwrap(),
                Some(DeviceId::new("stub-0".to_owned()))
            );
            assert_eq!(listed.default_output_format().unwrap(), stub_format());
            let space = listed.supported_output_formats().unwrap();
            assert_eq!(space.len(), 1);
            assert_eq!(space[0].channels, 2);
            assert_eq!(
                space[0].buffer_size,
                SupportedBufferSize::Range { min: 32, max: 1024 }
            );

            let device = enumerator.default_output_device().unwrap().unwrap();
            let frames_seen = Arc::new(AtomicUsize::new(0));
            let errors_seen = Arc::new(AtomicUsize::new(0));
            let seen = Arc::clone(&frames_seen);
            let err_seen = Arc::clone(&errors_seen);
            let config = OutputStreamConfig {
                format: device.default_output_format().unwrap(),
                buffer_size: BufferSize::Default,
            };
            let stream: Box<dyn OutputStream> = device
                .open_output_stream(
                    &config,
                    Box::new(move |out: AudioBufferViewMut<'_, f32>, _info| {
                        seen.fetch_add(out.frames(), Ordering::Relaxed);
                    }),
                    Box::new(move |_error: DeviceError| {
                        err_seen.fetch_add(1, Ordering::Relaxed);
                    }),
                )
                .unwrap();

            stream.start().unwrap();
            assert_eq!(stream.format(), config.format);
            assert_eq!(stream.buffer_size().unwrap(), BufferSize::Fixed(512));
            assert_eq!(
                stream.now().unwrap(),
                MediaTime::new(0, DEVICE_CLOCK_TIMEBASE)
            );
            stream.stop().unwrap();
            drop(stream);

            // The stub fired its scripted 4-frame callback once.
            assert_eq!(frames_seen.load(Ordering::Relaxed), 4);
            assert_eq!(errors_seen.load(Ordering::Relaxed), 0);
        }
    }
}
