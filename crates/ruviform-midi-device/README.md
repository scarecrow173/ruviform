# ruviform-midi-device

OS MIDI input device adapter for Ruviform — implements the
`ruviform_audio_core::midi_input` port traits
(`MidiInputEnumerator` / `MidiInputPort` / `MidiInput`) on the host OS's
native MIDI API.

- **Windows**: WinMM (`winmm.dll`) via `windows-sys` — real port
  enumeration, `CALLBACK_FUNCTION | MIDI_IO_STATUS` subscriptions,
  channel-voice decode, driver timestamps preserved as `InputStamp`
  (`Carried` provenance), bounded queue with observable
  `dropped`/`unsupported` accounting.
- **macOS / Linux**: `DeviceError::Unsupported` boundary only —
  backends are deferred to later Issue #253 slices.

The crate is the only place OS MIDI APIs are linked; WinMM types and
errors never appear on the public surface. Adapter layer: depends only
on `ruviform-audio-core` + `ruviform-media-core` (+ `windows-sys` on
Windows). Design/ADR: `docs/designs/issue-253-os-midi-backend.md`,
`docs/adr/select-midi-input-backend.md`.

```rust,no_run
use ruviform_audio_core::midi_input::MidiInputEnumerator;
use ruviform_midi_device::backend;

let ports = backend().input_ports()?;
for port in ports {
    println!("{}", port.info().name);
}
```

## Opt-in real-device verification

`#[ignore]`-gated E2E (`tests/ports.rs::real_device_open_read_observe_close`)
exercises the full real-device path — enumerate → open → repeated
`read` (timestamp domain/provenance + `dropped`/`unsupported`
observation) → close → `StreamInvalidated`. It exits honestly on
platforms without a backend (`DeviceError::Unsupported`) or with no
input ports; hardware traffic (nonzero counters, actual events) is
recorded, never asserted.

Interim entry point — canonical mise lane registration is a recorded
follow-up (Issue #253 slice: `mise.toml` was outside the slice's edit
scope; see `docs/impl/ir/issue-253-unsupported-transparency-slice.md`):

```sh
cargo test -p ruviform-midi-device -- --ignored
```

`examples/list_midi_ports.rs` remains the manual enumeration helper.
The bridge-level real path (`SystemBackend` → `InputTx::pump` →
`InputStage`) is covered by the `#[ignore]` E2E in
`ruviform-audio-plugin-bridge/tests/midi_wiring.rs`:

```sh
cargo test -p ruviform-audio-plugin-bridge -- --ignored
```
