---
id: "TSPEC-NstsDZYKTSF6CL9kQDoyXC"
type: "test-spec"
status: "approved"
title: "Issue #198 sequential offline render — OF-01..OF-07 test specification"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "docs/specs/issue-198-sequential-offline-render.md"
    - "docs/plans/implement-issue-198-sequential-offline-render.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-198-sequential-offline-render.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-198-sequential-offline-render.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-198-sequential-offline-render.md"
    - "docs/designs/issue-198-sequential-offline-render.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-198-sequential-offline-render.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Test spec — Issue #198

All tests live in `crates/ruviform-audio/tests/render.rs` using `tests/common` fixtures. WAV end-to-end uses `ruviform-io-wav` + reopen via `ruviform-io-symphonia` demuxer/decoder (lossless PCM → exact compare). Platform: Windows 11 x64 (recorded); `--locked` runs recorded in the impl record.

## Fixtures

- `OnePole` (new, in `tests/render.rs`): `y[n] = 0.5·(0.5·y[n−1] + x[n])`-shaped stateful filter — per-sample state → bit-exact across any legal block split; `latency()` is *declared* to the plan without shifting produced samples, so it drives the driver's `position − total_latency` emit math directly; `offline_support` configurable (defaults `Verified`).
- `SignalFeeder`: boundary input `signal[c][position + j]` — position-tick-indexed deterministic pattern, making frame selection directly assertable on emitted PCM; `SilentFeeder` covers the no-input path.
- `Captured`/`CaptureEncoder`/`NullMuxer`: records emitted runs (pts + channel planes) at the encoder boundary — the observable artifact the output lifecycle produces.
- `MockDsp` (in `tests/common`, extended): stores `OfflineRenderSupport`, `with_support(...)` configures the capability gate.
- `EventPlan` events carrying `NodeEvent`-mappable payloads (gain change at position).
- `FailSink { inner: File, budget, fail_flush }` — disk-full at a byte budget and/or flush failure injection through the muxer's owned sink handle.
- `fail_publish` closure — publish-stage (sync-class) failure injection.
- `MockFactory`-based compile; processors pre-seeded `Verified` or `Unknown` per test.

## Cases

### of01_block_size_invariance
- `chain()` graph: Source(boundary) → `OnePoleDsp` → Sink. Range [0, 4096), job [0,4096+`total_latency`), `TailSpec::Known(latency)`.
- Render once with `BlockContract` {min=max=continuous span} (single block per span) and with contracts forcing 64/128/256-frame splits (`min=64,max=64` etc. plus final remainder handling via `partition`).
- Assert emitted PCM equal **bit-exact** (tolerance spec: `f32` exact, tolerance 0 — stateful filters are per-sample; blocks are only call granularity).

### of02_midstream_preroll_pdc_tail
- (a) mid-stream range [200, 600) over job [0,600): warm-up positions processed but never emitted; emitted PCM equals the latency-0 reference's [200,600) window exactly; first emitted pts = 200.
- (b) `OnePole` declaring latency L=96 → first L processed frames are warm-up; emitted[p] = dsp(p + L); `TailSpec::Known(64)` extends the emitted window to `range_end + 64`; receipt `tail_evidence = Known`, `frames_emitted = 576`, `range_end_with_tail = 576`.
- Emitted values asserted equal to the reference render's shifted window (PDC alignment), not only frame counts.

### of03_events_loop_transport_order
- Transport program: Play at 0; `SetTempo` at boundary frame; loop enable with wrap inside job; `TransportPlan` with two events scheduled at the *same* engine frame + one at a tempo boundary.
- Assert: emitted stream is identical to a reference single-block render; `MockDsp`/event log shows events delivered before their covering `process` call in window order; undeliverable-span case (illegal command boundary `< min_frames`) → `TransportUndeliverable`/`ScheduleFault` error and no published file.

### of04_live_independence_and_capability_gate
- Factory returning `offline_support → Unknown` → `Preflight(OfflineUnsupported{node})` before any file is created (assert temp+dest absent).
- Two `PreparedRender`s from the same graph with separate factories → `MockDsp`-style counters prove disjoint state (render A emits state-offset N, B unaffected). Live runtime untouched — compile only.

### of05_wav_reopen_verify
- Full `RenderOutput` with `WavEncoder`/`WavMuxer` into tempdir; verify hook = symphonia demuxer→decoder, assert channels=2, sample_rate=48000, decoded frames == emitted, PCM equality vs expected ramp.
- Assert `WavReceipt` frames matches; file exists only after publish; temp removed.

### of06_failure_injection / of06_staging_policy
- (a) Pre-cancelled token → `Cancelled{phase:"process"}`, dest absent, temp absent.
- (b) `FailSink` byte budget mid-stream → `Output`/`Finalize` typed error; dest absent.
- (c) `FailSink` flush failure at finalize → `Finalize`/`Output` typed error; dest absent.
- (d) publish closure failure → `Publish` error; dest absent; temp cleaned.
- (e) Pre-created destination + `PublishOptions::no_overwrite` → `DestinationExists`; existing file byte-for-byte unchanged.
- (f) `verify` returning Err → `Verify` error; dest absent.
- Assert in each case: `render()` returns the typed error *and* no partial file is left at destination or as a `.rvf-tmp-*` leftover.

### of07_verification_record
- Record in impl record: `cargo test -p ruviform-audio --locked`, `cargo test -p ruviform-exec --locked`, `cargo test --workspace --locked`, platform, exit codes; plus `mise run` gates list and any unverified items.

## Not covered (explicit)

- Real-plugin `Verified` attestation (#196); SRC/dither kernels (consumer-supplied); estimated-vs-actual tail audio-quality metrics (evidence is classified, not measured).
