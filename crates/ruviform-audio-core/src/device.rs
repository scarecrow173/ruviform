//! Device boundary ports — the trait surface between the audio engine
//! (`ruviform-audio`, not yet created) and device adapter crates
//! (`ruviform-audio-device`), plus the value types, callback types and
//! the normalized [`DeviceError`] they exchange.
//!
//! The port is three object-safe traits mirroring the lifecycle split of
//! real backends: [`DeviceEnumerator`] (one-shot enumeration), [`Device`]
//! (a lightweight live handle that describes and opens) and
//! [`OutputStream`] (owns OS resources and the running callback). The
//! engine exchanges them as `Box<dyn …>` so backends can be substituted
//! without API breakage.
//!
//! Thread model: enumeration, description, `open`, `start`/`stop` are
//! control-thread operations — non-RT. The data callback
//! ([`OutputDataCallback`]) runs on the backend's real-time thread under
//! the RT contract documented on the type. Adapter implementations and
//! backend types never appear on this surface: an adapter maps its
//! backend errors onto [`DeviceError`].
//!
//! Contract source:
//! `docs/designs/audio-device-boundary-and-poc.md` — "ruviform-audio-core
//! の `device` モジュール（ポート定義）".

use core::fmt;

use ruviform_media_core::{MediaTime, TimeBase};

use crate::buffer::AudioBufferViewMut;
use crate::format::{AudioFormat, SampleFormat};

/// Device enumeration and default-device entry point — non-RT.
///
/// Implemented by adapters. A control-thread object: `Send`/`Sync` are
/// deliberately not required (re-enumeration and thread-affinity policy
/// belong to the engine's session arbitration).
///
/// **RT contract:** non-RT object — every method calls OS APIs and may
/// allocate.
pub trait DeviceEnumerator {
    /// Handles to the output-capable devices currently present — an
    /// empty `Vec` when none exist.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    ///
    /// # Errors
    ///
    /// [`DeviceError`] when the backend fails to enumerate (host API
    /// unavailable, permission denied, …). An empty set of devices is
    /// `Ok(vec![])`, not an error.
    fn output_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>>;

    /// The system default output device — `Ok(None)` when unset or
    /// absent ("none" is expressed in the type; it is not an error).
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    ///
    /// # Errors
    ///
    /// [`DeviceError`] when the default-device query itself fails.
    fn default_output_device(&self) -> DeviceResult<Option<Box<dyn Device>>>;
}

/// A live handle to one output device — non-RT.
///
/// `Send` so the handle can be handed off the control thread; `Sync` is
/// not required.
///
/// **RT contract:** non-RT object — every method calls OS APIs and may
/// allocate or block.
pub trait Device: Send {
    /// Human-readable device name for display. For identity, prefer
    /// [`Device::id`].
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Errors
    ///
    /// [`DeviceError`] when the name cannot be retrieved (e.g. the
    /// device vanished after enumeration).
    fn name(&self) -> DeviceResult<String>;

    /// The backend-specific opaque device identifier —
    /// `Ok(None)` when the backend exposes no stable ID ("unsupported"
    /// expressed in the type). A failed query (e.g. the device vanished
    /// after enumeration) is `Err`, distinct from `None`.
    ///
    /// **RT contract:** non-RT only — may call OS APIs and allocate.
    ///
    /// # Errors
    ///
    /// [`DeviceError`] when the identifier query fails.
    fn id(&self) -> DeviceResult<Option<DeviceId>>;

    /// The device's preferred output format.
    ///
    /// The adapter owns the first negotiation duty: considering both
    /// the backend default and the supported space, it returns an
    /// openable [`SampleFormat::F32`] form preferentially — even when
    /// the backend default is `F64` or an integer encoding, an
    /// [`AudioFormat`] synthesized from an F32-capable range keeps the
    /// caller's open flow off an immediate
    /// [`DeviceError::UnsupportedConfig`].
    ///
    /// **RT contract:** non-RT only — calls OS APIs.
    ///
    /// # Errors
    ///
    /// [`DeviceError::UnsupportedConfig`] only when no F32-capable
    /// range with a mappable [`ChannelLayout`](crate::ChannelLayout)
    /// exists. An unmappable *default* channel count is not itself an
    /// error: the adapter may substitute a different-channel F32
    /// format, so inspect the returned [`AudioFormat`] before use.
    fn default_output_format(&self) -> DeviceResult<AudioFormat>;

