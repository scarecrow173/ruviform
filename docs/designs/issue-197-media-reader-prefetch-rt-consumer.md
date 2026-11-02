---
id: "DESIGN-fl6bBJvoJRiJHSkk73mskx"
type: "design"
status: "approved"
title: "Issue #197 Generic Media Reader / Prefetch / RT Consumer — structural design"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/197"
  changes:
    added:
      - type: "dir"
        path: "crates/ruviform-media-reader/"
    modified:
      - type: "file"
        path: "crates/ruviform-media-io/src/demux.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/decode.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/adr/issue-197-media-reader-boundary-additions.md"
    - "docs/discovery/issue-197-media-reader-prefetch-rt-consumer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/adr/issue-197-media-reader-boundary-additions.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #197 Generic Media Reader / Prefetch / RT Consumer — structural design

## Overview

Issue #197 (M5) delivers the generic read path of the media foundation. Two additive `Demuxer` ports (`seekable`, `make_decoder`) close the boundary gaps found in discovery. A new `ruviform-media-reader` crate provides `SourceReader` (non-RT sequential decode in source frames with frame-accurate seek), `SourceMetadata` (capability projection), `FrameMap` (caller-supplied EngineFrame→SourceFrame contract), and a bounded prefetch pipeline: non-RT producer → fixed `PcmBlock` pool over two SPSC rings → generation-tagged `PrefetchItem` stream → `PrefetchConsumer::fill` (RT no-alloc/no-lock/no-I/O) that reports every non-content span via `FillReport`/`PrefetchTelemetry`.

## Architecture

### Boundary additions (`ruviform-media-io`)

```rust
#[non_exhaustive]
pub enum DecodeMode { Canonical, Native }

impl Demuxer {
    fn seekable(&self) -> Fact<()> { Fact::NotDeclared }          // default
    fn make_decoder(&self, id: StreamId, mode: DecodeMode)
        -> Result<Box<dyn Decoder>> { Err(unsupported) }          // default
}
```

- `Fact<()>`: `Known(())` seekable, `Unsupported` seek rejected, `NotDeclared` unknown. Symphonia plumbs its construction-time probe (capture `SourceAdapter::is_seekable()` before the adapter moves into `MediaSourceStream`); AVI returns `Unsupported` (matching its `seek` impl).
- `make_decoder` dispatches to the existing concrete `make_decoder`/`make_native_decoder` per mode. Third-party impls unaffected (defaults).

### `ruviform-media-reader` module map

| Module | Types | RT label |
| --- | --- | --- |
| `mapping` | `SourceFrame(u64)`, `EngineFrame(u64)`, `RequestedRange { start, frames }`, `FrameMap { clip_offset: i64, placement: i64, ratio_num: u32, ratio_den: u32 }` | pure value types |
| `metadata` | `SourceMetadata { container, backend, stream, canonical: AudioFormat, frame_count: Fact<u64>, seekable: Fact<()> }`, `stream_metadata(&dyn Demuxer, StreamId)` | non-RT |
| `reader` | `SourceReader`, `SeekReceipt { requested, landed, landing_provenance }`, `ReadOutcome { frames, eof }` | non-RT |
| `prefetch` | `PrefetchConfig`, `Prefetch { handle, consumer, producer_thread }`, `PrefetchHandle`, `PrefetchConsumer`, `PrefetchProducer`, `PcmBlock`, `PrefetchItem`, `SourceFault`, `FillReport`, `PrefetchTelemetry` | mixed |

### SourceReader internals

- Owns `demuxer: Box<dyn Demuxer>`, `decoder: Box<dyn Decoder>`, `stream: StreamInfo`, `meta: SourceMetadata`, `pending: Option<PendingFrame>` (decoded frame + sample cursor), `position: SourceFrame` (next sample to deliver), `position_anchor: Option<(u64 pts_frame, u64 delivered)>` , `finished: bool`.
- `open(demuxer, stream_id, mode)`: resolves stream, checks `stream.facts.decodable` (`InvalidValue` if not), constructs decoder via `demuxer.make_decoder(id, mode)`, derives canonical format (`AudioFormat::f32(channels, rate)` — `channels` from `ChannelLayout::count`, `Unsupported` if layout unsupported) and `frame_count` from `declared_units → duration.convert(1/rate)` (only when `declared_units` is `Known` and conversion exact-ish → `Fact::Known`, else `NotDeclared`).
- `next_frame()`: pump loop — `decoder.receive_frame()` → `Some` | `None` → `decoder.drain()` → `None` → `demuxer.read_packet()` → route target packet via `send_packet`, skip foreign/undecodable, EOF → `drain` once → `None` + `finished`. Errors propagate (kind distinguishes corrupt/truncated/io).
- `position`: after each decoded frame, `pts → MediaTime → convert(TimeBase::sample_rate(rate)) → value` = frame index; advance `position = anchor + delivered`. No pts/base → `Estimated` extrapolation from previous anchor.
- `seek_source_frame(target)`: `meta.seekable` gate (`Unsupported` → `Err(Unsupported)`); `demuxer.seek(MediaTime::new(target, TimeBase::sample_rate(rate)))` → `decoder.flush()` → clear pending → decode-discard forward; landing = first post-flush frame pts→frame. `SeekReceipt { requested, landed }` — `landed > requested` is reported, never silently clamped.
- `read_planar(dst, frames)`: canonical mode only (`Err(InvalidValue)` in native mode); copies `AudioFrame` planar f32 across frame boundaries; non-finite samples are passed through but the reader's `audit` hook counts them for prefetch-side fault marking. Returns `ReadOutcome { frames, eof }`.

