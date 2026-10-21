//! `list_devices` — non-RT diagnostic dump of the device boundary.
//!
//! Enumerates every host reported by
//! [`ruviform_audio_device::available_host_ids`], acquires each via
//! [`ruviform_audio_device::host`], and dumps per host: the output
//! device list, the default output device, and for every device the
//! [`Device::default_output_format`] plus the
//! [`Device::supported_output_formats`] space.
//!
//! Pure control-path program — every call may allocate and call OS
//! APIs. Hardware-dependent: it is run manually on real machines and
//! never in CI (design "実機手動検証手順").

use ruviform_audio_core::device::{
    Device, DeviceEnumerator, DeviceId, SupportedBufferSize, SupportedOutputFormat,
};
use ruviform_audio_device::{HostId, available_host_ids, host};

fn main() {
    let ids = available_host_ids();
    println!("available host ids: {}", ids.len());
    for id in &ids {
        println!("  - {}", id.name());
    }
    println!();

    for id in ids {
        dump_host(id);
        println!();
    }
}

/// Dumps one host: default output device, then every enumerated output
/// device with its format information.
fn dump_host(id: HostId) {
    println!("=== host: {} ===", id.name());
    let host = match host(id) {
        Ok(host) => host,
        Err(error) => {
            println!("  host(id) failed: {error}");
            return;
        }
    };

    let default_id = match host.default_output_device() {
        Ok(Some(device)) => {
            let name = device
                .name()
                .unwrap_or_else(|_| "<name query failed>".to_owned());
            match device.id() {
                Ok(device_id) => {
                    println!("  default output device: \"{name}\"  id={device_id:?}");
                    device_id
                }
                Err(error) => {
                    println!("  default output device: \"{name}\"  id query failed: {error}");
                    None
                }
            }
        }
        Ok(None) => {
            println!("  default output device: none");
            None
        }
        Err(error) => {
            println!("  default_output_device() failed: {error}");
            None
        }
    };

    match host.output_devices() {
        Ok(devices) => {
            println!("  output devices: {}", devices.len());
            for (index, device) in devices.iter().enumerate() {
                dump_device(index, device.as_ref(), default_id.as_ref());
            }
        }
        Err(error) => println!("  output_devices() failed: {error}"),
    }
}

/// Dumps one device: name, backend id, default format and the
/// supported format space. `default_id` marks the default device.
fn dump_device(index: usize, device: &dyn Device, default_id: Option<&DeviceId>) {
    let name = device
        .name()
        .unwrap_or_else(|_| "<name query failed>".to_owned());
    let id = device.id();
    let is_default = matches!(&id, Ok(Some(device_id)) if Some(device_id) == default_id);
    let marker = if is_default { " (default)" } else { "" };
    println!("  [{index}] \"{name}\"{marker}");
    match &id {
        Ok(device_id) => println!("      id: {device_id:?}"),
        Err(error) => println!("      id: query failed: {error}"),
    }
    match device.default_output_format() {
        Ok(format) => println!("      default_output_format: {format:?}"),
        Err(error) => println!("      default_output_format: query failed: {error}"),
    }
    match device.supported_output_formats() {
        Ok(ranges) => {
            println!("      supported_output_formats: {} range(s)", ranges.len());
            for range in &ranges {
                println!("        {}", describe_range(range));
            }
        }
        Err(error) => println!("      supported_output_formats: query failed: {error}"),
    }
}

/// One-line rendering of a supported-format range.
fn describe_range(range: &SupportedOutputFormat) -> String {
    let buffer = match range.buffer_size {
        SupportedBufferSize::Unknown => "unknown".to_owned(),
        SupportedBufferSize::Range { min, max } => format!("[{min}, {max}]"),
        // `SupportedBufferSize` is `#[non_exhaustive]` — future variants
        // render as their `Debug` form rather than being silently
        // dropped.
        other => format!("{other:?}"),
    };
    format!(
        "channels={} rate=[{}, {}] buffer_size={} sample_format={:?}",
        range.channels, range.min_sample_rate, range.max_sample_rate, buffer, range.sample_format
    )
}