    /// The supported output-format space as a list of range
    /// descriptions — the input to format negotiation.
    ///
    /// Only *usable* F32-capable ranges are reported: integer-only
    /// ranges and structurally invalid reports (inverted or
    /// zero-ceiling bounds, zero channel count) are filtered out by
    /// the adapter, so an empty `Vec` means "no usable F32-capable
    /// range" — distinct from a failed query, which is `Err`.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    ///
    /// # Errors
    ///
    /// [`DeviceError`] when the supported-format query itself fails.
    fn supported_output_formats(&self) -> DeviceResult<Vec<SupportedOutputFormat>>;

    /// Opens an output stream with the negotiated concrete `config`.
    ///
    /// The callbacks fire on the backend's real-time thread after
    /// [`OutputStream::start`]. There is deliberately no `open_timeout`
    /// parameter in v1 — opening delegates to the backend default,
    /// which may block the calling thread indefinitely while the
    /// device is acquired; call from a thread that may block (a field
    /// on [`OutputStreamConfig`] can absorb a timeout later).
    ///
    /// **RT contract:** non-RT only — acquires OS resources and spawns
    /// threads.
    ///
    /// # Errors
    ///
    /// [`DeviceError`] — typically [`DeviceError::UnsupportedConfig`]
    /// for a configuration outside the supported space, or
    /// [`DeviceError::DeviceBusy`] when the device is exclusively held.
    fn open_output_stream(
        &self,
        config: &OutputStreamConfig,
        data_callback: OutputDataCallback,
        error_callback: OutputErrorCallback,
    ) -> DeviceResult<Box<dyn OutputStream>>;
}

/// An opened output stream. Dropping the stream stops output
/// (backend-defined — the contract is that `Drop` stops delivery).
/// `Send` so the stream can be controlled from another thread.
///
/// **RT contract:** mixed — only [`OutputStream::format`] is
/// RT-suitable; `start`/`stop`/`buffer_size`/`now` are non-RT
/// control-path operations.
pub trait OutputStream: Send {
    /// Starts or resumes output delivery.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and may block
    /// briefly.
    ///
    /// # Errors
    ///
    /// [`DeviceError`] when the backend refuses or the stream was
    /// invalidated.
    fn start(&self) -> DeviceResult<()>;

    /// Stops output delivery.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and may block
    /// briefly.
    ///
    /// # Errors
    ///
    /// [`DeviceError`] when the backend refuses or the stream was
    /// invalidated.
    fn stop(&self) -> DeviceResult<()>;

    /// The format the stream was opened with. Whether it is the
    /// post-open truth is backend-defined — backends that cannot
    /// report the applied format return the requested one, so treat
    /// the value as the negotiated intent, not a measured fact.
    ///
    /// **RT contract:** RT-suitable — returns a stored value.
    #[must_use]
    fn format(&self) -> AudioFormat;

    /// Queries the backend for the actual callback buffer size:
    /// `Ok(n)` maps to `Ok(BufferSize::Fixed(n))` — a backend-reported
    /// estimate, not a guaranteed bound; observe the real frame count
    /// per callback via `out.frames()`/`OutputCallbackInfo`.
    ///
    /// The query is fallible rather than echoing the requested value:
    /// a query failure must not be conflated with "backend default".
    ///
    /// **RT contract:** non-RT — a backend query may be involved.
    /// Control-path API; prefer [`OutputCallbackInfo`] inside callbacks.
    ///
    /// # Errors
    ///
    /// [`DeviceError`] when the backend cannot report a size (e.g.
    /// unsupported operation).
    fn buffer_size(&self) -> DeviceResult<BufferSize>;

    /// The current device-clock value — the `MasterClock` wiring input.
    ///
    /// Expressed as a [`MediaTime`] at [`DEVICE_CLOCK_TIMEBASE`]
    /// (nanoseconds). Backend conversion saturates-fails: an instant
    /// that does not fit `i64` ticks is `Err`.
    ///
    /// **RT contract:** non-RT — whether the query is RT-suitable is
    /// backend-dependent, so this is a control-path API. Inside
    /// callbacks, prefer [`OutputTimestamp`], which is always
    /// RT-suitable.
    ///
    /// # Errors
    ///
    /// [`DeviceError`] when the backend cannot report a clock or the
    /// value overflows.
    fn now(&self) -> DeviceResult<MediaTime>;
}

