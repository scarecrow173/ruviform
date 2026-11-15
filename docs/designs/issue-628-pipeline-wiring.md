---
id: "DESIGN-RF628pwpipeline000001"
type: "design"
status: "approved"
title: "Issue #628 pipeline wiring — detailed design"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-628-pipeline-wiring.md"
    - "docs/designs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-628-pipeline-wiring.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-pipeline-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-628-pipeline-wiring.md"
    - "docs/designs/issue-628-scripted-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-628-pipeline-wiring.md"
    - "docs/test-specs/issue-628-pipeline-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-628-scripted-backend.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-628-pipeline-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 pipeline wiring — detailed design

## Overview

`crates/ruviform-audio-test` に `pipeline` module を新設する。
3 本の独立した harness 脚（output / input / midi）を提供し、各脚は
「scripted backend → 本番 consumer → Expected/Observed oracle」の
三段構造を持つ。全て non-RT driver 側の orchestration であり、
callback/processing 部分のみ RT 契約を持つ。

依存方向: `ruviform-audio-test` → `ruviform-audio` /
`ruviform-audio-plugin-bridge` / `ruviform-graph` /
`ruviform-plugin-core`（path 依存、理由コメント付き）。
本番 crate 側から audio-test への依存は作らない。

## Module layout

```
src/pipeline.rs          — module doc, shared error type, re-exports
src/pipeline/output.rs   — Playback leg (Session::open)
src/pipeline/input.rs    — Input leg (typed RtRuntime + capture handoff)
src/pipeline/midi.rs     — MIDI leg (InputStage pump/collect_block)
src/pipeline/common.rs   — shared helpers (expected PCM block assembly,
                           PcmBlock comparison, PumpReport collection)
```

## Output leg (`pipeline::output`)

**構成要素**:

- `PlaybackPipeline`: `Session`（`ruviform_audio::session::Session`）+
  `ScriptedDeviceHandle` + `Vec<PumpReport>` + clock driver handle。
- `PlaybackPipeline::open(handle: &ScriptedDeviceHandle, config:
  OutputStreamConfig, graph: Graph) -> Result<Self, PipelineError>`:
  `Session::open(handle as &dyn Device, config, GraphPlan::new(graph))`
  → `session.start()`。Session が内部で `open_output_stream` を呼び、
  data/error callback を本番 `plan.drive` に結線する。
- `pump(&mut self) -> Result<PumpReport, DeviceError>`:
  `handle.pump_output_next()` を呼び report を蓄積。callback 内では
  本番 `Graph::pull` → node closure が `PcmFixture::fill_view` で PCM を
  書き込み、`record_output` が scratch を capture する。
- `captured(&self) -> &[PcmBlock]` / `reports(&self) -> &[PumpReport]`。

**Graph**: Source 境界なしの単一 closure node（`Graph::with_node` +
`NodeKind::Processor` ではなく `Node::closure` 相当 — 実際は
`GraphBuilder`/`Validator` ではなく `ruviform_audio::Graph` の直接構築
API を使用。詳細は compile 経路に依存）。node は `PcmFixture` を
`Arc` で持ち、call ごとに内部 cursor を `out.frames()` 分進めて
`fill_view(cursor, view)` する。cursor と device `position` は
pump が frames 連続で進める限り一致する。

**Expected**: `oracle::pcm_reference_block(fixture, block.start_frame,
block.frames, channel)` で各 channel の参照 PCM を独立生成し、
`PcmBlock.data`（planar `channels*frames`）と `pcm_bits_eq` で比較。

**Fault**: `FaultScript` の Error/Xrun/QueueFull/Invalidate は
`pump_output_next` が `Err(DeviceError)` を返すか error callback を
発火する — Observed として `PipelineError::Device` / `PumpReport` /
error callback 記録に残る。`record_output` capture との Expected 照合は
正常 pump のみで行い、fault pump は fault 種別の一致を検証する。

## Input leg (`pipeline::input`)

**構成要素**:

- `InputCapture`: bounded handoff。`AudioBufferView` + `InputCallbackInfo`
  を preallocated `CaptureBlock`（`channels * max_frames` f32 + metadata）
  へ copy する。`ruviform_rt::channel` の free/filled SPSC pool +
  `working` retry slot + `dropped`/`starved`/`in_flight` atomic counters
  （`ruviform-audio-device/tests/mock_input_backend.rs` 先例に倣う）。
  data callback 内では alloc なし・lock なし・bounded copy のみ。
- `InputPipeline`: `ScriptedDeviceHandle` + `Box<dyn InputStream>` +
  `GraphRuntime` + `RtRuntime` + boundary `PortId`（`in_port`/`out_port`）+
  `InputCapture` + observed error Vec + reports。
- `InputPipeline::open(handle, config: InputStreamConfig, gain: f32,
  compiler: &CompilerConfig) -> Result<Self, PipelineError>`:
  1. `Graph` 構築: Source(stereo out) → Processor(stereo in/out,
     `GainNode`) → Sink(stereo in)。`GraphBuilder`+`Validator`→
     `ValidatedGraph`。
  2. `GraphRuntime::new(&config, cmd_cap, ev_cap)` → `(runtime, rt)`。
  3. `Compiler::new(config).compile(&graph, &mut factory, ...)` —
     factory は Source/Sink に `Boundary`、Processor に
     `PrepareOutcome::New(GainNode)`。
  4. `runtime.publish(compiled)`。
  5. `device.open_input_stream(&config, data_cb, err_cb)` — data_cb は
     `InputCapture::offer(view, info)`、err_cb は `Vec<DeviceError>` へ push。
  6. `stream.start()`。
