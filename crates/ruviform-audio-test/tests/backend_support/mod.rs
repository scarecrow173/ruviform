//! Shared fixtures for the scripted-backend integration tests.
//!
//! Not a test crate itself — included via `mod backend_support;` from
//! `tests/backend.rs` and `tests/backend_faults.rs`.

#![allow(dead_code)]

use std::sync::Arc;

use ruviform_audio_core::device::LatencyCoverage;
use ruviform_audio_core::device::{
    BufferSize, InputStreamConfig, OutputStreamConfig, StreamLatency, SupportedBufferSize,
    SupportedInputFormat, SupportedOutputFormat,
};
use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_audio_test::backend::{DeviceSpec, InputSpec, OutputSpec, StreamScript};
use ruviform_audio_test::clock::{JitterSpec, VirtualClock, VirtualClockConfig};
use ruviform_audio_test::pcm::{PcmFixture, PcmPattern, PcmScenario, PcmSegment};
use ruviform_media_core::{ClockDomainId, TimestampProvenance};

/// 48 kHz / epoch 7 / no drift / no jitter — the canonical test clock.
pub fn test_clock() -> Arc<VirtualClock> {
    Arc::new(
        VirtualClock::new(VirtualClockConfig {
            sample_rate: 48_000,
            epoch: 7,
            offset_ticks: 0,
            drift_ppm: 0,
            jitter: JitterSpec::DISABLED,
        })
        .unwrap(),
    )
}

/// Stereo F32 @ 48 kHz — the canonical negotiated format.
pub fn stereo_format() -> AudioFormat {
    AudioFormat {
        sample_rate: SampleRate::new(48_000).unwrap(),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::F32,
    }
}

/// A sine-wave stereo fixture at 48 kHz.
pub fn stereo_fixture() -> PcmFixture {
    PcmFixture::new(
        SampleRate::new(48_000).unwrap(),
        2,
        PcmPattern::Sine {
            freq_hz: 440.0,
            amplitude: 0.5,
        },
    )
    .unwrap()
}

/// A scenario with one contiguous segment `[start, start+len)`.
pub fn pcm_scenario(start: u64, len: u64) -> PcmScenario {
    PcmScenario::new(vec![PcmSegment {
        frame_start: start,
        frame_len: len,
        fixture: stereo_fixture(),
    }])
    .unwrap()
}

/// Output capability covering stereo F32 44.1k–96k, any buffer size.
pub fn output_spec(default_frames: u32) -> OutputSpec {
    OutputSpec {
        default_format: stereo_format(),
        supported_formats: vec![SupportedOutputFormat {
            channels: 2,
            min_sample_rate: 44_100,
            max_sample_rate: 96_000,
            buffer_size: SupportedBufferSize::Unknown,
            sample_format: SampleFormat::F32,
        }],
        stream: StreamScript::new(default_frames),
        record_output: true,
    }
}

/// Input capability replaying `pcm`, with capture reporting on.
pub fn input_spec(pcm: PcmScenario, default_frames: u32) -> InputSpec {
    InputSpec {
        default_format: stereo_format(),
        supported_formats: vec![SupportedInputFormat {
            channels: 2,
            min_sample_rate: 44_100,
            max_sample_rate: 96_000,
            buffer_size: SupportedBufferSize::Unknown,
            sample_format: SampleFormat::F32,
        }],
        pcm,
        stream: StreamScript::new(default_frames),
        capture_reporting: true,
    }
}

/// A device with both directions.
pub fn device_spec(name: &str) -> DeviceSpec {
    DeviceSpec {
        name: name.to_owned(),
        id: Some(format!("{name}-id")),
        clock: None,
        output: Some(output_spec(256)),
        input: Some(input_spec(pcm_scenario(0, 65_536), 256)),
    }
}

/// Output-only device spec.
pub fn output_only_spec(name: &str) -> DeviceSpec {
    DeviceSpec {
        input: None,
        ..device_spec(name)
    }
}

/// Input-only device spec.
pub fn input_only_spec(name: &str) -> DeviceSpec {
    DeviceSpec {
        output: None,
        ..device_spec(name)
    }
}

/// `BufferSize::Default` stream config at the canonical format.
pub fn out_config() -> OutputStreamConfig {
    OutputStreamConfig {
        format: stereo_format(),
        buffer_size: BufferSize::Default,
    }
}

/// Input-direction config.
pub fn in_config() -> InputStreamConfig {
    InputStreamConfig {
        format: stereo_format(),
        buffer_size: BufferSize::Default,
    }
}

/// A scripted 64-frame latency report (≈1333.3 µs… ns at 48 kHz —
/// `64 * 1e9 / 48000 = 1_333_333` ns).
pub fn latency64() -> StreamLatency {
    StreamLatency {
        frames: 64,
        coverage: LatencyCoverage::HardwareOnly,
        provenance: TimestampProvenance::Derived,
    }
}

/// The clock domain the canonical clock reports (`epoch = 7`).
pub fn test_domain() -> ClockDomainId {
    ClockDomainId::new(7)
}