/// Open configuration for an output stream — negotiated concrete
/// values.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::device::{BufferSize, OutputStreamConfig};
/// use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
///
/// let config = OutputStreamConfig {
///     format: AudioFormat {
///         sample_rate: SampleRate::new(48_000).unwrap(),
///         layout: ChannelLayout::Stereo,
///         sample_format: SampleFormat::F32,
///     },
///     buffer_size: BufferSize::Default,
/// };
/// assert_eq!(config.buffer_size, BufferSize::Default);
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OutputStreamConfig {
    /// Stream format (rate + layout + sample encoding).
    pub format: AudioFormat,
    /// Requested frames-per-callback.
    pub buffer_size: BufferSize,
}

/// Requested or reported frames-per-callback.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::device::BufferSize;
///
/// assert_eq!(BufferSize::default(), BufferSize::Default);
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum BufferSize {
    /// Let the backend choose (the default). The actual frames per
    /// callback may then vary — variable-frame handling is the
    /// adapter's duty, observable via `out.frames()` per callback.
    #[default]
    Default,
    /// Request a fixed frame count. Whether it is honored is
    /// backend-dependent.
    ///
    /// Adapter duty: values that overflow the backend's `u32` frame
    /// count (and `Fixed(0)`) are rejected with
    /// [`DeviceError::UnsupportedConfig`] before open.
    Fixed(usize),
}

/// Supported buffer-size capability of a device.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SupportedBufferSize {
    /// The backend reports no range information.
    Unknown,
    /// Inclusive `[min, max]` frame counts.
    ///
    /// Invariant: `min <= max` — the adapter validates backend-reported
    /// values while mapping and either drops violating ranges or folds
    /// them into [`SupportedBufferSize::Unknown`].
    Range { min: usize, max: usize },
}

/// One entry of a device's supported output-format space (a range
/// description).
///
/// A non-RT descriptor produced by adapters. `channels` is a plain
/// count; mapping it to a [`ChannelLayout`](crate::ChannelLayout) is a
/// negotiation-time duty.
///
/// Deliberately `Clone` but not `Copy`: keeps headroom for future
/// non-`Copy` fields in the supported-space description.
///
/// **RT contract:** non-RT — a description value built on the control
/// path (fields are `Copy`, construction itself is cheap, but the type
/// is obtained through allocating enumeration APIs).
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::device::{SupportedBufferSize, SupportedOutputFormat};
/// use ruviform_audio_core::SampleFormat;
///
/// let range = SupportedOutputFormat {
///     channels: 2,
///     min_sample_rate: 44_100,
///     max_sample_rate: 48_000,
///     buffer_size: SupportedBufferSize::Range { min: 32, max: 1024 },
///     sample_format: SampleFormat::F32,
/// };
/// assert_eq!(range.channels, 2);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct SupportedOutputFormat {
    /// Channel count.
    pub channels: u32,
    /// Minimum supported sample rate in Hz (inclusive).
    pub min_sample_rate: u32,
    /// Maximum supported sample rate in Hz (inclusive).
    pub max_sample_rate: u32,
    /// Buffer-size capability.
    pub buffer_size: SupportedBufferSize,
    /// PCM sample encoding.
    pub sample_format: SampleFormat,
}

/// Opaque backend-specific device identifier.
///
/// The string representation is an implementation detail: adapters
/// build values from backend-reported identifiers, and users may
/// display (`Debug`), persist and compare them — never interpret the
/// content. Obtained on the non-RT path only ([`Device::id`]).
///
/// **RT contract:** non-RT — construction and comparison may involve
/// the owned string.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::device::DeviceId;
///
/// let id = DeviceId::new("backend:default-0".to_owned());
/// assert_eq!(id, DeviceId::from("backend:default-0".to_owned()));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeviceId(String);

impl DeviceId {
    /// Builds an identifier from a backend-reported string — the public
    /// constructor adapters use.
    ///
    /// **RT contract:** non-RT only — may allocate.
    #[must_use]
    pub fn new(id: String) -> Self {
        Self(id)
    }
}

impl From<String> for DeviceId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

