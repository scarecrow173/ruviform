---
id: "DESIGN-034Qhua2OtuBq26hwS8LpX"
type: "design"
status: "approved"
title: "Issue #191 PCM WAV output backend design"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source:
    - "docs/specs/issue-191-pcm-wav-output-backend.md"
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-191-pcm-wav-output-backend.md"
  implemented-by: []
  depends-on:
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/media-io-backend-abstraction.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-191-pcm-wav-output-backend.md"
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #191 PCM WAV output backend design

## Context

Ruviform's media-I/O boundary (`ruviform-media-io`) exposes `Encoder`, `Muxer`, `Packet`, `StreamDescriptor`, `Capabilities`, `ErrorKind`, and the input port `MediaSource: Read + Seek + Send`. Issue #191 adds the first output backend (`ruviform-io-wav`) and the two boundary ports it needs: `MediaSink` (`Write + Seek + Send`) and `StagedOutput` (same-dir temp + fsync + rename publish). All encode/mux/FS work is non-RT by contract.

## Scope

### In Scope

- `ruviform-media-io`: `sink` module (`MediaSink` trait + `sync()` extension + blanket impls for `File`/`Cursor`/`&mut T`/`Box<T>`), `staged` module (`StagedOutput`, `PublishOptions`), re-exports; no signature changes to existing API.
- `ruviform-io-wav`: `WavEncoder` (`Encoder`), `WavMuxer` (`Muxer`), `WavCapabilities` + `wav_capabilities()`, `capabilities()` (boundary `Capabilities`), `WavReceipt`, `WavOutputSpec`, `make_encoder`/`make_muxer`/`open_muxer` entry points.
- Tests per spec scope (round-trip via `ruviform-io-symphonia` dev-dep; staged-publish matrix; capability rejection).

### Out of Scope

- RF64, BWF `bext`, other containers/codecs, WAV demux/decode in this crate, RT APIs, Synvelis product policy, dither, resampling, channel remapping.

## Components and Boundaries

### `ruviform-media-io::sink` (new module)

```rust
pub trait MediaSink: Write + Seek + Send {
    /// Flush buffered data and push to the storage layer (fsync for files).
    fn sync(&mut self) -> io::Result<()>;
}
impl MediaSink for File { fn sync(&mut self) -> io::Result<()> { self.sync_all() } }
impl<T: MediaSink + ?Sized> MediaSink for &mut T { ... }
impl<T: MediaSink + ?Sized> MediaSink for Box<T> { ... }
// Cursor<Vec<u8>>, Cursor<&mut [u8]> impls for tests: sync() = flush()
```

- `sync()` exists because `Write::flush` only reaches userspace buffers; completion requires an fsync-level operation, and generic `Write` can't express it. `File::sync` maps to `sync_all`; in-memory sinks treat `sync` as `flush`.

### `ruviform-media-io::staged` (new module)

```rust
pub struct StagedOutput {
    temp_path: PathBuf,      // <dest>.rvf-tmp-<pid>-<counter>
    dest_path: PathBuf,
    file: Option<File>,
    state: StagedState,      // Open | Published | Aborted
}
pub struct PublishOptions { pub replace_existing: bool, pub sync_before_publish: bool }

impl StagedOutput {
    pub fn create(dest: impl AsRef<Path>) -> Result<Self>;            // O_EXCL temp, ≤8 name retries
    pub fn sink(&mut self) -> Result<&mut File>;                      // &mut File as MediaSink
    pub fn take_file(&mut self) -> Result<File>;                      // hand File to muxer
    pub fn publish(mut self, opts: &PublishOptions) -> Result<()>;    // fsync → rename
    pub fn abort(self);                                               // rm temp
}
impl Drop for StagedOutput { /* rm temp if still Open */ }
```