- `pump_block(&mut self) -> Result<PumpReport, DeviceError>`:
  `handle.pump_input_next()` → data callback が `InputCapture` を満たす
  → drain して各 `CaptureBlock` について:
  `rt.input_writer(in_port, frames)` で channel 面を埋め、
  `rt.process(frames, &RtContext::mint(...))`、
  `rt.output_reader(out_port)` で観測 PCM を `ObservedBlock` として記録
  （`start_frame` = capture の `frame_position`）。
- `GainNode`: `GraphProcessor` 実装。`latency()=0`、`process` は
  `io.input(0)` を gain 倍して `io.output(0)` へ書く。pipeline module
  内の test-only processor（本番 processor registry には登録しない）。

**Expected**: observed input PCM は `PcmScenario` の covering segment を
`oracle::pcm_reference_block` で独立生成（gap → silence 0.0）。
observed output PCM は `expected_input * gain`（pipeline 内で独立に
f32 乗算 — 本番 processor の出力を正解値として使わない）。

**到達不能面の記録**: device `InputCallbackInfo.timestamp.capture` /
`clock_domain` / `frame_position` は `CaptureBlock` metadata として
観測可能だが、typed runtime の `input_writer` は PCM 面のみを受け取る
（timestamp は `RtContext`/slot には流れない）。timestamp が production
consumer へ届く面は typed runtime ではなく render.rs の InputFeeder
相当 — 本 slice では capture metadata の観測 + NotRun 記録に留める。

## MIDI leg (`pipeline::midi`)

**構成要素**:

- `MidiLeg`: `MockBackend` + `MockPort` handle + `Box<dyn MidiInput>`
  （`MidiInputPort::open` 経由）+ pipeline-local `InputPump`（`InputTx::pump`
  相当: bounded staging `Vec<TimedMidiInput>` + `ruviform_rt::channel`
  Producer/Consumer + flush→read→flush）+ `InputStage` + scratch
  `Vec<Event>`。
- `MidiLeg::open(feed: MidiFeed or scenario, anchor config) -> Result<Self>`:
  - `MockBackend`/`MockPort` で `TimedMidiInput` を供給。
  - `InputStage::new()` → `publish_anchor(Some(ClockAnchor{...}))`。
    anchor は `stamp_ticks=0`, `stamp_base=frame_base`,
    `frame=EngineFrame::new(epoch,0)`, `calibration=Carried`。
- `pump_block(&mut self, upto_frame: u64, block_start: EngineFrame,
  block_frames: u32, now: EngineFrame) -> Result<BlockObservation>`:
  1. `feed.pump_into_port(&mock_port, upto_frame)` — due cue を
     `TimedMidiInput` として port へ。
  2. `input_pump.pump(&mut midi_input)` — production `InputTx::pump`
     契約を mirror（`InputTx` は `pub(crate)` のため pipeline-local に
     同契約を実装。staging capacity = `INPUT_STAGE_CAPACITY`）。
  3. `stage.pump(&mut rx)` — pending へ移送。
  4. `stage.collect_block(block_start, block_frames, now, &mut scratch)`
     → `Event` 列 + `BlockInputStats`。
- Observed: `Vec<(u32 frame_offset, MidiEvent)>` + `BlockInputStats` +
  `InputStage::counters()` cumulative。

**Expected**: `MidiScenario` cues を `(position_ticks, source_sequence)`
で sort 済みとして、各 cue の frame 位置 `f` が block `[s, s+frames)`
に入るとき `frame_offset = f - s`、block 通過後に届いた cue は
late → offset 0 + `late` counter。これは `map_stamp_to_frame` の
anchor 変換を test 側で独立に行った値（production mapping の出力を
正解値にしない）。

## Error type

```rust
#[non_exhaustive]
pub enum PipelineError {
    Device(DeviceError),
    Session(SessionError),
    Graph(GraphError),          // ruviform-graph validation/build
    Compile(CompileError),      // typed compiler
    Activation(ActivationError),
    Process(ProcessError),
    CaptureOverflow { ... },    // handoff capacity 超過
    Config(&'static str),       // channel/sample-rate mismatch 等
}
```

`From` impls を各 variant に用意。`PipelineError` は `#[non_exhaustive]`
（公開 enum policy）。

## RT contract

- `InputCapture::offer`（data callback 内）: alloc なし・lock なし・
  bounded copy。`assert_no_alloc` で `rt_` test。
- `GainNode::process`（typed runtime callback 内）: alloc なし。
  `rt_` test で `rt.process` を測定。
- `Session` の `plan.drive`（output callback 内）: `Graph::pull` +
  closure node は alloc なし（fixture fill は stack/borrowed view）。
  `rt_` test で pump を測定。
- MIDI leg の `InputStage::pump`/`collect_block` は production 側の
  RT 契約に従う（bounded staging・SPSC）。`rt_` test で pump+collect
  を測定。
- driver 側 orchestration（`pump_*_next` 呼び出し、handoff drain、
  expected 計算）は non-RT — alloc 許容。

## Honest limits

- deterministic backend Pass = OS-L1 deterministic evidence のみ。
  WASAPI/WinMM・実 window・plugin ABI の証明ではない。
- `InputStage::attach()`/実 endpoint は範囲外（Issue #630 slice）。
- 本番 Recording コンポーネントは未存在 — typed runtime 境界が
  device-input-tap 相当の到達面。
- timestamp/capture provenance は capture metadata として観測するが
  typed runtime の PCM 面には流れない — NotRun/Deferred 記録。