/// Timebase convention for device-clock instants carried by
/// [`OutputTimestamp`] and [`OutputStream::now`]:
/// `TimeBase(1, 1_000_000_000)` — a nanosecond monotonic clock whose
/// epoch is backend-defined; only differences between instants are
/// meaningful.
///
/// Reusing [`MediaTime`] keeps `MasterClock` wiring free of a new time
/// type — consistent with the "no `f64` seconds as a core primitive"
/// contract.
///
/// **RT contract:** RT-suitable — `Copy` constant.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::device::DEVICE_CLOCK_TIMEBASE;
///
/// assert_eq!(DEVICE_CLOCK_TIMEBASE.numerator(), 1);
/// assert_eq!(DEVICE_CLOCK_TIMEBASE.denominator(), 1_000_000_000);
/// ```
pub const DEVICE_CLOCK_TIMEBASE: TimeBase = match TimeBase::new(1, 1_000_000_000) {
    Ok(base) => base,
    // `TimeBase::new` fails only on a zero numerator or denominator —
    // 1/1_000_000_000 cannot fail. `panic!` with a literal is
    // const-evaluable; `unreachable!`'s formatting expansion is not.
    Err(_) => panic!("invalid device clock timebase"),
};

/// The device-clock timestamps attached to one output callback.
///
/// **RT contract:** RT-suitable — `Copy` value type; the adapter builds
/// it on the stack inside the callback (no allocation).
#[derive(Copy, Clone, Debug)]
pub struct OutputTimestamp {
    /// The device-clock instant at which the callback fired —
    /// [`MediaTime`] at [`DEVICE_CLOCK_TIMEBASE`] (nanoseconds).
    pub callback: MediaTime,
    /// The device-clock instant at which the buffer written in this
    /// callback reaches the DAC (backend-reported, including hardware
    /// latency) — `None` when the backend cannot report it
    /// ("unsupported" expressed in the type).
    ///
    /// `MasterClock` wiring prefers `playback`, falling back to
    /// `callback`; the wiring itself is the engine's duty.
    pub playback: Option<MediaTime>,
}

/// Runtime information container passed to the output data callback.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// A struct — not a bare timestamp — so that future per-callback
/// information is absorbed as field additions rather than signature
/// changes (e.g. `RtContext` wiring). Note: adding a non-`Copy` field
/// can lose `Copy` conformance — fields added here must remain
/// RT-contract value types.
#[derive(Copy, Clone, Debug)]
pub struct OutputCallbackInfo {
    /// This callback's device-clock timestamp.
    pub timestamp: OutputTimestamp,
}

/// The output data callback, registered at
/// [`Device::open_output_stream`] and driven by the backend's real-time
/// thread after [`OutputStream::start`].
///
/// **RT contract:** inside the callback is an RT-contract section —
/// no alloc/dealloc; blocking, file/network I/O, unbounded waits and
/// unbounded processing are restricted (realtime ADR). `out` is a
/// bounded view built on a slab the adapter pre-allocated; read the
/// actual frame count per callback via `out.frames()` — it may vary
/// under [`BufferSize::Default`]. `Send`: the closure is transferred
/// to the backend thread.
///
/// # Lifecycle and usage contract (implementation duties)
///
/// - The callback must write **every** frame of `out` on every call.
///   The scratch is reused: unwritten leftovers are delivered as the
///   previous callback's stale data. The adapter may zero-fill the
///   exposed region before calling as a safe default (bounded memset —
///   RT-suitable).
/// - Where the callback object is dropped is backend-defined and may
///   be on the RT thread: keep captured values free of allocating or
///   blocking `Drop`s, and offload heavy teardown to another thread
///   engine-side (drop-offload).
/// - The callback must not panic — a panic on the backend RT thread
///   can lead to undefined behavior or abnormal process termination.
/// - Re-entering the stream control APIs ([`OutputStream::start`],
///   [`OutputStream::stop`], `Drop`) from inside the callback is
///   undefined — prohibited.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::device::{OutputCallbackInfo, OutputDataCallback};
/// use ruviform_audio_core::AudioBufferViewMut;
///
/// // A `FnMut + Send + 'static` closure coerces onto the HRTB
/// // (`for<'a>`) callback type.
/// let mut cb: OutputDataCallback =
///     Box::new(|mut out: AudioBufferViewMut<'_, f32>, _info: OutputCallbackInfo| {
///         for channel in 0..out.channels() {
///             out.channel_mut(channel).unwrap().fill(0.0);
///         }
///     });
///
/// // Driven by an adapter on its RT thread; here we fire it manually
/// // over a stack scratch slab — no device is touched.
/// use ruviform_audio_core::device::{DEVICE_CLOCK_TIMEBASE, OutputTimestamp};
/// use ruviform_media_core::MediaTime;
///
/// let mut scratch = [0.0_f32; 8];
/// let info = OutputCallbackInfo {
///     timestamp: OutputTimestamp {
///         callback: MediaTime::new(0, DEVICE_CLOCK_TIMEBASE),
///         playback: None,
///     },
/// };
/// cb(AudioBufferViewMut::try_new(&mut scratch, 2, 4).unwrap(), info);
/// assert_eq!(scratch, [0.0; 8]);
/// ```
pub type OutputDataCallback =
    Box<dyn for<'a> FnMut(AudioBufferViewMut<'a, f32>, OutputCallbackInfo) + Send + 'static>;