### Prefetch pipeline

```
PrefetchHandle (non-RT owner)                 PrefetchProducer (non-RT, thread or manual)
  ├─ seek(SourceFrame) ─┐                         ├─ pop cmd (mpsc recv_timeout)
  ├─ reopen(src, open) ─┼─ mpsc<PrefetchCmd> ────▶├─ pop free PcmBlock (SPSC try_pop)
  └─ close()           ─┘                         │   → read_planar fill → push data ring
                                                  ├─ on seek: flush decoder state via
                                                  │   reader.seek → push Boundary{gen,first}
                                                  └─ on EOF → End{gen}; on err → Fault{kind,span}+End

PrefetchItem SPSC data ring (cap = block_count + markers slack)
  = Block(PcmBlock) | Boundary{gen, first} | Fault{gen, kind, first, frames} | End{gen}
PcmBlock free-return SPSC ring (cap = block_count) — infallible push (pool sized)

PrefetchConsumer (RT endpoint)
  └─ fill(&mut [&mut [f32]], expected_first: SourceFrame, &RtContext) -> FillReport
```

- `PcmBlock { data: Vec<f32> (channels*block_frames, channel-major planar), channels, block_frames, first_frame, valid_frames, generation }` — allocated once at `Prefetch::new`; every block returns to the free ring; `close()` audits `reclaimed == block_count`.
- `PrefetchCmd { Seek { generation, first }, Reopen { generation, source, open }, Quit }` — mpsc, non-RT side only.
- Producer pump: `pump_once() -> PumpEvent` = `Idle`/`Backpressured`/`Pushed`/`Ended`/`Quit`. Worker thread loops `pump_once` with `recv_timeout(1ms)` cadence; manual `pump` gives tests determinism.
- Generation `u64`: starts 1; every seek/reopen/close bumps. Handle-side `next_generation` is authoritative; producer copies it into items; consumer adopts `generation` from the latest `Boundary`. Stale items (`item.gen != consumer.gen` after a Boundary arrived) → dropped to free ring + `stale_drops++`.
- `fill` loop consumes items while `need > 0`: `Block` → copy overlap into dst, return block when drained; `Boundary` → adopt gen/first (then re-check `expected_first` — mismatch after Boundary → report); `Fault` → advance over span, zero-fill dst region, `faults[kind]++`; `End` → `ended=true`, remaining dst zeroed, `eof` flag; ring empty → `underrun += need`, zero-fill, break.
- `FillReport { requested, written, underrun, faulted, eof, mismatch, faults: FaultCounts { truncated, corrupt, io, invalid_sample, unsupported_layout, unseekable } }`. `PrefetchTelemetry` (Arc, relaxed atomics): `underrun_frames`, `underrun_events`, `fault_frames`, `stale_drops`, `eof_seen`, `position`, `generation`, `blocks_in_flight`.
- Fallback contract: zero-fill for underrun/fault/eof regions is the declared bounded fallback — always counted in the report/telemetry, never represented as decoded content.
- Lifecycle: `Prefetch::start(reader, config) -> Prefetch { handle, consumer }` spawns `std::thread`. `handle.close()` → Quit → join → drain both rings → `PrefetchAudit { blocks_reclaimed }`. `handle.reopen(source, open_fn)` → `Reopen` cmd: producer rebuilds `SourceReader` on the new source (fresh probe/metadata), bumps generation, emits `Boundary`. `seek` pre-validates `meta.seekable` (`Unsupported` → immediate `Err`, no generation bump).
- `Prefetch` disableable: callers may use `SourceReader` alone (no prefetch) or raw `Demuxer`/`Decoder` — orthogonal layers.

### Two-consumer posture