- Temp lives in the destination's directory ⇒ same volume ⇒ `fs::rename` is a same-FS rename (best-effort atomic; no cross-FS claim).
- `publish(replace_existing: false)` + dest exists → `ErrorKind::InvalidValue`-style conflict error, temp removed, dest byte-identical. `true` → `fs::rename` overwrites (Rust std uses `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` on Windows, `rename(2)` on POSIX).
- Product policy (should-this-export-overwrite) is *not* in this type; callers pass the flag.

### `ruviform-io-wav`

```rust
pub struct WavCapabilities {
    pub container: ContainerFormat,                 // Wav
    pub rf64: bool,                                 // false
    pub bwf: bool,                                  // false
    pub sample_formats: &'static [SampleFormat],    // [U8, S16, S24, S32, F32]
    pub max_data_bytes: u64,                        // u32::MAX
    pub max_streams: usize,                         // 1
    pub channel_layouts: ChannelLayoutSupport,      // All (named + validated custom masks)
}
pub fn wav_capabilities() -> WavCapabilities;
pub fn capabilities() -> Capabilities;              // MUX|ENCODE, encode_codecs=[Pcm]
```

```rust
pub struct WavEncoder { descriptor: CodecDescriptor, wire: SampleFormat,
                        channels: u16, rate: u32, buffered: Vec<u8>,
                        next_index: u64, first_pts: Option<i64>, drained: bool }
impl WavEncoder {
    pub fn new(descriptor: CodecDescriptor) -> Result<Self>;   // validates codec=Pcm,
        // format=Audio(rate,layout,sample_format in supported set); else Unsupported
}
impl Encoder for WavEncoder {
    fn descriptor(&self) -> &CodecDescriptor;
    fn send_frame(&mut self, frame: Frame) -> Result<()>;   // Frame::Audio only;
        // layout/rate must match descriptor else InvalidData; converts to wire bytes
    fn receive_packet(&mut self) -> Result<Option<Packet>>; // one packet per buffered span
    fn drain(&mut self) -> Result<()>;                      // marks end-of-input;
        // receive_packet drains remaining then returns Ok(None)
}
```

```rust
pub struct WavMuxer<S: MediaSink> { sink: S, streams: Vec<StreamDescriptor>,
    data_start: u64, data_bytes: u64, finished: Option<WavReceipt>,
    first_pts_ticks: Option<i64>, fmt_tag: u16 }
impl<S: MediaSink> WavMuxer<S> {
    pub fn new(sink: S, streams: &[StreamDescriptor]) -> Result<Self>;  // writes header now
    pub fn finalize(&mut self) -> Result<WavReceipt>;                   // idempotent-error:
        // pad→backpatch riff+data sizes→flush→sync→receipt; second call InvalidData
    pub fn receipt(&self) -> Option<&WavReceipt>;
}
impl<S: MediaSink> Muxer for WavMuxer<S> {
    fn streams(&self) -> &[StreamDescriptor];
    fn write_packet(&mut self, p: Packet) -> Result<()>;   // appends p.data to data chunk;
        // after finalize → InvalidData; size > u32::MAX total → InvalidData
    fn finish(&mut self) -> Result<()> { self.finalize().map(|_| ()) }
}
```

```rust
pub struct WavReceipt { pub total_frames: u64, pub total_bytes: u64,
    pub duration_ticks: Option<u64>, pub first_pts_ticks: Option<i64> }
```

Entry points (mirroring `io-symphonia`):
`capabilities()`, `wav_capabilities()`, `make_encoder(descriptor) -> Result<Box<dyn Encoder>>`, `make_muxer(streams) -> Result<...>` (rejects non-file use: requires caller to `open_muxer(sink, streams)` — returns `Unsupported` for the sinkless form unless we give it a `File` target), `open_muxer<S: MediaSink>(sink, streams) -> Result<WavMuxer<S>>`.

Encode-side `CodecDescriptor`: `codec = Pcm`; `format = CodecFormat::Audio(AudioFormat{ sample_rate, channel_layout, sample_format: <wire format> })`; `native_format = Fact::NotDeclared` (meaningless encode-side; the wire encoding *is* `format` — documented, no fabricated value).

## Data and Control Flow

Write path (Frame→Packet→WAV):