/// Stream-error notification callback (xrun, invalidation, advisory).
///
/// May run on the backend's stream thread — treat it as RT-adjacent.
/// [`DeviceError`] is `Copy` precisely so this path never allocates;
/// implementations must not allocate or block either. Like
/// [`OutputDataCallback`], the callback must not panic — a panic on
/// the backend stream thread can lead to undefined behavior or
/// abnormal process termination.
///
/// A non-fatal advisory may arrive ([`DeviceError::Backend`]
/// `"device_changed"` — automatic re-routing of a default-device
/// stream): the stream is still running and needs no rebuild; the
/// engine treats it as guidance, not as an error stop.
///
/// **RT contract:** RT-adjacent — no allocation, no blocking.
pub type OutputErrorCallback = Box<dyn FnMut(DeviceError) + Send + 'static>;

/// The normalized error type of the device boundary.
///
/// Adapters map backend-specific errors onto this type; no external
/// error type is exposed. `Copy` + `&'static str` payloads so a value
/// can be produced on RT-adjacent paths without allocation — the same
/// technique as media-core's `Error`.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::device::DeviceError;
///
/// // Copy — no allocation, safe to emit on the error-callback path.
/// let error = DeviceError::Xrun;
/// let copy = error;
/// assert_eq!(error, copy);
/// assert_ne!(error.to_string(), "");
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DeviceError {
    /// The device does not exist or vanished after enumeration.
    DeviceNotFound,
    /// The device or backend is busy — retryable on the control path.
    DeviceBusy,
    /// The requested configuration (format, layout or buffer size) is
    /// unsupported by the device.
    UnsupportedConfig,
    /// The backend does not support the feature itself (e.g.
    /// timestamps, duplex, host selection).
    Unsupported,
    /// Buffer under/overrun (xrun).
    Xrun,
    /// Real-time thread priority promotion was denied.
    RealtimeDenied,
    /// The stream was invalidated backend-side (device removal etc.).
    StreamInvalidated,
    /// A backend failure not mappable to the variants above. Carries
    /// only a non-allocating `&'static str` classification name — the
    /// backend's message string is deliberately not captured.
    Backend(&'static str),
}

/// Result alias for the `device` module.
///
/// **RT contract:** RT-suitable — [`DeviceError`] is `Copy` and
/// allocation-free, so producing a `DeviceResult` never allocates.
pub type DeviceResult<T> = Result<T, DeviceError>;

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotFound => {
                f.write_str("device not found (absent or removed since enumeration)")
            }
            Self::DeviceBusy => {
                f.write_str("device or backend is busy; retryable on the control path")
            }
            Self::UnsupportedConfig => {
                f.write_str("requested configuration is unsupported by the device")
            }
            Self::Unsupported => f.write_str("the backend does not support this feature"),
            Self::Xrun => f.write_str("buffer under/overrun (xrun)"),
            Self::RealtimeDenied => f.write_str("real-time thread priority promotion was denied"),
            Self::StreamInvalidated => f.write_str("the stream was invalidated by the backend"),
            // The classification name is emitted as-is — the only
            // payload carried.
            Self::Backend(name) => f.write_str(name),
        }
    }
}

impl core::error::Error for DeviceError {}