- Canonical path (`DecodeMode::Canonical`, `read_planar`, prefetch): planar f32, the playback representation. Explicit conversion is `PcmTransform`-recorded upstream.
- Native path (`DecodeMode::Native`, `next_frame()` → `Frame::NativeAudio`): native-scale f64 interleaved + `native_format` + timestamp provenance — the lossless observation path AudioLens uses; untouched by this work and reachable via the new trait method too.
- Reader API never assumes `File`/`Path`: input is `Box<dyn Demuxer>`/`Box<dyn MediaSource>`; `Reopen` carries a caller-supplied `open` fn.

## Sequence

### seek + generation isolation (RD-02)

```
Engine callback wants source frames starting at F2 after loop
  → handle.seek(F2)                [gen 2; cmd queued]
  → producer: pop cmd → reader.seek_source_frame(F2) → push Boundary{2, F2}
  → consumer.fill(dst, F2, ctx):
      pops old-gen Block items → drop to free ring, stale_drops++
      pops Boundary{2,F2} → adopt gen=2, next_frame=F2
      copies new-gen blocks; report.mismatch=false
Stale-gen block arriving *after* Boundary → dropped (never written to dst)
```

### underrun (RD-04)

```
fill(dst, F, 512): data ring empty, gen live, !ended
  → zero-fill 512, report {written:0, underrun:512}, telemetry.underrun_frames += 512
EOF: End{gen} seen → report {written:k, eof:true}; dst tail zeroed
```

## Data Models

- `SourceMetadata.frame_count: Fact<u64>` — `Known` only when `declared_units` decodes to a frame count under the canonical rate; otherwise `NotDeclared` (never fabricated).
- `FillReport`/`SourceFault`/`FaultCounts` — all `Copy`/plain data; no heap.
- `PcmBlock` channel-major layout: `data[c*block_frames .. +valid_frames]`; `channel(c) -> &[f32]`, `channel_mut(c) -> &mut [f32]` (bounds-checked once, non-RT fill).

## Error Handling

- Reader layer reuses `ruviform_media_io::Error`; kinds map: short-read/truncated → `UnexpectedEos`, corrupt → `InvalidData`, codec/layout gaps → `Unsupported`, bad seek → `Unsupported`/`InvalidValue`, arithmetic → `TimeOverflow`.
- Pipeline layer: `SourceFault` markers carry { `Truncated`, `Corrupt`, `Io`, `InvalidSample`, `UnsupportedLayout`, `Unseekable` }; demux/decode errors are stream-fatal (`Fault`+`End`), seek failure keeps position and emits `Fault{Unseekable}`+`Boundary` under the new generation.
- `fill` never returns `Err` — it *reports*; the only hard failure modes are contract misuse documented as panics (e.g. dst channel count ≠ block channels), checked once non-RT at `Prefetch::new` where possible.

## Testing

- `tests/reader.rs` (non-RT): mono/stereo WAV at frame 0 / mid seek / tail / EOF; native-mode `next_frame` parity; unseekable source (`Seek`-failing `MediaSource`) → `seekable=Unsupported`, seek error kind.
- `tests/prefetch.rs` (non-RT drive): deterministic mono ramp; mapping offsets; generation isolation (seek mid-flight, assert no stale samples); two independent consumers; block-count audit on close; reopen.
- `tests/rt_fill.rs` (`rt_*` names): `assert_no_alloc` around `fill` with pre-populated rings; underrun/fault/eof/mismatch branches each alloc-free; two-source interleave alloc-free.
- Fault matrix: truncated WAV (bad RIFF size) → `Fault{Truncated}`; corrupt payload → `Fault{Corrupt}`; NaN-carrying samples → `Fault{InvalidSample}` via injected malformed decode path (a small fake `Decoder` in tests for classes WAV cannot produce).
- Regression: existing `ruviform-io-symphonia`/`io-avi` tests unchanged must still pass (RD-06).

## Cross-Repo Compatibility

- Synvelis: `impacted` — new optional API; no existing signature changes; clip/placement mapping is caller-side (Synvelis #18 composition).
- AudioLens: `no impact` — `make_native_decoder`/`read_packet`/`send_packet` surface unchanged; new methods are defaulted trait additions its Rust version compiles fine; native/f64 path untouched. Verified by API-surface review of `al-media/src/ruviform_backend.rs`; local-path `cargo check` of AudioLens is optional corroboration (its git pin predates this work anyway). Unmeasured items recorded `unverified` in the impl record (RD-10).

## Open Questions

- `FrameMap` rate-ratio semantics: keep `ratio_num/ratio_den` integer rational (covers 1:1, varispeed) — stretch algorithms themselves remain caller policy.
- Spawned worker cadence `recv_timeout(1ms)` documented as best-effort; manual `pump` remains the deterministic contract used by tests.