1. `WavEncoder::send_frame(Frame::Audio(f))` — validate `f.channels == descriptor.channels` and `f.sample_rate == descriptor.rate` (`InvalidData` otherwise); for each of `n` frames interleave channels L→R→…; convert per `wire` format; append to `buffered`; capture `first_pts`.
2. `receive_packet()` — if `buffered` non-empty: emit `Packet { data: take(buffered), pts: first/tracked pts ticks, duration: frames, index: next_index++, flags: KEYFRAME }`. If `drained && buffered empty` → `Ok(None)` (end-of-stream).
3. `WavMuxer::new` — write `RIFF/0/WAVE` + `fmt ` chunk (WAVEFORMATEX 16B for Mono/Stereo int-tag-1/float-tag-3; WAVEFORMATEXTENSIBLE 40B tag 0xFFFE + subformat GUID + channel mask for other layouts) + `data/0` placeholder; record `data_start`.
4. `write_packet` — `data_bytes += p.data.len()`; on overflow of `u32::MAX` → `InvalidData` (no RF64).
5. `finalize()` — if odd `data_bytes` write pad byte; seek to `4` → write `riff_size = file_len - 8`; seek to `data_start - 4` → write `data_bytes`; seek end; `flush`; `sync()`; build `WavReceipt` (`total_frames = data_bytes / block_align`, `duration_ticks = total_frames` in 1/rate base).
6. Publish — caller: `muxer.finalize()?` → `staged.publish(&opts)?`. Any earlier failure → drop `StagedOutput` → temp removed, dest untouched.

Conversion (per sample, deterministic): `f32` → int: `x.clamp(-1.0, 1.0)` then round-half-away to full scale (`S16: x*32768`, `S24: x*8388608`, `S32: x*2147483648`, `U8: (x+1)*128`); `NaN → 0`; `±Inf → ±max` via clamp. → `F32`: passthrough. No dither.

## Non-Functional Constraints

- Non-RT boundary: all encode/mux/FS calls documented non-RT; no `Send`-unsafety tricks.
- DAG: `ruviform-io-wav` depends only on `ruviform-media-io` (+ transitives); `ruviform-media-io` gains zero new deps; dev-dep `ruviform-io-symphonia` is test-only and introduces no cycle (symphonia→media-io, wav→media-io, wav -dev→ symphonia).
- Error honesty: every failure maps to the smallest `ErrorKind`; no fabricated `Fact`s; no silent format widening.

## Risks and Trade-offs

- Symphonia must parse WAVEFORMATEXTENSIBLE output for >2ch round-trip tests — risk mitigated by testing; fallback is plain `WAVEFORMATEX` for ≤2ch which every reader handles, and extensible is the standard form for multichannel PCM.
- `Muxer::finish() -> Result<()>` discards the receipt for `dyn Muxer` users — accepted; concrete `WavMuxer::finalize()`/`receipt()` expose it, and the trait stays untouched.
- `StagedOutput` in boundary crate widens its surface — accepted per ADR; alternative (per-crate) duplicates policy.
- `native_format = NotDeclared` on encode side is a convention, not a type-system guarantee — documented in impl record + module docs.

## Deferred Design Concerns

- RF64/BWF — spec `defers`; trigger = consumer needing >4 GiB/broadcast metadata.
- Generic boundary-level format capability map — trigger = second output backend.
- Seek-less streaming output — trigger = pipe/network sink consumer.
- Dither options — trigger = downstream mastering request.

## Verification Notes

- [ ] Round-trip write→reopen via `ruviform-io-symphonia` for rate/layout/depth matrix (AC1)
- [ ] Edge cases: 0-frame, channel mismatch, NaN/Inf/clip, unsupported codec/format (AC2, AC3)
- [ ] drain/finalize/finish/write-after-finish/write-error distinction (AC4)
- [ ] StagedOutput: success/abort/disk-full-mock/replace/fail-if-exists (AC5)
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`verify-ci` + `cargo test --workspace --locked` (AC6)
